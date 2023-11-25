use crate::prelude::{DrawContext, Drawable, ShaderRef, SpriteTexture, Vertex};
use smallvec::SmallVec;
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch},
    renderer::{GlowBlending, GlowUniformValue},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::{Index, IndexMut},
};
use vek::{Mat4, Quaternion, Rect, Rgba, Transform, Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct TileSetItem {
    pub region: Rect<f32, f32>,
    pub page: f32,
    pub tint: Rgba<f32>,
    pub size: Vec2<usize>,
    pub offset: Vec2<isize>,
}

impl Default for TileSetItem {
    fn default() -> Self {
        Self {
            region: Rect::new(0.0, 0.0, 1.0, 1.0),
            page: 0.0,
            tint: Rgba::white(),
            size: Vec2::new(1, 1),
            offset: Default::default(),
        }
    }
}

impl TileSetItem {
    pub fn region(mut self, value: Rect<f32, f32>) -> Self {
        self.region = value;
        self
    }

    pub fn page(mut self, value: f32) -> Self {
        self.page = value;
        self
    }

    pub fn tint(mut self, value: Rgba<f32>) -> Self {
        self.tint = value;
        self
    }

    pub fn size(mut self, value: Vec2<usize>) -> Self {
        self.size = value;
        self
    }

    pub fn offset(mut self, value: Vec2<isize>) -> Self {
        self.offset = value;
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct TileSet {
    pub shader: Option<ShaderRef>,
    pub textures: SmallVec<[SpriteTexture; 4]>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub blending: Option<GlowBlending>,
    pub mappings: HashMap<usize, TileSetItem>,
}

impl TileSet {
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

    pub fn mapping(mut self, id: usize, item: TileSetItem) -> Self {
        self.mappings.insert(id, item);
        self
    }

    pub fn mappings(mut self, iter: impl IntoIterator<Item = (usize, TileSetItem)>) -> Self {
        self.mappings.extend(iter);
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct TilesEmitter {
    pub transform: Transform<f32, f32, f32>,
    pub tile_size: Vec2<f32>,
    pub screen_space: bool,
}

impl TilesEmitter {
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

    pub fn tile_size(mut self, value: Vec2<f32>) -> Self {
        self.tile_size = value;
        self
    }

    pub fn screen_space(mut self, value: bool) -> Self {
        self.screen_space = value;
        self
    }

    pub fn emit<'a, I: IntoIterator<Item = TileInstance>>(
        &'a self,
        set: &'a TileSet,
        instances: I,
    ) -> TilesDraw<I> {
        TilesDraw {
            emitter: self,
            tileset: set,
            instances: RefCell::new(Some(instances)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileInstance {
    pub id: usize,
    pub location: Vec2<usize>,
}

impl TileInstance {
    pub fn new(id: usize, location: Vec2<usize>) -> Self {
        Self { id, location }
    }
}

pub struct TilesDraw<'a, I: IntoIterator<Item = TileInstance>> {
    emitter: &'a TilesEmitter,
    tileset: &'a TileSet,
    instances: RefCell<Option<I>>,
}

impl<'a, I: IntoIterator<Item = TileInstance>> Drawable for TilesDraw<'a, I> {
    fn draw(&self, context: &mut DrawContext, graphics: &mut Graphics<Vertex>) {
        let batch = GraphicsBatch {
            shader: context.shader(self.tileset.shader.as_ref()),
            uniforms: self
                .tileset
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
                    self.tileset
                        .textures
                        .iter()
                        .enumerate()
                        .map(|(index, texture)| {
                            (texture.sampler.clone(), GlowUniformValue::I1(index as _))
                        }),
                )
                .collect(),
            textures: self
                .tileset
                .textures
                .iter()
                .filter_map(|texture| {
                    Some((context.texture(Some(&texture.texture))?, texture.filtering))
                })
                .collect(),
            blending: self
                .tileset
                .blending
                .unwrap_or_else(|| context.top_blending()),
            scissor: None,
        };
        graphics.stream.batch_optimized(batch);
        let transform = Mat4::from(context.top_transform()) * Mat4::from(self.emitter.transform);
        graphics.stream.transformed(
            move |stream| {
                let instances = match self.instances.borrow_mut().take() {
                    Some(instances) => instances,
                    None => return,
                };
                for instance in instances {
                    if let Some(tile) = self.tileset.mappings.get(&instance.id) {
                        let offset = Vec2 {
                            x: (instance.location.x as isize + tile.offset.x) as f32,
                            y: (instance.location.y as isize + tile.offset.y) as f32,
                        } * self.emitter.tile_size;
                        let size = Vec2 {
                            x: tile.size.x as f32,
                            y: tile.size.y as f32,
                        } * self.emitter.tile_size;
                        let color = tile.tint.into_array();
                        stream.quad([
                            Vertex {
                                position: [offset.x, offset.y],
                                uv: [tile.region.x, tile.region.y, tile.page],
                                color,
                            },
                            Vertex {
                                position: [offset.x + size.x, offset.y],
                                uv: [tile.region.x + tile.region.w, tile.region.y, tile.page],
                                color,
                            },
                            Vertex {
                                position: [offset.x + size.x, offset.y + size.y],
                                uv: [
                                    tile.region.x + tile.region.w,
                                    tile.region.y + tile.region.h,
                                    tile.page,
                                ],
                                color,
                            },
                            Vertex {
                                position: [offset.x, offset.y + size.y],
                                uv: [tile.region.x, tile.region.y + tile.region.h, tile.page],
                                color,
                            },
                        ]);
                    }
                }
            },
            |vertex| {
                let point = transform.mul_point(Vec2::from(vertex.position));
                vertex.position[0] = point.x;
                vertex.position[1] = point.y;
            },
        );
    }
}

#[derive(Debug, Clone)]
pub struct TileMap {
    pub include_ids: HashSet<usize>,
    pub exclude_ids: HashSet<usize>,
    size: Vec2<usize>,
    buffer: Vec<usize>,
}

impl TileMap {
    pub fn new(size: Vec2<usize>, fill_id: usize) -> Self {
        Self {
            include_ids: Default::default(),
            exclude_ids: Default::default(),
            size,
            buffer: vec![fill_id; size.x * size.y],
        }
    }

    pub fn with_buffer(size: Vec2<usize>, buffer: Vec<usize>) -> Option<Self> {
        if buffer.len() == size.x * size.y {
            Some(Self {
                include_ids: Default::default(),
                exclude_ids: Default::default(),
                size,
                buffer,
            })
        } else {
            None
        }
    }

    pub fn size(&self) -> Vec2<usize> {
        self.size
    }

    pub fn buffer(&self) -> &[usize] {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut [usize] {
        &mut self.buffer
    }

    pub fn index(&self, location: impl Into<Vec2<usize>>) -> usize {
        let location = location.into();
        location.y * self.size.x + location.y
    }

    pub fn location(&self, index: usize) -> Vec2<usize> {
        Vec2 {
            x: index % self.size.x,
            y: index / self.size.y,
        }
    }

    pub fn get(&self, location: impl Into<Vec2<usize>>) -> Option<usize> {
        let index = self.index(location);
        self.buffer.get(index).copied()
    }

    pub fn set(&mut self, location: impl Into<Vec2<usize>>, id: usize) {
        let index = self.index(location);
        if let Some(item) = self.buffer.get_mut(index) {
            *item = id;
        }
    }

    pub fn fill(&mut self, from: impl Into<Vec2<usize>>, to: impl Into<Vec2<usize>>, id: usize) {
        let from = from.into();
        let to = to.into();
        for y in from.y..to.y {
            for x in from.x..to.x {
                self.set(Vec2::new(x, y), id);
            }
        }
    }

    pub fn emit(&self) -> impl Iterator<Item = TileInstance> + '_ {
        self.buffer.iter().enumerate().filter_map(|(index, id)| {
            if !self.include_ids.is_empty() && !self.include_ids.contains(id) {
                return None;
            }
            if !self.exclude_ids.is_empty() && self.exclude_ids.contains(id) {
                return None;
            }
            Some(TileInstance {
                id: *id,
                location: self.location(index),
            })
        })
    }
}

impl<T: Into<Vec2<usize>>> Index<T> for TileMap {
    type Output = usize;

    fn index(&self, location: T) -> &Self::Output {
        let location = location.into();
        let index = self.index(location);
        self.buffer
            .get(index)
            .unwrap_or_else(|| panic!("Invalid location: {}", location))
    }
}

impl<T: Into<Vec2<usize>>> IndexMut<T> for TileMap {
    fn index_mut(&mut self, location: T) -> &mut Self::Output {
        let location = location.into();
        let index = self.index(location);
        self.buffer
            .get_mut(index)
            .unwrap_or_else(|| panic!("Invalid location: {}", location))
    }
}
