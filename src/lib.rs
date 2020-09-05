#![allow(dead_code)]

mod log;
mod rect;
mod font;
mod matrix;
mod vector;

mod webgl_wrapper;
mod context;
mod shader;
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
use crate::matrix::Transform;


use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;


use web_sys::WebGl2RenderingContext;


#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> { 
    let context = get_webgl()?;
    // gl.viewport(0, 0, glCanvas.width, glCanvas.height);
    // context.webgl.clear_color(0.0, 0.0, 0.0, 1.0);
    context.clear();
    // draw_letter(97).await?;

    Ok(()) 
}

#[wasm_bindgen]
pub fn get_context() -> Result<Context, JsValue> {
    let context = Context::new(get_webgl()?)?;
    Ok(context)
}


fn get_webgl() -> Result<WebGlWrapper, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    Ok(WebGlWrapper::new(
        canvas.get_context("webgl2")?.unwrap().dyn_into()?
    ))
}




#[wasm_bindgen]
pub struct WrappedCubicBezierShader {
    // context : WebGl2RenderingContext,
    cubic_shader : CubicBezierShader
}

#[wasm_bindgen]
impl WrappedCubicBezierShader {
    pub fn add_cubic_bezier(&mut self, p00 : f32, p01 : f32, p10 : f32, p11 : f32, p20 : f32, p21 : f32, p30 : f32, p31 : f32 ){
        self.cubic_shader.add_cubic_bezier(
            Vec2::new(p00, p01), Vec2::new(p10, p11), 
            Vec2::new(p20, p21), Vec2::new(p30, p31)
        );
    }

    pub fn draw(&mut self) -> Result<(), JsValue> {
        self.cubic_shader.draw()?;
        Ok(())
    }
}


#[wasm_bindgen]
pub struct WrappedArcShader {
    // context : WebGl2RenderingContext,
    arc_shader : ArcShader
}

#[wasm_bindgen]
impl WrappedArcShader {
    pub fn add_arc(&mut self, p0 : f32, p1 : f32, q0 : f32, q1 : f32, theta : f32) -> Result<(), JsValue>{
        self.arc_shader.add_arc(
            Vec2::new(p0, p1), Vec2::new(q0, q1), theta
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), JsValue> {
        self.arc_shader.draw()?;
        Ok(())
    }
}



#[wasm_bindgen]
pub struct WrappedLineShader {
    // context : WebGl2RenderingContext,
    line_shader : LineShader
}

#[wasm_bindgen]
impl WrappedLineShader {
    pub fn add_line(&mut self, p0 : f32, p1 : f32, q0 : f32, q1 : f32, r : f32, g : f32, b : f32, thickness : f32) -> Result<(), JsValue> {
        self.line_shader.add_line(
            Vec2::new(p0, p1), Vec2::new(q0, q1), Vec4::new(r, g, b, 1.0), thickness
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), JsValue> {
        self.line_shader.draw(Transform::new())?;
        Ok(())
    }
}




#[wasm_bindgen]
pub fn get_cubic_shader() -> Result<WrappedCubicBezierShader, JsValue> {
    let context = get_webgl()?;
    let cubic_shader = CubicBezierShader::new(context)?;
    Ok(WrappedCubicBezierShader { cubic_shader })
}

#[wasm_bindgen]
pub fn get_arc_shader() -> Result<WrappedArcShader, JsValue> {
    let context = get_webgl()?;
    let arc_shader = ArcShader::new(context)?;
    Ok(WrappedArcShader { arc_shader })
}


#[wasm_bindgen]
pub fn get_line_shader() -> Result<WrappedLineShader, JsValue> {
    let context = get_webgl()?;
    let line_shader = LineShader::new(context)?;
    Ok(WrappedLineShader { line_shader })
}

