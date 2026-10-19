//! Native decomposition and time-sampling allocation contracts, run serially.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field, FieldData, TimeSeries, tetrahedra};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

#[test]
fn decomposition_and_sampling_respect_allocation_contracts() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    let nodes = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut probe = AllocProbe::new();
    probe.reset();
    let mut count = 0usize;
    for _ in 0..10_000 {
        for tet in tetrahedra(black_box(CellType::Hexahedron), black_box(&nodes)) {
            black_box(tet);
            count += 1;
        }
    }
    let allocations = probe.sample("mesh-tetrahedra-10000x");
    assert_eq!(count, 60_000);
    AllocationBudget::zero("mesh-tetrahedra-10000x").assert_contains(allocations);
    for points in [64, 4096] {
        let dataset = DataSet::new("sampling", vec![[0.0; 3]; points], Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![1.0; points]))
            .with_field(Field::vector("v", vec![[1.0; 3]; points]))
            .with_field(Field::tensor("t", vec![[1.0; 6]; points]))
            .with_field(Field::complex("c", vec![[1.0; 2]; points]));
        let prepared = TimeSeries::new(vec![dataset.clone(), dataset.clone()], vec![0.0, 1.0])
            .unwrap()
            .prepare()
            .unwrap();
        probe.reset();
        let cloned = black_box(prepared.clone());
        AllocationBudget::zero("prepared-owner-clone")
            .assert_contains(probe.sample("prepared-owner-clone"));
        drop(cloned);
        let axis = prepared.time_axis();
        probe.reset();
        for _ in 0..10_000 {
            black_box(axis.locate(black_box(0.5)).unwrap());
        }
        AllocationBudget::zero("prepared-axis-locate")
            .assert_contains(probe.sample("prepared-axis-locate"));
        probe.reset();
        let output = black_box(prepared.sample(0.5).unwrap());
        AllocationBudget::new("unchanged-sample-storage", 4, 4096)
            .assert_contains(probe.sample("unchanged-sample-storage"));
        output.validate().unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &output.positions,
            &dataset.positions
        ));
        let mut changed = dataset.clone();
        changed.point_data[0].data = FieldData::Scalar(vec![3.0; points].into());
        let prepared = TimeSeries::new(vec![dataset.clone(), changed.clone()], vec![0.0, 1.0])
            .unwrap()
            .prepare()
            .unwrap();
        probe.reset();
        let output = black_box(prepared.sample(0.5).unwrap());
        AllocationBudget::new("one-changing-field", 6, 16 * points + 4096)
            .assert_contains(probe.sample("one-changing-field"));
        output.validate().unwrap();
        assert_eq!(output.point_data[0].as_scalar().unwrap(), vec![2.0; points]);
        let mut masked = dataset;
        masked.point_data[0].valid = Some(
            (0..points)
                .map(|point| point % 2 != 0)
                .collect::<Vec<_>>()
                .into(),
        );
        changed.point_data[0].valid = Some(
            (0..points)
                .map(|point| point % 3 != 0)
                .collect::<Vec<_>>()
                .into(),
        );
        let prepared = TimeSeries::new(vec![masked, changed], vec![0.0, 1.0])
            .unwrap()
            .prepare()
            .unwrap();
        probe.reset();
        let output = black_box(prepared.sample(0.5).unwrap());
        AllocationBudget::new("intersected-changing-field", 8, 18 * points + 4096)
            .assert_contains(probe.sample("intersected-changing-field"));
        output.validate().unwrap();
    }
}
