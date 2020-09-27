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

use crate::shader::attributes::{Type, Attribute, Attributes};


const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aPosition", 2, Type::Float),
    Attribute::new("aColor", 4, Type::Float),
    Attribute::new("aGlyphNumVertices", 1, Type::Short), 
    Attribute::new("aGlyphDataIndex", 1, Type::Short),
]);



const DATA_ROW_SIZE : usize = 2048;



pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub shader : Shader,
    glyph_map : BTreeMap<String, (u16, u16)>,

    glyph_instances : Vec<GlyphInstance>,

    
    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,

    // Vertices has its length padded to a multiple of DATA_ROW_SIZE so that it will fit correctly into the data_texture
    // so we need to separately store the number of actually used entries separately.
    vertices : Vec<Point>,
    num_vertices : usize,
    max_glyph_num_vertices : usize,


    data_texture : Option<WebGlTexture>,
    texture_rows : usize, // This reminds us how big the texture currently is so we know whether we need to resize it.
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
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

                flat out vec4 fColor;

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
                    vec2 vertexPosition;
                    if(gl_VertexID < aGlyphNumVertices) {
                        int vertexIdx = aGlyphDataIndex + gl_VertexID;
                        vertexPosition = getValueByIndexFromTexture(uGlyphDataTexture, vertexIdx).xy;
                    } else {
                        vertexPosition = vec2(0.0, 0.0); // degenerate vertex
                    }
                    vec2 transformedPosition = uOrigin +  uScale * aPosition;
                    gl_Position = vec4(uTransformationMatrix * vec3(transformedPosition + vertexPosition, 1.0), 0.0, 1.0);
                    fColor = aColor;
                }
            "#,
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
        while self.num_vertices + indices.len() >= self.vertices.len() {
            self.vertices.extend_from_slice(&[Point::new(0.0, 0.0); DATA_ROW_SIZE]);
        }
        self.vertices.splice(self.num_vertices .. self.num_vertices + glyph_num_vertices, 
            indices.iter().map(|&i| vertices[(i - index_offset) as usize])
        ).for_each(drop);
        self.num_vertices += glyph_num_vertices;
        log!("glyph data : vertices: {:?}", &self.vertices[0..self.num_vertices]);
        self.max_glyph_num_vertices = self.max_glyph_num_vertices.max(glyph_num_vertices);
        self.glyph_map.insert(glyph_name, (glyph_index.try_into().unwrap(), glyph_num_vertices.try_into().unwrap()));
    }

    pub fn add_glyph(&mut self, glyph_name : &str, position : Point, color : Vec4) {
        let (index, num_vertices) = self.glyph_map[glyph_name];
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

    pub fn prepare(&mut self) -> Result<(), JsValue> {
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.set_buffer_data();
        self.ensure_texture_size();
        self.set_texture_data()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }

    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_int("uGlyphDataTexture", 0);
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.data_texture.as_ref());

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