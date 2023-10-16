use crate::renderer::{GlowBatch, GlowRenderer, GlowState, GlowUniformValue, GlowVertexAttribs};
use bytemuck::{Pod, Zeroable};
use glow::{
    Context, HasContext, Program as GlowProgram, Shader as GlowShader, Texture as GlowTexture,
    BLEND, COLOR_BUFFER_BIT, FRAGMENT_SHADER, RGBA, SCISSOR_TEST, TEXTURE_2D, UNSIGNED_BYTE,
    VERTEX_SHADER,
};
use spitfire_core::{VertexStream, VertexStreamRenderer};
use std::{borrow::Cow, cell::Cell, collections::HashMap, rc::Rc};
use vek::{FrustumPlanes, Mat4, Rect, Transform, Vec2};

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex2d {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl GlowVertexAttribs for Vertex2d {
    const ATTRIBS: &'static [&'static str] = &["a_position", "a_uv"];
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex3d {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl GlowVertexAttribs for Vertex3d {
    const ATTRIBS: &'static [&'static str] = &["a_position", "a_normal", "a_uv"];
}

pub struct Graphics<V: GlowVertexAttribs> {
    pub color: [f32; 3],
    pub stream: VertexStream<V, GraphicsBatch>,
    state: GlowState,
    context: Rc<Context>,
}

impl<V: GlowVertexAttribs> Drop for Graphics<V> {
    fn drop(&mut self) {
        self.state.dispose(&self.context);
    }
}

impl<V: GlowVertexAttribs> Graphics<V> {
    pub fn new(context: Context) -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            stream: Default::default(),
            state: Default::default(),
            context: Rc::new(context),
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn texture(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        generate_mipmaps: bool,
    ) -> Result<Texture, String> {
        unsafe {
            let texture = self.context.create_texture()?;
            let mut result = Texture {
                inner: Rc::new(TextureInner {
                    context: self.context.clone(),
                    texture,
                    size: Cell::new((0, 0)),
                }),
            };
            result.upload(width, height, data, generate_mipmaps);
            Ok(result)
        }
    }

    pub fn shader(&self, vertex: &str, fragment: &str) -> Result<Shader, String> {
        unsafe {
            let vertex_shader = self.context.create_shader(VERTEX_SHADER)?;
            let fragment_shader = self.context.create_shader(FRAGMENT_SHADER)?;
            let program = self.context.create_program()?;
            self.context.shader_source(vertex_shader, vertex);
            self.context.compile_shader(vertex_shader);
            self.context.shader_source(fragment_shader, fragment);
            self.context.compile_shader(fragment_shader);
            self.context.attach_shader(program, vertex_shader);
            self.context.attach_shader(program, fragment_shader);
            self.context.link_program(program);
            Ok(Shader {
                inner: Rc::new(ShaderInner {
                    context: self.context.clone(),
                    program,
                    vertex_shader,
                    fragment_shader,
                }),
            })
        }
    }

    pub fn draw<const TN: usize>(&mut self) -> Result<(), String> {
        let [r, g, b] = self.color;
        unsafe {
            self.context.bind_vertex_array(None);
            self.context.use_program(None);
            self.context.disable(BLEND);
            self.context.disable(SCISSOR_TEST);
            self.context.clear_color(r, g, b, 1.0);
            self.context.clear(COLOR_BUFFER_BIT);
        }
        let mut renderer = GlowRenderer::<GraphicsBatch, TN>::new(&self.context, &mut self.state);
        renderer.render(&mut self.stream)?;
        self.stream.clear();
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Camera {
    pub viewport_size: Vec2<f32>,
    pub transform: Transform<f32, f32, f32>,
}

impl Camera {
    pub fn projection_matrix(&self) -> Mat4<f32> {
        Mat4::orthographic_without_depth_planes(FrustumPlanes {
            left: 0.0,
            right: self.viewport_size.x,
            top: 0.0,
            bottom: self.viewport_size.y,
            near: -1.0,
            far: 1.0,
        })
    }

    pub fn view_matrix(&self) -> Mat4<f32> {
        Mat4::from(self.transform).inverted()
    }

    pub fn matrix(&self) -> Mat4<f32> {
        self.projection_matrix() * self.view_matrix()
    }
}

#[derive(Default, Clone)]
pub struct GraphicsBatch {
    pub shader: Option<(Shader, HashMap<Cow<'static, str>, GlowUniformValue>)>,
    pub textures: Vec<Option<Texture>>,
    /// (source, destination)?
    pub blending: Option<(u32, u32)>,
    pub scissor: Option<Rect<i32, i32>>,
}

#[allow(clippy::from_over_into)]
impl<const TN: usize> Into<GlowBatch<TN>> for GraphicsBatch {
    fn into(self) -> GlowBatch<TN> {
        GlowBatch {
            shader_program: self.shader.map(|(s, u)| (s.handle(), u)),
            textures: {
                let mut result = [None; TN];
                for (from, to) in self.textures.into_iter().zip(result.iter_mut()) {
                    *to = from.map(|v| (v.handle(), TEXTURE_2D));
                }
                result
            },
            blending: self.blending,
            scissor: self.scissor.map(|v| [v.x, v.y, v.w, v.h]),
        }
    }
}

struct TextureInner {
    context: Rc<Context>,
    texture: GlowTexture,
    size: Cell<(u32, u32)>,
}

impl Drop for TextureInner {
    fn drop(&mut self) {
        unsafe {
            self.context.delete_texture(self.texture);
        }
    }
}

#[derive(Clone)]
pub struct Texture {
    inner: Rc<TextureInner>,
}

impl Texture {
    pub fn handle(&self) -> GlowTexture {
        self.inner.texture
    }

    pub fn width(&self) -> u32 {
        self.inner.size.get().0
    }

    pub fn height(&self) -> u32 {
        self.inner.size.get().1
    }

    pub fn upload(&mut self, width: u32, height: u32, data: &[u8], generaet_mipmaps: bool) {
        unsafe {
            self.inner
                .context
                .bind_texture(TEXTURE_2D, Some(self.inner.texture));
            self.inner.context.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA as _,
                width as _,
                height as _,
                0,
                RGBA,
                UNSIGNED_BYTE,
                Some(data),
            );
            if generaet_mipmaps {
                self.inner.context.generate_mipmap(TEXTURE_2D);
            }
            self.inner.size.set((width, height));
        }
    }
}

struct ShaderInner {
    context: Rc<Context>,
    program: GlowProgram,
    vertex_shader: GlowShader,
    fragment_shader: GlowShader,
}

impl Drop for ShaderInner {
    fn drop(&mut self) {
        unsafe {
            self.context.delete_program(self.program);
            self.context.delete_shader(self.vertex_shader);
            self.context.delete_shader(self.fragment_shader);
        }
    }
}

#[derive(Clone)]
pub struct Shader {
    inner: Rc<ShaderInner>,
}

impl Shader {
    pub const DEFAULT_VERTEX_2D: &str = r#"#version 300 es
    in vec2 a_position;
    in vec4 a_color;
    out vec4 v_color;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 0.0, 1.0);
        v_color = a_color;
    }
    "#;
    pub const DEFAULT_VERTEX_3D: &str = r#"#version 300 es
    in vec3 a_position;
    in vec4 a_color;
    out vec4 v_color;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 1.0);
        v_color = a_color;
    }
    "#;
    pub const DEFAULT_FRAGMENT: &str = r#"#version 300 es
    precision highp float;
    in vec4 v_color;
    out vec4 o_color;

    void main() {
        o_color = v_color;
    }
    "#;

    pub fn handle(&self) -> GlowProgram {
        self.inner.program
    }
}
