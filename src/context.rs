use crate::log::log_str;
use crate::vector::Vec2;
use crate::matrix::Transform;
use crate::glyph_shader::{GlyphShader, TextShader};
use crate::line_shader::LineShader;
use crate::font::{Glyph, Font};

use wasm_bindgen::prelude::*;
use web_sys::{WebGlTexture, WebGlRenderingContext};

#[wasm_bindgen]
pub struct Context {
    webgl_context : WebGlRenderingContext,
    transform : Transform,
    glyph_shader : GlyphShader,
    text_shader : TextShader,
    glyph_buffer : WebGlTexture,
    width : i32,
    height : i32,
    density : i32
}

impl Context {
    pub fn new(webgl_context : WebGlRenderingContext) -> Result<Self, JsValue> {
        let glyph_buffer = webgl_context.create_texture().unwrap();
        let glyph_shader = GlyphShader::new(webgl_context.clone())?;
        let text_shader = TextShader::new(webgl_context.clone())?;
        let width = webgl_context.drawing_buffer_width();
        let height = webgl_context.drawing_buffer_height();
        let density = web_sys::window().unwrap().device_pixel_ratio() as i32;
        Ok(Self {
            webgl_context,
            transform : Transform::new(),
            glyph_shader,
            text_shader,
            glyph_buffer,
            width,
            height,
            density
        })
    }

    pub fn context(&self) -> &WebGlRenderingContext {
        &self.webgl_context
    }

    pub fn pixel_width(&self) -> i32 {
        self.width * self.density
    }

    pub fn pixel_height(&self) -> i32 {
        self.height * self.density
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn set_transform(&mut self, transform : Transform) {
        self.transform = transform;
    }

    pub fn start_frame(&mut self) -> Result<(), JsValue> {
        let ctx = &self.webgl_context;
        self.width = ctx.drawing_buffer_width();
        self.height = ctx.drawing_buffer_height();
        self.density = web_sys::window().unwrap().device_pixel_ratio() as i32;
        let mut transform = Transform::new();
        transform.translate(-1.0, 0.0);
        transform.scale(1.0/ (self.width as f32), -1.0/(self.height as f32));
        self.transform = transform;
        self.render_to_canvas();
        ctx.viewport(0, 0, self.pixel_width(), self.pixel_height());
        ctx.disable(WebGlRenderingContext::BLEND);
        self.clear();
        self.glyph_buffer = self.create_texture(self.pixel_width(), self.pixel_height())?;
        Ok(())
    }

    pub fn width(&self) -> i32 {
        self.webgl_context.drawing_buffer_width()
    }

    pub fn height(&self) -> i32 {
        self.webgl_context.drawing_buffer_height()
    }

    pub fn clear(&self){
        self.webgl_context.clear_color(0.5, 0.5, 0.5, 1.0);
        self.webgl_context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT); 
    }

    pub fn create_texture(&self, width : i32, height : i32) -> Result<WebGlTexture, JsValue> {
        let context = &self.webgl_context;
        let texture = context.create_texture();
        context.bind_texture(WebGlRenderingContext::TEXTURE_2D, texture.as_ref());
        context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_MAG_FILTER, WebGlRenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_MIN_FILTER, WebGlRenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_WRAP_S, WebGlRenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_parameteri(WebGlRenderingContext::TEXTURE_2D, WebGlRenderingContext::TEXTURE_WRAP_T, WebGlRenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGlRenderingContext::TEXTURE_2D, // target
            0, // level
            WebGlRenderingContext::RGBA as i32, //internal format, specifies the color components in the texture.
            width, height, 
            0, // border "Must be 0."
            WebGlRenderingContext::RGBA, // format, must be same as internal format (but apparently this time it's a u32????)
            WebGlRenderingContext::UNSIGNED_BYTE, // type: specifying the data type of the texel data
            None // u8 array source
        )?;
        Ok(texture.unwrap())
    }

    pub fn render_to_texture(&self, texture : &WebGlTexture) {
        let context = &self.webgl_context;
        let framebuffer = context.create_framebuffer();
        context.bind_framebuffer(WebGlRenderingContext::FRAMEBUFFER, framebuffer.as_ref());
        context.framebuffer_texture_2d(
            WebGlRenderingContext::FRAMEBUFFER, 
            WebGlRenderingContext::COLOR_ATTACHMENT0, 
            WebGlRenderingContext::TEXTURE_2D, 
            Some(texture), 0
        );
    }

    pub fn render_to_canvas(&self) {
        self.webgl_context.bind_framebuffer(WebGlRenderingContext::FRAMEBUFFER, None);
    }

    pub fn add_blend_mode(&self){
        self.webgl_context.enable(WebGlRenderingContext::BLEND);
        self.webgl_context.blend_func(WebGlRenderingContext::ONE, WebGlRenderingContext::ONE);
    }

    pub fn copy_blend_mode(&self){
        self.webgl_context.disable(WebGlRenderingContext::BLEND);
    }
    
    pub fn draw_letter_inner(&self, glyph : &Glyph, x : f32, y : f32, scale : f32) -> Result<(), JsValue> {
        self.add_blend_mode();
        self.render_to_texture(&self.glyph_buffer);
        self.webgl_context.viewport(0, 0, self.pixel_width(), self.pixel_height());
        self.clear();
        let mut transform = self.transform();
        transform.translate(x, y);
        self.glyph_shader.draw(transform, glyph, scale, self.density)?;
        transform.scale(scale, scale);

        self.render_to_canvas();
        self.copy_blend_mode();
        
        self.webgl_context.active_texture(WebGlRenderingContext::TEXTURE0);
        self.webgl_context.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&self.glyph_buffer));
        
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

    pub fn draw_line(&self, px : f32, py : f32, qx : f32, qy : f32, thickness : f32) -> Result<(), JsValue> {
        let mut line_shader = LineShader::new(self.webgl_context.clone())?;
        line_shader.add_line(Vec2::new(px, py), Vec2::new(qx, qy), thickness);
        self.copy_blend_mode();
        self.render_to_canvas();
        line_shader.draw(self.transform())?;
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