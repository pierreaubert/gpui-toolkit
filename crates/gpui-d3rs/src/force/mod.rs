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
        Self {
            nodes,
            ..Default::default()
        }
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

impl Default for ForceManyBody {
    fn default() -> Self {
        Self { strength: -30.0 }
    }
}

impl ForceManyBody {
    pub fn new() -> Self {
        Self::default()
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

                if l2 < 1e-12 {
                    l2 = 1e-12; // Small epsilon to avoid division by zero
                }

                let l = l2.sqrt();
                // D3.js many-body: F = strength * alpha / l²
                // Direction (dx/l, dy/l), so components = strength * alpha * dx / (l * l²)
                // Simplify: strength * alpha * dx / l³... but D3 actually uses:
                // w = strength * alpha / l, then vx += dx/l * w = dx * strength * alpha / l²
                let w = self.strength * alpha / l;
                let force_x = dx / l * w;
                let force_y = dy / l * w;

                node_i.vx += force_x;
                node_i.vy += force_y;

                node_j.vx -= force_x;
                node_j.vy -= force_y;
            }
        }
    }
}

/// Link Force (Spring)
///
/// Applies spring-like forces along links between nodes, pulling connected
/// nodes toward a target distance. Matches D3's `d3.forceLink()`.
///
/// D3.js behavior:
/// - Default strength is degree-based: `1 / min(degree(source), degree(target))`
/// - Force is distributed with degree bias: hub nodes move less
/// - A custom constant strength can be set with `.strength()`
pub struct ForceLink {
    links: Vec<(usize, usize)>,
    custom_strength: Option<f64>,
    distance: f64,
    iterations: usize,
    // Computed during initialize()
    per_link_strength: Vec<f64>,
    bias: Vec<f64>,
}

impl ForceLink {
    pub fn new(links: Vec<(usize, usize)>) -> Self {
        let n = links.len();
        Self {
            links,
            custom_strength: None,
            distance: 30.0,
            iterations: 1,
            per_link_strength: vec![1.0; n],
            bias: vec![0.5; n],
        }
    }

    pub fn strength(mut self, strength: f64) -> Self {
        self.custom_strength = Some(strength);
        self
    }

    pub fn distance(mut self, distance: f64) -> Self {
        self.distance = distance;
        self
    }

    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }
}

impl Force for ForceLink {
    fn initialize(&mut self, nodes: &[Rc<RefCell<SimulationNode>>]) {
        let n = nodes.len();
        // Compute degree (number of links) for each node
        let mut degree = vec![0usize; n];
        for &(source_idx, target_idx) in &self.links {
            if source_idx < n {
                degree[source_idx] += 1;
            }
            if target_idx < n {
                degree[target_idx] += 1;
            }
        }

        // Compute per-link strength and bias
        self.per_link_strength = Vec::with_capacity(self.links.len());
        self.bias = Vec::with_capacity(self.links.len());
        for &(source_idx, target_idx) in &self.links {
            let sd = degree.get(source_idx).copied().unwrap_or(1).max(1);
            let td = degree.get(target_idx).copied().unwrap_or(1).max(1);

            // D3.js default: 1 / min(degree(source), degree(target))
            let s = self.custom_strength.unwrap_or(1.0 / sd.min(td) as f64);
            self.per_link_strength.push(s);

            // D3.js bias: count[source] / (count[source] + count[target])
            // Higher-degree nodes move less
            self.bias.push(sd as f64 / (sd + td) as f64);
        }
    }

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        let n = nodes.len();
        for _ in 0..self.iterations {
            for (link_idx, &(source_idx, target_idx)) in self.links.iter().enumerate() {
                if source_idx >= n || target_idx >= n {
                    continue;
                }
                let (dx, dy, l) = {
                    let source = nodes[source_idx].borrow();
                    let target = nodes[target_idx].borrow();
                    let dx = target.x - source.x;
                    let dy = target.y - source.y;
                    let l = (dx * dx + dy * dy).sqrt().max(1e-6);
                    (dx, dy, l)
                };

                let strength = self.per_link_strength[link_idx];
                let bias = self.bias[link_idx];
                let f = (l - self.distance) / l * alpha * strength;

                let fx = dx * f;
                let fy = dy * f;

                // Apply with degree-based bias: target gets bias portion,
                // source gets (1-bias) — hub nodes move less
                {
                    let mut target = nodes[target_idx].borrow_mut();
                    target.vx -= fx * bias;
                    target.vy -= fy * bias;
                }
                {
                    let mut source = nodes[source_idx].borrow_mut();
                    source.vx += fx * (1.0 - bias);
                    source.vy += fy * (1.0 - bias);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_link_pulls_nodes_together() {
        // Two nodes far apart, linked with default distance 30
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 100.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        let mut link_force = ForceLink::new(vec![(0, 1)]);
        link_force.initialize(&nodes);

        // Apply force with alpha=1.0
        link_force.force(1.0, &nodes);

        let node0 = n0.borrow();
        let node1 = n1.borrow();

        // Nodes are 100 apart, target is 30, so they should attract
        // source.vx should be positive (pulled toward target)
        assert!(node0.vx > 0.0, "source should be pulled toward target");
        // target.vx should be negative (pulled toward source)
        assert!(node1.vx < 0.0, "target should be pulled toward source");
        // With degree-based bias (both degree=1), bias=0.5 so forces are symmetric
        assert!(
            (node0.vx + node1.vx).abs() < 1e-12,
            "forces should be symmetric for equal-degree nodes"
        );
        // No vertical force for horizontal link
        assert_eq!(node0.vy, 0.0);
        assert_eq!(node1.vy, 0.0);
    }

    #[test]
    fn test_force_link_pushes_nodes_apart_when_too_close() {
        // Two nodes closer than the target distance
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 10.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        let mut link_force = ForceLink::new(vec![(0, 1)]).distance(30.0);
        link_force.initialize(&nodes);
        link_force.force(1.0, &nodes);

        let node0 = n0.borrow();
        let node1 = n1.borrow();

        // Nodes are 10 apart, target is 30, so they should repel
        assert!(node0.vx < 0.0, "source should be pushed away from target");
        assert!(node1.vx > 0.0, "target should be pushed away from source");
    }

    #[test]
    fn test_force_link_no_force_at_rest_distance() {
        // Two nodes exactly at rest distance
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 30.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        let mut link_force = ForceLink::new(vec![(0, 1)]).distance(30.0);
        link_force.initialize(&nodes);
        link_force.force(1.0, &nodes);

        let node0 = n0.borrow();
        let node1 = n1.borrow();

        assert!(node0.vx.abs() < 1e-12, "no force at rest distance");
        assert!(node1.vx.abs() < 1e-12, "no force at rest distance");
    }

    #[test]
    fn test_force_link_multiple_iterations() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 100.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        // Single iteration
        let mut link_1iter = ForceLink::new(vec![(0, 1)]).iterations(1);
        link_1iter.initialize(&nodes);
        link_1iter.force(1.0, &nodes);
        let vx_1iter = n0.borrow().vx;

        // Reset velocities
        n0.borrow_mut().vx = 0.0;
        n1.borrow_mut().vx = 0.0;

        // Three iterations
        let mut link_3iter = ForceLink::new(vec![(0, 1)]).iterations(3);
        link_3iter.initialize(&nodes);
        link_3iter.force(1.0, &nodes);
        let vx_3iter = n0.borrow().vx;

        // More iterations should produce larger velocity change
        assert!(
            vx_3iter.abs() > vx_1iter.abs(),
            "3 iterations ({vx_3iter}) should produce more force than 1 ({vx_1iter})"
        );
    }

    #[test]
    fn test_force_link_diagonal() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 100.0, 100.0);
        let nodes = vec![n0.clone(), n1.clone()];

        let mut link_force = ForceLink::new(vec![(0, 1)]);
        link_force.initialize(&nodes);
        link_force.force(1.0, &nodes);

        let node0 = n0.borrow();
        // Both x and y should be affected for a diagonal link
        assert!(node0.vx > 0.0);
        assert!(node0.vy > 0.0);
        // Equal components due to 45-degree angle
        assert!((node0.vx - node0.vy).abs() < 1e-12);
    }

    #[test]
    fn test_force_many_body_near_zero_distance() {
        // Two nodes almost exactly on top of each other
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 1e-15, 1e-15);
        let nodes = vec![n0.clone(), n1.clone()];

        let mut force = ForceManyBody::new();
        force.force(1.0, &nodes);

        let node0 = n0.borrow();
        let node1 = n1.borrow();

        // Should not produce NaN or infinite velocities
        assert!(node0.vx.is_finite(), "vx should be finite");
        assert!(node0.vy.is_finite(), "vy should be finite");
        assert!(node1.vx.is_finite(), "vx should be finite");
        assert!(node1.vy.is_finite(), "vy should be finite");
    }

    #[test]
    fn test_force_link_in_simulation() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 100.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        let mut sim =
            Simulation::new(nodes).force(Box::new(ForceLink::new(vec![(0, 1)]).distance(30.0)));

        let initial_dist = {
            let a = n0.borrow();
            let b = n1.borrow();
            ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
        };

        for _ in 0..100 {
            sim.tick();
        }

        let final_dist = {
            let a = n0.borrow();
            let b = n1.borrow();
            ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
        };

        // After simulation, nodes should be closer to the target distance of 30
        assert!(
            (final_dist - 30.0).abs() < (initial_dist - 30.0).abs(),
            "nodes should converge toward target distance: initial={initial_dist}, final={final_dist}"
        );
    }
}
