use crate::{
    context::DrawContext,
    sprite::SpriteTexture,
    utils::{Drawable, ShaderRef, Vertex},
};
use smallvec::SmallVec;
use spitfire_core::{Triangle, VertexStream};
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowUniformValue},
};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, f32::consts::TAU};
use vek::{Mat4, Rect, Rgba, Vec2};

#[derive(Debug, Default, Clone)]
pub struct PrimitivesEmitter {
    pub shader: Option<ShaderRef>,
    pub textures: SmallVec<[SpriteTexture; 4]>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub blending: Option<GlowBlending>,
    pub screen_space: bool,
}

impl PrimitivesEmitter {
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

    pub fn blending(mut self, value: GlowBlending) -> Self {
        self.blending = Some(value);
        self
    }

    pub fn screen_space(mut self, value: bool) -> Self {
        self.screen_space = value;
        self
    }

    pub fn emit_triangles<I: IntoIterator<Item = [Vertex; 3]>>(
        &self,
        vertices: I,
    ) -> TrianglesDraw<I> {
        TrianglesDraw {
            emitter: self,
            vertices: RefCell::new(Some(vertices)),
            tint: Rgba::white(),
        }
    }

    pub fn emit_triangle_fan<I: IntoIterator<Item = Vertex>>(
        &self,
        vertices: I,
    ) -> TriangleFanDraw<I> {
        TriangleFanDraw {
            emitter: self,
            vertices: RefCell::new(Some(vertices)),
        }
    }

    pub fn emit_triangle_strip<I: IntoIterator<Item = Vertex>>(
        &self,
        vertices: I,
    ) -> TriangleStripDraw<I> {
        TriangleStripDraw {
            emitter: self,
            vertices: RefCell::new(Some(vertices)),
        }
    }

    pub fn emit_regular_polygon(&self, vertices: usize, radius: f32) -> RegularPolygonDraw {
        RegularPolygonDraw {
            emitter: self,
            vertices,
            radius,
            region: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            page: 0.0,
            tint: Rgba::white(),
        }
    }

    fn stream_transformed(
        &self,
        context: &mut DrawContext,
        graphics: &mut Graphics<Vertex>,
        f: impl FnMut(&mut VertexStream<Vertex, GraphicsBatch>),
    ) {
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
        graphics.stream.batch_optimized(batch);
        let transform = Mat4::from(context.top_transform());
        graphics.stream.transformed(f, |vertex| {
            let point = transform.mul_point(Vec2::from(vertex.position));
            vertex.position[0] = point.x;
            vertex.position[1] = point.y;
        });
    }
}

pub struct LinesDraw<'a, I: IntoIterator<Item = Vec2<f32>>> {
    emitter: &'a PrimitivesEmitter,
    vertices: RefCell<Option<I>>,
    pub tint: Rgba<f32>,
    pub thickness: f32,
}

impl<'a, I: IntoIterator<Item = Vec2<f32>>> LinesDraw<'a, I> {
    pub fn tint(mut self, value: Rgba<f32>) -> Self {
        self.tint = value;
        self
    }

    pub fn thickness(mut self, value: f32) -> Self {
        self.thickness = value;
        self
    }
}

impl<'a, I: IntoIterator<Item = Vec2<f32>>> Drawable for LinesDraw<'a, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        self.emitter
            .stream_transformed(context, graphics, |stream| {
                if let Some(vertices) = self.vertices.borrow_mut().take() {
                    let mut vertices = vertices.into_iter();
                    let Some(mut prev) = vertices.next() else {
                        return;
                    };
                    let color = self.tint.into_array();
                    for next in vertices {
                        let tangent = next - prev;
                        let normal = Vec2 {
                            x: tangent.y,
                            y: -tangent.x,
                        };
                        stream.triangle_strip([
                            Vertex {
                                position: (prev - normal).into_array(),
                                uv: [0.0, 0.0, 0.0],
                                color,
                            },
                            Vertex {
                                position: (prev + normal).into_array(),
                                uv: [0.0, 0.0, 0.0],
                                color,
                            },
                            Vertex {
                                position: (next + normal).into_array(),
                                uv: [0.0, 0.0, 0.0],
                                color,
                            },
                            Vertex {
                                position: (next - normal).into_array(),
                                uv: [0.0, 0.0, 0.0],
                                color,
                            },
                        ]);
                        prev = next;
                    }
                }
            });
    }
}

pub struct TrianglesDraw<'a, I: IntoIterator<Item = [Vertex; 3]>> {
    emitter: &'a PrimitivesEmitter,
    vertices: RefCell<Option<I>>,
    pub tint: Rgba<f32>,
}

impl<'a, I: IntoIterator<Item = [Vertex; 3]>> Drawable for TrianglesDraw<'a, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        self.emitter
            .stream_transformed(context, graphics, |stream| {
                if let Some(vertices) = self.vertices.borrow_mut().take() {
                    unsafe {
                        let start = stream.vertices().len();
                        stream.extend_vertices(vertices.into_iter().flatten());
                        let end = stream.vertices().len();
                        stream.extend_triangles(
                            false,
                            (start..end).step_by(3).map(|index| Triangle {
                                a: index as u32,
                                b: index as u32 + 1,
                                c: index as u32 + 2,
                            }),
                        );
                    }
                }
            });
    }
}

pub struct TriangleFanDraw<'a, I: IntoIterator<Item = Vertex>> {
    emitter: &'a PrimitivesEmitter,
    vertices: RefCell<Option<I>>,
}

impl<'a, I: IntoIterator<Item = Vertex>> Drawable for TriangleFanDraw<'a, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        self.emitter
            .stream_transformed(context, graphics, |stream| {
                if let Some(vertices) = self.vertices.borrow_mut().take() {
                    stream.triangle_fan(vertices);
                }
            });
    }
}

pub struct TriangleStripDraw<'a, I: IntoIterator<Item = Vertex>> {
    emitter: &'a PrimitivesEmitter,
    vertices: RefCell<Option<I>>,
}

impl<'a, I: IntoIterator<Item = Vertex>> Drawable for TriangleStripDraw<'a, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        self.emitter
            .stream_transformed(context, graphics, |stream| {
                if let Some(vertices) = self.vertices.borrow_mut().take() {
                    stream.triangle_strip(vertices);
                }
            });
    }
}

pub struct RegularPolygonDraw<'a> {
    emitter: &'a PrimitivesEmitter,
    vertices: usize,
    radius: f32,
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub tint: Rgba<f32>,
}

impl<'a> RegularPolygonDraw<'a> {
    pub fn region_page(mut self, region: Rect<f32, f32>, page: f32) -> Self {
        self.region = region;
        self.page = page;
        self
    }

    pub fn tint(mut self, value: Rgba<f32>) -> Self {
        self.tint = value;
        self
    }
}

impl<'a> Drawable for RegularPolygonDraw<'a> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        let color = self.tint.into_array();
        self.emitter
            .stream_transformed(context, graphics, |stream| {
                stream.triangle_fan((0..=self.vertices).map(|index| {
                    let angle = TAU / self.vertices as f32 * index as f32;
                    let (y, x) = angle.sin_cos();
                    let u = (x + 1.0) * 0.5;
                    let v = (y + 1.0) * 0.5;
                    Vertex {
                        position: [x * self.radius, y * self.radius],
                        uv: [
                            self.region.x + self.region.w * u,
                            self.region.y + self.region.h * v,
                            self.page,
                        ],
                        color,
                    }
                }));
            });
    }
}
