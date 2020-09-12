use crate::log::log_str;
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::{Shader, Geometry};

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;


pub struct DefaultShader {
    pub shader : Shader,
    geometry : Geometry
}


impl DefaultShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            webgl,
            // vertexShader : 
            r#"#version 300 es
                uniform mat3 uTransformationMatrix;
                in vec2 aVertexPosition;
                void main() {
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0).xywz;
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                out vec4 outColor;
                void main() {
                    outColor = vec4(0.0, 0.0, 0.0, 1.0);
                }
            "#
        )?;
        shader.add_attribute_vec2f(&"aVertexPosition", false)?;
        let geometry = shader.create_geometry()?;
        Ok(Self {
            shader,
            geometry,
        })
    }

    pub fn draw(&mut self, transform : Transform, vertices : &Vec<Vec2>, primitive : u32) -> Result<(), JsValue> {
        self.shader.use_program();
        self.geometry.num_vertices = vertices.len() as i32;
        self.geometry.num_instances = 1;
        self.shader.set_attribute_vec2(&mut self.geometry, "aVertexPosition", vertices)?;
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.draw(&self.geometry, primitive)?;
        Ok(())
    }

    // pub fn get_raster(&mut self, transform : Transform, vertices : &Vec<Vec2>, primitive : u32, target : Option<&WebGlFramebuffer>) -> Result<(Vec<u8>, i32, i32), JsValue> {
    //     self.webgl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, target.as_ref());
    //     self.webgl.clear_color(0.0, 0.0, 0.0, 0.0);
    //     self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

    //     self.draw(transform, vertices, primitive)?;


    //     let mut result = vec![0; (width * height * 4) as usize];
    //     self.webgl.read_pixels_with_opt_u8_array(
    //         left, self.buffer_dimensions.height() - height,
    //         width, height,
    //         WebGl2RenderingContext::RGBA,
    //         WebGl2RenderingContext::UNSIGNED_BYTE,
    //         Some(&mut result)
    //     )?;
    //     Ok(result, width, height)

    // }
}