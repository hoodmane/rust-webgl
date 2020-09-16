use crate::log;
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::{Shader, Geometry};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext};

use std::f32::consts::FRAC_PI_4;

pub struct ArcShader {
    webgl : WebGlWrapper,
    shader : Shader,
    geometry : Geometry,
}

#[derive(Debug)]
struct Arc {
    vertices : Vec<Vec2>,
    center : Vec2,
    radius : f32,
    thickness : f32
}


impl ArcShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            webgl.clone(),
            // vertexShader : 
            r#"#version 300 es
                precision highp float;
                uniform mat3 uTransformationMatrix;

                const vec2 JITTER_PATTERN[6] = vec2[](
                    vec2(-1.0 / 12.0, -5.0 / 12.0),
                    vec2( 1.0 / 12.0,  1.0 / 12.0),
                    vec2( 3.0 / 12.0, -1.0 / 12.0),
                    vec2( 5.0 / 12.0,  5.0 / 12.0),
                    vec2( 7.0 / 12.0, -3.0 / 12.0),
                    vec2( 9.0 / 12.0,  3.0 / 12.0)
                );

                uniform vec2 uCenter;
                in vec2 aVertexPosition;
                out vec2 vHelperCoord;

                void main() {
                    vec2 jitter_offset = JITTER_PATTERN[gl_InstanceID];
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition + jitter_offset, 1.0), 0.0).xywz;
                    vHelperCoord = aVertexPosition + jitter_offset - uCenter;
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                in vec2 vHelperCoord;

                // returns 0 if gradient << compValue, 1 if gradient >> compValue,
                // if gradient ~ compValue linearly interpolates a single pixel
                // https://www.ronja-tutorials.com/2019/11/29/fwidth.html#a-better-step
                float aaStep(float compValue, float gradient){
                    float halfChange = fwidth(gradient) / 2.0;
                    //base the range of the inverse lerp on the change over one pixel
                    float lowerEdge = compValue - halfChange;
                    float upperEdge = compValue + halfChange;
                    //do the inverse interpolation
                    float stepped = (gradient - lowerEdge) / (upperEdge - lowerEdge);
                    stepped = clamp(stepped, 0.0, 1.0);
                    return stepped;
                  }


                uniform float uRadius;
                uniform float uThickness;
                uniform vec4 uColor;
                out vec4 outColor;
                void main() {
                    float magnitude = length(vHelperCoord);
                    float rmax = uRadius + uThickness;
                    float rmin = uRadius - uThickness;
                    // alpha is 1 inside the arc and 0 outside the arc, in between near the edge
                    float gradient = max(rmin - magnitude, magnitude - rmax);
                    float alpha = 1.0 - aaStep(0.0, gradient);
                    outColor = uColor;
                    outColor.a *= alpha;
                    if(alpha == 0.0){ 
                        discard;
                    }
                }
            "#
        )?;
        shader.add_attribute_vec2f("aVertexPosition", false)?;
        let mut geometry = shader.create_geometry();
        geometry.num_instances = 1;

        Ok(Self {
            webgl,
            shader,
            geometry,
        })
    }

    pub fn draw_arc(&mut self, 
        transform : Transform,
        p : Vec2, q : Vec2, 
        theta : f32, color : Vec4, thickness : f32
    ) -> Result<(), JsValue> {
        let arc = self.compute_arc(p, q, theta, thickness)?;
        log!("{:?}", arc);
        self.render_arc(transform, &arc, color)?;
        Ok(())
    }


    fn compute_arc(&self, p : Vec2, q : Vec2, theta : f32, thickness : f32) -> Result<Arc, JsValue> {
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


        let theta0 = (p - center).angle();
        let num_segments = f32::ceil(theta / FRAC_PI_4);//NUM_SEGMENTS as f32; //
        let envelope_thickness = thickness * 5.0;
        let inner_radius = radius - envelope_thickness;
        let outer_radius = (radius + envelope_thickness) / f32::cos(theta / num_segments);

        let mut vertices = Vec::new();
        for i in 0 ..= num_segments as usize {
            let v = Vec2::direction(theta0 -  2.0 * (theta / num_segments) * i as f32);
            vertices.push(center + v * inner_radius);
            vertices.push(center + v * outer_radius);
        }
        Ok(Arc {
            vertices,
            center,
            radius,
            thickness
        })
    }

    fn render_arc(&mut self, transform : Transform, arc : &Arc, color : Vec4) -> Result<(), JsValue> {
        self.shader.use_program();
        // self.webgl.render_to_canvas();
        self.webgl.premultiplied_blend_mode();

        self.geometry.num_vertices = arc.vertices.len() as i32;
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_vec2("uCenter", arc.center);
        self.shader.set_uniform_float("uRadius", arc.radius);
        self.shader.set_uniform_float("uThickness", arc.thickness);
        self.shader.set_uniform_vec4("uColor", color);
        self.shader.set_attribute_vec2(&mut self.geometry, "aVertexPosition", &*arc.vertices)?;
        self.shader.draw(&self.geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }
}


