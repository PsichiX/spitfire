use bytemuck::{Pod, checked::cast_slice};
use glow::{
    ARRAY_BUFFER, BLEND, Buffer, Context, DST_COLOR, ELEMENT_ARRAY_BUFFER, FILL, FLOAT,
    FRONT_AND_BACK, HasContext, INT, LINE, LINEAR, NEAREST, ONE, ONE_MINUS_SRC_ALPHA, Program, RGB,
    RGBA, RGBA16F, RGBA32F, SCISSOR_TEST, SRC_ALPHA, STREAM_DRAW, TEXTURE_2D_ARRAY,
    TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER, TEXTURE0, TRIANGLES, Texture, UNSIGNED_INT,
    VertexArray, ZERO,
};
use spitfire_core::{Triangle, VertexStream, VertexStreamRenderer};
use std::{borrow::Cow, collections::HashMap, marker::PhantomData, ops::Range};

#[derive(Clone, Copy)]
pub enum GlowVertexAttrib {
    Float { channels: u8, normalized: bool },
    Integer { channels: u8 },
}

impl GlowVertexAttrib {
    pub fn channels(&self) -> u8 {
        match self {
            Self::Float { channels, .. } => *channels,
            Self::Integer { channels } => *channels,
        }
    }
}

pub trait GlowVertexAttribs: Pod {
    const ATTRIBS: &'static [(&'static str, GlowVertexAttrib)];
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GlowUniformValue {
    F1(f32),
    F2([f32; 2]),
    F3([f32; 3]),
    F4([f32; 4]),
    M2([f32; 4]),
    M3([f32; 9]),
    M4([f32; 16]),
    I1(i32),
    I2([i32; 2]),
    I3([i32; 3]),
    I4([i32; 4]),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GlowBlending {
    #[default]
    None,
    Alpha,
    Multiply,
    Additive,
}

impl GlowBlending {
    pub fn into_gl(self) -> Option<(u32, u32)> {
        match self {
            Self::None => None,
            Self::Alpha => Some((SRC_ALPHA, ONE_MINUS_SRC_ALPHA)),
            Self::Multiply => Some((DST_COLOR, ZERO)),
            Self::Additive => Some((ONE, ONE)),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GlowTextureFiltering {
    #[default]
    Nearest,
    Linear,
}

impl GlowTextureFiltering {
    pub fn into_gl(self) -> (i32, i32) {
        match self {
            Self::Nearest => (NEAREST as _, NEAREST as _),
            Self::Linear => (LINEAR as _, LINEAR as _),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GlowTextureFormat {
    #[default]
    Rgba,
    Rgb,
    Monochromatic,
    Data16,
    Data32,
}

impl GlowTextureFormat {
    pub fn into_gl(self) -> u32 {
        match self {
            Self::Rgba => RGBA,
            Self::Rgb => RGB,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Monochromatic => glow::RED,
            #[cfg(target_arch = "wasm32")]
            Self::Monochromatic => glow::LUMINANCE,
            Self::Data16 => RGBA16F,
            Self::Data32 => RGBA32F,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GlowBatch {
    pub shader_program: Option<Program>,
    pub uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
    /// [(texture object, texture target, min filter, mag filter)?]
    pub textures: Vec<(Texture, u32, i32, i32)>,
    /// (source, destination)?
    pub blending: Option<(u32, u32)>,
    /// [x, y, width, height]?
    pub scissor: Option<[i32; 4]>,
    pub wireframe: bool,
}

impl GlowBatch {
    pub fn draw<V: GlowVertexAttribs>(&self, context: &Context, range: Range<usize>, prev: &Self) {
        unsafe {
            if let Some(program) = self.shader_program {
                let changed = prev
                    .shader_program
                    .map(|program_prev| program != program_prev)
                    .unwrap_or(true);
                context.use_program(Some(program));
                for (name, value) in &self.uniforms {
                    if changed
                        || prev
                            .uniforms
                            .get(name)
                            .map(|v| value != v)
                            .unwrap_or_default()
                    {
                        let location = context.get_uniform_location(program, name.as_ref());
                        if let Some(location) = location {
                            match value {
                                GlowUniformValue::F1(value) => {
                                    context.uniform_1_f32(Some(&location), *value);
                                }
                                GlowUniformValue::F2(value) => {
                                    context.uniform_2_f32_slice(Some(&location), value);
                                }
                                GlowUniformValue::F3(value) => {
                                    context.uniform_3_f32_slice(Some(&location), value);
                                }
                                GlowUniformValue::F4(value) => {
                                    context.uniform_4_f32_slice(Some(&location), value);
                                }
                                GlowUniformValue::M2(value) => {
                                    context.uniform_matrix_2_f32_slice(
                                        Some(&location),
                                        false,
                                        value,
                                    );
                                }
                                GlowUniformValue::M3(value) => {
                                    context.uniform_matrix_3_f32_slice(
                                        Some(&location),
                                        false,
                                        value,
                                    );
                                }
                                GlowUniformValue::M4(value) => {
                                    context.uniform_matrix_4_f32_slice(
                                        Some(&location),
                                        false,
                                        value,
                                    );
                                }
                                GlowUniformValue::I1(value) => {
                                    context.uniform_1_i32(Some(&location), *value);
                                }
                                GlowUniformValue::I2(value) => {
                                    context.uniform_2_i32_slice(Some(&location), value);
                                }
                                GlowUniformValue::I3(value) => {
                                    context.uniform_3_i32_slice(Some(&location), value);
                                }
                                GlowUniformValue::I4(value) => {
                                    context.uniform_4_i32_slice(Some(&location), value);
                                }
                            }
                        }
                    }
                }
            }
            for (index, data) in self.textures.iter().enumerate() {
                context.active_texture(TEXTURE0 + index as u32);
                let data_prev = prev.textures.get(index);
                if data_prev.map(|prev| prev != data).unwrap_or(true) {
                    let (texture, target, min_filter, mag_filter) = data;
                    context.bind_texture(*target, Some(*texture));
                    context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_MIN_FILTER, *min_filter);
                    context.tex_parameter_i32(TEXTURE_2D_ARRAY, TEXTURE_MAG_FILTER, *mag_filter);
                }
            }
            if self.blending != prev.blending {
                if let Some((source, destination)) = self.blending {
                    context.enable(BLEND);
                    context.blend_func(source, destination);
                } else {
                    context.disable(BLEND);
                }
            }
            if self.scissor != prev.scissor {
                if let Some([x, y, w, h]) = self.scissor {
                    context.enable(SCISSOR_TEST);
                    context.scissor(x, y, w, h);
                } else {
                    context.disable(SCISSOR_TEST);
                }
            }
            if self.wireframe != prev.wireframe {
                if self.wireframe {
                    context.polygon_mode(FRONT_AND_BACK, LINE);
                } else {
                    context.polygon_mode(FRONT_AND_BACK, FILL);
                }
            }
            context.draw_elements(
                TRIANGLES,
                range.len() as i32 * 3,
                UNSIGNED_INT,
                (range.start * std::mem::size_of::<u32>() * 3) as i32,
            );
        }
    }
}

#[derive(Copy, Clone)]
struct GlowMesh {
    vertex_array: VertexArray,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl GlowMesh {
    fn new(context: &Context) -> Result<Self, String> {
        unsafe {
            Ok(GlowMesh {
                vertex_array: context.create_vertex_array()?,
                vertex_buffer: context.create_buffer()?,
                index_buffer: context.create_buffer()?,
            })
        }
    }

    fn dispose(self, context: &Context) {
        unsafe {
            context.delete_vertex_array(self.vertex_array);
            context.delete_buffer(self.vertex_buffer);
            context.delete_buffer(self.index_buffer);
        }
    }

    fn upload<V: GlowVertexAttribs>(
        &self,
        context: &Context,
        vertices: &[V],
        triangles: &[Triangle],
    ) {
        unsafe {
            context.bind_vertex_array(Some(self.vertex_array));
            context.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));
            context.buffer_data_u8_slice(ARRAY_BUFFER, cast_slice(vertices), STREAM_DRAW);
            context.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
            context.buffer_data_u8_slice(ELEMENT_ARRAY_BUFFER, cast_slice(triangles), STREAM_DRAW);
            let mut offset = 0;
            let stride = V::ATTRIBS
                .iter()
                .map(|(_, info)| info.channels() * 4)
                .sum::<u8>();
            for (location, (_, info)) in V::ATTRIBS.iter().enumerate() {
                match info {
                    GlowVertexAttrib::Float {
                        channels,
                        normalized,
                    } => {
                        context.vertex_attrib_pointer_f32(
                            location as _,
                            *channels as _,
                            FLOAT,
                            *normalized,
                            stride as _,
                            offset as _,
                        );
                    }
                    GlowVertexAttrib::Integer { channels } => {
                        context.vertex_attrib_pointer_i32(
                            location as _,
                            *channels as _,
                            INT,
                            stride as _,
                            offset as _,
                        );
                    }
                }
                context.enable_vertex_attrib_array(location as _);
                offset += info.channels() * 4;
            }
        }
    }
}

#[derive(Default)]
pub struct GlowState {
    mesh: Option<GlowMesh>,
}

impl Drop for GlowState {
    fn drop(&mut self) {
        if self.mesh.is_some() {
            panic!("Mesh was not disposed!");
        }
    }
}

impl GlowState {
    pub fn dispose(&mut self, context: &Context) {
        if let Some(mesh) = self.mesh.take() {
            mesh.dispose(context)
        }
    }

    fn mesh(&mut self, context: &Context) -> Result<GlowMesh, String> {
        if let Some(mesh) = self.mesh.as_ref().copied() {
            Ok(mesh)
        } else {
            self.mesh = Some(GlowMesh::new(context)?);
            Ok(self.mesh.unwrap())
        }
    }
}

pub struct GlowRenderer<'a, B: Into<GlowBatch>> {
    context: &'a Context,
    state: &'a mut GlowState,
    _phantom: PhantomData<fn() -> B>,
}

impl<'a, B> GlowRenderer<'a, B>
where
    B: Into<GlowBatch>,
{
    pub fn new(context: &'a Context, state: &'a mut GlowState) -> Self {
        Self {
            context,
            state,
            _phantom: Default::default(),
        }
    }
}

impl<V, B> VertexStreamRenderer<V, B> for GlowRenderer<'_, B>
where
    V: GlowVertexAttribs,
    B: Into<GlowBatch> + Default + Clone,
{
    type Error = String;

    fn render(&mut self, stream: &mut VertexStream<V, B>) -> Result<(), Self::Error> {
        let mesh = self.state.mesh(self.context)?;
        mesh.upload(self.context, stream.vertices(), stream.triangles());
        let mut prev = GlowBatch::default();
        for (batch, range) in stream.batches().iter().cloned() {
            let batch = batch.into();
            batch.draw::<V>(self.context, range, &prev);
            prev = batch;
        }
        Ok(())
    }
}
