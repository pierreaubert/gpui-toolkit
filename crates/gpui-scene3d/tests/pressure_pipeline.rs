//! End-to-end: pressure on a volume, plane slice, isosurface, movie frames.

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{Plane, isosurface, probe, slice};
use gpui_mesh::{Association, DataSet, Field, TimeSeries};
use gpui_scene3d::{Actor, Camera, Player, Scene, TransferFunction, render};

fn room(pressure_scale: f64) -> DataSet {
    let mut dataset =
        DataSet::structured_grid("room", [8, 6, 5], [0.0; 3], [0.5, 0.5, 0.5]).unwrap();
    let centre = [1.75, 1.25, 1.0];
    let pressure: Vec<f64> = dataset
        .positions
        .iter()
        .map(|p| {
            let d2 = (p[0] - centre[0]).powi(2)
                + (p[1] - centre[1]).powi(2)
                + (p[2] - centre[2]).powi(2);
            pressure_scale * (-d2).exp() * 100.0
        })
        .collect();
    dataset
        .point_data
        .push(Field::scalar("pressure", pressure).labelled("Pressure", "Pa"));
    dataset
}

fn scene_for(dataset: DataSet) -> Scene {
    Scene::new().with_actor(
        Actor::new(dataset).coloured_by("pressure", TransferFunction::viridis([0.0, 100.0])),
    )
}

fn covered(frame: &gpui_scene3d::Frame, background: [f32; 4]) -> usize {
    let bg = [
        (background[0] * 255.0) as u8,
        (background[1] * 255.0) as u8,
        (background[2] * 255.0) as u8,
    ];
    frame
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|px| px[..3] != bg)
        .count()
}

#[test]
fn pressure_slice_isosurface_and_movie() {
    let dataset = room(1.0);
    assert_eq!(dataset.validate(), Ok(()));

    // Probe at the centre recovers the peak pressure.
    // Linear interpolation of node-sampled Gaussian values underestimates
    // the analytic peak of 100; the exactness check lives in the probe
    // unit test on a linear field.
    let probed = probe(&dataset, "pressure", &[centre()]).unwrap();
    assert!(probed[0].unwrap() > 75.0);

    // A mid-plane slice is a flat surface that renders with coverage.
    let cut = slice(&dataset, &Plane::axis(2, 1.0).unwrap()).unwrap();
    assert!(cut.cell_count() > 0);
    assert_eq!(cut.validate(), Ok(()));
    let slice_scene = scene_for(cut);
    let camera = Camera::fitted(slice_scene.bounds().unwrap());
    let frame = render(&slice_scene, &camera, 96, 96).unwrap();
    assert!(covered(&frame, slice_scene.background) > 200);

    // The full volume renders coloured by pressure.
    let volume_scene = scene_for(dataset.clone());
    let frame = render(&volume_scene, &camera, 96, 96).unwrap();
    assert!(covered(&frame, volume_scene.background) > 200);

    // An isosurface at half peak is a closed, renderable surface.
    let iso = isosurface(&dataset, "pressure", 50.0).unwrap();
    assert!(iso.cell_count() > 10);
    assert_eq!(iso.validate(), Ok(()));

    // A two-step time series produces distinct movie frames.
    let series = TimeSeries::new(vec![room(0.2), room(1.0)], vec![0.0, 1.0]).unwrap();
    let player = Player::new(series).unwrap();
    let frames = player.movie_frames(scene_for, &camera, 64, 64, 3).unwrap();
    assert_eq!(frames.len(), 3);
    assert_ne!(frames[0].1.rgba, frames[2].1.rgba);

    // The colouring field is found through the public actor API.
    let actor = Actor::new(dataset);
    assert!(actor.scalars().is_none());
    let coloured = scene_for(room(1.0));
    assert_eq!(
        coloured.actors[0].scalars().map(|(a, _)| a),
        Some(Association::Point)
    );
}

fn centre() -> [f64; 3] {
    [1.75, 1.25, 1.0]
}
