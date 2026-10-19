//! Native conversion/flow allocation contracts, measured in one serial process.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    GlyphKind, ThresholdMode, append_datasets, calculator, cell_data_to_point_data, clip_sphere,
    decimate, field_statistics, flow_diagnostics, glyphs, histogram, hyperstreamlines,
    point_data_to_cell_data, point_data_to_cell_data_validated, probe as probe_points,
    solid_arrows, streamlines, tensor_glyphs, threshold,
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
    for points in [16, 4096] {
        let dataset = DataSet::new("analysis", vec![[1.0; 3]; points], Cells::from_cells(&[]))
            .with_field(Field::scalar("p", vec![4.0; points]));
        let mut allocations = AllocProbe::new();
        allocations.reset();
        let output = black_box(
            calculator(&dataset, "min(sqrt(p),abs(-p))+sin(x)+cos(y)", "result").unwrap(),
        );
        AllocationBudget::new("compiled-expression", 64, 18 * points + 32768)
            .assert_contains(allocations.sample("compiled-expression"));
        output.validate().unwrap();
        allocations.reset();
        let stats = black_box(field_statistics(&dataset, "p", Association::Point).unwrap());
        AllocationBudget::new("linear-statistics", 4, 8 * points + 8192)
            .assert_contains(allocations.sample("linear-statistics"));
        assert_eq!(stats.count, points);
        assert_eq!(stats.mean, 4.0);
        allocations.reset();
        let output = black_box(histogram(&dataset, "p", 4).unwrap());
        AllocationBudget::new("streaming-histogram", 4, 8192)
            .assert_contains(allocations.sample("streaming-histogram"));
        assert_eq!(output.iter().map(|(_, count)| count).sum::<usize>(), points);
        let mut missing = dataset;
        missing.point_data[0].valid = Some(vec![false; points].into());
        allocations.reset();
        black_box(field_statistics(&missing, "p", Association::Point)).unwrap_err();
        AllocationBudget::new("missing-statistics", 4, 8192)
            .assert_contains(allocations.sample("missing-statistics"));
    }
    let dataset = dataset.with_field(Field::scalar("p", vec![1.0; 4]));
    let mut allocations = AllocProbe::new();
    allocations.reset();
    let output = black_box(probe_points(&dataset, "p", &[[0.1; 3]; 128]).unwrap());
    AllocationBudget::new("compact-probe-index", 32, 16 * dataset.cell_count() + 8192)
        .assert_contains(allocations.sample("compact-probe-index"));
    assert_eq!(output, [Some(1.0); 128]);
    allocations.reset();
    let output = black_box(probe_points(&dataset, "p", &[[0.1; 3]]).unwrap());
    AllocationBudget::new("single-probe-no-index", 1, 16)
        .assert_contains(allocations.sample("single-probe-no-index"));
    assert_eq!(output, [Some(1.0)]);
    allocations.reset();
    let output = black_box(probe_points(&dataset, "p", &[]).unwrap());
    AllocationBudget::new("empty-probe-index", 0, 0)
        .assert_contains(allocations.sample("empty-probe-index"));
    assert!(output.is_empty());
    for n in [4usize, 16] {
        let mut data =
            DataSet::structured_grid("packed-geometry", [n; 3], [0.0; 3], [1.0; 3]).unwrap();
        let points = data.point_count();
        let cells = data.cell_count();
        for field in [
            Field::scalar("s", vec![0.5; points]),
            Field::vector("v", vec![[0.5; 3]; points]),
            Field::tensor("t", vec![[0.5; 6]; points]),
            Field::complex("z", vec![[0.5; 2]; points]),
        ] {
            data.point_data.push(field);
        }
        for field in [
            Field::scalar("s", vec![0.5; cells]),
            Field::vector("v", vec![[0.5; 3]; cells]),
            Field::tensor("t", vec![[0.5; 6]; cells]),
            Field::complex("z", vec![[0.5; 2]; cells]),
        ] {
            data.cell_data.push(field.on_cells());
        }
        let mut allocations = AllocProbe::new();
        allocations.reset();
        let output = black_box(decimate(&data, 2.0).unwrap());
        AllocationBudget::new(
            "clustered-packed-geometry",
            128,
            256 * points + 64 * cells + 65536,
        )
        .assert_contains(allocations.sample("clustered-packed-geometry"));
        output.validate().unwrap();
        allocations.reset();
        let output = black_box(append_datasets(&data, &data).unwrap());
        AllocationBudget::new(
            "appended-packed-geometry",
            96,
            640 * (points + cells) + 32768,
        )
        .assert_contains(allocations.sample("appended-packed-geometry"));
        output.validate().unwrap();
        allocations.reset();
        let output = black_box(
            threshold(&data, "s", Association::Cell, 0.5, 0.5, ThresholdMode::All).unwrap(),
        );
        AllocationBudget::new(
            "selected-packed-geometry",
            96,
            384 * points + 128 * cells + 32768,
        )
        .assert_contains(allocations.sample("selected-packed-geometry"));
        output.validate().unwrap();
        allocations.reset();
        let output =
            black_box(clip_sphere(&data, [0.0; 3], 1e10, true, ThresholdMode::All).unwrap());
        AllocationBudget::new(
            "sphere-packed-geometry",
            96,
            384 * points + 128 * cells + 32768,
        )
        .assert_contains(allocations.sample("sphere-packed-geometry"));
        output.validate().unwrap();
        let mut mismatch = data.clone();
        allocations.reset();
        let empty = black_box(
            threshold(&data, "s", Association::Cell, 2.0, 3.0, ThresholdMode::All).unwrap(),
        );
        AllocationBudget::new("empty-selected-geometry", 32, cells + 8192)
            .assert_contains(allocations.sample("empty-selected-geometry"));
        empty.validate().unwrap();
        assert_eq!(empty.point_count(), 0);
        mismatch.point_data[0].unit = Some("different".into());
        allocations.reset();
        black_box(append_datasets(&data, &mismatch)).unwrap_err();
        AllocationBudget::new("append-schema-error", 12, 8192)
            .assert_contains(allocations.sample("append-schema-error"));
    }
    let count = 4096usize;
    let nodes = (0..count as u32).map(|node| [node]).collect::<Vec<_>>();
    let cells = nodes
        .iter()
        .map(|node| (CellType::Vertex, node.as_slice()))
        .collect::<Vec<_>>();
    let data = DataSet::new(
        "exact-clusters",
        (0..count)
            .map(|i| [(i / 2) as f64, 0.0, 0.0])
            .collect::<Vec<_>>(),
        Cells::from_cells(&cells),
    )
    .with_field(Field::scalar("s", vec![f64::MAX; count]))
    .with_field(Field::vector("v", vec![[f64::MAX; 3]; count]))
    .with_field(Field::tensor("t", vec![[f64::MAX; 6]; count]))
    .with_field(Field::complex("z", vec![[f64::MAX; 2]; count]));
    let mut allocations = AllocProbe::new();
    allocations.reset();
    let output = black_box(decimate(&data, 1.0).unwrap());
    AllocationBudget::new("bounded-exact-clusters", 160, 576 * count + 65536)
        .assert_contains(allocations.sample("bounded-exact-clusters"));
    output.validate().unwrap();
    assert_eq!(output.point_count(), count / 2);
    assert_eq!(output.point_data[0].as_scalar().unwrap()[0], f64::MAX);
    for point_count in [64usize, 4096] {
        let mut values = vec![f64::NAN; point_count];
        values[..3].copy_from_slice(&[1e16, 3.0, -1e16]);
        let mut valid = vec![false; point_count];
        valid[..3].fill(true);
        let mut sparse = DataSet::new(
            "sparse-cancellation",
            vec![[0.0; 3]; point_count],
            Cells::triangles(&[[0, 1, 2]]),
        )
        .with_field(Field::scalar("p", values));
        sparse.point_data[0].valid = Some(valid.into());
        let checked = sparse.validated().unwrap();
        allocations.reset();
        let output = black_box(point_data_to_cell_data_validated(checked, "p", "mean").unwrap());
        AllocationBudget::new("sparse-exact-mean", 16, 8192)
            .assert_contains(allocations.sample("sparse-exact-mean"));
        assert_eq!(
            output
                .field(Association::Cell, "mean")
                .unwrap()
                .as_scalar()
                .unwrap(),
            [1.0]
        );
    }
    for point_count in [63usize, 4095] {
        let values = (0..point_count)
            .map(|i| [f64::MAX, -f64::MAX, 1e-300][i % 3])
            .collect::<Vec<_>>();
        let data = DataSet::new(
            "wide-mean",
            vec![[0.0; 3]; point_count],
            Cells::from_cells(&[]),
        )
        .with_field(Field::scalar("p", values));
        allocations.reset();
        let stats = black_box(field_statistics(&data, "p", Association::Point).unwrap());
        AllocationBudget::new("wide-exact-statistics", 1, 8 * point_count + 8192)
            .assert_contains(allocations.sample("wide-exact-statistics"));
        assert_eq!(stats.mean.to_bits(), (1e-300_f64 / 3.0).to_bits());
    }
    let point_count = 2560usize;
    let nodes = (0..point_count as u32)
        .map(|node| [node])
        .collect::<Vec<_>>();
    let cells = nodes
        .iter()
        .map(|node| (CellType::Vertex, node.as_slice()))
        .collect::<Vec<_>>();
    let values = (0..point_count)
        .map(|i| [1e308, 1.0, 1e-300, -1e308, -1.0][i % 5])
        .collect::<Vec<_>>();
    let data = DataSet::new(
        "wide-cancellation",
        (0..point_count)
            .map(|i| [(i / 5) as f64, 0.0, 0.0])
            .collect::<Vec<_>>(),
        Cells::from_cells(&cells),
    )
    .with_field(Field::scalar("s", values.clone()))
    .with_field(Field::vector(
        "v",
        values.iter().map(|&v| [v; 3]).collect::<Vec<_>>(),
    ))
    .with_field(Field::tensor(
        "t",
        values.iter().map(|&v| [v; 6]).collect::<Vec<_>>(),
    ))
    .with_field(Field::complex(
        "z",
        values.iter().map(|&v| [v; 2]).collect::<Vec<_>>(),
    ));
    allocations.reset();
    let output = black_box(decimate(&data, 1.0).unwrap());
    AllocationBudget::new("wide-exact-clusters", 160, 576 * point_count + 65536)
        .assert_contains(allocations.sample("wide-exact-clusters"));
    output.validate().unwrap();
    assert_eq!(output.point_count(), point_count / 5);
    assert!(
        output.point_data[0]
            .as_scalar()
            .unwrap()
            .iter()
            .all(|v| v.to_bits() == (1e-300_f64 / 5.0).to_bits())
    );
}
