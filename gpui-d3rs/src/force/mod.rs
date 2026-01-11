//! Force-directed graph layout (d3-force)
//!
//! This module implements a force-directed graph simulation using velocity Verlet integration.

use std::cell::RefCell;
use std::rc::Rc;

/// A node in the simulation
#[derive(Debug, Clone)]
pub struct SimulationNode {
    pub index: usize,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub fx: Option<f64>, // Fixed x position
    pub fy: Option<f64>, // Fixed y position
}

impl SimulationNode {
    pub fn new(index: usize, x: f64, y: f64) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            index,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            fx: None,
            fy: None,
        }))
    }
}

/// A force acting on nodes
pub trait Force {
    fn initialize(&mut self, nodes: &[Rc<RefCell<SimulationNode>>]);
    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]);
}

/// Force simulation engine
pub struct Simulation {
    pub nodes: Vec<Rc<RefCell<SimulationNode>>>,
    pub alpha: f64,
    pub alpha_min: f64,
    pub alpha_decay: f64,
    pub alpha_target: f64,
    pub velocity_decay: f64,
    forces: Vec<Box<dyn Force>>,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            alpha: 1.0,
            alpha_min: 0.001,
            alpha_decay: 1.0 - 0.001f64.powf(1.0 / 300.0),
            alpha_target: 0.0,
            velocity_decay: 0.6,
            forces: Vec::new(),
        }
    }
}

impl Simulation {
    pub fn new(nodes: Vec<Rc<RefCell<SimulationNode>>>) -> Self {
        let mut sim = Self::default();
        sim.nodes = nodes;
        sim
    }

    pub fn force(mut self, force: Box<dyn Force>) -> Self {
        // Initialize force with current nodes
        let mut f = force;
        f.initialize(&self.nodes);
        self.forces.push(f);
        self
    }

    pub fn tick(&mut self) {
        self.alpha += (self.alpha_target - self.alpha) * self.alpha_decay;

        // Apply forces
        for force in &mut self.forces {
            force.force(self.alpha, &self.nodes);
        }

        // Apply velocity and update positions
        for node_rc in &self.nodes {
            let mut node = node_rc.borrow_mut();

            if let Some(fx) = node.fx {
                node.x = fx;
                node.vx = 0.0;
            } else {
                node.vx *= self.velocity_decay;
                node.x += node.vx;
            }

            if let Some(fy) = node.fy {
                node.y = fy;
                node.vy = 0.0;
            } else {
                node.vy *= self.velocity_decay;
                node.y += node.vy;
            }
        }
    }
}

// Built-in forces

/// Centering Force
pub struct ForceCenter {
    pub x: f64,
    pub y: f64,
}

impl ForceCenter {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl Force for ForceCenter {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, _alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        let n = nodes.len() as f64;
        let mut sx = 0.0;
        let mut sy = 0.0;

        for node_rc in nodes {
            let node = node_rc.borrow();
            sx += node.x;
            sy += node.y;
        }

        sx = (sx / n - self.x) * 1.0; // Strength 1.0
        sy = (sy / n - self.y) * 1.0;

        for node_rc in nodes {
            let mut node = node_rc.borrow_mut();
            node.x -= sx;
            node.y -= sy;
        }
    }
}

/// Many-Body Force (Charge)
pub struct ForceManyBody {
    pub strength: f64,
}

impl ForceManyBody {
    pub fn new() -> Self {
        Self { strength: -30.0 }
    }
}

impl Force for ForceManyBody {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        // Brute force O(n^2) for simplicity in this MVP
        // Real D3 uses Barnes-Hut (Quadtree)

        let n = nodes.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let mut node_i = nodes[i].borrow_mut();
                let mut node_j = nodes[j].borrow_mut();

                let dx = node_j.x - node_i.x;
                let dy = node_j.y - node_i.y;
                let mut l2 = dx * dx + dy * dy;

                if l2 == 0.0 {
                    l2 = 1.0; // Avoid division by zero, should use random jiggle
                }

                let w = self.strength * alpha / l2;
                // Ideally should use distance bounds, etc.

                let l = l2.sqrt();
                // Apply force

                node_i.vx += dx / l * w;
                node_i.vy += dy / l * w;

                node_j.vx -= dx / l * w;
                node_j.vy -= dy / l * w;
            }
        }
    }
}
