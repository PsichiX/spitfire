use bytemuck::Pod;
use spitfire_core::{Batch, Renderer, VertexStream};

pub struct GlowBatch {}

impl Batch for GlowBatch {
    type Error = ();

    fn begin(self) -> Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<(), Self::Error> {
        todo!()
    }
}

pub struct GlowRenderer {}

impl<V: Pod> Renderer<V, GlowBatch> for GlowRenderer {
    type Error = ();

    fn render(&mut self, stream: &mut VertexStream<V, GlowBatch>) -> Result<(), Self::Error> {
        todo!()
    }
}
