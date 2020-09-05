use web_sys::{HtmlCanvasElement, WebGlTexture, WebGl2RenderingContext};
use wasm_bindgen::{JsValue, JsCast};
use std::ops::Deref;


#[derive(Clone)]
pub struct WebGlWrapper {
    pub inner : WebGl2RenderingContext
}

impl Deref for WebGlWrapper {
    type Target = WebGl2RenderingContext;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl WebGlWrapper {
    pub fn new(inner : WebGl2RenderingContext) -> Self {
        Self { inner }
    }

    pub fn canvas(&self) -> Result<HtmlCanvasElement, JsValue> {
        Ok(self.inner.canvas().unwrap().dyn_into()?)
    }

    pub fn density() -> f64 {
        web_sys::window().unwrap().device_pixel_ratio()
    }

    pub fn width(&self) -> i32 {
        self.inner.drawing_buffer_width()
    }

    pub fn height(&self) -> i32 {
        self.inner.drawing_buffer_height()
    }

    pub fn clear(&self){
        self.inner.clear_color(0.5, 0.5, 0.5, 1.0);
        self.inner.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT); 
    }

    pub fn create_texture(&self, width : i32, height : i32) -> Result<WebGlTexture, JsValue> {
        let context = &self.inner;
        let texture = context.create_texture();
        context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, texture.as_ref());
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D, // target
            0, // level
            WebGl2RenderingContext::RGBA as i32, //internal format, specifies the color components in the texture.
            width, height, 
            0, // border "Must be 0."
            WebGl2RenderingContext::RGBA, // format, must be same as internal format (but apparently this time it's a u32????)
            WebGl2RenderingContext::UNSIGNED_BYTE, // type: specifying the data type of the texel data
            None // u8 array source
        )?;
        Ok(texture.unwrap())
    }

    pub fn create_vec2_texture(&self, vecs : &[f32]) -> Result<WebGlTexture, JsValue> {
        let context = &self.inner;
        let texture = context.create_texture();
        context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, texture.as_ref());
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        // tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_f32_array doesn't exist =(
        unsafe {
            let array_view = js_sys::Float32Array::view(&vecs);
            context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D, 
                0, // mip level
                WebGl2RenderingContext::RG32F as i32, //internal format, specifies the color components in the texture.
                (vecs.len()/2) as i32, 1, 
                0, // border "Must be 0."
                WebGl2RenderingContext::RG, // format, must be same as internal format (but apparently this time it's a u32????)
                WebGl2RenderingContext::FLOAT, // type: specifying the data type of the texel data
                Some(&array_view) // data source
            )?;
        }
        Ok(texture.unwrap())
    }


    pub fn render_to_texture(&self, texture : &WebGlTexture) {
        let framebuffer = self.inner.create_framebuffer();
        self.inner.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, framebuffer.as_ref());
        self.inner.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER, 
            WebGl2RenderingContext::COLOR_ATTACHMENT0, 
            WebGl2RenderingContext::TEXTURE_2D, 
            Some(texture), 0
        );
    }

    pub fn render_to_canvas(&self) {
        self.inner.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
    }

    pub fn add_blend_mode(&self){
        self.inner.enable(WebGl2RenderingContext::BLEND);
        self.inner.blend_func(WebGl2RenderingContext::ONE, WebGl2RenderingContext::ONE);
    }

    pub fn copy_blend_mode(&self){
        self.inner.disable(WebGl2RenderingContext::BLEND);
    }
}