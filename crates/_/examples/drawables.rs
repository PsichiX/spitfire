use fontdue::{layout::TextStyle, Font};
use spitfire_draw::prelude::*;
use spitfire_glow::prelude::*;
use std::{fs::File, path::Path};

#[derive(Default)]
struct State {
    // We store drawing context for later use in app state.
    // Drawing context holds resources and stack-based states.
    context: DrawContext,
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>) {
        graphics.color = [0.25, 0.25, 0.25];
        graphics.main_camera.screen_alignment = 0.5.into();

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
            load_texture(graphics, "resources/ferris.png"),
        );

        self.context.fonts.push(
            Font::from_bytes(
                include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_],
                Default::default(),
            )
            .unwrap(),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>) {
        // Each scene draw phase should start with `DrawContext::begin_frame`
        // and should end with `DrawContext::end_frame`.
        self.context.begin_frame(graphics);

        // When new frame starts, there is no default shader and
        // no default blending, so we should push those.
        self.context.push_shader(&ShaderRef::name("image"));
        self.context.push_blending(GlowBlending::Alpha);

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
        Text::new(ShaderRef::name("text"))
            .text_style(
                &self.context,
                &TextStyle::with_user_data(
                    "Welcome to Spitfire!",
                    100.0,
                    0,
                    [0.0, 0.8, 1.0, 1.0].into(),
                ),
            )
            .position([-450.0, 170.0].into())
            .draw(&mut self.context, graphics);

        self.context.end_frame();
    }
}

fn main() {
    App::<Vertex>::default().run::<State>(State::default());
}

// Unfortunatelly, or fortunatelly, images loading is not part of
// drawing module, so make sure you bring your own texture loader.
fn load_texture(graphics: &Graphics<Vertex>, path: impl AsRef<Path>) -> Texture {
    let file = File::open(path).unwrap();
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];
    graphics
        .texture(info.width, info.height, 1, GlowTextureFormat::Rgba, bytes)
        .unwrap()
}
