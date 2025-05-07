use spitfire_draw::prelude::*;
use spitfire_glow::prelude::*;
use std::{fs::File, path::Path};

fn main() {
    App::<Vertex>::default().run(State::default());
}

#[derive(Default)]
struct State {
    context: DrawContext,
    canvas: Option<Canvas>,
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        graphics.color = [0.25, 0.25, 0.25, 1.0];
        graphics.main_camera.screen_alignment = 0.5.into();

        self.context.shaders.insert(
            "image".into(),
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );

        self.context.textures.insert(
            "ferris".into(),
            load_texture(graphics, "resources/ferris.png"),
        );

        // We create simple fixed size canvas with single texture.
        // Canvas stores Surface which points to one or many
        // Texture objects that we can render into at once.
        self.canvas = Some(
            Canvas::simple(400, 200, GlowTextureFormat::Rgba, graphics)
                .unwrap()
                .color([0.0, 0.0, 1.0, 0.5]),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        self.context.begin_frame(graphics);
        self.context.push_shader(&ShaderRef::name("image"));
        self.context.push_blending(GlowBlending::Alpha);

        Sprite::single(SpriteTexture::new(
            "u_image".into(),
            TextureRef::name("ferris"),
        ))
        .pivot(0.5.into())
        .position((-150.0).into())
        .draw(&mut self.context, graphics);

        if let Some(canvas) = &self.canvas {
            // To draw to Canvas we can either do it within `Canvas::with` call,
            // or by manually calling `Canvas::activate`, drawing something and
            // calling `Canvas::deactivate`.
            canvas.with(&mut self.context, graphics, true, |context, graphics| {
                // Once we start render to Canvas, we should initialize its defaults
                // like we do without Canvas, because by activating Canvas we are
                // resetting the defaults. The only thing that does not reset is
                // `Graphics::main_camera` (except `screen_size` which gets applied
                // from Surface texture size).
                context.push_shader(&ShaderRef::name("image"));
                context.push_blending(GlowBlending::Alpha);

                Sprite::single(SpriteTexture::new(
                    "u_image".into(),
                    TextureRef::name("ferris"),
                ))
                .pivot(0.5.into())
                .draw(context, graphics);
            });
        }

        // After drawing to Canvas we should again re-initialize defaults.
        self.context.push_shader(&ShaderRef::name("image"));
        self.context.push_blending(GlowBlending::Alpha);

        Sprite::single(SpriteTexture::new(
            "u_image".into(),
            TextureRef::name("ferris"),
        ))
        .pivot(0.5.into())
        .position(150.0.into())
        .draw(&mut self.context, graphics);

        if let Some(canvas) = &self.canvas {
            // After we get image in our Canvas, we can get (sprite) textures
            // out of it by index and draw with that Canvas content.
            Sprite::single(
                canvas
                    .sprite_texture(0, "u_image".into(), GlowTextureFiltering::Linear)
                    .unwrap(),
            )
            .pivot(0.5.into())
            // We apply negative scale in Y axis to account for flipped Canvas content.
            .scale([1.0, -1.0].into())
            .draw(&mut self.context, graphics);
        }

        self.context.end_frame();
    }
}

fn load_texture(graphics: &Graphics<Vertex>, path: impl AsRef<Path>) -> Texture {
    let file = File::open(path).unwrap();
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];
    graphics
        .texture(
            info.width,
            info.height,
            1,
            GlowTextureFormat::Rgba,
            Some(bytes),
        )
        .unwrap()
}
