use lyon::geom::math::{Point, Transform};
use crate::webgl_wrapper::{WebGlWrapper, Buffer};
use crate::shader::{ShaderIndexed, GeometryIndexed};
use crate::rect::{RectBuilder};


use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;


pub struct DefaultShaderIndexed {
    webgl : WebGlWrapper,
    pub shader : ShaderIndexed,
    geometry : GeometryIndexed
}


impl DefaultShaderIndexed {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut shader = ShaderIndexed::new(
            webgl.clone(),
            // vertexShader : 
            r#"#version 300 es
                uniform mat3x2 uTransformationMatrix;
                in vec2 aVertexPosition;
                void main() {
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0, 1.0);
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

    pub fn draw(&mut self, transform : Transform, vertices : &[Point], indices : &[u16],  primitive : u32) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_attribute_point(&mut self.geometry, "aVertexPosition", vertices)?;
        self.shader.set_indices(&mut self.geometry, indices)?;
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.draw_indexed(&self.geometry, primitive)?;
        Ok(())
    }
}