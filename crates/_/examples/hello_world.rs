use bytemuck::{Pod, Zeroable};
use fontdue::{
    Font,
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
};
use spitfire_fontdue::*;
use spitfire_glow::{
    app::{App, AppControl, AppState},
    graphics::{Graphics, GraphicsBatch, Shader, Texture},
    renderer::{
        GlowBlending, GlowTextureFiltering, GlowTextureFormat, GlowUniformValue, GlowVertexAttrib,
        GlowVertexAttribs,
    },
};
use std::{collections::HashMap, fs::File, path::Path};

fn main() {
    // App can be parameterized with AppConfig, here we just
    // use default and run it with our app state.
    App::<Vertex>::default().run(State::default());
}

// It all starts with creating vertex type
// that streaming renderers will use.
// Remember to make it repr(C) and implement Pod!
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 3],
    pub color: [f32; 4],
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Default::default(),
            uv: Default::default(),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

// This trait allows GLOW renderer to map
// vertex fields to GPU attributes.
impl GlowVertexAttribs for Vertex {
    const ATTRIBS: &'static [(&'static str, GlowVertexAttrib)] = &[
        (
            "a_position",
            GlowVertexAttrib::Float {
                channels: 2,
                normalized: false,
            },
        ),
        (
            "a_uv",
            GlowVertexAttrib::Float {
                channels: 3,
                normalized: false,
            },
        ),
        (
            "a_color",
            GlowVertexAttrib::Float {
                channels: 4,
                normalized: false,
            },
        ),
    ];
}

// This trait allows Fontdue renderer to
// apply text position and coords.
impl TextVertex<[f32; 4]> for Vertex {
    fn apply(&mut self, position: [f32; 2], tex_coord: [f32; 3], user_data: [f32; 4]) {
        self.position = position;
        self.uv = tex_coord;
        self.color = user_data;
    }
}

// Application state.
// It's advised to store acquired graphics resources here.
#[derive(Default)]
struct State {
    color_shader: Option<Shader>,
    sprite_shader: Option<Shader>,
    text_shader: Option<Shader>,
    ferris_texture: Option<Texture>,
    text_renderer: Option<TextRenderer<[f32; 4]>>,
    fonts_texture: Option<Texture>,
    fonts: Vec<Font>,
}

impl AppState<Vertex> for State {
    // init gets called as soon as Graphics gets constructed.
    // at this phase you might want to setup Graphics and
    // acquire resources.
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        graphics.state.color = [0.25, 0.25, 0.25, 1.0];
        graphics.state.main_camera.screen_alignment = 0.5.into();

        self.color_shader = Some(
            graphics
                .shader(Shader::COLORED_VERTEX_2D, Shader::PASS_FRAGMENT)
                .unwrap(),
        );

        self.sprite_shader = Some(
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );

        self.text_shader = Some(
            graphics
                .shader(Shader::TEXT_VERTEX, Shader::TEXT_FRAGMENT)
                .unwrap(),
        );

        self.ferris_texture = Some(load_texture(graphics, "resources/ferris.png"));

        self.text_renderer = Some(TextRenderer::new(1024, 1024));

        self.fonts_texture = Some(graphics.pixel_texture([0, 0, 0]).unwrap());

        self.fonts.push(
            Font::from_bytes(
                include_bytes!("../../../resources/Roboto-Regular.ttf") as &[_],
                Default::default(),
            )
            .unwrap(),
        );
    }

    // redraw gets called whenever window processes its main events.
    // here you want to stream vertices into Graphics's stream.
    // stream will be rendered after this method completes.
    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        let text_renderer = self.text_renderer.as_mut().unwrap();
        let fonts_texture = self.fonts_texture.as_mut().unwrap();
        let ferris_texture = self.ferris_texture.clone().unwrap();
        let ferris_vertices = texture_quad(&ferris_texture);
        text_renderer.clear();

        let mut uniforms = HashMap::default();
        uniforms.insert(
            "u_projection_view".into(),
            GlowUniformValue::M4(graphics.state.main_camera.world_matrix().into_col_array()),
        );
        uniforms.insert("u_image".into(), GlowUniformValue::I1(0));

        graphics.state.stream.batch(GraphicsBatch {
            shader: self.color_shader.clone(),
            uniforms: uniforms.clone(),
            ..Default::default()
        });

        graphics.state.stream.triangle_fan([
            Vertex {
                position: [-500.0, -500.0],
                uv: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [500.0, -500.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [500.0, 0.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            Vertex {
                position: [-500.0, 0.0],
                uv: [0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ]);

        graphics.state.stream.triangle_strip([
            Vertex {
                position: [-500.0, 0.0],
                uv: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [-500.0, 500.0],
                uv: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 500.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [500.0, 0.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            Vertex {
                position: [500.0, 500.0],
                uv: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ]);

        graphics.state.stream.batch(GraphicsBatch {
            shader: self.sprite_shader.clone(),
            uniforms: uniforms.clone(),
            textures: vec![(ferris_texture, GlowTextureFiltering::Linear)],
            blending: GlowBlending::Alpha,
            ..Default::default()
        });

        graphics.state.stream.quad(ferris_vertices);

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            x: -450.0,
            y: 170.0,
            ..Default::default()
        });
        let text =
            TextStyle::with_user_data("Welcome to Spitfire!", 100.0, 0, [0.0, 0.8, 1.0, 1.0]);
        layout.append(&self.fonts, &text);
        text_renderer.include(&self.fonts, &layout);

        let [width, height, depth] = text_renderer.atlas_size();
        fonts_texture.upload(
            width as _,
            height as _,
            depth as _,
            GlowTextureFormat::Monochromatic,
            Some(text_renderer.image()),
        );

        graphics.state.stream.batch(GraphicsBatch {
            shader: self.text_shader.clone(),
            uniforms: uniforms.clone(),
            textures: vec![(fonts_texture.clone(), GlowTextureFiltering::Linear)],
            blending: GlowBlending::Alpha,
            ..Default::default()
        });

        text_renderer.render_to_stream(&mut graphics.state.stream);
    }
}

fn texture_quad(texture: &Texture) -> [Vertex; 4] {
    let w = texture.width() as f32;
    let h = texture.height() as f32;
    [
        Vertex {
            position: [-w, -h],
            uv: [0.0, 0.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [w, -h],
            uv: [1.0, 0.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [w, h],
            uv: [1.0, 1.0, 0.0],
            ..Default::default()
        },
        Vertex {
            position: [-w, h],
            uv: [0.0, 1.0, 0.0],
            ..Default::default()
        },
    ]
}

fn load_texture<V: GlowVertexAttribs>(graphics: &Graphics<V>, path: impl AsRef<Path>) -> Texture {
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
