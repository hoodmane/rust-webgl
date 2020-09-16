use crate::vector::{Vec2};
use crate::matrix::Transform;
use crate::webgl_wrapper::{WebGlWrapper, Buffer};
use crate::shader::{Shader, Geometry};
use crate::rect::{RectBuilder};

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;


pub struct DefaultShader {
    webgl : WebGlWrapper,
    pub shader : Shader,
    geometry : Geometry
}


impl DefaultShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            webgl.clone(),
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
        let geometry = shader.create_geometry();
        Ok(Self {
            webgl,
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


    // pub fn draw_elements(&mut self, transform : Transform, vertices : &Vec<Vec2>, primitive : u32) -> Result<(), JsValue> {
    //     self.shader.use_program();
    //     self.geometry.num_vertices = vertices.len() as i32;
    //     self.geometry.num_instances = 1;
    //     self.shader.set_attribute_vec2(&mut self.geometry, "aVertexPosition", vertices)?;
    //     self.shader.set_uniform_transform("uTransformationMatrix", transform);
    //     self.shader.draw_elements(&self.geometry, primitive)?;
    //     Ok(())
    // }

    pub fn get_raster(&mut self, mut transform : Transform, vertices : &Vec<Vec2>, primitive : u32, target : &mut Buffer) -> Result<(Vec<u8>, i32, i32), JsValue> {
        self.webgl.render_to(target)?;
        self.webgl.clear_color(0.0, 0.0, 0.0, 0.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        let mut bounding_box_builder = RectBuilder::new();
        for &v in vertices {
            bounding_box_builder.include(v);
        }
        let bounding_box = bounding_box_builder.build();
        let density = target.dimensions.density() as f32;
        let width = f32::ceil((bounding_box.right() - bounding_box.left()) * density) as i32;
        let height = f32::ceil((bounding_box.bottom() - bounding_box.top()) * density) as i32;

        transform.translate( - bounding_box.left(), - bounding_box.top());

        self.draw(transform, vertices, primitive)?;

        let pixel_height = target.dimensions.pixel_height();
        let mut result = vec![0; (width * height * 4) as usize];
        self.webgl.read_pixels_with_opt_u8_array(
            0, pixel_height - height, width, height,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            Some(&mut result)
        )?;

        // self.webgl.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        // self.webgl.bind_framebuffer(WebGl2RenderingContext::READ_FRAMEBUFFER, target.framebuffer.as_ref());
        // self.webgl.blit_framebuffer(
        //     0, pixel_height - height, width, pixel_height,
        //     0, pixel_height - height, width, pixel_height,
        //     // 0, 0, target.dimensions.pixel_width(), target.dimensions.pixel_height(),// left, bottom, width, height,
        //     // 0, 0, target.dimensions.pixel_width(), target.dimensions.pixel_height(),// left, bottom, width, height,
        //     WebGl2RenderingContext::COLOR_BUFFER_BIT, WebGl2RenderingContext::NEAREST
        // );

        Ok((result, width, height))
    }
}