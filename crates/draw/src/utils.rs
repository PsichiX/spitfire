use crate::context::DrawContext;
use bytemuck::{Pod, Zeroable};
use spitfire_fontdue::TextVertex;
use spitfire_glow::{
    graphics::{Graphics, Shader, Texture},
    renderer::{GlowVertexAttrib, GlowVertexAttribs},
};
use std::borrow::Cow;
use vek::Rgba;

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 3],
    pub color: [f32; 4],
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Default::default(),
            uv: Default::default(),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl GlowVertexAttribs for Vertex {
    const ATTRIBS: &'static [(&'static str, GlowVertexAttrib)] = &[
        (
            "a_position",
            GlowVertexAttrib::Float {
                channels: 2,
                normalized: false,
            },
        ),
        (
            "a_uv",
            GlowVertexAttrib::Float {
                channels: 3,
                normalized: false,
            },
        ),
        (
            "a_color",
            GlowVertexAttrib::Float {
                channels: 4,
                normalized: false,
            },
        ),
    ];
}

impl TextVertex<Rgba<f32>> for Vertex {
    fn apply(&mut self, position: [f32; 2], tex_coord: [f32; 3], user_data: Rgba<f32>) {
        self.position = position;
        self.uv = tex_coord;
        self.color = user_data.into_array();
    }
}

pub trait Drawable {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>);
}

#[derive(Debug, Clone)]
pub enum ResourceRef<T> {
    Name(Cow<'static, str>),
    Object(T),
}

impl<T> ResourceRef<T> {
    pub fn name(value: impl Into<Cow<'static, str>>) -> Self {
        Self::Name(value.into())
    }

    pub fn object(value: T) -> Self {
        Self::Object(value)
    }
}

impl<T> From<&'static str> for ResourceRef<T> {
    fn from(value: &'static str) -> Self {
        Self::name(value)
    }
}

pub type ShaderRef = ResourceRef<Shader>;
pub type TextureRef = ResourceRef<Texture>;
