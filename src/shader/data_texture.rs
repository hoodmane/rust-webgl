#[allow(unused_imports)]
use crate::log;
use crate::shader::attributes::{Format, Type};
use web_sys::{WebGl2RenderingContext, WebGlTexture};
use wasm_bindgen::{JsValue};
use js_sys::Object;
use crate::webgl_wrapper::WebGlWrapper;


pub struct DataTexture<T> {
    webgl : WebGlWrapper,
    width : usize,
    format : Format,
    data : Vec<u32>,
    used_data : usize,
    texture : Option<WebGlTexture>,
    texture_rows : usize,
    marker : std::marker::PhantomData<T>
}

impl<T> DataTexture<T> {
    pub fn new(webgl : WebGlWrapper, format : Format) -> Self {
        Self {
            webgl,
            width : 2048, 
            format,
            data : Vec::new(),
            used_data : 0,
            texture : None,
            texture_rows : 0,
            marker : std::marker::PhantomData
        }
    }

    fn entry_bytes(&self) -> usize {
        std::mem::size_of::<T>()
    }

    fn row_bytes(&self) -> usize {
        self.width * (self.format.size() as usize)
    }

    fn num_rows(&mut self) -> usize {
        self.num_rows_to_fit_extra_data(0)
    }

    fn num_rows_to_fit_extra_data(&self, n : usize) -> usize {
        let total_bytes = self.used_data * 4 + n * self.entry_bytes();
        ( total_bytes + self.row_bytes() - 1) / self.row_bytes()
    }

    fn ensure_size(&mut self){
        let num_rows = self.num_rows();
        if num_rows <= self.texture_rows {
            return;
        }
        self.texture_rows = num_rows;
        self.webgl.delete_texture(self.texture.as_ref());
        self.texture = self.webgl.inner.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            self.format.internal_format(),
            self.width as i32, num_rows as i32
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
    }

    unsafe fn data_view(&self, num_rows : usize) -> Object {
        let data = &self.data[..num_rows * self.row_bytes()];
        match self.format.0 {
            Type::F32 => js_sys::Float32Array::view_mut_raw(data.as_ptr() as *mut f32, data.len()).into(),
            Type::I16 | Type::U16 | Type::U8 | Type::U32
                => js_sys::Uint8Array::view_mut_raw(data.as_ptr() as *mut u8, data.len() * 4).into(),
        }
    }

    pub fn len(&self) -> usize {
        self.used_data / (self.entry_bytes() / 4)
    }

    pub fn clear(&mut self){
        self.used_data = 0;
    }

    pub fn append<It : ExactSizeIterator<Item = T>>(&mut self, data : It) {
        let data_len = data.len();
        let total_rows_needed = self.num_rows_to_fit_extra_data(data_len);
        if total_rows_needed > self.num_rows() {
            self.data.resize_with(total_rows_needed * self.row_bytes(), ||0);
        }
        self.data.splice(self.used_data .. self.used_data + data_len * (self.entry_bytes() / 4), 
            data.flat_map(|e| unsafe {  
                std::slice::from_raw_parts(
                    &e as *const T as *const u32, 
                    std::mem::size_of::<T>()/std::mem::size_of::<u32>()
                ) 
            }.iter().cloned())
        ).for_each(drop);
        self.used_data += data_len * (self.entry_bytes()/4);
    }

    pub fn upload(&mut self) -> Result<(), JsValue> {
        self.ensure_size();
        let num_rows = self.num_rows();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.texture.as_ref());
        unsafe {
            let array_view = self.data_view(num_rows);
            self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D, 
                0, // mip level
                0, 0, // xoffset, yoffset: i32,
                self.width as i32, num_rows as i32, // width, height
                self.format.base_format(), // format: u32,
                self.format.webgl_type(), // type_: u32,
                Some(&array_view) // pixels: Option<&Object>
            )?; 
        }
        Ok(())
    }

    pub fn bind(&self, texture_unit : u32){
        self.webgl.active_texture(texture_unit);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.texture.as_ref());
    }
}
