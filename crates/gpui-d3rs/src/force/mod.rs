//! Force-directed graph layout (d3-force)
//!
//! This module implements a force-directed graph simulation using velocity Verlet integration.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::quadtree::QuadTree;

thread_local! {
    /// Reusable positions buffer for the Barnes-Hut many-body force.
    static MANY_BODY_POSITIONS: RefCell<Vec<(usize, f64, f64)>> = const { RefCell::new(Vec::new()) };
}

/// Recoverable errors for checked force simulation operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ForceError {
    /// Node positions must be finite before checked construction or ticking.
    NonFiniteNodeCoordinate {
        node_index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Node velocities must be finite before checked ticking.
    NonFiniteNodeVelocity {
        node_index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Fixed node positions must be finite when present.
    NonFiniteFixedCoordinate {
        node_index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Simulation parameters such as alpha or decay must be finite.
    NonFiniteSimulationParameter { parameter: &'static str, value: f64 },
    /// Simulation parameters that represent rates cannot be negative.
    NegativeSimulationParameter { parameter: &'static str, value: f64 },
    /// Force-center target coordinates must be finite.
    NonFiniteCenterCoordinate {
        coordinate: &'static str,
        value: f64,
    },
    /// Force-x/force-y target coordinates must be finite.
    NonFinitePositionForceTarget { axis: &'static str, value: f64 },
    /// Force-x/force-y strengths must be finite.
    NonFinitePositionForceStrength { axis: &'static str, value: f64 },
    /// Force-x/force-y strengths cannot be negative.
    NegativePositionForceStrength { axis: &'static str, value: f64 },
    /// Force-radial radius and center parameters must be finite.
    NonFiniteRadialForceParameter { parameter: &'static str, value: f64 },
    /// Force-radial radius and strength cannot be negative.
    NegativeRadialForceParameter { parameter: &'static str, value: f64 },
    /// Force-collide radius and strength must be finite.
    NonFiniteCollideForceParameter { parameter: &'static str, value: f64 },
    /// Per-node force-collide radii must be finite.
    NonFiniteCollideNodeRadius { node_index: usize, value: f64 },
    /// Force-collide radius and strength cannot be negative.
    NegativeCollideForceParameter { parameter: &'static str, value: f64 },
    /// Per-node force-collide radii cannot be negative.
    NegativeCollideNodeRadius { node_index: usize, value: f64 },
    /// Checked per-node force-collide radii must match the node count.
    CollideRadiiLengthMismatch { radii_len: usize, node_count: usize },
    /// Many-body scalar configuration must be finite, except theta/distance_max allow infinity.
    NonFiniteManyBodyParameter { parameter: &'static str, value: f64 },
    /// Many-body distances and theta cannot be negative.
    NegativeManyBodyParameter { parameter: &'static str, value: f64 },
    /// Many-body distance limits must be ordered.
    ReversedManyBodyDistances {
        distance_min: f64,
        distance_max: f64,
    },
    /// Link distances must be finite.
    NonFiniteLinkDistance { value: f64 },
    /// Link distances cannot be negative.
    NegativeLinkDistance { value: f64 },
    /// Link strengths must be finite.
    NonFiniteLinkStrength { value: f64 },
    /// Checked link construction requires endpoints to reference existing nodes.
    LinkEndpointOutOfBounds {
        link_index: usize,
        endpoint: &'static str,
        node_index: usize,
        node_count: usize,
    },
}

impl fmt::Display for ForceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteNodeCoordinate {
                node_index,
                coordinate,
                value,
            } => write!(
                f,
                "force node {node_index} coordinate {coordinate} is not finite: {value}"
            ),
            Self::NonFiniteNodeVelocity {
                node_index,
                coordinate,
                value,
            } => write!(
                f,
                "force node {node_index} velocity {coordinate} is not finite: {value}"
            ),
            Self::NonFiniteFixedCoordinate {
                node_index,
                coordinate,
                value,
            } => write!(
                f,
                "force node {node_index} fixed coordinate {coordinate} is not finite: {value}"
            ),
            Self::NonFiniteSimulationParameter { parameter, value } => {
                write!(
                    f,
                    "force simulation parameter {parameter} is not finite: {value}"
                )
            }
            Self::NegativeSimulationParameter { parameter, value } => {
                write!(
                    f,
                    "force simulation parameter {parameter} is negative: {value}"
                )
            }
            Self::NonFiniteCenterCoordinate { coordinate, value } => {
                write!(
                    f,
                    "force center coordinate {coordinate} is not finite: {value}"
                )
            }
            Self::NonFinitePositionForceTarget { axis, value } => {
                write!(f, "force {axis} target is not finite: {value}")
            }
            Self::NonFinitePositionForceStrength { axis, value } => {
                write!(f, "force {axis} strength is not finite: {value}")
            }
            Self::NegativePositionForceStrength { axis, value } => {
                write!(f, "force {axis} strength is negative: {value}")
            }
            Self::NonFiniteRadialForceParameter { parameter, value } => {
                write!(
                    f,
                    "force radial parameter {parameter} is not finite: {value}"
                )
            }
            Self::NegativeRadialForceParameter { parameter, value } => {
                write!(f, "force radial parameter {parameter} is negative: {value}")
            }
            Self::NonFiniteCollideForceParameter { parameter, value } => {
                write!(
                    f,
                    "force collide parameter {parameter} is not finite: {value}"
                )
            }
            Self::NonFiniteCollideNodeRadius { node_index, value } => {
                write!(
                    f,
                    "force collide radius for node {node_index} is not finite: {value}"
                )
            }
            Self::NegativeCollideForceParameter { parameter, value } => {
                write!(
                    f,
                    "force collide parameter {parameter} is negative: {value}"
                )
            }
            Self::NegativeCollideNodeRadius { node_index, value } => {
                write!(
                    f,
                    "force collide radius for node {node_index} is negative: {value}"
                )
            }
            Self::CollideRadiiLengthMismatch {
                radii_len,
                node_count,
            } => write!(
                f,
                "force collide radii length {radii_len} does not match node count {node_count}"
            ),
            Self::NonFiniteManyBodyParameter { parameter, value } => {
                write!(
                    f,
                    "force many-body parameter {parameter} is not finite: {value}"
                )
            }
            Self::NegativeManyBodyParameter { parameter, value } => {
                write!(
                    f,
                    "force many-body parameter {parameter} is negative: {value}"
                )
            }
            Self::ReversedManyBodyDistances {
                distance_min,
                distance_max,
            } => write!(
                f,
                "force many-body distances are reversed: {distance_min} > {distance_max}"
            ),
            Self::NonFiniteLinkDistance { value } => {
                write!(f, "force link distance is not finite: {value}")
            }
            Self::NegativeLinkDistance { value } => {
                write!(f, "force link distance is negative: {value}")
            }
            Self::NonFiniteLinkStrength { value } => {
                write!(f, "force link strength is not finite: {value}")
            }
            Self::LinkEndpointOutOfBounds {
                link_index,
                endpoint,
                node_index,
                node_count,
            } => write!(
                f,
                "force link {link_index} {endpoint} endpoint {node_index} is outside node count {node_count}"
            ),
        }
    }
}

impl std::error::Error for ForceError {}

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

    pub fn try_new(index: usize, x: f64, y: f64) -> Result<Rc<RefCell<Self>>, ForceError> {
        validate_finite_node_coordinate(index, "x", x)?;
        validate_finite_node_coordinate(index, "y", y)?;
        Ok(Self::new(index, x, y))
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

    pub fn try_new(nodes: Vec<Rc<RefCell<SimulationNode>>>) -> Result<Self, ForceError> {
        validate_nodes(&nodes)?;
        Ok(Self::new(nodes))
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

    pub fn try_tick(&mut self) -> Result<(), ForceError> {
        self.validate_configuration()?;
        validate_nodes(&self.nodes)?;
        self.tick();
        validate_nodes(&self.nodes)?;
        Ok(())
    }

    fn validate_configuration(&self) -> Result<(), ForceError> {
        validate_finite_simulation_parameter("alpha", self.alpha)?;
        validate_finite_simulation_parameter("alpha_min", self.alpha_min)?;
        validate_finite_simulation_parameter("alpha_decay", self.alpha_decay)?;
        validate_finite_simulation_parameter("alpha_target", self.alpha_target)?;
        validate_finite_simulation_parameter("velocity_decay", self.velocity_decay)?;
        validate_non_negative_simulation_parameter("alpha_min", self.alpha_min)?;
        validate_non_negative_simulation_parameter("alpha_decay", self.alpha_decay)?;
        validate_non_negative_simulation_parameter("velocity_decay", self.velocity_decay)?;
        Ok(())
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

    pub fn try_new(x: f64, y: f64) -> Result<Self, ForceError> {
        validate_center_coordinate("x", x)?;
        validate_center_coordinate("y", y)?;
        Ok(Self::new(x, y))
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

/// Positional force that pulls node x-velocity toward a target x coordinate.
pub struct ForceX {
    pub x: f64,
    pub strength: f64,
}

impl ForceX {
    pub fn new(x: f64) -> Self {
        Self { x, strength: 0.1 }
    }

    pub fn try_new(x: f64) -> Result<Self, ForceError> {
        validate_position_force_target("x", x)?;
        Ok(Self::new(x))
    }

    pub fn target(mut self, x: f64) -> Self {
        self.x = x;
        self
    }

    pub fn try_target(mut self, x: f64) -> Result<Self, ForceError> {
        validate_position_force_target("x", x)?;
        self.x = x;
        Ok(self)
    }

    pub fn strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }

    pub fn try_strength(mut self, strength: f64) -> Result<Self, ForceError> {
        validate_position_force_strength("x", strength)?;
        self.strength = strength;
        Ok(self)
    }
}

impl Force for ForceX {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        for node_rc in nodes {
            let mut node = node_rc.borrow_mut();
            node.vx += (self.x - node.x) * self.strength * alpha;
        }
    }
}

/// Positional force that pulls node y-velocity toward a target y coordinate.
pub struct ForceY {
    pub y: f64,
    pub strength: f64,
}

impl ForceY {
    pub fn new(y: f64) -> Self {
        Self { y, strength: 0.1 }
    }

    pub fn try_new(y: f64) -> Result<Self, ForceError> {
        validate_position_force_target("y", y)?;
        Ok(Self::new(y))
    }

    pub fn target(mut self, y: f64) -> Self {
        self.y = y;
        self
    }

    pub fn try_target(mut self, y: f64) -> Result<Self, ForceError> {
        validate_position_force_target("y", y)?;
        self.y = y;
        Ok(self)
    }

    pub fn strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }

    pub fn try_strength(mut self, strength: f64) -> Result<Self, ForceError> {
        validate_position_force_strength("y", strength)?;
        self.strength = strength;
        Ok(self)
    }
}

impl Force for ForceY {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        for node_rc in nodes {
            let mut node = node_rc.borrow_mut();
            node.vy += (self.y - node.y) * self.strength * alpha;
        }
    }
}

/// Radial force that pulls nodes toward a target radius around a center.
pub struct ForceRadial {
    pub radius: f64,
    pub x: f64,
    pub y: f64,
    pub strength: f64,
}

impl ForceRadial {
    pub fn new(radius: f64) -> Self {
        Self {
            radius,
            x: 0.0,
            y: 0.0,
            strength: 0.1,
        }
    }

    pub fn with_center(radius: f64, x: f64, y: f64) -> Self {
        Self {
            radius,
            x,
            y,
            strength: 0.1,
        }
    }

    pub fn try_new(radius: f64) -> Result<Self, ForceError> {
        validate_radial_force_non_negative("radius", radius)?;
        Ok(Self::new(radius))
    }

    pub fn try_with_center(radius: f64, x: f64, y: f64) -> Result<Self, ForceError> {
        validate_radial_force_non_negative("radius", radius)?;
        validate_radial_force_finite("x", x)?;
        validate_radial_force_finite("y", y)?;
        Ok(Self::with_center(radius, x, y))
    }

    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    pub fn try_radius(mut self, radius: f64) -> Result<Self, ForceError> {
        validate_radial_force_non_negative("radius", radius)?;
        self.radius = radius;
        Ok(self)
    }

    pub fn center(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn try_center(mut self, x: f64, y: f64) -> Result<Self, ForceError> {
        validate_radial_force_finite("x", x)?;
        validate_radial_force_finite("y", y)?;
        self.x = x;
        self.y = y;
        Ok(self)
    }

    pub fn x(mut self, x: f64) -> Self {
        self.x = x;
        self
    }

    pub fn try_x(mut self, x: f64) -> Result<Self, ForceError> {
        validate_radial_force_finite("x", x)?;
        self.x = x;
        Ok(self)
    }

    pub fn y(mut self, y: f64) -> Self {
        self.y = y;
        self
    }

    pub fn try_y(mut self, y: f64) -> Result<Self, ForceError> {
        validate_radial_force_finite("y", y)?;
        self.y = y;
        Ok(self)
    }

    pub fn strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }

    pub fn try_strength(mut self, strength: f64) -> Result<Self, ForceError> {
        validate_radial_force_non_negative("strength", strength)?;
        self.strength = strength;
        Ok(self)
    }
}

impl Force for ForceRadial {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        for node_rc in nodes {
            let mut node = node_rc.borrow_mut();
            let mut dx = node.x - self.x;
            let mut dy = node.y - self.y;
            let mut distance = (dx * dx + dy * dy).sqrt();

            if distance < 1e-12 {
                dx = 1e-6;
                dy = 0.0;
                distance = 1e-6;
            }

            let scale = (self.radius - distance) * self.strength * alpha / distance;
            node.vx += dx * scale;
            node.vy += dy * scale;
        }
    }
}

/// Collision force that pushes overlapping nodes apart.
pub struct ForceCollide {
    pub radius: f64,
    pub strength: f64,
    pub iterations: usize,
    radii: Option<Vec<f64>>,
}

impl Default for ForceCollide {
    fn default() -> Self {
        Self {
            radius: 1.0,
            strength: 1.0,
            iterations: 1,
            radii: None,
        }
    }
}

impl ForceCollide {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_radius(radius: f64) -> Self {
        Self {
            radius,
            ..Self::default()
        }
    }

    pub fn try_new() -> Result<Self, ForceError> {
        let force = Self::new();
        force.validate()?;
        Ok(force)
    }

    pub fn try_with_radius(radius: f64) -> Result<Self, ForceError> {
        validate_collide_force_non_negative("radius", radius)?;
        Ok(Self::with_radius(radius))
    }

    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    pub fn try_radius(mut self, radius: f64) -> Result<Self, ForceError> {
        validate_collide_force_non_negative("radius", radius)?;
        self.radius = radius;
        self.radii = None;
        Ok(self)
    }

    pub fn radii(mut self, radii: Vec<f64>) -> Self {
        self.radii = Some(radii);
        self
    }

    pub fn try_radii(mut self, radii: Vec<f64>) -> Result<Self, ForceError> {
        validate_collide_radii(&radii)?;
        self.radii = Some(radii);
        Ok(self)
    }

    pub fn try_radii_for_nodes(
        mut self,
        radii: Vec<f64>,
        node_count: usize,
    ) -> Result<Self, ForceError> {
        validate_collide_radii_for_nodes(&radii, node_count)?;
        self.radii = Some(radii);
        Ok(self)
    }

    pub fn strength(mut self, strength: f64) -> Self {
        self.strength = strength;
        self
    }

    pub fn try_strength(mut self, strength: f64) -> Result<Self, ForceError> {
        validate_collide_force_non_negative("strength", strength)?;
        self.strength = strength;
        Ok(self)
    }

    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn validate(&self) -> Result<(), ForceError> {
        validate_collide_force_non_negative("radius", self.radius)?;
        validate_collide_force_non_negative("strength", self.strength)?;
        if let Some(radii) = &self.radii {
            validate_collide_radii(radii)?;
        }
        Ok(())
    }

    fn radius_for(&self, node_index: usize) -> f64 {
        self.radii
            .as_ref()
            .and_then(|radii| radii.get(node_index))
            .copied()
            .unwrap_or(self.radius)
    }
}

impl Force for ForceCollide {
    fn initialize(&mut self, _nodes: &[Rc<RefCell<SimulationNode>>]) {}

    fn force(&mut self, alpha: f64, nodes: &[Rc<RefCell<SimulationNode>>]) {
        if self.radius <= 0.0 || self.strength == 0.0 {
            return;
        }

        for _ in 0..self.iterations {
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    let radius_i = self.radius_for(i);
                    let radius_j = self.radius_for(j);
                    let min_distance = radius_i + radius_j;
                    if min_distance <= 0.0 {
                        continue;
                    }

                    let mut source = nodes[i].borrow_mut();
                    let mut target = nodes[j].borrow_mut();

                    let mut dx = (target.x + target.vx) - (source.x + source.vx);
                    let mut dy = (target.y + target.vy) - (source.y + source.vy);
                    let mut distance = (dx * dx + dy * dy).sqrt();

                    if distance >= min_distance {
                        continue;
                    }
                    if distance < 1e-12 {
                        dx = 1e-6;
                        dy = 0.0;
                        distance = 1e-6;
                    }

                    let impulse = (min_distance - distance) * self.strength * alpha / distance;
                    let radius_i2 = radius_i * radius_i;
                    let radius_j2 = radius_j * radius_j;
                    let bias = if radius_i2 + radius_j2 > 0.0 {
                        radius_j2 / (radius_i2 + radius_j2)
                    } else {
                        0.5
                    };
                    let fx = dx * impulse;
                    let fy = dy * impulse;

                    source.vx -= fx * bias;
                    source.vy -= fy * bias;
                    target.vx += fx * (1.0 - bias);
                    target.vy += fy * (1.0 - bias);
                }
            }
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

    pub fn try_new() -> Result<Self, ForceError> {
        let force = Self::new();
        force.validate()?;
        Ok(force)
    }

    /// Set the Barnes-Hut opening angle.
    pub fn theta(mut self, theta: f64) -> Self {
        self.theta = theta;
        self
    }

    /// Set the Barnes-Hut opening angle after validation.
    pub fn try_theta(mut self, theta: f64) -> Result<Self, ForceError> {
        validate_many_body_theta(theta)?;
        self.theta = theta;
        self.validate()?;
        Ok(self)
    }

    /// Set the many-body strength after validation.
    pub fn try_strength(mut self, strength: f64) -> Result<Self, ForceError> {
        validate_many_body_finite_parameter("strength", strength)?;
        self.strength = strength;
        self.validate()?;
        Ok(self)
    }

    /// Set the minimum interaction distance.
    pub fn distance_min(mut self, distance_min: f64) -> Self {
        self.distance_min = distance_min;
        self
    }

    /// Set the minimum interaction distance after validation.
    pub fn try_distance_min(mut self, distance_min: f64) -> Result<Self, ForceError> {
        validate_many_body_distance("distance_min", distance_min)?;
        self.distance_min = distance_min;
        self.validate()?;
        Ok(self)
    }

    /// Set the maximum interaction distance.
    pub fn distance_max(mut self, distance_max: f64) -> Self {
        self.distance_max = distance_max;
        self
    }

    /// Set the maximum interaction distance after validation.
    pub fn try_distance_max(mut self, distance_max: f64) -> Result<Self, ForceError> {
        validate_many_body_distance("distance_max", distance_max)?;
        self.distance_max = distance_max;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), ForceError> {
        validate_many_body_finite_parameter("strength", self.strength)?;
        validate_many_body_theta(self.theta)?;
        validate_many_body_distance("distance_min", self.distance_min)?;
        validate_many_body_distance("distance_max", self.distance_max)?;
        if self.distance_min > self.distance_max {
            return Err(ForceError::ReversedManyBodyDistances {
                distance_min: self.distance_min,
                distance_max: self.distance_max,
            });
        }
        Ok(())
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

        MANY_BODY_POSITIONS.with(|positions| {
            let mut positions = positions.borrow_mut();
            positions.clear();
            positions.extend(nodes.iter().enumerate().map(|(i, node)| {
                let node = node.borrow();
                (i, node.x, node.y)
            }));

            let mut tree = QuadTree::from_data(&positions, |p| p.1, |p| p.2);
            tree.compute_aggregates();

            for (i, node_rc) in nodes.iter().enumerate().take(n) {
                let (target_x, target_y) = {
                    let node = node_rc.borrow();
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
                            if target_x >= x0 && target_x <= x1 && target_y >= y0 && target_y <= y1
                            {
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
        });
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

    pub fn try_new_for_nodes(
        links: Vec<(usize, usize)>,
        node_count: usize,
    ) -> Result<Self, ForceError> {
        validate_links_for_nodes(&links, node_count)?;
        Ok(Self::new(links))
    }

    pub fn strength(mut self, strength: f64) -> Self {
        self.custom_strength = Some(strength);
        self
    }

    pub fn try_strength(mut self, strength: f64) -> Result<Self, ForceError> {
        validate_link_strength(strength)?;
        self.custom_strength = Some(strength);
        Ok(self)
    }

    pub fn distance(mut self, distance: f64) -> Self {
        self.distance = distance;
        self
    }

    pub fn try_distance(mut self, distance: f64) -> Result<Self, ForceError> {
        validate_link_distance(distance)?;
        self.distance = distance;
        Ok(self)
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

fn validate_nodes(nodes: &[Rc<RefCell<SimulationNode>>]) -> Result<(), ForceError> {
    for node in nodes {
        let node = node.borrow();
        validate_finite_node_coordinate(node.index, "x", node.x)?;
        validate_finite_node_coordinate(node.index, "y", node.y)?;
        validate_finite_node_velocity(node.index, "vx", node.vx)?;
        validate_finite_node_velocity(node.index, "vy", node.vy)?;
        if let Some(fx) = node.fx {
            validate_finite_fixed_coordinate(node.index, "fx", fx)?;
        }
        if let Some(fy) = node.fy {
            validate_finite_fixed_coordinate(node.index, "fy", fy)?;
        }
    }
    Ok(())
}

fn validate_finite_node_coordinate(
    node_index: usize,
    coordinate: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteNodeCoordinate {
            node_index,
            coordinate,
            value,
        })
    }
}

fn validate_finite_node_velocity(
    node_index: usize,
    coordinate: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteNodeVelocity {
            node_index,
            coordinate,
            value,
        })
    }
}

fn validate_finite_fixed_coordinate(
    node_index: usize,
    coordinate: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteFixedCoordinate {
            node_index,
            coordinate,
            value,
        })
    }
}

fn validate_finite_simulation_parameter(
    parameter: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteSimulationParameter { parameter, value })
    }
}

fn validate_non_negative_simulation_parameter(
    parameter: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    if value < 0.0 {
        Err(ForceError::NegativeSimulationParameter { parameter, value })
    } else {
        Ok(())
    }
}

fn validate_center_coordinate(coordinate: &'static str, value: f64) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteCenterCoordinate { coordinate, value })
    }
}

fn validate_position_force_target(axis: &'static str, value: f64) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFinitePositionForceTarget { axis, value })
    }
}

fn validate_position_force_strength(axis: &'static str, value: f64) -> Result<(), ForceError> {
    if !value.is_finite() {
        return Err(ForceError::NonFinitePositionForceStrength { axis, value });
    }
    if value < 0.0 {
        return Err(ForceError::NegativePositionForceStrength { axis, value });
    }
    Ok(())
}

fn validate_radial_force_finite(parameter: &'static str, value: f64) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteRadialForceParameter { parameter, value })
    }
}

fn validate_radial_force_non_negative(
    parameter: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    validate_radial_force_finite(parameter, value)?;
    if value < 0.0 {
        return Err(ForceError::NegativeRadialForceParameter { parameter, value });
    }
    Ok(())
}

fn validate_collide_force_finite(parameter: &'static str, value: f64) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteCollideForceParameter { parameter, value })
    }
}

fn validate_collide_force_non_negative(
    parameter: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    validate_collide_force_finite(parameter, value)?;
    if value < 0.0 {
        return Err(ForceError::NegativeCollideForceParameter { parameter, value });
    }
    Ok(())
}

fn validate_collide_radii(radii: &[f64]) -> Result<(), ForceError> {
    for (node_index, &value) in radii.iter().enumerate() {
        if !value.is_finite() {
            return Err(ForceError::NonFiniteCollideNodeRadius { node_index, value });
        }
        if value < 0.0 {
            return Err(ForceError::NegativeCollideNodeRadius { node_index, value });
        }
    }
    Ok(())
}

fn validate_collide_radii_for_nodes(radii: &[f64], node_count: usize) -> Result<(), ForceError> {
    validate_collide_radii(radii)?;
    if radii.len() != node_count {
        return Err(ForceError::CollideRadiiLengthMismatch {
            radii_len: radii.len(),
            node_count,
        });
    }
    Ok(())
}

fn validate_many_body_finite_parameter(
    parameter: &'static str,
    value: f64,
) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteManyBodyParameter { parameter, value })
    }
}

fn validate_many_body_theta(theta: f64) -> Result<(), ForceError> {
    if theta.is_nan() {
        return Err(ForceError::NonFiniteManyBodyParameter {
            parameter: "theta",
            value: theta,
        });
    }
    if theta < 0.0 {
        return Err(ForceError::NegativeManyBodyParameter {
            parameter: "theta",
            value: theta,
        });
    }
    Ok(())
}

fn validate_many_body_distance(parameter: &'static str, value: f64) -> Result<(), ForceError> {
    if value.is_nan() || (value.is_infinite() && parameter != "distance_max") {
        return Err(ForceError::NonFiniteManyBodyParameter { parameter, value });
    }
    if value < 0.0 {
        return Err(ForceError::NegativeManyBodyParameter { parameter, value });
    }
    Ok(())
}

fn validate_link_distance(value: f64) -> Result<(), ForceError> {
    if !value.is_finite() {
        return Err(ForceError::NonFiniteLinkDistance { value });
    }
    if value < 0.0 {
        return Err(ForceError::NegativeLinkDistance { value });
    }
    Ok(())
}

fn validate_link_strength(value: f64) -> Result<(), ForceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ForceError::NonFiniteLinkStrength { value })
    }
}

fn validate_links_for_nodes(links: &[(usize, usize)], node_count: usize) -> Result<(), ForceError> {
    for (link_index, &(source, target)) in links.iter().enumerate() {
        if source >= node_count {
            return Err(ForceError::LinkEndpointOutOfBounds {
                link_index,
                endpoint: "source",
                node_index: source,
                node_count,
            });
        }
        if target >= node_count {
            return Err(ForceError::LinkEndpointOutOfBounds {
                link_index,
                endpoint: "target",
                node_index: target,
                node_count,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_simulation_node_rejects_non_finite_coordinates() {
        assert_eq!(
            SimulationNode::try_new(7, f64::INFINITY, 0.0)
                .err()
                .unwrap(),
            ForceError::NonFiniteNodeCoordinate {
                node_index: 7,
                coordinate: "x",
                value: f64::INFINITY
            }
        );

        assert!(matches!(
            SimulationNode::try_new(8, 0.0, f64::NAN),
            Err(ForceError::NonFiniteNodeCoordinate {
                node_index: 8,
                coordinate: "y",
                value,
            }) if value.is_nan()
        ));
    }

    #[test]
    fn checked_simulation_validates_node_state_before_tick() {
        let node = SimulationNode::try_new(0, 0.0, 0.0).unwrap();
        let mut simulation = Simulation::try_new(vec![node.clone()]).unwrap();

        node.borrow_mut().vx = f64::NAN;
        assert!(matches!(
            simulation.try_tick(),
            Err(ForceError::NonFiniteNodeVelocity {
                node_index: 0,
                coordinate: "vx",
                value,
            }) if value.is_nan()
        ));
    }

    #[test]
    fn checked_simulation_validates_configuration_before_tick() {
        let node = SimulationNode::try_new(0, 0.0, 0.0).unwrap();
        let mut simulation = Simulation::try_new(vec![node]).unwrap();
        simulation.alpha_decay = -0.1;

        assert_eq!(
            simulation.try_tick().err().unwrap(),
            ForceError::NegativeSimulationParameter {
                parameter: "alpha_decay",
                value: -0.1
            }
        );
    }

    #[test]
    fn checked_force_center_rejects_non_finite_targets() {
        assert_eq!(
            ForceCenter::try_new(0.0, f64::INFINITY).err().unwrap(),
            ForceError::NonFiniteCenterCoordinate {
                coordinate: "y",
                value: f64::INFINITY
            }
        );
    }

    #[test]
    fn force_x_and_y_pull_nodes_toward_targets() {
        let node = SimulationNode::new(0, 0.0, 0.0);
        let nodes = vec![node.clone()];

        ForceX::new(10.0).strength(0.5).force(1.0, &nodes);
        ForceY::new(-10.0).strength(0.25).force(1.0, &nodes);

        let node = node.borrow();
        assert_eq!(node.vx, 5.0);
        assert_eq!(node.vy, -2.5);
    }

    #[test]
    fn force_x_and_y_respect_alpha() {
        let node = SimulationNode::new(0, 2.0, -2.0);
        let nodes = vec![node.clone()];

        ForceX::new(10.0).strength(0.5).force(0.5, &nodes);
        ForceY::new(10.0).strength(0.25).force(0.5, &nodes);

        let node = node.borrow();
        assert_eq!(node.vx, 2.0);
        assert_eq!(node.vy, 1.5);
    }

    #[test]
    fn checked_position_forces_reject_invalid_targets_and_strengths() {
        assert!(matches!(
            ForceX::try_new(f64::NAN),
            Err(ForceError::NonFinitePositionForceTarget { axis: "x", value }) if value.is_nan()
        ));
        assert_eq!(
            ForceY::new(0.0).try_strength(f64::INFINITY).err().unwrap(),
            ForceError::NonFinitePositionForceStrength {
                axis: "y",
                value: f64::INFINITY
            }
        );
        assert_eq!(
            ForceX::new(0.0).try_strength(-0.1).err().unwrap(),
            ForceError::NegativePositionForceStrength {
                axis: "x",
                value: -0.1
            }
        );
        assert_eq!(ForceY::new(0.0).try_target(3.0).unwrap().y, 3.0);
    }

    #[test]
    fn force_radial_pulls_nodes_toward_radius() {
        let outside = SimulationNode::new(0, 10.0, 0.0);
        let inside = SimulationNode::new(1, 3.0, 4.0);
        let nodes = vec![outside.clone(), inside.clone()];

        ForceRadial::new(5.0).strength(0.2).force(1.0, &nodes);

        assert_eq!(outside.borrow().vx, -1.0);
        assert_eq!(outside.borrow().vy, 0.0);
        assert_eq!(inside.borrow().vx, 0.0);
        assert_eq!(inside.borrow().vy, 0.0);

        ForceRadial::new(10.0).strength(0.5).force(0.5, &nodes);

        let inside = inside.borrow();
        assert_eq!(inside.vx, 0.75);
        assert_eq!(inside.vy, 1.0);
    }

    #[test]
    fn force_radial_uses_configured_center() {
        let node = SimulationNode::new(0, 12.0, 5.0);
        let nodes = vec![node.clone()];

        ForceRadial::with_center(2.0, 10.0, 5.0)
            .strength(0.5)
            .force(1.0, &nodes);

        let node = node.borrow();
        assert_eq!(node.vx, 0.0);
        assert_eq!(node.vy, 0.0);
    }

    #[test]
    fn checked_force_radial_rejects_invalid_configuration() {
        assert_eq!(
            ForceRadial::try_new(f64::INFINITY).err().unwrap(),
            ForceError::NonFiniteRadialForceParameter {
                parameter: "radius",
                value: f64::INFINITY
            }
        );
        assert_eq!(
            ForceRadial::new(0.0).try_radius(-1.0).err().unwrap(),
            ForceError::NegativeRadialForceParameter {
                parameter: "radius",
                value: -1.0
            }
        );
        assert!(matches!(
            ForceRadial::new(0.0).try_center(f64::NAN, 0.0),
            Err(ForceError::NonFiniteRadialForceParameter { parameter: "x", value }) if value.is_nan()
        ));
        assert_eq!(
            ForceRadial::new(0.0).try_strength(-0.1).err().unwrap(),
            ForceError::NegativeRadialForceParameter {
                parameter: "strength",
                value: -0.1
            }
        );
        assert_eq!(
            ForceRadial::try_with_center(2.0, 3.0, 4.0)
                .unwrap()
                .try_x(5.0)
                .unwrap()
                .try_y(6.0)
                .unwrap()
                .y,
            6.0
        );
    }

    #[test]
    fn force_collide_pushes_overlapping_nodes_apart() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 5.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        ForceCollide::with_radius(4.0)
            .strength(1.0)
            .force(1.0, &nodes);

        assert_eq!(n0.borrow().vx, -1.5);
        assert_eq!(n0.borrow().vy, 0.0);
        assert_eq!(n1.borrow().vx, 1.5);
        assert_eq!(n1.borrow().vy, 0.0);
    }

    #[test]
    fn force_collide_respects_alpha_strength_and_iterations() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 5.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        ForceCollide::with_radius(4.0)
            .strength(0.5)
            .iterations(2)
            .force(0.5, &nodes);

        assert!((n0.borrow().vx + 0.65625).abs() < 1e-12);
        assert!((n1.borrow().vx - 0.65625).abs() < 1e-12);
    }

    #[test]
    fn force_collide_ignores_separated_nodes() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 10.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        ForceCollide::with_radius(4.0).force(1.0, &nodes);

        assert_eq!(n0.borrow().vx, 0.0);
        assert_eq!(n1.borrow().vx, 0.0);
    }

    #[test]
    fn force_collide_supports_per_node_radii() {
        let n0 = SimulationNode::new(0, 0.0, 0.0);
        let n1 = SimulationNode::new(1, 5.0, 0.0);
        let nodes = vec![n0.clone(), n1.clone()];

        ForceCollide::new().radii(vec![4.0, 2.0]).force(1.0, &nodes);

        assert!((n0.borrow().vx + 0.2).abs() < 1e-12);
        assert!((n1.borrow().vx - 0.8).abs() < 1e-12);
    }

    #[test]
    fn checked_force_collide_rejects_invalid_configuration() {
        assert_eq!(
            ForceCollide::try_with_radius(f64::INFINITY).err().unwrap(),
            ForceError::NonFiniteCollideForceParameter {
                parameter: "radius",
                value: f64::INFINITY
            }
        );
        assert_eq!(
            ForceCollide::new().try_radius(-1.0).err().unwrap(),
            ForceError::NegativeCollideForceParameter {
                parameter: "radius",
                value: -1.0
            }
        );
        assert!(matches!(
            ForceCollide::new().try_strength(f64::NAN),
            Err(ForceError::NonFiniteCollideForceParameter { parameter: "strength", value }) if value.is_nan()
        ));
        assert!(matches!(
            ForceCollide::new().try_radii(vec![1.0, f64::NAN]),
            Err(ForceError::NonFiniteCollideNodeRadius { node_index: 1, value }) if value.is_nan()
        ));
        assert_eq!(
            ForceCollide::new()
                .try_radii(vec![1.0, -1.0])
                .err()
                .unwrap(),
            ForceError::NegativeCollideNodeRadius {
                node_index: 1,
                value: -1.0
            }
        );
        assert_eq!(
            ForceCollide::new()
                .try_radii_for_nodes(vec![1.0], 2)
                .err()
                .unwrap(),
            ForceError::CollideRadiiLengthMismatch {
                radii_len: 1,
                node_count: 2
            }
        );
        assert_eq!(
            ForceCollide::new().try_strength(-0.1).err().unwrap(),
            ForceError::NegativeCollideForceParameter {
                parameter: "strength",
                value: -0.1
            }
        );
        ForceCollide::try_new().unwrap();
    }

    #[test]
    fn checked_many_body_rejects_invalid_configuration() {
        assert_eq!(
            ForceManyBody::new()
                .try_strength(f64::INFINITY)
                .err()
                .unwrap(),
            ForceError::NonFiniteManyBodyParameter {
                parameter: "strength",
                value: f64::INFINITY
            }
        );
        assert_eq!(
            ForceManyBody::new().try_theta(-0.1).err().unwrap(),
            ForceError::NegativeManyBodyParameter {
                parameter: "theta",
                value: -0.1
            }
        );
        assert_eq!(
            ForceManyBody::new()
                .try_distance_min(f64::INFINITY)
                .err()
                .unwrap(),
            ForceError::NonFiniteManyBodyParameter {
                parameter: "distance_min",
                value: f64::INFINITY
            }
        );
        assert_eq!(
            ForceManyBody::new()
                .try_distance_min(10.0)
                .unwrap()
                .try_distance_max(5.0)
                .err()
                .unwrap(),
            ForceError::ReversedManyBodyDistances {
                distance_min: 10.0,
                distance_max: 5.0
            }
        );

        ForceManyBody::try_new().unwrap();
        ForceManyBody::new()
            .try_theta(f64::INFINITY)
            .unwrap()
            .try_distance_max(f64::INFINITY)
            .unwrap();
    }

    #[test]
    fn checked_force_link_validates_endpoints_distance_and_strength() {
        assert_eq!(
            ForceLink::try_new_for_nodes(vec![(0, 2)], 2).err().unwrap(),
            ForceError::LinkEndpointOutOfBounds {
                link_index: 0,
                endpoint: "target",
                node_index: 2,
                node_count: 2
            }
        );
        assert_eq!(
            ForceLink::new(vec![(0, 1)])
                .try_distance(-1.0)
                .err()
                .unwrap(),
            ForceError::NegativeLinkDistance { value: -1.0 }
        );
        assert!(matches!(
            ForceLink::new(vec![(0, 1)])
                .try_strength(f64::NAN)
                .map(|_| ()),
            Err(ForceError::NonFiniteLinkStrength { value }) if value.is_nan()
        ));
    }

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
