//! D3-tile-inspired slippy-map tile coverage helpers.
//!
//! The layout is renderer-independent: given a continuous map scale,
//! translation, and viewport extent, it returns the integer tile coordinates
//! that intersect the viewport plus the screen-space origin/scale needed to
//! place each tile.

/// Maximum supported integer zoom level.
pub const MAX_TILE_ZOOM: u32 = 30;
/// Maximum number of visible tiles a checked layout will allocate.
pub const MAX_VISIBLE_TILES: usize = 1_000_000;

/// One integer tile coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tile {
    pub x: i64,
    pub y: i64,
    pub z: u32,
}

/// Checked tile layout error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileError {
    NonFiniteScale,
    NonPositiveScale,
    NonFiniteTileSize,
    NonPositiveTileSize,
    NonFiniteTranslate,
    NonFiniteExtent,
    InvalidExtent,
    ZoomOutOfRange,
    TooManyTiles,
}

impl std::fmt::Display for TileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteScale => write!(f, "tile scale must be finite"),
            Self::NonPositiveScale => write!(f, "tile scale must be positive"),
            Self::NonFiniteTileSize => write!(f, "tile size must be finite"),
            Self::NonPositiveTileSize => write!(f, "tile size must be positive"),
            Self::NonFiniteTranslate => write!(f, "tile translate must be finite"),
            Self::NonFiniteExtent => write!(f, "tile extent must be finite"),
            Self::InvalidExtent => write!(f, "tile extent must be ordered"),
            Self::ZoomOutOfRange => write!(f, "tile zoom is outside the supported range"),
            Self::TooManyTiles => write!(f, "tile layout would allocate too many tiles"),
        }
    }
}

impl std::error::Error for TileError {}

/// Screen-space tile coverage result.
#[derive(Debug, Clone, PartialEq)]
pub struct TileSet {
    pub tiles: Vec<Tile>,
    pub zoom: u32,
    pub tile_screen_size: f64,
    pub origin: [f64; 2],
}

impl TileSet {
    /// Return the screen-space bounds for a tile in this layout.
    pub fn tile_bounds(&self, tile: Tile) -> [[f64; 2]; 2] {
        let x0 = self.origin[0] + tile.x as f64 * self.tile_screen_size;
        let y0 = self.origin[1] + tile.y as f64 * self.tile_screen_size;
        [
            [x0, y0],
            [x0 + self.tile_screen_size, y0 + self.tile_screen_size],
        ]
    }

    /// Return the number of tiles in the set.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Return true when the layout found no visible tiles.
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

/// D3-tile-inspired tile layout configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileLayout {
    tile_size: f64,
    extent: [[f64; 2]; 2],
    scale: f64,
    translate: [f64; 2],
    zoom_delta: i32,
    clamp_x: bool,
    clamp_y: bool,
}

impl Default for TileLayout {
    fn default() -> Self {
        Self {
            tile_size: 256.0,
            extent: [[0.0, 0.0], [960.0, 500.0]],
            scale: 256.0,
            translate: [480.0, 250.0],
            zoom_delta: 0,
            clamp_x: true,
            clamp_y: true,
        }
    }
}

impl TileLayout {
    /// Create a default tile layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the viewport size and keep translation centered in that viewport.
    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.extent = [[0.0, 0.0], [width, height]];
        self.translate = [width / 2.0, height / 2.0];
        self
    }

    /// Set the viewport extent in screen coordinates.
    pub fn extent(mut self, extent: [[f64; 2]; 2]) -> Self {
        self.extent = extent;
        self
    }

    /// Set the continuous map scale.
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// Set the screen-space translation of the projected world center.
    pub fn translate(mut self, translate: [f64; 2]) -> Self {
        self.translate = translate;
        self
    }

    /// Set the source tile size in pixels.
    pub fn tile_size(mut self, tile_size: f64) -> Self {
        self.tile_size = tile_size;
        self
    }

    /// Add an integer zoom offset before rounding the continuous zoom.
    pub fn zoom_delta(mut self, zoom_delta: i32) -> Self {
        self.zoom_delta = zoom_delta;
        self
    }

    /// Configure whether x/y tile coordinates are clamped to the world range.
    pub fn clamp(mut self, clamp_x: bool, clamp_y: bool) -> Self {
        self.clamp_x = clamp_x;
        self.clamp_y = clamp_y;
        self
    }

    /// Return the checked tile coverage for the configured viewport.
    pub fn try_tiles(self) -> Result<TileSet, TileError> {
        self.validate()?;

        let continuous_zoom = (self.scale / self.tile_size).log2();
        let rounded_zoom = (continuous_zoom + self.zoom_delta as f64).round().max(0.0);
        if !rounded_zoom.is_finite() || rounded_zoom > MAX_TILE_ZOOM as f64 {
            return Err(TileError::ZoomOutOfRange);
        }

        let zoom = rounded_zoom as u32;
        let world_tile_count = 1_i64.checked_shl(zoom).ok_or(TileError::ZoomOutOfRange)?;
        let tile_screen_size = self.scale / world_tile_count as f64;
        let origin = [
            self.translate[0] - tile_screen_size / 2.0,
            self.translate[1] - tile_screen_size / 2.0,
        ];

        let mut x0 = ((self.extent[0][0] - origin[0]) / tile_screen_size).floor() as i64;
        let mut x1 = ((self.extent[1][0] - origin[0]) / tile_screen_size).ceil() as i64;
        let mut y0 = ((self.extent[0][1] - origin[1]) / tile_screen_size).floor() as i64;
        let mut y1 = ((self.extent[1][1] - origin[1]) / tile_screen_size).ceil() as i64;

        if self.clamp_x {
            x0 = x0.clamp(0, world_tile_count);
            x1 = x1.clamp(0, world_tile_count);
        }
        if self.clamp_y {
            y0 = y0.clamp(0, world_tile_count);
            y1 = y1.clamp(0, world_tile_count);
        }

        let tile_count = x1
            .saturating_sub(x0)
            .try_into()
            .ok()
            .and_then(|width: usize| {
                y1.saturating_sub(y0)
                    .try_into()
                    .ok()
                    .and_then(|height: usize| width.checked_mul(height))
            })
            .ok_or(TileError::TooManyTiles)?;
        if tile_count > MAX_VISIBLE_TILES {
            return Err(TileError::TooManyTiles);
        }

        let mut tiles = Vec::with_capacity(tile_count);
        for y in y0..y1 {
            for x in x0..x1 {
                tiles.push(Tile { x, y, z: zoom });
            }
        }

        Ok(TileSet {
            tiles,
            zoom,
            tile_screen_size,
            origin,
        })
    }

    /// Return tile coverage, panicking on invalid configuration.
    pub fn tiles(self) -> TileSet {
        self.try_tiles()
            .expect("invalid tile layout; use try_tiles for recoverable errors")
    }

    fn validate(self) -> Result<(), TileError> {
        if !self.scale.is_finite() {
            return Err(TileError::NonFiniteScale);
        }
        if self.scale <= 0.0 {
            return Err(TileError::NonPositiveScale);
        }
        if !self.tile_size.is_finite() {
            return Err(TileError::NonFiniteTileSize);
        }
        if self.tile_size <= 0.0 {
            return Err(TileError::NonPositiveTileSize);
        }
        if !self.translate.iter().all(|value| value.is_finite()) {
            return Err(TileError::NonFiniteTranslate);
        }
        if !self.extent.iter().flatten().all(|value| value.is_finite()) {
            return Err(TileError::NonFiniteExtent);
        }
        if self.extent[0][0] > self.extent[1][0] || self.extent[0][1] > self.extent[1][1] {
            return Err(TileError::InvalidExtent);
        }
        Ok(())
    }
}

/// Convenience helper matching the common d3-tile pattern.
pub fn tiles_for_viewport(
    width: f64,
    height: f64,
    scale: f64,
    translate: [f64; 2],
) -> Result<TileSet, TileError> {
    TileLayout::new()
        .size(width, height)
        .scale(scale)
        .translate(translate)
        .try_tiles()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_layout_returns_visible_world_tiles() {
        let set = TileLayout::new()
            .size(512.0, 512.0)
            .scale(512.0)
            .translate([256.0, 256.0])
            .try_tiles()
            .unwrap();

        assert_eq!(set.zoom, 1);
        assert_eq!(set.tile_screen_size, 256.0);
        assert_eq!(set.origin, [128.0, 128.0]);
        assert_eq!(
            set.tiles,
            vec![
                Tile { x: 0, y: 0, z: 1 },
                Tile { x: 1, y: 0, z: 1 },
                Tile { x: 0, y: 1, z: 1 },
                Tile { x: 1, y: 1, z: 1 },
            ]
        );
        assert_eq!(
            set.tile_bounds(set.tiles[0]),
            [[128.0, 128.0], [384.0, 384.0]]
        );
    }

    #[test]
    fn tile_layout_supports_zoom_delta_and_unclamped_tiles() {
        let set = TileLayout::new()
            .size(256.0, 256.0)
            .scale(256.0)
            .translate([0.0, 0.0])
            .zoom_delta(1)
            .clamp(false, false)
            .try_tiles()
            .unwrap();

        assert_eq!(set.zoom, 1);
        assert!(set.tiles.contains(&Tile { x: 0, y: 0, z: 1 }));
        assert!(set.tiles.contains(&Tile { x: 2, y: 2, z: 1 }));
    }

    #[test]
    fn tile_layout_reports_invalid_configuration() {
        assert_eq!(
            TileLayout::new().scale(f64::NAN).try_tiles().unwrap_err(),
            TileError::NonFiniteScale
        );
        assert_eq!(
            TileLayout::new().scale(0.0).try_tiles().unwrap_err(),
            TileError::NonPositiveScale
        );
        assert_eq!(
            TileLayout::new()
                .extent([[10.0, 0.0], [0.0, 10.0]])
                .try_tiles()
                .unwrap_err(),
            TileError::InvalidExtent
        );
        assert_eq!(
            TileLayout::new().zoom_delta(64).try_tiles().unwrap_err(),
            TileError::ZoomOutOfRange
        );
        assert_eq!(
            TileLayout::new()
                .extent([[0.0, 0.0], [1024.0 * 1024.0, 1024.0 * 1024.0]])
                .clamp(false, false)
                .try_tiles()
                .unwrap_err(),
            TileError::TooManyTiles
        );
    }

    #[test]
    fn tiles_for_viewport_uses_centered_defaults() {
        let set = tiles_for_viewport(512.0, 512.0, 512.0, [256.0, 256.0]).unwrap();

        assert_eq!(set.len(), 4);
        assert!(!set.is_empty());
    }
}
