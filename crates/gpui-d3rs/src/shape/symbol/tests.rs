use super::Symbol;
use super::SymbolGenerationError;
use super::try_symbol_radius;
use super::types::SymbolType;
use super::types::symbol_radius;

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

#[test]
fn test_all_symbol_types_generate() {
    for symbol_type in [
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
        let symbol = Symbol::new(symbol_type, 64.0);
        let path = symbol.generate();
        assert!(!path.is_empty(), "{symbol_type:?} should generate a path");
    }
}

#[test]
fn test_all_symbol_types_points() {
    for symbol_type in [
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
        let symbol = Symbol::new(symbol_type, 64.0);
        let points = symbol.points();
        assert!(!points.is_empty(), "{symbol_type:?} should produce points");
    }
}

#[test]
fn test_symbol_radius_all_types() {
    for symbol_type in [
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
        let radius = symbol_radius(symbol_type, 100.0);
        assert!(radius > 0.0, "{symbol_type:?} should have positive radius");
    }
}

#[test]
fn test_symbol_generate_at_translates() {
    let symbol = Symbol::square(64.0);
    let at_origin = symbol.generate();
    let translated = symbol.generate_at(100.0, 200.0);

    let origin_bounds = at_origin.bounds().unwrap();
    let translated_bounds = translated.bounds().unwrap();

    assert!((translated_bounds.0 - (origin_bounds.0 + 100.0)).abs() < 1e-9);
    assert!((translated_bounds.1 - (origin_bounds.1 + 200.0)).abs() < 1e-9);
}

#[test]
fn test_symbol_setters() {
    let symbol = Symbol::new(SymbolType::Circle, 16.0)
        .symbol_type(SymbolType::Star)
        .size(64.0);
    assert_eq!(symbol.symbol_type, SymbolType::Star);
    assert_eq!(symbol.size, 64.0);
}

#[test]
fn try_generate_matches_generate_for_valid_symbols() {
    for symbol_type in [
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
        let symbol = Symbol::new(symbol_type, 64.0);
        assert_eq!(
            symbol.generate().commands(),
            symbol.try_generate().unwrap().commands(),
            "{symbol_type:?} checked path should match permissive path"
        );
        assert_eq!(symbol.points(), symbol.try_points().unwrap());
        assert_eq!(
            symbol_radius(symbol_type, 64.0),
            symbol.try_radius().unwrap()
        );
    }
}

#[test]
fn try_generate_rejects_invalid_size() {
    let error = Symbol::circle(f64::NAN).try_generate().unwrap_err();
    match error {
        SymbolGenerationError::NonFiniteSize { size } => assert!(size.is_nan()),
        _ => panic!("unexpected error: {error:?}"),
    }

    assert_eq!(
        Symbol::square(-1.0).try_points().unwrap_err(),
        SymbolGenerationError::NegativeSize { size: -1.0 }
    );
    assert_eq!(
        try_symbol_radius(SymbolType::Star, f64::INFINITY).unwrap_err(),
        SymbolGenerationError::NonFiniteSize {
            size: f64::INFINITY
        }
    );
}

#[test]
fn try_generate_at_rejects_non_finite_coordinates() {
    let symbol = Symbol::circle(64.0);

    assert_eq!(
        symbol.try_generate_at(f64::INFINITY, 0.0).unwrap_err(),
        SymbolGenerationError::NonFiniteCoordinate {
            coordinate: "x",
            value: f64::INFINITY,
        }
    );

    let error = symbol.try_generate_at(0.0, f64::NAN).unwrap_err();
    match error {
        SymbolGenerationError::NonFiniteCoordinate { coordinate, value } => {
            assert_eq!(coordinate, "y");
            assert!(value.is_nan());
        }
        _ => panic!("unexpected error: {error:?}"),
    }
}

#[test]
fn try_generate_at_matches_generate_at_for_valid_inputs() {
    let symbol = Symbol::diamond(64.0);

    assert_eq!(
        symbol.generate_at(10.0, 20.0).commands(),
        symbol.try_generate_at(10.0, 20.0).unwrap().commands()
    );
}
