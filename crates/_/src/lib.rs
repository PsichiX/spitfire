pub use spitfire_core as core;

#[cfg(feature = "glow")]
pub use spitfire_glow as glow;

#[cfg(feature = "fontdue")]
pub use spitfire_fontdue as fontdue;

pub mod prelude {
    pub use spitfire_core::*;
    #[cfg(feature = "fontdue")]
    pub use spitfire_fontdue::*;
    #[cfg(feature = "glow")]
    pub use spitfire_glow::prelude::*;
}
