use crate::vector::{Vec2, Vec3, Vec4};
use crate::matrix::{Matrix3, Transform};
use crate::webgl_wrapper::WebGlWrapper;

use uuid::Uuid;


use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, 
    WebGlBuffer, 
    WebGlProgram, 
    WebGlShader,
    WebGlVertexArrayObject
};

pub struct Geometry {
    pub num_vertices : i32,
    pub num_instances : i32,
    attribute_state : WebGlVertexArrayObject,
    buffers : Vec<WebGlBuffer>,
    buffer_sizes : Vec<usize>,
    shader_uuid : Uuid,
}

#[derive(Debug)]
struct Attribute {
    name : String,
    attribute_type : u32,
    size : i32,
    loc : u32,
    instance_divisor : u32
}

impl Attribute {
    fn dummy() -> Self {
        Attribute {
            name : String::new(),
            attribute_type : u32::MAX,
            size : -1,
            loc : u32::MAX,
            instance_divisor : 0
        }
    }
    
}

pub struct Shader {
    pub webgl : WebGlWrapper,
    program : WebGlProgram,
    attributes : Vec<Attribute>,
    uuid : Uuid
}

impl Shader {
    pub fn new(webgl : WebGlWrapper, vertex_shader : &str, fragment_shader : &str) -> Result<Self, JsValue> {
        let vert_shader = compile_shader(
            &webgl,
            WebGl2RenderingContext::VERTEX_SHADER,
            vertex_shader
        )?;
        let frag_shader = compile_shader(
            &webgl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            fragment_shader
        )?;
        let program = link_program(&webgl, &vert_shader, &frag_shader)?;
        let num_attributes = webgl.get_program_parameter(&program, WebGl2RenderingContext::ACTIVE_ATTRIBUTES)
            .as_f64().ok_or("failed to get number of attributes")?
            as u32;
        let mut attributes = Vec::new();
        for _ in 0..num_attributes {
            attributes.push(Attribute::dummy());
        }
        Ok(Shader {
            webgl, 
            program,
            attributes,
            uuid : Uuid::new_v4()
        })
    }

    pub fn create_geometry(&self) -> Result<Geometry, JsValue> {
        let attribute_state = self.webgl.create_vertex_array().unwrap();
        let mut buffers = Vec::new();
        let mut buffer_sizes = vec![usize::MAX; self.attributes.len()];
        self.webgl.bind_vertex_array(Some(&attribute_state));
        for attribute in &self.attributes {
            let buffer = self.webgl.create_buffer().ok_or("failed to create buffer")?;
            buffer_sizes[attribute.loc as usize] = 0;
            self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));
            self.webgl.enable_vertex_attrib_array(attribute.loc);
            self.webgl.vertex_attrib_pointer_with_i32(attribute.loc, attribute.size, attribute.attribute_type, false, 0, 0);
            self.webgl.vertex_attrib_divisor(attribute.loc, attribute.instance_divisor);
            buffers.push(buffer);
        }
        self.webgl.bind_vertex_array(None); 
        Ok(Geometry {
            num_vertices : 0,
            num_instances : 0,
            attribute_state,
            buffers,
            buffer_sizes,
            shader_uuid : self.uuid
        })
    }

    
    fn check_geometry(&self, geometry : &Geometry) -> Result<(), JsValue> {
        if geometry.shader_uuid == self.uuid {
            Ok(())
        } else {
            Err(JsValue::from_str(&format!("Geometry does not correspond to this shader")))
        }
    }


    pub fn add_attribute_float(&mut self, name : &str, instanced : bool) -> Result<(), JsValue> {
        self.add_attribute(name, 1, WebGl2RenderingContext::FLOAT, instanced)?;
        Ok(())
    }

    pub fn add_attribute_vec2f(&mut self, name : &str, instanced : bool) -> Result<(), JsValue> {
        self.add_attribute(name, 2, WebGl2RenderingContext::FLOAT, instanced)?;
        Ok(())
    }

    pub fn add_attribute_vec3f(&mut self, name : &str, instanced : bool) -> Result<(), JsValue> {
        self.add_attribute(name, 3, WebGl2RenderingContext::FLOAT, instanced)?;
        Ok(())
    }

    pub fn add_attribute_vec4f(&mut self, name : &str, instanced : bool) -> Result<(), JsValue> {
        self.add_attribute(name, 4, WebGl2RenderingContext::FLOAT, instanced)?;
        Ok(())
    }

    fn attrib_location(&self, name : &str) -> Result<u32, JsValue> {
        let loc = self.webgl.get_attrib_location(&self.program, &name);
        if loc < 0 {
            Err(JsValue::from_str(&format!("Unknown attribute \"{}\"", name)))
        } else {
            Ok(loc as u32)
        }
    }

    fn add_attribute(&mut self, name : &str, size : i32, attribute_type : u32, instanced : bool) -> Result<(), JsValue> {
        let loc = self.attrib_location(name)?;
        self.attributes[loc as usize] = Attribute {
            name : name.to_string(),
            attribute_type,
            loc, 
            size,
            instance_divisor : if instanced { 1 } else { 0 }
        };
        Ok(())
    }

    fn set_array_buffer_data_from_slice(&self, data : &[f32]){
        unsafe {
            let vert_array = js_sys::Float32Array::view(&data);
            self.webgl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
    }


    pub fn set_attribute_data(&self, geometry : &mut Geometry, name : &str, data : &[f32]) -> Result<(), JsValue> {
        self.check_geometry(geometry)?;
        self.webgl.bind_vertex_array(Some(&geometry.attribute_state));
        let loc = self.attrib_location(&name)? as usize;
        let attribute = &self.attributes[loc];
        let attribute_size = attribute.size as usize;
        if data.len() % attribute_size != 0 {
            self.webgl.bind_vertex_array(None);
            return Err(JsValue::from_str(&format!(
                "Buffer has length {} not a multiple of attribute \"{}\" size {}", 
                data.len(),
                attribute.name,
                attribute_size
            )));
        }
        geometry.buffer_sizes[loc] = data.len() / attribute_size;
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&geometry.buffers[loc]));
        self.set_array_buffer_data_from_slice(data);
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, None);
        self.webgl.bind_vertex_array(None);
        Ok(())
    }

    pub fn use_program(&self){
        self.webgl.use_program(Some(&self.program));
    }

    pub fn unuse_program(&self){
        self.webgl.use_program(None);
        self.webgl.bind_vertex_array(None);
    }

    fn check_geometry_buffer_sizes(&self, geometry : &Geometry) -> Result<(), JsValue> {
        for (&buffer_size, attribute) in geometry.buffer_sizes.iter().zip(&self.attributes) {
            if buffer_size == usize::MAX {
                continue;
            }
            let expected_size;
            if attribute.instance_divisor == 0 {
                expected_size = geometry.num_vertices;
            } else {
                expected_size = geometry.num_instances / attribute.instance_divisor as i32;
            }
            if expected_size != buffer_size as i32 {
                return Err(JsValue::from_str(&format!(
                    "Buffer \"{}\" has incorrect size: it was expected to have size {} but it has size {}.", 
                    attribute.name,
                    expected_size, buffer_size
                )));
            }
        }
        Ok(())
    }

    pub fn draw(&self, geometry : &Geometry) -> Result<(), JsValue> {
        self.check_geometry_buffer_sizes(geometry)?;
        self.webgl.bind_vertex_array(Some(&geometry.attribute_state));
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            geometry.num_vertices,
            geometry.num_instances
        );
        self.webgl.bind_vertex_array(None);
        Ok(())
    }


    pub fn set_uniform_float(&self, name : &str, x : f32) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform1f(loc.as_ref(), x);
    }

    pub fn set_uniform_int(&self, name : &str, x : i32) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform1iv_with_i32_array(loc.as_ref(), &[x]);
    }

    pub fn set_uniform_vec2(&self, name : &str, v2 : Vec2<f32>) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform2fv_with_f32_array(loc.as_ref(), &[v2.x, v2.y]);
    }

    pub fn set_uniform_vec3(&self, name : &str, v3 : Vec3<f32>) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform3fv_with_f32_array(loc.as_ref(), &[v3.x, v3.y, v3.z]);
    }

    pub fn set_uniform_vec4(&self, name : &str, v4 : Vec4<f32>) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform4fv_with_f32_array(loc.as_ref(), &[v4.x, v4.y, v4.z, v4.w]);
    }

    pub fn set_uniform_mat3(&self, name : &str, mat3 : Matrix3) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform_matrix3fv_with_f32_array(loc.as_ref(), false, &mat3.data);
    }

    pub fn set_uniform_transform(&self, name : &str, transform : Transform) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform_matrix3fv_with_f32_array(loc.as_ref(), false, &transform.data);
    }

    pub fn set_uniform_transform_from_slice(&self, name : &str, slice : &[f32]) {
        let loc = self.webgl.get_uniform_location(&self.program, name);  
        self.webgl.uniform_matrix3fv_with_f32_array(loc.as_ref(), false, slice);
    }
}


fn compile_shader(
    webgl: &WebGlWrapper,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let webgl = &webgl.inner;
    let shader = webgl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    webgl.shader_source(&shader, source);
    webgl.compile_shader(&shader);

    if webgl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(webgl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

fn link_program(
    webgl: &WebGlWrapper,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let webgl = &webgl.inner;
    let program = webgl
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    webgl.attach_shader(&program, vert_shader);
    webgl.attach_shader(&program, frag_shader);
    webgl.link_program(&program);

    if webgl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(webgl
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
