// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    ParticleOptions, StreakOptions, ftle, hyperstreamlines, lic, particle_frames, streak_frames,
    streamlines, streamlines_adaptive,
};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, TimeSeries};

fn separated(missing: f64) -> DataSet {
    let mut vector = Field::vector("v", vec![[1.0, 0.0, 0.0]; 9]);
    let gpui_mesh::FieldData::Vector(values) = &mut vector.data else {
        unreachable!()
    };
    std::sync::Arc::make_mut(values)[3] = [missing; 3];
    vector.valid = Some(vec![true, true, true, false, true, true, true, true, true].into());
    let mut tensor = Field::tensor("t", vec![[4.0, 1.0, 1.0, 0.0, 0.0, 0.0]; 9]);
    let gpui_mesh::FieldData::Tensor(values) = &mut tensor.data else {
        unreachable!()
    };
    std::sync::Arc::make_mut(values)[3] = [missing; 6];
    tensor.valid.clone_from(&vector.valid);
    DataSet::new(
        "separated",
        vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [2.0, 0.0, 1.0],
            [4.0; 3],
        ],
        Cells::from_cells(&[
            (CellType::Tetra, &[0, 1, 2, 3]),
            (CellType::Tetra, &[4, 5, 6, 7]),
        ]),
    )
    .with_field(vector)
    .with_field(tensor)
}

#[test]
fn vector_and_tensor_traces_cannot_cross_missing_stencils() {
    for missing in [99.0, f64::NAN] {
        let dataset = separated(missing);
        for (seed, supported) in [([0.2; 3], false), ([2.2, 0.2, 0.2], true)] {
            for output in [
                streamlines(&dataset, "v", &[seed], 0.05, 3).unwrap(),
                streamlines_adaptive(&dataset, "v", &[seed], 0.05, 0.025, 0.1, 1e-5, 3).unwrap(),
                hyperstreamlines(&dataset, "t", &[seed], 0.05, 3).unwrap(),
            ] {
                output.validate().unwrap();
                assert_eq!(output.cell_count() > 0, supported);
                assert!(output.positions.iter().all(|p| p[0] > 2.0));
            }
        }
        let output = lic(&dataset, "v", 2, 0.05, 7).unwrap();
        output.validate().unwrap();
        let field = output.field(Association::Point, "lic").unwrap();
        for point in 0..9 {
            assert_eq!(field.is_valid(point), (4..8).contains(&point));
        }
        let mut missing = dataset;
        for field in &mut missing.point_data {
            field.valid = Some(vec![false; 9].into());
        }
        let output = streamlines(&missing, "v", &[[2.2, 0.2, 0.2]], 0.05, 3).unwrap();
        output.validate().unwrap();
        assert_eq!(output.cell_count(), 0);
    }
}

fn changing_series() -> TimeSeries {
    let step = |speed: f64| {
        let dataset = DataSet::structured_grid("time", [2; 3], [0.0; 3], [10.0; 3]).unwrap();
        let count = dataset.point_count();
        dataset.with_field(Field::vector("v", vec![[speed, 0.0, 0.0]; count]))
    };
    TimeSeries::new(vec![step(0.0), step(1.0)], vec![0.0, 1.0]).unwrap()
}

#[test]
fn time_varying_particles_integrate_previous_frame_and_streak_release_stays_at_seed() {
    let series = changing_series();
    let frames = particle_frames(
        &series,
        "v",
        &[[1.0; 3]],
        ParticleOptions {
            dt: 0.5,
            frames: 3,
            trail: 8,
        },
    )
    .unwrap();
    for (frame, x) in frames.iter().zip([1.0, 1.125, 1.5]) {
        frame.validate().unwrap();
        assert!((frame.positions.last().unwrap()[0] - x).abs() < 1e-12);
    }
    let frames = streak_frames(
        &series,
        "v",
        &[[1.0; 3]],
        StreakOptions {
            dt: 0.5,
            frames: 3,
            release_every: 1,
        },
    )
    .unwrap();
    for frame in &frames {
        frame.validate().unwrap();
    }
    assert_eq!(frames[2].point_count(), 3);
    for (point, x) in frames[2].positions.iter().zip([1.5, 1.375, 1.0]) {
        assert!((point[0] - x).abs() < 1e-12);
    }
    assert_eq!(
        frames[2].point_data[0].as_scalar().unwrap(),
        &[1.0, 0.5, 0.0]
    );
}

#[test]
fn missing_time_support_stops_particles_and_ftle_is_missing_not_zero() {
    let mut series = changing_series();
    series.steps[1].point_data[0].valid = Some(vec![false; 8].into());
    let frames = particle_frames(
        &series,
        "v",
        &[[1.0; 3]],
        ParticleOptions {
            dt: 0.5,
            frames: 3,
            trail: 8,
        },
    )
    .unwrap();
    for frame in frames {
        frame.validate().unwrap();
        assert_eq!(frame.positions.as_ref(), &[[1.0; 3]]);
    }
    let frames = streak_frames(
        &series,
        "v",
        &[[1.0; 3]],
        StreakOptions {
            dt: 0.5,
            frames: 3,
            release_every: 1,
        },
    )
    .unwrap();
    assert_eq!(frames[1].point_count(), 0);
    assert_eq!(frames[2].point_count(), 0);
    for step in &mut series.steps {
        step.point_data[0].valid = Some(vec![false; 8].into());
    }
    for frame in particle_frames(
        &series,
        "v",
        &[[1.0; 3]],
        ParticleOptions {
            dt: 0.5,
            frames: 2,
            trail: 8,
        },
    )
    .unwrap()
    {
        frame.validate().unwrap();
        assert_eq!(frame.point_count(), 0);
    }
    let output = ftle(&series, "v", 0.0, 0.1, 0.05, 2).unwrap();
    output.validate().unwrap();
    assert!((0..8).all(|point| {
        !output
            .field(Association::Point, "ftle")
            .unwrap()
            .is_valid(point)
    }));
}

#[test]
fn valid_zero_ftle_is_distinct_from_unsupported_neighborhoods_and_noise_overflow_is_missing() {
    let dataset = DataSet::structured_grid("zero", [3; 3], [0.0; 3], [1.0; 3])
        .unwrap()
        .with_field(Field::vector("v", vec![[0.0; 3]; 27]));
    let series = TimeSeries::new(vec![dataset.clone(), dataset], vec![0.0, 1.0]).unwrap();
    let output = ftle(&series, "v", 0.0, 0.1, 0.05, 2).unwrap();
    output.validate().unwrap();
    let field = output.field(Association::Point, "ftle").unwrap();
    for point in 0..27 {
        assert_eq!(field.is_valid(point), point == 13);
        if point == 13 {
            assert!(field.as_scalar().unwrap()[point].abs() < 1e-12);
        } else {
            assert!(field.as_scalar().unwrap()[point].is_nan());
        }
    }
    let output = lic(&separated(99.0), "v", 2, f64::from_bits(1), 7).unwrap();
    output.validate().unwrap();
    assert!((0..output.point_count()).all(|point| {
        !output
            .field(Association::Point, "lic")
            .unwrap()
            .is_valid(point)
    }));
}

#[test]
fn normalized_paths_and_temporal_blends_survive_extreme_units() {
    for magnitude in [1e-200, 1e200, f64::from_bits(1), f64::MAX] {
        let dataset = DataSet::structured_grid("units", [2; 3], [0.0; 3], [2.0; 3]).unwrap();
        let dataset = dataset.with_field(Field::vector("v", vec![[magnitude, magnitude, 0.0]; 8]));
        let output = streamlines(&dataset, "v", &[[1.0; 3]], 0.1, 1).unwrap();
        output.validate().unwrap();
        assert_eq!(output.point_count(), 4);
        let first = output.positions[1];
        assert!((first[0] - 1.0 - 0.1 / 2.0_f64.sqrt()).abs() < 1e-12);
        assert!((first[1] - 1.0 - 0.1 / 2.0_f64.sqrt()).abs() < 1e-12);
    }
    let mut series = changing_series();
    series.times = vec![-f64::MAX, f64::MAX];
    series.steps[1].point_data[0].data =
        gpui_mesh::FieldData::Vector(vec![[2.0 / f64::MAX, 0.0, 0.0]; 8].into());
    assert_eq!(series.locate(0.0), (0, 1, 0.5));
    let frames = particle_frames(
        &series,
        "v",
        &[[1.0; 3]],
        ParticleOptions {
            dt: f64::MAX,
            frames: 2,
            trail: 8,
        },
    )
    .unwrap();
    assert!((frames[1].positions.last().unwrap()[0] - 1.5).abs() < 1e-12);
    series.steps[1].point_data[0].data = gpui_mesh::FieldData::Vector(vec![[0.0; 3]; 8].into());
    let frames = streak_frames(
        &series,
        "v",
        &[[1.0; 3]],
        StreakOptions {
            dt: f64::MAX,
            frames: 3,
            release_every: 1,
        },
    )
    .unwrap();
    for frame in &frames {
        frame.validate().unwrap();
    }
    let age = frames[2].field(Association::Point, "age").unwrap();
    assert_eq!(age.unit.as_deref(), Some("s"));
    assert!(!age.is_valid(0));
    assert!(age.is_valid(1) && age.is_valid(2));
}

#[test]
fn trace_entry_points_reject_nonfinite_options_seeds_and_mutated_series() {
    let dataset = separated(99.0);
    let series = changing_series();
    for step in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        streamlines(&dataset, "v", &[[2.2, 0.2, 0.2]], step, 2).unwrap_err();
        streamlines_adaptive(&dataset, "v", &[[2.2, 0.2, 0.2]], step, 0.01, 0.1, 1e-4, 2)
            .unwrap_err();
        hyperstreamlines(&dataset, "t", &[[2.2, 0.2, 0.2]], step, 2).unwrap_err();
        lic(&dataset, "v", 2, step, 1).unwrap_err();
        particle_frames(
            &series,
            "v",
            &[[1.0; 3]],
            ParticleOptions {
                dt: step,
                frames: 1,
                trail: 8,
            },
        )
        .unwrap_err();
        streak_frames(
            &series,
            "v",
            &[[1.0; 3]],
            StreakOptions {
                dt: step,
                frames: 1,
                release_every: 1,
            },
        )
        .unwrap_err();
    }
    streamlines(&dataset, "v", &[[f64::NAN; 3]], 0.1, 2).unwrap_err();
    particle_frames(
        &series,
        "v",
        &[[f64::NAN; 3]],
        ParticleOptions {
            frames: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    ftle(&series, "v", f64::NAN, 0.1, 0.01, 2).unwrap_err();
    ftle(&series, "v", 0.0, f64::INFINITY, 0.01, 2).unwrap_err();
    let mut malformed = series.clone();
    malformed.times.clear();
    particle_frames(
        &malformed,
        "v",
        &[[1.0; 3]],
        ParticleOptions {
            frames: 1,
            ..Default::default()
        },
    )
    .unwrap_err();
    let mut malformed = series;
    malformed.steps[0].point_data[0].data = gpui_mesh::FieldData::Vector(vec![[0.0; 3]].into());
    ftle(&malformed, "v", 0.0, 0.1, 0.01, 2).unwrap_err();
    malformed.validate().unwrap_err();
    TimeSeries::new(malformed.steps, malformed.times).unwrap_err();
}
