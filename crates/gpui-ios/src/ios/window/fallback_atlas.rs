use super::types::FallbackAtlasState;
use gpui::{
    AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, Bounds, DevicePixels, PlatformAtlas,
    Size, TileId, point,
};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};

/// A minimal fallback `PlatformAtlas` used until a real Blade/Metal renderer is
/// wired up.  It records tiles in memory but does not upload texture data to the
/// GPU — just enough to satisfy GPUI's atlas queries without panicking.
pub(super) struct FallbackAtlas {
    pub(super) state: Mutex<FallbackAtlasState>,
}

impl FallbackAtlas {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(FallbackAtlasState {
                next_id: 1,
                tiles: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }
}

impl PlatformAtlas for FallbackAtlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &AtlasKey,
        build: &mut dyn FnMut() -> anyhow::Result<
            Option<(Size<DevicePixels>, std::borrow::Cow<'a, [u8]>)>,
        >,
    ) -> anyhow::Result<Option<AtlasTile>> {
        let mut state = self.state.lock();

        if let Some(tile) = state.tiles.get(key) {
            let tile = *tile;
            touch(&mut state.order, key);
            return Ok(Some(tile));
        }

        let data = build()?;
        if let Some((size, _pixels)) = data {
            let id = state.next_id;
            state.next_id += 1;

            let tile = AtlasTile {
                texture_id: AtlasTextureId {
                    index: 0,
                    kind: AtlasTextureKind::Monochrome,
                },
                tile_id: TileId(id),
                padding: 0,
                bounds: Bounds {
                    origin: point(DevicePixels(0), DevicePixels(0)),
                    size,
                },
            };

            evict_oldest(&mut state);
            state.order.push_back(key.clone());
            state.tiles.insert(key.clone(), tile);
            Ok(Some(tile))
        } else {
            Ok(None)
        }
    }

    fn remove(&self, key: &AtlasKey) {
        let mut state = self.state.lock();
        state.tiles.remove(key);
        if let Some(position) = state.order.iter().position(|queued| queued == key) {
            state.order.remove(position);
        }
    }
}

/// Move a cache hit to the back of the recency order.
fn touch(order: &mut VecDeque<AtlasKey>, key: &AtlasKey) {
    if let Some(position) = order.iter().position(|queued| queued == key) {
        order.remove(position);
    }
    order.push_back(key.clone());
}

/// Evict least-recently-used tiles while the atlas is at capacity.
fn evict_oldest(state: &mut FallbackAtlasState) {
    while state.tiles.len() >= FallbackAtlasState::MAX_TILES {
        let Some(oldest) = state.order.pop_front() else {
            break;
        };
        state.tiles.remove(&oldest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{DevicePixels, FontId, GlyphId, Point, RenderGlyphParams, Size, px};

    fn glyph_key(glyph_id: u32) -> AtlasKey {
        AtlasKey::Glyph(RenderGlyphParams {
            font_id: FontId(0),
            glyph_id: GlyphId(glyph_id),
            font_size: px(16.0),
            subpixel_variant: Point { x: 0, y: 0 },
            scale_factor: 1.0,
            is_emoji: false,
            subpixel_rendering: false,
            dilation: 0,
        })
    }

    fn insert_tile(atlas: &FallbackAtlas, glyph_id: u32) {
        let key = glyph_key(glyph_id);
        let mut build = || {
            Ok(Some((
                Size {
                    width: DevicePixels(8),
                    height: DevicePixels(8),
                },
                std::borrow::Cow::Borrowed(&[][..]),
            )))
        };
        atlas
            .get_or_insert_with(&key, &mut build)
            .expect("fallback insert succeeds");
    }

    #[test]
    fn atlas_evicts_least_recently_used_tile_at_capacity() {
        let atlas = FallbackAtlas::new();
        for glyph_id in 0..FallbackAtlasState::MAX_TILES as u32 {
            insert_tile(&atlas, glyph_id);
        }
        let state = atlas.state.lock();
        assert_eq!(state.tiles.len(), FallbackAtlasState::MAX_TILES);
        drop(state);

        // Touch tile 0 so tile 1 becomes the eviction candidate.
        insert_tile(&atlas, 0);
        insert_tile(&atlas, FallbackAtlasState::MAX_TILES as u32);

        let state = atlas.state.lock();
        assert_eq!(state.tiles.len(), FallbackAtlasState::MAX_TILES);
        assert!(state.tiles.contains_key(&glyph_key(0)));
        assert!(!state.tiles.contains_key(&glyph_key(1)));
    }

    #[test]
    fn atlas_remove_drops_tile_and_recency_entry() {
        let atlas = FallbackAtlas::new();
        insert_tile(&atlas, 7);
        atlas.remove(&glyph_key(7));
        let state = atlas.state.lock();
        assert!(state.tiles.is_empty());
        assert!(state.order.is_empty());
    }
}
