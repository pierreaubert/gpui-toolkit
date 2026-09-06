use super::diverging_scheme::DivergingScheme;
use super::sequential_scheme::SequentialScheme;

#[test]
fn test_sequential_scale_get() {
    let scale = SequentialScheme::blues();
    let c = scale.get(0.0);
    assert!(c.r >= 0.0 && c.r <= 1.0);

    let c = scale.get(1.0);
    assert!(c.r >= 0.0 && c.r <= 1.0);
}

#[test]
fn test_sequential_scale_midpoint() {
    let scale = SequentialScheme::viridis();
    let c = scale.get(0.5);
    assert!(c.r >= 0.0 && c.r <= 1.0);
    assert!(c.g >= 0.0 && c.g <= 1.0);
    assert!(c.b >= 0.0 && c.b <= 1.0);
}

#[test]
fn test_sequential_sample() {
    let scale = SequentialScheme::greens();
    let samples = scale.sample(5);
    assert_eq!(samples.len(), 5);
}

#[test]
fn test_diverging_scale_get() {
    let scale = DivergingScheme::rd_bu();
    let c = scale.get(0.0);
    assert!(c.r >= 0.0 && c.r <= 1.0);

    let c = scale.get(0.5);
    assert!(c.r >= 0.0 && c.r <= 1.0);
    assert!(c.g >= 0.0 && c.g <= 1.0);
    assert!(c.b >= 0.0 && c.b <= 1.0);

    let c = scale.get(1.0);
    assert!(c.r >= 0.0 && c.r <= 1.0);
}

#[test]
fn test_diverging_sample() {
    let scale = DivergingScheme::spectral();
    let samples = scale.sample(5);
    assert_eq!(samples.len(), 5);
}

#[test]
fn test_sequential_get_by_name() {
    let scale = SequentialScheme::get("Viridis").unwrap();
    let c = scale.get(0.5);
    assert!(c.r >= 0.0 && c.r <= 1.0);
}

#[test]
fn test_diverging_get_by_name() {
    let scale = DivergingScheme::get("RdBu").unwrap();
    let c = scale.get(0.5);
    assert!(c.r >= 0.0 && c.r <= 1.0);
}

#[test]
fn test_sequential_scale_bounds() {
    let scale = SequentialScheme::magma();
    let c0 = scale.get(-0.5);
    let c1 = scale.get(1.5);
    let c_mid = scale.get(0.5);

    assert!(c0.r >= 0.0 && c0.r <= 1.0);
    assert!(c1.r >= 0.0 && c1.r <= 1.0);
    assert!(c_mid.r >= 0.0 && c_mid.r <= 1.0);
}

#[test]
fn test_diverging_scale_bounds() {
    let scale = DivergingScheme::pi_yg();
    let c0 = scale.get(-0.5);
    let c1 = scale.get(1.5);
    let c_mid = scale.get(0.5);

    assert!(c0.r >= 0.0 && c0.r <= 1.0);
    assert!(c1.r >= 0.0 && c1.r <= 1.0);
    assert!(c_mid.r >= 0.0 && c_mid.r <= 1.0);
}

#[test]
fn test_sequential_rainbow_wraps() {
    let scale = SequentialScheme::rainbow();
    // Cyclic endpoints meet: t=0 and t=1 are the same hue.
    let c0 = scale.get(0.0);
    let c1 = scale.get(1.0);
    assert!((c0.r - c1.r).abs() < 1e-9);
    assert!((c0.g - c1.g).abs() < 1e-9);
    assert!((c0.b - c1.b).abs() < 1e-9);
    // Mid-sweep is vivid (high chroma), not grey.
    let mid = scale.get(0.25);
    let range = (mid.r - mid.g).abs().max((mid.g - mid.b).abs()).max((mid.b - mid.r).abs());
    assert!(range > 0.2);
    assert!(SequentialScheme::get("Rainbow").is_some());
}
