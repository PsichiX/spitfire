pub use spitfire_core as core;
#[cfg(feature = "draw")]
pub use spitfire_draw as draw;
#[cfg(feature = "fontdue")]
pub use spitfire_fontdue as fontdue;
#[cfg(feature = "glow")]
pub use spitfire_glow as glow;
#[cfg(feature = "input")]
pub use spitfire_input as input;

pub mod prelude {
    pub use spitfire_core::*;
    #[cfg(feature = "draw")]
    pub use spitfire_draw::prelude::*;
    #[cfg(feature = "fontdue")]
    pub use spitfire_fontdue::*;
    #[cfg(feature = "glow")]
    pub use spitfire_glow::prelude::*;
    #[cfg(feature = "input")]
    pub use spitfire_input::*;
}
