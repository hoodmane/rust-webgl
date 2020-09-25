use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::log;
use crate::vector::{Vec4};
use crate::shader::{Shader};
use crate::webgl_wrapper::WebGlWrapper;

use lyon::geom::math::{Point, Vector, Transform};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlVertexArrayObject, WebGlBuffer, WebGlTexture};

use crate::convex_hull::ANGLE_RESOLUTION;


const DATA_ROW_SIZE : usize = 2048 * 4;

#[derive(Copy, Clone, Debug)]
enum Type {
    Float,
    Short
}

fn size_of_type(ty : Type) -> i32 {
    match ty {
        Type::Float => std::mem::size_of::<f32>() as i32,
        Type::Short => std::mem::size_of::<u16>() as i32
    }
}

fn webgl_enum_of_type(ty : Type) -> u32 {
    match ty {
        Type::Float => WebGl2RenderingContext::FLOAT,
        Type::Short => WebGl2RenderingContext::SHORT
    }
}

const ATTRIBUTES : [(&str, i32, Type); 7] = [
    ("aColor", 4, Type::Float), // color
    ("aStartPosition", 2, Type::Float), // start_position
    ("aEndPosition", 2, Type::Float), // end_position
    ("aStartGlyph", 1, Type::Short), // start_glyph
    ("aEndGlyph", 1, Type::Short), // end_glyph
    ("aStartGlyphScale", 1, Type::Float), // start_glyph_scale
    ("aEndGlyphScale", 1, Type::Float), // end_glyph_scale
];

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct EdgeInstance {
    color : Vec4,
    start_position : Point,
    end_position : Point,
    start_glyph : u16,
    end_glyph : u16,
    start_glyph_scale : f32,
    end_glyph_scale : f32,
}


fn attribute_offset(idx : usize) -> i32 {
    ATTRIBUTES[..idx].iter().map(|&(_, size, ty)|
        size_of_type(ty) * size
    ).sum()
}

fn attribute_stride() -> i32 {
    attribute_offset(ATTRIBUTES.len())
}

fn set_up_attributes(attribute_state : Option<&WebGlVertexArrayObject>, attributes_buffer : Option<&WebGlBuffer>, webgl : &WebGlWrapper, shader : &Shader) -> Result<(), JsValue> {
    webgl.bind_vertex_array(attribute_state);
    // IMPORTANT: Must bind_buffer here!!!!
    // vertex_attrib_pointer uses the current bound buffer implicitly.
    webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, attributes_buffer);

    let stride = attribute_stride();
    for (idx, (name, size, ty)) in ATTRIBUTES.iter().enumerate() {
        let size = *size;
        let ty = *ty;
        let loc = webgl.get_attrib_location(&shader.program, name).try_into().map_err(|_| name.to_string())?;
        let offset = attribute_offset(idx);
        webgl.enable_vertex_attrib_array(loc);
        match ty {
            Type::Float => {webgl.vertex_attrib_pointer_with_i32(loc, size, webgl_enum_of_type(ty), false, stride, offset)},
            Type::Short => {webgl.vertex_attrib_i_pointer_with_i32(loc, size, webgl_enum_of_type(ty), stride, offset)}
        };
        webgl.vertex_attrib_divisor(loc, 1);
    }
    webgl.bind_vertex_array(None);
    Ok(())
}





pub struct TestEdgeShader {
    webgl : WebGlWrapper,
    shader : Shader,
    glyph_map : BTreeMap<String, u16>,
    
    attribute_state : Option<WebGlVertexArrayObject>,
    attributes_buffer : Option<WebGlBuffer>,

    num_glyphs : usize,
    glyph_boundary_data : Vec<f32>,
    glyph_boundary_texture : Option<WebGlTexture>,
    glyph_boundary_texture_rows : usize, // This reminds us how big the texture currently is so we know whether we need to resize it.
    
    edge_instances : Vec<EdgeInstance>,
}

fn glyph_boundary_index(glyph_index : usize) -> usize {
    glyph_index * ANGLE_RESOLUTION
}


impl TestEdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let shader = Shader::new(
            webgl.clone(), 
            r#"#version 300 es
                #define M_PI 3.1415926535897932384626433832795
                #define ANGLE_RES 180 // should be same as ANGLE_RESOLUTION
                uniform mat3x2 uTransformationMatrix;
                uniform vec2 uOrigin;
                uniform vec2 uScale;
                uniform sampler2D uGlyphDataTexture;

                in vec4 aColor;
                in vec2 aStartPosition;
                in vec2 aEndPosition;
                in int aStartGlyph;
                in int aEndGlyph;
                in float aStartGlyphScale;
                in float aEndGlyphScale;

                out vec4 fColor;


                ivec2 vertexIndexes[6] = ivec2[](
                    ivec2(0, 0), ivec2(0, 1), ivec2(1, 0),
                    ivec2(0, 1), ivec2(1, 0), ivec2(1, 1)
                );

                vec2 testPositions[3] = vec2[](
                    vec2(0.8, 1.34641), vec2(2.2, 1.34641), vec2(1.5, 0.133975)
                );

                float getValueByIndexFromTexture(sampler2D tex, int index) {
                    int texWidth = textureSize(tex, 0).x;
                    int channel = index % 4;
                    int texOffset = index / 4
                    int col = texOffset % texWidth;
                    int row = texOffset / texWidth;
                    return texelFetch(tex, ivec2(col, row), 0)[channel];
                }

                float getGlyphBoundaryPoint(sampler2D tex, int glyph, float angle){
                    int index = (int(angle / (2.0 * M_PI) * float(ANGLE_RES)) + ANGLE_RES) % ANGLE_RES;
                    int texOffset = ANGLE_RES * glyph + pixelOffset;
                    vec4 pixel = getValueByIndexFromTexture(tex, texOffset);
                    return pixel[pixelChannel];
                }
                
                void main() {
                    vec2 transformedStart = uOrigin +  uScale * aStartPosition;
                    vec2 transformedEnd = uOrigin +  uScale * aEndPosition;

                    vec2 displacement = normalize(transformedStart - transformedEnd);
                    float angle = atan(displacement.y, displacement.x);
                    float startOffset = aStartGlyphScale * getGlyphBoundaryPoint(uGlyphDataTexture, aStartGlyph, angle);
                    float endOffset = aEndGlyphScale * getGlyphBoundaryPoint(uGlyphDataTexture, aEndGlyph, angle + M_PI);

                    vec2 startVec = transformedStart - startOffset * displacement;
                    vec2 endVec = transformedEnd + startOffset * displacement;

                    vec2 normal = vec2(-displacement.y, displacement.x);

                    ivec2 vertexIndex = vertexIndexes[gl_VertexID];

                    if(vertexIndex.x == 1){
                        normal = - normal;
                    }

                    vec2 position;
                    if(vertexIndex.y == 0){
                        position = startVec + normal;
                    } else {
                        position = endVec + normal;
                    }

                    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
                    fColor = aColor;
                    if(aEndGlyph == 1){
                        fColor.r = 1.0;
                    }
                }
            "#,
            r#"#version 300 es
                precision highp float;
                in vec4 fColor;
                out vec4 outColor;

                void main() {
                    outColor = fColor;
                }
            "#
        )?;
        let attribute_state = webgl.create_vertex_array();
        let attributes_buffer = webgl.create_buffer();
        set_up_attributes(attribute_state.as_ref(), attributes_buffer.as_ref(), &webgl, &shader)?;

        let glyph_boundary_texture = webgl.inner.create_texture();

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

            edge_instances : Vec::new(),
        })
    }

    pub fn glyph_boundary_data(&mut self, glyph_name : String, boundary_data : &[Vector]){
        debug_assert!(boundary_data.len() == ANGLE_RESOLUTION);
        let start_idx = glyph_boundary_index(self.num_glyphs);
        let end_idx = glyph_boundary_index(self.num_glyphs + 1);
        while end_idx >= self.glyph_boundary_data.len() {
            self.glyph_boundary_data.extend_from_slice(&[0.0; DATA_ROW_SIZE]);
        }
        log!("start_idx : {}, end_idx : {},  self.glyph_boundary_data.len() : {}", start_idx, end_idx,  self.glyph_boundary_data.len());
        log!("boundary_data.len() : {}, start_idx - end_idx : {}", boundary_data.len(), end_idx  - start_idx);
        self.glyph_boundary_data.splice(start_idx .. end_idx, boundary_data.iter().map(|v| v.length())).for_each(drop);
        self.glyph_map.insert(glyph_name, self.num_glyphs as u16);
        self.num_glyphs += 1;
    }

    pub fn add_edge(&mut self, start : Point, end : Point, start_glyph : &str, end_glyph : &str, start_glyph_scale : f32, end_glyph_scale : f32){
        self.edge_instances.push(EdgeInstance {
            color : Vec4::new(0.0, 0.0, 0.0, 1.0),
            start_position : start,
            end_position : end,
            start_glyph : self.glyph_map[start_glyph],
            end_glyph : self.glyph_map[end_glyph],
            start_glyph_scale,
            end_glyph_scale,
        })
    }

    
    fn set_buffer_data(&self){
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, self.attributes_buffer.as_ref());
        let u8_len = self.edge_instances.len() * std::mem::size_of::<EdgeInstance>();
        log!("self.glyph_instances : {:?}", self.edge_instances);

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

    fn ensure_texture_size(&mut self){
        let num_rows = (glyph_boundary_index(self.num_glyphs + 1) + DATA_ROW_SIZE - 1) / DATA_ROW_SIZE;
        log!("num_rows : {}, self.glyph_boundary_texture_rows : {}", num_rows, self.glyph_boundary_texture_rows);
        if num_rows <= self.glyph_boundary_texture_rows {
            return;
        }
        self.glyph_boundary_texture_rows = num_rows;
        self.webgl.delete_texture(self.glyph_boundary_texture.as_ref());
        self.glyph_boundary_texture = self.webgl.inner.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.glyph_boundary_texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            WebGl2RenderingContext::RGBA32F, // internalformat:,
            DATA_ROW_SIZE as i32, num_rows as i32
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
    }

    fn set_texture_data(&self) -> Result<(), JsValue> {
        // let glyph_boundary_data_length = glyph_boundary_index(self.num_glyphs + 1);
        let num_rows = (glyph_boundary_index(self.num_glyphs + 1) + DATA_ROW_SIZE - 1) / DATA_ROW_SIZE;
        log!("num_rows : {}", num_rows);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.glyph_boundary_texture.as_ref());
        // log!("len : {}, num_rows : {}, DATA_ROW_SIZE * num_rows : {}", len, num_rows, DATA_ROW_SIZE * num_rows);
        unsafe {
            let array_view = js_sys::Float32Array::view(&self.glyph_boundary_data);
            crate::console_log::log_1(&array_view);
            self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D, 
                0, // mip level
                0, 0, // xoffset, yoffset: i32,
                (DATA_ROW_SIZE/4) as i32, num_rows as i32, // width, height
                WebGl2RenderingContext::RGBA, // format: u32,
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


    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point){
        self.shader.use_program();
        self.webgl.bind_vertex_array(self.attribute_state.as_ref());
        self.shader.set_uniform_int("uGlyphDataTexture", 0);
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            (6 * self.edge_instances.len()) as i32,
            1
        );
    }
}