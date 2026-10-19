//! Dataset-to-dataset transformations.
//!
//! This crate owns exactly one job: pure filters that consume a
//! [`gpui_mesh::DataSet`] and produce a new `DataSet`. It has no GPUI, wgpu,
//! file-I/O, threading, or time source dependency, so every filter runs
//! identically on native targets and `wasm32-unknown-unknown`, and can be
//! unit-tested headlessly. Filters never mutate their input; large inputs
//! are processed cell by cell without materializing a second copy of the
//! input geometry.
//!
//! Scope boundary with Sonium: this crate is a filter and file-format
//! layer, not a mesher. Delaunay triangulation delegates to the
//! `delaunator` kernel Sonium's mesher also builds on, and STL/OBJ/PLY
//! support is format glue for exchanging surfaces. Mesh generation,
//! tetrahedralization, refinement, quality repair, and meshlang live in
//! `sonium-mesher` and are not reimplemented here.

// Rust guideline compliant 2026-02-21

mod analysis;
mod base64;

pub use base64::{decode as decode_base64, encode as encode_base64};
mod calculator;
mod clip;
mod contours;
mod convert;
mod eigen;
mod flow;
mod ftle;
mod geometry;
mod glyph;
#[cfg(feature = "gpu")]
pub mod gpu;
mod interp;
mod isosurface;
mod lic;
mod lines;
mod parallel;
mod particles;
mod plane;
mod probe;
mod pvd;
mod stencil;
mod streamline;
mod surface;
mod surface_io;
mod tensor_trace;
mod threshold;
mod triangulate;
mod validity;
mod vtk_io;
mod vtu;
mod warp;
mod xdmf;

pub use analysis::{FieldStats, calculator, field_statistics, histogram, resample_to_grid};
pub use calculator::{complex_magnitude, complex_phase, vector_magnitude};
pub use clip::{clip_halfspace, slice};
pub use contours::contour_lines;
pub use convert::{
    cell_data_to_point_data, cell_data_to_point_data_validated, point_data_to_cell_data,
    point_data_to_cell_data_validated,
};
pub use flow::{
    divergence, flow_diagnostics, lambda2, q_criterion, swirling_strength, vorticity,
    vorticity_magnitude,
};
pub use ftle::ftle;
pub use geometry::{append_datasets, clip_sphere, decimate, smooth, transform_dataset};
pub use glyph::{GlyphKind, glyphs, solid_arrows, tensor_glyphs};
pub use isosurface::isosurface;
pub use lic::lic;
pub use lines::{dashed_lines, stream_ribbons, streamtubes};
pub use particles::{ParticleOptions, StreakOptions, particle_frames, streak_frames};
pub use plane::Plane;
pub use probe::probe;
pub use pvd::{PvdEntry, parse_pvd, pvd_part_bytes, read_pvd, write_pvd};
pub use streamline::{streamlines, streamlines_adaptive};
pub use surface::{feature_edges, surface_normals};
pub use surface_io::{read_obj, read_ply, read_stl, write_obj, write_ply, write_stl};
pub use tensor_trace::hyperstreamlines;
pub use threshold::{ThresholdMode, threshold};
pub use triangulate::{convex_hull, delaunay_2d};
pub use vtk_io::{read_vtk, write_vtk};
pub use vtu::{
    VtuReadLimits, read_vtu, read_vtu_bytes, read_vtu_bytes_with_limits, read_vtu_with_limits,
    write_vtu, write_vtu_appended, write_vtu_compressed,
};
pub use warp::{warp_by_scalar, warp_by_vector};
pub use xdmf::{XdmfBundle, read_xdmf, write_xdmf};

use gpui_mesh::CellType;

/// Failures reported by transformations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TransformError {
    /// The named field does not exist with the requested association.
    #[error("field '{0}' not found")]
    FieldNotFound(String),
    /// The named field has the wrong component shape for this filter.
    #[error("field '{field}' has the wrong shape: {reason}")]
    WrongFieldShape {
        /// Field name.
        field: String,
        /// Why the shape is unsuitable.
        reason: String,
    },
    /// The filter does not handle this cell kind yet.
    #[error("cell kind {0:?} is not supported by this filter")]
    UnsupportedCell(CellType),
    /// A geometric input is degenerate.
    #[error("degenerate input: {0}")]
    Degenerate(String),
    /// The input dataset failed validation.
    #[error(transparent)]
    Mesh(#[from] gpui_mesh::MeshError),
}
