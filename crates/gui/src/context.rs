use crate::{interactions::GuiInteractionsEngine, renderer::GuiRenderer};
#[cfg(target_arch = "wasm32")]
use instant::Instant;
use raui_core::{
    application::Application,
    layout::{CoordsMapping, CoordsMappingScaling, default_layout_engine::DefaultLayoutEngine},
    make_widget,
    widget::{
        component::containers::content_box::content_box,
        utils::{Color, Rect},
    },
};
use raui_immediate::*;
use spitfire_draw::{
    context::DrawContext,
    utils::{ShaderRef, Vertex},
};
use spitfire_fontdue::*;
use spitfire_glow::{
    graphics::{Graphics, Texture},
    renderer::{GlowTextureFiltering, GlowTextureFormat},
};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub struct GuiContext {
    pub coords_map_scaling: CoordsMappingScaling,
    pub texture_filtering: GlowTextureFiltering,
    pub interactions: GuiInteractionsEngine,
    pub application: Application,
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
            interactions: Default::default(),
            application: Default::default(),
            text_renderer: Default::default(),
            immediate: Default::default(),
            timer: Instant::now(),
            glyphs_texture: None,
        }
    }
}

impl GuiContext {
    pub fn mark_dirty(&mut self) {
        self.application.mark_dirty();
    }

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
                right: graphics.state.main_camera.screen_size.x,
                top: 0.0,
                bottom: graphics.state.main_camera.screen_size.y,
            },
            self.coords_map_scaling,
        );
        if self.application.process() {
            let mut layout_engine = DefaultLayoutEngine::<()>::default();
            let _ = self.application.layout(&coords_mapping, &mut layout_engine);
        }
        self.interactions.maintain(&coords_mapping);
        let _ = self.application.interact(&mut self.interactions);
        self.application.consume_signals();
        let mut renderer = GuiRenderer {
            texture_filtering: self.texture_filtering,
            draw,
            graphics,
            colored_shader,
            textured_shader,
            text_shader,
        };
        let _ = self.application.render(&coords_mapping, &mut renderer);
        let [w, h, d] = self.text_renderer.atlas_size();
        if let Some(texture) = self.glyphs_texture.as_mut() {
            texture.upload(
                w as _,
                h as _,
                d as _,
                GlowTextureFormat::Monochromatic,
                Some(self.text_renderer.image()),
            );
        } else {
            self.glyphs_texture = graphics
                .texture(
                    w as _,
                    h as _,
                    d as _,
                    GlowTextureFormat::Monochromatic,
                    Some(self.text_renderer.image()),
                )
                .ok();
        }
    }
}
