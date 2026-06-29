/// Spring physics simulation for natural motion
///
/// Simulates a damped spring for smooth, natural animations.
/// Good for drag-and-drop, pull-to-refresh, and other interactive animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Spring stiffness (higher = faster oscillation)
    pub stiffness: f32,
    /// Damping ratio (1.0 = critically damped, <1.0 = oscillates, >1.0 = overdamped)
    pub damping: f32,
    /// Mass of the object
    pub mass: f32,
}

impl Spring {
    /// Create a new spring with custom parameters
    pub fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        assert!(mass > 0.0, "spring mass must be positive, got {}", mass);
        Self {
            stiffness,
            damping,
            mass,
        }
    }

    /// Create a gentle spring (slow, smooth motion)
    pub fn gentle() -> Self {
        Self {
            stiffness: 100.0,
            damping: 15.0,
            mass: 1.0,
        }
    }

    /// Create a wobbly spring (bouncy, playful motion)
    pub fn wobbly() -> Self {
        Self {
            stiffness: 180.0,
            damping: 12.0,
            mass: 1.0,
        }
    }

    /// Create a stiff spring (quick, snappy motion)
    pub fn stiff() -> Self {
        Self {
            stiffness: 400.0,
            damping: 30.0,
            mass: 1.0,
        }
    }

    /// Create a slow spring (deliberate motion)
    pub fn slow() -> Self {
        Self {
            stiffness: 50.0,
            damping: 20.0,
            mass: 1.0,
        }
    }

    /// Calculate the spring force at a given displacement
    pub fn force(&self, displacement: f32, velocity: f32) -> f32 {
        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * velocity;
        let mass = self.mass.max(1e-6);
        (spring_force + damping_force) / mass
    }

    /// Step the spring simulation forward by dt seconds
    ///
    /// Returns the new (position, velocity) tuple
    pub fn step(&self, current: f32, target: f32, velocity: f32, dt: f32) -> (f32, f32) {
        let displacement = current - target;
        let acceleration = self.force(displacement, velocity);
        let new_velocity = velocity + acceleration * dt;
        let new_position = current + new_velocity * dt;
        (new_position, new_velocity)
    }

    /// Check if the spring has settled (velocity and displacement are negligible)
    pub fn is_settled(&self, current: f32, target: f32, velocity: f32, threshold: f32) -> bool {
        (current - target).abs() < threshold && velocity.abs() < threshold
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            stiffness: 170.0,
            damping: 26.0,
            mass: 1.0,
        }
    }
}
