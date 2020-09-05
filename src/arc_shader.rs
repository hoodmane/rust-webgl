use crate::log::log_str;
use crate::vector::{Vec2, Vec2Buffer, Vec4, Vec4Buffer};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::{Shader, Geometry};

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;

use std::f32::consts::FRAC_PI_4;

static NUM_SEGMENTS : i32 = 4;

pub struct ArcShader {
    pub shader : Shader,
    geometry : Geometry,
    vertices : Vec2Buffer<f32>,
    centers : Vec2Buffer<f32>,
    radiuses : Vec<f32>,
    colors : Vec4Buffer<f32>,
    thicknesses : Vec<f32>
}


impl ArcShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            webgl,
            // vertexShader : 
            r#"#version 300 es
                uniform mat3 uTransformationMatrix;
                uniform sampler2D uPositionTexture;

                out vec2 vHelperCoord;
                
                in vec2 aCenter;

                in float aRadius;
                flat out float fRadius;

                in vec4 aColor;
                flat out vec4 fColor;

                in float aThickness;
                flat out float fThickness;

                vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
                    int texWidth = textureSize(tex, 0).x;
                    int col = index % texWidth;
                    int row = index / texWidth;
                    return texelFetch(tex, ivec2(col, row), 0);
                }

                void main() {
                    int vertexIdx = gl_InstanceID * 10 + gl_VertexID;
                    vec2 inputPosition = getValueByIndexFromTexture(uPositionTexture, vertexIdx).xy;

                    fRadius = aRadius;
                    fColor = aColor;
                    fThickness = aThickness;
                    gl_Position = vec4(uTransformationMatrix * vec3(inputPosition.xy + aCenter, 1.0), 0.0).xywz;
                    vHelperCoord = inputPosition;
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                uniform vec4 uColor;
                in vec2 vHelperCoord;
                flat in float fRadius;
                flat in vec4 fColor;
                flat in float fThickness;
                out vec4 outColor;
                void main() {
                    float magnitude = dot(vHelperCoord, vHelperCoord);
                    float rsquaredmax = (fRadius + fThickness) * (fRadius + fThickness);
                    float rsquaredmin = (fRadius - fThickness) * (fRadius - fThickness);
                    if(magnitude > rsquaredmin && magnitude < rsquaredmax){
                        outColor = fColor;
                    } else {
                        discard;
                    }
                }
            "#
        )?;
        shader.add_attribute_vec2f("aCenter", true)?;
        shader.add_attribute_float("aRadius", true)?;
        shader.add_attribute_vec4f("aColor", true)?;
        shader.add_attribute_float("aThickness", true)?;
        let mut geometry = shader.create_geometry()?;
        geometry.num_vertices = (NUM_SEGMENTS + 1) * 2;
        Ok(Self {
            shader,
            geometry,
            vertices : Vec2Buffer::new(),
            centers : Vec2Buffer::new(),
            radiuses : Vec::new(),
            colors : Vec4Buffer::new(),
            thicknesses : Vec::new()
        })
    }

    pub fn add_arc(&mut self, p : Vec2<f32>, q : Vec2<f32>, theta : f32, color : Vec4<f32>, thickness : f32) -> Result<(), JsValue> {
        self.geometry.num_instances += 1;       
        if theta == 0.0 {
            return Err(JsValue::from_str(&"Theta should be nonzero."));
        }
        // If theta > 0, bend right, if theta < 0 bend left.
        // The logic we wrote (1) only works if theta > 0 and (2) bends the curve left. 
        // If theta is negative, then we want to bend left so no need to swap p and q, but we negate theta to ensure it is positive
        // If theta is positive, then we want to bend right, so swap p and q and leave sign of theta alone.
        let (p, q, theta) = if theta > 0.0 {
                (q, p, theta)
            } else {
                (p, q, -theta)
            };
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
        let num_segments_float = NUM_SEGMENTS as f32; //f32::ceil(theta / FRAC_PI_4);
        let envelope_thickness = thickness * 1.05;
        let inner_radius = radius - envelope_thickness;
        let outer_radius = (radius + envelope_thickness) / f32::cos(theta / num_segments_float);

        for i in 0 ..= NUM_SEGMENTS {
            let v = Vec2::direction(theta0 -  2.0 * (theta / num_segments_float) * i as f32);
            self.vertices.push_vec(v * inner_radius);
            self.vertices.push_vec(v * outer_radius);
        }
        log_str(&format!("radius : {}, inner_radius : {}, outer_radius : {}", radius, inner_radius, outer_radius));
        self.centers.push_vec(center);
        self.radiuses.push(radius);
        self.colors.push_vec(color);
        self.thicknesses.push(thickness);
        self.shader.set_attribute_data(&mut self.geometry, "aCenter", &*self.centers)?;
        self.shader.set_attribute_data(&mut self.geometry, "aRadius", &*self.radiuses)?;
        self.shader.set_attribute_data(&mut self.geometry, "aColor", &*self.colors)?;
        self.shader.set_attribute_data(&mut self.geometry, "aThickness", &*self.thicknesses)?;
        Ok(())
    }

    pub fn draw(&self, transform : Transform) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        log_str(&format!("vertices ({}) : {:?}", self.vertices.len(), self.vertices));
        let position_texture = self.shader.webgl.create_vec2_texture(&self.vertices)?;
        // Put the position data into texture 0.
        self.shader.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.shader.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&position_texture));
        self.shader.set_uniform_int("uPositionTexture", 0);
        self.shader.draw(&self.geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }
}


