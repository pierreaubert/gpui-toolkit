//! Bounded binary Arrow-IPC frames for revisioned dataset resources.
use arrow_array::RecordBatch;
use arrow_cast::display::{ArrayFormatter, FormatOptions};
use arrow_ipc::reader::StreamReader;
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

pub const MAX_DATASET_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DATASET_STORE_BYTES: usize = 1 << 30;
pub const MAX_DATASET_PREVIEW_ROWS: usize = 512;
pub const MAX_DATASET_CHART_POINTS: usize = 4096;
const MAX_DATASET_CHUNKS: u32 = 1 << 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetFrame {
    pub resource_id: String,
    pub generation: u64,
    pub sequence: u32,
    pub chunk_count: u32,
    pub byte_length: usize,
    pub schema_fingerprint: String,
    pub checksum: u64,
    #[serde(skip)]
    pub payload: Vec<u8>,
}

/// Metadata for one complete dataset or dense-array generation published
/// through a session-owned, read-only memory mapped file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedDatasetFrame {
    pub resource_id: String,
    pub generation: u64,
    pub sequence: u32,
    pub chunk_count: u32,
    pub byte_length: usize,
    pub schema_fingerprint: String,
    pub checksum: u64,
    pub filename: String,
    pub session_token: String,
    #[serde(skip)]
    pub payload: Option<Arc<MappedDatasetPayload>>,
}

impl MappedDatasetFrame {
    pub fn validate_header(&self) -> Result<(), DatasetFrameError> {
        if self.resource_id.trim().is_empty()
            || self.resource_id.len() > 128
            || self.schema_fingerprint.is_empty()
            || self.filename.is_empty()
            || self.session_token.is_empty()
        {
            return Err(DatasetFrameError::InvalidId);
        }
        if self.generation == 0
            || self.sequence != 0
            || self.chunk_count != 1
            || self.byte_length > MAX_DATASET_STORE_BYTES
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), DatasetFrameError> {
        self.validate_header()?;
        let payload = self
            .payload
            .as_ref()
            .ok_or(DatasetFrameError::InvalidMetadata)?;
        if payload.len() != self.byte_length {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        if DatasetFrame::checksum(payload.as_slice()) != self.checksum {
            return Err(DatasetFrameError::ChecksumMismatch);
        }
        Ok(())
    }
}

/// Retained read-only mapping. On Unix the publication file is unlinked as
/// soon as mapping succeeds; other platforms remove it when the last lease is
/// released.
pub struct MappedDatasetPayload {
    mmap: Mmap,
    cleanup_path: Option<PathBuf>,
}

impl MappedDatasetPayload {
    #[allow(unsafe_code)]
    pub fn map_file(path: &Path, expected_length: usize) -> Result<Self, DatasetFrameError> {
        if expected_length == 0 {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        }
        let file = options
            .open(path)
            .map_err(|_| DatasetFrameError::MappedFile)?;
        let metadata = file.metadata().map_err(|_| DatasetFrameError::MappedFile)?;
        if !metadata.is_file() || metadata.len() != expected_length as u64 {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(DatasetFrameError::InsecureMappedFile);
            }
        }
        // SAFETY: the host opens a session-private 0600 file read-only after
        // the Python publisher has closed it. The parent directory is owned by
        // the host and no writable file handle is retained by this API.
        let mmap = unsafe {
            MmapOptions::new()
                .len(expected_length)
                .map(&file)
                .map_err(|_| DatasetFrameError::MappedFile)?
        };
        #[cfg(unix)]
        let cleanup_path = {
            fs::remove_file(path).map_err(|_| DatasetFrameError::MappedFile)?;
            None
        };
        #[cfg(not(unix))]
        let cleanup_path = Some(path.to_path_buf());
        Ok(Self { mmap, cleanup_path })
    }

    pub fn as_slice(&self) -> &[u8] {
        self.mmap.as_ref()
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }
}

impl std::fmt::Debug for MappedDatasetPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MappedDatasetPayload")
            .field("byte_length", &self.len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for MappedDatasetPayload {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Drop for MappedDatasetPayload {
    fn drop(&mut self) {
        if let Some(path) = self.cleanup_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod dense_array_tests {
    use super::*;

    #[test]
    fn samples_dense_one_and_two_dimensional_buffers() {
        let one = [1.0_f32, 2.0, 3.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            sample_dense_xy(&one, &[3], "f32", 8).unwrap(),
            (vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 3.0])
        );

        let two = [1_i16, 10, 2, 20]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            sample_dense_xy(&two, &[2, 2], "i16", 8).unwrap(),
            (vec![1.0, 2.0], vec![10.0, 20.0])
        );
    }

    #[test]
    fn rejects_unsupported_dense_shapes_and_dtypes() {
        assert!(matches!(
            sample_dense_xy(&[0; 4], &[2, 1], "u8", 4),
            Err(DatasetFrameError::Decode(_))
        ));
        assert!(matches!(
            sample_dense_xy(&[0; 2], &[1], "f16", 4),
            Err(DatasetFrameError::Decode(_))
        ));
    }

    #[test]
    fn decodes_bounded_dense_grid_without_arrow() {
        let payload = [1_u16, 2, 3, 4]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            dense_grid(&payload, &[2, 2], "u16").unwrap(),
            (vec![1.0, 2.0, 3.0, 4.0], 2, 2)
        );
        assert!(matches!(
            dense_grid(&payload, &[4], "u16"),
            Err(DatasetFrameError::Decode(_))
        ));
        let column = [7_u8, 8, 9];
        assert_eq!(
            dense_grid(&column, &[3, 1], "u8").unwrap(),
            (vec![7.0, 8.0, 9.0], 1, 3)
        );
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum DatasetFrameError {
    #[error("invalid dataset resource id")]
    InvalidId,
    #[error("invalid dataset frame metadata")]
    InvalidMetadata,
    #[error("dataset frame exceeds {MAX_DATASET_FRAME_BYTES} bytes")]
    TooLarge,
    #[error("dataset frame checksum does not match payload")]
    ChecksumMismatch,
    #[error("memory-mapped dataset resource cannot be opened")]
    MappedFile,
    #[error("memory-mapped dataset resource permissions are not restrictive")]
    InsecureMappedFile,
    #[error("dataset Arrow IPC payload cannot be decoded: {0}")]
    Decode(String),
}

impl DatasetFrame {
    pub fn checksum(payload: &[u8]) -> u64 {
        payload
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
            })
    }
    pub fn validate(&self) -> Result<(), DatasetFrameError> {
        if self.resource_id.trim().is_empty()
            || self.resource_id.len() > 128
            || self.schema_fingerprint.is_empty()
        {
            return Err(DatasetFrameError::InvalidId);
        }
        if self.generation == 0
            || self.chunk_count == 0
            || self.chunk_count > MAX_DATASET_CHUNKS
            || self.sequence >= self.chunk_count
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        if self.payload.len() != self.byte_length || self.payload.len() > MAX_DATASET_FRAME_BYTES {
            return Err(DatasetFrameError::TooLarge);
        }
        if self.checksum != Self::checksum(&self.payload) {
            return Err(DatasetFrameError::ChecksumMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct PendingDataset {
    generation: u64,
    schema_fingerprint: String,
    chunk_count: u32,
    chunks: Vec<Option<Vec<u8>>>,
    payload_bytes: usize,
    allocation_bytes: usize,
}
impl PendingDataset {
    fn new(frame: &DatasetFrame) -> Self {
        let allocation_bytes =
            (frame.chunk_count as usize).saturating_mul(std::mem::size_of::<Option<Vec<u8>>>());
        Self {
            generation: frame.generation,
            schema_fingerprint: frame.schema_fingerprint.clone(),
            chunk_count: frame.chunk_count,
            chunks: vec![None; frame.chunk_count as usize],
            payload_bytes: 0,
            allocation_bytes,
        }
    }
    fn matches(&self, frame: &DatasetFrame) -> bool {
        self.generation == frame.generation
            && self.schema_fingerprint == frame.schema_fingerprint
            && self.chunk_count == frame.chunk_count
    }
    fn complete(&self) -> bool {
        self.chunks.iter().all(Option::is_some)
    }
    fn assemble(self, resource_id: String) -> DatasetFrame {
        let mut payload = Vec::with_capacity(self.payload_bytes);
        for chunk in self.chunks {
            payload.extend(chunk.expect("complete pending dataset contains every chunk"));
        }
        DatasetFrame {
            resource_id,
            generation: self.generation,
            sequence: 0,
            chunk_count: 1,
            byte_length: payload.len(),
            schema_fingerprint: self.schema_fingerprint,
            checksum: DatasetFrame::checksum(&payload),
            payload,
        }
    }
    fn held_bytes(&self) -> usize {
        self.payload_bytes.saturating_add(self.allocation_bytes)
    }
}

#[derive(Debug, Default)]
pub struct DatasetFrameStore {
    frames: HashMap<String, DatasetFrame>,
    retired_frames: HashMap<DatasetKey, DatasetFrame>,
    mapped_payloads: HashMap<DatasetKey, Arc<MappedDatasetPayload>>,
    pending: HashMap<String, PendingDataset>,
    latest_generations: HashMap<String, u64>,
    references: HashMap<DatasetKey, usize>,
    bytes_used: usize,
}

type DatasetKey = (String, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetFrameStats {
    pub resources: usize,
    pub pending_resources: usize,
    pub bytes_used: usize,
    pub referenced_resources: usize,
    pub references: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampledXySeries {
    pub label: String,
    pub color: Option<String>,
    pub dash: Option<String>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub y0: Vec<f64>,
    pub keys: Vec<String>,
}

fn validate_series_dash(value: Option<String>) -> Result<Option<String>, DatasetFrameError> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    if matches!(value.as_str(), "solid" | "dashed" | "dotted" | "dash_dot") {
        Ok(Some(value))
    } else {
        Err(DatasetFrameError::InvalidMetadata)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampledBarSeries {
    pub label: String,
    pub color: Option<String>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampledBarData {
    pub categories: Vec<String>,
    pub series: Vec<SampledBarSeries>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreemapResourceRow {
    pub id: String,
    pub parent: Option<String>,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetAggregationOp {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetAggregation {
    pub output: String,
    pub operation: DatasetAggregationOp,
    /// `None` is valid only for `count:*`.
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedRows {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatasetFilterValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatasetFilter {
    Field(String),
    Literal(DatasetFilterValue),
    Eq(Box<Self>, Box<Self>),
    Ne(Box<Self>, Box<Self>),
    Lt(Box<Self>, Box<Self>),
    Le(Box<Self>, Box<Self>),
    Gt(Box<Self>, Box<Self>),
    Ge(Box<Self>, Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Not(Box<Self>),
    IsNull(Box<Self>),
    In(Box<Self>, Vec<DatasetFilterValue>),
}

impl DatasetFilterValue {
    fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(value) => *value,
            Self::Number(value) => *value != 0.0,
            Self::String(value) => !value.is_empty(),
        }
    }
}

impl DatasetFilter {
    pub fn referenced_fields(&self) -> Vec<String> {
        let mut fields = Vec::new();
        self.fields(&mut fields);
        fields
    }

    fn fields(&self, output: &mut Vec<String>) {
        match self {
            Self::Field(field) => {
                if !output.contains(field) {
                    output.push(field.clone());
                }
            }
            Self::Literal(_) => {}
            Self::Eq(left, right)
            | Self::Ne(left, right)
            | Self::Lt(left, right)
            | Self::Le(left, right)
            | Self::Gt(left, right)
            | Self::Ge(left, right)
            | Self::And(left, right)
            | Self::Or(left, right) => {
                left.fields(output);
                right.fields(output);
            }
            Self::Not(value) | Self::IsNull(value) => value.fields(output),
            Self::In(value, _) => value.fields(output),
        }
    }

    fn evaluate(
        &self,
        values: &HashMap<String, Vec<DatasetFilterValue>>,
        row: usize,
    ) -> Result<DatasetFilterValue, DatasetFrameError> {
        let binary = |left: &Self, right: &Self| {
            Ok::<_, DatasetFrameError>((left.evaluate(values, row)?, right.evaluate(values, row)?))
        };
        let compare = |left: &DatasetFilterValue, right: &DatasetFilterValue| match (left, right) {
            (DatasetFilterValue::Number(left), DatasetFilterValue::Number(right)) => {
                Some(left.total_cmp(right))
            }
            (DatasetFilterValue::String(left), DatasetFilterValue::String(right)) => {
                Some(left.cmp(right))
            }
            (DatasetFilterValue::Bool(left), DatasetFilterValue::Bool(right)) => {
                Some(left.cmp(right))
            }
            _ => None,
        };
        Ok(match self {
            Self::Field(field) => values
                .get(field)
                .and_then(|values| values.get(row))
                .cloned()
                .ok_or(DatasetFrameError::InvalidMetadata)?,
            Self::Literal(value) => value.clone(),
            Self::Eq(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(left == right)
            }
            Self::Ne(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(left != right)
            }
            Self::Lt(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(compare(&left, &right).is_some_and(|value| value.is_lt()))
            }
            Self::Le(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(compare(&left, &right).is_some_and(|value| value.is_le()))
            }
            Self::Gt(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(compare(&left, &right).is_some_and(|value| value.is_gt()))
            }
            Self::Ge(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(compare(&left, &right).is_some_and(|value| value.is_ge()))
            }
            Self::And(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(left.truthy() && right.truthy())
            }
            Self::Or(left, right) => {
                let (left, right) = binary(left, right)?;
                DatasetFilterValue::Bool(left.truthy() || right.truthy())
            }
            Self::Not(value) => DatasetFilterValue::Bool(!value.evaluate(values, row)?.truthy()),
            Self::IsNull(value) => DatasetFilterValue::Bool(matches!(
                value.evaluate(values, row)?,
                DatasetFilterValue::Null
            )),
            Self::In(value, candidates) => {
                let value = value.evaluate(values, row)?;
                DatasetFilterValue::Bool(candidates.contains(&value))
            }
        })
    }

    fn batch_mask(&self, batch: &RecordBatch) -> Result<Vec<bool>, DatasetFrameError> {
        let mut fields = Vec::new();
        self.fields(&mut fields);
        let mut values = HashMap::<String, Vec<DatasetFilterValue>>::new();
        for field in fields {
            let index = batch
                .schema()
                .index_of(&field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let column = batch.column(index);
            let formatter = ArrayFormatter::try_new(column.as_ref(), &FormatOptions::default())
                .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let mut column_values = Vec::with_capacity(batch.num_rows());
            for row in 0..batch.num_rows() {
                let value = if column.is_null(row) {
                    DatasetFilterValue::Null
                } else {
                    let value = formatter.value(row).to_string();
                    match value.as_str() {
                        "true" => DatasetFilterValue::Bool(true),
                        "false" => DatasetFilterValue::Bool(false),
                        _ => value.parse::<f64>().map_or_else(
                            |_| DatasetFilterValue::String(value),
                            DatasetFilterValue::Number,
                        ),
                    }
                };
                column_values.push(value);
            }
            values.insert(field, column_values);
        }
        (0..batch.num_rows())
            .map(|row| self.evaluate(&values, row).map(|value| value.truthy()))
            .collect()
    }
}

#[derive(Debug, Clone)]
enum AggregationState {
    Count(u64),
    Sum(f64),
    Mean { sum: f64, count: u64 },
    Min(Option<f64>),
    Max(Option<f64>),
    First(Option<String>),
    Last(Option<String>),
}

impl AggregationState {
    fn new(operation: DatasetAggregationOp) -> Self {
        match operation {
            DatasetAggregationOp::Count => Self::Count(0),
            DatasetAggregationOp::Sum => Self::Sum(0.0),
            DatasetAggregationOp::Mean => Self::Mean { sum: 0.0, count: 0 },
            DatasetAggregationOp::Min => Self::Min(None),
            DatasetAggregationOp::Max => Self::Max(None),
            DatasetAggregationOp::First => Self::First(None),
            DatasetAggregationOp::Last => Self::Last(None),
        }
    }

    fn update(
        &mut self,
        _operation: DatasetAggregationOp,
        value: Option<String>,
    ) -> Result<(), DatasetFrameError> {
        match self {
            Self::Count(count) => {
                if value.is_some() {
                    *count = count.saturating_add(1);
                }
            }
            Self::First(first) => {
                if first.is_none() {
                    *first = value;
                }
            }
            Self::Last(last) => {
                if value.is_some() {
                    *last = value;
                }
            }
            state => {
                let Some(value) = value else {
                    return Ok(());
                };
                let number = value
                    .parse::<f64>()
                    .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                if !number.is_finite() {
                    return Err(DatasetFrameError::InvalidMetadata);
                }
                match state {
                    Self::Sum(sum) => *sum += number,
                    Self::Mean { sum, count } => {
                        *sum += number;
                        *count = count.saturating_add(1);
                    }
                    Self::Min(minimum) => {
                        *minimum = Some(minimum.map_or(number, |current| current.min(number)));
                    }
                    Self::Max(maximum) => {
                        *maximum = Some(maximum.map_or(number, |current| current.max(number)));
                    }
                    _ => return Err(DatasetFrameError::InvalidMetadata),
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> String {
        match self {
            Self::Count(count) => count.to_string(),
            Self::Sum(sum) => sum.to_string(),
            Self::Mean { sum, count } if count > 0 => (sum / count as f64).to_string(),
            Self::Mean { .. } | Self::Min(None) | Self::Max(None) => String::new(),
            Self::Min(Some(value)) | Self::Max(Some(value)) => value.to_string(),
            Self::First(value) | Self::Last(value) => value.unwrap_or_default(),
        }
    }
}

impl AggregatedRows {
    fn field_index(&self, field: &str) -> Result<usize, DatasetFrameError> {
        self.fields
            .iter()
            .position(|candidate| candidate == field)
            .ok_or(DatasetFrameError::InvalidMetadata)
    }

    pub fn sample_label_values(
        &self,
        label_field: &str,
        value_field: &str,
    ) -> Result<(Vec<String>, Vec<f64>), DatasetFrameError> {
        let label_index = self.field_index(label_field)?;
        let value_index = self.field_index(value_field)?;
        let mut labels = Vec::with_capacity(self.rows.len());
        let mut values = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let value = row[value_index]
                .parse::<f64>()
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            if !value.is_finite() {
                return Err(DatasetFrameError::InvalidMetadata);
            }
            labels.push(row[label_index].clone());
            values.push(value);
        }
        Ok((labels, values))
    }

    pub fn sample_xy_series(
        &self,
        x_field: &str,
        y_field: &str,
        series_field: Option<&str>,
        color_field: Option<&str>,
        key_field: Option<&str>,
        dash_field: Option<&str>,
        y0_field: Option<&str>,
    ) -> Result<Vec<SampledXySeries>, DatasetFrameError> {
        let x_index = self.field_index(x_field)?;
        let y_index = self.field_index(y_field)?;
        let series_index = series_field
            .map(|field| self.field_index(field))
            .transpose()?;
        let color_index = color_field
            .map(|field| self.field_index(field))
            .transpose()?;
        let key_index = key_field.map(|field| self.field_index(field)).transpose()?;
        let dash_index = dash_field
            .map(|field| self.field_index(field))
            .transpose()?;
        let y0_index = y0_field.map(|field| self.field_index(field)).transpose()?;
        let mut indexes = HashMap::<(String, Option<String>, Option<String>), usize>::new();
        let mut output = Vec::<SampledXySeries>::new();
        for row in &self.rows {
            let x = row[x_index]
                .parse::<f64>()
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let y = row[y_index]
                .parse::<f64>()
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            if !x.is_finite() || !y.is_finite() {
                return Err(DatasetFrameError::InvalidMetadata);
            }
            let series = series_index.map(|index| row[index].clone());
            let color = color_index.map(|index| row[index].clone());
            let dash = validate_series_dash(dash_index.map(|index| row[index].clone()))?;
            let y0 = y0_index
                .map(|index| row[index].parse::<f64>())
                .transpose()
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            if y0.is_some_and(|value| !value.is_finite()) {
                return Err(DatasetFrameError::InvalidMetadata);
            }
            let label = series
                .clone()
                .or_else(|| color.clone())
                .unwrap_or_else(|| "Series 1".into());
            let group = (label.clone(), color.clone(), dash.clone());
            let index = *indexes.entry(group).or_insert_with(|| {
                let index = output.len();
                output.push(SampledXySeries {
                    label,
                    color,
                    dash,
                    x: Vec::new(),
                    y: Vec::new(),
                    y0: Vec::new(),
                    keys: Vec::new(),
                });
                index
            });
            output[index].x.push(x);
            output[index].y.push(y);
            if let Some(y0) = y0 {
                output[index].y0.push(y0);
            }
            output[index].keys.push(
                key_index
                    .map(|index| row[index].clone())
                    .unwrap_or_default(),
            );
        }
        Ok(output)
    }

    pub fn sample_bar_series(
        &self,
        label_field: &str,
        value_field: &str,
        series_field: Option<&str>,
        color_field: Option<&str>,
    ) -> Result<SampledBarData, DatasetFrameError> {
        let label_index = self.field_index(label_field)?;
        let value_index = self.field_index(value_field)?;
        let series_index = series_field
            .map(|field| self.field_index(field))
            .transpose()?;
        let color_index = color_field
            .map(|field| self.field_index(field))
            .transpose()?;
        let mut category_indexes = HashMap::<String, usize>::new();
        let mut series_indexes = HashMap::<(String, Option<String>), usize>::new();
        let mut categories = Vec::<String>::new();
        let mut series = Vec::<SampledBarSeries>::new();
        for row in &self.rows {
            let value = row[value_index]
                .parse::<f64>()
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            if !value.is_finite() {
                return Err(DatasetFrameError::InvalidMetadata);
            }
            let category = row[label_index].clone();
            let category_index = *category_indexes.entry(category.clone()).or_insert_with(|| {
                let index = categories.len();
                categories.push(category);
                for item in &mut series {
                    item.values.push(0.0);
                }
                index
            });
            let color = color_index.map(|index| row[index].clone());
            let label = series_index
                .map(|index| row[index].clone())
                .or_else(|| color.clone())
                .unwrap_or_else(|| "Series 1".into());
            let group = (label.clone(), color.clone());
            let series_index = *series_indexes.entry(group).or_insert_with(|| {
                let index = series.len();
                series.push(SampledBarSeries {
                    label,
                    color,
                    values: vec![0.0; categories.len()],
                });
                index
            });
            series[series_index].values[category_index] += value;
        }
        Ok(SampledBarData { categories, series })
    }

    pub fn treemap_rows(
        &self,
        id_field: &str,
        parent_field: &str,
        value_field: &str,
    ) -> Result<Vec<TreemapResourceRow>, DatasetFrameError> {
        let id_index = self.field_index(id_field)?;
        let parent_index = self.field_index(parent_field)?;
        let value_index = self.field_index(value_field)?;
        self.rows
            .iter()
            .map(|row| {
                let value = row[value_index]
                    .parse::<f64>()
                    .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                if row[id_index].is_empty() || !value.is_finite() {
                    return Err(DatasetFrameError::InvalidMetadata);
                }
                Ok(TreemapResourceRow {
                    id: row[id_index].clone(),
                    parent: (!row[parent_index].is_empty()).then(|| row[parent_index].clone()),
                    value,
                })
            })
            .collect()
    }
}

impl DatasetFrameStore {
    fn payload_bytes<'a>(&'a self, frame: &'a DatasetFrame) -> &'a [u8] {
        self.mapped_payloads
            .get(&(frame.resource_id.clone(), frame.generation))
            .map_or_else(|| frame.payload.as_slice(), |payload| payload.as_slice())
    }

    pub fn stats(&self) -> DatasetFrameStats {
        DatasetFrameStats {
            resources: self.frames.len() + self.retired_frames.len(),
            pending_resources: self.pending.len(),
            bytes_used: self.bytes_used,
            referenced_resources: self.references.len(),
            references: self.references.values().sum(),
        }
    }

    /// Return a completed payload without decoding it as Arrow. ArrayData uses
    /// the same bounded, revisioned frame envelope but owns a dense raw buffer.
    pub fn raw_payload(&self, resource_id: &str) -> Option<&[u8]> {
        self.frames
            .get(resource_id)
            .map(|frame| self.payload_bytes(frame))
    }

    /// Return a completed dense payload only when its generation matches.
    pub fn raw_payload_at(&self, resource_id: &str, generation: u64) -> Option<&[u8]> {
        self.frames
            .get(resource_id)
            .filter(|frame| frame.generation == generation)
            .or_else(|| {
                self.retired_frames
                    .get(&(resource_id.to_owned(), generation))
            })
            .map(|frame| self.payload_bytes(frame))
    }

    /// Retain one completed generation for a native consumer. A retained old
    /// generation survives publication of a newer generation until all owners
    /// release it.
    pub fn retain(&mut self, resource_id: &str, generation: u64) -> bool {
        let key = (resource_id.to_owned(), generation);
        let exists = self
            .frames
            .get(resource_id)
            .is_some_and(|frame| frame.generation == generation)
            || self.retired_frames.contains_key(&key);
        if !exists {
            return false;
        }
        *self.references.entry(key).or_default() += 1;
        true
    }

    /// Release one native owner and reclaim an obsolete generation when its
    /// final lease ends.
    pub fn release_reference(&mut self, resource_id: &str, generation: u64) -> bool {
        let key = (resource_id.to_owned(), generation);
        let Some(count) = self.references.get_mut(&key) else {
            return false;
        };
        if *count > 1 {
            *count -= 1;
            return true;
        }
        self.references.remove(&key);
        if let Some(frame) = self.retired_frames.remove(&key) {
            let bytes = self
                .mapped_payloads
                .remove(&key)
                .map_or(frame.payload.len(), |payload| payload.len());
            self.bytes_used = self.bytes_used.saturating_sub(bytes);
        }
        true
    }

    /// Move a still-referenced current generation to the retired set, or
    /// reclaim it immediately. Returns the number of reclaimed bytes.
    fn retire_or_remove_current(&mut self, resource_id: &str) -> usize {
        let Some(frame) = self.frames.remove(resource_id) else {
            return 0;
        };
        let key = (resource_id.to_owned(), frame.generation);
        if self.references.contains_key(&key) {
            self.retired_frames.insert(key, frame);
            0
        } else {
            self.mapped_payloads
                .remove(&key)
                .map_or(frame.payload.len(), |payload| payload.len())
        }
    }

    /// Retain one complete mmap-backed generation without copying its bytes
    /// into the host heap.
    pub fn ingest_mapped(
        &mut self,
        mut frame: MappedDatasetFrame,
    ) -> Result<bool, DatasetFrameError> {
        frame.validate()?;
        let resource_id = frame.resource_id.clone();
        let latest = self
            .latest_generations
            .get(&resource_id)
            .copied()
            .unwrap_or(0);
        if frame.generation <= latest {
            return Ok(false);
        }
        let payload = frame
            .payload
            .take()
            .ok_or(DatasetFrameError::InvalidMetadata)?;
        let old_bytes = self.frames.get(&resource_id).map_or(0, |old| {
            let key = (resource_id.clone(), old.generation);
            if self.references.contains_key(&key) {
                0
            } else {
                self.payload_bytes(old).len()
            }
        });
        let pending_bytes = self
            .pending
            .get(&resource_id)
            .map_or(0, PendingDataset::held_bytes);
        let projected = self
            .bytes_used
            .saturating_sub(old_bytes)
            .saturating_sub(pending_bytes)
            .saturating_add(payload.len());
        if projected > MAX_DATASET_STORE_BYTES {
            return Err(DatasetFrameError::TooLarge);
        }
        self.pending.remove(&resource_id);
        self.retire_or_remove_current(&resource_id);
        self.frames.insert(
            resource_id.clone(),
            DatasetFrame {
                resource_id: resource_id.clone(),
                generation: frame.generation,
                sequence: 0,
                chunk_count: 1,
                byte_length: frame.byte_length,
                schema_fingerprint: frame.schema_fingerprint,
                checksum: frame.checksum,
                payload: Vec::new(),
            },
        );
        self.mapped_payloads
            .insert((resource_id.clone(), frame.generation), payload);
        self.latest_generations
            .insert(resource_id, frame.generation);
        self.bytes_used = projected;
        Ok(true)
    }

    /// Returns true only when this call makes a completed generation available.
    pub fn ingest(&mut self, frame: DatasetFrame) -> Result<bool, DatasetFrameError> {
        frame.validate()?;
        let resource_id = frame.resource_id.clone();
        let latest = self
            .latest_generations
            .get(&resource_id)
            .copied()
            .unwrap_or(0);
        if frame.generation < latest
            || (frame.generation == latest && !self.pending.contains_key(&resource_id))
        {
            return Ok(false);
        }
        if let Some(pending) = self.pending.get(&resource_id) {
            if frame.generation == pending.generation {
                if !pending.matches(&frame) {
                    return Err(DatasetFrameError::InvalidMetadata);
                }
                if let Some(existing) = &pending.chunks[frame.sequence as usize] {
                    return if existing == &frame.payload {
                        Ok(false)
                    } else {
                        Err(DatasetFrameError::InvalidMetadata)
                    };
                }
            }
        }
        let previous_pending_bytes = self
            .pending
            .get(&resource_id)
            .filter(|pending| pending.generation < frame.generation)
            .map_or(0, PendingDataset::held_bytes);
        let creates_pending =
            !self.pending.contains_key(&resource_id) || previous_pending_bytes > 0;
        let pending_allocation = if creates_pending {
            (frame.chunk_count as usize).saturating_mul(std::mem::size_of::<Option<Vec<u8>>>())
        } else {
            0
        };
        let prospective = self
            .bytes_used
            .saturating_sub(previous_pending_bytes)
            .saturating_add(pending_allocation)
            .saturating_add(frame.payload.len());
        if prospective > MAX_DATASET_STORE_BYTES {
            return Err(DatasetFrameError::TooLarge);
        }
        if self
            .pending
            .get(&resource_id)
            .is_some_and(|pending| pending.generation < frame.generation)
        {
            self.pending.remove(&resource_id);
            self.bytes_used = self.bytes_used.saturating_sub(previous_pending_bytes);
        }
        self.latest_generations
            .insert(resource_id.clone(), frame.generation);
        let pending = self
            .pending
            .entry(resource_id.clone())
            .or_insert_with(|| PendingDataset::new(&frame));
        pending.payload_bytes += frame.payload.len();
        pending.chunks[frame.sequence as usize] = Some(frame.payload);
        self.bytes_used += pending.chunks[frame.sequence as usize]
            .as_ref()
            .map_or(0, Vec::len)
            + pending_allocation;
        if !pending.complete() {
            return Ok(false);
        }
        let pending = self
            .pending
            .remove(&resource_id)
            .expect("pending dataset exists");
        self.bytes_used = self.bytes_used.saturating_sub(pending.allocation_bytes);
        let completed = pending.assemble(resource_id.clone());
        let reclaimed = self.retire_or_remove_current(&resource_id);
        self.bytes_used = self.bytes_used.saturating_sub(reclaimed);
        self.frames.insert(resource_id, completed);
        Ok(true)
    }
    pub fn get(&self, resource_id: &str) -> Option<&DatasetFrame> {
        self.frames.get(resource_id)
    }

    /// Read a bounded row window from completed Arrow IPC. Values remain in
    /// the binary resource channel: this is a host-side consumer API, never
    /// a UI-IR serialization path.
    pub fn preview_rows(
        &self,
        resource_id: &str,
        fields: &[String],
        start: usize,
        count: usize,
    ) -> Result<Option<Vec<Vec<String>>>, DatasetFrameError> {
        if count > MAX_DATASET_PREVIEW_ROWS {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut skipped = 0_usize;
        let mut output = Vec::with_capacity(count);
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let selected = if fields.is_empty() {
                (0..batch.num_columns()).collect::<Vec<_>>()
            } else {
                fields
                    .iter()
                    .map(|field| {
                        batch
                            .schema()
                            .index_of(field)
                            .map_err(|_| DatasetFrameError::InvalidMetadata)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let formatters = selected
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            for row_index in 0..batch.num_rows() {
                if skipped < start {
                    skipped += 1;
                    continue;
                }
                if output.len() == count {
                    return Ok(Some(output));
                }
                output.push(
                    formatters
                        .iter()
                        .map(|formatter| formatter.value(row_index).to_string())
                        .collect(),
                );
            }
        }
        Ok(Some(output))
    }

    pub fn count_rows_filtered(
        &self,
        resource_id: &str,
        filter: &DatasetFilter,
    ) -> Result<Option<usize>, DatasetFrameError> {
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut count = 0_usize;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            count = count.saturating_add(
                filter
                    .batch_mask(&batch)?
                    .into_iter()
                    .filter(|matches| *matches)
                    .count(),
            );
        }
        Ok(Some(count))
    }

    pub fn preview_rows_filtered(
        &self,
        resource_id: &str,
        fields: &[String],
        filter: &DatasetFilter,
        start: usize,
        count: usize,
    ) -> Result<Option<Vec<Vec<String>>>, DatasetFrameError> {
        if count > MAX_DATASET_PREVIEW_ROWS {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut output = Vec::with_capacity(count);
        let mut skipped = 0_usize;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let selected = fields
                .iter()
                .map(|field| {
                    batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let formatters = selected
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            for (row, matches) in filter.batch_mask(&batch)?.into_iter().enumerate() {
                if !matches {
                    continue;
                }
                if skipped < start {
                    skipped += 1;
                    continue;
                }
                if output.len() == count {
                    return Ok(Some(output));
                }
                output.push(
                    formatters
                        .iter()
                        .map(|formatter| formatter.value(row).to_string())
                        .collect(),
                );
            }
        }
        Ok(Some(output))
    }

    /// Count rows that satisfy a truthy predicate without materializing them
    /// as UI elements. The result lets virtualized tables retain correct
    /// paging bounds for a filtered `DatasetView`.
    pub fn count_rows_where_truthy(
        &self,
        resource_id: &str,
        predicate_field: &str,
    ) -> Result<Option<usize>, DatasetFrameError> {
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut count = 0_usize;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let predicate_index = batch
                .schema()
                .index_of(predicate_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let predicate_formatter = ArrayFormatter::try_new(
                batch.column(predicate_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            for row_index in 0..batch.num_rows() {
                let predicate = predicate_formatter.value(row_index).to_string();
                if matches!(predicate.trim().to_ascii_lowercase().as_str(), "true" | "1") {
                    count = count.saturating_add(1);
                }
            }
        }
        Ok(Some(count))
    }

    /// Read a bounded visible row window after applying the declarative
    /// truthy-field filter used by `DatasetView.filter(data.col("field"))`.
    /// The predicate is evaluated from Arrow values in the host; data never
    /// becomes part of a JSON UI patch.
    pub fn preview_rows_where_truthy(
        &self,
        resource_id: &str,
        fields: &[String],
        predicate_field: &str,
        start: usize,
        count: usize,
    ) -> Result<Option<Vec<Vec<String>>>, DatasetFrameError> {
        if count > MAX_DATASET_PREVIEW_ROWS {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut output = Vec::with_capacity(count);
        let mut skipped = 0_usize;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let selected = if fields.is_empty() {
                (0..batch.num_columns()).collect::<Vec<_>>()
            } else {
                fields
                    .iter()
                    .map(|field| {
                        batch
                            .schema()
                            .index_of(field)
                            .map_err(|_| DatasetFrameError::InvalidMetadata)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let formatters = selected
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let predicate_index = batch
                .schema()
                .index_of(predicate_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let predicate_formatter = ArrayFormatter::try_new(
                batch.column(predicate_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            for row_index in 0..batch.num_rows() {
                let predicate = predicate_formatter.value(row_index).to_string();
                if !matches!(predicate.trim().to_ascii_lowercase().as_str(), "true" | "1") {
                    continue;
                }
                if skipped < start {
                    skipped += 1;
                    continue;
                }
                if output.len() == count {
                    return Ok(Some(output));
                }
                output.push(
                    formatters
                        .iter()
                        .map(|formatter| formatter.value(row_index).to_string())
                        .collect(),
                );
            }
        }
        Ok(Some(output))
    }

    /// Sort a visible table window in the host without materializing rows in
    /// the UI document. Numeric display values use total numeric ordering;
    /// other Arrow values use stable lexical ordering.
    pub fn preview_rows_sorted(
        &self,
        resource_id: &str,
        fields: &[String],
        predicate_field: Option<&str>,
        sort_field: &str,
        descending: bool,
        start: usize,
        count: usize,
    ) -> Result<Option<Vec<Vec<String>>>, DatasetFrameError> {
        if count > MAX_DATASET_PREVIEW_ROWS || sort_field.is_empty() {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut rows = Vec::<(String, Vec<String>)>::new();
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let selected = if fields.is_empty() {
                (0..batch.num_columns()).collect::<Vec<_>>()
            } else {
                fields
                    .iter()
                    .map(|field| {
                        batch
                            .schema()
                            .index_of(field)
                            .map_err(|_| DatasetFrameError::InvalidMetadata)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let formatters = selected
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let sort_index = batch
                .schema()
                .index_of(sort_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let sort_formatter = ArrayFormatter::try_new(
                batch.column(sort_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let predicate_formatter = predicate_field
                .map(|field| {
                    let index = batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                    ArrayFormatter::try_new(batch.column(index).as_ref(), &FormatOptions::default())
                        .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .transpose()?;
            for row_index in 0..batch.num_rows() {
                if predicate_formatter.as_ref().is_some_and(|formatter| {
                    !matches!(
                        formatter
                            .value(row_index)
                            .to_string()
                            .trim()
                            .to_ascii_lowercase()
                            .as_str(),
                        "true" | "1"
                    )
                }) {
                    continue;
                }
                rows.push((
                    sort_formatter.value(row_index).to_string(),
                    formatters
                        .iter()
                        .map(|formatter| formatter.value(row_index).to_string())
                        .collect(),
                ));
            }
        }
        rows.sort_by(|left, right| {
            let ordering = match (left.0.parse::<f64>(), right.0.parse::<f64>()) {
                (Ok(left), Ok(right)) => left.total_cmp(&right),
                _ => left.0.cmp(&right.0),
            };
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        Ok(Some(
            rows.into_iter()
                .skip(start)
                .take(count)
                .map(|(_, row)| row)
                .collect(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn preview_rows_sorted_filtered(
        &self,
        resource_id: &str,
        fields: &[String],
        filter: &DatasetFilter,
        sort_field: &str,
        descending: bool,
        start: usize,
        count: usize,
    ) -> Result<Option<Vec<Vec<String>>>, DatasetFrameError> {
        if count > MAX_DATASET_PREVIEW_ROWS || sort_field.is_empty() {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut rows = Vec::<(String, Vec<String>)>::new();
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let selected = fields
                .iter()
                .map(|field| {
                    batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let formatters = selected
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let sort_index = batch
                .schema()
                .index_of(sort_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let sort_formatter = ArrayFormatter::try_new(
                batch.column(sort_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            for (row, matches) in filter.batch_mask(&batch)?.into_iter().enumerate() {
                if matches {
                    rows.push((
                        sort_formatter.value(row).to_string(),
                        formatters
                            .iter()
                            .map(|formatter| formatter.value(row).to_string())
                            .collect(),
                    ));
                }
            }
        }
        rows.sort_by(|left, right| {
            let ordering = match (left.0.parse::<f64>(), right.0.parse::<f64>()) {
                (Ok(left), Ok(right)) => left.total_cmp(&right),
                _ => left.0.cmp(&right.0),
            };
            if descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        Ok(Some(
            rows.into_iter()
                .skip(start)
                .take(count)
                .map(|(_, row)| row)
                .collect(),
        ))
    }

    /// Execute one declarative grouping/aggregation stage directly over Arrow
    /// IPC. Only the bounded requested window is returned to the UI consumer.
    #[allow(clippy::too_many_arguments)]
    pub fn aggregate_rows(
        &self,
        resource_id: &str,
        group_fields: &[String],
        aggregations: &[DatasetAggregation],
        predicate_field: Option<&str>,
        sort_field: Option<&str>,
        descending: bool,
        start: usize,
        count: usize,
    ) -> Result<Option<AggregatedRows>, DatasetFrameError> {
        let filter = predicate_field.map(|field| DatasetFilter::Field(field.to_owned()));
        self.aggregate_rows_filtered(
            resource_id,
            group_fields,
            aggregations,
            filter.as_ref(),
            sort_field,
            descending,
            start,
            count,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn aggregate_rows_filtered(
        &self,
        resource_id: &str,
        group_fields: &[String],
        aggregations: &[DatasetAggregation],
        filter: Option<&DatasetFilter>,
        sort_field: Option<&str>,
        descending: bool,
        start: usize,
        count: usize,
    ) -> Result<Option<AggregatedRows>, DatasetFrameError> {
        if count > MAX_DATASET_CHART_POINTS
            || aggregations.is_empty()
            || group_fields.iter().any(|field| field.is_empty())
            || aggregations.iter().any(|aggregation| {
                aggregation.output.is_empty()
                    || (aggregation.operation == DatasetAggregationOp::Count
                        && aggregation.field.as_deref() == Some("*"))
                    || (aggregation.operation != DatasetAggregationOp::Count
                        && aggregation.field.is_none())
            })
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let mut output_fields = group_fields.to_vec();
        for aggregation in aggregations {
            if output_fields.contains(&aggregation.output) {
                return Err(DatasetFrameError::InvalidMetadata);
            }
            output_fields.push(aggregation.output.clone());
        }
        if sort_field.is_some_and(|field| !output_fields.iter().any(|item| item == field)) {
            return Err(DatasetFrameError::InvalidMetadata);
        }

        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut group_indexes = HashMap::<Vec<Option<String>>, usize>::new();
        let mut groups = Vec::<(Vec<Option<String>>, Vec<AggregationState>)>::new();

        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let group_columns = group_fields
                .iter()
                .map(|field| {
                    batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let group_formatters = group_columns
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let aggregation_columns = aggregations
                .iter()
                .map(|aggregation| {
                    aggregation
                        .field
                        .as_deref()
                        .map(|field| {
                            batch
                                .schema()
                                .index_of(field)
                                .map_err(|_| DatasetFrameError::InvalidMetadata)
                        })
                        .transpose()
                })
                .collect::<Result<Vec<_>, _>>()?;
            let aggregation_formatters = aggregation_columns
                .iter()
                .map(|index| {
                    index
                        .map(|index| {
                            ArrayFormatter::try_new(
                                batch.column(index).as_ref(),
                                &FormatOptions::default(),
                            )
                            .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                        })
                        .transpose()
                })
                .collect::<Result<Vec<_>, _>>()?;
            let filter_mask = filter.map(|filter| filter.batch_mask(&batch)).transpose()?;

            for row_index in 0..batch.num_rows() {
                if filter_mask
                    .as_ref()
                    .is_some_and(|matches| !matches[row_index])
                {
                    continue;
                }
                let key = group_columns
                    .iter()
                    .zip(&group_formatters)
                    .map(|(index, formatter)| {
                        (!batch.column(*index).is_null(row_index))
                            .then(|| formatter.value(row_index).to_string())
                    })
                    .collect::<Vec<_>>();
                let group_index = *group_indexes.entry(key.clone()).or_insert_with(|| {
                    let index = groups.len();
                    groups.push((
                        key,
                        aggregations
                            .iter()
                            .map(|aggregation| AggregationState::new(aggregation.operation))
                            .collect(),
                    ));
                    index
                });
                for (((aggregation, column), formatter), state) in aggregations
                    .iter()
                    .zip(&aggregation_columns)
                    .zip(&aggregation_formatters)
                    .zip(&mut groups[group_index].1)
                {
                    let value = match (column, formatter) {
                        (Some(column), Some(formatter))
                            if !batch.column(*column).is_null(row_index) =>
                        {
                            Some(formatter.value(row_index).to_string())
                        }
                        (None, None) => Some(String::new()),
                        _ => None,
                    };
                    state.update(aggregation.operation, value)?;
                }
            }
        }

        let total_rows = groups.len();
        let mut rows = groups
            .into_iter()
            .map(|(key, states)| {
                key.into_iter()
                    .map(Option::unwrap_or_default)
                    .chain(states.into_iter().map(AggregationState::finish))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if let Some(sort_field) = sort_field {
            let sort_index = output_fields
                .iter()
                .position(|field| field == sort_field)
                .ok_or(DatasetFrameError::InvalidMetadata)?;
            rows.sort_by(|left, right| {
                let ordering = match (
                    left[sort_index].parse::<f64>(),
                    right[sort_index].parse::<f64>(),
                ) {
                    (Ok(left), Ok(right)) => left.total_cmp(&right),
                    _ => left[sort_index].cmp(&right[sort_index]),
                };
                if descending {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }
        Ok(Some(AggregatedRows {
            fields: output_fields,
            rows: rows.into_iter().skip(start).take(count).collect(),
            total_rows,
        }))
    }

    /// Stream a typed-filtered, bounded row sample for chart consumers. The
    /// full matching dataset is never materialized, and all semantic-role
    /// columns stay aligned through deterministic reservoir replacement.
    pub fn sample_filtered_rows(
        &self,
        resource_id: &str,
        fields: &[String],
        filter: &DatasetFilter,
        row_range: Option<(usize, usize)>,
        max_rows: usize,
    ) -> Result<Option<AggregatedRows>, DatasetFrameError> {
        if fields.is_empty()
            || max_rows == 0
            || max_rows > MAX_DATASET_CHART_POINTS
            || row_range.is_some_and(|(start, stop)| start > stop)
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let mut unique = std::collections::HashSet::new();
        if fields
            .iter()
            .any(|field| field.is_empty() || !unique.insert(field))
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let (range_start, range_stop) = row_range.unwrap_or((0, usize::MAX));
        let mut matching_index = 0_usize;
        let mut sampled_count = 0_u64;
        let mut total_rows = 0_usize;
        let mut rows = Vec::<Vec<String>>::with_capacity(max_rows);
        'batches: for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let selected = fields
                .iter()
                .map(|field| {
                    batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let formatters = selected
                .iter()
                .map(|index| {
                    ArrayFormatter::try_new(
                        batch.column(*index).as_ref(),
                        &FormatOptions::default(),
                    )
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            for (row, matches) in filter.batch_mask(&batch)?.into_iter().enumerate() {
                if !matches {
                    continue;
                }
                let position = matching_index;
                matching_index = matching_index.saturating_add(1);
                if position < range_start {
                    continue;
                }
                if position >= range_stop {
                    break 'batches;
                }
                total_rows = total_rows.saturating_add(1);
                sampled_count = sampled_count.saturating_add(1);
                let values = formatters
                    .iter()
                    .map(|formatter| formatter.value(row).to_string())
                    .collect::<Vec<_>>();
                if rows.len() < max_rows {
                    rows.push(values);
                    continue;
                }
                let slot = sampled_count.wrapping_mul(0x9e37_79b9_7f4a_7c15) % sampled_count;
                if (slot as usize) < max_rows {
                    rows[slot as usize] = values;
                }
            }
        }
        Ok(Some(AggregatedRows {
            fields: fields.to_vec(),
            rows,
            total_rows,
        }))
    }

    /// Deterministically reservoir-sample finite numeric x/y values across a
    /// completed resource. This bounds chart draw work while giving LOD a
    /// representative view of late appends, not only the first record batch.
    pub fn sample_xy(
        &self,
        resource_id: &str,
        x_field: &str,
        y_field: &str,
        max_points: usize,
    ) -> Result<Option<(Vec<f64>, Vec<f64>)>, DatasetFrameError> {
        if max_points == 0 || max_points > MAX_DATASET_CHART_POINTS {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut x = Vec::with_capacity(max_points);
        let mut y = Vec::with_capacity(max_points);
        let mut seen = 0_u64;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let x_index = batch
                .schema()
                .index_of(x_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let y_index = batch
                .schema()
                .index_of(y_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let x_formatter =
                ArrayFormatter::try_new(batch.column(x_index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let y_formatter =
                ArrayFormatter::try_new(batch.column(y_index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            for row_index in 0..batch.num_rows() {
                let (Ok(x_value), Ok(y_value)) = (
                    x_formatter.value(row_index).to_string().parse::<f64>(),
                    y_formatter.value(row_index).to_string().parse::<f64>(),
                ) else {
                    continue;
                };
                if !x_value.is_finite() || !y_value.is_finite() {
                    continue;
                }
                seen = seen.saturating_add(1);
                if x.len() < max_points {
                    x.push(x_value);
                    y.push(y_value);
                    continue;
                }
                // A fixed mix makes the sample deterministic for a stable
                // generation while retaining each eligible point with its
                // standard reservoir probability.
                let slot = seen.wrapping_mul(0x9e37_79b9_7f4a_7c15) % seen;
                if (slot as usize) < max_points {
                    x[slot as usize] = x_value;
                    y[slot as usize] = y_value;
                }
            }
        }
        Ok(Some((x, y)))
    }
    /// Deterministically reservoir-sample finite numeric x/y values only
    /// where `predicate_field` is truthy. This is deliberately limited to
    /// the serializable `DatasetView.filter(data.col("field"))` form rather
    /// than evaluating arbitrary Python expressions in the host.
    pub fn sample_xy_where_truthy(
        &self,
        resource_id: &str,
        x_field: &str,
        y_field: &str,
        predicate_field: &str,
        max_points: usize,
    ) -> Result<Option<(Vec<f64>, Vec<f64>)>, DatasetFrameError> {
        if max_points == 0 || max_points > MAX_DATASET_CHART_POINTS {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut x = Vec::with_capacity(max_points);
        let mut y = Vec::with_capacity(max_points);
        let mut seen = 0_u64;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let x_index = batch
                .schema()
                .index_of(x_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let y_index = batch
                .schema()
                .index_of(y_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let predicate_index = batch
                .schema()
                .index_of(predicate_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let x_formatter =
                ArrayFormatter::try_new(batch.column(x_index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let y_formatter =
                ArrayFormatter::try_new(batch.column(y_index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let predicate_formatter = ArrayFormatter::try_new(
                batch.column(predicate_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            for row_index in 0..batch.num_rows() {
                let predicate = predicate_formatter.value(row_index).to_string();
                if !matches!(predicate.trim().to_ascii_lowercase().as_str(), "true" | "1") {
                    continue;
                }
                let (Ok(x_value), Ok(y_value)) = (
                    x_formatter.value(row_index).to_string().parse::<f64>(),
                    y_formatter.value(row_index).to_string().parse::<f64>(),
                ) else {
                    continue;
                };
                if !x_value.is_finite() || !y_value.is_finite() {
                    continue;
                }
                seen = seen.saturating_add(1);
                if x.len() < max_points {
                    x.push(x_value);
                    y.push(y_value);
                } else {
                    let slot = seen.wrapping_mul(0x9e37_79b9_7f4a_7c15) % seen;
                    if (slot as usize) < max_points {
                        x[slot as usize] = x_value;
                        y[slot as usize] = y_value;
                    }
                }
            }
        }
        Ok(Some((x, y)))
    }

    /// Sample a DatasetView row range before applying the chart LOD budget.
    /// `predicate_field` implements the supported field-truthiness filter;
    /// range offsets count only rows that satisfy that predicate.
    pub fn sample_xy_window(
        &self,
        resource_id: &str,
        x_field: &str,
        y_field: &str,
        predicate_field: Option<&str>,
        start: usize,
        stop: usize,
        max_points: usize,
    ) -> Result<Option<(Vec<f64>, Vec<f64>)>, DatasetFrameError> {
        if max_points == 0 || max_points > MAX_DATASET_CHART_POINTS || start > stop {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut x = Vec::with_capacity(max_points);
        let mut y = Vec::with_capacity(max_points);
        let mut eligible = 0_usize;
        let mut seen = 0_u64;
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let x_index = batch
                .schema()
                .index_of(x_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let y_index = batch
                .schema()
                .index_of(y_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let x_formatter =
                ArrayFormatter::try_new(batch.column(x_index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let y_formatter =
                ArrayFormatter::try_new(batch.column(y_index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let predicate_formatter = predicate_field
                .map(|field| {
                    let index = batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                    ArrayFormatter::try_new(batch.column(index).as_ref(), &FormatOptions::default())
                        .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .transpose()?;
            for row_index in 0..batch.num_rows() {
                if let Some(formatter) = &predicate_formatter {
                    let predicate = formatter.value(row_index).to_string();
                    if !matches!(predicate.trim().to_ascii_lowercase().as_str(), "true" | "1") {
                        continue;
                    }
                }
                if eligible < start {
                    eligible += 1;
                    continue;
                }
                if eligible >= stop {
                    return Ok(Some((x, y)));
                }
                eligible += 1;
                let (Ok(x_value), Ok(y_value)) = (
                    x_formatter.value(row_index).to_string().parse::<f64>(),
                    y_formatter.value(row_index).to_string().parse::<f64>(),
                ) else {
                    continue;
                };
                if !x_value.is_finite() || !y_value.is_finite() {
                    continue;
                }
                seen = seen.saturating_add(1);
                if x.len() < max_points {
                    x.push(x_value);
                    y.push(y_value);
                } else {
                    let slot = seen.wrapping_mul(0x9e37_79b9_7f4a_7c15) % seen;
                    if (slot as usize) < max_points {
                        x[slot as usize] = x_value;
                        y[slot as usize] = y_value;
                    }
                }
            }
        }
        Ok(Some((x, y)))
    }

    /// Deterministically sample categorical labels and finite numeric values.
    ///
    /// This is the resource-backed input path for bar, pie, and donut charts.
    /// It preserves Arrow string and dictionary labels instead of coercing the
    /// category column through `f64`, and applies filter/range/LOD bounds.
    pub fn sample_label_values(
        &self,
        resource_id: &str,
        label_field: &str,
        value_field: &str,
        predicate_field: Option<&str>,
        row_range: Option<(usize, usize)>,
        max_points: usize,
    ) -> Result<Option<(Vec<String>, Vec<f64>)>, DatasetFrameError> {
        if max_points == 0
            || max_points > MAX_DATASET_CHART_POINTS
            || row_range.is_some_and(|(start, stop)| start > stop)
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let (range_start, range_stop) = row_range.unwrap_or((0, usize::MAX));
        let mut labels = Vec::with_capacity(max_points);
        let mut values = Vec::with_capacity(max_points);
        let mut eligible = 0_usize;
        let mut seen = 0_u64;

        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let label_index = batch
                .schema()
                .index_of(label_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let value_index = batch
                .schema()
                .index_of(value_field)
                .map_err(|_| DatasetFrameError::InvalidMetadata)?;
            let label_formatter = ArrayFormatter::try_new(
                batch.column(label_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let value_formatter = ArrayFormatter::try_new(
                batch.column(value_index).as_ref(),
                &FormatOptions::default(),
            )
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let predicate_formatter = predicate_field
                .map(|field| {
                    let index = batch
                        .schema()
                        .index_of(field)
                        .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                    ArrayFormatter::try_new(batch.column(index).as_ref(), &FormatOptions::default())
                        .map_err(|error| DatasetFrameError::Decode(error.to_string()))
                })
                .transpose()?;

            for row_index in 0..batch.num_rows() {
                if let Some(formatter) = &predicate_formatter {
                    let predicate = formatter.value(row_index).to_string();
                    if !matches!(predicate.trim().to_ascii_lowercase().as_str(), "1" | "true") {
                        continue;
                    }
                }
                if eligible < range_start {
                    eligible += 1;
                    continue;
                }
                if eligible >= range_stop {
                    return Ok(Some((labels, values)));
                }
                eligible += 1;

                let label = label_formatter.value(row_index).to_string();
                let Ok(value) = value_formatter.value(row_index).to_string().parse::<f64>() else {
                    continue;
                };
                if label.trim().is_empty() || !value.is_finite() {
                    continue;
                }

                seen = seen.saturating_add(1);
                if labels.len() < max_points {
                    labels.push(label);
                    values.push(value);
                    continue;
                }
                let slot = seen.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17) % seen;
                if slot < max_points as u64 {
                    labels[slot as usize] = label;
                    values[slot as usize] = value;
                }
            }
        }
        Ok(Some((labels, values)))
    }

    /// Sample categorical rows once, then pivot them into a bounded matrix
    /// suitable for `gpui_px::BarChart::add_series`.
    ///
    /// Duplicate category/series cells are summed and missing cells are zero.
    /// The retained category-by-series cell count never exceeds `max_points`.
    pub fn sample_bar_series(
        &self,
        resource_id: &str,
        category_field: &str,
        value_field: &str,
        series_field: Option<&str>,
        color_field: Option<&str>,
        predicate_field: Option<&str>,
        row_range: Option<(usize, usize)>,
        max_points: usize,
    ) -> Result<Option<SampledBarData>, DatasetFrameError> {
        if max_points == 0
            || max_points > MAX_DATASET_CHART_POINTS
            || (series_field.is_none() && color_field.is_none())
            || row_range.is_some_and(|(start, stop)| start > stop)
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let (range_start, range_stop) = row_range.unwrap_or((0, usize::MAX));
        let mut sampled: Vec<(String, String, Option<String>, f64)> =
            Vec::with_capacity(max_points);
        let mut eligible = 0_usize;
        let mut seen = 0_u64;

        'batches: for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let formatter = |field: &str| -> Result<ArrayFormatter<'_>, DatasetFrameError> {
                let index = batch
                    .schema()
                    .index_of(field)
                    .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                ArrayFormatter::try_new(batch.column(index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
            };
            let category_formatter = formatter(category_field)?;
            let value_formatter = formatter(value_field)?;
            let series_formatter = series_field.map(&formatter).transpose()?;
            let color_formatter = color_field.map(&formatter).transpose()?;
            let predicate_formatter = predicate_field.map(&formatter).transpose()?;

            for row_index in 0..batch.num_rows() {
                if let Some(formatter) = &predicate_formatter {
                    let predicate = formatter.value(row_index).to_string();
                    if !matches!(predicate.trim().to_ascii_lowercase().as_str(), "1" | "true") {
                        continue;
                    }
                }
                if eligible < range_start {
                    eligible += 1;
                    continue;
                }
                if eligible >= range_stop {
                    break 'batches;
                }
                eligible += 1;

                let category = category_formatter.value(row_index).to_string();
                let Ok(value) = value_formatter.value(row_index).to_string().parse::<f64>() else {
                    continue;
                };
                if category.trim().is_empty() || !value.is_finite() {
                    continue;
                }
                let label = series_formatter
                    .as_ref()
                    .or(color_formatter.as_ref())
                    .map(|formatter| formatter.value(row_index).to_string())
                    .unwrap_or_else(|| "Series".into());
                if label.trim().is_empty() {
                    continue;
                }
                let color = color_formatter
                    .as_ref()
                    .map(|formatter| formatter.value(row_index).to_string())
                    .filter(|value| !value.trim().is_empty());
                let row = (category, label, color, value);
                seen = seen.saturating_add(1);
                if sampled.len() < max_points {
                    sampled.push(row);
                    continue;
                }
                let slot = seen.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17) % seen;
                if slot < max_points as u64 {
                    sampled[slot as usize] = row;
                }
            }
        }

        let mut categories = Vec::<String>::new();
        let mut labels = Vec::<String>::new();
        for (category, label, _, _) in &sampled {
            if !categories.contains(category) {
                categories.push(category.clone());
            }
            if !labels.contains(label) {
                labels.push(label.clone());
            }
        }
        if categories.is_empty() || labels.is_empty() {
            return Ok(Some(SampledBarData {
                categories,
                series: Vec::new(),
            }));
        }

        if categories.len().saturating_mul(labels.len()) > max_points {
            let series_cap = labels
                .len()
                .min((max_points as f64).sqrt().floor() as usize)
                .max(1);
            labels.truncate(series_cap);
            categories.truncate((max_points / labels.len()).max(1));
        }
        let category_indexes = categories
            .iter()
            .enumerate()
            .map(|(index, category)| (category.as_str(), index))
            .collect::<HashMap<_, _>>();
        let series_indexes = labels
            .iter()
            .enumerate()
            .map(|(index, label)| (label.as_str(), index))
            .collect::<HashMap<_, _>>();
        let mut values = vec![vec![0.0; categories.len()]; labels.len()];
        let mut colors = vec![None::<String>; labels.len()];
        for (category, label, color, value) in &sampled {
            let (Some(&category_index), Some(&series_index)) = (
                category_indexes.get(category.as_str()),
                series_indexes.get(label.as_str()),
            ) else {
                continue;
            };
            values[series_index][category_index] += value;
            if let Some(color) = color {
                if colors[series_index]
                    .as_ref()
                    .is_some_and(|existing| existing != color)
                {
                    return Err(DatasetFrameError::Decode(format!(
                        "bar color field must be constant within series {label:?}"
                    )));
                }
                colors[series_index] = Some(color.clone());
            }
        }

        Ok(Some(SampledBarData {
            categories,
            series: labels
                .into_iter()
                .zip(colors)
                .zip(values)
                .map(|((label, color), values)| SampledBarSeries {
                    label,
                    color,
                    values,
                })
                .collect(),
        }))
    }

    /// Sample numeric x/y rows once, then group the bounded sample into
    /// semantic series. The total number of retained points never exceeds the
    /// chart LOD budget, regardless of series cardinality.
    pub fn sample_xy_series(
        &self,
        resource_id: &str,
        x_field: &str,
        y_field: &str,
        series_field: Option<&str>,
        color_field: Option<&str>,
        key_field: Option<&str>,
        dash_field: Option<&str>,
        y0_field: Option<&str>,
        predicate_field: Option<&str>,
        row_range: Option<(usize, usize)>,
        max_points: usize,
    ) -> Result<Option<Vec<SampledXySeries>>, DatasetFrameError> {
        if max_points == 0
            || max_points > MAX_DATASET_CHART_POINTS
            || (series_field.is_none()
                && color_field.is_none()
                && key_field.is_none()
                && dash_field.is_none()
                && y0_field.is_none())
            || row_range.is_some_and(|(start, stop)| start > stop)
        {
            return Err(DatasetFrameError::InvalidMetadata);
        }
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let (range_start, range_stop) = row_range.unwrap_or((0, usize::MAX));
        let mut sampled: Vec<(
            f64,
            f64,
            Option<f64>,
            String,
            Option<String>,
            Option<String>,
            String,
        )> = Vec::with_capacity(max_points);
        let mut eligible = 0_usize;
        let mut seen = 0_u64;

        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let formatter = |field: &str| -> Result<ArrayFormatter<'_>, DatasetFrameError> {
                let index = batch
                    .schema()
                    .index_of(field)
                    .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                ArrayFormatter::try_new(batch.column(index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
            };
            let x_formatter = formatter(x_field)?;
            let y_formatter = formatter(y_field)?;
            let series_formatter = series_field.map(&formatter).transpose()?;
            let color_formatter = color_field.map(&formatter).transpose()?;
            let key_formatter = key_field.map(&formatter).transpose()?;
            let dash_formatter = dash_field.map(&formatter).transpose()?;
            let y0_formatter = y0_field.map(&formatter).transpose()?;
            let predicate_formatter = predicate_field.map(&formatter).transpose()?;

            for row_index in 0..batch.num_rows() {
                if let Some(formatter) = &predicate_formatter {
                    let predicate = formatter.value(row_index).to_string();
                    if !matches!(predicate.trim().to_ascii_lowercase().as_str(), "1" | "true") {
                        continue;
                    }
                }
                if eligible < range_start {
                    eligible += 1;
                    continue;
                }
                if eligible >= range_stop {
                    break;
                }
                eligible += 1;
                let (Ok(x), Ok(y)) = (
                    x_formatter.value(row_index).to_string().parse::<f64>(),
                    y_formatter.value(row_index).to_string().parse::<f64>(),
                ) else {
                    continue;
                };
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let y0 = y0_formatter
                    .as_ref()
                    .map(|formatter| formatter.value(row_index).to_string().parse::<f64>())
                    .transpose()
                    .ok()
                    .flatten();
                if y0_field.is_some() && y0.is_none_or(|value| !value.is_finite()) {
                    continue;
                }
                let label = series_formatter
                    .as_ref()
                    .or(color_formatter.as_ref())
                    .map(|formatter| formatter.value(row_index).to_string())
                    .unwrap_or_else(|| "Series".into());
                if label.trim().is_empty() {
                    continue;
                }
                let color = color_formatter
                    .as_ref()
                    .map(|formatter| formatter.value(row_index).to_string())
                    .filter(|value| !value.trim().is_empty());
                let key = key_formatter
                    .as_ref()
                    .map(|formatter| formatter.value(row_index).to_string())
                    .unwrap_or_default();
                let dash = validate_series_dash(
                    dash_formatter
                        .as_ref()
                        .map(|formatter| formatter.value(row_index).to_string())
                        .filter(|value| !value.trim().is_empty()),
                )?;
                let point = (x, y, y0, label, color, dash, key);
                seen = seen.saturating_add(1);
                if sampled.len() < max_points {
                    sampled.push(point);
                    continue;
                }
                let slot = seen.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17) % seen;
                if slot < max_points as u64 {
                    sampled[slot as usize] = point;
                }
            }
        }

        let mut output: Vec<SampledXySeries> = Vec::new();
        for (x, y, y0, label, color, dash, key) in sampled {
            if let Some(series) = output.iter_mut().find(|series| {
                series.label == label && series.color == color && series.dash == dash
            }) {
                series.x.push(x);
                series.y.push(y);
                if let Some(y0) = y0 {
                    series.y0.push(y0);
                }
                series.keys.push(key);
            } else {
                output.push(SampledXySeries {
                    label,
                    color,
                    dash,
                    x: vec![x],
                    y: vec![y],
                    y0: y0.into_iter().collect(),
                    keys: vec![key],
                });
            }
        }
        Ok(Some(output))
    }

    /// Decode a bounded hierarchy without flattening values into UI JSON.
    /// Hierarchies are not reservoir-sampled because dropping an ancestor can
    /// change their meaning; oversized inputs fail explicitly.
    pub fn treemap_rows(
        &self,
        resource_id: &str,
        id_field: &str,
        parent_field: &str,
        value_field: &str,
    ) -> Result<Option<Vec<TreemapResourceRow>>, DatasetFrameError> {
        let Some(frame) = self.frames.get(resource_id) else {
            return Ok(None);
        };
        let reader = StreamReader::try_new(Cursor::new(self.payload_bytes(frame)), None)
            .map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
        let mut rows = Vec::new();
        for batch in reader {
            let batch = batch.map_err(|error| DatasetFrameError::Decode(error.to_string()))?;
            let formatter = |field: &str| -> Result<ArrayFormatter<'_>, DatasetFrameError> {
                let index = batch
                    .schema()
                    .index_of(field)
                    .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                ArrayFormatter::try_new(batch.column(index).as_ref(), &FormatOptions::default())
                    .map_err(|error| DatasetFrameError::Decode(error.to_string()))
            };
            let id_formatter = formatter(id_field)?;
            let parent_formatter = formatter(parent_field)?;
            let value_formatter = formatter(value_field)?;
            for row_index in 0..batch.num_rows() {
                if rows.len() == MAX_DATASET_CHART_POINTS {
                    return Err(DatasetFrameError::TooLarge);
                }
                let id = id_formatter.value(row_index).to_string();
                let parent = parent_formatter.value(row_index).to_string();
                let value = value_formatter
                    .value(row_index)
                    .to_string()
                    .parse::<f64>()
                    .map_err(|_| DatasetFrameError::InvalidMetadata)?;
                if id.trim().is_empty() || !value.is_finite() || value < 0.0 {
                    return Err(DatasetFrameError::InvalidMetadata);
                }
                let parent = (!parent.trim().is_empty()
                    && !parent.trim().eq_ignore_ascii_case("null"))
                .then(|| parent.trim().to_owned());
                rows.push(TreemapResourceRow {
                    id: id.trim().to_owned(),
                    parent,
                    value,
                });
            }
        }
        Ok(Some(rows))
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.retired_frames.clear();
        self.mapped_payloads.clear();
        self.pending.clear();
        self.latest_generations.clear();
        self.references.clear();
        self.bytes_used = 0;
    }
    pub fn release(&mut self, resource_id: &str, generation: u64) -> bool {
        let key = (resource_id.to_owned(), generation);
        if self.references.contains_key(&key) {
            return false;
        }
        let mut released = false;
        if self
            .frames
            .get(resource_id)
            .is_some_and(|frame| frame.generation == generation)
        {
            let frame = self.frames.remove(resource_id).expect("frame exists");
            let bytes = self
                .mapped_payloads
                .remove(&key)
                .map_or(frame.payload.len(), |payload| payload.len());
            self.bytes_used = self.bytes_used.saturating_sub(bytes);
            released = true;
        }
        if let Some(frame) = self.retired_frames.remove(&key) {
            let bytes = self
                .mapped_payloads
                .remove(&key)
                .map_or(frame.payload.len(), |payload| payload.len());
            self.bytes_used = self.bytes_used.saturating_sub(bytes);
            released = true;
        }
        if self
            .pending
            .get(resource_id)
            .is_some_and(|pending| pending.generation == generation)
        {
            let pending = self.pending.remove(resource_id).expect("pending exists");
            self.bytes_used = self.bytes_used.saturating_sub(pending.held_bytes());
            released = true;
        }
        if self.latest_generations.get(resource_id).copied() == Some(generation) {
            self.latest_generations.remove(resource_id);
        }
        released
    }
}

/// Sample a dense numeric ArrayData payload without expanding it into JSON.
/// A one-dimensional array is plotted as `(index, value)`; a two-dimensional
/// array uses its first two columns as `(x, y)` points.
pub fn sample_dense_xy(
    payload: &[u8],
    shape: &[usize],
    dtype: &str,
    point_limit: usize,
) -> Result<(Vec<f64>, Vec<f64>), DatasetFrameError> {
    if point_limit == 0 {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    let (rows, stride, x_column, y_column) = match shape {
        [rows] => (*rows, 1, None, 0),
        [rows, columns] if *columns >= 2 => (*rows, *columns, Some(0), 1),
        _ => {
            return Err(DatasetFrameError::Decode(
                "ArrayData chart requires shape [N] or [N, >=2]".into(),
            ));
        }
    };
    let width = dense_dtype_width(dtype)?;
    let expected = rows
        .checked_mul(stride)
        .and_then(|values| values.checked_mul(width))
        .ok_or(DatasetFrameError::TooLarge)?;
    if payload.len() != expected {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    let step = (rows / point_limit).max(1);
    let mut x = Vec::with_capacity(rows.min(point_limit));
    let mut y = Vec::with_capacity(rows.min(point_limit));
    for row in (0..rows).step_by(step) {
        let y_value = dense_number(payload, dtype, row * stride + y_column)?;
        let x_value = match x_column {
            Some(column) => dense_number(payload, dtype, row * stride + column)?,
            None => row as f64,
        };
        if x_value.is_finite() && y_value.is_finite() {
            x.push(x_value);
            y.push(y_value);
        }
        if x.len() == point_limit {
            break;
        }
    }
    Ok((x, y))
}

/// Decode a bounded two-dimensional ArrayData grid for native raster charts.
/// The raw buffer stays outside UI JSON; this is the first unavoidable CPU
/// materialization before a chart's GPU upload.
pub fn dense_grid(
    payload: &[u8],
    shape: &[usize],
    dtype: &str,
) -> Result<(Vec<f64>, usize, usize), DatasetFrameError> {
    const MAX_GRID_VALUES: usize = 1_048_576;
    let [height, width] = shape else {
        return Err(DatasetFrameError::Decode(
            "ArrayData grid chart requires shape [height, width]".into(),
        ));
    };
    let values = height
        .checked_mul(*width)
        .ok_or(DatasetFrameError::TooLarge)?;
    if values > MAX_GRID_VALUES {
        return Err(DatasetFrameError::TooLarge);
    }
    let bytes = values
        .checked_mul(dense_dtype_width(dtype)?)
        .ok_or(DatasetFrameError::TooLarge)?;
    if payload.len() != bytes {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    let mut z = Vec::with_capacity(values);
    for index in 0..values {
        z.push(dense_number(payload, dtype, index)?);
    }
    Ok((z, *width, *height))
}

/// Decode a bounded dense numeric ArrayData payload while preserving its shape.
pub fn dense_array_values(
    payload: &[u8],
    shape: &[usize],
    dtype: &str,
) -> Result<Vec<f64>, DatasetFrameError> {
    const MAX_DENSE_VALUES: usize = 16_000_000;
    if shape.is_empty() || shape.iter().any(|dimension| *dimension == 0) {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    let count = shape
        .iter()
        .try_fold(1usize, |total, dimension| total.checked_mul(*dimension))
        .ok_or(DatasetFrameError::TooLarge)?;
    if count > MAX_DENSE_VALUES {
        return Err(DatasetFrameError::TooLarge);
    }
    let expected = count
        .checked_mul(dense_dtype_width(dtype)?)
        .ok_or(DatasetFrameError::TooLarge)?;
    if payload.len() != expected {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    (0..count)
        .map(|index| dense_number(payload, dtype, index))
        .collect()
}

/// Decode bounded unsigned ArrayData exactly, for triangle index buffers.
pub fn dense_array_unsigned(
    payload: &[u8],
    shape: &[usize],
    dtype: &str,
) -> Result<Vec<u64>, DatasetFrameError> {
    const MAX_DENSE_VALUES: usize = 16_000_000;
    if shape.is_empty() || shape.iter().any(|dimension| *dimension == 0) {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    let count = shape
        .iter()
        .try_fold(1usize, |total, dimension| total.checked_mul(*dimension))
        .ok_or(DatasetFrameError::TooLarge)?;
    if count > MAX_DENSE_VALUES {
        return Err(DatasetFrameError::TooLarge);
    }
    let width = dense_dtype_width(dtype)?;
    let expected = count
        .checked_mul(width)
        .ok_or(DatasetFrameError::TooLarge)?;
    if payload.len() != expected {
        return Err(DatasetFrameError::InvalidMetadata);
    }
    (0..count)
        .map(|index| {
            let start = index
                .checked_mul(width)
                .ok_or(DatasetFrameError::TooLarge)?;
            let bytes = payload
                .get(start..start + width)
                .ok_or(DatasetFrameError::InvalidMetadata)?;
            match dtype.to_ascii_lowercase().as_str() {
                "u8" | "uint8" => Ok(u64::from(bytes[0])),
                "u16" | "uint16" => Ok(u64::from(u16::from_le_bytes(
                    bytes.try_into().expect("validated u16 width"),
                ))),
                "u32" | "uint32" => Ok(u64::from(u32::from_le_bytes(
                    bytes.try_into().expect("validated u32 width"),
                ))),
                "u64" | "uint64" => Ok(u64::from_le_bytes(
                    bytes.try_into().expect("validated u64 width"),
                )),
                _ => Err(DatasetFrameError::Decode(
                    "ArrayData index dtype must be unsigned integer".into(),
                )),
            }
        })
        .collect()
}

pub(crate) fn dense_dtype_width(dtype: &str) -> Result<usize, DatasetFrameError> {
    match dtype.to_ascii_lowercase().as_str() {
        "u8" | "uint8" | "i8" | "int8" | "bool" => Ok(1),
        "u16" | "uint16" | "i16" | "int16" => Ok(2),
        "u32" | "uint32" | "i32" | "int32" | "f32" | "float32" => Ok(4),
        "u64" | "uint64" | "i64" | "int64" | "f64" | "float64" => Ok(8),
        "f16" | "float16" => Err(DatasetFrameError::Decode(
            "ArrayData f16 charts are not supported by this host".into(),
        )),
        _ => Err(DatasetFrameError::InvalidMetadata),
    }
}

pub(crate) fn dense_number(
    payload: &[u8],
    dtype: &str,
    index: usize,
) -> Result<f64, DatasetFrameError> {
    let width = dense_dtype_width(dtype)?;
    let start = index
        .checked_mul(width)
        .ok_or(DatasetFrameError::TooLarge)?;
    let bytes = payload
        .get(start..start + width)
        .ok_or(DatasetFrameError::InvalidMetadata)?;
    Ok(match dtype.to_ascii_lowercase().as_str() {
        "u8" | "uint8" => f64::from(bytes[0]),
        "i8" | "int8" => f64::from(bytes[0] as i8),
        "bool" => {
            if bytes[0] == 0 {
                0.0
            } else {
                1.0
            }
        }
        "u16" | "uint16" => f64::from(u16::from_le_bytes(bytes.try_into().unwrap())),
        "i16" | "int16" => f64::from(i16::from_le_bytes(bytes.try_into().unwrap())),
        "u32" | "uint32" => f64::from(u32::from_le_bytes(bytes.try_into().unwrap())),
        "i32" | "int32" => f64::from(i32::from_le_bytes(bytes.try_into().unwrap())),
        "f32" | "float32" => f64::from(f32::from_le_bytes(bytes.try_into().unwrap())),
        "u64" | "uint64" => u64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        "i64" | "int64" => i64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        "f64" | "float64" => f64::from_le_bytes(bytes.try_into().unwrap()),
        _ => return Err(DatasetFrameError::InvalidMetadata),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, BooleanArray, Int64Array, RecordBatch, StringArray};
    use arrow_ipc::writer::StreamWriter;
    use std::sync::Arc;
    fn frame(generation: u64, sequence: u32, chunk_count: u32, payload: &[u8]) -> DatasetFrame {
        DatasetFrame {
            resource_id: "events".into(),
            generation,
            sequence,
            chunk_count,
            byte_length: payload.len(),
            schema_fingerprint: "schema".into(),
            checksum: DatasetFrame::checksum(payload),
            payload: payload.to_vec(),
        }
    }
    #[test]
    fn frame_rejects_tampering() {
        let mut frame = frame(1, 0, 1, b"ARROW1");
        assert!(frame.validate().is_ok());
        frame.payload.push(0);
        frame.byte_length = frame.payload.len();
        assert_eq!(frame.validate(), Err(DatasetFrameError::ChecksumMismatch));
    }
    #[test]
    fn store_assembles_out_of_order_chunks_only_when_complete() {
        let mut store = DatasetFrameStore::default();
        assert!(!store.ingest(frame(2, 1, 3, b"world")).unwrap());
        assert!(store.get("events").is_none());
        assert!(!store.ingest(frame(2, 0, 3, b"hello ")).unwrap());
        assert!(store.ingest(frame(2, 2, 3, b"!")).unwrap());
        let result = store.get("events").unwrap();
        assert_eq!(result.payload, b"hello world!");
        assert_eq!(result.generation, 2);
        assert_eq!(result.chunk_count, 1);
        assert_eq!(store.stats().pending_resources, 0);
        assert_eq!(store.stats().bytes_used, b"hello world!".len());
    }
    #[test]
    fn store_rejects_conflicting_duplicate_and_metadata() {
        let mut store = DatasetFrameStore::default();
        assert!(!store.ingest(frame(1, 0, 2, b"first")).unwrap());
        assert!(!store.ingest(frame(1, 0, 2, b"first")).unwrap());
        assert_eq!(
            store.ingest(frame(1, 0, 2, b"other")),
            Err(DatasetFrameError::InvalidMetadata)
        );
        assert_eq!(
            store.ingest(frame(1, 1, 3, b"second")),
            Err(DatasetFrameError::InvalidMetadata)
        );
    }
    #[test]
    fn newer_generation_discards_pending_and_stale_frames() {
        let mut store = DatasetFrameStore::default();
        assert!(!store.ingest(frame(1, 0, 2, b"old")).unwrap());
        assert!(!store.ingest(frame(2, 0, 2, b"new")).unwrap());
        assert!(!store.ingest(frame(1, 1, 2, b"stale")).unwrap());
        assert!(store.ingest(frame(2, 1, 2, b"!")).unwrap());
        assert_eq!(store.get("events").unwrap().payload, b"new!");
    }
    #[test]
    fn release_removes_pending_and_completed_bytes() {
        let mut store = DatasetFrameStore::default();
        assert!(!store.ingest(frame(1, 0, 2, b"partial")).unwrap());
        store.release("events", 1);
        assert_eq!(store.stats().bytes_used, 0);
        assert!(store.ingest(frame(2, 0, 1, b"complete")).unwrap());
        store.release("events", 2);
        assert_eq!(store.stats().resources, 0);
        assert_eq!(store.stats().bytes_used, 0);
    }

    #[test]
    fn preview_reads_only_requested_arrow_rows_and_fields() {
        let batch = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(Int64Array::from(vec![1, 2, 3])) as ArrayRef),
            (
                "label",
                Arc::new(StringArray::from(vec!["one", "two", "three"])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let mut events_frame = frame(1, 0, 1, &payload);
        events_frame.schema_fingerprint = "arrow-schema".into();
        events_frame.checksum = DatasetFrame::checksum(&events_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(events_frame).unwrap());
        assert_eq!(
            store
                .preview_rows("events", &["label".into()], 1, 1)
                .unwrap(),
            Some(vec![vec!["two".into()]])
        );
        assert_eq!(
            store
                .preview_rows_sorted("events", &["label".into()], None, "id", true, 0, 2,)
                .unwrap(),
            Some(vec![vec!["three".into()], vec!["two".into()]])
        );
        let sampled = store.sample_xy("events", "id", "id", 2).unwrap().unwrap();
        assert_eq!(sampled.0.len(), 2);
        assert_eq!(sampled.0, sampled.1);
        let predicate_batch = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(Int64Array::from(vec![1, 2, 3])) as ArrayRef),
            (
                "enabled",
                Arc::new(BooleanArray::from(vec![true, false, true])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut predicate_payload = Vec::new();
        let mut writer =
            StreamWriter::try_new(&mut predicate_payload, &predicate_batch.schema()).unwrap();
        writer.write(&predicate_batch).unwrap();
        writer.finish().unwrap();
        let mut predicate_frame = frame(1, 0, 1, &predicate_payload);
        predicate_frame.schema_fingerprint = "predicate-schema".into();
        predicate_frame.checksum = DatasetFrame::checksum(&predicate_frame.payload);
        let mut predicate_store = DatasetFrameStore::default();
        assert!(predicate_store.ingest(predicate_frame).unwrap());
        let filtered = predicate_store
            .sample_xy_where_truthy("events", "id", "id", "enabled", 3)
            .unwrap()
            .unwrap();
        assert_eq!(filtered.0, vec![1.0, 3.0]);
        assert_eq!(filtered.0, filtered.1);
        let ranged = predicate_store
            .sample_xy_window("events", "id", "id", Some("enabled"), 1, 2, 3)
            .unwrap()
            .unwrap();
        assert_eq!(ranged.0, vec![3.0]);
        assert_eq!(ranged.0, ranged.1);
        assert_eq!(
            predicate_store
                .count_rows_where_truthy("events", "enabled")
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            predicate_store
                .preview_rows_where_truthy("events", &["id".into()], "enabled", 1, 1)
                .unwrap(),
            Some(vec![vec!["3".into()]])
        );
        assert_eq!(
            predicate_store
                .preview_rows_sorted("events", &["id".into()], Some("enabled"), "id", true, 0, 2,)
                .unwrap(),
            Some(vec![vec!["3".into()], vec!["1".into()]])
        );
        assert_eq!(
            store.preview_rows("events", &["missing".into()], 0, 1),
            Err(DatasetFrameError::InvalidMetadata)
        );
    }

    #[test]
    fn categorical_sampling_preserves_labels_filter_range_and_lod() {
        let batch = RecordBatch::try_from_iter(vec![
            (
                "category",
                Arc::new(StringArray::from(vec!["Low", "Mid", "High", "Ultra"])) as ArrayRef,
            ),
            (
                "value",
                Arc::new(Int64Array::from(vec![2, 5, 3, 7])) as ArrayRef,
            ),
            (
                "enabled",
                Arc::new(BooleanArray::from(vec![true, false, true, true])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let mut events_frame = frame(1, 0, 1, &payload);
        events_frame.schema_fingerprint = "categorical-schema".into();
        events_frame.checksum = DatasetFrame::checksum(&events_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(events_frame).unwrap());

        let sampled = store
            .sample_label_values(
                "events",
                "category",
                "value",
                Some("enabled"),
                Some((1, 3)),
                8,
            )
            .unwrap()
            .unwrap();
        assert_eq!(sampled.0, vec!["High", "Ultra"]);
        assert_eq!(sampled.1, vec![3.0, 7.0]);

        let bounded = store
            .sample_label_values("events", "category", "value", None, None, 2)
            .unwrap()
            .unwrap();
        assert_eq!(bounded.0.len(), 2);
        assert_eq!(bounded.0.len(), bounded.1.len());
    }

    #[test]
    fn bar_series_sampling_pivots_sums_and_fills_missing_cells() {
        let batch = RecordBatch::try_from_iter(vec![
            (
                "category",
                Arc::new(StringArray::from(vec!["A", "A", "B", "B", "A", "C"])) as ArrayRef,
            ),
            (
                "value",
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5, 7])) as ArrayRef,
            ),
            (
                "channel",
                Arc::new(StringArray::from(vec!["L", "R", "L", "R", "L", "L"])) as ArrayRef,
            ),
            (
                "color",
                Arc::new(StringArray::from(vec![
                    "#ff0000", "#00ff00", "#ff0000", "#00ff00", "#ff0000", "#ff0000",
                ])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let mut events_frame = frame(1, 0, 1, &payload);
        events_frame.schema_fingerprint = "bar-series-schema".into();
        events_frame.checksum = DatasetFrame::checksum(&events_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(events_frame).unwrap());

        let grouped = store
            .sample_bar_series(
                "events",
                "category",
                "value",
                Some("channel"),
                Some("color"),
                None,
                Some((0, 5)),
                8,
            )
            .unwrap()
            .unwrap();
        assert_eq!(grouped.categories, vec!["A", "B"]);
        assert_eq!(grouped.series.len(), 2);
        assert_eq!(grouped.series[0].label, "L");
        assert_eq!(grouped.series[0].color.as_deref(), Some("#ff0000"));
        assert_eq!(grouped.series[0].values, vec![6.0, 3.0]);
        assert_eq!(grouped.series[1].label, "R");
        assert_eq!(grouped.series[1].values, vec![2.0, 4.0]);

        let bounded = store
            .sample_bar_series(
                "events",
                "category",
                "value",
                Some("channel"),
                None,
                None,
                None,
                2,
            )
            .unwrap()
            .unwrap();
        assert!(bounded.categories.len() * bounded.series.len() <= 2);
    }

    #[test]
    fn series_sampling_groups_after_applying_global_lod_budget() {
        let batch = RecordBatch::try_from_iter(vec![
            (
                "id",
                Arc::new(Int64Array::from(vec![101, 102, 103, 104, 105, 106])) as ArrayRef,
            ),
            (
                "x",
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5, 6])) as ArrayRef,
            ),
            (
                "y",
                Arc::new(Int64Array::from(vec![10, 20, 30, 40, 50, 60])) as ArrayRef,
            ),
            (
                "channel",
                Arc::new(StringArray::from(vec!["L", "R", "L", "R", "L", "R"])) as ArrayRef,
            ),
            (
                "color",
                Arc::new(StringArray::from(vec![
                    "#ff0000", "#00ff00", "#ff0000", "#00ff00", "#ff0000", "#00ff00",
                ])) as ArrayRef,
            ),
            (
                "dash",
                Arc::new(StringArray::from(vec![
                    "solid", "dashed", "solid", "dashed", "solid", "dashed",
                ])) as ArrayRef,
            ),
            (
                "baseline",
                Arc::new(Int64Array::from(vec![0, 1, 2, 3, 4, 5])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let mut events_frame = frame(1, 0, 1, &payload);
        events_frame.schema_fingerprint = "series-schema".into();
        events_frame.checksum = DatasetFrame::checksum(&events_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(events_frame).unwrap());

        let series = store
            .sample_xy_series(
                "events",
                "x",
                "y",
                Some("channel"),
                Some("color"),
                Some("id"),
                Some("dash"),
                Some("baseline"),
                None,
                None,
                4,
            )
            .unwrap()
            .unwrap();
        assert_eq!(series.iter().map(|item| item.x.len()).sum::<usize>(), 4);
        assert_eq!(series.iter().map(|item| item.keys.len()).sum::<usize>(), 4);
        assert_eq!(series.iter().map(|item| item.y0.len()).sum::<usize>(), 4);
        assert!(
            series
                .iter()
                .flat_map(|item| item.keys.iter())
                .all(|key| !key.is_empty())
        );
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].label, "L");
        assert_eq!(series[0].color.as_deref(), Some("#ff0000"));
        assert_eq!(series[0].dash.as_deref(), Some("solid"));
        assert_eq!(series[1].label, "R");
        assert_eq!(series[1].color.as_deref(), Some("#00ff00"));
        assert_eq!(series[1].dash.as_deref(), Some("dashed"));
    }

    #[test]
    fn grouping_and_aggregation_execute_over_arrow_without_unbounded_output() {
        let batch = RecordBatch::try_from_iter(vec![
            (
                "channel",
                Arc::new(StringArray::from(vec!["L", "R", "L"])) as ArrayRef,
            ),
            (
                "value",
                Arc::new(Int64Array::from(vec![1, 2, 3])) as ArrayRef,
            ),
            (
                "label",
                Arc::new(StringArray::from(vec![Some("first"), None, Some("last")])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let mut dataset_frame = frame(1, 0, 1, &payload);
        dataset_frame.schema_fingerprint = "aggregate-schema".into();
        dataset_frame.checksum = DatasetFrame::checksum(&dataset_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(dataset_frame).unwrap());

        let filter = DatasetFilter::And(
            Box::new(DatasetFilter::In(
                Box::new(DatasetFilter::Field("channel".into())),
                vec![DatasetFilterValue::String("L".into())],
            )),
            Box::new(DatasetFilter::Gt(
                Box::new(DatasetFilter::Field("value".into())),
                Box::new(DatasetFilter::Literal(DatasetFilterValue::Number(1.0))),
            )),
        );
        assert_eq!(
            store.count_rows_filtered("events", &filter).unwrap(),
            Some(1)
        );
        assert_eq!(
            store
                .preview_rows_filtered("events", &["label".into()], &filter, 0, 8)
                .unwrap(),
            Some(vec![vec!["last".into()]])
        );
        let sampled = store
            .sample_filtered_rows(
                "events",
                &["channel".into(), "value".into(), "label".into()],
                &filter,
                None,
                8,
            )
            .unwrap()
            .unwrap();
        assert_eq!(sampled.total_rows, 1);
        assert_eq!(
            sampled.sample_label_values("label", "value").unwrap(),
            (vec!["last".into()], vec![3.0])
        );
        let null_filter = DatasetFilter::IsNull(Box::new(DatasetFilter::Field("label".into())));
        assert_eq!(
            store.count_rows_filtered("events", &null_filter).unwrap(),
            Some(1)
        );

        let aggregations = vec![
            DatasetAggregation {
                output: "rows".into(),
                operation: DatasetAggregationOp::Count,
                field: None,
            },
            DatasetAggregation {
                output: "labels".into(),
                operation: DatasetAggregationOp::Count,
                field: Some("label".into()),
            },
            DatasetAggregation {
                output: "total".into(),
                operation: DatasetAggregationOp::Sum,
                field: Some("value".into()),
            },
            DatasetAggregation {
                output: "mean".into(),
                operation: DatasetAggregationOp::Mean,
                field: Some("value".into()),
            },
            DatasetAggregation {
                output: "minimum".into(),
                operation: DatasetAggregationOp::Min,
                field: Some("value".into()),
            },
            DatasetAggregation {
                output: "maximum".into(),
                operation: DatasetAggregationOp::Max,
                field: Some("value".into()),
            },
            DatasetAggregation {
                output: "first".into(),
                operation: DatasetAggregationOp::First,
                field: Some("label".into()),
            },
            DatasetAggregation {
                output: "last".into(),
                operation: DatasetAggregationOp::Last,
                field: Some("label".into()),
            },
        ];
        let result = store
            .aggregate_rows(
                "events",
                &["channel".into()],
                &aggregations,
                None,
                Some("total"),
                true,
                0,
                1,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            result.fields,
            vec![
                "channel", "rows", "labels", "total", "mean", "minimum", "maximum", "first", "last"
            ]
        );
        assert_eq!(result.total_rows, 2);
        assert_eq!(
            result.rows,
            vec![vec!["L", "2", "2", "4", "2", "1", "3", "first", "last"]]
        );
        let filtered = store
            .aggregate_rows_filtered(
                "events",
                &["channel".into()],
                &aggregations,
                Some(&filter),
                None,
                false,
                0,
                8,
            )
            .unwrap()
            .unwrap();
        assert_eq!(filtered.rows[0][1], "1");
        assert_eq!(filtered.rows[0][3], "3");

        let full = store
            .aggregate_rows(
                "events",
                &["channel".into()],
                &aggregations,
                None,
                Some("total"),
                true,
                0,
                MAX_DATASET_CHART_POINTS,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            full.sample_label_values("channel", "total").unwrap(),
            (vec!["L".into(), "R".into()], vec![4.0, 2.0])
        );
        let xy = full
            .sample_xy_series("total", "mean", Some("channel"), None, None, None, None)
            .unwrap();
        assert_eq!(xy.len(), 2);
        assert_eq!(xy[0].label, "L");
        assert_eq!(xy[0].x, vec![4.0]);
        let bars = full
            .sample_bar_series("channel", "total", None, None)
            .unwrap();
        assert_eq!(bars.categories, vec!["L", "R"]);
        assert_eq!(bars.series[0].values, vec![4.0, 2.0]);
    }

    #[test]
    fn treemap_rows_preserve_parent_relationships() {
        let batch = RecordBatch::try_from_iter(vec![
            (
                "id",
                Arc::new(StringArray::from(vec!["root", "left", "right"])) as ArrayRef,
            ),
            (
                "parent",
                Arc::new(StringArray::from(vec![None, Some("root"), Some("root")])) as ArrayRef,
            ),
            (
                "value",
                Arc::new(Int64Array::from(vec![0, 2, 3])) as ArrayRef,
            ),
        ])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let mut events_frame = frame(1, 0, 1, &payload);
        events_frame.schema_fingerprint = "treemap-schema".into();
        events_frame.checksum = DatasetFrame::checksum(&events_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(events_frame).unwrap());

        let rows = store
            .treemap_rows("events", "id", "parent", "value")
            .unwrap()
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].parent, None);
        assert_eq!(rows[1].parent.as_deref(), Some("root"));
    }

    #[test]
    fn million_row_preview_materializes_only_the_requested_window() {
        let values = (0_i64..1_000_000).collect::<Vec<_>>();
        let batch = RecordBatch::try_from_iter(vec![(
            "id",
            Arc::new(Int64Array::from(values)) as ArrayRef,
        )])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        assert!(payload.len() < MAX_DATASET_FRAME_BYTES);

        let mut events_frame = frame(1, 0, 1, &payload);
        events_frame.schema_fingerprint = "million-row-schema".into();
        events_frame.checksum = DatasetFrame::checksum(&events_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(events_frame).unwrap());

        let rows = store
            .preview_rows("events", &["id".into()], 500_000, MAX_DATASET_PREVIEW_ROWS)
            .unwrap()
            .unwrap();
        assert_eq!(rows.len(), MAX_DATASET_PREVIEW_ROWS);
        assert_eq!(rows.first(), Some(&vec!["500000".into()]));
        assert_eq!(rows.last(), Some(&vec!["500511".into()]));
    }

    #[test]
    fn multi_million_point_chart_sampling_stays_at_lod_budget() {
        let values = (0_i64..2_000_000).collect::<Vec<_>>();
        let batch = RecordBatch::try_from_iter(vec![(
            "sample",
            Arc::new(Int64Array::from(values)) as ArrayRef,
        )])
        .unwrap();
        let mut payload = Vec::new();
        let mut writer = StreamWriter::try_new(&mut payload, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        assert!(payload.len() < MAX_DATASET_FRAME_BYTES);

        let mut chart_frame = frame(1, 0, 1, &payload);
        chart_frame.schema_fingerprint = "multi-million-chart-schema".into();
        chart_frame.checksum = DatasetFrame::checksum(&chart_frame.payload);
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest(chart_frame).unwrap());

        let (x, y) = store
            .sample_xy("events", "sample", "sample", 512)
            .unwrap()
            .unwrap();
        assert_eq!(x.len(), 512);
        assert_eq!(y.len(), 512);
        assert!(x.iter().chain(&y).all(|value| value.is_finite()));
    }

    #[test]
    fn mmap_generation_is_retained_without_copying_into_frame_payload() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("resource.bin");
        let payload = b"mapped-array-values";
        std::fs::write(&path, payload).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let mapped = MappedDatasetPayload::map_file(&path, payload.len()).unwrap();
        let frame = MappedDatasetFrame {
            resource_id: "array".into(),
            generation: 1,
            sequence: 0,
            chunk_count: 1,
            byte_length: payload.len(),
            schema_fingerprint: "array-schema".into(),
            checksum: DatasetFrame::checksum(payload),
            filename: "resource.bin".into(),
            session_token: "session".into(),
            payload: Some(Arc::new(mapped)),
        };
        let mut store = DatasetFrameStore::default();
        assert!(store.ingest_mapped(frame).unwrap());
        assert_eq!(store.raw_payload("array"), Some(payload.as_slice()));
        assert!(store.get("array").unwrap().payload.is_empty());
        assert_eq!(store.stats().bytes_used, payload.len());
        store.release("array", 1);
        assert_eq!(store.stats().bytes_used, 0);
        assert!(!path.exists());
    }

    #[test]
    fn mmap_arrow_generation_is_consumed_directly_by_table_window() {
        let batch = RecordBatch::try_from_iter(vec![(
            "value",
            Arc::new(Int64Array::from(vec![10, 20, 30])) as ArrayRef,
        )])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer = StreamWriter::try_new(&mut bytes, &batch.schema()).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("table.arrow");
        std::fs::write(&path, &bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let mapped = MappedDatasetPayload::map_file(&path, bytes.len()).unwrap();
        let mut store = DatasetFrameStore::default();
        assert!(
            store
                .ingest_mapped(MappedDatasetFrame {
                    resource_id: "table".into(),
                    generation: 1,
                    sequence: 0,
                    chunk_count: 1,
                    byte_length: bytes.len(),
                    schema_fingerprint: "table-schema".into(),
                    checksum: DatasetFrame::checksum(&bytes),
                    filename: "table.arrow".into(),
                    session_token: "session".into(),
                    payload: Some(Arc::new(mapped)),
                })
                .unwrap()
        );
        assert_eq!(
            store
                .preview_rows("table", &["value".into()], 1, 1)
                .unwrap()
                .unwrap(),
            vec![vec!["20".to_string()]]
        );
        assert!(store.get("table").unwrap().payload.is_empty());
    }

    #[test]
    fn leased_generation_survives_replacement_until_final_owner_releases() {
        fn mapped_frame(directory: &Path, generation: u64, payload: &[u8]) -> MappedDatasetFrame {
            let filename = format!("generation-{generation}.bin");
            let path = directory.join(&filename);
            std::fs::write(&path, payload).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
            MappedDatasetFrame {
                resource_id: "live-array".into(),
                generation,
                sequence: 0,
                chunk_count: 1,
                byte_length: payload.len(),
                schema_fingerprint: "stable-schema".into(),
                checksum: DatasetFrame::checksum(payload),
                filename,
                session_token: "session".into(),
                payload: Some(Arc::new(
                    MappedDatasetPayload::map_file(&path, payload.len()).unwrap(),
                )),
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let mut store = DatasetFrameStore::default();
        assert!(
            store
                .ingest_mapped(mapped_frame(directory.path(), 1, b"old"))
                .unwrap()
        );
        assert!(store.retain("live-array", 1));
        assert!(store.retain("live-array", 1));
        assert!(
            store
                .ingest_mapped(mapped_frame(directory.path(), 2, b"newer"))
                .unwrap()
        );
        assert_eq!(store.raw_payload("live-array"), Some(b"newer".as_slice()));
        assert_eq!(
            store.raw_payload_at("live-array", 1),
            Some(b"old".as_slice())
        );
        assert!(!store.release("live-array", 1));
        assert_eq!(store.stats().resources, 2);
        assert_eq!(store.stats().references, 2);
        assert!(store.release_reference("live-array", 1));
        assert!(store.raw_payload_at("live-array", 1).is_some());
        assert!(store.release_reference("live-array", 1));
        assert!(store.raw_payload_at("live-array", 1).is_none());
        assert_eq!(store.stats().resources, 1);
        assert_eq!(store.stats().references, 0);
        assert_eq!(store.stats().bytes_used, 5);
    }

    #[cfg(unix)]
    #[test]
    fn mmap_generation_rejects_group_readable_publications() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("resource.bin");
        std::fs::write(&path, b"private").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(matches!(
            MappedDatasetPayload::map_file(&path, 7),
            Err(DatasetFrameError::InsecureMappedFile)
        ));
    }
}
