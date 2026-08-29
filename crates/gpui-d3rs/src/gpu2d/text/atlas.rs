//! Font glyph atlas management

use std::collections::HashMap;
use std::sync::Arc;

const GLYPH_PADDING: u32 = 1;

/// Cache key for glyph lookup
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct GlyphKey {
    pub codepoint: char,
    /// Font size in pixels (quantized to reduce cache size)
    pub size_px: u16,
}

impl GlyphKey {
    pub fn new(c: char, size: f32) -> Self {
        Self {
            codepoint: c,
            size_px: size.round() as u16,
        }
    }
}

/// Information about a cached glyph in the atlas
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    /// UV coordinates in atlas (min_u, min_v, max_u, max_v)
    pub uv: [f32; 4],
    /// Offset from baseline (x, y)
    pub bearing: [f32; 2],
    /// Horizontal advance width
    pub advance: f32,
    /// Glyph dimensions in pixels
    pub width: u32,
    pub height: u32,
}

/// Font glyph atlas using shelf-packing algorithm
pub struct TextAtlas {
    /// GPU texture for the atlas
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    /// The font used for rasterization
    font: fontdue::Font,
    /// Cached glyph info
    glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    /// Insertion order retained so a grown atlas can be repacked identically.
    glyph_order: Vec<GlyphKey>,
    /// Current packing state
    current_x: u32,
    current_y: u32,
    row_height: u32,
    /// Atlas texture size
    size: u32,
    /// Device reference for texture updates
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Bind group for sampling
    bind_group: Option<wgpu::BindGroup>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl TextAtlas {
    /// Create a new text atlas with the given font data
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        font_data: &[u8],
        atlas_size: u32,
    ) -> Self {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
            .expect("Failed to parse font");

        let mut atlas = Self {
            texture: None,
            texture_view: None,
            font,
            glyph_cache: HashMap::new(),
            glyph_order: Vec::new(),
            current_x: 0,
            current_y: 0,
            row_height: 0,
            size: atlas_size,
            device,
            queue,
            bind_group: None,
            bind_group_layout: None,
        };

        atlas.create_texture();
        atlas
    }

    fn create_texture(&mut self) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas"),
            size: wgpu::Extent3d {
                width: self.size,
                height: self.size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.texture_view = Some(texture.create_view(&Default::default()));
        self.texture = Some(texture);

        // Create bind group layout and bind group
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Text Atlas Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Atlas Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Atlas Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        self.texture_view.as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.bind_group_layout = Some(bind_group_layout);
        self.bind_group = Some(bind_group);
    }

    /// Get or rasterize a glyph.
    ///
    /// The atlas grows and repacks its retained glyph order when the current
    /// texture fills. A glyph is only absent when it cannot fit within the
    /// device's maximum 2D texture dimension.
    pub fn get_glyph(&mut self, c: char, size: f32) -> Option<GlyphInfo> {
        let key = GlyphKey::new(c, size);
        if let Some(info) = self.glyph_cache.get(&key) {
            return Some(*info);
        }

        if let Some(info) = self.insert_glyph(key) {
            return Some(info);
        }

        self.grow_and_repack(key)
            .then(|| self.glyph_cache.get(&key).copied())
            .flatten()
    }

    fn insert_glyph(&mut self, key: GlyphKey) -> Option<GlyphInfo> {
        let info = self.rasterize_and_pack(key)?;
        self.glyph_cache.insert(key, info);
        self.glyph_order.push(key);
        Some(info)
    }

    fn rasterize_and_pack(&mut self, key: GlyphKey) -> Option<GlyphInfo> {
        let (metrics, bitmap) = self.font.rasterize(key.codepoint, key.size_px as f32);
        if metrics.width == 0 || metrics.height == 0 {
            return Some(GlyphInfo {
                uv: [0.0, 0.0, 0.0, 0.0],
                bearing: [metrics.xmin as f32, metrics.ymin as f32],
                advance: metrics.advance_width,
                width: 0,
                height: 0,
            });
        }

        let glyph_width = metrics.width as u32;
        let glyph_height = metrics.height as u32;
        let padded_width = glyph_width.checked_add(GLYPH_PADDING)?;
        let padded_height = glyph_height.checked_add(GLYPH_PADDING)?;

        if self
            .current_x
            .checked_add(padded_width)
            .is_none_or(|x| x > self.size)
        {
            self.current_x = 0;
            self.current_y = self.current_y.checked_add(self.row_height)?;
            self.row_height = 0;
        }
        if self
            .current_y
            .checked_add(padded_height)
            .is_none_or(|y| y > self.size)
        {
            return None;
        }

        {
            let texture = self.texture.as_ref()?;
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: self.current_x,
                        y: self.current_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &bitmap,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(glyph_width),
                    rows_per_image: Some(glyph_height),
                },
                wgpu::Extent3d {
                    width: glyph_width,
                    height: glyph_height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let size = self.size as f32;
        let info = GlyphInfo {
            uv: [
                self.current_x as f32 / size,
                self.current_y as f32 / size,
                (self.current_x + glyph_width) as f32 / size,
                (self.current_y + glyph_height) as f32 / size,
            ],
            bearing: [metrics.xmin as f32, metrics.ymin as f32],
            advance: metrics.advance_width,
            width: glyph_width,
            height: glyph_height,
        };
        self.current_x += padded_width;
        self.row_height = self.row_height.max(padded_height);
        Some(info)
    }

    fn grow_and_repack(&mut self, new_key: GlyphKey) -> bool {
        let maximum_size = self.device.limits().max_texture_dimension_2d;
        if self.size >= maximum_size {
            return false;
        }

        let mut keys = self.glyph_order.clone();
        keys.push(new_key);
        let old_size = self.size;
        let old_current_x = self.current_x;
        let old_current_y = self.current_y;
        let old_row_height = self.row_height;
        let old_texture = self.texture.take();
        let old_texture_view = self.texture_view.take();
        let old_bind_group = self.bind_group.take();
        let old_bind_group_layout = self.bind_group_layout.take();
        let old_glyph_cache = std::mem::take(&mut self.glyph_cache);
        let old_glyph_order = std::mem::take(&mut self.glyph_order);

        let mut candidate_size = self.size;
        while candidate_size < maximum_size {
            candidate_size = candidate_size.saturating_mul(2).min(maximum_size);
            self.size = candidate_size;
            self.current_x = 0;
            self.current_y = 0;
            self.row_height = 0;
            self.glyph_cache.clear();
            self.glyph_order.clear();
            self.create_texture();

            let mut repacked = true;
            for key in &keys {
                if self.insert_glyph(*key).is_none() {
                    repacked = false;
                    break;
                }
            }
            if repacked {
                return true;
            }
        }

        self.size = old_size;
        self.current_x = old_current_x;
        self.current_y = old_current_y;
        self.row_height = old_row_height;
        self.texture = old_texture;
        self.texture_view = old_texture_view;
        self.bind_group = old_bind_group;
        self.bind_group_layout = old_bind_group_layout;
        self.glyph_cache = old_glyph_cache;
        self.glyph_order = old_glyph_order;
        false
    }

    /// Get the bind group for rendering
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    /// Get the bind group layout
    pub fn bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.bind_group_layout.as_ref()
    }
}
