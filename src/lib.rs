#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod console_log;
mod rect;
mod font;
mod vector;

mod arrow;

mod webgl_wrapper;
mod canvas;


mod convex_hull;
mod shader;

mod path_segment;
mod path;
mod tesselate;

pub use font::read_font;


use crate::canvas::Canvas;


use wasm_bindgen::prelude::*;


use web_sys::{WebGl2RenderingContext};

#[wasm_bindgen]
pub fn get_rust_canvas(context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
    Ok(Canvas::new(context)?)
}




// use std::f32::consts::PI;
// #[wasm_bindgen]
// pub fn test_lyon() -> Result<(), JsValue> {
//     let mut path_builder = Path::builder();
//     path_builder.move_to(point(0.0, 0.0));
//     path_builder.line_to(point(100.0, 200.0));
//     path_builder.line_to(point(200.0, 0.0));
//     path_builder.line_to(point(100.0, 100.0));
//     path_builder.close();
//     let path = path_builder.build();
    
//     // Create the destination vertex and index buffers.
//     let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    
//     {
//         // Create the destination vertex and index buffers.
//         let mut vertex_builder = simple_builder(&mut buffers);
    
//         // Create the tessellator.
//         let mut tessellator = StrokeTessellator::new();
    
//         // Compute the tessellation.
//         tessellator.tessellate(
//             &path,
//             &StrokeOptions::default(),
//             &mut vertex_builder
//         ).map_err(convert_error)?;
//     }
//     log!("buffers : {:?}", buffers);


//     // let mut path_builder = Path::builder();
//     // path_builder.move_to(point(0.0, 0.0));
//     // path_builder.line_to(point(1.0, 2.0));
//     // path_builder.line_to(point(2.0, 0.0));
//     // path_builder.line_to(point(1.0, 1.0));
//     // path_builder.close();
//     // lyon_tesselate::tesselate_path(&path)?;
//     Ok(())
// }

use std::f32::consts::PI;
#[wasm_bindgen]
pub fn test_lyon2() -> Result<(), JsValue> {
    let mut path = crate::path::Path::new((0.0, 0.0));
    path.arc_to((100.0, 100.0), PI/180.0 * 15.0);
    path.line_to((200.0, 0.0));
    path.cubic_curve_to((250.0, 100.0), (550.0, 200.0), (300.0, 200.0));
    crate::tesselate::tesselate_path(&path)?;
    Ok(())
}