use std::convert::TryInto;
use std::collections::BTreeMap;

use wasm_bindgen::JsValue;
use web_sys::{
    WebGl2RenderingContext, 
    WebGlBuffer, 
    WebGlProgram, 
    WebGlShader,
    WebGlVertexArrayObject,
    WebGlTexture
};

use lyon::geom::math::{Point, Transform};

use crate::log;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::Shader;
use crate::vector::Vec4;


const DATA_ROW_SIZE : usize = 2048;


const POSITION_SIZE : i32 = 2;
const COLOR_SIZE : i32  = 4;
const NUM_VERTICES_SIZE : i32  = 1;
const DATA_INDEX_SIZE : i32  = 1;


const POSITION_TYPE : u32 = WebGl2RenderingContext::FLOAT;
const COLOR_TYPE : u32 = WebGl2RenderingContext::FLOAT;
const NUM_VERTICES_TYPE : u32 = WebGl2RenderingContext::SHORT;
const DATA_INDEX_TYPE : u32 = WebGl2RenderingContext::SHORT;

const POSITION_BYTES : i32 = POSITION_SIZE * std::mem::size_of::<f32>() as i32;
const COLOR_BYTES : i32 = COLOR_SIZE * std::mem::size_of::<f32>() as i32;
const NUM_VERTICES_BYTES : i32 = NUM_VERTICES_SIZE * std::mem::size_of::<u16>() as i32;
const DATA_INDEX_BYTES : i32 = DATA_INDEX_SIZE * std::mem::size_of::<u16>() as i32;

const POSITION_OFFSET : i32 = 0;
const COLOR_OFFSET : i32 = POSITION_OFFSET + POSITION_BYTES;
const NUM_VERTICES_OFFSET : i32 = COLOR_OFFSET + COLOR_BYTES;
const DATA_INDEX_OFFSET : i32 = NUM_VERTICES_OFFSET + NUM_VERTICES_BYTES;
const STRIDE : i32 = DATA_INDEX_OFFSET + DATA_INDEX_BYTES;


pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub shader : Shader,
    glyph_map : BTreeMap<String, (u16, u16)>,

    // Vertices has its length padded to a multiple of DATA_ROW_SIZE so that it will fit correctly into the data_texture
    // so we need to separately store the number of actually used entries separately.
    vertices : Vec<Point>,
    num_vertices : usize,

    max_glyph_num_vertices : usize,
    glyph_instances : Vec<GlyphInstance>,

    
    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,

    data_texture : Option<WebGlTexture>,
    texture_rows : usize, // This reminds us how big the texture currently is so we know whether we need to resize it.
}

// #[derive(Debug)]
// #[repr(C)]
// struct Test {
//     pt : Point,
//     integer : i16,
// }

#[derive(Debug)]
struct GlyphInstance {
    position : Point,
    color : Vec4,
    num_vertices : u16,
    index : u16,
}

impl GlyphShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let shader = Shader::new(
            webgl.clone(), 
            r#"#version 300 es
                uniform mat3x2 uTransformationMatrix;
                uniform vec2 uOrigin;
                uniform vec2 uScale;
                uniform sampler2D uGlyphDataTexture;

                in vec2 aPosition;
                in vec4 aColor;
                in int aGlyphDataIndex;
                in int aGlyphNumVertices;

                out vec4 fColor;

                vec2 testPositions[6] = vec2[](
                    vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(0.5, 0.5),
                    vec2(-0.5, -0.5), vec2(-0.5, 0.5), vec2(0.5, 0.5)
                );

                vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
                    int texWidth = textureSize(tex, 0).x;
                    int col = index % texWidth;
                    int row = index / texWidth;
                    return texelFetch(tex, ivec2(col, row), 0);
                }                
                
                void main() {
                    // int aGlyphDataIndex = 0;
                    int vertexIdx = 0 + gl_VertexID;
                    vec2 vertexPosition = getValueByIndexFromTexture(uGlyphDataTexture, vertexIdx).xy;
                    
                    vec2 transformedPosition = uOrigin +  uScale * aPosition;
                    gl_Position = vec4(uTransformationMatrix * vec3(transformedPosition + vertexPosition, 1.0), 0.0, 1.0);

                    float blue_channel;
                    if(aGlyphDataIndex == 0){
                        blue_channel = 0.0;
                    } else {
                        blue_channel = 1.0;
                    }

                    blue_channel = float(aGlyphNumVertices);
                    
                    fColor = aColor;
                    fColor = vec4(aPosition/1000.0, blue_channel/255.0, 1.0);

                }
            "#,
            r#"#version 300 es
                precision highp float;
                /*flat*/ in vec4 fColor;
                out vec4 outColor;
                void main() {
                    outColor = fColor;
                }
            "#
        )?;

        let attribute_state = webgl.create_vertex_array();
        webgl.bind_vertex_array(attribute_state.as_ref());

        let attributes_buffer = webgl.create_buffer();

        // IMPORTANT: Must bind_buffer here!!!!
        // vertex_attrib_pointer uses the current bound buffer implicitly.
        webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, attributes_buffer.as_ref());


        // let position_loc : u32 = webgl.get_attrib_location(&shader.program, "aPosition").try_into().unwrap();
        // let stride = std::mem::size_of::<Test>() as i32;
        // log!("stride=10? {}", stride);
        // webgl.vertex_attrib_pointer_with_i32(position_loc, 2, WebGl2RenderingContext::FLOAT, false, stride, 0);
        // webgl.enable_vertex_attrib_array(position_loc);
        // webgl.vertex_attrib_divisor(position_loc, 1);


        // let test_loc : u32 = webgl.get_attrib_location(&shader.program, "aTest").try_into().unwrap();
        // webgl.vertex_attrib_i_pointer_with_i32(test_loc, 1, WebGl2RenderingContext::SHORT, stride, 8);
        // webgl.enable_vertex_attrib_array(test_loc);
        // webgl.vertex_attrib_divisor(test_loc, 1);
        // webgl.bind_vertex_array(None);


        let position_loc : u32 = webgl.get_attrib_location(&shader.program, "aPosition").try_into().map_err(|_| "aPosition")?;
        let color_loc : u32  = webgl.get_attrib_location(&shader.program, "aColor").try_into().map_err(|_| "aColor")?;
        let num_vertices_loc : u32  = webgl.get_attrib_location(&shader.program, "aGlyphNumVertices").try_into().map_err(|_| "aGlyphNumVertices")?;
        let data_index_loc : u32  = webgl.get_attrib_location(&shader.program, "aGlyphDataIndex").try_into().map_err(|_| "aGlyphDataIndex")?;

        webgl.enable_vertex_attrib_array(position_loc);
        webgl.enable_vertex_attrib_array(color_loc);
        webgl.enable_vertex_attrib_array(num_vertices_loc);
        webgl.enable_vertex_attrib_array(data_index_loc);

        webgl.vertex_attrib_pointer_with_i32(position_loc, POSITION_SIZE, POSITION_TYPE, false, 0, 0);
        webgl.vertex_attrib_pointer_with_i32(position_loc, POSITION_SIZE, POSITION_TYPE, false, STRIDE, POSITION_OFFSET);
        webgl.vertex_attrib_pointer_with_i32(color_loc, COLOR_SIZE, COLOR_TYPE, false, STRIDE, COLOR_OFFSET);
        webgl.vertex_attrib_i_pointer_with_i32(num_vertices_loc, NUM_VERTICES_SIZE, NUM_VERTICES_TYPE, STRIDE, NUM_VERTICES_OFFSET);
        webgl.vertex_attrib_i_pointer_with_i32(data_index_loc, DATA_INDEX_SIZE, DATA_INDEX_TYPE, STRIDE, DATA_INDEX_OFFSET);
        
        webgl.vertex_attrib_divisor(position_loc, 1);
        webgl.vertex_attrib_divisor(color_loc, 1);
        webgl.vertex_attrib_divisor(num_vertices_loc, 1);
        webgl.vertex_attrib_divisor(data_index_loc, 1);


        let data_texture = webgl.inner.create_texture();


        Ok(Self {
            webgl,
            shader,
            glyph_map : BTreeMap::new(),
            vertices : Vec::new(),
            num_vertices : 0,

            glyph_instances : Vec::new(), 
            max_glyph_num_vertices : 0,
            
            attribute_state,
            attributes_buffer,

            data_texture,
            texture_rows : 0
        })
    }

    pub fn clear_glyphs(&mut self, ){
        self.max_glyph_num_vertices = 0;
        self.num_vertices = 0;
        self.vertices.clear();
        self.glyph_map.clear();
        self.glyph_instances.clear();
    }

    pub fn glyph_data(&mut self, glyph_name : String, vertices : &[Point], indices : &[u16], index_offset : u16){
        let glyph_index = self.num_vertices;
        let glyph_num_vertices = indices.len();
        if self.num_vertices + glyph_num_vertices > self.vertices.len() {
            self.vertices.extend_from_slice(&[Point::new(0.0, 0.0); DATA_ROW_SIZE]);
        }
        for &i in indices {
            self.vertices[self.num_vertices] = vertices[(i - index_offset) as usize];
            self.num_vertices += 1;
        }
        log!("glyph data : vertices: {:?}", &self.vertices[0..self.num_vertices]);
        self.max_glyph_num_vertices = self.max_glyph_num_vertices.max(glyph_num_vertices);
        self.glyph_map.insert(glyph_name, (glyph_index.try_into().unwrap(), glyph_num_vertices.try_into().unwrap()));
    }

    pub fn add_glyph(&mut self, glyph_name : &str, position : Point, color : Vec4) {
        let (index, num_vertices) = self.glyph_map[glyph_name];
        // log!("add_glyph::: index : {}, num_vertices : {}", index, num_vertices);
        self.glyph_instances.push(GlyphInstance {
            position,
            color, 
            index,
            num_vertices
        });
    }

    fn set_buffer_data(&self){
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, self.attributes_buffer.as_ref());
        let u8_len = self.glyph_instances.len() * std::mem::size_of::<GlyphInstance>();
        // let u8_len = std::mem::size_of::<GlyphInstance>();
        // let mut temp : [GlyphInstance; 1] = 
        // [GlyphInstance { 
        //     position : Point::new(400.0, 200.0),
        //     color : Vec4::new(0.0, 0.0, 0.0, 1.0),
        //     index : 10,
        //     num_vertices : 180,
        // }];
        log!("self.glyph_instances : {:?}", self.glyph_instances);

        let u8_ptr = self.glyph_instances.as_ptr() as *mut u8;
        unsafe {
            let vert_array = js_sys::Uint8Array::view_mut_raw(u8_ptr, u8_len);
            // let vert_array = js_sys::Float32Array::view(&temp);
            crate::console_log::log_1(&vert_array);
            self.webgl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
    }

    fn ensure_texture_size(&mut self){
        let num_rows = self.vertices.len() / DATA_ROW_SIZE;
        if num_rows <= self.texture_rows {
            return;
        }
        self.texture_rows = num_rows;
        self.webgl.delete_texture(self.data_texture.as_ref());
        self.data_texture = self.webgl.inner.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.data_texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            WebGl2RenderingContext::RG32F, // internalformat:,
            DATA_ROW_SIZE as i32, num_rows as i32
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
    }

    fn set_texture_data(&self) -> Result<(), JsValue> {
        let num_rows = self.vertices.len() / DATA_ROW_SIZE;
        let len = self.vertices.len() * std::mem::size_of::<Point>() / std::mem::size_of::<f32>();
        // log!("set_texture_data::: vertices.len() : {}, num_rows : {}, len : {}", self.vertices.len(), num_rows, len);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.data_texture.as_ref());
        log!("len : {}, num_rows : {}, DATA_ROW_SIZE * num_rows : {}", len, num_rows, DATA_ROW_SIZE * num_rows);
        unsafe {
            let array_view = js_sys::Float32Array::view_mut_raw(self.vertices.as_ptr() as *mut f32, len);
            self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D, 
                0, // mip level
                0, 0, // xoffset, yoffset: i32,
                DATA_ROW_SIZE as i32, num_rows as i32, // width, height
                WebGl2RenderingContext::RG, // format: u32,
                WebGl2RenderingContext::FLOAT, // type_: u32,
                Some(&array_view) // pixels: Option<&Object>
            )?; 
        }
        Ok(())
    }

    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_int("uGlyphDataTexture", 0);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.set_buffer_data();
        self.ensure_texture_size();
        self.set_texture_data()?;
        let num_instances = self.glyph_instances.len() as i32;
        let num_vertices = self.max_glyph_num_vertices as i32;
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