use crate::{context::DrawContext, draw_buffer::DrawBuffer, utils::Vertex};
use spitfire_glow::{
    graphics::{Graphics, GraphicsBatch, GraphicsTarget, Mesh},
    renderer::GlowUniformValue,
};
use std::{borrow::Cow, collections::HashMap, ops::Range};

pub struct StaticMesh {
    mesh: Mesh,
    batches: Vec<(GraphicsBatch, Range<usize>)>,
    uniforms: HashMap<Cow<'static, str>, GlowUniformValue>,
}

impl StaticMesh {
    pub fn record(
        context: &mut DrawContext,
        graphics: &Graphics<Vertex>,
        mut f: impl FnMut(&mut DrawContext, &mut DrawBuffer),
    ) -> Result<Self, String> {
        let mut buffer = DrawBuffer::new(graphics);
        f(context, &mut buffer);
        Self::from_buffer(&mut buffer, graphics)
    }

    pub fn from_buffer(
        buffer: &mut DrawBuffer,
        graphics: &Graphics<Vertex>,
    ) -> Result<Self, String> {
        let stream = &mut buffer.state.stream;
        stream.batch_end();
        let mesh = graphics.mesh(stream.vertices(), stream.triangles())?;
        Ok(Self {
            mesh,
            batches: stream.batches().to_vec(),
            uniforms: Default::default(),
        })
    }

    pub fn set_uniform(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        value: GlowUniformValue,
    ) -> &mut Self {
        self.uniforms.insert(name.into(), value);
        self
    }

    pub fn uniforms(&self) -> &HashMap<Cow<'static, str>, GlowUniformValue> {
        &self.uniforms
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn batches(&self) -> &[(GraphicsBatch, Range<usize>)] {
        &self.batches
    }

    pub fn triangles(&self) -> usize {
        self.mesh.triangles()
    }

    pub fn draw(&self, graphics: &mut dyn GraphicsTarget<Vertex>) {
        let matrix = self.matrix(graphics);
        for index in 0..self.batches.len() {
            let range = self.batches[index].1.clone();
            self.push(graphics, index, range, Some(matrix));
        }
    }

    pub fn draw_recorded(&self, graphics: &mut dyn GraphicsTarget<Vertex>) {
        for index in 0..self.batches.len() {
            let range = self.batches[index].1.clone();
            self.push(graphics, index, range, None);
        }
    }

    pub fn draw_ranges<I>(&self, graphics: &mut dyn GraphicsTarget<Vertex>, ranges: I)
    where
        I: IntoIterator<Item = Range<usize>> + Clone,
    {
        let matrix = self.matrix(graphics);
        for index in 0..self.batches.len() {
            let batch = self.batches[index].1.clone();
            for range in ranges.clone() {
                let start = range.start.max(batch.start);
                let end = range.end.min(batch.end);
                if start < end {
                    self.push(graphics, index, start..end, Some(matrix));
                }
            }
        }
    }

    fn matrix(&self, graphics: &dyn GraphicsTarget<Vertex>) -> [f32; 16] {
        graphics.state().main_camera.world_matrix().into_col_array()
    }

    fn push(
        &self,
        graphics: &mut dyn GraphicsTarget<Vertex>,
        index: usize,
        range: Range<usize>,
        matrix: Option<[f32; 16]>,
    ) {
        let mut batch = self.batches[index].0.clone();
        if let Some(matrix) = matrix {
            batch
                .uniforms
                .insert("u_projection_view".into(), GlowUniformValue::M4(matrix));
        }
        for (name, value) in &self.uniforms {
            batch.uniforms.insert(name.clone(), *value);
        }
        batch.mesh = Some((self.mesh.clone(), range));
        graphics.state_mut().stream.batch(batch);
    }
}
