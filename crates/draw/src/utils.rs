use crate::context::DrawContext;
use bytemuck::{Pod, Zeroable};
use fontdue::Font;
use spitfire_fontdue::TextVertex;
use spitfire_glow::{
    graphics::{Graphics, Shader, Texture},
    renderer::{GlowVertexAttrib, GlowVertexAttribs},
};
use std::borrow::Cow;
use vek::{Mat4, Rgba, Transform};

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

#[derive(Debug, Default, Clone)]
pub struct FontMap {
    keys: Vec<Cow<'static, str>>,
    values: Vec<Font>,
}

impl FontMap {
    pub fn insert(&mut self, name: impl Into<Cow<'static, str>>, font: Font) {
        let name = name.into();
        if let Some(index) = self.index_of(&name) {
            self.values[index] = font;
        } else {
            self.keys.push(name);
            self.values.push(font);
        }
    }

    pub fn remove(&mut self, name: &str) -> Option<Font> {
        if let Some(index) = self.index_of(name) {
            self.keys.remove(index);
            Some(self.values.remove(index))
        } else {
            None
        }
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.keys.iter().position(|key| key == name)
    }

    pub fn get(&self, name: &str) -> Option<&Font> {
        if let Some(index) = self.index_of(name) {
            self.values.get(index)
        } else {
            None
        }
    }

    pub fn keys(&self) -> &[Cow<'static, str>] {
        &self.keys
    }

    pub fn values(&self) -> &[Font] {
        &self.values
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Cow<'static, str>, &Font)> {
        self.keys.iter().zip(self.values.iter())
    }
}

pub fn transform_to_matrix(transform: Transform<f32, f32, f32>) -> Mat4<f32> {
    Mat4::<f32>::scaling_3d(transform.scale)
        * Mat4::<f32>::from(transform.orientation)
        * Mat4::<f32>::translation_3d(transform.position)
}
