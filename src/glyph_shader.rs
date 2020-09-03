use crate::log::log_str;
use crate::font::Glyph;
use crate::shader::Shader;
use crate::vector::{Vec2, Vec4};
use crate::matrix::Transform;
use crate::rect::Rect;

use wasm_bindgen::JsValue;
use web_sys::WebGlRenderingContext;

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


pub struct GlyphShader {
    pub shader : Shader,
}

impl GlyphShader {
    pub fn new(context : WebGlRenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context, 
            r#"
                attribute vec4 aVertexPosition;
                varying vec2 vBezierParameter;
                uniform mat3 uTransformationMatrix;
                void main() {
                    vBezierParameter = aVertexPosition.zw;
                    gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition.xy, 1.0), 0.0).xywz;
                    // gl_Position = vec4(vec3(aVertexPosition.x, -aVertexPosition.y, 1.0), 0.0).xywz;
                }
            "#,
            r#"
                precision highp float;
                uniform vec3 uColor;
                varying vec2 vBezierParameter;
                void main() {
                    if (vBezierParameter.x * vBezierParameter.x > vBezierParameter.y) {
                        discard;
                    }

                    // Upper 4 bits: front faces
                    // Lower 4 bits: back faces
                    gl_FragColor = vec4(vec3(1.0, 1.0, 1.0)*(1./255.), 1.0); //  * (gl_FrontFacing ? 16.0 / 255.0 : 1.0 / 255.0), 1.0); //color;
                }
            "#
        )?;
        shader.add_attribute(&"aVertexPosition", 4, WebGlRenderingContext::FLOAT)?;
        Ok(Self {
            shader
        })
    }

    pub fn draw(&self, transform : Transform, glyph : &Glyph) -> Result<(), JsValue> {
        self.shader.use_program();
        let vertices = glyph.vertices();
        self.shader.set_data("aVertexPosition", &*vertices)?;
        self.shader.set_uniform_transform("uTransformationMatrix", transform);        
        self.shader.draw(vertices.len());
        Ok(())
    }
}



pub struct TextShader {
    pub shader : Shader,
}

impl TextShader {
    pub fn new(context : WebGlRenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context, 
            r#"
                uniform vec4 uBoundingBox;
                attribute vec2 aVertexPosition;
                uniform mat3 uTransformationMatrix;
                varying vec2 vTextureCoord;
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
                    // vTextureCoord = aVertexPosition;
                //     vTextureCoord = mix(uBoundingBox.xy, uBoundingBox.zw, aVertexPosition);
                }
            "#,
            r#"
                precision highp float;
                varying vec2 vTextureCoord;
                uniform sampler2D uTexture;
                void main() {
                    float should_draw =  mod(texture2D(uTexture, vTextureCoord).x * 255.0, 2.0);
                    if(should_draw == 0.0){
                        discard;
                    }
                    gl_FragColor = vec4(0.2, 0.7, 0.7, 1);
                    
                    // gl_FragColor = texture2D(uTexture, vTextureCoord);
                    // // Get samples for 0, +1/3, and +2/3
                    // vec3 valueR = texture2D(uTexture, _coord2).xyz * 255.0;
                    // vec3 lowerR = mod(valueR, 16.0);
                    // vec3 upperR = (valueR - lowerR) / 16.0;
                    // vec3 alphaR = min(abs(upperR - lowerR), 2.0);
                
                    // // Average the energy over the pixels on either side
                    // vec4 rgba = vec4(
                    //     (alphaR.x + alphaR.y + alphaR.z) / 6.0,
                    //     (alphaL.y + alphaR.x + alphaR.y) / 6.0,
                    //     (alphaL.x + alphaL.y + alphaR.x) / 6.0,
                    //     0.0);
                
                    // Optionally scale by a color
                    // gl_FragColor = 1.0 - rgba; // color.a == 0.0 ? 1.0 - rgba : color * rgba;
                }
            "#
        )?;
        shader.add_attribute(&"aVertexPosition", 2, WebGlRenderingContext::FLOAT)?;
        Ok(Self {
            shader
        })
    }

    pub fn draw(&self, transform : Transform, glyph : &Glyph) -> Result<(), JsValue> {
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);        

        self.shader.set_data("aVertexPosition", &vec![
            0.0, 0.0, 
            1.0, 0.0,
            0.0, 1.0,
            1.0, 0.0,
            0.0, 1.0,
             1.0, 1.0
        ])?;

        let canvas_width = self.shader.context.drawing_buffer_width() as f32;
        let canvas_height = self.shader.context.drawing_buffer_height() as f32;
        let bounding_box = glyph.bounding_box();
        let left = bounding_box.left();
        let right = bounding_box.right();
        let top = bounding_box.top(); //- 1.0;
        let bottom = bounding_box.bottom();// - 1.0;    



        log_str(&format!("bounding_box : {{ top : {},  bottom : {}, left : {}, right : {}}}", 
            bounding_box.top(), 
            bounding_box.bottom(),
            bounding_box.left(),
            bounding_box.right(),
        ));

        let trans_bb = transform.transform_rect(bounding_box);
        log_str(&format!("trans_bb : {{ top : {},  bottom : {}, left : {}, right : {}}}", 
            trans_bb.top(), 
            trans_bb.bottom(),
            trans_bb.left(),
            trans_bb.right(),
        ));


        log_str(&format!("left : {}", left));
        log_str(&format!("right : {}", right));
        log_str(&format!("top : {}", top));
        log_str(&format!("bottom : {}", bottom));
        self.shader.set_uniform_int("uTexture", 0);
        self.shader.set_uniform_vec4("uBoundingBox", Vec4::new(left, top, right, bottom));
        self.shader.draw(6);
        Ok(())
    }
}