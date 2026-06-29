use super::cubehelix::cubehelix_default;
use super::hsl::Hsl;
use super::interpolate::interpolate_hsl;
use super::interpolate::interpolate_rgb;
use super::lab::Lab;
use crate::color::D3Color;

use approx::assert_relative_eq;

#[test]
fn test_hsl_roundtrip() {
    let original = D3Color::rgb(128, 64, 192);
    let hsl = Hsl::from_rgb(&original);
    let result = hsl.to_rgb();

    assert_relative_eq!(original.r, result.r, epsilon = 0.01);
    assert_relative_eq!(original.g, result.g, epsilon = 0.01);
    assert_relative_eq!(original.b, result.b, epsilon = 0.01);
}

#[test]
fn test_lab_roundtrip() {
    let original = D3Color::rgb(128, 64, 192);
    let lab = Lab::from_rgb(&original);
    let result = lab.to_rgb();

    assert_relative_eq!(original.r, result.r, epsilon = 0.02);
    assert_relative_eq!(original.g, result.g, epsilon = 0.02);
    assert_relative_eq!(original.b, result.b, epsilon = 0.02);
}

#[test]
fn test_interpolate_rgb() {
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);
    let interp = interpolate_rgb(red, blue);

    let start = interp(0.0);
    assert_relative_eq!(start.r, 1.0);
    assert_relative_eq!(start.b, 0.0);

    let end = interp(1.0);
    assert_relative_eq!(end.r, 0.0);
    assert_relative_eq!(end.b, 1.0);
}

#[test]
fn test_interpolate_hsl() {
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);
    let interp = interpolate_hsl(red, blue);

    let mid = interp(0.5);
    // Should be a purple/magenta color (around 270 degrees on HSL)
    assert!(mid.r > 0.3);
    assert!(mid.b > 0.3);
}

#[test]
fn test_cubehelix_default() {
    let interp = cubehelix_default();

    let start = interp(0.0);
    let mid = interp(0.5);
    let end = interp(1.0);

    // Start should be dark, end should be light
    let start_lum = 0.299 * start.r as f64 + 0.587 * start.g as f64 + 0.114 * start.b as f64;
    let end_lum = 0.299 * end.r as f64 + 0.587 * end.g as f64 + 0.114 * end.b as f64;

    assert!(start_lum < mid.g as f64);
    assert!(end_lum > mid.g as f64);
}

#[test]
fn test_hcl_roundtrip() {
    let original = D3Color::rgb(128, 64, 192);
    let hcl = super::hcl::Hcl::from_rgb(&original);
    let result = hcl.to_rgb();

    assert_relative_eq!(original.r, result.r, epsilon = 0.02);
    assert_relative_eq!(original.g, result.g, epsilon = 0.02);
    assert_relative_eq!(original.b, result.b, epsilon = 0.02);
}

#[test]
fn test_hcl_from_lab_zero_chroma() {
    use super::lab::Lab;

    let gray = Lab {
        l: 50.0,
        a: 0.0,
        b: 0.0,
        alpha: 1.0,
    };
    let hcl = super::hcl::Hcl::from_lab(&gray);
    assert_eq!(hcl.c, 0.0);
    assert_eq!(hcl.h, 0.0);
}

#[test]
fn test_hcl_alpha_preserved() {
    use super::lab::Lab;

    let lab = Lab {
        l: 50.0,
        a: 10.0,
        b: 10.0,
        alpha: 0.5,
    };
    let hcl = super::hcl::Hcl::from_lab(&lab);
    assert_relative_eq!(hcl.alpha, 0.5, epsilon = 1e-10);
}

#[test]
fn test_interpolate_hsl_long() {
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);
    let interp = super::interpolate::interpolate_hsl_long(red, blue);

    let start = interp(0.0);
    let end = interp(1.0);
    assert_relative_eq!(start.r, red.r, epsilon = 0.01);
    assert_relative_eq!(end.b, blue.b, epsilon = 0.01);
}

#[test]
fn test_interpolate_hcl() {
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);
    let interp = super::interpolate::interpolate_hcl(red, blue);

    let mid = interp(0.5);
    assert!(mid.r > 0.2 || mid.b > 0.2);
}

#[test]
fn test_interpolate_hcl_long() {
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);
    let interp = super::interpolate::interpolate_hcl_long(red, blue);

    let start = interp(0.0);
    let end = interp(1.0);
    assert_relative_eq!(start.r, red.r, epsilon = 0.01);
    assert_relative_eq!(end.b, blue.b, epsilon = 0.01);
}

#[test]
fn test_interpolate_lab_endpoints() {
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);
    let interp = super::interpolate::interpolate_lab(red, blue);

    let start = interp(0.0);
    let end = interp(1.0);
    assert_relative_eq!(start.r, red.r, epsilon = 0.01);
    assert_relative_eq!(end.b, blue.b, epsilon = 0.01);
}

#[test]
fn test_interpolate_cubehelix() {
    let black = D3Color::rgb(0, 0, 0);
    let white = D3Color::rgb(255, 255, 255);
    let interp = super::interpolate::interpolate_cubehelix(black, white);

    let start = interp(0.0);
    let end = interp(1.0);
    assert_relative_eq!(start.r, black.r, epsilon = 0.01);
    assert_relative_eq!(end.r, white.r, epsilon = 0.01);
}

#[test]
fn test_interpolate_cubehelix_long() {
    let black = D3Color::rgb(0, 0, 0);
    let white = D3Color::rgb(255, 255, 255);
    let interp = super::interpolate::interpolate_cubehelix_long(black, white);

    let start = interp(0.0);
    let end = interp(1.0);
    assert_relative_eq!(start.r, black.r, epsilon = 0.01);
    assert_relative_eq!(end.r, white.r, epsilon = 0.01);
}

#[test]
fn test_cubehelix_custom() {
    let interp = super::cubehelix::cubehelix_custom(0.0, 1.0, 1.0, 1.0);
    let start = interp(0.0);
    let end = interp(1.0);

    assert_relative_eq!(start.r, 0.0, epsilon = 0.01);
    assert_relative_eq!(end.r, 1.0, epsilon = 0.01);
}
