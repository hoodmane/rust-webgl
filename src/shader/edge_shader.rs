use std::collections::{BTreeMap, btree_map};
use uuid::Uuid;
use std::rc::Rc;

#[allow(unused_imports)]
use crate::log;
use crate::vector::{Vec4};
use crate::shader::Program;
use crate::webgl_wrapper::WebGlWrapper;

use crate::glyph::{GlyphInstance, Glyph};
use crate::arrow::Arrow;

use crate::shader::attributes::{Format, Type, NumChannels, Attribute, Attributes};
use crate::shader::data_texture::DataTexture;

use crate::coordinate_system::CoordinateSystem;

use lyon::geom::math::{Point, Angle};


use lyon::tessellation::{VertexBuffers};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlTexture};

const DASH_PATTERN_TEXTURE_WIDTH : usize = 512;

const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aColor", 4, Type::F32), // color
    Attribute::new("aPositions", 4, Type::F32), // (start_position, end_position)
    Attribute::new("aGlyphScales_angle_thickness", 4, Type::F32), // (start_glyph_scale, end_glyph_scale, angle, thickness)

    Attribute::new("aStart", 4, Type::I16), // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aEnd", 4, Type::I16), // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )
    Attribute::new("aDashPattern", 4, Type::I16), // (dash_length, dash_index, dash_offset, dash_padding )
]);


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
    
    edge_instances : Vec<EdgeInstance>,
    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,
    
    glyph_map : BTreeMap<Uuid, u16>,
    glyph_boundary_data : DataTexture<f32>,

    tip_map : BTreeMap<Uuid, ArrowIndices>,
    max_arrow_tip_num_vertices : usize,
    arrow_header_data : DataTexture<ArrowHeader>,
    arrow_path_data : DataTexture<Point>,

    dash_data : Vec<u8>,
    dash_texture : Option<WebGlTexture>,
    dash_texture_num_rows : usize,
    dash_map : BTreeMap<Vec<u8>, (u16, u16)>,
}

impl EdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let program = Program::new(
            webgl.clone(), 
            include_str!("edge.vert"),
            include_str!("edge.frag")
        )?;
        let attribute_state = webgl.create_vertex_array();
        let attributes_buffer = webgl.create_buffer();
        ATTRIBUTES.set_up_vertex_array(&webgl, &program.program, attribute_state.as_ref(), attributes_buffer.as_ref())?;

        let glyph_boundary_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
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

            edge_instances : Vec::new(),

            attribute_state,
            attributes_buffer,

            glyph_map : BTreeMap::new(),
            glyph_boundary_data,

            tip_map : BTreeMap::new(),
            arrow_header_data,
            arrow_path_data,
            max_arrow_tip_num_vertices : 0,
            
            dash_data : Vec::new(),
            dash_texture,
            dash_texture_num_rows : 0,
            dash_map,
        })
    }

    fn dash_data(&mut self, dash_pattern : Vec<u8>) -> (u16, u16) {
        let next_header_index = self.dash_map.len();
        let entry = self.dash_map.entry(dash_pattern);
        match entry {
            btree_map::Entry::Occupied(oe) => *oe.get(),
            btree_map::Entry::Vacant(ve) => {
                let orig_dash_data_len = self.dash_data.len();
                let dash_pattern = ve.key();
                for (i, &e) in dash_pattern.iter().enumerate() {
                    let value = if i%2 == 1 { 0 } else { 255 };
                    for _ in 0..e {
                        self.dash_data.extend(&[value]);
                    }
                }
                if dash_pattern.len() % 2 == 1 {
                    for (i, &e) in dash_pattern.iter().enumerate() {
                        let value = if i%2 == 1 { 255 } else { 0 };
                        for _ in 0..e {
                            self.dash_data.extend(&[value]);
                        }
                    }
                }
                let pattern_len = self.dash_data.len() - orig_dash_data_len;
                self.dash_data.extend(&[255]);
                self.dash_data.resize_with(orig_dash_data_len + DASH_PATTERN_TEXTURE_WIDTH, ||0);
                *ve.insert((next_header_index as u16, pattern_len as u16))
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
        log!("dash_data : {:?}", self.dash_data);
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
        self.glyph_map.clear();
        self.tip_map.clear();
        self.edge_instances.clear();
        self.glyph_boundary_data.clear();
        self.arrow_header_data.clear();
        self.arrow_path_data.clear();
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

    fn glyph_boundary_data(&mut self, glyph : &Rc<Glyph>) -> u16 {
        let next_glyph_index = self.glyph_map.len();
        let entry = self.glyph_map.entry(glyph.uuid);
        match entry {
            btree_map::Entry::Occupied(oe) => *oe.get(),
            btree_map::Entry::Vacant(ve) => {
                self.glyph_boundary_data.append(glyph.boundary().iter().map(|v| v.length()));
                *ve.insert(next_glyph_index as u16)
            }
        }
    }



    pub fn add_edge(&mut self, 
        start : GlyphInstance, 
        end : GlyphInstance, 
        start_tip : Option<&Arrow>, end_tip : Option<&Arrow>,
        angle : Angle,
        thickness : f32,
        dash_pattern : &[u8],
    ) -> Result<(), JsValue> {
        let start_arrow = start_tip.map(|tip| self.arrow_tip_data(tip)).unwrap_or(Ok(Default::default()))?;
        let end_arrow = end_tip.map(|tip| self.arrow_tip_data(tip)).unwrap_or(Ok(Default::default()))?;
        let start_glyph_idx = self.glyph_boundary_data(&start.glyph);
        let end_glyph_idx = self.glyph_boundary_data(&end.glyph);
        let (dash_index, dash_length) = self.dash_data(dash_pattern.to_vec());

        log!("dash_length : {}, dash_index : {}", dash_length, dash_index);

        self.edge_instances.push(EdgeInstance {
            color : Vec4::new(0.0, 0.0, 0.0, 1.0),
            start_position : start.center,
            end_position : end.center,
            start_glyph : start_glyph_idx,
            end_glyph : end_glyph_idx,
            start_glyph_scale : start.scale,
            end_glyph_scale : end.scale,
            angle : angle.radians,
            thickness,

            start_arrow,
            end_arrow,

            dash_length,
            dash_index,
            dash_offset : 0,
            dash_padding : 0,
        });
        Ok(())
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
        self.program.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.set_buffer_data();

        self.glyph_boundary_data.upload()?;
        self.arrow_header_data.upload()?;
        self.arrow_path_data.upload()?;
        self.upload_dash_texture_data()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }


    pub fn draw(&mut self, coordinate_system : CoordinateSystem){
        self.program.use_program();
        self.glyph_boundary_data.bind(WebGl2RenderingContext::TEXTURE0);
        self.arrow_header_data.bind(WebGl2RenderingContext::TEXTURE1);
        self.arrow_path_data.bind(WebGl2RenderingContext::TEXTURE2);
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE3);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.dash_texture.as_ref());

        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.program.set_uniform_transform("uTransformationMatrix", coordinate_system.transform);
        self.program.set_uniform_point("uOrigin", coordinate_system.origin);
        self.program.set_uniform_vector("uScale", coordinate_system.scale);
        
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            (12 + 2 * self.max_arrow_tip_num_vertices) as i32,
            self.edge_instances.len() as i32
        );
    }
}