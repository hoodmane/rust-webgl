use std::convert::TryInto;
use std::collections::{BTreeMap, btree_map};
use uuid::Uuid;


use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, 
    WebGlVertexArrayObject,
};

use lyon::geom::math::{Point};

use lyon::tessellation::{VertexBuffers};

#[allow(unused_imports)]
use crate::log;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::Program;
use crate::vector::Vec4;

use crate::glyph::{GlyphInstance, Glyph};

use crate::shader::attributes::{Format, Type, NumChannels,  Attribute, Attributes};
use crate::shader::data_texture::DataTexture;
use crate::shader::vertex_buffer::VertexBuffer;

use crate::coordinate_system::CoordinateSystem;


const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aPosition", 2, Type::F32),
    Attribute::new("aScale", 1, Type::F32),
    Attribute::new("aColors",4, Type::U16),
    Attribute::new("aGlyphData", 4, Type::U16), // ShaderGlyphHeader: (index, num_fill_vertices, num_stroke_vertices, padding)
]);


#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ShaderGlyphHeader {
    index : u16,
    num_fill_triangles : u16,
    num_stroke_triangles : u16,
    padding : u16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ShaderGlyphInstance {
    position : Point,
    scale : f32,
    fill_color : [u16;2],
    stroke_color : [u16;2],
    
    // aGlyphData
    glyph : ShaderGlyphHeader
}


fn vec4_to_u8_array(v : Vec4) -> [u16;2] {
    [u16::from_le_bytes([
        (v.x * 255.0) as u8, 
        (v.y * 255.0) as u8, 
    ]),
    u16::from_le_bytes([
        (v.z * 255.0) as u8, 
        (v.w * 255.0) as u8, 
    ])]
}



pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub program : Program,
    glyph_map : BTreeMap<Uuid, ShaderGlyphHeader>,
    glyph_instances : VertexBuffer<ShaderGlyphInstance>,    
    attribute_state : Option<WebGlVertexArrayObject>,

    // Vertices has its length padded to a multiple of DATA_ROW_SIZE so that it will fit correctly into the data_texture
    // so we need to separately store the number of actually used entries separately.
    max_glyph_num_triangles : usize,
    vertices_data : DataTexture<Point>,

    ready : bool,
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

        let glyph_instances = VertexBuffer::new(webgl.clone());
        let attribute_state = webgl.create_vertex_array();

        ATTRIBUTES.set_up_vertex_array(&webgl, &program.program, attribute_state.as_ref(), glyph_instances.buffer.as_ref())?;

        let vertices_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));
        program.use_program();
        program.set_uniform_int("uGlyphDataTexture", 0);

        Ok(Self {
            webgl,
            program,
            glyph_map : BTreeMap::new(),

            attribute_state,
            glyph_instances, 
            max_glyph_num_triangles : 0,
            
            vertices_data,
            ready : false
        })
    }

    pub fn clear_glyphs(&mut self){
        self.max_glyph_num_triangles = 0;
        self.vertices_data.clear();
        self.glyph_map.clear();
        self.glyph_instances.clear();
        self.ready = false;
    }

    fn glyph_data(&mut self, glyph : &Glyph) -> Result<ShaderGlyphHeader, JsValue> {
        let entry = self.glyph_map.entry(glyph.uuid);
        // If btree_map::Entry had a method "or_try_insert(f : K -> Result<V, E>) -> Result<&V, E>" we could use that instead.
        match entry {
            btree_map::Entry::Occupied(oe) => Ok(*oe.get()),
            btree_map::Entry::Vacant(ve) => {
                let index = self.vertices_data.len() / 3;
                let index : Result<u16, _> = index.try_into();
                let index = index.map_err(|_| "Too many total glyph vertices : max number of triangles in all glyphs is 65535.")?;

                let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
                let scale = 100.0;
                
                glyph.tessellate_fill(&mut buffers, scale)?;
                let num_fill_triangles = buffers.indices.len()  / 3;
                self.vertices_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                
                buffers.vertices.clear();
                buffers.indices.clear();

                glyph.tessellate_stroke(&mut buffers, scale)?;
                let num_stroke_triangles = buffers.indices.len() / 3;
                self.vertices_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                
                self.max_glyph_num_triangles = self.max_glyph_num_triangles.max(num_fill_triangles + num_stroke_triangles);

                let num_fill_triangles = num_fill_triangles.try_into().unwrap();
                let num_stroke_triangles  = num_stroke_triangles.try_into().unwrap();
                Ok(*ve.insert(ShaderGlyphHeader {
                    index, 
                    num_fill_triangles, 
                    num_stroke_triangles,
                    padding : 0
                }))
            }
        }
    }

    pub fn add_glyph(&mut self, glyph_instance : GlyphInstance) -> Result<(), JsValue> {
        let glyph = self.glyph_data(&glyph_instance.glyph)?;
        self.glyph_instances.push(ShaderGlyphInstance {
            position : glyph_instance.center,
            scale : glyph_instance.scale / 100.0,
            fill_color : vec4_to_u8_array(glyph_instance.fill_color),
            stroke_color : vec4_to_u8_array(glyph_instance.stroke_color),
            glyph 
        });
        self.ready = false;
        Ok(())
    }

    fn prepare(&mut self) -> Result<(), JsValue> {
        if self.ready {
            return Ok(());
        }
        self.ready = true;
        self.glyph_instances.prepare();
        self.vertices_data.upload()?;
        Ok(())
    }

    pub fn draw(&mut self, coordinate_system : CoordinateSystem) -> Result<(), JsValue> {
        if self.glyph_instances.is_empty() {
            return Ok(());
        }
        self.program.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.prepare()?;
        self.vertices_data.bind(WebGl2RenderingContext::TEXTURE0);
        self.program.set_uniform_transform("uTransformationMatrix", coordinate_system.transform);
        self.program.set_uniform_point("uOrigin", coordinate_system.origin);
        self.program.set_uniform_vector("uScale", coordinate_system.scale);
        self.program.set_uniform_float("uGlyphScale", coordinate_system.glyph_scale);

        let num_instances = self.glyph_instances.len() as i32;
        let num_vertices = (self.max_glyph_num_triangles * 3) as i32;
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            num_vertices,
            num_instances
        );
        self.webgl.bind_vertex_array(None);
        Ok(())
    }
}