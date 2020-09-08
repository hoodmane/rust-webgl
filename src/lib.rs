#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod log;
mod rect;
mod font;
mod matrix;
mod vector;

mod webgl_wrapper;
mod context;
mod canvas;
mod shader;
mod stencil_shader;
mod arc_shader;
mod cubic_shader;
mod line_shader;
mod glyph_shader;


pub use font::read_font;

use crate::log::*;
use crate::vector::*;

use crate::webgl_wrapper::WebGlWrapper;
use crate::cubic_shader::CubicBezierShader;
use crate::arc_shader::ArcShader;
use crate::line_shader::LineShader;
use crate::context::Context;
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
    let webgl_wrapper = WebGlWrapper::new(
        context.clone() //canvas_element.get_context_with_context_options("webgl2", options)?.unwrap().dyn_into()?
    );
    Ok(Canvas::new(webgl_wrapper)?)
}

#[wasm_bindgen]
pub fn get_rust_context(context : &WebGl2RenderingContext) -> Result<Context, JsValue> {
    // let webgl = WebGlWrapper::new(canvas.get_context("webgl2")?.unwrap().dyn_into()?);
    // let document = web_sys::window().unwrap().document().unwrap();
    // let canvas_element : HtmlCanvasElement = 
    //     document.query_selector(selector)?
    //     .ok_or(JsValue::from_str(&format!("No element found with selector \"{}\".", selector)))?
    //     .dyn_into()
    //     .map_err(|_e| JsValue::from_str(&format!("Element found with selector \"{}\" is not a canvas element.", selector)))?;
    let webgl_wrapper = WebGlWrapper::new(
        context.clone() //canvas_element.get_context_with_context_options("webgl2", options)?.unwrap().dyn_into()?
    );
    Ok(Context::new(webgl_wrapper)?)
}