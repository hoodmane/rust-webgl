use crate::vector::{Vec2, Vec3, Vec4};
use crate::matrix::{Matrix3, Transform};

use std::collections::BTreeMap;

use wasm_bindgen::JsValue;
use web_sys::{
    WebGlRenderingContext, 
    WebGlBuffer, 
    WebGlProgram, 
    WebGlShader
};


#[derive(Debug)]
struct Attribute {
    name : String,
    attribute_type : u32,
    size : i32,
    loc : usize
}

pub struct Shader {
    pub context : WebGlRenderingContext,
    program : WebGlProgram,
    buffers : Vec<WebGlBuffer>,
    attributes : BTreeMap<String, Attribute>,
}

impl Shader {
    pub fn new(context : WebGlRenderingContext, vertex_shader : &str, fragment_shader : &str) -> Result<Self, JsValue> {
        let vert_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            vertex_shader
        )?;
        let frag_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            fragment_shader
        )?;
        let program = link_program(&context, &vert_shader, &frag_shader)?;
        let num_attributes = context.get_program_parameter(&program, WebGlRenderingContext::ACTIVE_ATTRIBUTES)
            .as_f64().ok_or("failed to get number of attributes")?
            as u32;
        let attributes = BTreeMap::new();
        let mut buffers = Vec::new();
        for _ in 0..num_attributes {
            buffers.push(context.create_buffer().ok_or("failed to create buffer")?);
        }
        Ok(Shader {
            context, 
            program,
            buffers,
            attributes,
        })
    }

    pub fn add_attribute(&mut self, name : &str, size : i32, attribute_type : u32) -> Result<(), JsValue> {
        let loc = self.context.get_attrib_location(&self.program, &name);
        if loc == -1 {
            let s = format!("Unknown attribute {}", name);
            return Err(JsValue::from_str(&s));
        }
        self.attributes.insert(name.to_string(), Attribute {
            name : name.to_string(),
            attribute_type,
            size,
            loc : loc as usize
        });
        Ok(())
    }

    pub fn set_data(&self, name : &str, data : &[f32]) -> Result<(), JsValue> {
        let attribute = self.attributes.get(name).ok_or(format!("Unknown attribute {}", name))?;
        self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.buffers[attribute.loc]));
        // Note that `Float32Array::view` is somewhat dangerous (hence the
        // `unsafe`!). This is creating a raw view into our module's
        // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        // (aka do a memory allocation in Rust) it'll cause the buffer to change,
        // causing the `Float32Array` to be invalid.
        //
        // As a result, after `Float32Array::view` we have to be very careful not to
        // do any memory allocations before it's dropped.
        unsafe {
            let vert_array = js_sys::Float32Array::view(&data);
            self.context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }
        Ok(())
    }

    fn bind_attributes(&self){
        for (_k, attribute) in self.attributes.iter() {
            // log_str(&format!("Attribute: {:?}", attribute));
            self.context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.buffers[attribute.loc]));
            self.context.vertex_attrib_pointer_with_i32(attribute.loc as u32, attribute.size, attribute.attribute_type, false, 0, 0);
            self.context.enable_vertex_attrib_array(attribute.loc as u32);
        }
    }

    pub fn use_program(&self){
        self.context.use_program(Some(&self.program));
    }

    pub fn draw(&self, num_vertices : usize) {
        self.bind_attributes();
        self.context.draw_arrays(
            WebGlRenderingContext::TRIANGLES,
            0,
            num_vertices as i32
        );
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
    context: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
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
    context: &WebGlRenderingContext,
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
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
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
