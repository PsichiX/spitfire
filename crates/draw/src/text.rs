use crate::{
    context::DrawContext,
    prelude::{Drawable, ShaderRef, Vertex},
};
use fontdue::layout::{CoordinateSystem, Layout, TextStyle};
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowTextureFiltering, GlowUniformValue},
};
use std::{borrow::Cow, collections::HashMap};
use vek::{Mat4, Quaternion, Rgba, Transform, Vec2, Vec3};

pub struct Text {
    pub shader: Option<ShaderRef>,
    pub text: Layout<Rgba<f32>>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub transform: Transform<f32, f32, f32>,
    pub blending: Option<GlowBlending>,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            shader: Default::default(),
            text: Layout::new(CoordinateSystem::PositiveYDown),
            uniforms: Default::default(),
            transform: Default::default(),
            blending: Default::default(),
        }
    }
}

impl Text {
    pub fn new(shader: ShaderRef) -> Self {
        Self {
            shader: Some(shader),
            ..Default::default()
        }
    }

    pub fn shader(mut self, value: ShaderRef) -> Self {
        self.shader = Some(value);
        self
    }

    pub fn text_layout(mut self, value: Layout<Rgba<f32>>) -> Self {
        self.text = value;
        self
    }

    pub fn text_style(mut self, context: &DrawContext, style: &TextStyle<Rgba<f32>>) -> Self {
        self.text.append(&context.fonts, style);
        self
    }

    pub fn uniform(mut self, key: Cow<'static, str>, value: GlowUniformValue) -> Self {
        self.uniforms.insert(key, value);
        self
    }

    pub fn transform(mut self, value: Transform<f32, f32, f32>) -> Self {
        self.transform = value;
        self
    }

    pub fn position(mut self, value: Vec2<f32>) -> Self {
        self.transform.position = value.into();
        self
    }

    pub fn orientation(mut self, value: Quaternion<f32>) -> Self {
        self.transform.orientation = value;
        self
    }

    pub fn rotation(mut self, angle_radians: f32) -> Self {
        self.transform.orientation = Quaternion::rotation_z(angle_radians);
        self
    }

    pub fn scale(mut self, value: Vec2<f32>) -> Self {
        self.transform.scale = Vec3::new(value.x, value.y, 1.0);
        self
    }

    pub fn blending(mut self, value: GlowBlending) -> Self {
        self.blending = Some(value);
        self
    }
}

impl Drawable for Text {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        context.text_renderer.include(&context.fonts, &self.text);
        graphics.stream.batch_optimized(GraphicsBatch {
            shader: context.shader(self.shader.as_ref()),
            uniforms: self
                .uniforms
                .iter()
                .map(|(k, v)| (k.clone(), v.to_owned()))
                .chain(std::iter::once((
                    "u_projection_view".into(),
                    GlowUniformValue::M4(graphics.main_camera.matrix().into_col_array()),
                )))
                .chain(std::iter::once(("u_image".into(), GlowUniformValue::I1(0))))
                .collect(),
            textures: if let Some(texture) = context.fonts_texture() {
                vec![(texture, GlowTextureFiltering::Linear)]
            } else {
                vec![]
            },
            blending: GlowBlending::Alpha,
            scissor: Default::default(),
        });
        let transform = Mat4::from(context.top_transform()) * Mat4::from(self.transform);
        graphics.stream.transformed(
            |stream| {
                context.text_renderer.render_to_stream(stream);
            },
            |vertex| {
                let point = transform.mul_point(Vec2::from(vertex.position));
                vertex.position[0] = point.x;
                vertex.position[1] = point.y;
            },
        );
    }
}
