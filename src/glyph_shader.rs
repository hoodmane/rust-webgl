use crate::log::log_str;
use crate::font::Glyph;
use crate::shader::Shader;
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::rect::Rect;


use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;

static JITTER_PATTERN : [Vec2<f32>; 6] = [
    Vec2::new(-1.0 / 12.0, -5.0 / 12.0),
    Vec2::new( 1.0 / 12.0,  1.0 / 12.0),
    Vec2::new( 3.0 / 12.0, -1.0 / 12.0),
    Vec2::new( 5.0 / 12.0,  5.0 / 12.0),
    Vec2::new( 7.0 / 12.0, -3.0 / 12.0),
    Vec2::new( 9.0 / 12.0,  3.0 / 12.0),
];

static JITTER_COLORS : [Vec4<f32>; 6] = [
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
];


static STANDARD_QUAD : [f32; 6 * 2] = [
    0.0, 0.0, 
    1.0, 0.0,
    0.0, 1.0,
    1.0, 0.0,
    0.0, 1.0,
    1.0, 1.0
];

pub struct GlyphShader {
    pub shader : Shader,
}

impl GlyphShader {
    pub fn new(context : WebGl2RenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context, 
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
                void main() {
                    if (vBezierParameter.x * vBezierParameter.x > vBezierParameter.y) {
                        discard;
                    }

                    // Upper 4 bits: front faces
                    // Lower 4 bits: back faces
                    gl_FragColor = uColor * ((gl_FrontFacing ? 16.0 : 1.0) / 255.0);
                }
            "#
        )?;
        shader.add_attribute_vec4f("aVertexPosition")?;
        Ok(Self {
            shader
        })
    }

    // pub fn draw(&self, transform : Transform, glyph : &Glyph) -> Result<(), JsValue> {
    pub fn draw(&mut self, transform : Transform, glyph : &Glyph, scale : f32, pixel_density : f64) -> Result<(), JsValue> {
        self.shader.use_program();
        let vertices = glyph.vertices();
        self.shader.set_attribute_data("aVertexPosition", &*vertices)?;
        for (&offset, &color) in JITTER_PATTERN.iter().zip(JITTER_COLORS.iter()) {
            let mut cur_transform = transform;
            cur_transform.translate_vec(offset * ((1.0 / pixel_density) as f32));
            cur_transform.scale(scale, scale);
            self.shader.set_uniform_vec4("uColor", color);
            // self.shader.set_uniform_vec4("uColor", Vec4::new(2.0, 2.0, 2.0, 2.0));
            self.shader.set_uniform_transform("uTransformationMatrix", cur_transform);        
            self.shader.draw(vertices.len())?;
        }
        Ok(())
    }
}



pub struct TextShader {
    pub shader : Shader,
}

impl TextShader {
    pub fn new(context : WebGl2RenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context, 
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
                void main() {
                    // float should_draw =  mod(texture2D(uTexture, vTextureCoord).x * 255.0, 2.0);
                    // if(should_draw == 0.0){
                    //     discard;
                    // }
                    // gl_FragColor = vec4(0.2, 0.7, 0.7, 1);
                    
                    vec2 valueL = texture2D(uTexture, vec2(vTextureCoord.x + dFdx(vTextureCoord.x), vTextureCoord.y)).yz * 255.0;
                    vec2 lowerL = mod(valueL, 16.0);
                    vec2 upperL = (valueL - lowerL) / 16.0;
                    vec2 alphaL = min(abs(upperL - lowerL), 2.0);
                
                    // Get samples for 0, +1/3, and +2/3
                    vec3 valueR = texture2D(uTexture, vTextureCoord).xyz * 255.0;
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
                    gl_FragColor = uColor.a == 0.0 ? 1.0 - rgba : uColor * rgba;
                }
            "#
        )?;
        shader.add_attribute_vec2f(&"aVertexPosition")?;
        Ok(Self {
            shader
        })
    }

    pub fn draw(&mut self, transform : Transform, glyph : &Glyph) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);        

        self.shader.set_attribute_data("aVertexPosition", &STANDARD_QUAD)?;

        let bounding_box = glyph.bounding_box();
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

        self.shader.set_uniform_int("uTexture", 0);
        self.shader.set_uniform_vec4("uBoundingBox", Vec4::new(left, top, right, bottom));
        self.shader.set_uniform_vec4("uColor", Vec4::new(0.0, 0.0, 0.0, 0.0));
        self.shader.draw(6)?;
        Ok(())
    }
}