//! Dataset storage: positions, VTK-style cells, and fields.

// Rust guideline compliant 2026-02-21

use crate::field::{Association, Field, FieldData};
use std::sync::Arc;

/// Cell kinds supported by the dataset model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellType {
    /// Single point.
    Vertex,
    /// 2-node segment.
    Line,
    /// 3-node triangle.
    Triangle,
    /// 4-node quadrilateral.
    Quad,
    /// 4-node tetrahedron.
    Tetra,
    /// 8-node hexahedron.
    Hexahedron,
    /// 6-node wedge (triangular prism).
    Wedge,
    /// 5-node pyramid.
    Pyramid,
    /// Variable-node polygon (3 or more nodes, planar surface cell).
    Polygon,
}

impl CellType {
    /// Fixed node count for this cell kind.
    #[must_use]
    pub fn node_count(self) -> usize {
        match self {
            Self::Vertex => 1,
            Self::Line => 2,
            Self::Triangle => 3,
            Self::Quad => 4,
            Self::Tetra => 4,
            Self::Hexahedron => 8,
            Self::Wedge => 6,
            Self::Pyramid => 5,
            // Variable: validated as >= 3 nodes instead of a fixed count.
            Self::Polygon => 0,
        }
    }

    /// Whether this is a volume (3D) cell.
    #[must_use]
    pub fn is_volume(self) -> bool {
        matches!(
            self,
            Self::Tetra | Self::Hexahedron | Self::Wedge | Self::Pyramid
        )
    }

    /// Whether this is a surface (2D) cell.
    #[must_use]
    pub fn is_surface(self) -> bool {
        matches!(self, Self::Triangle | Self::Quad | Self::Polygon)
    }
}

/// VTK-style cell storage: per-cell types, offsets, and connectivity.
///
/// `offsets` has one more entry than the cell count; cell `i` occupies
/// `connectivity[offsets[i]..offsets[i+1]]`. Offsets are `u64` and
/// connectivity is `u32`, so a single dataset can address far more cells
/// and connectivity entries than fit in a single GPU buffer; GPU consumers
/// split the surface through [`crate::surface_chunks`].
#[derive(Debug, Clone, PartialEq)]
pub struct Cells {
    /// One type per cell.
    pub types: Arc<[CellType]>,
    /// Start offset of each cell in `connectivity`, plus a trailing end.
    pub offsets: Arc<[u64]>,
    /// Node indices into the dataset positions.
    pub connectivity: Arc<[u32]>,
}

impl Cells {
    /// Builds cells from per-cell node lists.
    pub fn from_cells(cells: &[(CellType, &[u32])]) -> Self {
        let mut types = Vec::with_capacity(cells.len());
        let mut offsets = Vec::with_capacity(cells.len() + 1);
        let mut connectivity = Vec::new();
        offsets.push(0u64);
        for (ty, nodes) in cells {
            types.push(*ty);
            connectivity.extend_from_slice(nodes);
            offsets.push(connectivity.len() as u64);
        }
        Self {
            types: types.into(),
            offsets: offsets.into(),
            connectivity: connectivity.into(),
        }
    }

    /// Convenience constructor for an all-triangle surface.
    pub fn triangles(triangles: &[[u32; 3]]) -> Self {
        let mut offsets = Vec::with_capacity(triangles.len() + 1);
        let mut connectivity = Vec::with_capacity(triangles.len() * 3);
        offsets.push(0u64);
        for tri in triangles {
            connectivity.extend_from_slice(tri);
            offsets.push(connectivity.len() as u64);
        }
        Self {
            types: vec![CellType::Triangle; triangles.len()].into(),
            offsets: offsets.into(),
            connectivity: connectivity.into(),
        }
    }

    /// Number of cells.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether there are no cells.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Node indices of cell `index`, when in range.
    #[must_use]
    pub fn cell(&self, index: usize) -> Option<&[u32]> {
        let start = *self.offsets.get(index)? as usize;
        let end = *self.offsets.get(index + 1)? as usize;
        self.connectivity.get(start..end)
    }
}

/// Axis-aligned bounds in dataset coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Minimum corner.
    pub min: [f64; 3],
    /// Maximum corner.
    pub max: [f64; 3],
}

impl Bounds {
    /// Centre of the bounds.
    #[must_use]
    pub fn center(&self) -> [f64; 3] {
        [
            f64::midpoint(self.min[0], self.max[0]),
            f64::midpoint(self.min[1], self.max[1]),
            f64::midpoint(self.min[2], self.max[2]),
        ]
    }

    /// Largest extent over the three axes; zero for a single point.
    #[must_use]
    pub fn diagonal(&self) -> f64 {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        let dz = self.max[2] - self.min[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Broad dataset category, derived from its cell types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSetKind {
    /// Only vertex/line/triangle/quad cells.
    Surface,
    /// At least one volume cell.
    Volume,
    /// No cells.
    Empty,
}

/// Validation failures for datasets and fields.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeshError {
    /// Time-series steps are inconsistent with each other.
    #[error("invalid time series: {reason}")]
    InvalidTimeSeries {
        /// What is inconsistent.
        reason: String,
    },
    /// Positions must not be empty.
    #[error("positions array is empty")]
    EmptyPositions,
    /// Every position component must be finite.
    #[error("position {index} is not finite")]
    NonFinitePosition {
        /// Index of the offending position.
        index: usize,
    },
    /// Offsets must hold exactly one more entry than the cell count.
    #[error("offsets length {offsets} != cells length {cells} + 1")]
    OffsetsLengthMismatch {
        /// Provided offsets length.
        offsets: usize,
        /// Cell count.
        cells: usize,
    },
    /// Offsets must start at zero, increase monotonically, and end at the connectivity length.
    #[error("offsets are not a valid partition of the connectivity array")]
    InvalidOffsets,
    /// A cell's node count must match its declared type.
    #[error("cell {cell} has {nodes} nodes, expected {expected} for {kind:?}")]
    CellNodeCountMismatch {
        /// Cell index.
        cell: usize,
        /// Actual node count.
        nodes: usize,
        /// Expected node count.
        expected: usize,
        /// Declared cell kind.
        kind: CellType,
    },
    /// Connectivity indices must address existing positions.
    #[error("connectivity index {index} out of range for {positions} positions")]
    IndexOutOfRange {
        /// Offending index value.
        index: u32,
        /// Number of positions.
        positions: usize,
    },
    /// Field length must match its association target.
    #[error("field '{field}' has {values} values, expected {expected} for {association:?}")]
    FieldLengthMismatch {
        /// Field name.
        field: String,
        /// Provided value count.
        values: usize,
        /// Expected value count.
        expected: usize,
        /// Field association.
        association: Association,
    },
    /// A validity mask must match the field length.
    #[error("field '{field}' validity mask length {mask} != values length {values}")]
    MaskLengthMismatch {
        /// Field name.
        field: String,
        /// Mask length.
        mask: usize,
        /// Value count.
        values: usize,
    },
    /// Unmasked values must be finite; masked entries may be NaN only.
    #[error("field '{field}' value {index} is not finite and not masked")]
    NonFiniteValue {
        /// Field name.
        field: String,
        /// Value index.
        index: usize,
    },
    /// Structured-grid dimensions must each be at least one.
    #[error("structured grid dimensions must be >= 1, got {dims:?}")]
    InvalidDims {
        /// Provided dimensions.
        dims: [usize; 3],
    },
}

/// One dataset: positions, cells, and named point/cell fields.
#[derive(Debug, Clone, PartialEq)]
pub struct DataSet {
    /// Stable dataset identifier.
    pub id: Arc<str>,
    /// Point coordinates, `f64` at the boundary.
    pub positions: Arc<[[f64; 3]]>,
    /// Cell storage.
    pub cells: Cells,
    /// Fields stored per point.
    pub point_data: Vec<Field>,
    /// Fields stored per cell.
    pub cell_data: Vec<Field>,
}

impl DataSet {
    /// Creates a dataset from raw parts. Call [`DataSet::validate`] before use.
    pub fn new(
        id: impl Into<Arc<str>>,
        positions: impl Into<Arc<[[f64; 3]]>>,
        cells: Cells,
    ) -> Self {
        Self {
            id: id.into(),
            positions: positions.into(),
            cells,
            point_data: Vec::new(),
            cell_data: Vec::new(),
        }
    }

    /// Creates a rectilinear grid: per-axis coordinates, structured
    /// hexahedral connectivity (x-fastest point order).
    ///
    /// # Errors
    ///
    /// Returns an error when any axis is empty.
    pub fn rectilinear_grid(
        id: impl Into<Arc<str>>,
        xs: &[f64],
        ys: &[f64],
        zs: &[f64],
    ) -> Result<Self, MeshError> {
        let dims = [xs.len(), ys.len(), zs.len()];
        if dims.contains(&0) {
            return Err(MeshError::InvalidDims { dims });
        }
        let mut positions = Vec::with_capacity(dims[0] * dims[1] * dims[2]);
        for &z in zs {
            for &y in ys {
                for &x in xs {
                    positions.push([x, y, z]);
                }
            }
        }
        let cells = structured_cells(dims);
        let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
        Ok(Self::new(id, positions, Cells::from_cells(&refs)))
    }

    /// Creates a curvilinear structured grid from explicit points.
    ///
    /// `positions` must hold exactly `dims[0] * dims[1] * dims[2]` points
    /// in x-fastest order; connectivity is the structured hexahedral
    /// layout, positions arbitrary (warped grids, body-fitted meshes).
    ///
    /// # Errors
    ///
    /// Returns an error on empty dimensions or a point-count mismatch.
    pub fn curvilinear_grid(
        id: impl Into<Arc<str>>,
        dims: [usize; 3],
        positions: Vec<[f64; 3]>,
    ) -> Result<Self, MeshError> {
        if dims.contains(&0) {
            return Err(MeshError::InvalidDims { dims });
        }
        if positions.len() != dims[0] * dims[1] * dims[2] {
            return Err(MeshError::InvalidDims { dims });
        }
        let cells = structured_cells(dims);
        let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
        Ok(Self::new(id, positions, Cells::from_cells(&refs)))
    }

    /// Creates a triangle-surface dataset.
    pub fn surface(
        id: impl Into<Arc<str>>,
        positions: impl Into<Arc<[[f64; 3]]>>,
        triangles: &[[u32; 3]],
    ) -> Self {
        Self::new(id, positions, Cells::triangles(triangles))
    }

    /// Creates a structured hexahedral grid from dimensions and an origin/spacing mapping.
    ///
    /// Points are ordered x-fastest. A dimension of 1 yields a surface or line grid
    /// topologically; hexahedra are only emitted when all dimensions exceed 1.
    pub fn structured_grid(
        id: impl Into<Arc<str>>,
        dims: [usize; 3],
        origin: [f64; 3],
        spacing: [f64; 3],
    ) -> Result<Self, MeshError> {
        if dims.contains(&0) {
            return Err(MeshError::InvalidDims { dims });
        }
        let [nx, ny, nz] = dims;
        let mut positions = Vec::with_capacity(nx * ny * nz);
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    positions.push([
                        origin[0] + i as f64 * spacing[0],
                        origin[1] + j as f64 * spacing[1],
                        origin[2] + k as f64 * spacing[2],
                    ]);
                }
            }
        }
        let idx = |i: usize, j: usize, k: usize| (k * ny * nx + j * nx + i) as u32;
        let mut cells: Vec<(CellType, [u32; 8])> = Vec::new();
        if nx > 1 && ny > 1 && nz > 1 {
            for k in 0..nz - 1 {
                for j in 0..ny - 1 {
                    for i in 0..nx - 1 {
                        cells.push((
                            CellType::Hexahedron,
                            [
                                idx(i, j, k),
                                idx(i + 1, j, k),
                                idx(i + 1, j + 1, k),
                                idx(i, j + 1, k),
                                idx(i, j, k + 1),
                                idx(i + 1, j, k + 1),
                                idx(i + 1, j + 1, k + 1),
                                idx(i, j + 1, k + 1),
                            ],
                        ));
                    }
                }
            }
        }
        let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
        Ok(Self::new(id, positions, Cells::from_cells(&refs)))
    }

    /// Adds a field, routing it by association. Returns `self` for chaining.
    #[must_use]
    pub fn with_field(mut self, field: Field) -> Self {
        match field.association {
            Association::Point => self.point_data.push(field),
            Association::Cell => self.cell_data.push(field),
        }
        self
    }

    /// Looks up a point or cell field by name.
    #[must_use]
    pub fn field(&self, association: Association, name: &str) -> Option<&Field> {
        let fields = match association {
            Association::Point => &self.point_data,
            Association::Cell => &self.cell_data,
        };
        fields.iter().find(|f| f.name.as_ref() == name)
    }

    /// Number of points.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of cells.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Surface or volume classification from cell types.
    #[must_use]
    pub fn kind(&self) -> DataSetKind {
        if self.cells.is_empty() {
            return DataSetKind::Empty;
        }
        if self.cells.types.iter().any(|t| t.is_volume()) {
            DataSetKind::Volume
        } else {
            DataSetKind::Surface
        }
    }

    /// Axis-aligned bounds, or `None` when there are no positions.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        let first = *self.positions.first()?;
        let mut bounds = Bounds {
            min: first,
            max: first,
        };
        for p in &self.positions[1..] {
            for (axis, &value) in p.iter().enumerate() {
                bounds.min[axis] = bounds.min[axis].min(value);
                bounds.max[axis] = bounds.max[axis].max(value);
            }
        }
        Some(bounds)
    }

    /// Checked total byte size of positions, cells, and fields.
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        let mut total = (self.positions.len() as u64) * 24;
        total += (self.cells.types.len() as u64) * 8;
        total += (self.cells.offsets.len() as u64) * 8;
        total += (self.cells.connectivity.len() as u64) * 4;
        for field in self.point_data.iter().chain(&self.cell_data) {
            total += field.data.byte_len();
        }
        total
    }

    /// Validates positions, cells, and every field.
    ///
    /// Empty filter results are valid when they contain no cells and all
    /// point/cell fields are empty. Their bounds are `None`.
    ///
    /// # Errors
    ///
    /// Returns the first [`MeshError`] encountered.
    pub fn validate(&self) -> Result<(), MeshError> {
        if self.positions.is_empty() && !self.cells.is_empty() {
            return Err(MeshError::EmptyPositions);
        }
        for (index, p) in self.positions.iter().enumerate() {
            if !p.iter().all(|c| c.is_finite()) {
                return Err(MeshError::NonFinitePosition { index });
            }
        }
        if self.cells.offsets.len() != self.cells.types.len() + 1 {
            return Err(MeshError::OffsetsLengthMismatch {
                offsets: self.cells.offsets.len(),
                cells: self.cells.types.len(),
            });
        }
        if self.cells.offsets.first() != Some(&0)
            || self.cells.offsets.last() != Some(&(self.cells.connectivity.len() as u64))
            || self.cells.offsets.windows(2).any(|w| w[1] < w[0])
        {
            return Err(MeshError::InvalidOffsets);
        }
        for (cell, ty) in self.cells.types.iter().enumerate() {
            let nodes = self.cells.cell(cell).unwrap_or(&[]);
            let bad_len = if *ty == CellType::Polygon {
                nodes.len() < 3
            } else {
                nodes.len() != ty.node_count()
            };
            if bad_len {
                return Err(MeshError::CellNodeCountMismatch {
                    cell,
                    nodes: nodes.len(),
                    expected: ty.node_count(),
                    kind: *ty,
                });
            }
            for &index in nodes {
                if index as usize >= self.positions.len() {
                    return Err(MeshError::IndexOutOfRange {
                        index,
                        positions: self.positions.len(),
                    });
                }
            }
        }
        for field in self.point_data.iter().chain(&self.cell_data) {
            self.validate_field(field)?;
        }
        Ok(())
    }

    fn validate_field(&self, field: &Field) -> Result<(), MeshError> {
        let expected = match field.association {
            Association::Point => self.positions.len(),
            Association::Cell => self.cells.len(),
        };
        if field.data.len() != expected {
            return Err(MeshError::FieldLengthMismatch {
                field: field.name.to_string(),
                values: field.data.len(),
                expected,
                association: field.association,
            });
        }
        if let Some(mask) = &field.valid
            && mask.len() != field.data.len()
        {
            return Err(MeshError::MaskLengthMismatch {
                field: field.name.to_string(),
                mask: mask.len(),
                values: field.data.len(),
            });
        }
        let masked = |i: usize| field.valid.as_ref().is_some_and(|m| !m[i]);
        let check = |index: usize, value: f64, field: &Field| -> Result<(), MeshError> {
            if value.is_infinite() || (value.is_nan() && !masked(index)) {
                return Err(MeshError::NonFiniteValue {
                    field: field.name.to_string(),
                    index,
                });
            }
            Ok(())
        };
        match &field.data {
            FieldData::Scalar(values) => {
                for (i, v) in values.iter().enumerate() {
                    check(i, *v, field)?;
                }
            }
            FieldData::Vector(values) => {
                for (i, v) in values.iter().enumerate() {
                    for c in v {
                        check(i, *c, field)?;
                    }
                }
            }
            FieldData::Tensor(values) => {
                for (i, v) in values.iter().enumerate() {
                    for c in v {
                        check(i, *c, field)?;
                    }
                }
            }
            FieldData::Complex(values) => {
                for (i, v) in values.iter().enumerate() {
                    for c in v {
                        check(i, *c, field)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_dataset_validates_and_bounds() {
        let ds = DataSet::surface(
            "tri",
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::scalar("pressure", vec![0.0, 1.0, 0.5]).labelled("Pressure", "Pa"));
        assert_eq!(ds.validate(), Ok(()));
        assert_eq!(ds.kind(), DataSetKind::Surface);
        assert_eq!(ds.bounds().unwrap().max, [1.0, 1.0, 0.0]);
        assert_eq!(
            ds.field(Association::Point, "pressure")
                .unwrap()
                .unit
                .as_deref(),
            Some("Pa")
        );
    }

    #[test]
    fn structured_grid_emits_hexahedra() {
        let ds = DataSet::structured_grid("grid", [2, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        assert_eq!(ds.point_count(), 8);
        assert_eq!(ds.cell_count(), 1);
        assert_eq!(ds.kind(), DataSetKind::Volume);
        assert_eq!(ds.validate(), Ok(()));
    }

    #[test]
    fn rejects_bad_index_and_field_length() {
        let bad = DataSet::surface(
            "bad",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 9]],
        );
        assert!(matches!(
            bad.validate(),
            Err(MeshError::IndexOutOfRange { .. })
        ));
        let bad_field = DataSet::surface(
            "bad2",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::scalar("p", vec![1.0]));
        assert!(matches!(
            bad_field.validate(),
            Err(MeshError::FieldLengthMismatch { .. })
        ));
    }
}

/// Structured hexahedral connectivity for `dims` (x-fastest points).
fn structured_cells(dims: [usize; 3]) -> Vec<(CellType, [u32; 8])> {
    let [nx, ny, nz] = dims;
    let idx = |i: usize, j: usize, k: usize| (k * ny * nx + j * nx + i) as u32;
    let mut cells = Vec::new();
    if nx > 1 && ny > 1 && nz > 1 {
        for k in 0..nz - 1 {
            for j in 0..ny - 1 {
                for i in 0..nx - 1 {
                    cells.push((
                        CellType::Hexahedron,
                        [
                            idx(i, j, k),
                            idx(i + 1, j, k),
                            idx(i + 1, j + 1, k),
                            idx(i, j + 1, k),
                            idx(i, j, k + 1),
                            idx(i + 1, j, k + 1),
                            idx(i + 1, j + 1, k + 1),
                            idx(i, j + 1, k + 1),
                        ],
                    ));
                }
            }
        }
    }
    cells
}
