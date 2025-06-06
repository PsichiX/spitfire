use bytemuck::Pod;
use etagere::{AtlasAllocator, euclid::default::Rect, size2};
use fontdue::{
    Font,
    layout::{GlyphPosition, GlyphRasterConfig, Layout},
};
use spitfire_core::VertexStream;
use std::{
    collections::{HashMap, hash_map::Entry},
    marker::PhantomData,
};

pub trait TextVertex<UD: Copy> {
    fn apply(&mut self, position: [f32; 2], tex_coord: [f32; 3], user_data: UD);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TextRendererGlyph {
    pub page: usize,
    pub rectangle: Rect<u32>,
}

pub struct TextRendererUnpacked<UD: Copy> {
    pub glyphs: HashMap<GlyphRasterConfig, TextRendererGlyph>,
    pub atlas_size: [usize; 3],
    pub image: Vec<u8>,
    pub renderables: Vec<GlyphPosition<UD>>,
}

#[derive(Clone)]
pub struct TextRenderer<UD: Copy = ()> {
    pub renderables_resize: usize,
    used_glyphs: HashMap<GlyphRasterConfig, TextRendererGlyph>,
    atlas_size: [usize; 3],
    image: Vec<u8>,
    atlases: Vec<AtlasAllocator>,
    ready_to_render: Vec<GlyphPosition<UD>>,
    _phantom: PhantomData<fn() -> UD>,
}

impl<UD: Copy> Default for TextRenderer<UD> {
    fn default() -> Self {
        Self::new(1024, 1024)
    }
}

impl<UD: Copy> TextRenderer<UD> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            renderables_resize: 1024,
            used_glyphs: Default::default(),
            atlas_size: [width, height, 0],
            image: Default::default(),
            atlases: Default::default(),
            ready_to_render: Default::default(),
            _phantom: Default::default(),
        }
    }

    pub fn clear(&mut self) {
        self.used_glyphs.clear();
        self.atlas_size[2] = 0;
        self.image.clear();
        self.atlases.clear();
        self.ready_to_render.clear();
    }

    pub fn measure(layout: &Layout<UD>, fonts: &[Font], compact: bool) -> [f32; 4] {
        let mut xmin = f32::INFINITY;
        let mut ymin = f32::INFINITY;
        let mut xmax = f32::NEG_INFINITY;
        let mut ymax = f32::NEG_INFINITY;
        if compact {
            for glyph in layout.glyphs() {
                if glyph.char_data.rasterize() {
                    xmin = xmin.min(glyph.x);
                    ymin = ymin.min(glyph.y);
                    xmax = xmax.max(glyph.x + glyph.width as f32);
                    ymax = ymax.max(glyph.y + glyph.height as f32);
                }
            }
        } else if let Some(lines) = layout.lines() {
            for line in lines {
                ymin = ymin.min(line.baseline_y - line.max_ascent);
                ymax = ymax.max(line.baseline_y - line.max_ascent + line.max_new_line_size + 1.0);
            }
            for glyph in layout.glyphs() {
                if glyph.char_data.rasterize() {
                    let font = &fonts[glyph.font_index];
                    let metrics = font.metrics_indexed(glyph.key.glyph_index, glyph.key.px);
                    xmin = xmin.min(glyph.x);
                    xmax = xmax.max(glyph.x + metrics.advance_width.ceil() + 1.0);
                }
            }
            xmin = layout.settings().x.min(xmin);
            ymin = layout.settings().y.min(ymin);
        }
        [xmin, ymin, xmax, ymax]
    }

    pub fn include(&mut self, fonts: &[Font], layout: &Layout<UD>) {
        for glyph in layout.glyphs() {
            if glyph.char_data.rasterize() {
                if self.ready_to_render.len() == self.ready_to_render.capacity() {
                    self.ready_to_render.reserve(self.renderables_resize);
                }
                self.ready_to_render.push(*glyph);
            }
            if let Entry::Vacant(entry) = self.used_glyphs.entry(glyph.key) {
                let font = &fonts[glyph.font_index];
                let (metrics, coverage) = font.rasterize_config(glyph.key);
                if glyph.char_data.rasterize() {
                    let allocation = self
                        .atlases
                        .iter_mut()
                        .enumerate()
                        .find_map(|(page, atlas)| {
                            Some((
                                page,
                                atlas
                                    .allocate(size2(
                                        metrics.width as i32 + 1,
                                        metrics.height as i32 + 1,
                                    ))?
                                    .rectangle
                                    .to_rect()
                                    .origin
                                    .to_u32(),
                            ))
                        })
                        .or_else(|| {
                            let w = self.atlas_size[0];
                            let h = self.atlas_size[1];
                            let mut atlas = AtlasAllocator::new(size2(w as _, h as _));
                            let page = self.atlases.len();
                            let origin = atlas
                                .allocate(size2(
                                    metrics.width as i32 + 1,
                                    metrics.height as i32 + 1,
                                ))?
                                .rectangle
                                .to_rect()
                                .origin
                                .to_u32();
                            self.atlases.push(atlas);
                            self.atlas_size[2] += 1;
                            let [w, h, d] = self.atlas_size;
                            self.image.resize(w * h * d, 0);
                            Some((page, origin))
                        });
                    if let Some((page, origin)) = allocation {
                        let [w, h, _] = self.atlas_size;
                        for (index, value) in coverage.iter().enumerate() {
                            let x = origin.x as usize + index % metrics.width;
                            let y = origin.y as usize + index / metrics.width;
                            let index = page * w * h + y * w + x;
                            self.image[index] = *value;
                        }
                        entry.insert(TextRendererGlyph {
                            page,
                            rectangle: Rect::new(
                                origin,
                                [metrics.width as _, metrics.height as _].into(),
                            ),
                        });
                    }
                }
            }
        }
    }

    pub fn include_consumed(
        &mut self,
        fonts: &[Font],
        layout: &Layout<UD>,
    ) -> impl Iterator<Item = (GlyphPosition<UD>, TextRendererGlyph)> + '_ {
        self.include(fonts, layout);
        self.consume_renderables()
    }

    pub fn glyph(&self, key: &GlyphRasterConfig) -> Option<TextRendererGlyph> {
        self.used_glyphs.get(key).copied()
    }

    pub fn consume_renderables(
        &mut self,
    ) -> impl Iterator<Item = (GlyphPosition<UD>, TextRendererGlyph)> + '_ {
        self.ready_to_render
            .drain(..)
            .filter_map(|glyph| Some((glyph, *self.used_glyphs.get(&glyph.key)?)))
    }

    pub fn image(&self) -> &[u8] {
        &self.image
    }

    pub fn atlas_size(&self) -> [usize; 3] {
        self.atlas_size
    }

    pub fn into_image(self) -> (Vec<u8>, [usize; 3]) {
        (self.image, self.atlas_size)
    }

    pub fn into_inner(self) -> TextRendererUnpacked<UD> {
        TextRendererUnpacked {
            glyphs: self.used_glyphs,
            atlas_size: self.atlas_size,
            image: self.image,
            renderables: self.ready_to_render,
        }
    }

    pub fn render_to_stream<V, B>(&mut self, stream: &mut VertexStream<V, B>)
    where
        V: TextVertex<UD> + Pod + Default,
    {
        let [w, h, _] = self.atlas_size;
        let w = w as f32;
        let h = h as f32;
        for glyph in self.ready_to_render.drain(..) {
            if let Some(data) = self.used_glyphs.get(&glyph.key) {
                let mut a = V::default();
                let mut b = V::default();
                let mut c = V::default();
                let mut d = V::default();
                a.apply(
                    [glyph.x, glyph.y],
                    [
                        data.rectangle.min_x() as f32 / w,
                        data.rectangle.min_y() as f32 / h,
                        data.page as f32,
                    ],
                    glyph.user_data,
                );
                b.apply(
                    [glyph.x + glyph.width as f32, glyph.y],
                    [
                        data.rectangle.max_x() as f32 / w,
                        data.rectangle.min_y() as f32 / h,
                        data.page as f32,
                    ],
                    glyph.user_data,
                );
                c.apply(
                    [glyph.x + glyph.width as f32, glyph.y + glyph.height as f32],
                    [
                        data.rectangle.max_x() as f32 / w,
                        data.rectangle.max_y() as f32 / h,
                        data.page as f32,
                    ],
                    glyph.user_data,
                );
                d.apply(
                    [glyph.x, glyph.y + glyph.height as f32],
                    [
                        data.rectangle.min_x() as f32 / w,
                        data.rectangle.max_y() as f32 / h,
                        data.page as f32,
                    ],
                    glyph.user_data,
                );
                stream.quad([a, b, c, d]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TextRenderer;
    use fontdue::{
        Font,
        layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    };
    use image::RgbImage;

    #[test]
    fn test_text_renderer() {
        let text = include_str!("../../../resources/text.txt");
        let font = include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_];
        let font = Font::from_bytes(font, Default::default()).unwrap();
        let fonts = [font];
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        let mut renderer = TextRenderer::new(256, 256);

        for line in text.lines() {
            layout.append(&fonts, &TextStyle::new(line, 32.0, 0));
        }

        renderer.include(&fonts, &layout);
        let (image, [width, height, _]) = renderer.into_image();
        let image = RgbImage::from_vec(
            width as _,
            height as _,
            image
                .into_iter()
                .flat_map(|value| [value, value, value])
                .collect(),
        )
        .unwrap();
        image.save("../../resources/test.png").unwrap();
    }

    #[test]
    fn test_text_measurements() {
        let font = include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_];
        let font = Font::from_bytes(font, Default::default()).unwrap();
        let fonts = [font];
        let style = TextStyle::new(include_str!("../../../resources/long_text.txt"), 32.0, 0);
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.append(&fonts, &style);

        let aabb_non_compact = TextRenderer::measure(&layout, &fonts, false);
        assert_eq!(aabb_non_compact, [0.0, 0.0, 450.0, 115.0]);
        let aabb_compact = TextRenderer::measure(&layout, &fonts, true);
        assert_eq!(aabb_compact, [0.0, 6.0, 448.0, 113.0]);

        layout.reset(&LayoutSettings {
            max_width: Some(aabb_non_compact[2] - aabb_non_compact[0]),
            max_height: Some(aabb_non_compact[3] - aabb_non_compact[1]),
            ..Default::default()
        });
        layout.append(&fonts, &style);
        let aabb = TextRenderer::measure(&layout, &fonts, false);
        assert_eq!(aabb, aabb_non_compact);
        let aabb = TextRenderer::measure(&layout, &fonts, true);
        assert_eq!(aabb, aabb_compact);

        layout.reset(&LayoutSettings {
            max_width: Some(aabb_compact[2] - aabb_compact[0]),
            max_height: Some(aabb_compact[3] - aabb_compact[1]),
            ..Default::default()
        });
        layout.append(&fonts, &style);
        let aabb = TextRenderer::measure(&layout, &fonts, false);
        assert_ne!(aabb, aabb_non_compact);
        let aabb = TextRenderer::measure(&layout, &fonts, true);
        assert_ne!(aabb, aabb_compact);
    }
}
