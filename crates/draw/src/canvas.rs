use crate::{
    context::DrawContext,
    sprite::SpriteTexture,
    utils::{TextureRef, Vertex},
};
use spitfire_glow::{
    graphics::{Graphics, Surface},
    renderer::{GlowTextureFiltering, GlowTextureFormat},
};
use std::borrow::Cow;

pub struct Canvas {
    surface: Surface,
}

impl Canvas {
    pub fn simple(
        width: u32,
        height: u32,
        format: GlowTextureFormat,
        graphics: &Graphics<Vertex>,
    ) -> Result<Self, String> {
        Ok(Self {
            surface: graphics.surface(vec![graphics
                .texture(width, height, 1, format, None)?
                .into()])?,
        })
    }

    pub fn from_surface(surface: Surface) -> Self {
        Self { surface }
    }

    pub fn from_screen(
        texture_formats: Vec<GlowTextureFormat>,
        graphics: &Graphics<Vertex>,
    ) -> Result<Self, String> {
        let width = graphics.main_camera.screen_size.x as _;
        let height = graphics.main_camera.screen_size.y as _;
        Ok(Self {
            surface: graphics.surface(
                texture_formats
                    .into_iter()
                    .filter_map(|format| {
                        graphics
                            .texture(width, height, 1, format, None)
                            .ok()
                            .map(|texture| texture.into())
                    })
                    .collect(),
            )?,
        })
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.surface.set_color(color);
        self
    }

    pub fn match_to_screen(&mut self, graphics: &Graphics<Vertex>) -> Result<(), String> {
        let width = graphics.main_camera.screen_size.x as _;
        let height = graphics.main_camera.screen_size.y as _;
        if self.surface.width() != width || self.surface.height() != height {
            self.surface = graphics.surface(
                self.surface
                    .attachments()
                    .iter()
                    .filter_map(|attachment| {
                        graphics
                            .texture(width, height, 1, attachment.texture.format(), None)
                            .ok()
                            .map(|texture| texture.into())
                    })
                    .collect(),
            )?;
        }
        Ok(())
    }

    pub fn activate(
        &self,
        context: &mut DrawContext,
        graphics: &mut Graphics<Vertex>,
        clear: bool,
    ) {
        context.end_frame();
        let _ = graphics.draw();
        let _ = graphics.push_surface(self.surface.clone());
        let _ = graphics.prepare_frame(clear);
        context.begin_frame(graphics);
    }

    pub fn deactivate(context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        context.end_frame();
        let _ = graphics.draw();
        let _ = graphics.pop_surface();
        let _ = graphics.prepare_frame(false);
        context.begin_frame(graphics);
    }

    pub fn with<R>(
        &self,
        context: &mut DrawContext,
        graphics: &mut Graphics<Vertex>,
        clear: bool,
        mut f: impl FnMut(&mut DrawContext, &mut Graphics<Vertex>) -> R,
    ) -> R {
        self.activate(context, graphics, clear);
        let result = f(context, graphics);
        Self::deactivate(context, graphics);
        result
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn surface_mut(&mut self) -> &mut Surface {
        &mut self.surface
    }

    pub fn sprite_texture(
        &self,
        index: usize,
        sampler: Cow<'static, str>,
        filtering: GlowTextureFiltering,
    ) -> Option<SpriteTexture> {
        Some(SpriteTexture {
            sampler,
            texture: TextureRef::object(self.surface.attachments().get(index)?.texture.clone()),
            filtering,
        })
    }
}
