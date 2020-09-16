use crate::font::{GlyphPath};
use crate::shader::{Shader, Geometry};
use crate::rect::BufferDimensions;
use crate::vector::{Vec4};
use lyon::geom::math::{Point, point, Vector, vector, Transform};
use crate::webgl_wrapper::{WebGlWrapper, Buffer, RenderTarget};


use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlTexture, WebGlFramebuffer};

static JITTER_PATTERN : [Vector; 6] = [
    Vector::new(-1.0 / 12.0, -5.0 / 12.0),
    Vector::new( 1.0 / 12.0,  1.0 / 12.0),
    Vector::new( 3.0 / 12.0, -1.0 / 12.0),
    Vector::new( 5.0 / 12.0,  5.0 / 12.0),
    Vector::new( 7.0 / 12.0, -3.0 / 12.0),
    Vector::new( 9.0 / 12.0,  3.0 / 12.0),
];

static JITTER_COLORS : [Vec4; 6] = [
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
];


static STANDARD_QUAD : [Point; 4] = [
    Point::new(0.0, 0.0), 
    Point::new(1.0, 0.0), 
    Point::new(0.0, 1.0), 
    Point::new(1.0, 1.0), 
];

#[wasm_bindgen]
pub enum HorizontalAlignment {
    Left,
    Right,
    Center,
}

#[wasm_bindgen]
pub enum VerticalAlignment {
    Top,
    Center,
    Baseline,
    Bottom,
}



pub struct GlyphShader {
    webgl : WebGlWrapper,
    pub antialias_shader : Shader,
    render_shader : Shader,
    quad_geometry : Geometry,
    antialias_buffer : Buffer
}

impl GlyphShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let mut antialias_shader = Shader::new(
            webgl.clone(), 
            r#"#version 300 es
                in vec4 aVertexPosition;
                out vec2 vBezierParameter;
                uniform mat3x2 uTransformationMatrix;
                void main() {
                    vBezierParameter = aVertexPosition.zw;
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition.xy, 1.0), 0.0, 1.0);
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
                uniform mat3x2 uTransformationMatrix;
                in vec2 aVertexPosition;
                out vec2 vTextureCoord;
                void main() {
                    gl_Position = vec4(
                        mix(
                            uTransformationMatrix * vec3(uBoundingBox.xy, 1.0), 
                            uTransformationMatrix * vec3(uBoundingBox.zw, 1.0), 
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
        let mut quad_geometry = render_shader.create_geometry();
        quad_geometry.num_vertices = 4;
        quad_geometry.num_instances = 1;
        let standard_quad : &[_] = &STANDARD_QUAD; 
        render_shader.set_attribute_point(&mut quad_geometry, "aVertexPosition", standard_quad)?;

        let antialias_buffer = webgl.new_buffer();

        Ok(Self {
            webgl,
            antialias_shader,
            render_shader,
            quad_geometry,
            antialias_buffer
        })
    }



    pub fn recover_context(&mut self) {
        self.antialias_buffer.recover_context();
    }

    pub fn resize_buffer(&mut self, buffer_dimensions : BufferDimensions) -> Result<(), JsValue> {
        self.antialias_buffer.resize(buffer_dimensions)?;
        Ok(())
    }


    fn antialias_render(&self, glyph : &GlyphPath, transform : Transform, scale : f32) -> Result<(), JsValue>{
        self.antialias_shader.use_program();
        let vertices = &glyph.vertices;
        let mut geometry = self.antialias_shader.create_geometry();
        geometry.num_vertices = vertices.len() as i32;
        geometry.num_instances = 1;
        self.antialias_shader.set_attribute_vec4(&mut geometry, "aVertexPosition", vertices.as_slice())?;
        for (&offset, &color) in JITTER_PATTERN.iter().zip(JITTER_COLORS.iter()) {
            let cur_transform = transform.pre_translate(offset * (1.0 / WebGlWrapper::pixel_density() as f32))
                .then_scale(scale, scale);
            self.antialias_shader.set_uniform_vec4("uColor", color);
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

        self.render_shader.set_uniform_int("uTexture", 0);
        self.render_shader.set_uniform_vec4("uBoundingBox", Vec4::new(left, top, right, bottom));
        self.render_shader.set_uniform_vec4("uColor", color);
        self.render_shader.draw(&self.quad_geometry, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        Ok(())
    }

    pub fn draw(&mut self, 
        glyph : &GlyphPath, 
        transform : Transform, 
        pos : Point, scale : f32, 
        horizontal_alignment : HorizontalAlignment,
        vertical_alignment : VerticalAlignment,
        color : Vec4
    ) -> Result<(), JsValue> {
        let mut target : Option<&WebGlFramebuffer> = None;
        self.draw_to_target(glyph, transform, pos, scale, horizontal_alignment, vertical_alignment, color, &mut target)
    }

    pub fn get_raster(&mut self, 
        glyph : &GlyphPath, 
        transform : Transform, 
        scale : f32, 
        target : &mut Buffer
    ) -> Result<(Vec<u8>, i32, i32), JsValue>{
        self.webgl.render_to(target)?;
        self.webgl.clear_color(0.0, 0.0, 0.0, 0.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        self.draw_to_target(
            glyph, transform, Point::new(0.0, 0.0), scale, HorizontalAlignment::Left, VerticalAlignment::Top, Vec4::new(0.0, 0.0, 1.0, 1.0), 
            target
        )?;
        let dimensions = self.antialias_buffer.dimensions;
        let density = dimensions.density() as f32;
        let left = f32::ceil(glyph.bounding_box.left() * scale  * density) as i32;
        let width = f32::ceil((glyph.bounding_box.right() - glyph.bounding_box.left()) * scale  * density) as i32;
        let height = f32::ceil((glyph.bounding_box.bottom() - glyph.bounding_box.top()) * scale * density) as i32;
        

        let mut result = vec![0; (width * height * 4) as usize];
        self.webgl.read_pixels_with_opt_u8_array(
            left, dimensions.pixel_height() - height,
            width, height,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            Some(&mut result)
        )?;

        Ok((result, width, height))
    }


    pub fn draw_to_target<T : RenderTarget>(&mut self, 
        glyph : &GlyphPath, 
        transform : Transform, 
        mut pos : Point, scale : f32, 
        horizontal_alignment : HorizontalAlignment,
        vertical_alignment : VerticalAlignment,
        color : Vec4,
        render_target : &mut T
    ) -> Result<(), JsValue> {
        let x_offset = match horizontal_alignment {
            HorizontalAlignment::Left => { 0.0 }
            HorizontalAlignment::Right => {
                - glyph.bounding_box.right() + glyph.bounding_box.left()
            }
            HorizontalAlignment::Center => {
                ( - glyph.bounding_box.right() + glyph.bounding_box.left() ) / 2.0
            }
        };
        let y_offset = match vertical_alignment {
            VerticalAlignment::Baseline => { 0.0 }
            VerticalAlignment::Center => {
                ( - glyph.bounding_box.top() - glyph.bounding_box.bottom() ) / 2.0
            }
            VerticalAlignment::Top => { - glyph.bounding_box.top() }
            VerticalAlignment::Bottom => { - glyph.bounding_box.bottom() }            
        };

        pos.x += x_offset * scale;
        pos.y += y_offset * scale;
        
        let transform = transform.pre_translate(pos.to_vector());


        self.webgl.add_blend_mode();
        self.webgl.render_to(&mut self.antialias_buffer)?;

        self.webgl.clear_color(0.0, 0.0, 0.0, 1.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        
        self.antialias_render(glyph, transform, scale)?;
        
        self.webgl.render_to(render_target)?;

        let transform = transform.then_scale(scale, scale);
        self.webgl.enable(WebGl2RenderingContext::BLEND);
        self.webgl.blend_func(WebGl2RenderingContext::ZERO, WebGl2RenderingContext::SRC_COLOR);
        
        self.antialias_buffer.use_as_texture(WebGl2RenderingContext::TEXTURE0);

        self.main_render(glyph, transform, Vec4::new(0.0, 0.0, 0.0, 0.0))?;
        if color.x != 0.0 || color.y != 0.0 || color.z != 0.0 {
            self.webgl.add_blend_mode();
            self.main_render(glyph, transform, color)?;
        }
        Ok(())
    }
}