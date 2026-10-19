// Rust guideline compliant 2026-02-21

use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData, TimeSeries};
use gpui_scene3d::{Camera, Player, Scene};

fn step(value: f64) -> DataSet {
    DataSet::new(
        "time",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
    .with_field(Field::scalar("p", vec![value; 4]))
    .with_field(Field::vector("v", vec![[value; 3]; 4]))
    .with_field(Field::tensor("t", vec![[value; 6]; 4]))
    .with_field(Field::complex("c", vec![[value; 2]; 4]))
    .with_field(Field::scalar("p", vec![10.0 * value]).on_cells())
}

fn player(range: [f64; 2]) -> Player {
    Player::new(TimeSeries::new(vec![step(1.0), step(3.0)], range.to_vec()).unwrap()).unwrap()
}

#[test]
fn playback_uses_named_fields_for_every_shape_association_and_mask() {
    let mut a = step(1.0);
    let mut b = step(3.0);
    for field in &mut a.point_data {
        field.valid = Some(vec![false, true, true, true].into());
    }
    for field in &mut b.point_data {
        field.valid = Some(vec![true, false, true, true].into());
    }
    b.point_data.reverse();
    let series = TimeSeries::new(vec![a, b], vec![0.0, 1.0]).unwrap();
    let expected = series.sample(0.5).unwrap();
    let mut player = Player::new(series).unwrap();
    player.seek(0.5).unwrap();
    let output = player.current().unwrap();
    output.validate().unwrap();
    assert_eq!(
        output
            .point_data
            .iter()
            .map(|f| f.name.as_ref())
            .collect::<Vec<_>>(),
        expected
            .point_data
            .iter()
            .map(|f| f.name.as_ref())
            .collect::<Vec<_>>()
    );
    for field in &output.point_data {
        assert_eq!(
            field.valid.as_deref(),
            Some([false, false, true, true].as_slice())
        );
        assert!(!field.is_valid(0) && !field.is_valid(1) && field.is_valid(2));
        match &field.data {
            FieldData::Scalar(v) => assert_eq!(v[2], 2.0),
            FieldData::Vector(v) => assert_eq!(v[2], [2.0; 3]),
            FieldData::Tensor(v) => assert_eq!(v[2], [2.0; 6]),
            FieldData::Complex(v) => assert_eq!(v[2], [2.0; 2]),
        }
    }
    assert_eq!(
        output
            .field(Association::Cell, "p")
            .unwrap()
            .as_scalar()
            .unwrap(),
        &[20.0]
    );
}

#[test]
fn playback_controls_wrap_reverse_pause_and_report_no_change() {
    let mut player = player([0.0, 2.0]);
    assert!(!player.advance(1.0).unwrap());
    player.play();
    assert!(!player.advance(0.0).unwrap());
    player.speed = 0.0;
    assert!(!player.advance(1.0).unwrap());
    player.speed = -1.0;
    assert!(player.advance(0.25).unwrap());
    assert_eq!(player.time, 1.75);
    player.speed = 1.0;
    player.advance(-0.5).unwrap();
    assert_eq!(player.time, 1.25);
    player.seek(0.25).unwrap();
    player.advance(6.5).unwrap();
    assert_eq!(player.time, 0.75);
    player.looping = false;
    player.seek(1.0).unwrap();
    player.speed = -1.0;
    player.advance(1.0).unwrap();
    assert_eq!(player.time, 0.0);
    assert!(!player.playing);
    player.speed = 1.0;
    player.play();
    player.advance(2.0).unwrap();
    assert_eq!(player.time, 2.0);
    assert!(!player.playing);
    let mut single = Player::new(TimeSeries::new(vec![step(1.0)], vec![7.0]).unwrap()).unwrap();
    single.play();
    assert!(single.advance(1.0).unwrap());
    assert!(!single.playing);
    assert_eq!(single.time, 7.0);
}

#[test]
fn nonfinite_controls_and_unrepresentable_loop_increments_fail_atomically() {
    let mut player = player([0.0, 2.0]);
    player.seek(1.0).unwrap();
    player.play();
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        player.seek(invalid).unwrap_err();
        assert_eq!(player.time, 1.0);
        player.advance(invalid).unwrap_err();
        assert_eq!(player.time, 1.0);
        assert!(player.playing);
        player.speed = invalid;
        player.advance(1.0).unwrap_err();
        assert_eq!(player.time, 1.0);
        assert!(player.playing);
        player.speed = 1.0;
        player.time = invalid;
        player.current().unwrap_err();
        player.advance(1.0).unwrap_err();
        player.seek(1.0).unwrap();
    }
    player.speed = f64::MAX;
    player.advance(f64::MAX).unwrap_err();
    assert_eq!(player.time, 1.0);
    assert!(player.playing);
    Player::new(TimeSeries {
        steps: vec![],
        times: vec![],
    })
    .unwrap_err();
}

#[test]
fn extreme_ranges_wrap_without_overflow_and_movie_times_stay_finite() {
    let mut playback = player([-f64::MAX, f64::MAX]);
    playback.seek(f64::MAX).unwrap();
    playback.play();
    playback.advance(f64::MAX).unwrap();
    assert_eq!(playback.time, 0.0);
    playback.looping = false;
    playback.speed = 2.0;
    playback.seek(-f64::MAX).unwrap();
    playback.play();
    playback.advance(f64::MAX).unwrap();
    assert_eq!(playback.time, f64::MAX);
    assert!(!playback.playing);
    let frames = playback
        .movie_frames(|_| Scene::new(), &Camera::default(), 8, 8, 3)
        .unwrap();
    assert_eq!(
        frames.iter().map(|f| f.0).collect::<Vec<_>>(),
        vec![-f64::MAX, 0.0, f64::MAX]
    );
    playback
        .movie_frames(
            |_| panic!("zero-frame movie must not render"),
            &Camera::default(),
            8,
            8,
            0,
        )
        .unwrap_err();
    let mut playback = player([1e308, f64::MAX]);
    playback.seek(f64::MAX).unwrap();
    playback.play();
    playback.advance(f64::MAX - 1e308).unwrap();
    assert_eq!(playback.time, 1e308);
    let mut playback = player([0.0, 1.0]);
    playback.speed = -1.0;
    playback.play();
    playback.advance(f64::from_bits(1)).unwrap();
    assert_eq!(playback.time, 1.0);
}
