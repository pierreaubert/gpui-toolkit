//! Workflow canvas theme

use crate::theme::Theme;
use gpui::Rgba;

/// Theme for the workflow canvas
#[derive(Debug, Clone)]
pub struct WorkflowTheme {
    // Canvas
    /// Canvas background color
    pub canvas_background: Rgba,
    /// Grid line color
    pub grid_color: Rgba,
    /// Grid line spacing in pixels
    pub grid_spacing: f32,

    // Nodes
    /// Node background color
    pub node_background: Rgba,
    /// Node border color
    pub node_border: Rgba,
    /// Node border color when selected
    pub node_border_selected: Rgba,
    /// Node header background
    pub node_header: Rgba,
    /// Node text color
    pub node_text: Rgba,
    /// Node border radius
    pub node_border_radius: f32,
    /// Node header height in pixels (used for port positioning)
    pub node_header_height: f32,
    /// Node content padding in pixels (py_2 = 8px)
    pub node_content_padding: f32,

    // Ports
    /// Input port color
    pub port_input: Rgba,
    /// Output port color
    pub port_output: Rgba,
    /// Port color when hovered
    pub port_hover: Rgba,
    /// Port color when connection is valid
    pub port_valid: Rgba,
    /// Port color when connection is invalid
    pub port_invalid: Rgba,
    /// Port radius
    pub port_radius: f32,

    // Connections
    /// Connection line color
    pub connection_color: Rgba,
    /// Connection line color when selected
    pub connection_selected: Rgba,
    /// Connection line width for fat links (all channels)
    pub connection_width: f32,
    /// Connection line width for thin links (single channel)
    pub connection_width_thin: f32,
    /// Connection preview color (while dragging)
    pub connection_preview: Rgba,

    // Selection
    /// Selection box fill color
    pub selection_fill: Rgba,
    /// Selection box border color
    pub selection_border: Rgba,
}

impl WorkflowTheme {
    /// Create theme from the global theme
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            // Canvas
            canvas_background: theme.background,
            grid_color: Rgba {
                r: theme.border.r,
                g: theme.border.g,
                b: theme.border.b,
                a: 0.3,
            },
            grid_spacing: 20.0,

            // Nodes
            node_background: theme.surface,
            node_border: theme.border,
            node_border_selected: theme.accent,
            node_header: Rgba {
                r: theme.surface.r * 0.8,
                g: theme.surface.g * 0.8,
                b: theme.surface.b * 0.8,
                a: theme.surface.a,
            },
            node_text: theme.text_primary,
            node_border_radius: 8.0,
            // Header height: py_1 (4px) + text_sm (~20px line height) + py_1 (4px) = 28px
            node_header_height: 28.0,
            // Content padding: py_2 = 8px
            node_content_padding: 8.0,

            // Ports
            port_input: theme.info,
            port_output: theme.success,
            port_hover: theme.accent_hover,
            port_valid: theme.success,
            port_invalid: theme.error,
            port_radius: 6.0,

            // Connections
            connection_color: theme.text_secondary,
            connection_selected: theme.accent,
            connection_width: 4.0,      // Fat links (all channels)
            connection_width_thin: 1.5, // Thin links (single channel)
            connection_preview: Rgba {
                r: theme.accent.r,
                g: theme.accent.g,
                b: theme.accent.b,
                a: 0.6,
            },

            // Selection
            selection_fill: Rgba {
                r: theme.accent.r,
                g: theme.accent.g,
                b: theme.accent.b,
                a: 0.1,
            },
            selection_border: theme.accent,
        }
    }

    /// Create default dark theme
    pub fn dark() -> Self {
        Self::from_theme(&Theme::dark())
    }
    /// Scale the theme dimensions by a factor
    pub fn scale(&self, factor: f32) -> Self {
        let mut scaled = self.clone();
        scaled.grid_spacing *= factor;
        scaled.node_border_radius *= factor;
        scaled.node_header_height *= factor;
        scaled.node_content_padding *= factor;
        scaled.port_radius *= factor;
        scaled.connection_width *= factor;
        scaled.connection_width_thin *= factor;
        scaled
    }
}

impl Default for WorkflowTheme {
    fn default() -> Self {
        Self::dark()
    }
}
