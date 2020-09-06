use crate::log::{log_str };
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::glyph_shader::{GlyphShader, TextShader};
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
    text_shader : TextShader,
    glyph_buffer : WebGlTexture,
    
    arc_shader : ArcShader,
    line_shader : LineShader,
    width : i32,
    height : i32,
    density : f64
}

impl Context {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let glyph_buffer = webgl.inner.create_texture().unwrap();
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let text_shader = TextShader::new(webgl.clone())?;
        let line_shader = LineShader::new(webgl.clone())?;
        let arc_shader = ArcShader::new(webgl.clone())?;
        let width = webgl.width();
        let height = webgl.height();
        let density = WebGlWrapper::density();
        Ok(Self {
            webgl,
            transform : Transform::new(),
            glyph_shader,
            text_shader,
            glyph_buffer,


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
        self.webgl.inner.viewport(0, 0, self.pixel_width(), self.pixel_height());
        self.webgl.inner.disable(WebGl2RenderingContext::BLEND);
        self.webgl.clear();
        self.glyph_buffer = self.webgl.create_texture(self.pixel_width(), self.pixel_height(), WebGl2RenderingContext::RGBA8)?;
        Ok(())
    }

    
    pub fn draw_letter_inner(&mut self, glyph : &Glyph, x : f32, y : f32, scale : f32) -> Result<(), JsValue> {
        self.webgl.add_blend_mode();
        self.webgl.render_to_texture(&self.glyph_buffer);
        self.webgl.inner.viewport(0, 0, self.pixel_width(), self.pixel_height());
        self.webgl.inner.clear_color(0.0, 0.0, 0.0, 1.0);
        self.webgl.inner.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT); 

        let mut transform = self.transform();
        transform.translate(x, y);
        self.glyph_shader.draw(transform, glyph, scale, self.density)?;

        transform.scale(scale, scale);
        self.webgl.inner.blend_func(WebGl2RenderingContext::ZERO, WebGl2RenderingContext::SRC_COLOR);
        self.webgl.render_to_canvas();
        
        self.webgl.inner.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.webgl.inner.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.glyph_buffer));
        
        self.text_shader.draw(transform, glyph)?;
        Ok(())
    }
}

#[wasm_bindgen]
impl Context {
    pub fn start_frame_js(&mut self) -> Result<(), JsValue> {
        self.start_frame()
    }

    pub fn draw_letter(&mut self, font : &Font, codepoint : u16,  x : f32, y : f32, scale : f32) -> Result<(), JsValue> {
        self.draw_letter_inner(font.glyph(codepoint)?, x, y, scale)?;
        Ok(())
    }

    pub fn draw_line(&self, px : f32, py : f32, qx : f32, qy : f32, thickness : f32, r : f32, g : f32, b : f32) -> Result<(), JsValue> {
        let mut line_shader = LineShader::new(self.webgl.clone())?;
        let p = Vec2::new(px, py);
        let q = Vec2::new(qx, qy);
        let r = if r.is_finite() { r } else { 0.0 };
        let g = if g.is_finite() { g } else { 0.0 };
        let b = if b.is_finite() { b } else { 0.0 };
        line_shader.add_line(p, q, Vec4::new(r, g, b, 1.0), thickness)?;
        let transform = self.transform();
        log_str(&format!("p : {:?}", transform.transform_point(p)));
        log_str(&format!("q : {:?}", transform.transform_point(q)));
        self.webgl.copy_blend_mode();
        self.webgl.render_to_canvas();
        line_shader.draw(self.transform())?;
        Ok(())
    }

    pub fn draw_arc(&mut self, p0 : f32, p1 : f32, q0 : f32, q1 : f32, theta : f32, r : f32, g : f32, b : f32, thickness : f32) -> Result<(), JsValue> {
        self.arc_shader.draw_arc(
            self.transform(),
            Vec2::new(p0, p1), Vec2::new(q0, q1), 
            theta,
            Vec4::new(r, g, b, 1.0),
            thickness
        )?;
        Ok(())
    }

    pub fn translate(&mut self, x : f32, y : f32) {
        let mut transform = self.transform();
        transform.translate(x, y);
        self.set_transform(transform);
    }

    pub fn scale(&mut self, x : f32, y : f32) {
        let mut transform = self.transform();
        transform.scale(x, y);
        self.set_transform(transform);
    }
}