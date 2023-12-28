use crate::renderer::{
    GlowBatch, GlowBlending, GlowRenderer, GlowState, GlowTextureFiltering, GlowTextureFormat,
    GlowUniformValue, GlowVertexAttrib, GlowVertexAttribs,
};
use bytemuck::{Pod, Zeroable};
use glow::{
    Context, HasContext, Program as GlowProgram, Shader as GlowShader, Texture as GlowTexture,
    BLEND, CLAMP_TO_EDGE, COLOR_BUFFER_BIT, FRAGMENT_SHADER, NEAREST, SCISSOR_TEST,
    TEXTURE_2D_ARRAY, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE_WRAP_R, TEXTURE_WRAP_S,
    TEXTURE_WRAP_T, UNSIGNED_BYTE, VERTEX_SHADER,
};
use spitfire_core::{VertexStream, VertexStreamRenderer};
use std::{
    borrow::Cow,
    cell::{Cell, Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};
use vek::{FrustumPlanes, Mat4, Rect, Transform, Vec2};

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex3d {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 3],
    pub color: [f32; 4],
}

impl GlowVertexAttribs for Vertex3d {
    const ATTRIBS: &'static [(&'static str, GlowVertexAttrib)] = &[
        (
            "a_position",
            GlowVertexAttrib::Float {
                channels: 3,
                normalized: false,
            },
        ),
        (
            "a_normal",
            GlowVertexAttrib::Float {
                channels: 3,
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

impl Default for Vertex3d {
    fn default() -> Self {
        Self {
            position: Default::default(),
            normal: [0.0, 0.0, 1.0],
            uv: Default::default(),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaybeContext(Rc<RefCell<(Context, bool)>>);

impl MaybeContext {
    pub fn get(&self) -> Option<Ref<Context>> {
        let access = self.0.borrow();
        if access.1 {
            Some(Ref::map(access, |access| &access.0))
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct StrongContext(MaybeContext);

impl Drop for StrongContext {
    fn drop(&mut self) {
        (self.0).0.borrow_mut().1 = false;
    }
}

impl StrongContext {
    fn get(&self) -> Option<Ref<Context>> {
        self.0.get()
    }

    fn new(context: Context) -> Self {
        Self(MaybeContext(Rc::new(RefCell::new((context, true)))))
    }
}

pub struct Graphics<V: GlowVertexAttribs> {
    pub main_camera: Camera,
    pub color: [f32; 3],
    pub stream: VertexStream<V, GraphicsBatch>,
    state: GlowState,
    context: StrongContext,
}

impl<V: GlowVertexAttribs> Drop for Graphics<V> {
    fn drop(&mut self) {
        if let Some(context) = self.context.get() {
            self.state.dispose(&context);
        }
    }
}

impl<V: GlowVertexAttribs> Graphics<V> {
    pub fn new(context: Context) -> Self {
        Self {
            main_camera: Default::default(),
            color: [1.0, 1.0, 1.0],
            stream: Default::default(),
            state: Default::default(),
            context: StrongContext::new(context),
        }
    }

    pub fn context(&self) -> Option<Ref<Context>> {
        self.context.get()
    }

    pub fn pixel_texture(&self, color: [u8; 3]) -> Result<Texture, String> {
        self.texture(1, 1, 1, GlowTextureFormat::Rgb, &color)
    }

    pub fn texture(
        &self,
        width: u32,
        height: u32,
        depth: u32,
        format: GlowTextureFormat,
        data: &[u8],
    ) -> Result<Texture, String> {
        unsafe {
            if let Some(context) = self.context.get() {
                let texture = context.create_texture()?;
                let mut result = Texture {
                    inner: Rc::new(TextureInner {
                        context: self.context.0.clone(),
                        texture,
                        size: Cell::new((0, 0, 0)),
                    }),
                };
                result.upload(width, height, depth, format, data);
                Ok(result)
            } else {
                Err("Invalid context".to_owned())
            }
        }
    }

    pub fn shader(&self, vertex: &str, fragment: &str) -> Result<Shader, String> {
        unsafe {
            if let Some(context) = self.context.get() {
                let vertex_shader = context.create_shader(VERTEX_SHADER)?;
                let fragment_shader = context.create_shader(FRAGMENT_SHADER)?;
                let program = context.create_program()?;
                context.shader_source(vertex_shader, vertex);
                context.compile_shader(vertex_shader);
                if !context.get_shader_compile_status(vertex_shader) {
                    return Err(format!(
                        "Vertex Shader: {}",
                        context.get_shader_info_log(vertex_shader)
                    ));
                }
                context.shader_source(fragment_shader, fragment);
                context.compile_shader(fragment_shader);
                if !context.get_shader_compile_status(fragment_shader) {
                    return Err(format!(
                        "Fragment Shader: {}",
                        context.get_shader_info_log(fragment_shader)
                    ));
                }
                context.attach_shader(program, vertex_shader);
                context.attach_shader(program, fragment_shader);
                context.link_program(program);
                if !context.get_program_link_status(program) {
                    return Err(format!(
                        "Shader Program: {}",
                        context.get_program_info_log(program)
                    ));
                }
                Ok(Shader {
                    inner: Rc::new(ShaderInner {
                        context: self.context.0.clone(),
                        program,
                        vertex_shader,
                        fragment_shader,
                        shared_uniforms: Default::default(),
                    }),
                })
            } else {
                Err("Invalid context".to_owned())
            }
        }
    }

    pub fn prepare_frame(&self) {
        unsafe {
            if let Some(context) = self.context.get() {
                let [r, g, b] = self.color;
                context.bind_texture(TEXTURE_2D_ARRAY, None);
                context.bind_vertex_array(None);
                context.use_program(None);
                context.disable(BLEND);
                context.disable(SCISSOR_TEST);
                context.clear_color(r, g, b, 1.0);
                context.clear(COLOR_BUFFER_BIT);
            }
        }
    }

    pub fn draw(&mut self) -> Result<(), String> {
        if let Some(context) = self.context.get() {
            let mut renderer = GlowRenderer::<GraphicsBatch>::new(&context, &mut self.state);
            self.stream.batch_end();
            renderer.render(&mut self.stream)?;
            self.stream.clear();
            Ok(())
        } else {
            Err("Invalid context".to_owned())
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum CameraScaling {
    #[default]
    None,
    Constant(f32),
    Stretch(Vec2<f32>),
    FitHorizontal(f32),
    FitVertical(f32),
    FitToView {
        size: Vec2<f32>,
        inside: bool,
    },
}

impl CameraScaling {
    pub fn world_size(self, viewport_size: Vec2<f32>) -> Vec2<f32> {
        match self {
            Self::None => viewport_size,
            Self::Constant(value) => viewport_size * value,
            Self::Stretch(size) => size,
            Self::FitHorizontal(value) => Vec2 {
                x: value,
                y: value * viewport_size.y / viewport_size.x,
            },
            Self::FitVertical(value) => Vec2 {
                x: value * viewport_size.x / viewport_size.y,
                y: value,
            },
            Self::FitToView { size, inside } => {
                let source_aspect = size.x / size.y;
                let target_aspect = viewport_size.x / viewport_size.y;
                if (target_aspect >= source_aspect) != inside {
                    Vec2 {
                        x: viewport_size.x * size.x / viewport_size.y,
                        y: size.y,
                    }
                } else {
                    Vec2 {
                        x: size.x,
                        y: viewport_size.y * size.y / viewport_size.x,
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Camera {
    pub screen_alignment: Vec2<f32>,
    pub screen_size: Vec2<f32>,
    pub scaling: CameraScaling,
    pub transform: Transform<f32, f32, f32>,
}

impl Camera {
    pub fn screen_projection_matrix(&self) -> Mat4<f32> {
        Mat4::orthographic_without_depth_planes(FrustumPlanes {
            left: 0.0,
            right: self.screen_size.x,
            top: 0.0,
            bottom: self.screen_size.y,
            near: -1.0,
            far: 1.0,
        })
    }

    pub fn screen_matrix(&self) -> Mat4<f32> {
        self.screen_projection_matrix()
    }

    pub fn world_size(&self) -> Vec2<f32> {
        self.scaling.world_size(self.screen_size)
    }

    pub fn world_offset(&self) -> Vec2<f32> {
        self.world_size() * -self.screen_alignment
    }

    pub fn world_projection_matrix(&self) -> Mat4<f32> {
        let size = self.world_size();
        let offset = size * -self.screen_alignment;
        Mat4::orthographic_without_depth_planes(FrustumPlanes {
            left: offset.x,
            right: size.x + offset.x,
            top: offset.y,
            bottom: size.y + offset.y,
            near: -1.0,
            far: 1.0,
        })
    }

    pub fn world_view_matrix(&self) -> Mat4<f32> {
        Mat4::from(self.transform).inverted()
    }

    pub fn world_matrix(&self) -> Mat4<f32> {
        self.world_projection_matrix() * self.world_view_matrix()
    }

    pub fn world_polygon(&self) -> [Vec2<f32>; 4] {
        let matrix = self.world_matrix().inverted();
        [
            matrix.mul_point(Vec2::new(-1.0, -1.0)),
            matrix.mul_point(Vec2::new(1.0, -1.0)),
            matrix.mul_point(Vec2::new(1.0, 1.0)),
            matrix.mul_point(Vec2::new(-1.0, 1.0)),
        ]
    }

    pub fn world_rectangle(&self) -> Rect<f32, f32> {
        let [tl, tr, br, bl] = self.world_polygon();
        let xf = tl.x.min(tr.x).min(br.x).min(bl.x);
        let xt = tl.x.max(tr.x).max(br.x).max(bl.x);
        let yf = tl.y.min(tr.y).min(br.y).min(bl.y);
        let yt = tl.y.max(tr.y).max(br.y).max(bl.y);
        Rect {
            x: xf,
            y: yf,
            w: xt - xf,
            h: yt - yf,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GraphicsBatch {
    pub shader: Option<Shader>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    pub textures: Vec<(Texture, GlowTextureFiltering)>,
    /// (source, destination)?
    pub blending: GlowBlending,
    pub scissor: Option<Rect<i32, i32>>,
}

#[allow(clippy::from_over_into)]
impl Into<GlowBatch> for GraphicsBatch {
    fn into(self) -> GlowBatch {
        GlowBatch {
            shader_program: self.shader.as_ref().map(|shader| shader.handle()),
            uniforms: if let Some(shader) = self.shader.as_ref() {
                let uniforms = &*shader.inner.shared_uniforms.borrow();
                if uniforms.is_empty() {
                    self.uniforms
                } else {
                    uniforms
                        .iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .chain(self.uniforms)
                        .collect()
                }
            } else {
                self.uniforms
            },
            textures: self
                .textures
                .into_iter()
                .map(|(texture, filtering)| {
                    let (min, mag) = filtering.into_gl();
                    (texture.handle(), TEXTURE_2D_ARRAY, min, mag)
                })
                .collect(),
            blending: self.blending.into_gl(),
            scissor: self.scissor.map(|v| [v.x, v.y, v.w, v.h]),
        }
    }
}

#[derive(Debug)]
struct TextureInner {
    context: MaybeContext,
    texture: GlowTexture,
    size: Cell<(u32, u32, u32)>,
}

impl Drop for TextureInner {
    fn drop(&mut self) {
        unsafe {
            if let Some(context) = self.context.get() {
                context.delete_texture(self.texture);
            }
        }
    }
}

#[derive(Debug, Clone)]
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

    pub fn depth(&self) -> u32 {
        self.inner.size.get().2
    }

    pub fn upload(
        &mut self,
        width: u32,
        height: u32,
        depth: u32,
        format: GlowTextureFormat,
        data: &[u8],
    ) {
        unsafe {
            if let Some(context) = self.inner.context.get() {
                context.bind_texture(TEXTURE_2D_ARRAY, Some(self.inner.texture));
                context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_WRAP_S, CLAMP_TO_EDGE as _);
                context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_WRAP_T, CLAMP_TO_EDGE as _);
                context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_WRAP_R, CLAMP_TO_EDGE as _);
                context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_MIN_FILTER, NEAREST as _);
                context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_MAG_FILTER, NEAREST as _);
                context.tex_image_3d(
                    TEXTURE_2D_ARRAY,
                    0,
                    format.into_gl() as _,
                    width as _,
                    height as _,
                    depth as _,
                    0,
                    format.into_gl(),
                    UNSIGNED_BYTE,
                    Some(data),
                );
                self.inner.size.set((width, height, depth));
            }
        }
    }
}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Debug)]
struct ShaderInner {
    context: MaybeContext,
    program: GlowProgram,
    vertex_shader: GlowShader,
    fragment_shader: GlowShader,
    shared_uniforms: RefCell<HashMap<Cow<'static, str>, GlowUniformValue>>,
}

impl Drop for ShaderInner {
    fn drop(&mut self) {
        unsafe {
            if let Some(context) = self.context.get() {
                context.delete_program(self.program);
                context.delete_shader(self.vertex_shader);
                context.delete_shader(self.fragment_shader);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shader {
    inner: Rc<ShaderInner>,
}

impl Shader {
    pub const PASS_VERTEX_2D: &'static str = r#"#version 300 es
    layout(location = 0) in vec2 a_position;
    layout(location = 2) in vec4 a_color;
    out vec4 v_color;

    void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
        v_color = a_color;
    }
    "#;

    pub const PASS_VERTEX_3D: &'static str = r#"#version 300 es
    layout(location = 0) in vec3 a_position;
    layout(location = 3) in vec4 a_color;
    out vec4 v_color;

    void main() {
        gl_Position = vec4(a_position, 1.0);
        v_color = a_color;
    }
    "#;

    pub const PASS_FRAGMENT: &'static str = r#"#version 300 es
    precision highp float;
    precision highp int;
    in vec4 v_color;
    out vec4 o_color;

    void main() {
        o_color = v_color;
    }
    "#;

    pub const COLORED_VERTEX_2D: &'static str = r#"#version 300 es
    layout(location = 0) in vec2 a_position;
    layout(location = 2) in vec4 a_color;
    out vec4 v_color;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 0.0, 1.0);
        v_color = a_color;
    }
    "#;

    pub const COLORED_VERTEX_3D: &'static str = r#"#version 300 es
    layout(location = 0) in vec3 a_position;
    layout(location = 3) in vec4 a_color;
    out vec4 v_color;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 1.0);
        v_color = a_color;
    }
    "#;

    pub const TEXTURED_VERTEX_2D: &'static str = r#"#version 300 es
    layout(location = 0) in vec2 a_position;
    layout(location = 1) in vec3 a_uv;
    layout(location = 2) in vec4 a_color;
    out vec4 v_color;
    out vec3 v_uv;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 0.0, 1.0);
        v_color = a_color;
        v_uv = a_uv;
    }
    "#;

    pub const TEXTURED_VERTEX_3D: &'static str = r#"#version 300 es
    layout(location = 0) in vec3 a_position;
    layout(location = 2) in vec3 a_uv;
    layout(location = 3) in vec4 a_color;
    out vec4 v_color;
    out vec3 v_uv;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 1.0);
        v_color = a_color;
        v_uv = a_uv;
    }
    "#;

    pub const TEXTURED_FRAGMENT: &'static str = r#"#version 300 es
    precision highp float;
    precision highp int;
    precision highp sampler2DArray;
    in vec4 v_color;
    in vec3 v_uv;
    out vec4 o_color;
    uniform sampler2DArray u_image;

    void main() {
        o_color = texture(u_image, v_uv) * v_color;
    }
    "#;

    pub const TEXT_VERTEX: &'static str = r#"#version 300 es
    layout(location = 0) in vec2 a_position;
    layout(location = 1) in vec3 a_uv;
    layout(location = 2) in vec4 a_color;
    out vec4 v_color;
    out vec3 v_uv;
    uniform mat4 u_projection_view;

    void main() {
        gl_Position = u_projection_view * vec4(a_position, 0.0, 1.0);
        v_color = a_color;
        v_uv = a_uv;
    }
    "#;

    pub const TEXT_FRAGMENT: &'static str = r#"#version 300 es
    precision highp float;
    precision highp int;
    precision highp sampler2DArray;
    in vec4 v_color;
    in vec3 v_uv;
    out vec4 o_color;
    uniform sampler2DArray u_image;

    void main() {
        float alpha = texture(u_image, v_uv).x;
        o_color = vec4(v_color.xyz, v_color.w * alpha);
    }
    "#;

    pub fn handle(&self) -> GlowProgram {
        self.inner.program
    }

    pub fn set_shared_uniform(
        &mut self,
        id: impl Into<Cow<'static, str>>,
        value: GlowUniformValue,
    ) {
        self.inner
            .shared_uniforms
            .borrow_mut()
            .insert(id.into(), value);
    }

    pub fn unset_shared_uniform(&mut self, id: &str) {
        self.inner.shared_uniforms.borrow_mut().remove(id);
    }

    pub fn get_shared_uniform(&self, id: &str) -> Option<GlowUniformValue> {
        self.inner.shared_uniforms.borrow().get(id).cloned()
    }
}

impl PartialEq for Shader {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
