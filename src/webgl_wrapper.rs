use web_sys::{HtmlCanvasElement, Element, WebGlTexture, WebGl2RenderingContext, WebGlFramebuffer};
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

    pub fn width_and_height(&self) -> Result<(i32, i32), JsValue> {
        let canvas = self.canvas()?;
        Ok((canvas.client_width(), canvas.client_height()))
    }

    pub fn pixel_density() -> f64 {
        web_sys::window().unwrap().device_pixel_ratio()
    }

    pub fn point_to_pixels(points : f32) -> f32 {
        ((points as f64) * WebGlWrapper::pixel_density()) as f32
    }

    // pub fn pixel_width(&self) -> i32 {
    //     (self.width() as f64 * WebGlWrapper::pixel_density()) as i32
    // }

    // pub fn pixel_height(&self) -> i32 {
    //     (self.height() as f64 * WebGlWrapper::pixel_density()) as i32
    // }

    // pub fn clear(&self){
    //     self.inner.clear_color(0.5, 0.5, 0.5, 1.0);
    //     self.inner.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT); 
    // }

    pub fn create_texture(&self, width : i32, height : i32, internal_format : u32) -> Result<Option<WebGlTexture>, JsValue> {
        let context = &self.inner;
        let texture = context.create_texture();
        context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, texture.as_ref());
        context.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // levels
            internal_format, //WebGl2RenderingContext::RGBA8, // internalformat,
            width,
            height
        );
        // context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        //     WebGl2RenderingContext::TEXTURE_2D, 
        //     0, // level, 
        //     WebGl2RenderingContext::RGBA as i32, // internalFormat,
        //     width, height, 
        //     0, // border
        //     WebGl2RenderingContext::RGBA,// format, 
        //     WebGl2RenderingContext::UNSIGNED_BYTE, // type, 
        //     None // data
        // );

        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::LINEAR as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::LINEAR as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        Ok(texture)
    }

    pub fn create_vec2_texture(&self, vecs : &[f32]) -> Result<Option<WebGlTexture>, JsValue> {
        self.create_float_storage_texture(2, WebGl2RenderingContext::RG, WebGl2RenderingContext::RG32F, vecs)
    }

    pub fn create_vec4_texture(&self, vecs : &[f32]) -> Result<Option<WebGlTexture>, JsValue> {
        self.create_float_storage_texture(4, WebGl2RenderingContext::RGBA, WebGl2RenderingContext::RGBA32F, vecs)
    }

    fn create_float_storage_texture(&self, size : usize, external_format : u32, internal_format : u32,  vecs : &[f32]) -> Result<Option<WebGlTexture>, JsValue> {
        let context = &self.inner;
        let texture = context.create_texture();
        let width = (vecs.len()/size) as i32;
        let height = 1;
        context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, texture.as_ref());
        context.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            internal_format, // internalformat:,
            width, height
        );
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        context.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        // tex_sub_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_f32_array doesn't exist =(
        unsafe {
            let array_view = js_sys::Float32Array::view(&vecs);
            context.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D, 
                0, // mip level
                0, 0, // xoffset, yoffset: i32,
                width, height,
                external_format, // format: u32,
                WebGl2RenderingContext::FLOAT, // type_: u32,
                Some(&array_view) // pixels: Option<&Object>
            )?;
        }
        Ok(texture)
    }


    pub fn render_to_texture(&self, texture : Option<&WebGlTexture>) -> Option<WebGlFramebuffer> {
        let framebuffer = self.inner.create_framebuffer();
        self.inner.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, framebuffer.as_ref());
        self.inner.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER, 
            WebGl2RenderingContext::COLOR_ATTACHMENT0, 
            WebGl2RenderingContext::TEXTURE_2D, 
            texture, 0
        );
        framebuffer
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

    pub fn premultiplied_blend_mode(&self){
        self.enable(WebGl2RenderingContext::BLEND);
        self.blend_func(WebGl2RenderingContext::ONE, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);
    }
}