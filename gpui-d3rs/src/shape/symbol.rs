//! Symbol generators
//!
//! Provides various symbol shapes for scatter plots and data markers.

use std::f64::consts::PI;

use super::path::{Path, PathBuilder, Point};

/// Symbol type for data markers.
///
/// # Example
///
/// ```
/// use d3rs::shape::symbol::{Symbol, SymbolType};
///
/// let circle = Symbol::new(SymbolType::Circle, 64.0);
/// let path = circle.generate();
/// assert!(!path.is_empty());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolType {
    /// Circle symbol
    #[default]
    Circle,
    /// Cross (plus sign)
    Cross,
    /// Diamond shape
    Diamond,
    /// Square shape
    Square,
    /// Star with 5 points
    Star,
    /// Triangle pointing up
    Triangle,
    /// Triangle pointing down
    TriangleDown,
    /// Triangle pointing left
    TriangleLeft,
    /// Triangle pointing right
    TriangleRight,
    /// X shape (rotated cross)
    Wye,
}

/// Symbol generator.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The symbol type
    pub symbol_type: SymbolType,
    /// The area of the symbol in square pixels
    pub size: f64,
}

impl Default for Symbol {
    fn default() -> Self {
        Self {
            symbol_type: SymbolType::Circle,
            size: 64.0,
        }
    }
}

impl Symbol {
    /// Create a new symbol with the given type and size.
    pub fn new(symbol_type: SymbolType, size: f64) -> Self {
        Self { symbol_type, size }
    }

    /// Create a circle symbol.
    pub fn circle(size: f64) -> Self {
        Self::new(SymbolType::Circle, size)
    }

    /// Create a cross symbol.
    pub fn cross(size: f64) -> Self {
        Self::new(SymbolType::Cross, size)
    }

    /// Create a diamond symbol.
    pub fn diamond(size: f64) -> Self {
        Self::new(SymbolType::Diamond, size)
    }

    /// Create a square symbol.
    pub fn square(size: f64) -> Self {
        Self::new(SymbolType::Square, size)
    }

    /// Create a star symbol.
    pub fn star(size: f64) -> Self {
        Self::new(SymbolType::Star, size)
    }

    /// Create a triangle symbol.
    pub fn triangle(size: f64) -> Self {
        Self::new(SymbolType::Triangle, size)
    }

    /// Set the symbol type.
    pub fn symbol_type(mut self, t: SymbolType) -> Self {
        self.symbol_type = t;
        self
    }

    /// Set the symbol size (area in square pixels).
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Generate the symbol path.
    pub fn generate(&self) -> Path {
        match self.symbol_type {
            SymbolType::Circle => self.generate_circle(),
            SymbolType::Cross => self.generate_cross(),
            SymbolType::Diamond => self.generate_diamond(),
            SymbolType::Square => self.generate_square(),
            SymbolType::Star => self.generate_star(),
            SymbolType::Triangle => self.generate_triangle(0.0),
            SymbolType::TriangleDown => self.generate_triangle(PI),
            SymbolType::TriangleLeft => self.generate_triangle(-PI / 2.0),
            SymbolType::TriangleRight => self.generate_triangle(PI / 2.0),
            SymbolType::Wye => self.generate_wye(),
        }
    }

    /// Generate the symbol centered at a specific point.
    pub fn generate_at(&self, x: f64, y: f64) -> Path {
        let base = self.generate();
        let mut builder = PathBuilder::new();

        for cmd in base.commands() {
            match *cmd {
                super::path::PathCommand::MoveTo { x: px, y: py } => {
                    builder = builder.move_to(px + x, py + y);
                }
                super::path::PathCommand::LineTo { x: px, y: py } => {
                    builder = builder.line_to(px + x, py + y);
                }
                super::path::PathCommand::ClosePath => {
                    builder = builder.close_path();
                }
                super::path::PathCommand::Arc {
                    x: cx,
                    y: cy,
                    radius,
                    start_angle,
                    end_angle,
                    anticlockwise,
                } => {
                    builder = builder.arc(
                        cx + x,
                        cy + y,
                        radius,
                        start_angle,
                        end_angle,
                        anticlockwise,
                    );
                }
                _ => {}
            }
        }

        builder.build()
    }

    /// Generate points for the symbol outline.
    pub fn points(&self) -> Vec<Point> {
        match self.symbol_type {
            SymbolType::Circle => self.circle_points(16),
            SymbolType::Cross => self.cross_points(),
            SymbolType::Diamond => self.diamond_points(),
            SymbolType::Square => self.square_points(),
            SymbolType::Star => self.star_points(),
            SymbolType::Triangle
            | SymbolType::TriangleDown
            | SymbolType::TriangleLeft
            | SymbolType::TriangleRight => {
                let rotation = match self.symbol_type {
                    SymbolType::Triangle => 0.0,
                    SymbolType::TriangleDown => PI,
                    SymbolType::TriangleLeft => -PI / 2.0,
                    SymbolType::TriangleRight => PI / 2.0,
                    _ => 0.0,
                };
                self.triangle_points(rotation)
            }
            SymbolType::Wye => self.wye_points(),
        }
    }

    fn generate_circle(&self) -> Path {
        let r = (self.size / PI).sqrt();
        PathBuilder::new()
            .move_to(r, 0.0)
            .arc(0.0, 0.0, r, 0.0, PI, false)
            .arc(0.0, 0.0, r, PI, 2.0 * PI, false)
            .close_path()
            .build()
    }

    fn generate_cross(&self) -> Path {
        let r = (self.size / 5.0).sqrt();
        let r3 = r * 3.0;
        PathBuilder::new()
            .move_to(-r3, -r)
            .line_to(-r, -r)
            .line_to(-r, -r3)
            .line_to(r, -r3)
            .line_to(r, -r)
            .line_to(r3, -r)
            .line_to(r3, r)
            .line_to(r, r)
            .line_to(r, r3)
            .line_to(-r, r3)
            .line_to(-r, r)
            .line_to(-r3, r)
            .close_path()
            .build()
    }

    fn generate_diamond(&self) -> Path {
        let r = (self.size / 2.0).sqrt();
        let r2 = r * 2.0_f64.sqrt();
        PathBuilder::new()
            .move_to(0.0, -r2)
            .line_to(r2, 0.0)
            .line_to(0.0, r2)
            .line_to(-r2, 0.0)
            .close_path()
            .build()
    }

    fn generate_square(&self) -> Path {
        let r = self.size.sqrt() / 2.0;
        PathBuilder::new()
            .move_to(-r, -r)
            .line_to(r, -r)
            .line_to(r, r)
            .line_to(-r, r)
            .close_path()
            .build()
    }

    fn generate_star(&self) -> Path {
        let ka = 0.890_813_091_529_285_2; // sin(π/10) / sin(7π/10)
        let kr = (3.0 - 5.0_f64.sqrt()) / 2.0; // Inner radius ratio
        let r = (self.size * ka).sqrt();
        let r_inner = r * kr;

        let mut builder = PathBuilder::new();
        for i in 0..10 {
            let angle = (i as f64) * PI / 5.0 - PI / 2.0;
            let radius = if i % 2 == 0 { r } else { r_inner };
            let x = radius * angle.cos();
            let y = radius * angle.sin();

            if i == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }

        builder.close_path().build()
    }

    fn generate_triangle(&self, rotation: f64) -> Path {
        let y = -(self.size / (3.0_f64.sqrt() * 3.0)).sqrt();
        let x = y * 3.0_f64.sqrt();

        let points = [
            Point::new(0.0, y * 2.0),
            Point::new(-x, -y),
            Point::new(x, -y),
        ];

        let rotated: Vec<Point> = points
            .iter()
            .map(|p| {
                let cos = rotation.cos();
                let sin = rotation.sin();
                Point::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos)
            })
            .collect();

        PathBuilder::new()
            .move_to(rotated[0].x, rotated[0].y)
            .line_to(rotated[1].x, rotated[1].y)
            .line_to(rotated[2].x, rotated[2].y)
            .close_path()
            .build()
    }

    fn generate_wye(&self) -> Path {
        let r = (self.size / (3.0 + 3.0_f64.sqrt())).sqrt();
        let c = 1.0 / 3.0_f64.sqrt();
        let s = 2.0 / 3.0_f64.sqrt();

        PathBuilder::new()
            .move_to(0.0, -r * 2.0)
            .line_to(-r * c, -r)
            .line_to(-r * s, -r)
            .line_to(-r * s, r)
            .line_to(-r * c, r)
            .line_to(0.0, 0.0)
            .line_to(r * c, r)
            .line_to(r * s, r)
            .line_to(r * s, -r)
            .line_to(r * c, -r)
            .close_path()
            .build()
    }

    fn circle_points(&self, segments: usize) -> Vec<Point> {
        let r = (self.size / PI).sqrt();
        let mut points = Vec::with_capacity(segments + 1);

        for i in 0..=segments {
            let angle = (i as f64) * 2.0 * PI / (segments as f64);
            points.push(Point::new(r * angle.cos(), r * angle.sin()));
        }

        points
    }

    fn cross_points(&self) -> Vec<Point> {
        let r = (self.size / 5.0).sqrt();
        let r3 = r * 3.0;

        vec![
            Point::new(-r3, -r),
            Point::new(-r, -r),
            Point::new(-r, -r3),
            Point::new(r, -r3),
            Point::new(r, -r),
            Point::new(r3, -r),
            Point::new(r3, r),
            Point::new(r, r),
            Point::new(r, r3),
            Point::new(-r, r3),
            Point::new(-r, r),
            Point::new(-r3, r),
            Point::new(-r3, -r),
        ]
    }

    fn diamond_points(&self) -> Vec<Point> {
        let r = (self.size / 2.0).sqrt();
        let r2 = r * 2.0_f64.sqrt();

        vec![
            Point::new(0.0, -r2),
            Point::new(r2, 0.0),
            Point::new(0.0, r2),
            Point::new(-r2, 0.0),
            Point::new(0.0, -r2),
        ]
    }

    fn square_points(&self) -> Vec<Point> {
        let r = self.size.sqrt() / 2.0;

        vec![
            Point::new(-r, -r),
            Point::new(r, -r),
            Point::new(r, r),
            Point::new(-r, r),
            Point::new(-r, -r),
        ]
    }

    fn star_points(&self) -> Vec<Point> {
        let ka = 0.890_813_091_529_285_2;
        let kr = (3.0 - 5.0_f64.sqrt()) / 2.0;
        let r = (self.size * ka).sqrt();
        let r_inner = r * kr;

        let mut points = Vec::with_capacity(11);
        for i in 0..10 {
            let angle = (i as f64) * PI / 5.0 - PI / 2.0;
            let radius = if i % 2 == 0 { r } else { r_inner };
            points.push(Point::new(radius * angle.cos(), radius * angle.sin()));
        }
        points.push(points[0]);
        points
    }

    fn triangle_points(&self, rotation: f64) -> Vec<Point> {
        let y = -(self.size / (3.0_f64.sqrt() * 3.0)).sqrt();
        let x = y * 3.0_f64.sqrt();

        let points = [
            Point::new(0.0, y * 2.0),
            Point::new(-x, -y),
            Point::new(x, -y),
        ];

        let cos = rotation.cos();
        let sin = rotation.sin();

        let mut result: Vec<Point> = points
            .iter()
            .map(|p| Point::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos))
            .collect();

        result.push(result[0]);
        result
    }

    fn wye_points(&self) -> Vec<Point> {
        let r = (self.size / (3.0 + 3.0_f64.sqrt())).sqrt();
        let c = 1.0 / 3.0_f64.sqrt();
        let s = 2.0 / 3.0_f64.sqrt();

        vec![
            Point::new(0.0, -r * 2.0),
            Point::new(-r * c, -r),
            Point::new(-r * s, -r),
            Point::new(-r * s, r),
            Point::new(-r * c, r),
            Point::new(0.0, 0.0),
            Point::new(r * c, r),
            Point::new(r * s, r),
            Point::new(r * s, -r),
            Point::new(r * c, -r),
            Point::new(0.0, -r * 2.0),
        ]
    }
}

/// Get the radius for a symbol with the given size.
///
/// This is useful for positioning labels or calculating hit areas.
pub fn symbol_radius(symbol_type: SymbolType, size: f64) -> f64 {
    match symbol_type {
        SymbolType::Circle => (size / PI).sqrt(),
        SymbolType::Square => size.sqrt() / 2.0,
        SymbolType::Diamond => (size / 2.0).sqrt() * 2.0_f64.sqrt(),
        SymbolType::Cross => (size / 5.0).sqrt() * 3.0,
        SymbolType::Star => {
            let ka = 0.890_813_091_529_285_2;
            (size * ka).sqrt()
        }
        SymbolType::Triangle
        | SymbolType::TriangleDown
        | SymbolType::TriangleLeft
        | SymbolType::TriangleRight => (size / (3.0_f64.sqrt() * 3.0)).sqrt() * 2.0,
        SymbolType::Wye => (size / (3.0 + 3.0_f64.sqrt())).sqrt() * 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_symbol() {
        let symbol = Symbol::circle(64.0);
        let path = symbol.generate();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_cross_symbol() {
        let symbol = Symbol::cross(64.0);
        let path = symbol.generate();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_diamond_symbol() {
        let symbol = Symbol::diamond(64.0);
        let path = symbol.generate();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_square_symbol() {
        let symbol = Symbol::square(64.0);
        let path = symbol.generate();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_star_symbol() {
        let symbol = Symbol::star(64.0);
        let path = symbol.generate();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_triangle_symbol() {
        let symbol = Symbol::triangle(64.0);
        let path = symbol.generate();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_symbol_at_point() {
        let symbol = Symbol::circle(64.0);
        let path = symbol.generate_at(100.0, 100.0);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_symbol_points() {
        let symbol = Symbol::circle(64.0);
        let points = symbol.points();
        assert!(!points.is_empty());
    }

    #[test]
    fn test_symbol_radius() {
        let radius = symbol_radius(SymbolType::Circle, 64.0);
        assert!(radius > 0.0);
    }
}
