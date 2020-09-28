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

use crate::shader::attributes::{Format, Type, NumChannels,  Attribute, Attributes};
use crate::shader::data_texture::DataTexture;


const ATTRIBUTES : Attributes = Attributes::new(&[
    Attribute::new("aPosition", 2, Type::F32),
    Attribute::new("aColor", 4, Type::F32),
    Attribute::new("aGlyphNumVertices", 1, Type::I16), 
    Attribute::new("aGlyphDataIndex", 1, Type::I16),
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
    max_glyph_num_vertices : usize,
    vertices_data : DataTexture<Point>
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

        let vertices_data = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Two));

        Ok(Self {
            webgl,
            shader,
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

    pub fn glyph_data(&mut self, glyph_name : String, vertices : &[Point], indices : &[u16], index_offset : u16){
        let glyph_index = self.vertices_data.len();
        let glyph_num_vertices = indices.len();
        self.vertices_data.insert(indices.iter().map(|&i| vertices[(i - index_offset) as usize]));
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


    pub fn prepare(&mut self) -> Result<(), JsValue> {
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.set_buffer_data();
        self.vertices_data.upload()?;
        self.webgl.bind_vertex_array(None);
        Ok(())
    }

    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_int("uGlyphDataTexture", 0);
        self.vertices_data.bind(WebGl2RenderingContext::TEXTURE0);
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);

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