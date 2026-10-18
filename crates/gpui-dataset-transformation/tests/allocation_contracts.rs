//! Native conversion/flow allocation contracts, measured in one serial process.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    cell_data_to_point_data, flow_diagnostics, point_data_to_cell_data,
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
}
