use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum Scene3DError {
    #[error("empty data: {field} is empty")]
    EmptyData { field: &'static str },

    #[error("invalid data in {field}: {reason}")]
    InvalidData {
        field: &'static str,
        reason: &'static str,
    },

    #[error("{x_field} has {x_len} elements but {y_field} has {y_len} elements")]
    DataLengthMismatch {
        x_field: &'static str,
        y_field: &'static str,
        x_len: usize,
        y_len: usize,
    },

    #[error(
        "grid dimension mismatch: z has {z_len} values but expected {width} x {height} = {expected}"
    )]
    GridDimensionMismatch {
        z_len: usize,
        width: usize,
        height: usize,
        expected: usize,
    },

    #[error("mesh index {index} at position {position} references missing vertex {vertex_count}")]
    InvalidMeshIndex {
        position: usize,
        index: u32,
        vertex_count: usize,
    },

    #[error("unknown colormap: {name}")]
    UnknownColormap { name: String },

    #[error("unknown interaction mode: {name}")]
    UnknownInteraction { name: String },

    #[error("invalid color: {value}")]
    InvalidColor { value: String },

    #[error("unsupported scene node for this adapter: {kind}")]
    UnsupportedNode { kind: &'static str },
}
