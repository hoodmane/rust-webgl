use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::log;
use crate::vector::{Vec4};
use crate::shader::{Shader};
use crate::webgl_wrapper::WebGlWrapper;

use crate::arrow::Arrow;

use crate::shader::attributes::{Format, Type, NumChannels, Attribute, Attributes};
use crate::shader::data_texture::DataTexture;

use lyon::geom::math::{Point, Vector, Angle, Transform};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlTexture};

use crate::convex_hull::ANGLE_RESOLUTION;


const DATA_ROW_SIZE : usize = 2048;



const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aColor", 4, Type::F32), // color
    Attribute::new("aStartPosition", 4, Type::F32), // (start_position, start_tangent)
    Attribute::new("aEndPosition", 4, Type::F32), // (end_position, end_tangent)
    Attribute::new("aGlyphScales_angle_thickness", 4, Type::F32), // (start_glyph_scale, end_glyph_scale, angle, thickness)

    Attribute::new("aStart", 4, Type::I16), // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aEnd", 4, Type::I16), // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )
]);



#[derive(Clone, Copy, Debug)]
#[repr(C, align(4))]
struct EdgeInstance {
    color : Vec4,
    start_position : Point,
    start_tangent : Vector,
    end_position : Point,
    end_tangent : Vector,

    start_glyph_scale : f32,
    end_glyph_scale : f32,
    angle : f32,
    thickness : f32,
    
    start_glyph : u16,
    start_arrow : ArrowIndices,
    end_glyph : u16,
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
    header_index : u16,
    vertices_index : u16
}

pub struct EdgeShader {
    webgl : WebGlWrapper,
    shader : Shader,
    
    edge_instances : Vec<EdgeInstance>,


    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,
    
    glyph_map : BTreeMap<String, u16>,
    glyph_boundary_data : DataTexture<f32>,

    tip_map : BTreeMap<String, ArrowIndices>,
    max_arrow_tip_num_vertices : usize,
    arrow_header_data : DataTexture<ArrowHeader>,
    arrow_vertices_data : DataTexture<Point>,
}

fn glyph_boundary_index(glyph_index : usize) -> usize {
    glyph_index * ANGLE_RESOLUTION
}


impl EdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let shader = Shader::new(
            webgl.clone(), 
            include_str!("edge.vert"),
            include_str!("edge.frag")
        )?;
        let attribute_state = webgl.create_vertex_array();
        let attributes_buffer = webgl.create_buffer();
        ATTRIBUTES.set_up_vertex_array(&webgl, &shader, attribute_state.as_ref(), attributes_buffer.as_ref())?;

        let glyph_boundary_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let arrow_header_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let arrow_vertices_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));
        Ok(Self {
            webgl,
            shader,

            attribute_state,
            attributes_buffer,

            glyph_map : BTreeMap::new(),
            glyph_boundary_data,

            tip_map : BTreeMap::new(),
            arrow_header_data,
            arrow_vertices_data,
            max_arrow_tip_num_vertices : 0,
            

            edge_instances : Vec::new(),
        })
    }

    pub fn glyph_boundary_data(&mut self, glyph_name : String, boundary_data : &[Vector]){
        debug_assert!(boundary_data.len() == ANGLE_RESOLUTION);
        self.glyph_map.insert(glyph_name, self.glyph_map.len() as u16);
        self.glyph_boundary_data.append(boundary_data.iter().map(|v| v.length()));
    }

    pub fn arrow_tip_data(&mut self, tip_name : String, arrow : &Arrow, vertices : &[Point], indices : &[u16], index_offset : u16) {
        let vertices_index = self.arrow_vertices_data.len();
        let num_vertices = indices.len();
        self.arrow_vertices_data.append(indices.iter().map(|&i| vertices[(i - index_offset) as usize]));
        self.arrow_header_data.append([ArrowHeader {     
            tip_end : arrow.tip_end,
            back_end : arrow.back_end,
            visual_tip_end : arrow.visual_tip_end,
            visual_back_end : arrow.visual_back_end,
            line_end : arrow.line_end, 
        }].iter().cloned());
        self.max_arrow_tip_num_vertices = self.max_arrow_tip_num_vertices.max(num_vertices);
        let arrow_indices = ArrowIndices {
            num_vertices : num_vertices as u16,
            header_index : self.tip_map.len() as u16,
            vertices_index : vertices_index as u16,
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
        let angle = Angle::degrees(90.0);
        let segment_angle = (end - start).angle_from_x_axis();
        let start_tangent = Vector::from_angle_and_length(segment_angle - angle, 1.0);
        let end_tangent = Vector::from_angle_and_length(segment_angle + angle, 1.0);
        self.edge_instances.push(EdgeInstance {
            color : Vec4::new(0.0, 0.0, 0.0, 1.0),
            start_position : start,
            start_tangent,
            end_position : end,
            end_tangent,
            start_glyph : self.glyph_map[start_glyph],
            end_glyph : self.glyph_map[end_glyph],
            start_glyph_scale,
            end_glyph_scale,
            angle : angle.radians,
            thickness : 5.0,

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
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.set_buffer_data();
        self.glyph_boundary_data.upload()?;
        self.arrow_header_data.upload()?;
        self.arrow_vertices_data.upload()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }


    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point){
        log!("origin : {:?}, scale : {:?}", origin, scale);
        self.shader.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        
        self.shader.set_uniform_int("uGlyphBoundaryTexture", 0);
        self.glyph_boundary_data.bind(WebGl2RenderingContext::TEXTURE0);
        
        self.shader.set_uniform_int("uArrowHeaderTexture", 1);
        self.arrow_header_data.bind(WebGl2RenderingContext::TEXTURE1);

        self.shader.set_uniform_int("uArrowPathTexture", 2);
        self.arrow_vertices_data.bind(WebGl2RenderingContext::TEXTURE2);

        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            (12 + 2 * self.max_arrow_tip_num_vertices) as i32,
            self.edge_instances.len() as i32
        );
    }
}