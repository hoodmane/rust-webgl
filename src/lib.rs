#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod console_log;
mod rect;
mod font;

mod convex_hull;
mod node;

mod vector;

mod path_segment;
mod path;
mod arrow;

mod webgl_wrapper;
mod shader;
mod canvas;


mod glyph;



pub use font::read_font;

// #[wasm_bindgen]
// pub fn fonterator_test() {
//     crate::fonterator_test::test();
// }

use crate::canvas::Canvas;


use wasm_bindgen::prelude::*;


use web_sys::{WebGl2RenderingContext};

#[wasm_bindgen]
pub fn get_rust_canvas(context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
    Ok(Canvas::new(context)?)
}