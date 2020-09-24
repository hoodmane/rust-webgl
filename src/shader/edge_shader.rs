
use crate::vector::{Vec4};
use crate::shader::{Shader};
use crate::webgl_wrapper::WebGlWrapper;

use lyon::geom::math::{Point, Transform};

use wasm_bindgen::JsValue;
use web_sys::{WebGl2RenderingContext};




#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct EdgeInstance {
    color : Vec4,
    dash_pattern : u8, // Dash pattern a texture?

    num_edge_vertices : u8, // Possibly pack is-it-a-circle in here too?
    edge_index : u16,
    start_arrow_index : u16,
    num_vertices_start_arrow : u16,
    end_arrow_index : u16,
    num_vertices_end_arrow : u16,

    start_tangent_angle : f32,
    start_tip_setback : f32,
    start_line_setback : f32,

    end_tangent_angle : f32,
    end_tip_setback : f32,
    end_line_setback : f32,
}

pub struct EdgeShader {
    webgl : WebGlWrapper,
    shader : Shader,
}


impl EdgeShader {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let shader = Shader::new(
            webgl.clone(), 
            r#"#version 300 es
                uniform mat3x2 uTransformationMatrix;
                uniform vec2 uOrigin;
                uniform vec2 uScale;
                uniform sampler2D uGlyphDataTexture;

                in vec2 aPosition;
                in vec4 aColor;
                in int aGlyphDataIndex;
                in int aGlyphNumVertices;

                flat out vec4 fColor;
                flat out float fCurvature;
                flat out vec2 fP0;
                flat out vec2 fN0;
                flat out float fHalfThickness;
                out vec2 vPosition;


                vec2 testPositions[3] = vec2[](
                    vec2(0.8, 1.34641), vec2(2.2, 1.34641), vec2(1.5, 0.133975)
                );

                vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
                    int texWidth = textureSize(tex, 0).x;
                    int col = index % texWidth;
                    int row = index / texWidth;
                    return texelFetch(tex, ivec2(col, row), 0);
                }        
                
                void main() {
                    vec2 vertexPosition;
                    // if(gl_VertexID < aGlyphNumVertices) {
                    //     int vertexIdx = aGlyphDataIndex + gl_VertexID;
                    //     vertexPosition = getValueByIndexFromTexture(uGlyphDataTexture, vertexIdx).xy;
                    // } else {
                    //     vertexPosition = vec2(0.0, 0.0); // degenerate vertex
                    // }
                    fColor = vec4(0.0, 0.0, 0.0, 1.0);
                    fCurvature = 1.0;
                    fP0 = vec2(1.0, 1.0);
                    fN0 = vec2(-0.5, 0.8660254037844);
                    fHalfThickness = 0.1;

                    vec2 position = testPositions[gl_VertexID];

                    vec2 transformedPosition = uOrigin +  uScale * position;
                    gl_Position = vec4(uTransformationMatrix * vec3(transformedPosition + vertexPosition, 1.0), 0.0, 1.0);
                    vPosition = position;
                    // fColor = aColor;
                }
            "#,
            r#"#version 300 es
                precision highp float;
                flat in vec4 fColor;
                flat in float fCurvature;
                flat in vec2 fP0;
                flat in vec2 fN0;
                flat in float fHalfThickness;
                in vec2 vPosition;
                out vec4 outColor;

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

                float circleConstraint(float ab_dot_ab, float ab_dot_n, float epsilon, float curvature){
                    float numerator = -2.0 * (ab_dot_n + epsilon);
                    float denominator = ab_dot_ab + 2.0 * epsilon * ab_dot_n + epsilon * epsilon;
                    float C_e = curvature * epsilon;
                    float C_2_e = curvature * C_e;
                    float comparison = curvature + C_2_e + C_2_e * C_e;
                    return numerator/denominator - comparison;
                }

                void main() {
                    bool weHaveDashPattern = false;
                    outColor = fColor;
                    if(fCurvature == 0.0){
                        if(weHaveDashPattern){
                            float arc_length = length(vPosition - fP0);
                            // Sample from dash pattern texture
                        }
                        return;
                    }
                    vec2 ab = vPosition - fP0;
                    float ab_dot_n = dot(ab, fN0);
                    float ab_dot_ab = dot(ab, ab);
                    if(weHaveDashPattern){
                        float ab_length = sqrt(ab_dot_ab);
                        float sin_theta = ab_dot_n / ab_length;
                        float theta = asin(sin_theta);
                        float arc_length = (theta/sin_theta) * ab_length;
                        // TODO: Now sample from dash pattern texture
                    }
                    float inner_bound =   circleConstraint(ab_dot_ab, ab_dot_n, - fHalfThickness, fCurvature);
                    float outer_bound = - circleConstraint(ab_dot_ab, ab_dot_n,   fHalfThickness, fCurvature);
                    float bound = min(inner_bound, outer_bound);
                    float alpha = aaStep(0.0, bound);
                    outColor.a *= alpha;

                    if(alpha == 0.0) {
                        discard;
                    }
                }
            "#
        )?;
        Ok(Self {
            webgl,
            shader
        })
    }

    pub fn draw(&mut self, transform : Transform, origin : Point, scale : Point){
        self.shader.use_program();
        self.shader.set_uniform_transform("uTransformationMatrix", transform);
        self.shader.set_uniform_point("uOrigin", origin);
        self.shader.set_uniform_point("uScale", scale);
        self.webgl.draw_arrays_instanced(
            WebGl2RenderingContext::TRIANGLES,
            0,
            3,
            1
        );
    }
}