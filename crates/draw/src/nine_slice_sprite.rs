use crate::{
    context::DrawContext,
    sprite::SpriteTexture,
    utils::{Drawable, ShaderRef, Vertex, transform_to_matrix},
};
use smallvec::SmallVec;
use spitfire_core::Triangle;
use spitfire_glow::{
    graphics::{GraphicsBatch, GraphicsTarget},
    renderer::{GlowBlending, GlowUniformValue},
};
use std::{borrow::Cow, collections::HashMap};
use vek::{Quaternion, Rect, Rgba, Transform, Vec2, Vec3};

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

#[derive(Debug, Clone, Copy)]
struct NineSliceSegment {
    from: f32,
    to: f32,
    uv_from: f32,
    uv_to: f32,
}

impl NineSliceSegment {
    fn new(from: f32, to: f32, uv_from: f32, uv_to: f32) -> Self {
        Self {
            from,
            to,
            uv_from,
            uv_to,
        }
    }

    fn repeated(self, repeat: Option<f32>) -> Vec<Self> {
        let length = self.to - self.from;
        let count = match repeat {
            Some(repeat) if repeat > 0.0 => (length / repeat).round().max(1.0) as usize,
            _ => 1,
        };
        let step = length / count as f32;
        (0..count)
            .map(|index| Self {
                from: self.from + step * index as f32,
                to: self.from + step * (index + 1) as f32,
                ..self
            })
            .collect()
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
    pub repeat_horizontal: Option<f32>,
    pub repeat_vertical: Option<f32>,
    pub center_repeat: bool,
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
            repeat_horizontal: None,
            repeat_vertical: None,
            center_repeat: false,
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

    pub fn repeat_horizontal(mut self, value: Option<f32>) -> Self {
        self.repeat_horizontal = value;
        self
    }

    pub fn repeat_vertical(mut self, value: Option<f32>) -> Self {
        self.repeat_vertical = value;
        self
    }

    pub fn center_repeat(mut self, value: bool) -> Self {
        self.center_repeat = value;
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
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
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
                            graphics.state().main_camera.screen_matrix()
                        } else {
                            graphics.state().main_camera.world_matrix()
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
            wireframe: context.wireframe,
            mesh: None,
        };
        let transform = context.top_transform() * transform_to_matrix(self.transform);
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
        let columns = [
            vec![NineSliceSegment::new(plf, plc, tlf, tlc)],
            NineSliceSegment::new(plc, prc, tlc, trc).repeated(self.repeat_horizontal),
            vec![NineSliceSegment::new(prc, prf, trc, trf)],
        ];
        let rows = [
            vec![NineSliceSegment::new(ptf, ptc, ttf, ttc)],
            NineSliceSegment::new(ptc, pbc, ttc, tbc).repeated(self.repeat_vertical),
            vec![NineSliceSegment::new(pbc, pbf, tbc, tbf)],
        ];
        let stretched_columns = NineSliceSegment::new(plc, prc, tlc, trc).repeated(None);
        let stretched_rows = NineSliceSegment::new(ptc, pbc, ttc, tbc).repeated(None);
        let mut triangles = Vec::new();
        let mut vertices = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, column) in columns.iter().enumerate() {
                let center = row_index == 1 && column_index == 1;
                if center && self.frame_only {
                    continue;
                }
                let (row, column) = if center && !self.center_repeat {
                    (&stretched_rows, &stretched_columns)
                } else {
                    (row, column)
                };
                for y in row {
                    for x in column {
                        let base = vertices.len() as u32;
                        triangles.push(Triangle {
                            a: base,
                            b: base + 1,
                            c: base + 2,
                        });
                        triangles.push(Triangle {
                            a: base + 2,
                            b: base + 3,
                            c: base,
                        });
                        vertices.extend([
                            Vertex {
                                position: [x.from, y.from],
                                uv: [x.uv_from, y.uv_from, self.page],
                                color,
                            },
                            Vertex {
                                position: [x.to, y.from],
                                uv: [x.uv_to, y.uv_from, self.page],
                                color,
                            },
                            Vertex {
                                position: [x.to, y.to],
                                uv: [x.uv_to, y.uv_to, self.page],
                                color,
                            },
                            Vertex {
                                position: [x.from, y.to],
                                uv: [x.uv_from, y.uv_to, self.page],
                                color,
                            },
                        ]);
                    }
                }
            }
        }
        graphics.state_mut().stream.batch_optimized(batch);
        graphics.state_mut().stream.transformed(
            |stream| unsafe {
                stream.extend_triangles(true, triangles.iter().copied());
                stream.extend_vertices(vertices.iter().copied());
            },
            |vertex| {
                let point = transform.mul_point(Vec2::from(vertex.position) - offset);
                vertex.position[0] = point.x;
                vertex.position[1] = point.y;
            },
        );
    }
}
