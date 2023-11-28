#![cfg(target_arch = "wasm32")]

mod game;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn main() {
    game::main();
}
