//! Native conversion/flow allocation contracts, measured in one serial process.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    GlyphKind, cell_data_to_point_data, flow_diagnostics, glyphs, hyperstreamlines,
    point_data_to_cell_data, solid_arrows, streamlines, tensor_glyphs,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

#[test]
fn allocation_cost_is_bounded_by_outputs_and_points_without_per_cell_scratch() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    for n in [4usize, 16] {
        let dataset = DataSet::structured_grid("allocation", [n; 3], [0.0; 3], [1.0; 3]).unwrap();
        let points = dataset.point_count();
        let cells = dataset.cell_count();
        let dataset = dataset
            .with_field(Field::scalar("point", vec![0.5; points]))
            .with_field(Field::vector("point-v", vec![[0.5; 3]; points]))
            .with_field(Field::scalar("cell", vec![0.5; cells]).on_cells())
            .with_field(Field::vector("cell-v", vec![[0.5; 3]; cells]).on_cells());
        for (name, bytes_per_cell) in [("point", 16), ("point-v", 48)] {
            let mut probe = AllocProbe::new();
            probe.reset();
            let output = black_box(point_data_to_cell_data(&dataset, name, "mean").unwrap());
            let allocations = probe.sample("point-to-cell");
            AllocationBudget::new("point-to-cell", 16, bytes_per_cell * cells + 8192)
                .assert_contains(allocations);
            output.validate().unwrap();
        }
        for (name, bytes_per_point) in [("cell", 48), ("cell-v", 80)] {
            let mut probe = AllocProbe::new();
            probe.reset();
            let output = black_box(cell_data_to_point_data(&dataset, name, "mean").unwrap());
            let allocations = probe.sample("cell-to-point");
            AllocationBudget::new("cell-to-point", 16, bytes_per_point * points + 8192)
                .assert_contains(allocations);
            output.validate().unwrap();
        }
        let mut missing = dataset;
        missing.cell_data[0].valid = Some(vec![false; cells].into());
        let mut probe = AllocProbe::new();
        probe.reset();
        let output = black_box(cell_data_to_point_data(&missing, "cell", "mean").unwrap());
        let allocations = probe.sample("missing-cell-to-point");
        AllocationBudget::new("missing-cell-to-point", 16, 18 * points + 8192)
            .assert_contains(allocations);
        let field = output.field(Association::Point, "mean").unwrap();
        assert!((0..points).all(|index| !field.is_valid(index)));
    }
    for cells in [1, 20_000] {
        let nodes = [0, 1, 2, 3];
        let connectivity = vec![(CellType::Tetra, nodes.as_slice()); cells];
        let dataset = DataSet::new(
            "flow-scratch",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            Cells::from_cells(&connectivity),
        )
        .with_field(Field::vector(
            "velocity",
            vec![[0.0; 3], [0.0, 2.0, 0.0], [-2.0, 0.0, 0.0], [0.0; 3]],
        ));
        let mut probe = AllocProbe::new();
        probe.reset();
        let output = black_box(flow_diagnostics(&dataset, "velocity").unwrap());
        let allocations = probe.sample("flow-point-scratch");
        AllocationBudget::new("flow-point-scratch", 32, 8192).assert_contains(allocations);
        output.validate().unwrap();
        assert!(
            output
                .field(Association::Point, "vorticity")
                .unwrap()
                .as_vector()
                .unwrap()
                .iter()
                .all(|v| (v[2] - 4.0).abs() < 1e-9)
        );
    }
    for points in [16, 256] {
        let dataset = DataSet::new(
            "glyph-scratch",
            vec![[0.0; 3]; points],
            Cells::from_cells(&[]),
        )
        .with_field(Field::vector("v", vec![[1.0, 0.0, 0.0]; points]))
        .with_field(Field::tensor(
            "t",
            vec![[4.0, 1.0, 1.0, 0.0, 0.0, 0.0]; points],
        ));
        let mut probe = AllocProbe::new();
        probe.reset();
        let output = black_box(glyphs(&dataset, "v", 1.0, 1, GlyphKind::Arrow).unwrap());
        AllocationBudget::new("line-glyph-storage", 96, 1024 * points + 8192)
            .assert_contains(probe.sample("line-glyph-storage"));
        output.validate().unwrap();
        probe.reset();
        let output = black_box(solid_arrows(&dataset, "v", 1.0, 1, 0.05).unwrap());
        AllocationBudget::new("solid-glyph-storage", 96, 6000 * points + 8192)
            .assert_contains(probe.sample("solid-glyph-storage"));
        output.validate().unwrap();
        probe.reset();
        let output = black_box(tensor_glyphs(&dataset, "t", 1.0, 1).unwrap());
        AllocationBudget::new("tensor-glyph-storage", 96, 9000 * points + 8192)
            .assert_contains(probe.sample("tensor-glyph-storage"));
        output.validate().unwrap();
    }
    let paths = DataSet::structured_grid("trace-scratch", [2; 3], [0.0; 3], [10_000.0, 1.0, 1.0])
        .unwrap()
        .with_field(Field::vector("v", vec![[1.0, 0.0, 0.0]; 8]))
        .with_field(Field::tensor("t", vec![[4.0, 1.0, 1.0, 0.0, 0.0, 0.0]; 8]));
    for steps in [8, 256] {
        let mut probe = AllocProbe::new();
        probe.reset();
        let output =
            black_box(streamlines(&paths, "v", &[[5000.0, 0.5, 0.5]], 1.0, steps).unwrap());
        AllocationBudget::new("vector-line-storage", 64, 256 * output.point_count() + 8192)
            .assert_contains(probe.sample("vector-line-storage"));
        output.validate().unwrap();
        assert_eq!(output.point_count(), 2 * (steps + 1));
        probe.reset();
        let output =
            black_box(hyperstreamlines(&paths, "t", &[[5000.0, 0.5, 0.5]], 1.0, steps).unwrap());
        AllocationBudget::new("tensor-line-storage", 64, 256 * output.point_count() + 8192)
            .assert_contains(probe.sample("tensor-line-storage"));
        output.validate().unwrap();
        assert_eq!(output.point_count(), 2 * (steps + 1));
    }
    let nodes = [0, 1, 2, 3];
    let cells = vec![(CellType::Tetra, nodes.as_slice()); 4096];
    let dataset = DataSet::new(
        "compact-stencils",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        Cells::from_cells(&cells),
    )
    .with_field(Field::vector("v", vec![[1.0, 0.0, 0.0]; 4]));
    let mut probe = AllocProbe::new();
    probe.reset();
    let output = black_box(streamlines(&dataset, "v", &[], 0.1, 8).unwrap());
    AllocationBudget::new(
        "compact-stencil-index",
        32,
        16 * dataset.cell_count() + 8192,
    )
    .assert_contains(probe.sample("compact-stencil-index"));
    output.validate().unwrap();
    assert_eq!(output.point_count(), 0);
}
