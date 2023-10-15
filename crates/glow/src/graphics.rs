use crate::renderer::{GlowBatch, GlowRenderer, GlowState, GlowUniformValue, GlowVertexAttribs};
use glow::{
    Context, HasContext, Program as GlowProgram, Shader as GlowShader, Texture as GlowTexture,
    FRAGMENT_SHADER, RGBA, TEXTURE_2D, UNSIGNED_BYTE, VERTEX_SHADER,
};
use spitfire_core::{VertexStream, VertexStreamRenderer};
use std::{borrow::Cow, cell::Cell, collections::HashMap, rc::Rc};
use vek::{FrustumPlanes, Mat4, Rect, Transform, Vec2};

pub struct Graphics<V: GlowVertexAttribs> {
    pub stream: VertexStream<V, GraphicsBatch>,
    state: GlowState,
    context: Rc<Context>,
}

impl<V: GlowVertexAttribs> Graphics<V> {
    pub fn new(context: Context) -> Self {
        Self {
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

    pub fn render_stream<const TN: usize>(&mut self) -> Result<(), String> {
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
    pub fn handle(&self) -> GlowProgram {
        self.inner.program
    }
}
