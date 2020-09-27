use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::log;
use crate::vector::{Vec4};
use crate::shader::{Shader};
use crate::webgl_wrapper::WebGlWrapper;

use crate::shader::attributes::{Type, Attribute, Attributes};

use lyon::geom::math::{Point, Vector, Transform};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlTexture};

use crate::convex_hull::ANGLE_RESOLUTION;


const DATA_ROW_SIZE : usize = 2048;



const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aColor", 4, Type::Float), // color
    Attribute::new("aStartPosition", 2, Type::Float), // start_position
    Attribute::new("aEndPosition", 2, Type::Float), // end_position
    Attribute::new("aStartGlyph", 1, Type::Short), // start_glyph
    Attribute::new("aEndGlyph", 1, Type::Short), // end_glyph
    Attribute::new("aStartGlyphScale", 1, Type::Float), // start_glyph_scale
    Attribute::new("aEndGlyphScale", 1, Type::Float), // end_glyph_scale

    Attribute::new("aStartArrowNumVertices", 1, Type::Short), 
    // Attribute::new("aStartArrowHeaderIndex", 1, Type::Short), 
    Attribute::new("aStartArrowVerticesIndex", 1, Type::Short),

    Attribute::new("aEndArrowNumVertices", 1, Type::Short), 
    // Attribute::new("aEndArrowHeaderIndex", 1, Type::Short), 
    Attribute::new("aEndArrowVerticesIndex", 1, Type::Short), 
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
    num_glyphs : usize,
    glyph_boundary_data : Vec<f32>,
    glyph_boundary_texture : Option<WebGlTexture>,
    glyph_boundary_texture_rows : usize, // This reminds us how big the texture currently is so we know whether we need to resize it.

    // Vertices has its length padded to a multiple of DATA_ROW_SIZE so that it will fit correctly into the data_texture
    // so we need to separately store the number of actually used entries separately.
    tip_map : BTreeMap<String, ArrowIndices>,
    arrow_tip_vertices : Vec<Point>,
    num_arrow_tip_vertices : usize,
    max_arrow_tip_num_vertices : usize,
    arrow_tip_path_texture : Option<WebGlTexture>,
    arrow_tip_path_texture_rows : usize, // This reminds us how big the texture currently is so we know whether we need to resize it.
    
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

        let glyph_boundary_texture = webgl.inner.create_texture();
        let arrow_tip_path_texture = webgl.inner.create_texture();

        Ok(Self {
            webgl,
            shader,

            attribute_state,
            attributes_buffer,

            glyph_map : BTreeMap::new(),
            num_glyphs : 0,
            glyph_boundary_data : Vec::new(),
            glyph_boundary_texture,
            glyph_boundary_texture_rows : 0,

            tip_map : BTreeMap::new(),
            arrow_tip_vertices : Vec::new(),
            num_arrow_tip_vertices : 0,
            max_arrow_tip_num_vertices : 0,
            arrow_tip_path_texture,
            arrow_tip_path_texture_rows : 0,
            

            edge_instances : Vec::new(),
        })
    }

    pub fn glyph_boundary_data(&mut self, glyph_name : String, boundary_data : &[Vector]){
        debug_assert!(boundary_data.len() == ANGLE_RESOLUTION);
        let start_idx = glyph_boundary_index(self.num_glyphs);
        let end_idx = glyph_boundary_index(self.num_glyphs + 1);
        while end_idx >= self.glyph_boundary_data.len() {
            self.glyph_boundary_data.extend_from_slice(&[0.0; DATA_ROW_SIZE * 4]);
        }
        log!("start_idx : {}, end_idx : {},  self.glyph_boundary_data.len() : {}", start_idx, end_idx,  self.glyph_boundary_data.len());
        log!("boundary_data.len() : {}, start_idx - end_idx : {}", boundary_data.len(), end_idx  - start_idx);
        self.glyph_boundary_data.splice(start_idx .. end_idx, boundary_data.iter().map(|v| v.length())).for_each(drop);
        self.glyph_map.insert(glyph_name, self.num_glyphs as u16);
        self.num_glyphs += 1;
    }

    pub fn arrow_tip_data(&mut self, tip_name : String, vertices : &[Point], indices : &[u16], index_offset : u16) {
        let vertices_index = self.num_arrow_tip_vertices;
        let num_vertices = indices.len();
        while self.num_arrow_tip_vertices + indices.len() >= self.arrow_tip_vertices.len() {
            self.arrow_tip_vertices.extend_from_slice(&[Point::new(0.0, 0.0); DATA_ROW_SIZE]);
        }
        self.arrow_tip_vertices.splice(self.num_arrow_tip_vertices .. self.num_arrow_tip_vertices + num_vertices, 
            indices.iter().map(|&i| vertices[(i - index_offset) as usize])
        ).for_each(drop);
        self.num_arrow_tip_vertices += num_vertices;
        // log!("glyph data : vertices: {:?}", &self.arrow_tip_vertices[0..self.num_vertices]);
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
        log!("self.edge_instances : {:?}", self.edge_instances);

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

    fn allocate_texture(&self, width : i32, height : i32, internalformat : u32) -> Option<WebGlTexture> {
        let texture = self.webgl.inner.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            internalformat, // internalformat:,
            width, height
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        texture
    }

    fn set_texture_data(&self, width : i32, height : i32, externalformat : u32, ty : u32, array_view : js_sys::Float32Array) -> Result<(), JsValue> {
        self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
            WebGl2RenderingContext::TEXTURE_2D, 
            0, // mip level
            0, 0, // xoffset, yoffset: i32,
            width, height, // width, height
            externalformat, // format: u32,
            ty, // type_: u32,
            Some(&array_view) // pixels: Option<&Object>
        )?; 
        Ok(())
    }

    fn ensure_glyph_texture_size(&mut self){
        let num_rows = (glyph_boundary_index(self.num_glyphs + 1) + DATA_ROW_SIZE - 1) / DATA_ROW_SIZE;
        log!("num_rows : {}, self.glyph_boundary_texture_rows : {}", num_rows, self.glyph_boundary_texture_rows);
        if num_rows <= self.glyph_boundary_texture_rows {
            return;
        }
        self.glyph_boundary_texture_rows = num_rows;
        self.webgl.delete_texture(self.glyph_boundary_texture.as_ref());
        self.glyph_boundary_texture = self.allocate_texture(DATA_ROW_SIZE as i32, num_rows as i32, WebGl2RenderingContext::RGBA32F);
    }

    fn set_glyph_texture_data(&self) -> Result<(), JsValue> {
        let num_rows = (glyph_boundary_index(self.num_glyphs + 1) + DATA_ROW_SIZE - 1) / DATA_ROW_SIZE;
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.glyph_boundary_texture.as_ref());
        unsafe {
            let array_view = js_sys::Float32Array::view(&self.glyph_boundary_data);
            self.set_texture_data(DATA_ROW_SIZE as i32, num_rows as i32, WebGl2RenderingContext::RGBA, WebGl2RenderingContext::FLOAT, array_view)?;
        }
        Ok(())
    }

    fn ensure_arrow_tip_path_texture_size(&mut self){
        let num_rows = self.arrow_tip_vertices.len() / DATA_ROW_SIZE;
        if num_rows <= self.arrow_tip_path_texture_rows {
            return;
        }
        self.webgl.delete_texture(self.arrow_tip_path_texture.as_ref());
        self.arrow_tip_path_texture = self.allocate_texture(DATA_ROW_SIZE as i32, num_rows as i32, WebGl2RenderingContext::RG32F);
    }

    fn set_arrow_tip_path_texture_data(&self) -> Result<(), JsValue> {
        let num_rows = self.arrow_tip_vertices.len() / DATA_ROW_SIZE;
        let len = self.arrow_tip_vertices.len() * std::mem::size_of::<Point>() / std::mem::size_of::<f32>();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.arrow_tip_path_texture.as_ref());
        unsafe {
            let array_view = js_sys::Float32Array::view_mut_raw(self.arrow_tip_vertices.as_ptr() as *mut f32, len);
            self.set_texture_data(DATA_ROW_SIZE as i32, num_rows as i32, WebGl2RenderingContext::RG, WebGl2RenderingContext::FLOAT, array_view)?;
        }
        Ok(())
    }

    pub fn prepare(&mut self) -> Result<(), JsValue> {
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.set_buffer_data();
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.ensure_glyph_texture_size();
        self.set_glyph_texture_data()?;
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE1);
        self.ensure_arrow_tip_path_texture_size();
        self.set_arrow_tip_path_texture_data()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }


    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point){
        self.shader.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.shader.set_uniform_int("uGlyphDataTexture", 0);
        self.shader.set_uniform_int("uArrowPathTexture", 1);
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