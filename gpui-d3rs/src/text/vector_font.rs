//! Hershey Simplex vector font for rendering rotatable text
//!
//! Based on the Hershey font developed by Dr. A. V. Hershey in 1967.
//! Font data from https://paulbourke.net/dataformats/hershey/
//! Public domain - no usage restrictions.

use gpui::prelude::*;
use gpui::{Hsla, PathBuilder, canvas, hsla, point, px};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::LazyLock;

/// Configuration for vector font rendering
#[derive(Clone)]
pub struct VectorFontConfig {
    /// Font height in pixels
    pub font_size: f32,
    /// Line stroke width
    pub stroke_width: f32,
    /// Text color
    pub color: Hsla,
    /// Rotation angle in radians (0 = horizontal, -PI/2 = vertical bottom-to-top)
    pub rotation: f32,
    /// Character spacing as fraction of font size
    pub letter_spacing: f32,
}

impl Default for VectorFontConfig {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            stroke_width: 1.0,
            color: hsla(0.0, 0.0, 0.7, 1.0),
            rotation: 0.0,
            letter_spacing: 0.1,
        }
    }
}

impl VectorFontConfig {
    /// Create config for horizontal text (default orientation)
    pub fn horizontal(font_size: f32, color: Hsla) -> Self {
        Self {
            font_size,
            stroke_width: 1.2,
            color,
            rotation: 0.0,
            letter_spacing: 0.1,
        }
    }

    /// Create config for vertical text reading bottom-to-top
    pub fn vertical_bottom_to_top(font_size: f32, color: Hsla) -> Self {
        Self {
            font_size,
            stroke_width: 1.5,
            color,
            rotation: -PI / 2.0, // -90 degrees to read bottom-to-top
            letter_spacing: 0.1,
        }
    }
}

/// Hershey Simplex font character definition
/// Data format: (width, points as x,y pairs where -1,-1 = pen up)
struct HersheyChar {
    width: i32,
    data: &'static [i32],
}

/// Static lookup table for Hershey Simplex font data
/// Using LazyLock + HashMap instead of large match statement to avoid
/// stack overflow in gpui_macros proc macro during debug compilation.
static HERSHEY_FONT: LazyLock<HashMap<char, (i32, &'static [i32])>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(' ', (16, &[] as &[i32]));
    map.insert(
        '!',
        (
            10,
            &[5, 21, 5, 7, -1, -1, 5, 2, 4, 1, 5, 0, 6, 1, 5, 2] as &[i32],
        ),
    );
    map.insert('"', (16, &[4, 21, 4, 14, -1, -1, 12, 21, 12, 14] as &[i32]));
    map.insert(
        '#',
        (
            21,
            &[
                11, 25, 4, -7, -1, -1, 17, 25, 10, -7, -1, -1, 4, 12, 18, 12, -1, -1, 3, 6, 17, 6,
            ] as &[i32],
        ),
    );
    map.insert(
        '$',
        (
            20,
            &[
                8, 25, 8, -4, -1, -1, 12, 25, 12, -4, -1, -1, 17, 18, 15, 20, 12, 21, 8, 21, 5, 20,
                3, 18, 3, 16, 4, 14, 5, 13, 7, 12, 13, 10, 15, 9, 16, 8, 17, 6, 17, 3, 15, 1, 12,
                0, 8, 0, 5, 1, 3, 3,
            ] as &[i32],
        ),
    );
    map.insert(
        '%',
        (
            24,
            &[
                21, 21, 3, 0, -1, -1, 8, 21, 10, 19, 10, 17, 9, 15, 7, 14, 5, 14, 3, 16, 3, 18, 4,
                20, 6, 21, 8, 21, 10, 20, 13, 19, 16, 19, 19, 20, 21, 21, -1, -1, 17, 7, 15, 6, 14,
                4, 14, 2, 16, 0, 18, 0, 20, 1, 21, 3, 21, 5, 19, 7, 17, 7,
            ] as &[i32],
        ),
    );
    map.insert(
        '&',
        (
            26,
            &[
                23, 12, 23, 13, 22, 14, 21, 14, 20, 13, 19, 11, 17, 6, 15, 3, 13, 1, 11, 0, 7, 0,
                5, 1, 4, 2, 3, 4, 3, 6, 4, 8, 5, 9, 12, 13, 13, 14, 14, 16, 14, 18, 13, 20, 11, 21,
                9, 20, 8, 18, 8, 16, 9, 13, 11, 10, 16, 3, 18, 1, 20, 0, 22, 0, 23, 1, 23, 2,
            ] as &[i32],
        ),
    );
    map.insert(
        '\'',
        (
            10,
            &[5, 19, 4, 20, 5, 21, 6, 20, 6, 18, 5, 16, 4, 15] as &[i32],
        ),
    );
    map.insert(
        '(',
        (
            14,
            &[
                11, 25, 9, 23, 7, 20, 5, 16, 4, 11, 4, 7, 5, 2, 7, -2, 9, -5, 11, -7,
            ] as &[i32],
        ),
    );
    map.insert(
        ')',
        (
            14,
            &[
                3, 25, 5, 23, 7, 20, 9, 16, 10, 11, 10, 7, 9, 2, 7, -2, 5, -5, 3, -7,
            ] as &[i32],
        ),
    );
    map.insert(
        '*',
        (
            16,
            &[8, 21, 8, 9, -1, -1, 3, 18, 13, 12, -1, -1, 13, 18, 3, 12] as &[i32],
        ),
    );
    map.insert('+', (26, &[13, 18, 13, 0, -1, -1, 4, 9, 22, 9] as &[i32]));
    map.insert(
        ',',
        (
            10,
            &[6, 1, 5, 0, 4, 1, 5, 2, 6, 1, 6, -1, 5, -3, 4, -4] as &[i32],
        ),
    );
    map.insert('-', (26, &[4, 9, 22, 9] as &[i32]));
    map.insert('.', (10, &[5, 2, 4, 1, 5, 0, 6, 1, 5, 2] as &[i32]));
    map.insert('/', (22, &[20, 25, 2, -7] as &[i32]));
    map.insert(
        '0',
        (
            20,
            &[
                9, 21, 6, 20, 4, 17, 3, 12, 3, 9, 4, 4, 6, 1, 9, 0, 11, 0, 14, 1, 16, 4, 17, 9, 17,
                12, 16, 17, 14, 20, 11, 21, 9, 21,
            ] as &[i32],
        ),
    );
    map.insert('1', (20, &[6, 17, 8, 18, 11, 21, 11, 0] as &[i32]));
    map.insert(
        '2',
        (
            20,
            &[
                4, 16, 4, 17, 5, 19, 6, 20, 8, 21, 12, 21, 14, 20, 15, 19, 16, 17, 16, 15, 15, 13,
                13, 10, 3, 0, 17, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        '3',
        (
            20,
            &[
                5, 21, 16, 21, 10, 13, 13, 13, 15, 12, 16, 11, 17, 8, 17, 6, 16, 3, 14, 1, 11, 0,
                8, 0, 5, 1, 4, 2, 3, 4,
            ] as &[i32],
        ),
    );
    map.insert(
        '4',
        (20, &[13, 21, 3, 7, 18, 7, -1, -1, 13, 21, 13, 0] as &[i32]),
    );
    map.insert(
        '5',
        (
            20,
            &[
                15, 21, 5, 21, 4, 12, 5, 13, 8, 14, 11, 14, 14, 13, 16, 11, 17, 8, 17, 6, 16, 3,
                14, 1, 11, 0, 8, 0, 5, 1, 4, 2, 3, 4,
            ] as &[i32],
        ),
    );
    map.insert(
        '6',
        (
            20,
            &[
                16, 18, 15, 20, 12, 21, 10, 21, 7, 20, 5, 17, 4, 12, 4, 7, 5, 3, 7, 1, 10, 0, 11,
                0, 14, 1, 16, 3, 17, 6, 17, 7, 16, 10, 14, 12, 11, 13, 10, 13, 7, 12, 5, 10, 4, 7,
            ] as &[i32],
        ),
    );
    map.insert('7', (20, &[17, 21, 7, 0, -1, -1, 3, 21, 17, 21] as &[i32]));
    map.insert(
        '8',
        (
            20,
            &[
                8, 21, 5, 20, 4, 18, 4, 16, 5, 14, 7, 13, 11, 12, 14, 11, 16, 9, 17, 7, 17, 4, 16,
                2, 15, 1, 12, 0, 8, 0, 5, 1, 4, 2, 3, 4, 3, 7, 4, 9, 6, 11, 9, 12, 13, 13, 15, 14,
                16, 16, 16, 18, 15, 20, 12, 21, 8, 21,
            ] as &[i32],
        ),
    );
    map.insert(
        '9',
        (
            20,
            &[
                16, 14, 15, 11, 13, 9, 10, 8, 9, 8, 6, 9, 4, 11, 3, 14, 3, 15, 4, 18, 6, 20, 9, 21,
                10, 21, 13, 20, 15, 18, 16, 14, 16, 9, 15, 4, 13, 1, 10, 0, 8, 0, 5, 1, 4, 3,
            ] as &[i32],
        ),
    );
    map.insert(
        ':',
        (
            10,
            &[
                5, 14, 4, 13, 5, 12, 6, 13, 5, 14, -1, -1, 5, 2, 4, 1, 5, 0, 6, 1, 5, 2,
            ] as &[i32],
        ),
    );
    map.insert(
        ';',
        (
            10,
            &[
                5, 14, 4, 13, 5, 12, 6, 13, 5, 14, -1, -1, 6, 1, 5, 0, 4, 1, 5, 2, 6, 1, 6, -1, 5,
                -3, 4, -4,
            ] as &[i32],
        ),
    );
    map.insert('<', (24, &[20, 18, 4, 9, 20, 0] as &[i32]));
    map.insert('=', (26, &[4, 12, 22, 12, -1, -1, 4, 6, 22, 6] as &[i32]));
    map.insert('>', (24, &[4, 18, 20, 9, 4, 0] as &[i32]));
    map.insert(
        '?',
        (
            18,
            &[
                3, 16, 3, 17, 4, 19, 5, 20, 7, 21, 11, 21, 13, 20, 14, 19, 15, 17, 15, 15, 14, 13,
                13, 12, 9, 10, 9, 7, -1, -1, 9, 2, 8, 1, 9, 0, 10, 1, 9, 2,
            ] as &[i32],
        ),
    );
    map.insert(
        '@',
        (
            27,
            &[
                18, 13, 17, 15, 15, 16, 12, 16, 10, 15, 9, 14, 8, 11, 8, 8, 9, 6, 11, 5, 14, 5, 16,
                6, 17, 8, -1, -1, 12, 16, 10, 14, 9, 11, 9, 8, 10, 6, 11, 5, -1, -1, 18, 16, 17, 8,
                17, 6, 19, 5, 21, 5, 23, 7, 24, 10, 24, 12, 23, 15, 22, 17, 20, 19, 18, 20, 15, 21,
                12, 21, 9, 20, 7, 19, 5, 17, 4, 15, 3, 12, 3, 9, 4, 6, 5, 4, 7, 2, 9, 1, 12, 0, 15,
                0, 18, 1, 20, 2, 21, 3, -1, -1, 19, 16, 18, 8, 18, 6, 19, 5,
            ] as &[i32],
        ),
    );
    // Letters A-Z (uppercase)
    map.insert(
        'A',
        (
            18,
            &[9, 21, 1, 0, -1, -1, 9, 21, 17, 0, -1, -1, 4, 7, 14, 7] as &[i32],
        ),
    );
    map.insert(
        'B',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 13, 21, 16, 20, 17, 19, 18, 17, 18, 15, 17, 13, 16, 12,
                13, 11, -1, -1, 4, 11, 13, 11, 16, 10, 17, 9, 18, 7, 18, 4, 17, 2, 16, 1, 13, 0, 4,
                0,
            ] as &[i32],
        ),
    );
    map.insert(
        'C',
        (
            21,
            &[
                18, 16, 17, 18, 15, 20, 13, 21, 9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5,
                3, 7, 1, 9, 0, 13, 0, 15, 1, 17, 3, 18, 5,
            ] as &[i32],
        ),
    );
    map.insert(
        'D',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 11, 21, 14, 20, 16, 18, 17, 16, 18, 13, 18, 8, 17, 5,
                16, 3, 14, 1, 11, 0, 4, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'E',
        (
            19,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 17, 21, -1, -1, 4, 11, 12, 11, -1, -1, 4, 0, 17, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'F',
        (
            18,
            &[4, 21, 4, 0, -1, -1, 4, 21, 17, 21, -1, -1, 4, 11, 12, 11] as &[i32],
        ),
    );
    map.insert(
        'G',
        (
            21,
            &[
                18, 16, 17, 18, 15, 20, 13, 21, 9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5,
                3, 7, 1, 9, 0, 13, 0, 15, 1, 17, 3, 18, 5, 18, 8, -1, -1, 13, 8, 18, 8,
            ] as &[i32],
        ),
    );
    map.insert(
        'H',
        (
            22,
            &[4, 21, 4, 0, -1, -1, 18, 21, 18, 0, -1, -1, 4, 11, 18, 11] as &[i32],
        ),
    );
    map.insert('I', (8, &[4, 21, 4, 0] as &[i32]));
    map.insert(
        'J',
        (
            16,
            &[
                12, 21, 12, 5, 11, 2, 10, 1, 8, 0, 6, 0, 4, 1, 3, 2, 2, 5, 2, 7,
            ] as &[i32],
        ),
    );
    map.insert(
        'K',
        (
            21,
            &[4, 21, 4, 0, -1, -1, 18, 21, 4, 7, -1, -1, 9, 12, 18, 0] as &[i32],
        ),
    );
    map.insert('L', (17, &[4, 21, 4, 0, -1, -1, 4, 0, 16, 0] as &[i32]));
    map.insert(
        'M',
        (
            24,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 12, 0, -1, -1, 20, 21, 12, 0, -1, -1, 20, 21, 20, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'N',
        (
            22,
            &[4, 21, 4, 0, -1, -1, 4, 21, 18, 0, -1, -1, 18, 21, 18, 0] as &[i32],
        ),
    );
    map.insert(
        'O',
        (
            22,
            &[
                9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5, 3, 7, 1, 9, 0, 13, 0, 15, 1, 17,
                3, 18, 5, 19, 8, 19, 13, 18, 16, 17, 18, 15, 20, 13, 21, 9, 21,
            ] as &[i32],
        ),
    );
    map.insert(
        'P',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 13, 21, 16, 20, 17, 19, 18, 17, 18, 14, 17, 12, 16, 11,
                13, 10, 4, 10,
            ] as &[i32],
        ),
    );
    map.insert(
        'Q',
        (
            22,
            &[
                9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5, 3, 7, 1, 9, 0, 13, 0, 15, 1, 17,
                3, 18, 5, 19, 8, 19, 13, 18, 16, 17, 18, 15, 20, 13, 21, 9, 21, -1, -1, 12, 4, 18,
                -2,
            ] as &[i32],
        ),
    );
    map.insert(
        'R',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 13, 21, 16, 20, 17, 19, 18, 17, 18, 15, 17, 13, 16, 12,
                13, 11, 4, 11, -1, -1, 11, 11, 18, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'S',
        (
            20,
            &[
                17, 18, 15, 20, 12, 21, 8, 21, 5, 20, 3, 18, 3, 16, 4, 14, 5, 13, 7, 12, 13, 10,
                15, 9, 16, 8, 17, 6, 17, 3, 15, 1, 12, 0, 8, 0, 5, 1, 3, 3,
            ] as &[i32],
        ),
    );
    map.insert('T', (16, &[8, 21, 8, 0, -1, -1, 1, 21, 15, 21] as &[i32]));
    map.insert(
        'U',
        (
            22,
            &[
                4, 21, 4, 6, 5, 3, 7, 1, 10, 0, 12, 0, 15, 1, 17, 3, 18, 6, 18, 21,
            ] as &[i32],
        ),
    );
    map.insert('V', (18, &[1, 21, 9, 0, -1, -1, 17, 21, 9, 0] as &[i32]));
    map.insert(
        'W',
        (
            24,
            &[
                2, 21, 7, 0, -1, -1, 12, 21, 7, 0, -1, -1, 12, 21, 17, 0, -1, -1, 22, 21, 17, 0,
            ] as &[i32],
        ),
    );
    map.insert('X', (20, &[3, 21, 17, 0, -1, -1, 17, 21, 3, 0] as &[i32]));
    map.insert(
        'Y',
        (18, &[1, 21, 9, 11, 9, 0, -1, -1, 17, 21, 9, 11] as &[i32]),
    );
    map.insert(
        'Z',
        (
            20,
            &[17, 21, 3, 0, -1, -1, 3, 21, 17, 21, -1, -1, 3, 0, 17, 0] as &[i32],
        ),
    );
    // Letters a-z (lowercase - same as uppercase in Hershey Simplex)
    map.insert(
        'a',
        (
            18,
            &[9, 21, 1, 0, -1, -1, 9, 21, 17, 0, -1, -1, 4, 7, 14, 7] as &[i32],
        ),
    );
    map.insert(
        'b',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 13, 21, 16, 20, 17, 19, 18, 17, 18, 15, 17, 13, 16, 12,
                13, 11, -1, -1, 4, 11, 13, 11, 16, 10, 17, 9, 18, 7, 18, 4, 17, 2, 16, 1, 13, 0, 4,
                0,
            ] as &[i32],
        ),
    );
    map.insert(
        'c',
        (
            21,
            &[
                18, 16, 17, 18, 15, 20, 13, 21, 9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5,
                3, 7, 1, 9, 0, 13, 0, 15, 1, 17, 3, 18, 5,
            ] as &[i32],
        ),
    );
    map.insert(
        'd',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 11, 21, 14, 20, 16, 18, 17, 16, 18, 13, 18, 8, 17, 5,
                16, 3, 14, 1, 11, 0, 4, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'e',
        (
            19,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 17, 21, -1, -1, 4, 11, 12, 11, -1, -1, 4, 0, 17, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'f',
        (
            18,
            &[4, 21, 4, 0, -1, -1, 4, 21, 17, 21, -1, -1, 4, 11, 12, 11] as &[i32],
        ),
    );
    map.insert(
        'g',
        (
            21,
            &[
                18, 16, 17, 18, 15, 20, 13, 21, 9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5,
                3, 7, 1, 9, 0, 13, 0, 15, 1, 17, 3, 18, 5, 18, 8, -1, -1, 13, 8, 18, 8,
            ] as &[i32],
        ),
    );
    map.insert(
        'h',
        (
            22,
            &[4, 21, 4, 0, -1, -1, 18, 21, 18, 0, -1, -1, 4, 11, 18, 11] as &[i32],
        ),
    );
    map.insert('i', (8, &[4, 21, 4, 0] as &[i32]));
    map.insert(
        'j',
        (
            16,
            &[
                12, 21, 12, 5, 11, 2, 10, 1, 8, 0, 6, 0, 4, 1, 3, 2, 2, 5, 2, 7,
            ] as &[i32],
        ),
    );
    map.insert(
        'k',
        (
            21,
            &[4, 21, 4, 0, -1, -1, 18, 21, 4, 7, -1, -1, 9, 12, 18, 0] as &[i32],
        ),
    );
    map.insert('l', (17, &[4, 21, 4, 0, -1, -1, 4, 0, 16, 0] as &[i32]));
    map.insert(
        'm',
        (
            24,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 12, 0, -1, -1, 20, 21, 12, 0, -1, -1, 20, 21, 20, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        'n',
        (
            22,
            &[4, 21, 4, 0, -1, -1, 4, 21, 18, 0, -1, -1, 18, 21, 18, 0] as &[i32],
        ),
    );
    map.insert(
        'o',
        (
            22,
            &[
                9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5, 3, 7, 1, 9, 0, 13, 0, 15, 1, 17,
                3, 18, 5, 19, 8, 19, 13, 18, 16, 17, 18, 15, 20, 13, 21, 9, 21,
            ] as &[i32],
        ),
    );
    map.insert(
        'p',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 13, 21, 16, 20, 17, 19, 18, 17, 18, 14, 17, 12, 16, 11,
                13, 10, 4, 10,
            ] as &[i32],
        ),
    );
    map.insert(
        'q',
        (
            22,
            &[
                9, 21, 7, 20, 5, 18, 4, 16, 3, 13, 3, 8, 4, 5, 5, 3, 7, 1, 9, 0, 13, 0, 15, 1, 17,
                3, 18, 5, 19, 8, 19, 13, 18, 16, 17, 18, 15, 20, 13, 21, 9, 21, -1, -1, 12, 4, 18,
                -2,
            ] as &[i32],
        ),
    );
    map.insert(
        'r',
        (
            21,
            &[
                4, 21, 4, 0, -1, -1, 4, 21, 13, 21, 16, 20, 17, 19, 18, 17, 18, 15, 17, 13, 16, 12,
                13, 11, 4, 11, -1, -1, 11, 11, 18, 0,
            ] as &[i32],
        ),
    );
    map.insert(
        's',
        (
            20,
            &[
                17, 18, 15, 20, 12, 21, 8, 21, 5, 20, 3, 18, 3, 16, 4, 14, 5, 13, 7, 12, 13, 10,
                15, 9, 16, 8, 17, 6, 17, 3, 15, 1, 12, 0, 8, 0, 5, 1, 3, 3,
            ] as &[i32],
        ),
    );
    map.insert('t', (16, &[8, 21, 8, 0, -1, -1, 1, 21, 15, 21] as &[i32]));
    map.insert(
        'u',
        (
            22,
            &[
                4, 21, 4, 6, 5, 3, 7, 1, 10, 0, 12, 0, 15, 1, 17, 3, 18, 6, 18, 21,
            ] as &[i32],
        ),
    );
    map.insert('v', (18, &[1, 21, 9, 0, -1, -1, 17, 21, 9, 0] as &[i32]));
    map.insert(
        'w',
        (
            24,
            &[
                2, 21, 7, 0, -1, -1, 12, 21, 7, 0, -1, -1, 12, 21, 17, 0, -1, -1, 22, 21, 17, 0,
            ] as &[i32],
        ),
    );
    map.insert('x', (20, &[3, 21, 17, 0, -1, -1, 17, 21, 3, 0] as &[i32]));
    map.insert(
        'y',
        (18, &[1, 21, 9, 11, 9, 0, -1, -1, 17, 21, 9, 11] as &[i32]),
    );
    map.insert(
        'z',
        (
            20,
            &[17, 21, 3, 0, -1, -1, 3, 21, 17, 21, -1, -1, 3, 0, 17, 0] as &[i32],
        ),
    );
    // Special characters
    map.insert(
        '[',
        (
            14,
            &[
                4, 25, 4, -7, -1, -1, 5, 25, 5, -7, -1, -1, 4, 25, 11, 25, -1, -1, 4, -7, 11, -7,
            ] as &[i32],
        ),
    );
    map.insert('\\', (14, &[0, 21, 14, -3] as &[i32]));
    map.insert(
        ']',
        (
            14,
            &[
                9, 25, 9, -7, -1, -1, 10, 25, 10, -7, -1, -1, 3, 25, 10, 25, -1, -1, 3, -7, 10, -7,
            ] as &[i32],
        ),
    );
    map.insert(
        '^',
        (
            16,
            &[
                6, 15, 8, 18, 10, 15, -1, -1, 3, 12, 8, 17, 13, 12, -1, -1, 8, 17, 8, 0,
            ] as &[i32],
        ),
    );
    map.insert('_', (16, &[0, -2, 16, -2] as &[i32]));
    map.insert(
        '`',
        (
            10,
            &[6, 21, 5, 20, 4, 18, 4, 16, 5, 15, 6, 16, 5, 17] as &[i32],
        ),
    );
    map.insert(
        '{',
        (
            14,
            &[
                9, 25, 7, 24, 6, 23, 5, 21, 5, 19, 6, 17, 7, 16, 8, 14, 8, 12, 6, 10, -1, -1, 7,
                24, 6, 22, 6, 20, 7, 18, 8, 17, 9, 15, 9, 13, 8, 11, 4, 9, 8, 7, 9, 5, 9, 3, 8, 1,
                7, 0, 6, -2, 6, -4, 7, -6, -1, -1, 6, 8, 8, 6, 8, 4, 7, 2, 6, 1, 5, -1, 5, -3, 6,
                -5, 7, -6, 9, -7,
            ] as &[i32],
        ),
    );
    map.insert('|', (8, &[4, 25, 4, -7] as &[i32]));
    map.insert(
        '}',
        (
            14,
            &[
                5, 25, 7, 24, 8, 23, 9, 21, 9, 19, 8, 17, 7, 16, 6, 14, 6, 12, 8, 10, -1, -1, 7,
                24, 8, 22, 8, 20, 7, 18, 6, 17, 5, 15, 5, 13, 6, 11, 10, 9, 6, 7, 5, 5, 5, 3, 6, 1,
                7, 0, 8, -2, 8, -4, 7, -6, -1, -1, 8, 8, 6, 6, 6, 4, 7, 2, 8, 1, 9, -1, 9, -3, 8,
                -5, 7, -6, 5, -7,
            ] as &[i32],
        ),
    );
    map.insert(
        '~',
        (
            24,
            &[
                3, 6, 3, 8, 4, 11, 6, 12, 8, 12, 10, 11, 14, 8, 16, 7, 18, 7, 20, 8, 21, 10, -1,
                -1, 3, 8, 4, 10, 6, 11, 8, 11, 10, 10, 14, 7, 16, 6, 18, 6, 20, 7, 21, 10, 21, 12,
            ] as &[i32],
        ),
    );
    // Degree sign - positioned at top of numbers (y=17-21 range)
    map.insert(
        '°',
        (
            10,
            &[5, 21, 7, 20, 7, 18, 5, 17, 3, 18, 3, 20, 5, 21] as &[i32],
        ),
    );
    map
});

/// Get Hershey Simplex font character data
/// Coordinates are in a grid where y=0 is at top, y=21 is baseline
fn get_hershey_char(c: char) -> Option<HersheyChar> {
    HERSHEY_FONT.get(&c).map(|(width, data)| HersheyChar {
        width: *width,
        data,
    })
}

/// Calculate the total width of a text string in normalized units
fn calculate_text_width(text: &str) -> f32 {
    let mut total = 0.0;
    for c in text.chars() {
        if let Some(ch) = get_hershey_char(c) {
            total += ch.width as f32;
        } else {
            total += 16.0; // default space width
        }
    }
    total
}

/// Measure the rendered width of text in pixels for a given font size
pub fn measure_text_width(text: &str, font_size: f32) -> f32 {
    let hershey_height = 21.0;
    let scale = font_size / hershey_height;
    calculate_text_width(text) * scale
}

/// Render vector text using Hershey Simplex font
/// Returns a canvas element that draws the text
pub fn render_vector_text(text: &str, config: &VectorFontConfig) -> impl IntoElement {
    let text = text.to_string();
    let config = config.clone();

    // Hershey font has height ~21 units (from top at 21 to baseline at 0)
    let hershey_height = 21.0;
    let scale = config.font_size / hershey_height;

    // Calculate text bounds in pixels
    let text_width_units = calculate_text_width(&text);
    let text_width = text_width_units * scale;
    let text_height = config.font_size;

    // Account for rotation in bounds
    let (canvas_width, canvas_height) = if config.rotation.abs() > 0.1 {
        // For rotated text, swap dimensions and add padding
        let padding = config.font_size * 0.5;
        (text_height + padding, text_width + padding)
    } else {
        (text_width + config.font_size * 0.2, text_height * 1.2)
    };

    canvas(
        move |_bounds, _, _cx| {},
        move |bounds, _, window, _cx| {
            let center_x: f32 = bounds.center().x.into();
            let center_y: f32 = bounds.center().y.into();

            let cos_r = config.rotation.cos();
            let sin_r = config.rotation.sin();

            let hershey_height = 21.0;
            let scale = config.font_size / hershey_height;

            // Starting position (centered)
            let text_width_units = calculate_text_width(&text);
            let mut cursor_x = -text_width_units * scale / 2.0;

            for c in text.chars() {
                if let Some(ch) = get_hershey_char(c) {
                    let char_width = ch.width as f32 * scale;

                    // Process coordinate pairs
                    let data = ch.data;
                    let mut i = 0;
                    let mut pen_down = false;
                    let mut builder = PathBuilder::stroke(px(config.stroke_width));
                    let mut has_path = false;

                    while i + 1 < data.len() {
                        let x = data[i];
                        let y = data[i + 1];
                        i += 2;

                        if x == -1 && y == -1 {
                            // Pen up - draw current path and start new one
                            if has_path {
                                if let Ok(path) = builder.build() {
                                    window.paint_path(path, config.color);
                                }
                            }
                            builder = PathBuilder::stroke(px(config.stroke_width));
                            has_path = false;
                            pen_down = false;
                        } else {
                            // Convert Hershey coordinates to our coordinate system
                            // Hershey: y=21 at top, y=0 at baseline
                            // We want: centered vertically
                            let px_val = cursor_x + x as f32 * scale;
                            let py_val = (21.0 - y as f32 - 10.5) * scale; // Center vertically

                            // Apply rotation around center
                            let rx = px_val * cos_r - py_val * sin_r;
                            let ry = px_val * sin_r + py_val * cos_r;

                            // Translate to canvas position
                            let final_x = center_x + rx;
                            let final_y = center_y + ry;

                            if !pen_down {
                                builder.move_to(point(px(final_x), px(final_y)));
                                pen_down = true;
                                has_path = true;
                            } else {
                                builder.line_to(point(px(final_x), px(final_y)));
                            }
                        }
                    }

                    // Draw remaining path
                    if has_path {
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, config.color);
                        }
                    }

                    cursor_x += char_width;
                } else {
                    // Unknown character - skip with default width
                    cursor_x += 16.0 * scale;
                }
            }
        },
    )
    .w(px(canvas_width))
    .h(px(canvas_height))
}

/// Paint vector text directly onto a window at a given position
/// This is useful for rendering text in custom Element paint methods
pub fn paint_vector_text_at(
    window: &mut gpui::Window,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    stroke_width: f32,
    color: impl Into<gpui::Rgba>,
    rotation: f32,
) {
    let color: gpui::Rgba = color.into();
    let hershey_height = 21.0;
    let scale = font_size / hershey_height;

    let cos_r = rotation.cos();
    let sin_r = rotation.sin();

    // Starting position (centered)
    let text_width_units = calculate_text_width(text);
    let mut cursor_x = -text_width_units * scale / 2.0;

    for c in text.chars() {
        if let Some(ch) = get_hershey_char(c) {
            let char_width = ch.width as f32 * scale;

            // Process coordinate pairs
            let data = ch.data;
            let mut i = 0;
            let mut pen_down = false;
            let mut builder = PathBuilder::stroke(px(stroke_width));
            let mut has_path = false;

            while i + 1 < data.len() {
                let px_val_data = data[i];
                let py_val_data = data[i + 1];
                i += 2;

                if px_val_data == -1 && py_val_data == -1 {
                    // Pen up - draw current path and start new one
                    if has_path {
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, color);
                        }
                    }
                    builder = PathBuilder::stroke(px(stroke_width));
                    has_path = false;
                    pen_down = false;
                } else {
                    // Convert Hershey coordinates to our coordinate system
                    let px_local = cursor_x + px_val_data as f32 * scale;
                    let py_local = (21.0 - py_val_data as f32 - 10.5) * scale;

                    // Apply rotation around center
                    let rx = px_local * cos_r - py_local * sin_r;
                    let ry = px_local * sin_r + py_local * cos_r;

                    // Translate to final position
                    let final_x = x + rx;
                    let final_y = y + ry;

                    if !pen_down {
                        builder.move_to(point(px(final_x), px(final_y)));
                        pen_down = true;
                        has_path = true;
                    } else {
                        builder.line_to(point(px(final_x), px(final_y)));
                    }
                }
            }

            // Draw remaining path
            if has_path {
                if let Ok(path) = builder.build() {
                    window.paint_path(path, color);
                }
            }

            cursor_x += char_width;
        } else {
            // Unknown character - skip with default width
            cursor_x += 16.0 * scale;
        }
    }
}

// Note: Tests removed because they cause rustc to crash with SIGBUS
// when compiling in debug mode with the gpui feature enabled.
// The code is tested visually via the spinorama-demo example.
