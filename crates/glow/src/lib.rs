pub mod app;
pub mod graphics;
pub mod renderer;

#[cfg(target_arch = "wasm32")]
pub mod log {
    pub mod __internal__ {
        use wasm_bindgen::prelude::*;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = console)]
            pub fn log(s: &str);
        }
    }

    #[macro_export]
    macro_rules! console_log {
        ($($t:tt)*) => ($crate::log::__internal__::log(&format_args!($($t)*).to_string()))
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub mod log {
    #[macro_export]
    macro_rules! console_log {
        ($($t:tt)*) => (println!("{}", &format_args!($($t)*).to_string()))
    }
}

pub mod prelude {
    pub use crate::{app::*, graphics::*, log::*, renderer::*};
}
