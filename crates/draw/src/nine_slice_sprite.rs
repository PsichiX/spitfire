use crate::{
    context::DrawContext,
    sprite::SpriteTexture,
    utils::{Drawable, ShaderRef, Vertex},
};
use smallvec::SmallVec;
use spitfire_core::Triangle;
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowUniformValue},
};
use std::{borrow::Cow, collections::HashMap};
use vek::{Mat4, Quaternion, Rect, Rgba, Transform, Vec2, Vec3};

#[derive(Debug, Default, Clone, Copy)]
pub struct NineSliceMargins {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl NineSliceMargins {
    pub fn clamp(self) -> Self {
        Self {
            left: self.left.clamp(0.0, 1.0),
            right: self.right.clamp(0.0, 1.0),
            top: self.top.clamp(0.0, 1.0),
            bottom: self.bottom.clamp(0.0, 1.0),
        }
    }

    pub fn fit_to_size(self, size: Vec2<f32>) -> Self {
        let mut result = self;
        let width = result.left + result.right;
        let height = result.top + result.bottom;
        if width > size.x {
            result.left = result.left / width * size.x;
            result.right = result.right / width * size.x;
        }
        if height > size.x {
            result.top = result.top / height * size.y;
            result.bottom = result.bottom / height * size.y;
        }
        result
    }
}

impl From<f32> for NineSliceMargins {
    fn from(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

impl From<[f32; 2]> for NineSliceMargins {
    fn from([hor, ver]: [f32; 2]) -> Self {
        Self {
            left: hor,
            right: hor,
            top: ver,
            bottom: ver,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NineSliceSprite {
    pub shader: Option<ShaderRef>,
    pub textures: SmallVec<[SpriteTexture; 4]>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub margins_source: NineSliceMargins,
    pub margins_target: NineSliceMargins,
    pub frame_only: bool,
    pub tint: Rgba<f32>,
    pub transform: Transform<f32, f32, f32>,
    pub size: Option<Vec2<f32>>,
    pub pivot: Vec2<f32>,
    pub blending: Option<GlowBlending>,
    pub screen_space: bool,
}

impl Default for NineSliceSprite {
    fn default() -> Self {
        Self {
            shader: Default::default(),
            textures: Default::default(),
            uniforms: Default::default(),
            region: Rect::new(0.0, 0.0, 1.0, 1.0),
            page: Default::default(),
            margins_source: Default::default(),
            margins_target: Default::default(),
            frame_only: false,
            tint: Rgba::white(),
            transform: Default::default(),
            size: Default::default(),
            pivot: Default::default(),
            blending: Default::default(),
            screen_space: Default::default(),
        }
    }
}

impl NineSliceSprite {
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

    pub fn margins_source(mut self, margins: NineSliceMargins) -> Self {
        self.margins_source = margins;
        self
    }

    pub fn margins_target(mut self, margins: NineSliceMargins) -> Self {
        self.margins_target = margins;
        self
    }

    pub fn frame_only(mut self, value: bool) -> Self {
        self.frame_only = value;
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

    pub fn screen_space(mut self, value: bool) -> Self {
        self.screen_space = value;
        self
    }
}

impl Drawable for NineSliceSprite {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        let batch = GraphicsBatch {
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
        let transform = context.top_transform() * Mat4::from(self.transform);
        let size = self
            .size
            .or_else(|| {
                batch
                    .textures
                    .first()
                    .map(|(texture, _)| Vec2::new(texture.width() as _, texture.height() as _))
            })
            .unwrap_or_default();
        let offset = size * self.pivot;
        let color = self.tint.into_array();
        let margins_source = self.margins_source.clamp();
        let margins_target = self.margins_target.fit_to_size(size);
        let plf = 0.0;
        let plc = margins_target.left;
        let prc = size.x - margins_target.right;
        let prf = size.x;
        let ptf = 0.0;
        let ptc = margins_target.top;
        let pbc = size.y - margins_target.bottom;
        let pbf = size.y;
        let tlf = self.region.x;
        let tlc = self.region.x + self.region.w * margins_source.left;
        let trc = self.region.x + (1.0 - margins_source.right) * self.region.w;
        let trf = self.region.x + self.region.w;
        let ttf = self.region.y;
        let ttc = self.region.y + self.region.h * margins_source.top;
        let tbc = self.region.y + (1.0 - margins_source.bottom) * self.region.h;
        let tbf = self.region.y + self.region.h;
        graphics.stream.batch_optimized(batch);
        graphics.stream.transformed(
            |stream| unsafe {
                stream.extend_triangles(
                    true,
                    [
                        Triangle { a: 0, b: 1, c: 5 },
                        Triangle { a: 5, b: 4, c: 0 },
                        Triangle { a: 1, b: 2, c: 6 },
                        Triangle { a: 6, b: 5, c: 1 },
                        Triangle { a: 2, b: 3, c: 7 },
                        Triangle { a: 7, b: 6, c: 2 },
                        Triangle { a: 4, b: 5, c: 9 },
                        Triangle { a: 9, b: 8, c: 4 },
                    ],
                );
                if !self.frame_only {
                    stream.extend_triangles(
                        true,
                        [
                            Triangle { a: 5, b: 6, c: 10 },
                            Triangle { a: 10, b: 9, c: 5 },
                        ],
                    );
                }
                stream.extend_triangles(
                    true,
                    [
                        Triangle { a: 6, b: 7, c: 11 },
                        Triangle { a: 11, b: 10, c: 6 },
                        Triangle { a: 8, b: 9, c: 13 },
                        Triangle { a: 13, b: 12, c: 8 },
                        Triangle { a: 9, b: 10, c: 14 },
                        Triangle { a: 14, b: 13, c: 9 },
                        Triangle {
                            a: 10,
                            b: 11,
                            c: 15,
                        },
                        Triangle {
                            a: 15,
                            b: 14,
                            c: 10,
                        },
                    ],
                );
                stream.extend_vertices([
                    Vertex {
                        position: [plf, ptf],
                        uv: [tlf, ttf, self.page],
                        color,
                    },
                    Vertex {
                        position: [plc, ptf],
                        uv: [tlc, ttf, self.page],
                        color,
                    },
                    Vertex {
                        position: [prc, ptf],
                        uv: [trc, ttf, self.page],
                        color,
                    },
                    Vertex {
                        position: [prf, ptf],
                        uv: [trf, ttf, self.page],
                        color,
                    },
                    Vertex {
                        position: [plf, ptc],
                        uv: [tlf, ttc, self.page],
                        color,
                    },
                    Vertex {
                        position: [plc, ptc],
                        uv: [tlc, ttc, self.page],
                        color,
                    },
                    Vertex {
                        position: [prc, ptc],
                        uv: [trc, ttc, self.page],
                        color,
                    },
                    Vertex {
                        position: [prf, ptc],
                        uv: [trf, ttc, self.page],
                        color,
                    },
                    Vertex {
                        position: [plf, pbc],
                        uv: [tlf, tbc, self.page],
                        color,
                    },
                    Vertex {
                        position: [plc, pbc],
                        uv: [tlc, tbc, self.page],
                        color,
                    },
                    Vertex {
                        position: [prc, pbc],
                        uv: [trc, tbc, self.page],
                        color,
                    },
                    Vertex {
                        position: [prf, pbc],
                        uv: [trf, tbc, self.page],
                        color,
                    },
                    Vertex {
                        position: [plf, pbf],
                        uv: [tlf, tbf, self.page],
                        color,
                    },
                    Vertex {
                        position: [plc, pbf],
                        uv: [tlc, tbf, self.page],
                        color,
                    },
                    Vertex {
                        position: [prc, pbf],
                        uv: [trc, tbf, self.page],
                        color,
                    },
                    Vertex {
                        position: [prf, pbf],
                        uv: [trf, tbf, self.page],
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
