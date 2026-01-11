//! Font glyph atlas management

use std::collections::HashMap;
use std::sync::Arc;

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

    /// Get or rasterize a glyph
    pub fn get_glyph(&mut self, c: char, size: f32) -> Option<GlyphInfo> {
        let key = GlyphKey::new(c, size);

        if let Some(info) = self.glyph_cache.get(&key) {
            return Some(*info);
        }

        // Rasterize the glyph
        let (metrics, bitmap) = self.font.rasterize(c, size);

        if metrics.width == 0 || metrics.height == 0 {
            // Whitespace or empty glyph
            let info = GlyphInfo {
                uv: [0.0, 0.0, 0.0, 0.0],
                bearing: [metrics.xmin as f32, metrics.ymin as f32],
                advance: metrics.advance_width,
                width: 0,
                height: 0,
            };
            self.glyph_cache.insert(key, info);
            return Some(info);
        }

        // Pack into atlas using shelf algorithm
        let glyph_width = metrics.width as u32;
        let glyph_height = metrics.height as u32;

        // Check if we need to move to next row
        if self.current_x + glyph_width > self.size {
            self.current_x = 0;
            self.current_y += self.row_height;
            self.row_height = 0;
        }

        // Check if atlas is full
        if self.current_y + glyph_height > self.size {
            // Atlas is full - in production, we'd resize or use multiple atlases
            return None;
        }

        // Upload glyph to texture
        if let Some(texture) = &self.texture {
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

        // Calculate UV coordinates
        let size_f = self.size as f32;
        let info = GlyphInfo {
            uv: [
                self.current_x as f32 / size_f,
                self.current_y as f32 / size_f,
                (self.current_x + glyph_width) as f32 / size_f,
                (self.current_y + glyph_height) as f32 / size_f,
            ],
            bearing: [metrics.xmin as f32, metrics.ymin as f32],
            advance: metrics.advance_width,
            width: glyph_width,
            height: glyph_height,
        };

        // Update packing state
        self.current_x += glyph_width + 1; // 1px padding
        self.row_height = self.row_height.max(glyph_height + 1);

        self.glyph_cache.insert(key, info);
        Some(info)
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
