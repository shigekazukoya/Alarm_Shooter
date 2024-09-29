use wasm_bindgen::prelude::*;

mod game;
mod app;
mod utils;

#[wasm_bindgen]
pub fn start_game() {
    app::start_game();
}

#[wasm_bindgen]
pub fn reset_game() {
    app::reset_game();
}
