use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::log;
use crate::vector::{Vec4};
use crate::shader::{Shader};
use crate::webgl_wrapper::WebGlWrapper;

use crate::shader::attributes::{Format, Type, NumChannels, Attribute, Attributes};
use crate::shader::data_texture::DataTexture;

use lyon::geom::math::{Point, Vector, Transform};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlTexture};

use crate::convex_hull::ANGLE_RESOLUTION;


const DATA_ROW_SIZE : usize = 2048;



const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aColor", 4, Type::F32), // color
    Attribute::new("aStartPosition", 2, Type::F32), // start_position
    Attribute::new("aEndPosition", 2, Type::F32), // end_position
    Attribute::new("aStartGlyph", 1, Type::I16), // start_glyph
    Attribute::new("aEndGlyph", 1, Type::I16), // end_glyph
    Attribute::new("aStartGlyphScale", 1, Type::F32), // start_glyph_scale
    Attribute::new("aEndGlyphScale", 1, Type::F32), // end_glyph_scale

    Attribute::new("aStartArrowNumVertices", 1, Type::I16), 
    // Attribute::new("aStartArrowHeaderIndex", 1, Type::Short), 
    Attribute::new("aStartArrowVerticesIndex", 1, Type::I16),

    Attribute::new("aEndArrowNumVertices", 1, Type::I16), 
    // Attribute::new("aEndArrowHeaderIndex", 1, Type::Short), 
    Attribute::new("aEndArrowVerticesIndex", 1, Type::I16), 
]);



#[derive(Clone, Copy, Debug)]
#[repr(C, align(4))]
struct EdgeInstance {
    color : Vec4,
    start_position : Point,
    end_position : Point,
    start_glyph : u16,
    end_glyph : u16,
    start_glyph_scale : f32,
    end_glyph_scale : f32,

    start_arrow : ArrowIndices,
    end_arrow : ArrowIndices
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct ArrowHeader {
    tip_end : f32,
    back_end : f32,
    visual_tip_end : f32,
    visual_back_end : f32,
    line_end : f32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct ArrowIndices {
    num_vertices : u16,
    // header_index : u16,
    vertices_index : u16
}

pub struct TestEdgeShader {
    webgl : WebGlWrapper,
    shader : Shader,
    
    edge_instances : Vec<EdgeInstance>,


    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,
    
    glyph_map : BTreeMap<String, u16>,
    glyph_boundary_data : DataTexture<f32>,

    tip_map : BTreeMap<String, ArrowIndices>,
    max_arrow_tip_num_vertices : usize,
    arrow_tip_data : DataTexture<Point>,
}

fn glyph_boundary_index(glyph_index : usize) -> usize {
    glyph_index * ANGLE_RESOLUTION
}


impl TestEdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let shader = Shader::new(
            webgl.clone(), 
            include_str!("edge.vert"),
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
        ATTRIBUTES.set_up_vertex_array(&webgl, &shader, attribute_state.as_ref(), attributes_buffer.as_ref())?;

        let glyph_boundary_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let arrow_tip_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));

        Ok(Self {
            webgl,
            shader,

            attribute_state,
            attributes_buffer,

            glyph_map : BTreeMap::new(),
            glyph_boundary_data,

            tip_map : BTreeMap::new(),
            arrow_tip_data,
            max_arrow_tip_num_vertices : 0,
            

            edge_instances : Vec::new(),
        })
    }

    pub fn glyph_boundary_data(&mut self, glyph_name : String, boundary_data : &[Vector]){
        debug_assert!(boundary_data.len() == ANGLE_RESOLUTION);
        self.glyph_map.insert(glyph_name, self.glyph_map.len() as u16);
        self.glyph_boundary_data.insert(boundary_data.iter().map(|v| v.length()));
    }

    pub fn arrow_tip_data(&mut self, tip_name : String, vertices : &[Point], indices : &[u16], index_offset : u16) {
        let vertices_index = self.arrow_tip_data.len();
        let num_vertices = indices.len();
        self.arrow_tip_data.insert(indices.iter().map(|&i| vertices[(i - index_offset) as usize]));
        self.max_arrow_tip_num_vertices = self.max_arrow_tip_num_vertices.max(num_vertices);
        let arrow_indices = ArrowIndices {
            num_vertices : num_vertices as u16,
            vertices_index : vertices_index as u16,
            // header_index
        };
        self.tip_map.insert(tip_name, arrow_indices);
    }


    pub fn add_edge(&mut self, 
        start : Point, end : Point, 
        start_glyph : &str, end_glyph : &str, 
        start_glyph_scale : f32, end_glyph_scale : f32,
        start_tip : Option<&str>, end_tip : Option<&str>
    ){
        let start_arrow = start_tip.map(|tip| self.tip_map[tip]).unwrap_or_default();
        let end_arrow = end_tip.map(|tip| self.tip_map[tip]).unwrap_or_default();
        self.edge_instances.push(EdgeInstance {
            color : Vec4::new(0.0, 0.0, 0.0, 1.0),
            start_position : start,
            end_position : end,
            start_glyph : self.glyph_map[start_glyph],
            end_glyph : self.glyph_map[end_glyph],
            start_glyph_scale,
            end_glyph_scale,

            start_arrow,
            end_arrow
        })
    }

    
    fn set_buffer_data(&self){
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, self.attributes_buffer.as_ref());
        let u8_len = self.edge_instances.len() * std::mem::size_of::<EdgeInstance>();
        let u8_ptr = self.edge_instances.as_ptr() as *mut u8;
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
        log!("edge_instances : {:?}", self.edge_instances);
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.set_buffer_data();
        self.glyph_boundary_data.upload()?;
        self.arrow_tip_data.upload()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }


    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point){
        self.shader.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.shader.set_uniform_int("uGlyphDataTexture", 0);
        self.glyph_boundary_data.bind(WebGl2RenderingContext::TEXTURE0);
        self.shader.set_uniform_int("uArrowPathTexture", 1);
        self.arrow_tip_data.bind(WebGl2RenderingContext::TEXTURE1);
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            (6 + 2 * self.max_arrow_tip_num_vertices) as i32,
            1
        );
    }
}