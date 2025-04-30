use fontdue::Font;
use rand::Rng;
use spitfire_draw::prelude::*;
use spitfire_glow::prelude::*;
use std::{fs::File, ops::Range, path::Path, time::Instant};
use vek::{Rgba, Transform, Vec2};

const DELTA_TIME: f32 = 1.0 / 60.0;

fn main() {
    App::<Vertex>::default().run(State::default());
}

struct State {
    // We store drawing context for later use in app state.
    // Drawing context holds resources and stack-based states.
    context: DrawContext,
    // Tile maps store IDs 2D array of single tiles layer.
    tilemap: TileMap,
    // Tile sets store mappings from ID to texture region.
    tileset: TileSet,
    // We also store particle system instance that stores particle
    // data used for emitting particles to render.
    particles: ParticleSystem<ParticlesProcessor, ParticleData, ()>,
    // This tells the angular location of particles spawner.
    particles_phase: f32,
    // Timer used for fixed step frame particle system simulation.
    timer: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            context: Default::default(),
            tilemap: TileMap::with_buffer(
                [6, 6].into(),
                (0..36).map(|_| rand::random::<usize>() % 11).collect(),
            )
            .unwrap(),
            tileset: TileSet::single(SpriteTexture {
                sampler: "u_image".into(),
                texture: TextureRef::name("tileset"),
                filtering: GlowTextureFiltering::Nearest,
            })
            .shader(ShaderRef::name("image"))
            .mappings((0..11).map(|index| (index, TileSetItem::default().page(index as f32)))),
            particles: ParticleSystem::new((), 100),
            particles_phase: 0.0,
            timer: Instant::now(),
        }
    }
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>) {
        graphics.color = [0.25, 0.25, 0.25, 1.0];
        graphics.main_camera.screen_alignment = 0.5.into();
        graphics.main_camera.scaling = CameraScaling::FitToView {
            size: 1000.0.into(),
            inside: false,
        };

        self.context.shaders.insert(
            "color".into(),
            graphics
                .shader(Shader::COLORED_VERTEX_2D, Shader::PASS_FRAGMENT)
                .unwrap(),
        );

        self.context.shaders.insert(
            "image".into(),
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );

        self.context.shaders.insert(
            "text".into(),
            graphics
                .shader(Shader::TEXT_VERTEX, Shader::TEXT_FRAGMENT)
                .unwrap(),
        );

        self.context.textures.insert(
            "ferris".into(),
            load_texture(graphics, "resources/ferris.png", 1),
        );

        self.context.textures.insert(
            "tileset".into(),
            load_texture(graphics, "resources/tileset.png", 11),
        );

        self.context
            .textures
            .insert("checkerboard".into(), checkerboard_texture(graphics));

        self.context.fonts.insert(
            "roboto",
            Font::from_bytes(
                include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_],
                Default::default(),
            )
            .unwrap(),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>) {
        // Here we emulate fixed step simulation.
        if self.timer.elapsed().as_secs_f32() > DELTA_TIME {
            self.timer = Instant::now();

            // Update particles in particle system and spawn new ones.
            self.particles.process();
            self.particles_phase += DELTA_TIME * 2.0;
            self.particles.push(ParticleData::new(
                1.0..2.0,
                50.0..100.0,
                20.0..30.0,
                Vec2::new(self.particles_phase.sin(), self.particles_phase.cos()) * 200.0,
                Rgba::new_opaque(1.0, 1.0, 0.0),
            ));
        }

        // Each scene draw phase should start with `DrawContext::begin_frame`
        // and should end with `DrawContext::end_frame`.
        self.context.begin_frame(graphics);

        // When new frame starts, there is no default shader and
        // no default blending, so we should push those.
        self.context.push_shader(&ShaderRef::name("image"));
        self.context.push_blending(GlowBlending::Alpha);

        // Drawing lines is done with primitives emitter. You can emit different
        // primitives, both textured and colored, all in series or one-by-one.
        PrimitivesEmitter::default()
            .shader(ShaderRef::name("color"))
            .emit_lines([
                Vec2::new(-490.0, -490.0),
                Vec2::new(490.0, -490.0),
                Vec2::new(-490.0, 490.0),
                Vec2::new(490.0, 490.0),
            ])
            .thickness(5.0)
            .draw(&mut self.context, graphics);

        // Brush is a special kind of lines drawer, where segments vertices are connected.
        PrimitivesEmitter::default()
            .shader(ShaderRef::name("color"))
            .emit_brush([
                (Vec2::new(0.0, -450.0), 0.0, Rgba::red()),
                (Vec2::new(450.0, 0.0), 10.0, Rgba::blue()),
                (Vec2::new(0.0, 450.0), 20.0, Rgba::green()),
                (Vec2::new(-450.0, 0.0), 10.0, Rgba::yellow()),
                (Vec2::new(0.0, -450.0), 0.0, Rgba::red()),
            ])
            .draw(&mut self.context, graphics);

        // Tile maps are rendered by emitting tiles with iterators.
        // Here TileMap container is providing tiles it stores.
        TilesEmitter::default()
            .tile_size(64.0.into())
            .position([-192.0, -192.0].into())
            .emit(&self.tileset, self.tilemap.emit())
            .draw(&mut self.context, graphics);

        // Nine slices are renderables that split sprite into its
        // frame and content. Useful for stretching GUI panels.
        NineSliceSprite::single(SpriteTexture {
            sampler: "u_image".into(),
            texture: TextureRef::name("checkerboard"),
            filtering: GlowTextureFiltering::Nearest,
        })
        .size(512.0.into())
        .pivot(0.5.into())
        .margins_source(0.25.into())
        .margins_target(64.0.into())
        .frame_only(true)
        .draw(&mut self.context, graphics);

        // To draw a sprite, we construct one using builder pattern.
        // You can also store that sprite somewhere and just draw
        // it's instance.
        Sprite::single(SpriteTexture::new(
            "u_image".into(),
            TextureRef::name("ferris"),
        ))
        .pivot(0.5.into())
        .draw(&mut self.context, graphics);

        // Drawing texts is done in similar way to drawing sprites.
        // In matter of fact, you can create custom renderables
        // by implementing `Drawable` trait on a type!
        let text = Text::new(ShaderRef::name("text"))
            .font("roboto")
            .size(64.0)
            .text(include_str!("../../../resources/long_text.txt"))
            .tint([0.0, 0.8, 1.0, 1.0].into())
            .position([-450.0, 170.0].into());
        text.draw(&mut self.context, graphics);
        // We can use text bounding box to draw a rectangle around it.
        if let Some(aabb) = text.get_local_space_bounding_box(&self.context, false) {
            self.context.push_transform_relative(text.transform.into());
            PrimitivesEmitter::default()
                .shader(ShaderRef::name("color"))
                .emit_lines([
                    Vec2::new(aabb.x, aabb.y),
                    Vec2::new(aabb.x + aabb.w, aabb.y),
                    Vec2::new(aabb.x + aabb.w, aabb.y + aabb.h),
                    Vec2::new(aabb.x, aabb.y + aabb.h),
                ])
                .looped(true)
                .tint(text.tint)
                .draw(&mut self.context, graphics);
            self.context.pop_transform();
        }

        // Drawing particles is done with emitter that defines how
        // to render them, and expects iterator of particle instances
        // provided by some source of data, here we use particle system.
        // Additionally, we will render them as wireframe, so we can see
        // their triangles instead of what they are. Wireframe rendering
        // mode is useful for debug visuals for example.
        self.context.wireframe = true;
        ParticleEmitter::default()
            .shader(ShaderRef::name("color"))
            .emit(self.particles.emit())
            .draw(&mut self.context, graphics);
        self.context.wireframe = false;

        self.context.end_frame();
    }
}

struct ParticleData {
    size: f32,
    position: Vec2<f32>,
    velocity: Vec2<f32>,
    color: Rgba<f32>,
    lifetime: f32,
    lifetime_max: f32,
}

impl ParticleData {
    fn new(
        lifetime: Range<f32>,
        speed: Range<f32>,
        size: Range<f32>,
        position: Vec2<f32>,
        color: Rgba<f32>,
    ) -> Self {
        let lifetime = rand::thread_rng().gen_range(lifetime);
        let speed = rand::thread_rng().gen_range(speed);
        let size = rand::thread_rng().gen_range(size);
        let angle = rand::thread_rng().gen_range(0.0_f32..360.0).to_radians();
        Self {
            size,
            position,
            velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
            color,
            lifetime,
            lifetime_max: lifetime,
        }
    }
}

struct ParticlesProcessor;

// Particle processor tells particle system how to process particle
// data and how to emit particle instance out of them.
impl ParticleSystemProcessor<ParticleData, ()> for ParticlesProcessor {
    fn process(_: &(), mut data: ParticleData) -> Option<ParticleData> {
        data.lifetime -= DELTA_TIME;
        if data.lifetime >= 0.0 {
            data.position += data.velocity * DELTA_TIME;
            data.color.a = data.lifetime / data.lifetime_max;
            Some(data)
        } else {
            None
        }
    }

    fn emit(_: &(), data: &ParticleData) -> Option<ParticleInstance> {
        Some(ParticleInstance {
            tint: data.color,
            transform: Transform {
                position: data.position.into(),
                ..Default::default()
            },
            size: data.size.into(),
            ..Default::default()
        })
    }
}

// Unfortunatelly, or fortunatelly, images loading is not part of
// drawing module, so make sure you bring your own texture loader.
fn load_texture(graphics: &Graphics<Vertex>, path: impl AsRef<Path>, pages: u32) -> Texture {
    let file = File::open(path).unwrap();
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];
    graphics
        .texture(
            info.width,
            info.height / pages,
            pages,
            GlowTextureFormat::Rgba,
            Some(bytes),
        )
        .unwrap()
}

fn checkerboard_texture(graphics: &Graphics<Vertex>) -> Texture {
    graphics
        .texture(
            4,
            4,
            1,
            GlowTextureFormat::Monochromatic,
            Some(&[
                0, 255, 0, 255, 255, 0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 0,
            ]),
        )
        .unwrap()
}
