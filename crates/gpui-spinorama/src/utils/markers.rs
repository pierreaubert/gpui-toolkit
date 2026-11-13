//! Match marker convex-hull areas to the existing small triangle.

use d3rs::polygon::{polygon_area, polygon_hull};
use d3rs::shape::{Symbol, SymbolType};

fn hull_area(symbol: SymbolType) -> f64 {
    if symbol == SymbolType::Circle {
        // The line renderer and legend draw a disk using the nominal diameter.
        return std::f64::consts::PI / 4.0;
    }
    let points: Vec<_> = Symbol::new(symbol, 1.0)
        .points()
        .iter()
        .map(|point| (point.x, point.y))
        .collect();
    polygon_area(&polygon_hull(&points)).abs()
}

/// Scales each marker to the small triangle's convex-hull area.
pub(crate) fn scale(symbol: SymbolType) -> f64 {
    (hull_area(SymbolType::Triangle) / hull_area(symbol)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rendered_marker_has_the_triangle_convex_hull_area() {
        let diameter = 7.0;
        let target = hull_area(SymbolType::Triangle) * diameter * diameter;
        for symbol in [
            SymbolType::Circle,
            SymbolType::Cross,
            SymbolType::Diamond,
            SymbolType::Square,
            SymbolType::Star,
            SymbolType::Triangle,
            SymbolType::TriangleDown,
            SymbolType::TriangleLeft,
            SymbolType::TriangleRight,
            SymbolType::Wye,
        ] {
            let adjusted = diameter * scale(symbol);
            let area = if symbol == SymbolType::Circle {
                std::f64::consts::PI * (adjusted / 2.0).powi(2)
            } else {
                let points: Vec<_> = Symbol::new(symbol, adjusted * adjusted)
                    .points()
                    .iter()
                    .map(|point| (point.x, point.y))
                    .collect();
                polygon_area(&polygon_hull(&points)).abs()
            };
            assert!(
                (area - target).abs() < 1e-6,
                "{symbol:?}: {area} vs {target}"
            );
        }
        assert_eq!(scale(SymbolType::Triangle), 1.0);
    }
}
