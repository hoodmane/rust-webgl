#![allow(dead_code)]

mod log;
mod rect;
mod font;
mod matrix;
mod vector;
mod context;
mod shader;
mod arc_shader;
mod cubic_shader;
mod line_shader;
mod glyph_shader;


pub use font::read_font;

use crate::log::*;
// use crate::webgl::*;
use crate::vector::*;

use crate::cubic_shader::CubicBezierShader;
use crate::arc_shader::ArcShader;
use crate::line_shader::LineShader;
use crate::glyph_shader::{GlyphShader, TextShader};
use crate::context::Context;
use crate::matrix::Transform;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;


use web_sys::WebGlRenderingContext;


#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> { 
    let context = get_webgl_context()?;
    // gl.viewport(0, 0, glCanvas.width, glCanvas.height);
    context.clear_color(0.0, 0.0, 0.0, 1.0);
    context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    // draw_letter(97).await?;

    Ok(()) 
}

#[wasm_bindgen]
pub fn get_context() -> Result<Context, JsValue> {
    let context = Context::new(get_webgl_context()?)?;
    Ok(context)
}


#[wasm_bindgen]
pub async fn draw_letter(idx : u16) -> Result<(), JsValue> { 
    // let context = get_webgl_context()?;
    // // gl.viewport(0, 0, glCanvas.width, glCanvas.height);
    // let font = font::read_font().await?;
    // context.clear_color(0.0, 0.0, 0.0, 1.0);
    // context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

    // let width = context.drawing_buffer_width();
    // let height = context.drawing_buffer_height();
    // let texture = context.create_texture();
    // context.bind_texture(WebGlRenderingContext::TEXTURE_2D, texture.as_ref());
    // context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_MAG_FILTER, WebGlRenderingContext::NEAREST as i32);
    // context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_MIN_FILTER, WebGlRenderingContext::NEAREST as i32);
    // context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_WRAP_S, WebGlRenderingContext::CLAMP_TO_EDGE as i32);
    // context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_WRAP_T, WebGlRenderingContext::CLAMP_TO_EDGE as i32);
    // context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
    //     WebGlRenderingContext::TEXTURE_2D, // target
    //     0, // level
    //     WebGlRenderingContext::RGBA as i32, //internal format, specifies the color components in the texture.
    //     width, height, 
    //     0, // border "Must be 0."
    //     WebGlRenderingContext::RGBA, // format, must be same as internal format (but apparently this time it's a u32????)
    //     WebGlRenderingContext::UNSIGNED_BYTE, // type: specifying the data type of the texel data
    //     None // u8 array source
    // )?;

    // context.enable(WebGlRenderingContext::BLEND);
    // context.blend_func(WebGlRenderingContext::ONE, WebGlRenderingContext::ONE);





    // // Set render target
    // let framebuffer = context.create_framebuffer();
    // context.bind_framebuffer(WebGlRenderingContext::FRAMEBUFFER, framebuffer.as_ref());
    // context.framebuffer_texture_2d(
    //     WebGlRenderingContext::FRAMEBUFFER, 
    //     WebGlRenderingContext::COLOR_ATTACHMENT0, 
    //     WebGlRenderingContext::TEXTURE_2D, 
    //     texture.as_ref(), 0
    // );
    
    // context.clear_color(0.0, 0.0, 0.0, 0.0);
    // context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);



    // let glyph_shader = GlyphShader::new(context.clone())?;
    // let a_glyph = font.glyph(idx)?;
    // glyph_shader.draw(Transform::new(), a_glyph)?;

    
    // context.bind_framebuffer(WebGlRenderingContext::FRAMEBUFFER, None);


    // context.disable(WebGlRenderingContext::BLEND);

    // context.active_texture(WebGlRenderingContext::TEXTURE0);
    // context.bind_texture(WebGlRenderingContext::TEXTURE_2D, texture.as_ref());


    // // context.get_uniform_location
    // // context.uniform1i(programInfo.uniformLocations.uSampler, 0);
    // let text_shader = TextShader::new(context.clone())?;
    // text_shader.draw(&a_glyph, 0.0, 0.0, 1.0)?;

    Ok(()) 
}




fn get_webgl_context() -> Result<WebGlRenderingContext, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;
    Ok(canvas.get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?)
}




#[wasm_bindgen]
pub struct WrappedCubicBezierShader {
    // context : WebGlRenderingContext,
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

    pub fn draw(&self) -> Result<(), JsValue> {
        log_str("Drawing...");
        log_1(&self.cubic_shader.shader.context);
        // self.cubic_shader.shader.context.clear_color(0.8, 0.9, 1.0, 1.0);
        // self.cubic_shader.shader.context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
        self.cubic_shader.draw()?;
        log_str("Drawn");
        Ok(())
    }
}


#[wasm_bindgen]
pub struct WrappedArcShader {
    // context : WebGlRenderingContext,
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

    pub fn draw(&self) -> Result<(), JsValue> {
        self.arc_shader.draw()?;
        Ok(())
    }
}



// #[wasm_bindgen]
// pub struct WrappedLineShader {
//     // context : WebGlRenderingContext,
//     line_shader : LineShader
// }

// #[wasm_bindgen]
// impl WrappedLineShader {
//     pub fn add_line(&mut self, p0 : f32, p1 : f32, q0 : f32, q1 : f32) {
//         self.line_shader.add_line(
//             Vec2::new(p0, p1), Vec2::new(q0, q1)
//         );
//     }

//     pub fn draw(&self) -> Result<(), JsValue> {
//         self.line_shader.draw()?;
//         Ok(())
//     }
// }




#[wasm_bindgen]
pub fn get_cubic_shader() -> Result<WrappedCubicBezierShader, JsValue> {
    let context = get_webgl_context()?;
    context.get_extension(&"OES_standard_derivatives")?;
    let cubic_shader = CubicBezierShader::new(context)?;
    Ok(WrappedCubicBezierShader { cubic_shader })
}

#[wasm_bindgen]
pub fn get_arc_shader() -> Result<WrappedArcShader, JsValue> {
    let context = get_webgl_context()?;
    let arc_shader = ArcShader::new(context)?;
    Ok(WrappedArcShader { arc_shader })
}


// #[wasm_bindgen]
// pub fn get_line_shader() -> Result<WrappedLineShader, JsValue> {
//     let context = get_webgl_context()?;
//     let line_shader = LineShader::new(context)?;
//     Ok(WrappedLineShader { line_shader })
// }

