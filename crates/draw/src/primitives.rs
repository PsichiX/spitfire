use crate::{
    context::DrawContext,
    sprite::SpriteTexture,
    utils::{Drawable, ShaderRef, Vertex},
};
use smallvec::SmallVec;
use spitfire_core::{Triangle, VertexStream};
use spitfire_glow::{
    graphics::{GraphicsBatch, GraphicsTarget},
    renderer::{GlowBlending, GlowUniformValue},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    f32::consts::{PI, TAU},
};
use vek::{Rect, Rgba, Vec2};

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

    pub fn emit_lines<I: IntoIterator<Item = Vec2<f32>>>(&self, vertices: I) -> LinesDraw<I> {
        LinesDraw {
            emitter: self,
            vertices: RefCell::new(Some(vertices)),
            region: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            page: 0.0,
            tint: Rgba::white(),
            thickness: 1.0,
            looped: false,
        }
    }

    pub fn emit_brush<I: IntoIterator<Item = (Vec2<f32>, f32, Rgba<f32>)>>(
        &self,
        vertices: I,
    ) -> BrushDraw<I> {
        BrushDraw {
            emitter: self,
            vertices: RefCell::new(Some(vertices)),
            region: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            page: 0.0,
        }
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

    pub fn emit_regular_polygon(
        &self,
        vertices: usize,
        position: Vec2<f32>,
        radius: f32,
    ) -> RegularPolygonDraw {
        RegularPolygonDraw {
            emitter: self,
            vertices,
            position,
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

    pub fn emit_circle(
        &self,
        position: Vec2<f32>,
        radius: f32,
        maximum_error: f32,
    ) -> RegularPolygonDraw {
        RegularPolygonDraw {
            emitter: self,
            vertices: (PI / (1.0 - maximum_error / radius).acos()).ceil() as _,
            position,
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
        graphics: &mut dyn GraphicsTarget<Vertex>,
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
        };
        graphics.state_mut().stream.batch_optimized(batch);
        let transform = context.top_transform();
        graphics.state_mut().stream.transformed(f, |vertex| {
            let point = transform.mul_point(Vec2::from(vertex.position));
            vertex.position[0] = point.x;
            vertex.position[1] = point.y;
        });
    }
}

pub struct LinesDraw<'a, I: IntoIterator<Item = Vec2<f32>>> {
    emitter: &'a PrimitivesEmitter,
    vertices: RefCell<Option<I>>,
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub tint: Rgba<f32>,
    pub thickness: f32,
    pub looped: bool,
}

impl<I: IntoIterator<Item = Vec2<f32>>> LinesDraw<'_, I> {
    pub fn region_page(mut self, region: Rect<f32, f32>, page: f32) -> Self {
        self.region = region;
        self.page = page;
        self
    }

    pub fn tint(mut self, value: Rgba<f32>) -> Self {
        self.tint = value;
        self
    }

    pub fn thickness(mut self, value: f32) -> Self {
        self.thickness = value;
        self
    }

    pub fn looped(mut self, value: bool) -> Self {
        self.looped = value;
        self
    }
}

impl<I: IntoIterator<Item = Vec2<f32>>> Drawable for LinesDraw<'_, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
        fn push(
            stream: &mut VertexStream<Vertex, GraphicsBatch>,
            region: Rect<f32, f32>,
            page: f32,
            color: [f32; 4],
            prev: Vec2<f32>,
            next: Vec2<f32>,
            normal: Vec2<f32>,
        ) {
            stream.extend(
                [
                    Vertex {
                        position: (prev - normal).into_array(),
                        uv: [region.x, region.y, page],
                        color,
                    },
                    Vertex {
                        position: (prev + normal).into_array(),
                        uv: [region.x + region.w, region.y, page],
                        color,
                    },
                    Vertex {
                        position: (next + normal).into_array(),
                        uv: [region.x + region.w, region.y + region.h, page],
                        color,
                    },
                    Vertex {
                        position: (next - normal).into_array(),
                        uv: [region.x, region.y + region.h, page],
                        color,
                    },
                ],
                [Triangle { a: 0, b: 1, c: 2 }, Triangle { a: 2, b: 3, c: 0 }],
            );
        }

        self.emitter
            .stream_transformed(context, graphics, |stream| {
                if let Some(vertices) = self.vertices.borrow_mut().take() {
                    let mut vertices = vertices.into_iter();
                    let Some(mut prev) = vertices.next() else {
                        return;
                    };
                    let start = prev;
                    let color = self.tint.into_array();
                    for next in vertices {
                        let tangent = next - prev;
                        let normal = Vec2 {
                            x: tangent.y,
                            y: -tangent.x,
                        }
                        .try_normalized()
                        .unwrap_or_default()
                            * self.thickness;
                        push(stream, self.region, self.page, color, prev, next, normal);
                        prev = next;
                    }
                    if self.looped {
                        let tangent = start - prev;
                        let normal = Vec2 {
                            x: tangent.y,
                            y: -tangent.x,
                        }
                        .try_normalized()
                        .unwrap_or_default()
                            * self.thickness;
                        push(stream, self.region, self.page, color, prev, start, normal);
                    }
                }
            });
    }
}

pub struct BrushDraw<'a, I: IntoIterator<Item = (Vec2<f32>, f32, Rgba<f32>)>> {
    emitter: &'a PrimitivesEmitter,
    vertices: RefCell<Option<I>>,
    pub region: Rect<f32, f32>,
    pub page: f32,
}

impl<I: IntoIterator<Item = (Vec2<f32>, f32, Rgba<f32>)>> BrushDraw<'_, I> {
    pub fn region_page(mut self, region: Rect<f32, f32>, page: f32) -> Self {
        self.region = region;
        self.page = page;
        self
    }
}

impl<I: IntoIterator<Item = (Vec2<f32>, f32, Rgba<f32>)>> Drawable for BrushDraw<'_, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
        fn push(
            stream: &mut VertexStream<Vertex, GraphicsBatch>,
            region: Rect<f32, f32>,
            page: f32,
            prev: (Vec2<f32>, f32, Rgba<f32>),
            next: (Vec2<f32>, f32, Rgba<f32>),
            normal_prev: Vec2<f32>,
            normal_next: Vec2<f32>,
        ) {
            stream.extend(
                [
                    Vertex {
                        position: ((prev.0 + next.0) * 0.5).into_array(),
                        uv: [region.x, region.y, page],
                        color: ((prev.2 + next.2) * 0.5).into_array(),
                    },
                    Vertex {
                        position: (prev.0 - normal_prev * prev.1).into_array(),
                        uv: [region.x, region.y, page],
                        color: prev.2.into_array(),
                    },
                    Vertex {
                        position: (prev.0 + normal_prev * prev.1).into_array(),
                        uv: [region.x + region.w, region.y, page],
                        color: prev.2.into_array(),
                    },
                    Vertex {
                        position: (next.0 + normal_next * next.1).into_array(),
                        uv: [region.x + region.w, region.y + region.h, page],
                        color: next.2.into_array(),
                    },
                    Vertex {
                        position: (next.0 - normal_next * next.1).into_array(),
                        uv: [region.x, region.y + region.h, page],
                        color: next.2.into_array(),
                    },
                ],
                [
                    Triangle { a: 0, b: 1, c: 2 },
                    Triangle { a: 0, b: 2, c: 3 },
                    Triangle { a: 0, b: 3, c: 4 },
                    Triangle { a: 0, b: 4, c: 1 },
                ],
            );
        }

        self.emitter
            .stream_transformed(context, graphics, |stream| {
                if let Some(vertices) = self.vertices.borrow_mut().take() {
                    let mut vertices = vertices.into_iter().peekable();
                    let Some(mut prev) = vertices.next() else {
                        return;
                    };
                    let mut prev_tangent = Option::<Vec2<f32>>::None;
                    while let Some(curr) = vertices.next() {
                        let next = vertices.peek().copied();
                        let curr_tangent = (curr.0 - prev.0).try_normalized().unwrap_or_default();
                        let tangent = prev_tangent
                            .replace(curr_tangent)
                            .and_then(|tangent| (curr_tangent + tangent).try_normalized())
                            .unwrap_or(curr_tangent);
                        let next_tangent = next
                            .and_then(|next| (next.0 - curr.0).try_normalized())
                            .and_then(|tangent| (curr_tangent + tangent).try_normalized())
                            .unwrap_or(curr_tangent);
                        let normal_prev = Vec2 {
                            x: tangent.y,
                            y: -tangent.x,
                        }
                        .try_normalized()
                        .unwrap_or_default();
                        let normal_next = Vec2 {
                            x: next_tangent.y,
                            y: -next_tangent.x,
                        }
                        .try_normalized()
                        .unwrap_or_default();
                        push(
                            stream,
                            self.region,
                            self.page,
                            prev,
                            curr,
                            normal_prev,
                            normal_next,
                        );
                        prev = curr;
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

impl<I: IntoIterator<Item = [Vertex; 3]>> Drawable for TrianglesDraw<'_, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
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

impl<I: IntoIterator<Item = Vertex>> Drawable for TriangleFanDraw<'_, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
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

impl<I: IntoIterator<Item = Vertex>> Drawable for TriangleStripDraw<'_, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
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
    position: Vec2<f32>,
    radius: f32,
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub tint: Rgba<f32>,
}

impl RegularPolygonDraw<'_> {
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

impl Drawable for RegularPolygonDraw<'_> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut dyn GraphicsTarget<Vertex>) {
        let color = self.tint.into_array();
        self.emitter
            .stream_transformed(context, graphics, |stream| {
                stream.triangle_fan((0..=self.vertices).map(|index| {
                    let angle = TAU / self.vertices as f32 * index as f32;
                    let (y, x) = angle.sin_cos();
                    let u = (x + 1.0) * 0.5;
                    let v = (y + 1.0) * 0.5;
                    Vertex {
                        position: [
                            self.position.x + x * self.radius,
                            self.position.y + y * self.radius,
                        ],
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
