//! Explicit grid budgets and pre-allocation size rejection.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, GridBuildLimits, MeshError};

#[test]
fn workspace_limit_checks_conversion_overlap_and_input_capacity() {
    let exact = GridBuildLimits {
        max_working_bytes: 384,
    };
    DataSet::structured_grid_with_limits("cube", [2; 3], [0.0; 3], [1.0; 3], exact).unwrap();
    let error = DataSet::structured_grid_with_limits(
        "cube",
        [2; 3],
        [0.0; 3],
        [1.0; 3],
        GridBuildLimits {
            max_working_bytes: 383,
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        MeshError::ResourceLimit {
            resource: "grid array workspace bytes",
            requested: 384,
            limit: 383
        }
    ));
    let mut positions = Vec::with_capacity(64);
    positions.resize(8, [0.0; 3]);
    let expected = (positions.capacity() as u64 + 8) * 24;
    let error =
        DataSet::curvilinear_grid_with_limits("capacity", [2; 3], positions, exact).unwrap_err();
    assert!(
        matches!(error,MeshError::ResourceLimit{resource:"grid array workspace bytes",requested,limit:384} if requested==expected)
    );
    let tiny = GridBuildLimits {
        max_working_bytes: 0,
    };
    DataSet::rectilinear_grid_with_limits("cube", &[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0], tiny)
        .unwrap_err();
    DataSet::curvilinear_grid_with_limits("cube", [2; 3], vec![[0.0; 3]; 8], tiny).unwrap_err();
}

#[test]
fn default_limits_reject_giant_arrays_and_overrides_keep_address_checks() {
    DataSet::structured_grid("budget", [256; 3], [0.0; 3], [1.0; 3]).unwrap_err();
    let unlimited = GridBuildLimits {
        max_working_bytes: u64::MAX,
    };
    DataSet::structured_grid_with_limits(
        "overflow",
        [usize::MAX; 3],
        [0.0; 3],
        [1.0; 3],
        unlimited,
    )
    .unwrap_err();
    #[cfg(target_pointer_width = "64")]
    {
        let error = DataSet::structured_grid_with_limits(
            "index",
            [u32::MAX as usize + 2, 1, 1],
            [0.0; 3],
            [1.0; 3],
            unlimited,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            MeshError::ResourceLimit {
                resource: "grid point count",
                ..
            }
        ));
    }
}
