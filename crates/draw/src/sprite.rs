use crate::{
    context::DrawContext,
    utils::{Drawable, ShaderRef, TextureRef, Vertex},
};
use smallvec::SmallVec;
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowTextureFiltering, GlowUniformValue},
};
use std::{borrow::Cow, collections::HashMap};
use vek::{Mat4, Quaternion, Rect, Rgba, Transform, Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct SpriteTexture {
    pub sampler: Cow<'static, str>,
    pub texture: TextureRef,
    pub filtering: GlowTextureFiltering,
}

impl SpriteTexture {
    pub fn new(sampler: Cow<'static, str>, texture: TextureRef) -> Self {
        Self {
            sampler,
            texture,
            filtering: GlowTextureFiltering::Linear,
        }
    }

    pub fn filtering(mut self, value: GlowTextureFiltering) -> Self {
        self.filtering = value;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub shader: Option<ShaderRef>,
    pub textures: SmallVec<[SpriteTexture; 4]>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub tint: Rgba<f32>,
    pub transform: Transform<f32, f32, f32>,
    pub size: Option<Vec2<f32>>,
    pub pivot: Vec2<f32>,
    pub blending: Option<GlowBlending>,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            shader: Default::default(),
            textures: Default::default(),
            uniforms: Default::default(),
            region: Rect::new(0.0, 0.0, 1.0, 1.0),
            page: Default::default(),
            tint: Rgba::white(),
            transform: Default::default(),
            size: Default::default(),
            pivot: Default::default(),
            blending: Default::default(),
        }
    }
}

impl Sprite {
    pub fn single(texture: SpriteTexture) -> Self {
        Self {
            textures: vec![texture].into(),
            ..Default::default()
        }
    }

    pub fn shader(mut self, value: ShaderRef) -> Self {
        self.shader = Some(value);
        self
    }

    pub fn texture(mut self, value: SpriteTexture) -> Self {
        self.textures.push(value);
        self
    }

    pub fn uniform(mut self, key: Cow<'static, str>, value: GlowUniformValue) -> Self {
        self.uniforms.insert(key, value);
        self
    }

    pub fn region_page(mut self, region: Rect<f32, f32>, page: f32) -> Self {
        self.region = region;
        self.page = page;
        self
    }

    pub fn tint(mut self, value: Rgba<f32>) -> Self {
        self.tint = value;
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

    pub fn size(mut self, value: Vec2<f32>) -> Self {
        self.size = Some(value);
        self
    }

    pub fn pivot(mut self, value: Vec2<f32>) -> Self {
        self.pivot = value;
        self
    }

    pub fn blending(mut self, value: GlowBlending) -> Self {
        self.blending = Some(value);
        self
    }
}

impl Drawable for Sprite {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        let batch = GraphicsBatch {
            shader: context.shader(self.shader.as_ref()),
            uniforms: self
                .uniforms
                .iter()
                .map(|(k, v)| (k.clone(), v.to_owned()))
                .chain(std::iter::once((
                    "u_projection_view".into(),
                    GlowUniformValue::M4(graphics.main_camera.matrix().into_col_array()),
                )))
                .chain(self.textures.iter().enumerate().map(|(index, texture)| {
                    (texture.sampler.clone(), GlowUniformValue::I1(index as _))
                }))
                .collect(),
            textures: self
                .textures
                .iter()
                .filter_map(|texture| {
                    Some((context.texture(Some(&texture.texture))?, texture.filtering))
                })
                .collect(),
            blending: self.blending.unwrap_or_else(|| context.top_blending()),
            scissor: None,
        };
        let transform = Mat4::from(context.top_transform()) * Mat4::from(self.transform);
        let size = self
            .size
            .or_else(|| {
                batch
                    .textures
                    .get(0)
                    .map(|(texture, _)| Vec2::new(texture.width() as _, texture.height() as _))
            })
            .unwrap_or_default();
        let offset = size * self.pivot;
        let color = self.tint.into_array();
        graphics.stream.batch_optimized(batch);
        graphics.stream.transformed(
            |stream| {
                stream.quad([
                    Vertex {
                        position: [0.0, 0.0],
                        uv: [self.region.x, self.region.y, self.page],
                        color,
                    },
                    Vertex {
                        position: [size.x, 0.0],
                        uv: [self.region.x + self.region.w, self.region.y, self.page],
                        color,
                    },
                    Vertex {
                        position: [size.x, size.y],
                        uv: [
                            self.region.x + self.region.w,
                            self.region.y + self.region.h,
                            self.page,
                        ],
                        color,
                    },
                    Vertex {
                        position: [0.0, size.y],
                        uv: [self.region.x, self.region.y + self.region.h, self.page],
                        color,
                    },
                ]);
            },
            |vertex| {
                let point = transform.mul_point(Vec2::from(vertex.position) - offset);
                vertex.position[0] = point.x;
                vertex.position[1] = point.y;
            },
        );
    }
}
