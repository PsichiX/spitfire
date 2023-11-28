#![cfg(not(target_arch = "wasm32"))]

mod game;

fn main() {
    game::main();
}
