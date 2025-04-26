use crate::{
    context::DrawContext,
    utils::{Drawable, ShaderRef, Vertex, transform_to_matrix},
};
use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
};
use spitfire_fontdue::TextRenderer;
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowTextureFiltering, GlowUniformValue},
};
use std::{borrow::Cow, collections::HashMap};
use vek::{Quaternion, Rect, Rgba, Transform, Vec2, Vec3};

pub struct Text {
    pub shader: Option<ShaderRef>,
    pub font: Cow<'static, str>,
    pub size: f32,
    pub text: Cow<'static, str>,
    pub tint: Rgba<f32>,
    pub horizontal_align: HorizontalAlign,
    pub vertical_align: VerticalAlign,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub transform: Transform<f32, f32, f32>,
    pub blending: Option<GlowBlending>,
    pub screen_space: bool,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            shader: Default::default(),
            font: Default::default(),
            size: 32.0,
            text: Default::default(),
            tint: Rgba::white(),
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            width: Default::default(),
            height: Default::default(),
            uniforms: Default::default(),
            transform: Default::default(),
            blending: Default::default(),
            screen_space: Default::default(),
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

    pub fn font(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.font = value.into();
        self
    }

    pub fn size(mut self, value: f32) -> Self {
        self.size = value;
        self
    }

    pub fn text(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.text = value.into();
        self
    }

    pub fn tint(mut self, value: Rgba<f32>) -> Self {
        self.tint = value;
        self
    }

    pub fn horizontal_align(mut self, value: HorizontalAlign) -> Self {
        self.horizontal_align = value;
        self
    }

    pub fn vertical_align(mut self, value: VerticalAlign) -> Self {
        self.vertical_align = value;
        self
    }

    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value);
        self
    }

    pub fn height(mut self, value: f32) -> Self {
        self.height = Some(value);
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

    pub fn screen_space(mut self, value: bool) -> Self {
        self.screen_space = value;
        self
    }

    pub fn get_local_space_bounding_box(&self, context: &DrawContext) -> Option<Rect<f32, f32>> {
        let layout = self.make_text_layout(context)?;
        let aabb = TextRenderer::measure(context.fonts.values(), &layout);
        if aabb.iter().all(|v| v.is_finite()) {
            Some(Rect::new(
                aabb[0],
                aabb[1],
                aabb[2] - aabb[0],
                aabb[3] - aabb[1],
            ))
        } else {
            None
        }
    }

    fn make_text_layout(&self, context: &DrawContext) -> Option<Layout<Rgba<f32>>> {
        if let Some(index) = context.fonts.index_of(&self.font) {
            let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
            layout.reset(&LayoutSettings {
                x: 0.0,
                y: 0.0,
                max_width: self.width,
                max_height: self.height,
                horizontal_align: self.horizontal_align,
                vertical_align: self.vertical_align,
                ..Default::default()
            });
            layout.append(
                context.fonts.values(),
                &TextStyle {
                    text: &self.text,
                    px: self.size,
                    font_index: index,
                    user_data: self.tint,
                },
            );
            Some(layout)
        } else {
            None
        }
    }
}

impl Drawable for Text {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        if let Some(layout) = self.make_text_layout(context) {
            context
                .text_renderer
                .include(context.fonts.values(), &layout);
            graphics.stream.batch_optimized(GraphicsBatch {
                shader: context.shader(self.shader.as_ref()),
                uniforms: self
                    .uniforms
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_owned()))
                    .chain(std::iter::once((
                        "u_projection_view".into(),
                        GlowUniformValue::M4(
                            if self.screen_space {
                                graphics.main_camera.screen_matrix()
                            } else {
                                graphics.main_camera.world_matrix()
                            }
                            .into_col_array(),
                        ),
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
            let transform = context.top_transform() * transform_to_matrix(self.transform);
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
}
