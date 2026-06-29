use super::super::events::*;
use super::types::FallbackAtlasState;
use gpui::{
    AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, Bounds, DevicePixels, PlatformAtlas,
    Size, TileId, point, size,
};
use parking_lot::Mutex;
use std::collections::HashMap;

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
            return Ok(Some(tile.clone()));
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

            state.tiles.insert(key.clone(), tile.clone());
            Ok(Some(tile))
        } else {
            Ok(None)
        }
    }

    fn remove(&self, key: &AtlasKey) {
        self.state.lock().tiles.remove(key);
    }
}
