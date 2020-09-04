use crate::vector::{Vec2, Vec3, Vec4};
use crate::matrix::{Matrix3, Transform};

use std::collections::BTreeMap;

use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, 
    WebGlBuffer, 
    WebGlProgram, 
    WebGlShader,
    WebGlVertexArrayObject
};


#[derive(Debug)]
struct Attribute {
    name : String,
    attribute_type : u32,
    size : i32,
    loc : i32,
    buffer_size : usize,
}

impl Attribute {
    fn new() -> Self {
        Attribute {
            name : String::new(),
            attribute_type : u32::MAX,
            size : -1,
            loc : -1,
            buffer_size : usize::MAX,
        }
    }
    
}

pub struct Shader {
    pub context : WebGl2RenderingContext,
    program : WebGlProgram,
    attributes : Vec<Attribute>,
    attribute_state : WebGlVertexArrayObject,
    buffers : Vec<WebGlBuffer>,
}

impl Shader {
    pub fn new(context : WebGl2RenderingContext, vertex_shader : &str, fragment_shader : &str) -> Result<Self, JsValue> {
        let vert_shader = compile_shader(
            &context,
            WebGl2RenderingContext::VERTEX_SHADER,
            vertex_shader
        )?;
        let frag_shader = compile_shader(
            &context,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            fragment_shader
        )?;
        let program = link_program(&context, &vert_shader, &frag_shader)?;
        let num_attributes = context.get_program_parameter(&program, WebGl2RenderingContext::ACTIVE_ATTRIBUTES)
            .as_f64().ok_or("failed to get number of attributes")?
            as u32;
        let attribute_state = context.create_vertex_array().unwrap();
        let mut buffers = Vec::new();
        let mut attributes = Vec::new();
        for _ in 0..num_attributes {
            attributes.push(Attribute::new());
            buffers.push(context.create_buffer().ok_or("failed to create buffer")?);
        }
        Ok(Shader {
            context, 
            program,
            attributes,
            attribute_state,
            buffers,
        })
    }

    pub fn add_attribute_float(&mut self, name : &str) -> Result<(), JsValue> {
        self.add_attribute(name, 1, WebGl2RenderingContext::FLOAT)?;
        Ok(())
    }

    pub fn add_attribute_vec2f(&mut self, name : &str) -> Result<(), JsValue> {
        self.add_attribute(name, 2, WebGl2RenderingContext::FLOAT)?;
        Ok(())
    }

    pub fn add_attribute_vec3f(&mut self, name : &str) -> Result<(), JsValue> {
        self.add_attribute(name, 3, WebGl2RenderingContext::FLOAT)?;
        Ok(())
    }

    pub fn add_attribute_vec4f(&mut self, name : &str) -> Result<(), JsValue> {
        self.add_attribute(name, 4, WebGl2RenderingContext::FLOAT)?;
        Ok(())
    }

    fn attrib_location(&self, name : &str) -> Result<i32, JsValue> {
        let loc = self.context.get_attrib_location(&self.program, &name);
        if loc == -1 {
            Err(JsValue::from_str(&format!("Unknown attribute \"{}\"", name)))
        } else {
            Ok(loc)
        }
    }

    fn add_attribute(&mut self, name : &str, size : i32, attribute_type : u32) -> Result<(), JsValue> {
        let loc = self.attrib_location(&name)?;
        self.context.bind_vertex_array(Some(&self.attribute_state));
        self.context.enable_vertex_attrib_array(loc as u32);
        self.context.vertex_attrib_pointer_with_i32(loc as u32, size, attribute_type, false, 0, 0);
        self.context.bind_vertex_array(None); 
        self.attributes[loc as usize] = Attribute {
            name : name.to_string(),
            attribute_type,
            loc, 
            size,
            buffer_size : 0
        };
        Ok(())
    }

    fn set_array_buffer_data_from_slice(&self, data : &[f32]){
        unsafe {
            let vert_array = js_sys::Float32Array::view(&data);
            self.context.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
    }


    pub fn set_attribute_data(&mut self, name : &str, data : &[f32]) -> Result<(), JsValue> {
        self.context.bind_vertex_array(Some(&self.attribute_state));
        let loc = self.attrib_location(&name)? as usize;
        let attribute = &mut self.attributes[loc];
        let attribute_size = attribute.size as usize;
        if data.len() % attribute_size != 0 {
            self.context.bind_vertex_array(None);
            return Err(JsValue::from_str(&format!(
                "Buffer has length {} not a multiple of attribute \"{}\" size {}", 
                data.len(),
                attribute.name,
                attribute_size
            )));
        }
        attribute.buffer_size = data.len() / attribute_size;
        self.context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffers[loc]));
        self.set_array_buffer_data_from_slice(data);
        self.context.bind_vertex_array(None);
        Ok(())
    }

    pub fn use_program(&self){
        self.context.use_program(Some(&self.program));
        self.context.bind_vertex_array(Some(&self.attribute_state));
    }

    pub fn unuse_program(&self){
        self.context.use_program(None);
        self.context.bind_vertex_array(None);
    }

    pub fn draw(&self, num_vertices : usize) -> Result<(), JsValue> {
        for attribute in &self.attributes {
            if attribute.buffer_size != num_vertices && attribute.buffer_size != usize::MAX {
                return Err(JsValue::from_str(&format!(
                    "Attribute \"{}\" buffer has wrong length. Length should be {} but is {}.", 
                    attribute.name, num_vertices, attribute.buffer_size
                )));
            }
        }
        self.context.draw_arrays(
            WebGl2RenderingContext::TRIANGLES,
            0,
            num_vertices as i32
        );
        Ok(())
    }

    // fn get_uniform(&self, name : &str) -> {
    //     let loc = self.context.get_uniform_location(&self.program, name);  
    //     if loc == -1 {
    //         let s = format!("Unknown attribute {}", name);
    //         return Err(JsValue::from_str(&s));
    //     }
    //     return loc
    // }


    pub fn set_uniform_float(&self, name : &str, x : f32) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform1f(loc.as_ref(), x);
    }

    pub fn set_uniform_int(&self, name : &str, x : i32) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform1iv_with_i32_array(loc.as_ref(), &[x]);
    }

    pub fn set_uniform_vec2(&self, name : &str, v2 : Vec2<f32>) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform2fv_with_f32_array(loc.as_ref(), &[v2.x, v2.y]);
    }

    pub fn set_uniform_vec3(&self, name : &str, v3 : Vec3<f32>) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform3fv_with_f32_array(loc.as_ref(), &[v3.x, v3.y, v3.z]);
    }

    pub fn set_uniform_vec4(&self, name : &str, v4 : Vec4<f32>) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform4fv_with_f32_array(loc.as_ref(), &[v4.x, v4.y, v4.z, v4.w]);
    }

    pub fn set_uniform_mat3(&self, name : &str, mat3 : Matrix3) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform_matrix3fv_with_f32_array(loc.as_ref(), false, &mat3.data);
    }

    pub fn set_uniform_transform(&self, name : &str, transform : Transform) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform_matrix3fv_with_f32_array(loc.as_ref(), false, &transform.data);
    }

    pub fn set_uniform_transform_from_slice(&self, name : &str, slice : &[f32]) {
        let loc = self.context.get_uniform_location(&self.program, name);  
        self.context.uniform_matrix3fv_with_f32_array(loc.as_ref(), false, slice);
    }
}


pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
