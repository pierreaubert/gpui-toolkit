/// Mouse event state for tracking interactions.
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseState {
    /// Current mouse position in pixels (relative to plot area)
    pub position: Option<(f32, f32)>,
    /// Whether the left mouse button is pressed
    pub left_down: bool,
    /// Whether the right mouse button is pressed
    pub right_down: bool,
    /// Last click timestamp for double-click detection
    pub last_click_time: Option<std::time::Instant>,
    /// Last click position for double-click detection
    pub last_click_pos: Option<(f32, f32)>,
}

impl MouseState {
    /// Check if this is a double-click event.
    ///
    /// Returns true if the click occurred within 300ms and 5 pixels of the last click.
    pub fn is_double_click(&self, x: f32, y: f32) -> bool {
        if let (Some(last_time), Some((last_x, last_y))) =
            (self.last_click_time, self.last_click_pos)
        {
            let elapsed = last_time.elapsed();
            let distance = ((x - last_x).powi(2) + (y - last_y).powi(2)).sqrt();
            elapsed.as_millis() < 300 && distance < 5.0
        } else {
            false
        }
    }

    /// Record a click for double-click detection.
    pub fn record_click(&mut self, x: f32, y: f32) {
        self.last_click_time = Some(std::time::Instant::now());
        self.last_click_pos = Some((x, y));
    }
}
