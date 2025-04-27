use crate::utils::{FontMap, ResourceRef, ShaderRef, TextureRef, Vertex};
use spitfire_fontdue::TextRenderer;
use spitfire_glow::{
    graphics::{Graphics, Shader, Texture},
    renderer::{GlowBlending, GlowTextureFormat},
};
use std::{borrow::Cow, collections::HashMap};
use vek::{Mat4, Rgba};

#[derive(Default, Clone)]
pub struct DrawContext {
    pub shaders: HashMap<Cow<'static, str>, Shader>,
    pub textures: HashMap<Cow<'static, str>, Texture>,
    pub fonts: FontMap,
    pub text_renderer: TextRenderer<Rgba<f32>>,
    pub wireframe: bool,
    pass_shader: Option<Shader>,
    empty_texture: Option<Texture>,
    fonts_texture: Option<Texture>,
    shaders_stack: Vec<Shader>,
    transform_stack: Vec<Mat4<f32>>,
    blending_stack: Vec<GlowBlending>,
}

impl DrawContext {
    pub fn begin_frame(&mut self, graphics: &mut Graphics<Vertex>) {
        if self.pass_shader.is_none() {
            self.pass_shader = graphics
                .shader(Shader::PASS_VERTEX_2D, Shader::PASS_FRAGMENT)
                .ok();
        }
        if self.empty_texture.is_none() {
            self.empty_texture = graphics.pixel_texture([255, 255, 255]).ok();
        }
        if self.fonts_texture.is_none() {
            self.fonts_texture = graphics.pixel_texture([255, 255, 255]).ok();
        }
        self.text_renderer.clear();
        self.shaders_stack.clear();
        self.transform_stack.clear();
        self.blending_stack.clear();
    }

    pub fn end_frame(&mut self) {
        let [width, height, depth] = self.text_renderer.atlas_size();
        if let Some(fonts_texture) = self.fonts_texture.as_mut() {
            fonts_texture.upload(
                width as _,
                height as _,
                depth as _,
                GlowTextureFormat::Monochromatic,
                Some(self.text_renderer.image()),
            );
        }
    }

    pub fn shader(&self, reference: Option<&ShaderRef>) -> Option<Shader> {
        reference
            .and_then(|reference| match reference {
                ResourceRef::Name(name) => self.shaders.get(name).cloned(),
                ResourceRef::Object(object) => Some(object.to_owned()),
            })
            .or_else(|| self.shaders_stack.last().cloned())
    }

    pub fn shader_or_pass(&self, reference: Option<&ShaderRef>) -> Option<Shader> {
        self.shader(reference).or_else(|| self.pass_shader.clone())
    }

    pub fn texture(&self, reference: Option<&TextureRef>) -> Option<Texture> {
        reference.and_then(|reference| match reference {
            ResourceRef::Name(name) => self.textures.get(name).cloned(),
            ResourceRef::Object(object) => Some(object.to_owned()),
        })
    }

    pub fn texture_or_empty(&self, reference: Option<&TextureRef>) -> Option<Texture> {
        self.texture(reference)
            .or_else(|| self.empty_texture.clone())
    }

    pub fn pass_shader(&self) -> Option<Shader> {
        self.pass_shader.clone()
    }

    pub fn empty_texture(&self) -> Option<Texture> {
        self.empty_texture.clone()
    }

    pub fn fonts_texture(&self) -> Option<Texture> {
        self.fonts_texture.clone()
    }

    pub fn push_shader(&mut self, shader: &ShaderRef) {
        match shader {
            ResourceRef::Name(name) => {
                if let Some(shader) = self.shaders.get(name) {
                    self.shaders_stack.push(shader.clone());
                }
            }
            ResourceRef::Object(object) => {
                self.shaders_stack.push(object.clone());
            }
        }
    }

    pub fn pop_shader(&mut self) -> Option<Shader> {
        self.shaders_stack.pop()
    }

    pub fn top_shader(&self) -> Option<Shader> {
        self.shaders_stack.last().cloned()
    }

    pub fn with_shader<R>(&mut self, shader: &ShaderRef, mut f: impl FnMut() -> R) -> R {
        self.push_shader(shader);
        let result = f();
        self.pop_shader();
        result
    }

    pub fn push_transform(&mut self, transform: Mat4<f32>) {
        self.transform_stack.push(transform);
    }

    pub fn push_transform_relative(&mut self, transform: Mat4<f32>) {
        self.push_transform(self.top_transform() * transform);
    }

    pub fn pop_transform(&mut self) -> Option<Mat4<f32>> {
        self.transform_stack.pop()
    }

    pub fn top_transform(&self) -> Mat4<f32> {
        self.transform_stack.last().copied().unwrap_or_default()
    }

    pub fn with_transform<R>(&mut self, transform: Mat4<f32>, mut f: impl FnMut() -> R) -> R {
        self.push_transform(transform);
        let result = f();
        self.pop_transform();
        result
    }

    pub fn push_blending(&mut self, blending: GlowBlending) {
        self.blending_stack.push(blending);
    }

    pub fn pop_blending(&mut self) -> Option<GlowBlending> {
        self.blending_stack.pop()
    }

    pub fn top_blending(&self) -> GlowBlending {
        self.blending_stack.last().copied().unwrap_or_default()
    }

    pub fn with_blending<R>(&mut self, blending: GlowBlending, mut f: impl FnMut() -> R) -> R {
        self.push_blending(blending);
        let result = f();
        self.pop_blending();
        result
    }
}
