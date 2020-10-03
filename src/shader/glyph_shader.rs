use std::convert::TryInto;
use std::collections::{BTreeMap, btree_map};
use uuid::Uuid;
use std::rc::Rc;


use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, 
    WebGlBuffer, 
    WebGlVertexArrayObject,
};

use lyon::geom::math::{Point};

use lyon::tessellation::{VertexBuffers};

use crate::log;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::Program;
use crate::vector::Vec4;

use crate::glyph::{GlyphInstance, Glyph};

use crate::shader::attributes::{Format, Type, NumChannels,  Attribute, Attributes};
use crate::shader::data_texture::DataTexture;

use crate::coordinate_system::CoordinateSystem;


const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aPosition", 2, Type::F32),
    Attribute::new("aScale", 1, Type::F32),
    Attribute::new("aFillColor", 4, Type::F32),
    Attribute::new("aStrokeColor", 4, Type::F32),
    Attribute::new("aGlyphData", 4, Type::I16), // (index, num_fill_vertices, num_stroke_vertices, padding)
]);



#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ShaderGlyphHeader {
    index : u16,
    num_fill_vertices : u16,
    num_stroke_vertices : u16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ShaderGlyphInstance {
    position : Point,
    scale : f32,
    fill_color : Vec4,
    stroke_color : Vec4,
    
    // aGlyphData
    index : u16,
    num_fill_vertices : u16,
    num_stroke_vertices : u16,
    padding : u16,
}


pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub program : Program,
    glyph_map : BTreeMap<Uuid, ShaderGlyphHeader>,

    glyph_instances : Vec<ShaderGlyphInstance>,

    
    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,

    // Vertices has its length padded to a multiple of DATA_ROW_SIZE so that it will fit correctly into the data_texture
    // so we need to separately store the number of actually used entries separately.
    max_glyph_num_vertices : usize,
    vertices_data : DataTexture<Point>
}



impl GlyphShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl.clone(), 
            include_str!("glyph.vert"),
            // include_str!("glyph.frag"),
            r#"#version 300 es
                precision highp float;
                flat in vec4 fColor;
                out vec4 outColor;
                void main() {
                    outColor = fColor;
                }
            "#
        )?;

        let attribute_state = webgl.create_vertex_array();
        let attributes_buffer = webgl.create_buffer();

        ATTRIBUTES.set_up_vertex_array(&webgl, &program.program, attribute_state.as_ref(), attributes_buffer.as_ref())?;

        let vertices_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));
        program.use_program();
        program.set_uniform_int("uGlyphDataTexture", 0);

        Ok(Self {
            webgl,
            program,
            glyph_map : BTreeMap::new(),

            glyph_instances : Vec::new(), 
            max_glyph_num_vertices : 0,
            
            attribute_state,
            attributes_buffer,
            vertices_data
        })
    }

    pub fn clear_glyphs(&mut self, ){
        self.max_glyph_num_vertices = 0;
        self.vertices_data.clear();
        self.glyph_map.clear();
        self.glyph_instances.clear();
    }

    fn glyph_data(&mut self, glyph : &Rc<Glyph>) -> Result<ShaderGlyphHeader, JsValue> {
        let entry = self.glyph_map.entry(glyph.uuid);
        // If btree_map::Entry had a method "or_try_insert(f : K -> Result<V, E>) -> Result<&V, E>" we could use that instead.
        match entry {
            btree_map::Entry::Occupied(oe) => Ok(*oe.get()),
            btree_map::Entry::Vacant(ve) => {
                let index = self.vertices_data.len();

                let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
                let scale = 100.0;
                
                glyph.tessellate_fill(&mut buffers, scale)?;
                let fill_num_vertices = buffers.indices.len();
                self.vertices_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                
                buffers.vertices.clear();
                buffers.indices.clear();

                glyph.tessellate_stroke(&mut buffers, scale)?;
                let stroke_num_vertices = buffers.indices.len();
                self.vertices_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                
                self.max_glyph_num_vertices = self.max_glyph_num_vertices.max(fill_num_vertices + stroke_num_vertices);
                log!("New glyph... index : {}, fill_num_vertices : {}, stroke_num_vertices : {}", index, fill_num_vertices, stroke_num_vertices);


                Ok(*ve.insert(ShaderGlyphHeader {
                    index : index.try_into().unwrap(), 
                    num_fill_vertices : fill_num_vertices.try_into().unwrap(), 
                    num_stroke_vertices : stroke_num_vertices.try_into().unwrap()
                }))
            }
        }
    }

    pub fn add_glyph(&mut self, glyph_instance : GlyphInstance) -> Result<(), JsValue> {
        let ShaderGlyphHeader { index, num_fill_vertices, num_stroke_vertices } = self.glyph_data(&glyph_instance.glyph)?;
        self.glyph_instances.push(ShaderGlyphInstance {
            position : glyph_instance.center,
            scale : glyph_instance.scale / 100.0,
            fill_color : glyph_instance.fill_color,
            stroke_color : glyph_instance.stroke_color,
            index,
            num_fill_vertices,
            num_stroke_vertices,
            padding : 0
        });
        Ok(())
    }

    fn set_buffer_data(&self){
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, self.attributes_buffer.as_ref());
        let u8_len = self.glyph_instances.len() * std::mem::size_of::<ShaderGlyphInstance>();
        let u8_ptr = self.glyph_instances.as_ptr() as *mut u8;
        unsafe {
            let vert_array = js_sys::Uint8Array::view_mut_raw(u8_ptr, u8_len);
            crate::console_log::log_1(&vert_array);
            self.webgl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
    }


    pub fn prepare(&mut self) -> Result<(), JsValue> {
        log!("glyph_instances : {:?}", self.glyph_instances);
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.program.use_program();
        self.set_buffer_data();
        self.vertices_data.upload()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }

    pub fn draw(&mut self, coordinate_system : CoordinateSystem) {
        self.program.use_program();
        self.vertices_data.bind(WebGl2RenderingContext::TEXTURE0);
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.program.set_uniform_transform("uTransformationMatrix", coordinate_system.transform);
        self.program.set_uniform_point("uOrigin", coordinate_system.origin);
        self.program.set_uniform_vector("uScale", coordinate_system.scale);

        let num_instances = self.glyph_instances.len() as i32;
        let num_vertices = self.max_glyph_num_vertices as i32;
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            num_vertices,
            num_instances
        );
        self.webgl.bind_vertex_array(None);
    }
}