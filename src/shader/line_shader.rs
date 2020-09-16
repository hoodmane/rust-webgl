use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::{Shader, Geometry};

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;


pub struct LineShader {
    pub shader : Shader,
    vertices : Vec<Vec2>,
    colors : Vec<Vec4>,
    geometry : Geometry
}


impl LineShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            webgl,
            // vertexShader : 
            r#"#version 300 es
                uniform mat3 uTransformationMatrix;
                uniform sampler2D uPositionTexture;

                in vec4 aColor;
                flat out vec4 fColor;

                vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
                    int texWidth = textureSize(tex, 0).x;
                    int col = index % texWidth;
                    int row = index / texWidth;
                    return texelFetch(tex, ivec2(col, row), 0);
                }

                void main() {
                    int vertexIdx = gl_InstanceID * 4 + gl_VertexID;
                    vec2 vertexPosition = getValueByIndexFromTexture(uPositionTexture, vertexIdx).xy;
                    fColor = aColor;
                    gl_Position = vec4(uTransformationMatrix * vec3(vertexPosition, 1.0), 0.0).xywz;
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                flat in vec4 fColor;
                out vec4 outColor;
                void main() {
                    outColor = fColor;
                }
            "#
        )?;
        shader.add_attribute_vec4f(&"aColor", true)?;
        let mut geometry = shader.create_geometry();
        geometry.num_vertices = 4;
        Ok(Self {
            shader,
            geometry,
            vertices : Vec::new(),
            colors : Vec::new(),
        })
    }

    pub fn clear(&mut self){
        self.vertices.clear();
        self.colors.clear();
        self.geometry.num_instances = 0;
    }

    pub fn add_line(&mut self, p : Vec2, q : Vec2, color : Vec4, thickness : f32) -> Result<(), JsValue> {
        self.geometry.num_instances += 1;
        let pq = q - p;
        let pq_perp = Vec2::new(pq.y, -pq.x).normalize() * thickness;

        self.vertices.push(p + pq_perp);
        self.vertices.push(p - pq_perp);
        self.vertices.push(q + pq_perp);
        self.vertices.push(q - pq_perp);

        self.colors.push(color);
        Ok(())
    }


    pub fn draw(&mut self, transform : Transform) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_attribute_vec4(&mut self.geometry, "aColor", &self.colors)?;
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        let position_texture = self.shader.webgl.create_vec2_texture(self.vertices.as_slice())?;
        // Put the position data into texture 0.
        self.shader.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.shader.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, position_texture.as_ref());
        self.shader.set_uniform_int("uPositionTexture", 0);
        self.shader.draw(&self.geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }
}