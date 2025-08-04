use crate::utils::Vertex;
use spitfire_glow::graphics::{GraphicsState, GraphicsTarget};

#[derive(Default, Clone)]
pub struct DrawBuffer {
    pub state: GraphicsState<Vertex>,
}

impl DrawBuffer {
    pub fn new(source: &dyn GraphicsTarget<Vertex>) -> Self {
        Self {
            state: source.state().fork(),
        }
    }

    pub fn submit(&mut self, target: &mut dyn GraphicsTarget<Vertex>) {
        target.state_mut().stream.append(&mut self.state.stream);
    }

    pub fn submit_cloned(&self, target: &mut dyn GraphicsTarget<Vertex>) {
        target.state_mut().stream.append_cloned(&self.state.stream);
    }
}

impl GraphicsTarget<Vertex> for DrawBuffer {
    fn state(&self) -> &GraphicsState<Vertex> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut GraphicsState<Vertex> {
        &mut self.state
    }
}
