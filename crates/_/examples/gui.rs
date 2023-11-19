use fontdue::Font;
use raui_immediate_widgets::prelude::*;
use spitfire_draw::prelude::*;
use spitfire_glow::prelude::*;
use spitfire_gui::prelude::*;
use std::{fs::File, path::Path};

#[derive(Default)]
struct State {
    draw: DrawContext,
    gui: GuiContext,
}

impl State {
    fn draw_gui(&mut self) {
        vertical_box((), || {
            content_box((), || {
                image_box(ImageBoxProps::colored(Color {
                    r: 0.0,
                    g: 0.75,
                    b: 0.0,
                    a: 1.0,
                }));

                text_box(TextBoxProps {
                    text: "Hello World!".to_owned(),
                    horizontal_align: TextBoxHorizontalAlign::Center,
                    vertical_align: TextBoxVerticalAlign::Middle,
                    font: TextBoxFont {
                        name: "roboto".to_owned(),
                        size: 64.0,
                    },
                    color: Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.75,
                        a: 1.0,
                    },
                    ..Default::default()
                });
            });

            image_box(ImageBoxProps::image_aspect_ratio("ferris", false));
        });
    }
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>) {
        graphics.color = [0.25, 0.25, 0.25];
        graphics.main_camera.screen_alignment = 0.5.into();

        self.draw.shaders.insert(
            "color".into(),
            graphics
                .shader(Shader::COLORED_VERTEX_2D, Shader::PASS_FRAGMENT)
                .unwrap(),
        );

        self.draw.shaders.insert(
            "image".into(),
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );

        self.draw.shaders.insert(
            "text".into(),
            graphics
                .shader(Shader::TEXT_VERTEX, Shader::TEXT_FRAGMENT)
                .unwrap(),
        );

        self.draw.textures.insert(
            "ferris".into(),
            load_texture(graphics, "resources/ferris.png"),
        );

        self.draw.fonts.insert(
            "roboto",
            Font::from_bytes(
                include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_],
                Default::default(),
            )
            .unwrap(),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>) {
        self.draw.begin_frame(graphics);
        self.draw.push_shader(&ShaderRef::name("image"));
        self.draw.push_blending(GlowBlending::Alpha);

        self.gui.begin_frame();
        self.draw_gui();
        self.gui.end_frame(
            &mut self.draw,
            graphics,
            &ShaderRef::name("color"),
            &ShaderRef::name("image"),
            &ShaderRef::name("text"),
        );

        self.draw.end_frame();
    }
}

fn main() {
    App::<Vertex>::default().run::<State>(State::default());
}

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
