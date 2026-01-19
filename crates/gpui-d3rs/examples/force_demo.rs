//! Force Directed Graph Demo
//!
//! Visualizes a force-directed graph simulation.

use d3rs::force::{ForceCenter, ForceManyBody, Simulation, SimulationNode};
use gpui::prelude::*;
use gpui::*;

#[allow(dead_code)]
struct ForceDemo {
    simulation: Simulation,
    width: f64,
    height: f64,
}

impl ForceDemo {
    fn new(_cx: &mut Context<Self>) -> Self {
        let width = 800.0;
        let height = 600.0;

        // Create nodes
        let mut nodes = Vec::new();
        for i in 0..50 {
            // Random initial positions
            // In a real app, use a proper random generator (d3-random is available in d3rs)
            // Here we just use some simple math for determinism
            let x = width / 2.0 + (i as f64 * 13.0 % 100.0 - 50.0);
            let y = height / 2.0 + (i as f64 * 17.0 % 100.0 - 50.0);
            nodes.push(SimulationNode::new(i, x, y));
        }

        let sim = Simulation::new(nodes)
            .force(Box::new(ForceManyBody::new()))
            .force(Box::new(ForceCenter::new(width / 2.0, height / 2.0)));

        Self {
            simulation: sim,
            width,
            height,
        }
    }

    #[allow(dead_code)]
    fn tick(&mut self, cx: &mut Context<Self>) {
        self.simulation.tick();
        cx.notify();
    }
}

impl Render for ForceDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Run a simulation tick
        let _view = cx.entity().clone();
        /*
        cx.spawn(|_, mut cx: &mut AsyncAppContext| async move {
            // Run at ~60fps
            cx.background_executor().timer(Duration::from_millis(16)).await;
            view.update(&mut cx, |demo, cx| {
                demo.tick(cx);
            }).ok();
        }).detach();
        */

        let mut elements = Vec::new();

        // Render nodes
        for node_rc in &self.simulation.nodes {
            let n = node_rc.borrow();
            elements.push(
                div()
                    .absolute()
                    .left(px(n.x as f32 - 5.0))
                    .top(px(n.y as f32 - 5.0))
                    .size(px(10.0))
                    .bg(rgb(0xff4444))
                    .rounded_full(),
            );
        }

        div()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .child(div().relative().size_full().children(elements))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ForceDemo::new(cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}
