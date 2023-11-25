use crate::{
    context::DrawContext,
    prelude::{Drawable, ShaderRef, Vertex},
    sprite::SpriteTexture,
};
use smallvec::SmallVec;
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowUniformValue},
};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, marker::PhantomData};
use vek::{Mat4, Quaternion, Rect, Rgba, Transform, Vec2, Vec3};

#[derive(Debug, Default, Clone)]
pub struct ParticleEmitter {
    pub shader: Option<ShaderRef>,
    pub textures: SmallVec<[SpriteTexture; 4]>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub blending: Option<GlowBlending>,
    pub screen_space: bool,
}

impl ParticleEmitter {
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

    pub fn emit<I: IntoIterator<Item = ParticleInstance>>(&self, instances: I) -> ParticleDraw<I> {
        ParticleDraw {
            emitter: self,
            instances: RefCell::new(Some(instances)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParticleInstance {
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub tint: Rgba<f32>,
    pub transform: Transform<f32, f32, f32>,
    pub size: Vec2<f32>,
    pub pivot: Vec2<f32>,
}

impl Default for ParticleInstance {
    fn default() -> Self {
        Self {
            region: Rect::new(0.0, 0.0, 1.0, 1.0),
            page: Default::default(),
            tint: Rgba::white(),
            transform: Default::default(),
            size: Default::default(),
            pivot: Default::default(),
        }
    }
}

impl ParticleInstance {
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
        self.size = value;
        self
    }

    pub fn pivot(mut self, value: Vec2<f32>) -> Self {
        self.pivot = value;
        self
    }
}

pub struct ParticleDraw<'a, I: IntoIterator<Item = ParticleInstance>> {
    emitter: &'a ParticleEmitter,
    instances: RefCell<Option<I>>,
}

impl<'a, I: IntoIterator<Item = ParticleInstance>> Drawable for ParticleDraw<'a, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        let instances = match self.instances.borrow_mut().take() {
            Some(instances) => instances,
            None => return,
        };
        let batch = GraphicsBatch {
            shader: context.shader(self.emitter.shader.as_ref()),
            uniforms: self
                .emitter
                .uniforms
                .iter()
                .map(|(k, v)| (k.clone(), v.to_owned()))
                .chain(std::iter::once((
                    "u_projection_view".into(),
                    GlowUniformValue::M4(
                        if self.emitter.screen_space {
                            graphics.main_camera.screen_matrix()
                        } else {
                            graphics.main_camera.world_matrix()
                        }
                        .into_col_array(),
                    ),
                )))
                .chain(
                    self.emitter
                        .textures
                        .iter()
                        .enumerate()
                        .map(|(index, texture)| {
                            (texture.sampler.clone(), GlowUniformValue::I1(index as _))
                        }),
                )
                .collect(),
            textures: self
                .emitter
                .textures
                .iter()
                .filter_map(|texture| {
                    Some((context.texture(Some(&texture.texture))?, texture.filtering))
                })
                .collect(),
            blending: self
                .emitter
                .blending
                .unwrap_or_else(|| context.top_blending()),
            scissor: None,
        };
        graphics.stream.batch_optimized(batch);
        let parent = Mat4::from(context.top_transform());
        for instance in instances {
            let transform = parent * Mat4::from(instance.transform);
            let offset = instance.size * instance.pivot;
            let color = instance.tint.into_array();
            graphics.stream.transformed(
                |stream| {
                    stream.quad([
                        Vertex {
                            position: [0.0, 0.0],
                            uv: [instance.region.x, instance.region.y, instance.page],
                            color,
                        },
                        Vertex {
                            position: [instance.size.x, 0.0],
                            uv: [
                                instance.region.x + instance.region.w,
                                instance.region.y,
                                instance.page,
                            ],
                            color,
                        },
                        Vertex {
                            position: [instance.size.x, instance.size.y],
                            uv: [
                                instance.region.x + instance.region.w,
                                instance.region.y + instance.region.h,
                                instance.page,
                            ],
                            color,
                        },
                        Vertex {
                            position: [0.0, instance.size.y],
                            uv: [
                                instance.region.x,
                                instance.region.y + instance.region.h,
                                instance.page,
                            ],
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
}

pub trait ParticleSystemProcessor<D, C> {
    fn process(config: &C, data: D) -> Option<D>;
    fn emit(config: &C, data: &D) -> Option<ParticleInstance>;
}

pub struct ParticleSystem<P: ParticleSystemProcessor<D, C>, D, C> {
    pub config: C,
    source: Vec<D>,
    target: Vec<D>,
    _phantom: PhantomData<fn() -> P>,
}

impl<P: ParticleSystemProcessor<D, C>, D, C> ParticleSystem<P, D, C> {
    pub fn new(config: C, capacity: usize) -> Self {
        Self {
            config,
            source: Vec::with_capacity(capacity),
            target: Vec::with_capacity(capacity),
            _phantom: Default::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.source.len()
    }

    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    pub fn push(&mut self, data: D) {
        if self.source.len() < self.source.capacity() {
            self.source.push(data);
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = D>) {
        self.source.extend(iter);
    }

    pub fn clear(&mut self) {
        self.source.clear();
        self.target.clear();
    }

    pub fn process(&mut self) {
        self.target.clear();
        self.target.reserve(self.source.len());
        for item in self.source.drain(..) {
            if let Some(item) = P::process(&self.config, item) {
                self.target.push(item);
            }
        }
        std::mem::swap(&mut self.source, &mut self.target);
    }

    pub fn emit(&self) -> impl Iterator<Item = ParticleInstance> + '_ {
        self.source
            .iter()
            .filter_map(|item| P::emit(&self.config, item))
    }
}

impl<P: ParticleSystemProcessor<D, C>, D: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Debug
    for ParticleSystem<P, D, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParticleSystem")
            .field("config", &self.config)
            .field("data", &self.source)
            .finish_non_exhaustive()
    }
}
