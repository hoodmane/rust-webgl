use crate::log::log_str;
use crate::font::GlyphPath;
use crate::shader::{Shader, Geometry};
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::webgl_wrapper::WebGlWrapper;


use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext, WebGlTexture};

static JITTER_PATTERN : [Vec2; 6] = [
    Vec2::new(-1.0 / 12.0, -5.0 / 12.0),
    Vec2::new( 1.0 / 12.0,  1.0 / 12.0),
    Vec2::new( 3.0 / 12.0, -1.0 / 12.0),
    Vec2::new( 5.0 / 12.0,  5.0 / 12.0),
    Vec2::new( 7.0 / 12.0, -3.0 / 12.0),
    Vec2::new( 9.0 / 12.0,  3.0 / 12.0),
];

static JITTER_COLORS : [Vec4; 6] = [
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
];


static STANDARD_QUAD : [f32; 4 * 2] = [
    0.0, 0.0, 
    1.0, 0.0,
    0.0, 1.0,
    1.0, 1.0
];

pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub antialias_shader : Shader,
    render_shader : Shader,
    quad_geometry : Geometry,
    antialias_buffer : WebGlTexture,
    buffer_width : i32,
    buffer_height : i32
}

impl GlyphShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut antialias_shader = Shader::new(
            webgl.clone(), 
            r#"#version 300 es
                in vec4 aVertexPosition;
                out vec2 vBezierParameter;
                uniform mat3 uTransformationMatrix;
                void main() {
                    vBezierParameter = aVertexPosition.zw;
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition.xy, 1.0), 0.0).xywz;
                }
            "#,
            r#"#version 300 es
                precision highp float;
                uniform vec4 uColor;
                in vec2 vBezierParameter;
                out vec4 outColor;
                void main() {
                    if (vBezierParameter.x * vBezierParameter.x > vBezierParameter.y) {
                        discard;
                    }

                    // Upper 4 bits: front faces
                    // Lower 4 bits: back faces
                    outColor = uColor * ((gl_FrontFacing ? 16.0 : 1.0) / 255.0);
                }
            "#
        )?;
        antialias_shader.add_attribute_vec4f("aVertexPosition", false)?;

        let mut render_shader = Shader::new(
            webgl.clone(), 
            r#"#version 300 es
                uniform vec4 uBoundingBox;
                uniform mat3 uTransformationMatrix;
                in vec2 aVertexPosition;
                out vec2 vTextureCoord;
                void main() {
                    gl_Position = vec4(
                        mix(
                            (uTransformationMatrix * vec3(uBoundingBox.xy, 1.0)).xy, 
                            (uTransformationMatrix * vec3(uBoundingBox.zw, 1.0)).xy, 
                            aVertexPosition
                        ), 
                        0.0, 1.0
                    );
                    // The coordinate system for writing 
                    vTextureCoord = (gl_Position.xy + 1.0) * 0.5;                    
                }
            "#,
            r#"#version 300 es
                precision highp float;
                uniform sampler2D uTexture;
                uniform vec4 uColor;
                in vec2 vTextureCoord;
                out vec4 outColor;
                void main() {
                    vec2 valueL = texture(uTexture, vec2(vTextureCoord.x + dFdx(vTextureCoord.x), vTextureCoord.y)).yz * 255.0;
                    vec2 lowerL = mod(valueL, 16.0);
                    vec2 upperL = (valueL - lowerL) / 16.0;
                    vec2 alphaL = min(abs(upperL - lowerL), 2.0);
                
                    // Get samples for 0, +1/3, and +2/3
                    vec3 valueR = texture(uTexture, vTextureCoord).xyz * 255.0;
                    vec3 lowerR = mod(valueR, 16.0);
                    vec3 upperR = (valueR - lowerR) / 16.0;
                    vec3 alphaR = min(abs(upperR - lowerR), 2.0);
                
                    // Average the energy over the pixels on either side
                    vec4 rgba = vec4(
                        (alphaR.x + alphaR.y + alphaR.z) / 6.0,
                        (alphaL.y + alphaR.x + alphaR.y) / 6.0,
                        (alphaL.x + alphaL.y + alphaR.x) / 6.0,
                        0.0);
                
                    // Optionally scale by a color
                    outColor = uColor.a == 0.0 ? 1.0 - rgba : uColor * rgba;
                }
            "#
        )?;
        render_shader.add_attribute_vec2f("aVertexPosition", false)?;
        let mut quad_geometry = render_shader.create_geometry()?;
        quad_geometry.num_vertices = 4;
        quad_geometry.num_instances = 1;
        render_shader.set_attribute_data(&mut quad_geometry, "aVertexPosition", &STANDARD_QUAD)?;

        let antialias_buffer = webgl.inner.create_texture().unwrap();

        Ok(Self {
            webgl,
            antialias_shader,
            render_shader,
            quad_geometry,
            antialias_buffer,
            buffer_width : 0,
            buffer_height : 0
        })
    }

    pub fn resize_buffer(&mut self, width : i32, height : i32) -> Result<(), JsValue> {
        self.webgl.delete_texture(Some(&self.antialias_buffer));
        self.antialias_buffer = self.webgl.create_texture(width, height, WebGl2RenderingContext::RGBA8)?;
        self.buffer_width = width;
        self.buffer_height = height;
        Ok(())
    }


    fn antialias_render(&self, glyph : &GlyphPath, transform : Transform, scale : f32) -> Result<(), JsValue>{
        self.antialias_shader.use_program();
        let vertices = &glyph.vertices;
        let mut geometry = self.antialias_shader.create_geometry()?;
        geometry.num_vertices = vertices.len() as i32;
        geometry.num_instances = 1;
        self.antialias_shader.set_attribute_data(&mut geometry, "aVertexPosition", &*vertices)?;
        for (&offset, &color) in JITTER_PATTERN.iter().zip(JITTER_COLORS.iter()) {
            let mut cur_transform = transform;
            cur_transform.translate_vec(offset * (1.0 / WebGlWrapper::pixel_density() as f32));
            cur_transform.scale(scale, scale);
            self.antialias_shader.set_uniform_vec4("uColor", color);
            // self.shader.set_uniform_vec4("uColor", Vec4::new(2.0, 2.0, 2.0, 2.0));
            self.antialias_shader.set_uniform_transform("uTransformationMatrix", cur_transform);        
            self.antialias_shader.draw(&geometry, WebGl2RenderingContext::TRIANGLES)?;
        }
        Ok(())
    }

    fn main_render(&self, glyph : &GlyphPath, transform : Transform, color : Vec4) -> Result<(), JsValue>{
        self.render_shader.use_program();
        self.render_shader.set_uniform_transform("uTransformationMatrix", transform);        

        let bounding_box = glyph.bounding_box;
        let left = bounding_box.left();
        let right = bounding_box.right();
        let top = bounding_box.top();
        let bottom = bounding_box.bottom();

        log_str(&format!("bounding_box : {{ top : {},  bottom : {}, left : {}, right : {}}}", 
            bounding_box.top(), bounding_box.bottom(), bounding_box.left(), bounding_box.right(),
        ));

        let trans_bb = transform.transform_rect(bounding_box);
        log_str(&format!("trans_bb : {{ top : {},  bottom : {}, left : {}, right : {}}}", 
            trans_bb.top(), trans_bb.bottom(), trans_bb.left(), trans_bb.right(),
        ));

        self.render_shader.set_uniform_int("uTexture", 0);
        self.render_shader.set_uniform_vec4("uBoundingBox", Vec4::new(left, top, right, bottom));
        self.render_shader.set_uniform_vec4("uColor", color);
        self.render_shader.draw(&self.quad_geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }

    // pub fn draw(&self, transform : Transform, glyph : &Glyph) -> Result<(), JsValue> {
    pub fn draw(&mut self, glyph : &GlyphPath, mut transform : Transform, pos : Vec2, scale : f32) -> Result<(), JsValue> {
        transform.translate_vec(pos);



        let bounding_box = glyph.bounding_box;

        log_str(&format!("bounding_box : {{ top : {},  bottom : {}, left : {}, right : {}}}", 
            bounding_box.top(), bounding_box.bottom(), bounding_box.left(), bounding_box.right(),
        ));

        let trans_bb = transform.transform_rect(bounding_box);
        log_str(&format!("trans_bb : {{ top : {},  bottom : {}, left : {}, right : {}}}", 
            trans_bb.top(), trans_bb.bottom(), trans_bb.left(), trans_bb.right(),
        ));


        self.webgl.add_blend_mode();
        self.webgl.render_to_texture(&self.antialias_buffer);
        self.webgl.viewport(0, 0, self.buffer_width, self.buffer_height);
        self.webgl.clear_color(0.0, 0.0, 0.0, 1.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        
        self.antialias_render(glyph, transform, scale)?;
        
        self.webgl.render_to_canvas();
        // self.webgl.copy_blend_mode();
        // self.antialias_render(glyph, transform, scale)?;
        

        transform.scale(scale, scale);
        self.webgl.enable(WebGl2RenderingContext::BLEND);
        self.webgl.blend_func(WebGl2RenderingContext::ZERO, WebGl2RenderingContext::SRC_COLOR);
        
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.antialias_buffer));

        self.main_render(glyph, transform, Vec4::new(0.0, 0.0, 0.0, 0.0))?;


        Ok(())
    }
}


