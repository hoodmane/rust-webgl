#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod console_log;
mod rect;
mod font;
mod matrix;
mod vector;

mod poly_line;
mod arrow;

mod webgl_wrapper;
mod canvas;


mod convex_hull;
mod shader;

pub use font::read_font;


use crate::canvas::Canvas;


use wasm_bindgen::prelude::*;


use web_sys::{WebGl2RenderingContext};

#[wasm_bindgen]
pub fn get_rust_canvas(context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
    Ok(Canvas::new(context)?)
}