//! Checked grid construction without intermediate cell tuples or reference arrays.

// Rust guideline compliant 2026-02-21

use crate::{CellType, Cells, DataSet, MeshError};
use std::sync::Arc;

/// Bounds planned simultaneous array storage during grid construction.
///
/// Counts element bytes, including Vec-to-Arc conversion overlap. It excludes
/// allocator/Arc headers, rounding, identifiers, borrowed coordinate axes and
/// later-added fields. Curvilinear input vector capacity is included. This is
/// neither an RSS bound nor a guarantee against allocation failure. Constrained
/// hosts should lower the default to their grid share of the total budget.
/// Vector reservations return allocation errors; final Arc allocations retain
/// Rust's standard infallible allocation behavior.
///
/// # Examples
///
/// ```
/// use gpui_mesh::{DataSet, GridBuildLimits};
/// let limits = GridBuildLimits { max_working_bytes: 32 * 1024 * 1024 };
/// let grid = DataSet::structured_grid_with_limits("grid", [32; 3], [0.0; 3], [1.0; 3], limits)?;
/// grid.validate()?;
/// # Ok::<(), gpui_mesh::MeshError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridBuildLimits {
    /// Maximum planned simultaneous grid-array element bytes.
    pub max_working_bytes: u64,
}

impl Default for GridBuildLimits {
    fn default() -> Self {
        Self {
            // Reject accidental giant builds while allowing existing 96-cubed
            // workloads. Hosts can explicitly choose a smaller/larger budget.
            max_working_bytes: 256 * 1024 * 1024,
        }
    }
}

struct GridPlan {
    cell_dims: [usize; 3],
    points: usize,
    count: usize,
    entries: usize,
    kind: CellType,
    strides: [usize; 3],
    active: [usize; 3],
    limits: GridBuildLimits,
    peak_without_positions_conversion: u64,
}

fn limit(resource: &'static str, requested: u64, limit: u64) -> MeshError {
    MeshError::ResourceLimit {
        resource,
        requested,
        limit,
    }
}

fn array_bytes<T>(length: usize) -> Result<u64, MeshError> {
    // Keep address headroom for the Arc counters/alignment on conversion.
    // All grid arrays have primitive elements with alignment <= eight bytes.
    let address_limit = isize::MAX as u64 - 64;
    let bytes = (length as u64)
        .checked_mul(size_of::<T>() as u64)
        .ok_or_else(|| limit("grid array bytes", u64::MAX, address_limit))?;
    if bytes > address_limit {
        return Err(limit("grid array bytes", bytes, address_limit));
    }
    Ok(bytes)
}

fn reserve<T>(length: usize) -> Result<Vec<T>, MeshError> {
    let bytes = array_bytes::<T>(length)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(length)
        .map_err(|_| MeshError::Allocation { bytes })?;
    Ok(result)
}

impl GridPlan {
    fn new(dims: [usize; 3], limits: GridBuildLimits) -> Result<Self, MeshError> {
        if dims.contains(&0) {
            return Err(MeshError::InvalidDims { dims });
        }
        let point_limit = (u64::from(u32::MAX) + 1).min(usize::MAX as u64);
        let points = dims
            .into_iter()
            .try_fold(1u64, |a, b| a.checked_mul(b as u64))
            .ok_or_else(|| limit("grid point count", u64::MAX, point_limit))?;
        if points > point_limit {
            return Err(limit("grid point count", points, point_limit));
        }
        let points = points as usize;
        let cell_dims = dims.map(|n| if n > 1 { n - 1 } else { 1 });
        // Each cell dimension is <= its point dimension; their product fits.
        let count = cell_dims.into_iter().product::<usize>();
        let mut active = [0; 3];
        let mut dimension = 0;
        for (axis, &n) in dims.iter().enumerate() {
            if n > 1 {
                active[dimension] = axis;
                dimension += 1;
            }
        }
        let kind = [
            CellType::Vertex,
            CellType::Line,
            CellType::Quad,
            CellType::Hexahedron,
        ][dimension];
        let entries = count
            .checked_mul(kind.node_count())
            .ok_or_else(|| limit("grid connectivity entries", u64::MAX, usize::MAX as u64))?;
        let offsets = count
            .checked_add(1)
            .ok_or_else(|| limit("grid offset count", u64::MAX, usize::MAX as u64))?;
        let position_bytes = array_bytes::<[f64; 3]>(points)?;
        let type_bytes = array_bytes::<CellType>(count)?;
        let offset_bytes = array_bytes::<u64>(offsets)?;
        let connectivity_bytes = array_bytes::<u32>(entries)?;
        let cell_bytes = type_bytes
            .checked_add(offset_bytes)
            .and_then(|n| n.checked_add(connectivity_bytes))
            .ok_or_else(|| {
                limit(
                    "grid array workspace bytes",
                    u64::MAX,
                    limits.max_working_bytes,
                )
            })?;
        let peak = position_bytes
            .checked_add(cell_bytes)
            .and_then(|n| n.checked_add(type_bytes.max(offset_bytes).max(connectivity_bytes)))
            .ok_or_else(|| {
                limit(
                    "grid array workspace bytes",
                    u64::MAX,
                    limits.max_working_bytes,
                )
            })?;
        let plan = Self {
            cell_dims,
            points,
            count,
            entries,
            kind,
            strides: [1, dims[0], dims[0] * dims[1]],
            active,
            limits,
            peak_without_positions_conversion: peak,
        };
        plan.check_position_capacity(points)?;
        Ok(plan)
    }

    fn check_position_capacity(&self, capacity: usize) -> Result<(), MeshError> {
        let conversion = array_bytes::<[f64; 3]>(capacity)?
            .checked_add(array_bytes::<[f64; 3]>(self.points)?)
            .ok_or_else(|| {
                limit(
                    "grid array workspace bytes",
                    u64::MAX,
                    self.limits.max_working_bytes,
                )
            })?;
        let requested = conversion.max(self.peak_without_positions_conversion);
        if requested > self.limits.max_working_bytes {
            return Err(limit(
                "grid array workspace bytes",
                requested,
                self.limits.max_working_bytes,
            ));
        }
        Ok(())
    }

    fn cells(&self) -> Result<Cells, MeshError> {
        let mut types = reserve::<CellType>(self.count)?;
        types.resize(self.count, self.kind);
        let mut offsets = reserve::<u64>(self.count + 1)?;
        let arity = self.kind.node_count();
        offsets.extend((0..=self.count).map(|i| (i * arity) as u64));
        let mut connectivity = reserve::<u32>(self.entries)?;
        for k in 0..self.cell_dims[2] {
            for j in 0..self.cell_dims[1] {
                for i in 0..self.cell_dims[0] {
                    let p = k * self.strides[2] + j * self.strides[1] + i;
                    let (nodes, len) = match self.kind {
                        CellType::Vertex => ([p; 8], 1),
                        CellType::Line => {
                            let a = self.strides[self.active[0]];
                            ([p, p + a, 0, 0, 0, 0, 0, 0], 2)
                        }
                        CellType::Quad => {
                            let a = self.strides[self.active[0]];
                            let b = self.strides[self.active[1]];
                            ([p, p + a, p + a + b, p + b, 0, 0, 0, 0], 4)
                        }
                        CellType::Hexahedron => {
                            let row = self.strides[1];
                            let plane = self.strides[2];
                            (
                                [
                                    p,
                                    p + 1,
                                    p + row + 1,
                                    p + row,
                                    p + plane,
                                    p + plane + 1,
                                    p + plane + row + 1,
                                    p + plane + row,
                                ],
                                8,
                            )
                        }
                        _ => unreachable!("grid cell dimension"),
                    };
                    // Point count and every stride were checked before allocation.
                    connectivity.extend(nodes[..len].iter().map(|&node| node as u32));
                }
            }
        }
        debug_assert_eq!(connectivity.len(), self.entries);
        Ok(Cells {
            types: types.into(),
            offsets: offsets.into(),
            connectivity: connectivity.into(),
        })
    }

    fn dataset(
        &self,
        id: impl Into<Arc<str>>,
        positions: Vec<[f64; 3]>,
    ) -> Result<DataSet, MeshError> {
        self.check_position_capacity(positions.capacity())?;
        // Convert positions before constructing cells so those two conversion
        // working sets cannot overlap. Each cell array is converted once.
        let positions: Arc<[[f64; 3]]> = positions.into();
        let cells = self.cells()?;
        Ok(DataSet::new(id, positions, cells))
    }
}

impl DataSet {
    /// Creates an affine structured grid with the default construction budget.
    ///
    /// Points and cells are x-fastest. Active axes select hexahedra, quads,
    /// lines or one vertex; quad corners follow increasing active-axis order.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions, unaddressable sizes, budget excess,
    /// nonfinite coordinates or a failed buffer reservation.
    pub fn structured_grid(
        id: impl Into<Arc<str>>,
        dims: [usize; 3],
        origin: [f64; 3],
        spacing: [f64; 3],
    ) -> Result<Self, MeshError> {
        Self::structured_grid_with_limits(id, dims, origin, spacing, GridBuildLimits::default())
    }

    /// Creates an affine structured grid within an explicit array budget.
    ///
    /// # Errors
    ///
    /// Returns the same construction errors as [`Self::structured_grid`].
    pub fn structured_grid_with_limits(
        id: impl Into<Arc<str>>,
        dims: [usize; 3],
        origin: [f64; 3],
        spacing: [f64; 3],
        limits: GridBuildLimits,
    ) -> Result<Self, MeshError> {
        let plan = GridPlan::new(dims, limits)?;
        for axis in 0..3 {
            if !origin[axis].is_finite() || !spacing[axis].is_finite() {
                return Err(MeshError::NonFinitePosition { index: 0 });
            }
            if !(origin[axis] + (dims[axis] - 1) as f64 * spacing[axis]).is_finite() {
                return Err(MeshError::NonFinitePosition {
                    index: (dims[axis] - 1) * plan.strides[axis],
                });
            }
        }
        let mut positions = reserve(plan.points)?;
        for k in 0..dims[2] {
            for j in 0..dims[1] {
                for i in 0..dims[0] {
                    positions.push([
                        origin[0] + i as f64 * spacing[0],
                        origin[1] + j as f64 * spacing[1],
                        origin[2] + k as f64 * spacing[2],
                    ]);
                }
            }
        }
        plan.dataset(id, positions)
    }

    /// Creates a rectilinear grid with the default construction budget.
    ///
    /// Singleton axes select lower-dimensional cells as in [`Self::structured_grid`].
    ///
    /// # Errors
    ///
    /// Returns an error for empty axes, nonfinite coordinates, unaddressable
    /// sizes, budget excess or a failed buffer reservation.
    pub fn rectilinear_grid(
        id: impl Into<Arc<str>>,
        xs: &[f64],
        ys: &[f64],
        zs: &[f64],
    ) -> Result<Self, MeshError> {
        Self::rectilinear_grid_with_limits(id, xs, ys, zs, GridBuildLimits::default())
    }

    /// Creates a rectilinear grid within an explicit array budget.
    ///
    /// # Errors
    ///
    /// Returns the same construction errors as [`Self::rectilinear_grid`].
    pub fn rectilinear_grid_with_limits(
        id: impl Into<Arc<str>>,
        xs: &[f64],
        ys: &[f64],
        zs: &[f64],
        limits: GridBuildLimits,
    ) -> Result<Self, MeshError> {
        let plan = GridPlan::new([xs.len(), ys.len(), zs.len()], limits)?;
        for (axis, values) in [xs, ys, zs].into_iter().enumerate() {
            if let Some(index) = values.iter().position(|v| !v.is_finite()) {
                return Err(MeshError::NonFinitePosition {
                    index: index * plan.strides[axis],
                });
            }
        }
        let mut positions = reserve(plan.points)?;
        for &z in zs {
            for &y in ys {
                for &x in xs {
                    positions.push([x, y, z]);
                }
            }
        }
        plan.dataset(id, positions)
    }

    /// Creates a curvilinear grid with the default construction budget.
    ///
    /// Explicit finite positions must have the dimension product in x-fastest
    /// order. Their shape does not affect the canonical cell connectivity.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions/counts/coordinates, budget excess,
    /// unaddressable sizes or a failed cell-buffer reservation.
    pub fn curvilinear_grid(
        id: impl Into<Arc<str>>,
        dims: [usize; 3],
        positions: Vec<[f64; 3]>,
    ) -> Result<Self, MeshError> {
        Self::curvilinear_grid_with_limits(id, dims, positions, GridBuildLimits::default())
    }

    /// Creates a curvilinear grid within an explicit array budget.
    ///
    /// Input vector capacity counts toward the planned conversion workspace.
    ///
    /// # Errors
    ///
    /// Returns the same construction errors as [`Self::curvilinear_grid`].
    pub fn curvilinear_grid_with_limits(
        id: impl Into<Arc<str>>,
        dims: [usize; 3],
        positions: Vec<[f64; 3]>,
        limits: GridBuildLimits,
    ) -> Result<Self, MeshError> {
        let plan = GridPlan::new(dims, limits)?;
        if positions.len() != plan.points {
            return Err(MeshError::GridPositionCountMismatch {
                positions: positions.len(),
                expected: plan.points,
            });
        }
        plan.check_position_capacity(positions.capacity())?;
        if let Some(index) = positions
            .iter()
            .position(|p| p.iter().any(|v| !v.is_finite()))
        {
            return Err(MeshError::NonFinitePosition { index });
        }
        plan.dataset(id, positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_headroom_and_wrong_position_counts_fail_before_building() {
        array_bytes::<u8>(isize::MAX as usize).unwrap_err();
        assert_eq!(
            array_bytes::<u8>(isize::MAX as usize - 64).unwrap(),
            isize::MAX as u64 - 64
        );
        let error = DataSet::curvilinear_grid("short", [2; 3], vec![[0.0; 3]; 7]).unwrap_err();
        assert_eq!(
            error,
            MeshError::GridPositionCountMismatch {
                positions: 7,
                expected: 8
            }
        );
    }
}
