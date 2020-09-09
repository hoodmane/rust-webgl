use crate::log::{log_str };
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::glyph_shader::GlyphShader;
use crate::line_shader::LineShader;
use crate::arc_shader::ArcShader;
use crate::font::{Glyph, Font};
use crate::webgl_wrapper::WebGlWrapper;



use wasm_bindgen::prelude::*;
use web_sys::{WebGlTexture, WebGl2RenderingContext};

#[wasm_bindgen]
pub struct Context {
    webgl : WebGlWrapper,
    transform : Transform,


    glyph_shader : GlyphShader,
    
    arc_shader : ArcShader,
    line_shader : LineShader,
    width : i32,
    height : i32,
    density : f64
}

impl Context {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let line_shader = LineShader::new(webgl.clone())?;
        let arc_shader = ArcShader::new(webgl.clone())?;
        let width = webgl.width();
        let height = webgl.height();
        let density = WebGlWrapper::pixel_density();
        Ok(Self {
            webgl,
            transform : Transform::new(),
            glyph_shader,


            line_shader,
            arc_shader,
            width,
            height,
            density
        })
    }

    pub fn webgl(&self) -> &WebGlWrapper {
        &self.webgl
    }

    pub fn resize(&mut self, width : i32, height : i32, density : f64) -> Result<(), JsValue> {
        self.width = width;
        self.height = height;
        self.density = density;
        let canvas = self.webgl.canvas()?;
        canvas.style().set_property("width", &format!("{}px", self.width))?;
        canvas.style().set_property("height", &format!("{}px", self.height))?;
        canvas.set_width(self.pixel_width() as u32);
        canvas.set_height(self.pixel_height() as u32);
        self.glyph_shader.resize_buffer(self.pixel_width(), self.pixel_height())?;
        Ok(())
    }

    pub fn pixel_width(&self) -> i32 {
        (self.width as f64 * self.density) as i32
    }

    pub fn pixel_height(&self) -> i32 {
        (self.height as f64 * self.density) as i32
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn set_transform(&mut self, transform : Transform) {
        self.transform = transform;
    }

    pub fn start_frame(&mut self) -> Result<(), JsValue> {
        self.resize(
            self.width,
            self.height,
            web_sys::window().unwrap().device_pixel_ratio()
        )?;
        let mut transform = Transform::new();
        transform.translate(-1.0, 1.0);
        transform.scale(2.0/ (self.width as f32), -2.0/(self.height as f32));
        self.transform = transform;
        self.webgl.render_to_canvas();
        self.webgl.viewport(0, 0, self.pixel_width(), self.pixel_height());
        self.webgl.disable(WebGl2RenderingContext::BLEND);
        self.webgl.clear_color(0.5, 0.5, 0.5, 1.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        Ok(())
    }
}

#[wasm_bindgen]
impl Context {
    pub fn start_frame_js(&mut self) -> Result<(), JsValue> {
        self.start_frame()
    }

    // pub fn draw_letter(&mut self, font : &Font, codepoint : u16,  x : f32, y : f32, scale : f32) -> Result<(), JsValue> {
    //     let glyph = font.glyph(codepoint)?.path();
    //     self.glyph_shader.draw(glyph, self.transform, Vec2::new(x, y), scale)?;
    //     Ok(())
    // }

    pub fn draw_line(&self, p : Vec2, q : Vec2, color : Vec4, thickness : f32) -> Result<(), JsValue> {
        let mut line_shader = LineShader::new(self.webgl.clone())?;
        line_shader.add_line(p, q, color, thickness)?;
        let transform = self.transform();
        log_str(&format!("p : {:?}", transform.transform_point(p)));
        log_str(&format!("q : {:?}", transform.transform_point(q)));
        self.webgl.copy_blend_mode();
        self.webgl.render_to_canvas();
        line_shader.draw(self.transform())?;
        Ok(())
    }

    pub fn draw_arc(&mut self,p : Vec2, q : Vec2,  theta : f32, color : Vec4, thickness : f32) -> Result<(), JsValue> {
        self.arc_shader.draw_arc(
            self.transform(),
            p, q, 
            theta,
            color,
            thickness
        )?;
        Ok(())
    }
}