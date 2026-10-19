//! Mesh data model for 2D and 3D visualization.
//!
//! This crate owns exactly one job: representing datasets. It has no GPUI,
//! wgpu, file-I/O, threading, or time source dependency, so it compiles and
//! behaves identically on native targets and `wasm32-unknown-unknown`.
//!
//! Positions are `f64` at every public boundary. Large datasets are held in
//! shared `Arc` slices and consumed through chunk iterators, so callers never
//! need to materialize a second monolithic copy for rendering.

// Rust guideline compliant 2026-02-21

mod chunk;
mod composite;
mod dataset;
mod decomposition;
mod field;
mod grid;
mod structured_boundary;
mod time_series;

pub use chunk::{
    SurfaceChunk, SurfaceExtractionLimits, boundary_triangles, surface_chunks,
    surface_chunks_with_limits, try_for_each_surface_chunk, try_for_each_surface_chunk_with_limits,
};
pub use composite::{Block, CompositeDataSet};
pub use dataset::{Bounds, CellType, Cells, DataSet, DataSetKind, MeshError, ValidatedDataSet};
pub use decomposition::tetrahedra;
pub use field::{Association, Field, FieldData};
#[doc(inline)]
pub use grid::GridBuildLimits;
pub use time_series::{PreparedTimeSeries, TimeAxis, TimeSeries, TimeSeriesSampler};
