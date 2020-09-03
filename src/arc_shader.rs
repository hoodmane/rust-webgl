use wasm_bindgen::prelude::JsValue;
use crate::shader::Shader;
use crate::log::log_str;
use web_sys::WebGlRenderingContext;

use crate::vector::{Vec2, Vec2Buffer};

pub struct ArcShader {
    pub shader : Shader,
    vertices : Vec2Buffer<f32>,
    helper_coords : Vec2Buffer<f32>,
}


impl ArcShader {
    pub fn new(context : WebGlRenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context,
            // vertexShader : 
            r#"
                attribute vec2 aVertexPosition;
                attribute vec2 aHelperCoord;
                varying vec2 vHelperCoord;
                uniform mat3 uTransformationMatrix;
                void main() {
                    vHelperCoord = aHelperCoord;
                    // gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0).xywz;
                    gl_Position = vec4(vec3(aVertexPosition, 1.0), 0.0).xywz;
                }
            "#,
            // fragmentShader :
            r#"
                precision highp float;
                uniform vec4 uColor;
                varying vec2 vHelperCoord;
                void main() {
                    float magnitude = length(vHelperCoord);
                    float thickness = 0.005;
                    float rmax = (1. + thickness) * (1. + thickness);
                    float rmin = (1. - thickness) * (1. - thickness);
                    if(magnitude < rmax && magnitude > rmin){
                        gl_FragColor = vec4(0, 0, 0, 1);
                    } else {
                        gl_FragColor = vec4(1, 0, 0, 1);
                        // discard;
                    }
                }
            "#
        )?;
        shader.add_attribute("aVertexPosition", 2, WebGlRenderingContext::FLOAT)?;
        shader.add_attribute("aHelperCoord", 2, WebGlRenderingContext::FLOAT)?;
        Ok(Self {
            shader,
            vertices : Vec2Buffer::new(),
            helper_coords : Vec2Buffer::new(),
        })
    }

    pub fn add_arc(&mut self, p : Vec2<f32>, q : Vec2<f32>, theta : f32) -> Result<(), JsValue> {
        let thickness = 0.0;
        if theta == 0.0 {
            return Err(JsValue::from_str(&"Theta should be nonzero."));
        }
        let pq = q - p;
        // half of distance between p and q
        let d = pq.magnitude() * 0.5;
        if d == 0.0 {
            return Err(JsValue::from_str(&"Two points should not be equal."));
        }
        // distance from (p1 + p0)/2 to center (negative if we're left-handed)
        let e = d/f32::tan(theta);
        let radius = d/f32::sin(theta);
        let pq_perp = Vec2::new(pq.y, -pq.x).normalize() * e;
        let center = (p + q) * 0.5 + pq_perp;
        let side_length = (radius + thickness)/f32::cos(theta);
        let aux1 = Vec2::new(0.0, 0.0);
        let aux2 = (p - center).normalize() * side_length;
        let aux3 = (q - center).normalize() * side_length;
        let v1 = center;
        let v2 = center + aux2;
        let v3 = center + aux3;
        self.vertices.push_vec(v1);
        self.vertices.push_vec(v2);
        self.vertices.push_vec(v3);
        self.helper_coords.push_vec(aux1);
        self.helper_coords.push_vec(aux2 * (1.0/radius));
        self.helper_coords.push_vec(aux3 * (1.0/radius));
        Ok(())
    }

    pub fn draw(&self) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_data("aVertexPosition", &*self.vertices)?;
        self.shader.set_data("aHelperCoord", &*self.helper_coords)?;
        log_str(&format!("vertices ({}) : {:?}", self.vertices.len(), self.vertices));
        self.shader.draw(self.vertices.len());
        Ok(())
    }
}


