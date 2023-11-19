use crate::{interactions::GuiInteractionsEngine, prelude::GuiRenderer};
use raui_core::prelude::*;
use raui_immediate::*;
use spitfire_draw::prelude::*;
use spitfire_fontdue::*;
use spitfire_glow::prelude::*;
use std::time::Instant;
use vek::{Transform, Vec3};

pub struct GuiContext {
    pub coords_map_scaling: CoordsMappingScaling,
    pub texture_filtering: GlowTextureFiltering,
    application: Application,
    interactions: GuiInteractionsEngine,
    text_renderer: TextRenderer<Color>,
    immediate: ImmediateContext,
    timer: Instant,
    glyphs_texture: Option<Texture>,
}

impl Default for GuiContext {
    fn default() -> Self {
        Self {
            coords_map_scaling: Default::default(),
            texture_filtering: Default::default(),
            application: Default::default(),
            interactions: Default::default(),
            text_renderer: Default::default(),
            immediate: Default::default(),
            timer: Instant::now(),
            glyphs_texture: None,
        }
    }
}

impl GuiContext {
    pub fn begin_frame(&self) {
        ImmediateContext::activate(&self.immediate);
        begin();
    }

    pub fn end_frame(
        &mut self,
        draw: &mut DrawContext,
        graphics: &mut Graphics<Vertex>,
        colored_shader: &ShaderRef,
        textured_shader: &ShaderRef,
        text_shader: &ShaderRef,
    ) {
        let widgets = end();
        ImmediateContext::deactivate();
        self.application
            .apply(make_widget!(content_box).key("root").listed_slots(widgets));
        let elapsed = std::mem::replace(&mut self.timer, Instant::now())
            .elapsed()
            .as_secs_f32();
        self.timer = Instant::now();
        self.application.animations_delta_time = elapsed;
        let coords_mapping = CoordsMapping::new_scaling(
            Rect {
                left: 0.0,
                right: graphics.main_camera.viewport_size.x,
                top: 0.0,
                bottom: graphics.main_camera.viewport_size.y,
            },
            self.coords_map_scaling,
        );
        if self.application.process() {
            let _ = self
                .application
                .layout(&coords_mapping, &mut DefaultLayoutEngine);
        }
        self.interactions.maintain(&coords_mapping);
        let _ = self.application.interact(&mut self.interactions);
        self.application.consume_signals();
        let offset = graphics.main_camera.viewport_offset();
        draw.push_transform(Transform {
            position: Vec3 {
                x: offset.x,
                y: offset.y,
                z: 0.0,
            },
            ..Default::default()
        });
        let mut renderer = GuiRenderer {
            texture_filtering: self.texture_filtering,
            draw,
            graphics,
            colored_shader,
            textured_shader,
            text_shader,
        };
        let _ = self.application.render(&coords_mapping, &mut renderer);
        draw.pop_transform();
        let [w, h, d] = self.text_renderer.atlas_size();
        if let Some(texture) = self.glyphs_texture.as_mut() {
            texture.upload(
                w as _,
                h as _,
                d as _,
                GlowTextureFormat::Luminance,
                self.text_renderer.image(),
            );
        } else {
            self.glyphs_texture = graphics
                .texture(
                    w as _,
                    h as _,
                    d as _,
                    GlowTextureFormat::Luminance,
                    self.text_renderer.image(),
                )
                .ok();
        }
    }
}
