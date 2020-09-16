use crate::vector::{Vec2};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;
use crate::shader::{Shader, Geometry};
use crate::rect::Rect;

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;


pub struct StencilShader {
    webgl : WebGlWrapper,
    pub shader : Shader,
    geometry : Geometry,
    vertices : Vec<Vec2>,
}


impl StencilShader {
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
                    outColor = vec4(0.5, 0.5, 0.5, 1.0);
                }
            "#
        )?;
        shader.add_attribute_vec2f(&"aVertexPosition", false)?;
        let mut geometry = shader.create_geometry();
        geometry.num_vertices = 4;
        geometry.num_instances = 1;
        Ok(Self {
            webgl,
            shader,
            geometry,
            vertices : Vec::new(),
        })
    }

    pub fn set_stencil_rect(&mut self, transform : Transform, rect : Rect) -> Result<(), JsValue> {
        let stencil_was_disabled = !self.webgl.is_enabled(WebGl2RenderingContext::STENCIL_TEST);
        self.webgl.enable(WebGl2RenderingContext::STENCIL_TEST);
        self.webgl.stencil_func(WebGl2RenderingContext::ALWAYS, 1, 0xFF);
        self.webgl.clear_stencil(0);
        self.webgl.clear(WebGl2RenderingContext::STENCIL_BUFFER_BIT);
        self.webgl.stencil_op(WebGl2RenderingContext::REPLACE, WebGl2RenderingContext::REPLACE, WebGl2RenderingContext::REPLACE);
        self.webgl.color_mask(false, false, false, false);

        // Draw stencil region
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.vertices.clear();
        self.vertices.push(Vec2::new(rect.left(), rect.top()));
        self.vertices.push(Vec2::new(rect.left(), rect.bottom()));
        self.vertices.push(Vec2::new(rect.right(), rect.top()));
        self.vertices.push(Vec2::new(rect.right(), rect.bottom()));
        self.shader.set_attribute_vec2(&mut self.geometry, "aVertexPosition", &self.vertices)?;
        self.shader.draw(&self.geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;

        // Don't change stencil buffer in future
        self.webgl.stencil_func(WebGl2RenderingContext::EQUAL, 1, 0xFF);
        self.webgl.stencil_op(WebGl2RenderingContext::KEEP, WebGl2RenderingContext::KEEP, WebGl2RenderingContext::KEEP);
        self.webgl.color_mask(true, true, true, true);
        // Restore stenciling enabled/disabled state
        if stencil_was_disabled {
            self.webgl.disable(WebGl2RenderingContext::STENCIL_TEST);
        }
        Ok(())
    }
}