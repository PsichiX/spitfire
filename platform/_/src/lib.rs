pub use spitfire_core as core;

pub mod backends {
    #[cfg(feature = "glow")]
    pub use spitfire_glow as glow;
}

pub mod prelude {
    pub use spitfire_core::*;
    #[cfg(feature = "glow")]
    pub use spitfire_glow::prelude::*;
}
