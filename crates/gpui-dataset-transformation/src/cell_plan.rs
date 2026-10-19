//! Bounded cell-to-point preparation tied to one immutable validated snapshot.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::cell_mean::CellWeights;
use crate::convert::{converted_cell_field, insert_converted_field};
use gpui_mesh::{DataSet, Field, ValidatedDataSet};
use std::fmt;

/// Payload limits for prepared weights and the optional latest-field cache.
///
/// Limits exclude the borrowed source, returned datasets, allocator metadata and
/// temporary conversion scratch. Weight preparation checks simultaneous payload
/// before allocating, including bitmap-to-dense promotion. The output limit
/// bounds retained numeric/mask payload; it does not reject uncached outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellToPointLimits {
    /// Maximum simultaneous weight/bitmap payload bytes during preparation.
    pub max_geometry_bytes: usize,
    /// Maximum retained numeric/mask bytes for one most recently converted field.
    /// Zero disables this cache. A miss releases the previous cache reference.
    pub max_output_cache_bytes: usize,
}

impl Default for CellToPointLimits {
    fn default() -> Self {
        Self {
            // Bound reusable geometry to eight MiB without retaining output
            // buffers unless the caller explicitly chooses an output budget.
            max_geometry_bytes: 8 * 1024 * 1024,
            max_output_cache_bytes: 0,
        }
    }
}

struct CachedField {
    index: usize,
    field: Field,
    payload_bytes: usize,
}

/// Reuses geometric weights and optionally the latest converted cell field.
///
/// Each conversion starts from the same immutable source snapshot. Use
/// [`Self::convert_fields`] to keep multiple outputs in one result. Masks,
/// per-point dimension priority and exact weighted rounding match
/// [`crate::cell_data_to_point_data`]. No global cache or source-buffer copy is
/// created. Drop the plan before changing the source, then build a new plan.
///
/// # Examples
///
/// ```
/// use gpui_dataset_transformation::{CellToPointLimits, CellToPointPlan};
/// use gpui_mesh::{Association, DataSet, Field};
/// let data = DataSet::structured_grid("mesh", [2; 3], [0.0; 3], [1.0; 3])
///     .unwrap().with_field(Field::scalar("p", vec![2.0]).on_cells());
/// let mut plan = CellToPointPlan::with_limits(data.validated().unwrap(),
///     CellToPointLimits { max_output_cache_bytes: 1024, ..Default::default() })
///     .unwrap();
/// let out = plan.convert_field("p", "point_p").unwrap();
/// assert_eq!(out.field(Association::Point, "point_p").unwrap()
///     .as_scalar().unwrap(), &[2.0; 8]);
/// ```
///
/// Source mutation cannot invalidate a live plan:
///
/// ```compile_fail
/// use gpui_dataset_transformation::CellToPointPlan;
/// use gpui_mesh::{DataSet, Field};
/// let mut data = DataSet::structured_grid("mesh", [2; 3], [0.0; 3], [1.0; 3])
///     .unwrap().with_field(Field::scalar("p", vec![2.0]).on_cells());
/// let mut plan = CellToPointPlan::new(data.validated().unwrap()).unwrap();
/// std::sync::Arc::make_mut(&mut data.positions)[0][0] = 5.0;
/// let out = plan.convert_field("p", "mean").unwrap();
/// ```
pub struct CellToPointPlan<'a> {
    source: ValidatedDataSet<'a>,
    weights: CellWeights,
    limits: CellToPointLimits,
    cached: Option<CachedField>,
}

impl fmt::Debug for CellToPointPlan<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CellToPointPlan")
            .field("points", &self.source.as_dataset().point_count())
            .field("cells", &self.source.as_dataset().cell_count())
            .field("geometry_payload_bytes", &self.geometry_payload_bytes())
            .field(
                "output_cache_payload_bytes",
                &self.output_cache_payload_bytes(),
            )
            .finish_non_exhaustive()
    }
}

impl<'a> CellToPointPlan<'a> {
    /// Prepares weights with an eight-MiB budget and no output cache.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::ResourceLimit`] before a weight allocation
    /// would exceed the preparation budget.
    pub fn new(source: ValidatedDataSet<'a>) -> Result<Self, TransformError> {
        Self::with_limits(source, CellToPointLimits::default())
    }

    /// Prepares weights with caller-selected payload limits.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::ResourceLimit`] before a weight allocation
    /// would exceed `max_geometry_bytes`, or an error on byte-count overflow.
    pub fn with_limits(
        source: ValidatedDataSet<'a>,
        limits: CellToPointLimits,
    ) -> Result<Self, TransformError> {
        let weights =
            CellWeights::prepare_geometry(source.as_dataset(), limits.max_geometry_bytes)?;
        Ok(Self {
            source,
            weights,
            limits,
            cached: None,
        })
    }

    /// Converts one cell field, replacing a same-named point output.
    ///
    /// A fitting latest-field result is shared by subsequent calls, including
    /// calls with different output names. Returned payloads use copy-on-write
    /// ownership, so callers can change outputs without corrupting the cache.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::FieldNotFound`] for an absent cell field.
    pub fn convert_field(
        &mut self,
        field_name: &str,
        out_name: &str,
    ) -> Result<DataSet, TransformError> {
        let index = self.field_index(field_name)?;
        let mut field = self.converted(index);
        field.name = out_name.into();
        let mut output = self.source.as_dataset().clone();
        insert_converted_field(&mut output, field);
        Ok(output)
    }

    /// Converts multiple source cell fields using one prepared geometry.
    ///
    /// Pairs are `(source_name, output_name)`. Selection always reads the
    /// original snapshot. Output replacement follows pair order, so the last
    /// pair wins for repeated output names. All names are resolved before work;
    /// a missing field leaves the plan's output cache unchanged. Empty input
    /// returns a clone of the original dataset.
    ///
    /// # Errors
    ///
    /// Returns [`TransformError::FieldNotFound`] for an absent cell field.
    pub fn convert_fields(&mut self, fields: &[(&str, &str)]) -> Result<DataSet, TransformError> {
        for &(name, _) in fields {
            self.field_index(name)?;
        }
        let mut output = self.source.as_dataset().clone();
        for &(name, target) in fields {
            let mut field = self.converted(self.field_index(name)?);
            field.name = target.into();
            insert_converted_field(&mut output, field);
        }
        Ok(output)
    }

    /// Returns the retained prepared weight/bitmap payload size.
    #[must_use]
    pub fn geometry_payload_bytes(&self) -> usize {
        self.weights.payload_bytes()
    }

    /// Returns the latest-field cache's retained numeric/mask payload size.
    #[must_use]
    pub fn output_cache_payload_bytes(&self) -> usize {
        self.cached.as_ref().map_or(0, |entry| entry.payload_bytes)
    }

    /// Releases the plan's cached-output reference while retaining preparation.
    pub fn clear_output_cache(&mut self) {
        self.cached = None;
    }

    fn field_index(&self, name: &str) -> Result<usize, TransformError> {
        self.source
            .as_dataset()
            .cell_data
            .iter()
            .position(|field| field.name.as_ref() == name)
            .ok_or_else(|| TransformError::FieldNotFound(name.to_owned()))
    }

    fn converted(&mut self, index: usize) -> Field {
        if let Some(entry) = &self.cached
            && entry.index == index
        {
            return entry.field.clone();
        }
        self.cached = None;
        let dataset = self.source.as_dataset();
        let field = converted_cell_field(
            dataset,
            &dataset.cell_data[index],
            self.source.uniform_cell_dimension().is_some(),
            Some(&self.weights),
        );
        if self.limits.max_output_cache_bytes != 0 {
            let bytes = usize::try_from(field.data.byte_len())
                .ok()
                .and_then(|data| {
                    data.checked_add(field.valid.as_ref().map_or(0, |mask| mask.len()))
                });
            if let Some(payload_bytes) = bytes
                && payload_bytes <= self.limits.max_output_cache_bytes
            {
                self.cached = Some(CachedField {
                    index,
                    field: field.clone(),
                    payload_bytes,
                });
            }
        }
        field
    }
}

const fn assert_send_sync<T: Send + Sync>() {}
const _: () = assert_send_sync::<CellToPointPlan<'static>>();
