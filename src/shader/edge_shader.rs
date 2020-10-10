use std::collections::{BTreeMap, btree_map};
use uuid::Uuid;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlTexture};

use lyon::geom::math::{Point, Angle, Vector};
use lyon::tessellation::{VertexBuffers};

#[allow(unused_imports)]
use crate::log;
use crate::vector::{Vec4};
use crate::shader::Program;
use crate::webgl_wrapper::WebGlWrapper;

use crate::glyph::{GlyphInstance};
use crate::arrow::Arrow;

use crate::shader::attributes::{Format, Type, NumChannels, Attribute, Attributes};
use crate::shader::data_texture::DataTexture;
use crate::shader::vertex_buffer::VertexBuffer;

use crate::coordinate_system::CoordinateSystem;



const DASH_PATTERN_TEXTURE_WIDTH : usize = 512;

const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aColor", 4, Type::F32), // color
    Attribute::new("aPositions", 4, Type::F32), // (start_position, end_position)
    Attribute::new("aGlyphScales_angle_thickness", 4, Type::F32), // (start_glyph_scale, end_glyph_scale, angle, thickness)

    Attribute::new("aStart", 4, Type::I16), // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aEnd", 4, Type::I16), // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aDashPattern", 4, Type::I16), // (dash_length, dash_index, dash_offset, dash_padding )
]);

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct EdgeOptions {
    start_tip : Option<Arrow>, 
    end_tip : Option<Arrow>,
    angle : Angle,
    thickness : f32,
    dash_pattern : Vec<u8>,
}

#[wasm_bindgen]
impl EdgeOptions {
    pub fn new() -> Self {
        Self {
            start_tip : None,
            end_tip : None,
            angle : Angle::zero(),
            thickness : 1.0,
            dash_pattern : vec![]
        }
    }

    pub fn set_tips(&mut self, arrow : Arrow) {
        self.start_tip = Some(arrow.clone());
        self.end_tip = Some(arrow);
    }

    pub fn set_start_tip(&mut self, arrow : Arrow) {
        self.start_tip = Some(arrow);
    }

    pub fn set_end_tip(&mut self, arrow : Arrow) {
        self.end_tip = Some(arrow);
    }

    pub fn no_tips(&mut self) {
        self.start_tip = None;
        self.end_tip = None;
    }

    pub fn no_start_tip(&mut self) {
        self.start_tip = None;
    }

    pub fn no_end_tip(&mut self) {
        self.end_tip = None;
    }

    pub fn set_bend_degrees(&mut self, degrees : f32) {
        self.angle = Angle::degrees(degrees);
    }

    pub fn set_thickness(&mut self, thickness : f32) {
        self.thickness = thickness;
    }

    pub fn set_dash_pattern(&mut self, dash_pattern : Vec<u8>) {
        self.dash_pattern = dash_pattern;
    }
}


#[derive(Clone, Copy, Debug)]
#[repr(C, align(4))]
struct EdgeInstance {
    color : Vec4,
    start_position : Point,
    end_position : Point,

    start_glyph_scale : f32,
    end_glyph_scale : f32,
    angle : f32,
    thickness : f32,
    
    start_glyph : u16,
    start_arrow : ArrowIndices,
    end_glyph : u16,
    end_arrow : ArrowIndices,

    dash_length : u16, 
    dash_index : u16, 
    dash_offset : u16, 
    dash_padding : u16,
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
    program : Program,
    
    edge_instances : VertexBuffer<EdgeInstance>,
    attribute_state : Option<WebGlVertexArrayObject>,
    
    tip_map : BTreeMap<Uuid, ArrowIndices>,
    max_arrow_tip_num_vertices : usize,
    arrow_header_data : DataTexture<ArrowHeader>,
    arrow_path_data : DataTexture<Point>,

    dash_data : Vec<u8>,
    dash_texture : Option<WebGlTexture>,
    dash_texture_num_rows : usize,
    dash_map : BTreeMap<Vec<u8>, (u16, u16)>,

    ready : bool,
}

impl EdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl.clone(), 
            include_str!("edge.vert"),
            include_str!("edge.frag")
        )?;
        let attribute_state = webgl.create_vertex_array();
        let edge_instances = VertexBuffer::new(webgl.clone());
        ATTRIBUTES.set_up_vertex_array(&webgl, &program.program, attribute_state.as_ref(), edge_instances.buffer.as_ref())?;

        let arrow_header_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let arrow_path_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));
        
        program.use_program();
        program.set_uniform_int("uGlyphBoundaryTexture", 0);
        program.set_uniform_int("uArrowHeaderTexture", 1);
        program.set_uniform_int("uArrowPathTexture", 2);
        program.set_uniform_int("uDashPatterns", 3);

        let dash_texture = webgl.create_texture();
        let mut dash_map = BTreeMap::new();
        dash_map.insert(vec![], (0, 0));

        Ok(Self {
            webgl,
            program,

            edge_instances,
            attribute_state,

            tip_map : BTreeMap::new(),
            arrow_header_data,
            arrow_path_data,
            max_arrow_tip_num_vertices : 0,
            
            dash_data : Vec::new(),
            dash_texture,
            dash_texture_num_rows : 0,
            dash_map,
            ready : false,
        })
    }

    fn dash_data(&mut self, dash_pattern : Vec<u8>) -> (u16, u16) {
        let entry = self.dash_map.entry(dash_pattern);
        match entry {
            btree_map::Entry::Occupied(oe) => *oe.get(),
            btree_map::Entry::Vacant(ve) => {
                let orig_dash_data_len = self.dash_data.len();
                let dash_pattern_row = orig_dash_data_len / DASH_PATTERN_TEXTURE_WIDTH;
                let dash_pattern = ve.key();
                for (i, &e) in dash_pattern.iter().enumerate() {
                    let value = if i%2 == 1 { 0 } else { 255 };
                    for _ in 0..e {
                        self.dash_data.extend(&[value]);
                    }
                }
                // If pattern has odd length, then double it up with its negation
                if dash_pattern.len() % 2 == 1 {
                    for (i, &e) in dash_pattern.iter().enumerate() {
                        let value = if i%2 == 1 { 255 } else { 0 };
                        for _ in 0..e {
                            self.dash_data.extend(&[value]);
                        }
                    }
                }
                self.dash_data.extend(&[255]);
                self.dash_data.resize_with(orig_dash_data_len +  DASH_PATTERN_TEXTURE_WIDTH, ||0);
                let pattern_len : u16 = dash_pattern.iter().map(|&b| b as u16).sum();
                *ve.insert((dash_pattern_row as u16, pattern_len))
            }
        }
    }

    fn ensure_dash_texture_size(&mut self){
        let num_rows = self.dash_data.len() / DASH_PATTERN_TEXTURE_WIDTH;
        if num_rows <= self.dash_texture_num_rows {
            return;
        }
        self.dash_texture_num_rows = num_rows;
        self.webgl.delete_texture(self.dash_texture.as_ref());
        self.dash_texture = self.webgl.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            WebGl2RenderingContext::R8,
            DASH_PATTERN_TEXTURE_WIDTH as i32, self.dash_texture_num_rows as i32
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::LINEAR as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::LINEAR as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
    }

    fn upload_dash_texture_data(&mut self) -> Result<(), JsValue>{
        self.ensure_dash_texture_size();
        let num_rows = self.dash_data.len() / DASH_PATTERN_TEXTURE_WIDTH;
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());
        unsafe {
            let array_view = js_sys::Uint8Array::view(&self.dash_data);
            self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D, 
                0, // mip level
                0, 0, // xoffset, yoffset: i32,
                DASH_PATTERN_TEXTURE_WIDTH as i32, num_rows as i32, // width, height
                WebGl2RenderingContext::RED, // format: u32,
                WebGl2RenderingContext::UNSIGNED_BYTE, // type_: u32,
                Some(&array_view) // pixels: Option<&Object>
            )?; 
        }
        Ok(())
    }

    pub fn clear(&mut self){
        self.max_arrow_tip_num_vertices = 0;
        self.tip_map.clear();
        self.edge_instances.clear();
        self.arrow_header_data.clear();
        self.arrow_path_data.clear();
        self.ready = false;
    }

    fn arrow_tip_data(&mut self, arrow : &Arrow) -> Result<ArrowIndices, JsValue> {
        let next_header_index = self.tip_map.len();
        let entry = self.tip_map.entry(arrow.uuid);
        match entry {
            btree_map::Entry::Occupied(oe) => Ok(*oe.get()),
            btree_map::Entry::Vacant(ve) => {
                let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
                arrow.tesselate_into_buffers(&mut buffers)?;

                let vertices_index = self.arrow_path_data.len();
                let num_vertices = buffers.indices.len();
                self.arrow_path_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
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
                    header_index : next_header_index as u16,
                    vertices_index : vertices_index as u16,
                };
                Ok(*ve.insert(arrow_indices))
            }
        }
    }


    pub fn add_edge(&mut self, 
        start : GlyphInstance, 
        end : GlyphInstance, 
        start_glyph_id : usize,
        end_glyph_id : usize,
        options : &EdgeOptions,
        // start_tip : Option<&Arrow>, end_tip : Option<&Arrow>,
        // angle : Angle,
        // thickness : f32,
        // dash_pattern : &[u8],
    ) -> Result<(), JsValue> {
        let start_arrow = options.start_tip.as_ref().map(|tip| self.arrow_tip_data(tip)).unwrap_or_else(|| Ok(Default::default()))?;
        let end_arrow = options.end_tip.as_ref().map(|tip| self.arrow_tip_data(tip)).unwrap_or_else(|| Ok(Default::default()))?;
        let start_glyph_idx = start_glyph_id as u16;
        let end_glyph_idx = end_glyph_id as u16;
        let (dash_index, dash_length) = self.dash_data(options.dash_pattern.to_vec());

        self.ready = false;
        self.edge_instances.push(EdgeInstance {
            color : Vec4::new(0.0, 0.0, 0.0, 1.0),
            start_position : start.center,
            end_position : end.center,
            start_glyph : start_glyph_idx,
            end_glyph : end_glyph_idx,
            start_glyph_scale : start.scale,
            end_glyph_scale : end.scale,
            angle : options.angle.radians,
            thickness : options.thickness,

            start_arrow,
            end_arrow,

            dash_length,
            dash_index,
            dash_offset : 0,
            dash_padding : 0,
        });
        Ok(())
    }

    fn prepare(&mut self) -> Result<(), JsValue> {
        if self.ready  {
            return Ok(());
        }
        self.edge_instances.prepare();

        if !self.dash_data.is_empty() {
            self.upload_dash_texture_data()?;
        }
        Ok(())
    }


    pub fn draw(&mut self, coordinate_system : CoordinateSystem, glyph_boundary_data : &mut DataTexture<Vector>) -> Result<(), JsValue> {
        if self.edge_instances.is_empty() {
            return Ok(());
        }
        self.program.use_program();
        glyph_boundary_data.bind(WebGl2RenderingContext::TEXTURE0)?;
        self.arrow_header_data.bind(WebGl2RenderingContext::TEXTURE1)?;
        self.arrow_path_data.bind(WebGl2RenderingContext::TEXTURE2)?;
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE3);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());

        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.prepare()?;

        
        self.program.set_uniform_transform("uTransformationMatrix", coordinate_system.transform);
        self.program.set_uniform_point("uOrigin", coordinate_system.origin);
        self.program.set_uniform_vector("uScale", coordinate_system.scale);
        self.program.set_uniform_float("uGlyphScale", coordinate_system.glyph_scale);
        
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            (12 + 2 * self.max_arrow_tip_num_vertices) as i32,
            self.edge_instances.len() as i32
        );
        Ok(())
    }
}