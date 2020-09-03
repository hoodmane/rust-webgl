use wasm_bindgen::JsValue;
use crate::matrix::Transform;
use crate::shader::Shader;
// use crate::log::log_str;

use web_sys::WebGlRenderingContext;

use crate::vector::{Vec2, Vec2Buffer};

pub struct LineShader {
    pub shader : Shader,
    vertices : Vec2Buffer<f32>,
}


impl LineShader {
    pub fn new(context : WebGlRenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context,
            // vertexShader : 
            r#"
                attribute vec2 aVertexPosition;
                uniform mat3 uTransformationMatrix;
                void main() {
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0).xywz;
                    // gl_Position = vec4(vec3(aVertexPosition, 1.0), 0.0).xywz;
                }
            "#,
            // fragmentShader :
            r#"
                precision highp float;
                uniform vec4 uColor;
                void main() {
                    gl_FragColor = vec4(0, 0, 0, 1);
                }
            "#
        )?;
        shader.add_attribute(&"aVertexPosition", 2, WebGlRenderingContext::FLOAT)?;
        Ok(Self {
            shader,
            vertices : Vec2Buffer::new()
        })
    }

    pub fn add_line(&mut self, p : Vec2<f32>, q : Vec2<f32>, thickness : f32) {
        let pq = q - p;
        let pq_perp = Vec2::new(pq.y, -pq.x).normalize() * thickness;
        let points = [p + pq_perp, p - pq_perp, q-pq_perp, q + pq_perp];
        self.vertices.push_vec(points[0]);
        self.vertices.push_vec(points[1]);
        self.vertices.push_vec(points[2]);

        self.vertices.push_vec(points[0]);
        self.vertices.push_vec(points[2]);
        self.vertices.push_vec(points[3]);       
    }


    pub fn draw(&self, transform : Transform) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);        
        self.shader.set_data("aVertexPosition", &*self.vertices)?;
        self.shader.draw(self.vertices.len());
        Ok(())
    }
}