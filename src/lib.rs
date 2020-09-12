#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod log;
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

use crate::log::*;
use crate::vector::*;

use crate::webgl_wrapper::WebGlWrapper;

use crate::canvas::Canvas;
use crate::matrix::Transform;


use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;


use web_sys::{WebGl2RenderingContext, HtmlCanvasElement};

#[wasm_bindgen]
pub fn get_rust_canvas(context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
    // let webgl = WebGlWrapper::new(canvas.get_context("webgl2")?.unwrap().dyn_into()?);
    // let document = web_sys::window().unwrap().document().unwrap();
    // let canvas_element : HtmlCanvasElement = 
    //     document.query_selector(selector)?
    //     .ok_or(JsValue::from_str(&format!("No element found with selector \"{}\".", selector)))?
    //     .dyn_into()
    //     .map_err(|_e| JsValue::from_str(&format!("Element found with selector \"{}\" is not a canvas element.", selector)))?;
    Ok(Canvas::new(context)?)
}