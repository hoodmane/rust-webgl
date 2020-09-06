use crate::log::log_str;
use crate::vector::{Vec2, Vec2Buffer, Vec4, Vec4Buffer};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::{Shader, Geometry};

use wasm_bindgen::JsValue;
use web_sys::{WebGlTexture, WebGl2RenderingContext};

use std::f32::consts::FRAC_PI_4;


static JITTER_PATTERN : [Vec2<f32>; 6] = [
    Vec2::new(-1.0 / 12.0, -5.0 / 12.0),
    Vec2::new( 1.0 / 12.0,  1.0 / 12.0),
    Vec2::new( 3.0 / 12.0, -1.0 / 12.0),
    Vec2::new( 5.0 / 12.0,  5.0 / 12.0),
    Vec2::new( 7.0 / 12.0, -3.0 / 12.0),
    Vec2::new( 9.0 / 12.0,  3.0 / 12.0),
];

pub struct ArcShader {
    webgl : WebGlWrapper,
    antialias_buffer : WebGlTexture,
    antialias_shader : Shader,
    antialias_geometry : Geometry,
    render_shader : Shader,
    render_geometry : Geometry,
}

#[derive(Debug)]
struct Arc {
    vertices : Vec2Buffer<f32>,
    center : Vec2<f32>,
    radius : f32,
    thickness : f32
}


impl ArcShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let antialias_buffer = webgl.create_texture(webgl.pixel_width(), webgl.pixel_height(), WebGl2RenderingContext::R8)?;
        let mut antialias_shader = Shader::new(
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

                uniform float uRadius;
                uniform float uThickness;
                out vec4 outColor;
                void main() {
                    float magnitude = length(vHelperCoord);
                    float rmax = uRadius + uThickness;
                    float rmin = uRadius - uThickness;
                    // alpha is 1 inside the arc and 0 outside the arc, in between near the edge
                    // float alpha = 1.0 - smoothstep(0.0, 1.0, max(rmin - magnitude, magnitude - rmax));
                    float alpha = clamp(mix(1.0, 0.0, max(rmin - magnitude, magnitude - rmax)), 0.0, 1.0);
                    
                    float rsquaredmax = (uRadius + uThickness) * (uRadius + uThickness);
                    float rsquaredmin = (uRadius - uThickness) * (uRadius - uThickness);
                    outColor = vec4(0.0, 0.0, 0.0, alpha);
                    if(alpha == 0.){ 
                        discard;
                    }
                }
            "#
        )?;
        antialias_shader.add_attribute_vec2f("aVertexPosition", false)?;
        let mut antialias_geometry = antialias_shader.create_geometry()?;
        antialias_geometry.num_instances = 3;




        let mut render_shader = Shader::new(
            webgl.clone(),
            // vertexShader : 
            r#"#version 300 es
                uniform mat3 uTransformationMatrix;
                in vec2 aVertexPosition;
                out vec2 vTextureCoord;

                void main() {
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0).xywz;
                    vTextureCoord = (gl_Position.xy + 1.0) * 0.5;                    
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                uniform sampler2D uTexture;
                uniform vec4 uColor;
                in vec2 vTextureCoord;
                out vec4 outColor;
                void main() {
                    float opacity = texture(uTexture, vTextureCoord).x;
                    outColor = uColor;
                    outColor.a *= opacity;
                }
            "#
        )?;
        render_shader.add_attribute_vec2f("aVertexPosition", false)?;
        let mut render_geometry = render_shader.create_geometry()?;
        render_geometry.num_instances = 2;

        Ok(Self {
            webgl,
            antialias_shader,
            antialias_buffer,
            antialias_geometry,
            render_shader,
            render_geometry
        })
    }

    pub fn draw_arc(&mut self, 
        transform : Transform,
        p : Vec2<f32>, q : Vec2<f32>, 
        theta : f32, color : Vec4<f32>, thickness : f32
    ) -> Result<(), JsValue> {
        let arc = self.compute_arc(p, q, theta, thickness)?;
        log_str(&format!("{:?}", arc));
        self.antialias_to_buffer(transform, &arc)?;
        // self.render_from_buffer(transform, &arc, color)?;
        Ok(())
    }


    fn compute_arc(&self, p : Vec2<f32>, q : Vec2<f32>, theta : f32, thickness : f32) -> Result<Arc, JsValue> {
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
        let num_segments = f32::ceil(theta / FRAC_PI_4);//NUM_SEGMENTS as f32; //
        let envelope_thickness = thickness * 5.0;
        let inner_radius = radius - envelope_thickness;
        let outer_radius = (radius + envelope_thickness) / f32::cos(theta / num_segments);

        let mut vertices = Vec2Buffer::new();
        for i in 0 ..= num_segments as usize {
            let v = Vec2::direction(theta0 -  2.0 * (theta / num_segments) * i as f32);
            vertices.push_vec(center + v * inner_radius);
            vertices.push_vec(center + v * outer_radius);
        }
        Ok(Arc {
            vertices,
            center,
            radius,
            thickness
        })
    }

    fn antialias_to_buffer(&mut self, transform : Transform, arc : &Arc) -> Result<(), JsValue> {
        self.antialias_shader.use_program();
        // self.webgl.add_blend_mode();
        // self.webgl.render_to_texture(&self.antialias_buffer);
        self.webgl.enable(WebGl2RenderingContext::BLEND);
        self.webgl.blend_func(WebGl2RenderingContext::ONE, WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA);
        self.webgl.render_to_canvas();
        // self.webgl.viewport(0, 0, self.webgl.pixel_width(), self.webgl.pixel_height());
        // self.webgl.inner.clear_color(0.0, 0.0, 0.0, 1.0);
        // self.webgl.inner.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT); 

        self.antialias_geometry.num_vertices = arc.vertices.len() as i32;
        self.antialias_shader.set_uniform_transform("uTransformationMatrix", transform);
        self.antialias_shader.set_uniform_vec2("uCenter", arc.center);
        self.antialias_shader.set_uniform_float("uRadius", arc.radius);
        self.antialias_shader.set_uniform_float("uThickness", arc.thickness);
        self.antialias_shader.set_attribute_data(&mut self.antialias_geometry, "aVertexPosition", &*arc.vertices)?;
        self.antialias_shader.draw(&self.antialias_geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }

    fn render_from_buffer(&mut self, transform : Transform, arc : &Arc, color : Vec4<f32>) -> Result<(), JsValue> {
        self.render_shader.use_program();
        self.webgl.blend_func(WebGl2RenderingContext::ZERO, WebGl2RenderingContext::SRC_COLOR);
        self.webgl.render_to_canvas();
        self.webgl.inner.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.webgl.inner.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.antialias_buffer));
        
        self.render_geometry.num_vertices = arc.vertices.len() as i32;
        self.render_shader.set_uniform_transform("uTransformationMatrix", transform);
        self.render_shader.set_uniform_int("uTexture", 0);
        self.render_shader.set_uniform_vec4("uColor", color);
        self.render_shader.set_attribute_data(&mut self.render_geometry, "aVertexPosition", &*arc.vertices)?;
        self.render_shader.draw(&self.render_geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }
}


