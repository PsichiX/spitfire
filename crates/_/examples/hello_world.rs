use spitfire_glow::prelude::*;
use std::{collections::HashMap, fs::File, path::Path};

// Application state.
// It's advised to store acquired graphics resources here.
#[derive(Debug, Default)]
struct State {
    shader: Option<Shader>,
    texture: Option<Texture>,
}

impl AppState<Vertex2d> for State {
    // init gets called as soon as Graphics gets constructed.
    // at this phase you might want to setup Graphics and
    // acquire resources.
    fn on_init(&mut self, graphics: &mut Graphics<Vertex2d>) {
        graphics.color = [0.25, 0.25, 0.25];
        graphics.main_camera.screen_alignment = 0.5.into();

        self.shader = Some(
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );

        self.texture = Some(load_texture(graphics, "resources/ferris.png"));
    }

    // redraw gets called whenever window processes its main events.
    // here you want to stream vertices into Graphics's stream.
    // stream will be rendered after this method completes.
    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex2d>) {
        let texture = self.texture.clone().unwrap();
        let vertices = texture_quad(&texture);

        let mut uniforms = HashMap::default();
        uniforms.insert(
            "u_projection_view".into(),
            GlowUniformValue::M4(graphics.main_camera.projection_matrix().into_col_array()),
        );
        uniforms.insert("u_image".into(), GlowUniformValue::I1(0));

        graphics.stream.batch(GraphicsBatch {
            shader: self
                .shader
                .as_ref()
                .map(|shader| (shader.clone(), uniforms)),
            textures: vec![Some((texture, GlowTextureFiltering::Linear))],
            blending: GlowBlending::Alpha,
            ..Default::default()
        });

        graphics.stream.quad(vertices);

        graphics.stream.batch_end();
    }
}

fn main() {
    // App can be parameterized, here we just use defaults and run
    // it with our app state. It's worth noting that you can make
    // your own Vertex types, here we use default one provided.
    // we also have to define number of texture units we are using.
    App::default().run::<State, Vertex2d, 1>(State::default());
}

fn texture_quad(texture: &Texture) -> [Vertex2d; 4] {
    let w = texture.width() as f32;
    let h = texture.height() as f32;
    [
        Vertex2d {
            position: [-w, -h],
            uv: [0.0, 0.0],
            ..Default::default()
        },
        Vertex2d {
            position: [w, -h],
            uv: [1.0, 0.0],
            ..Default::default()
        },
        Vertex2d {
            position: [w, h],
            uv: [1.0, 1.0],
            ..Default::default()
        },
        Vertex2d {
            position: [-w, h],
            uv: [0.0, 1.0],
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
        .texture(info.width, info.height, bytes, true)
        .unwrap()
}
