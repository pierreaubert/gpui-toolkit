//! Bounds guard shared by the Metal backend and its host-runnable regression.

/// An interleaved scalar update writes one vertex for each retained index.
/// A changed topology must therefore be rebuilt instead of writing through a
/// buffer from its previous resource generation.
pub(super) fn interleaved_upload_fits_retained_vertices(
    index_count: usize,
    vertex_count: usize,
) -> bool {
    index_count == vertex_count
}

#[cfg(test)]
mod tests {
    use super::interleaved_upload_fits_retained_vertices;

    #[test]
    fn interleaved_field_upload_rejects_a_changed_vertex_topology() {
        assert!(interleaved_upload_fits_retained_vertices(12, 12));
        assert!(!interleaved_upload_fits_retained_vertices(9, 12));
        assert!(!interleaved_upload_fits_retained_vertices(15, 12));
    }
}
