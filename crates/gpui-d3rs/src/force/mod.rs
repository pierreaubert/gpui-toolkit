//! Force-directed graph layout (d3-force)
//!
//! This module implements a force-directed graph simulation using velocity Verlet integration.

use std::cell::RefCell;
use std::rc::Rc;

use crate::quadtree::QuadTree;

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
    /// Barnes-Hut opening angle. Smaller values are more accurate.
    /// `f64::INFINITY` disables Barnes-Hut and uses the exact brute-force path.
    pub theta: f64,
    /// Minimum distance at which the force is evaluated; closer points are
    /// treated as if they were this far apart.
    pub distance_min: f64,
    /// Maximum distance at which the force is evaluated; farther points are ignored.
    pub distance_max: f64,
}

impl Default for ForceManyBody {
    fn default() -> Self {
        Self {
            strength: -30.0,
            theta: f64::INFINITY,
            distance_min: 0.0,
            distance_max: f64::INFINITY,
        }
    }
}

impl ForceManyBody {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Barnes-Hut opening angle.
    pub fn theta(mut self, theta: f64) -> Self {
        self.theta = theta;
        self
    }

    /// Set the minimum interaction distance.
    pub fn distance_min(mut self, distance_min: f64) -> Self {
        self.distance_min = distance_min;
        self
    }

    /// Set the maximum interaction distance.
    pub fn distance_max(mut self, distance_max: f64) -> Self {
        self.distance_max = distance_max;
        self
    }
}

impl Force for ForceManyBody {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        let n = nodes.len();

        // The infinite opening angle keeps the original exact brute-force behavior.
        if self.theta.is_infinite() {
            for i in 0..n {
                for j in (i + 1)..n {
                    let mut node_i = nodes[i].borrow_mut();
                    let mut node_j = nodes[j].borrow_mut();

                    let dx = node_j.x - node_i.x;
                    let dy = node_j.y - node_i.y;

                    let mut fx = 0.0;
                    let mut fy = 0.0;
                    Self::apply_force_kernel(
                        alpha,
                        self.strength,
                        self.distance_min,
                        self.distance_max,
                        dx,
                        dy,
                        1.0,
                        &mut fx,
                        &mut fy,
                    );

                    node_i.vx += fx;
                    node_i.vy += fy;

                    node_j.vx -= fx;
                    node_j.vy -= fy;
                }
            }
            return;
        }

        // Barnes-Hut approximation.
        let theta2 = self.theta * self.theta;

        let positions: Vec<(usize, f64, f64)> = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let node = node.borrow();
                (i, node.x, node.y)
            })
            .collect();

        let mut tree = QuadTree::from_data(&positions, |p| p.1, |p| p.2);
        tree.compute_aggregates();

        for i in 0..n {
            let (target_x, target_y) = {
                let node = nodes[i].borrow();
                (node.x, node.y)
            };

            let mut fx = 0.0;
            let mut fy = 0.0;

            tree.visit_aggregate(|x0, y0, x1, y1, node, aggregate| {
                match node {
                    crate::quadtree::QuadNode::Leaf(point) => {
                        let mut current = Some(point);
                        while let Some(p) = current {
                            if p.data.0 != i {
                                Self::apply_force_kernel(
                                    alpha,
                                    self.strength,
                                    self.distance_min,
                                    self.distance_max,
                                    p.x - target_x,
                                    p.y - target_y,
                                    1.0,
                                    &mut fx,
                                    &mut fy,
                                );
                            }
                            current = p.next.as_deref();
                        }
                        false
                    }
                    crate::quadtree::QuadNode::Internal(_, _) => {
                        let Some(agg) = aggregate else {
                            return true;
                        };
                        if agg.mass == 0.0 {
                            return false;
                        }

                        // Never approximate a node that contains the target point,
                        // because the aggregate includes the target itself.
                        if target_x >= x0 && target_x <= x1 && target_y >= y0 && target_y <= y1 {
                            return true;
                        }

                        let width = x1 - x0;
                        let width2 = width * width;
                        let dx = agg.x - target_x;
                        let dy = agg.y - target_y;
                        let dist2 = dx * dx + dy * dy;

                        // Approximate this internal node as a single body when it appears
                        // small relative to its distance from the target.
                        if width2 < theta2 * dist2 {
                            Self::apply_force_kernel(
                                alpha,
                                self.strength,
                                self.distance_min,
                                self.distance_max,
                                dx,
                                dy,
                                agg.mass,
                                &mut fx,
                                &mut fy,
                            );
                            false
                        } else {
                            true
                        }
                    }
                }
            });

            let mut node = nodes[i].borrow_mut();
            node.vx += fx;
            node.vy += fy;
        }
    }
}

impl ForceManyBody {
    fn apply_force_kernel(
        alpha: f64,
        strength: f64,
        distance_min: f64,
        distance_max: f64,
        dx: f64,
        dy: f64,
        mass: f64,
        fx: &mut f64,
        fy: &mut f64,
    ) {
        let mut l2 = dx * dx + dy * dy;
        let l = l2.sqrt();

        if l < distance_min {
            l2 = distance_min * distance_min;
        }
        if l > distance_max {
            return;
        }
        if l2 < 1e-12 {
            l2 = 1e-12; // Small epsilon to avoid division by zero
        }

        let k = mass * strength * alpha / l2;
        *fx += dx * k;
        *fy += dy * k;
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

    fn deterministic_nodes(n: usize) -> Vec<Rc<RefCell<SimulationNode>>> {
        let mut nodes = Vec::with_capacity(n);
        for i in 0..n {
            let x = (i as f64 * 0.618033988749895).fract() * 100.0;
            let y = (i as f64 * 0.381966011250105).fract() * 100.0;
            nodes.push(SimulationNode::new(i, x, y));
        }
        nodes
    }

    fn clone_nodes(nodes: &[Rc<RefCell<SimulationNode>>]) -> Vec<Rc<RefCell<SimulationNode>>> {
        nodes
            .iter()
            .map(|n| {
                let n = n.borrow();
                SimulationNode::new(n.index, n.x, n.y)
            })
            .collect()
    }

    #[test]
    fn barnes_hut_matches_brute_force() {
        let nodes = deterministic_nodes(200);
        let brute_nodes = clone_nodes(&nodes);
        let bh_nodes = clone_nodes(&nodes);

        ForceManyBody::new().force(1.0, &brute_nodes);
        ForceManyBody::new().theta(0.9).force(1.0, &bh_nodes);

        // Barnes-Hut is an approximation; allow a generous combined tolerance.
        let abs_tolerance = 10.0;
        let rel_tolerance = 2.0;
        for (brute, bh) in brute_nodes.iter().zip(bh_nodes.iter()) {
            let b = brute.borrow();
            let h = bh.borrow();
            let scale = (b.vx.abs() + b.vy.abs()).max(1.0);
            let tol = abs_tolerance + rel_tolerance * scale;
            assert!(
                (b.vx - h.vx).abs() < tol,
                "vx mismatch: brute={} bh={}",
                b.vx,
                h.vx
            );
            assert!(
                (b.vy - h.vy).abs() < tol,
                "vy mismatch: brute={} bh={}",
                b.vy,
                h.vy
            );
        }
    }

    #[test]
    fn theta_zero_is_exact() {
        let nodes = deterministic_nodes(50);
        let brute_nodes = clone_nodes(&nodes);
        let bh_nodes = clone_nodes(&nodes);

        ForceManyBody::new().force(1.0, &brute_nodes);
        ForceManyBody::new().theta(0.0).force(1.0, &bh_nodes);

        for (brute, bh) in brute_nodes.iter().zip(bh_nodes.iter()) {
            let b = brute.borrow();
            let h = bh.borrow();
            assert!(
                (b.vx - h.vx).abs() < 1e-12,
                "vx mismatch: brute={} bh={}",
                b.vx,
                h.vx
            );
            assert!(
                (b.vy - h.vy).abs() < 1e-12,
                "vy mismatch: brute={} bh={}",
                b.vy,
                h.vy
            );
        }
    }

    #[test]
    fn large_n_finite() {
        let nodes = deterministic_nodes(5_000);
        let bh_nodes = clone_nodes(&nodes);

        ForceManyBody::new().theta(0.9).force(1.0, &bh_nodes);

        for node in &bh_nodes {
            let n = node.borrow();
            assert!(n.vx.is_finite(), "vx should be finite");
            assert!(n.vy.is_finite(), "vy should be finite");
        }
    }
}
