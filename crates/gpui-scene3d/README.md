# gpui-scene3d

3D scene rendering for datasets in the GPUI toolkit.

This crate owns exactly one job: turning datasets into pictures. It holds
scenes and actors, an orbit camera (perspective and orthographic),
colour/opacity transfer functions with GPU lookup tables, chunked GPU-ready
buffer preparation (rebased `f32` positions, smooth normals, per-vertex
scalars, chunk-local indices), a deterministic software rasterizer, and
time-series playback with movie-frame generation.

The default build has no GPUI, wgpu, file-I/O, threading, or wall-clock
dependency, so the full pipeline runs identically on native targets and
`wasm32-unknown-unknown`; hosts drive playback by supplying elapsed times.
GPU hosts upload the prepared chunks directly and compile the WGSL in
`shaders.rs` (scalar LUT colouring plus up to six clipping planes with
discard semantics identical to the software path). The software
rasterizer covers headless QA, movie frames, and wasm fallback, with
world-space clipping planes and a back-to-front sorted transparent pass.
`raycast_volume` / `VolumeRenderer` add front-to-back volume raycasting
over a retained uniform tet grid with clipping planes. Actors clip
against any number of planes, or a `ClipBox` (six planes from bounds);
the interactive viewer example drives three orthogonal planes plus a
clip box. `legend` computes colour bars (nice ticks, formatted labels,
pixel layout) and draws the gradient strip and an axis triad into
software frames; scenes also carry text `Annotation`s for GPUI hosts.
`TransferFunction::banded` quantises a ramp for filled contours.
Examples: `pcfan_airflow`, `speaker_port`, `necking_bar`,
`interactive_viewer`, and `flow_showcase` (Taylor-Green vortex exercising
arrows, tubes, LIC, contours, tensor glyphs, particles, legend).
`mesh-showcase` is the same Taylor-Green field as a browser demo
(`just wasm-serve-mesh`): vortex cores, slice, contours, shells. The
`gpu` feature adds `GpuRenderer`: scenes upload their geometry and LUTs
once and then draw by rewriting only the 288-byte per-actor uniform
block, so frames never read geometry back to the CPU (Metal on Apple
Silicon, Vulkan/DX12 adapters elsewhere, WebGPU in the browser);
`gpu_render` is the matching example with a QA readback. Enable the `png`
feature for PNG frame export, and `mp4` (native only) to mux frames to
H.264 MP4 through the system `ffmpeg`.

```rust
use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{render, Actor, Camera, Scene, TransferFunction};

let dataset = DataSet::surface("plate", vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], &[[0, 1, 2]])
    .with_field(Field::scalar("pressure", vec![0.0, 0.5, 1.0]));
let scene = Scene::new().with_actor(
    Actor::new(dataset).coloured_by("pressure", TransferFunction::viridis([0.0, 1.0])),
);
let frame = render(&scene, &Camera::fitted(scene.bounds().unwrap()), 320, 240).unwrap();
assert_eq!(frame.rgba.len(), 320 * 240 * 4);
```
