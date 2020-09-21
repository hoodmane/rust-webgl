#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod console_log;
mod rect;

mod convex_hull;

mod vector;

mod path_segment;
mod path;
mod arrow;

mod webgl_wrapper;
mod shader;
mod canvas;


mod glyph;
mod edge;

use crate::canvas::Canvas;


use wasm_bindgen::prelude::*;


use web_sys::{WebGl2RenderingContext};

#[wasm_bindgen]
pub fn get_rust_canvas(context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
    Ok(Canvas::new(context)?)
}

#[wasm_bindgen]
pub fn rust_main() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    // #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
