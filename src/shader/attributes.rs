use std::convert::TryInto;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlTexture};
use wasm_bindgen::JsValue;

use crate::shader::{Shader};
use crate::webgl_wrapper::WebGlWrapper;

#[derive(Copy, Clone, Debug)]
pub enum Type {
    Float,
    Short
}

impl Type { 
    fn size(self) -> i32 {
        match self {
            Type::Float => std::mem::size_of::<f32>() as i32,
            Type::Short => std::mem::size_of::<u16>() as i32
        }
    }

    fn webgl_type(self) -> u32 {
        match self {
            Type::Float => WebGl2RenderingContext::FLOAT,
            Type::Short => WebGl2RenderingContext::SHORT
        }
    }
}

pub struct Attribute {
    name : &'static str, 
    size : usize, 
    ty : Type,
}

impl Attribute {
    pub const fn new(name : &'static str, size : usize, ty : Type) -> Self {
        Self {
            name, size, ty
        }
    }
}


pub struct Attributes {
    attributes : &'static [Attribute]
}

impl Attributes {
    pub const fn new(attributes : &'static [Attribute]) -> Self {
        Self {
            attributes
        }
    }

    fn offset(&self, idx : usize) -> i32 {
        self.attributes[..idx].iter().map(|&Attribute {size, ty, ..}|
            ty.size() * (size as i32)
        ).sum()
    }
    
    fn stride(&self) -> i32 {
        self.offset(self.attributes.len())
    }

    pub fn set_up_vertex_array(&self, webgl : &WebGlWrapper, shader : &Shader, attribute_state : Option<&WebGlVertexArrayObject>, attributes_buffer : Option<&WebGlBuffer>) -> Result<(), JsValue> {
        webgl.bind_vertex_array(attribute_state);
        // IMPORTANT: Must bind_buffer here!!!!
        // vertex_attrib_pointer uses the current bound buffer implicitly.
        webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, attributes_buffer);
    
        let stride = self.stride();
        for (idx, &Attribute {name, size, ty}) in self.attributes.iter().enumerate() {
            let size = size as i32;
            let loc = webgl.get_attrib_location(&shader.program, name).try_into().map_err(|_| name.to_string())?;
            let offset = self.offset(idx);
            webgl.enable_vertex_attrib_array(loc);
            match ty {
                Type::Float => {webgl.vertex_attrib_pointer_with_i32(loc, size, ty.webgl_type(), false, stride, offset)},
                Type::Short => {webgl.vertex_attrib_i_pointer_with_i32(loc, size, ty.webgl_type(), stride, offset)}
            };
            webgl.vertex_attrib_divisor(loc, 1);
        }
        webgl.bind_vertex_array(None);
        Ok(())
    }
    
}

