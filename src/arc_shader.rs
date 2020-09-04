use wasm_bindgen::prelude::JsValue;
use crate::shader::Shader;
use crate::log::log_str;

use std::f32::consts::FRAC_PI_4;
use web_sys::WebGl2RenderingContext;

use crate::vector::{Vec2, Vec2Buffer};

pub struct ArcShader {
    pub shader : Shader,
    vertices : Vec2Buffer<f32>,
    helper_coords : Vec2Buffer<f32>,
}


impl ArcShader {
    pub fn new(context : WebGl2RenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context,
            // vertexShader : 
            r#"#version 300 es
                in vec2 aVertexPosition;
                in vec2 aHelperCoord;
                out vec2 vHelperCoord;
                uniform mat3 uTransformationMatrix;
                void main() {
                    vHelperCoord = aHelperCoord;
                    // gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0).xywz;
                    gl_Position = vec4(vec3(aVertexPosition, 1.0), 0.0).xywz;
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                uniform vec4 uColor;
                in vec2 vHelperCoord;
                void main() {
                    float magnitude = length(vHelperCoord);
                    float thickness = 0.005;
                    float rmax = (1. + thickness) * (1. + thickness);
                    float rmin = (1. - thickness) * (1. - thickness);
                    if(magnitude < rmax && magnitude > rmin){
                        gl_FragColor = vec4(0, 0, 0, 1);
                    } else {
                        discard;
                        // gl_FragColor = vec4(1, 0, 0, 1);
                    }
                }
            "#
        )?;
        shader.add_attribute_vec2f("aVertexPosition")?;
        shader.add_attribute_vec2f("aHelperCoord")?;
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


        let center_to_p = (p - center).normalize();
        let theta0 = f32::atan2(center_to_p.y, center_to_p.x);
        let n = f32::ceil(theta / FRAC_PI_4);
        let n_usize = n as usize;
        let inner_radius = radius - thickness;
        let outer_radius = (radius + thickness) / f32::cos(theta / n);

        let mut directions = Vec::new();
        for i in 0..= n_usize {
            let i = i as f32;
            directions.push(Vec2::direction(theta0 -  2.0 * (theta / n) * i));
        }
        log_str(&format!("directions : {:?}", directions));
        log_str(&format!("center_to_p : {:?}, center_to_q : {:?}", p - center, q - center));

        log_str(&format!("center : {:?}, p : {:?}, q : {:?}", center, p, q));
        log_str(&format!("inner_radius : {}, outer_radius : {}, theta0 : {}", inner_radius, outer_radius, theta0));

        for i in 0 .. n_usize {
            let v1 = directions[i] * inner_radius;
            let v2 = directions[i] * outer_radius;
            let v3 = directions[i+1] * outer_radius;
            let v4 = directions[i+1] * inner_radius;
            log_str(&format!("v1 : {:?}, v2 : {:?}, v3 : {:?}, v4 : {:?}", v1, v2, v3, v4));
            for &v in &[v1, v2, v3, v1, v3, v4] {
                self.vertices.push_vec(center + v);
                self.helper_coords.push_vec(v * (1.0/radius));
            }            
        }
        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_attribute_data("aVertexPosition", &*self.vertices)?;
        self.shader.set_attribute_data("aHelperCoord", &*self.helper_coords)?;
        log_str(&format!("vertices ({}) : {:?}", self.vertices.len(), self.vertices));
        self.shader.draw(self.vertices.len())?;
        Ok(())
    }
}


