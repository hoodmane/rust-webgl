use crate::vector::{Vec2, Vec2Buffer, Vec4, Vec4Buffer};
use crate::matrix::{Matrix3, Matrix4};
use crate::shader::Shader;
use crate::log::log_str;

use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;
use std::ops::{Add, Mul};
use std::cmp::Ordering;

fn distance_sq(a : Vec2<f32>, b : Vec2<f32>) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

// compute the distance squared from p to the line segment
// formed by v and w
fn distance_to_segment_sq(p : Vec2<f32>, v : Vec2<f32>, w : Vec2<f32>) -> f32 {
    let l2 = distance_sq(v, w);
    if l2 == 0.0 {
        return distance_sq(p, v);
    }
    let mut t = ((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2;
    t = t.min(1.0).max(0.0);
    distance_sq(p, v * (1.0 - t) +  w * t)
}

fn barycentric_coordinates(triangle : [Vec2<f32>; 3], p : Vec2<f32>) -> (f32, f32, f32) {
    let [a, b, c] = triangle;
    let det_t = (a.x - c.x) * (b.y - c.y) - (b.x - c.x) * (a.y - c.y);
    let numerator1 = (p.x  - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
    let numerator2 = (a.x - c.x) * (p.y  - c.y) - (p.x  - c.x) * (a.y - c.y);
    let l1 = numerator1/det_t;
    let l2 = numerator2/det_t;
    let l3 = 1.0 - l1 - l2;
    (l1, l2, l3)
}

fn barycentric_average<V : Add<Output = V> + Mul<f32, Output=V>>(
    triangle : [ Vec2<f32> ; 3], 
    v : [V ; 3], 
    point : Vec2<f32>
) -> V {
    let [v1, v2, v3] = v;
    let (l1, l2, l3) = barycentric_coordinates(triangle, point);
    v1 * l1 + v2 * l2 + v3 * l3
}



fn flatness(points : &Vec2Buffer<f32>, offset : usize) -> f32 {
    let p1 = points.get(offset + 0);
    let p2 = points.get(offset + 1);
    let p3 = points.get(offset + 2);
    let p4 = points.get(offset + 3);

    let mut ux = 3.0 * p2.x - 2.0 * p1.x - p4.x; ux *= ux;
    let mut uy = 3.0 * p2.y - 2.0 * p1.y - p4.y; uy *= uy;
    let mut vx = 3.0 * p3.x - 2.0 * p4.x - p1.x; vx *= vx;
    let mut vy = 3.0 * p3.y - 2.0 * p4.y - p1.y; vy *= vy;

    if ux < vx {
        ux = vx;
    }

    if uy < vy {
        uy = vy;
    }
    ux + uy
}

fn get_points_on_bezier_curve_with_splitting(points : &mut Vec2Buffer<f32>, offset : usize, tolerance : f32, out_points : &mut Vec2Buffer<f32>) {
    if flatness(points, offset) < tolerance {
        // just add the end points of this curve
        out_points.push_vec(points.get(offset + 0));
        // out_points.push_vec(points.get(offset + 3));
        return;
    } 

    // subdivide
    let t = 0.5;
    let p1 = points.get(offset + 0);
    let p2 = points.get(offset + 1);
    let p3 = points.get(offset + 2);
    let p4 = points.get(offset + 3);

    let q1 = p1 * (1.0 - t) + p2 * t;
    let q2 = p2 * (1.0 - t) + p3 * t;
    let q3 = p3 * (1.0 - t) + p4 * t;

    let r1 = q1 * (1.0 - t) + q2 * t;
    let r2 = q2 * (1.0 - t) + q3 * t;

    let red = r1 * (1.0 - t) + r2 * t;

    // do 1st half
    points.set_vec(0, p1);
    points.set_vec(1, q1);
    points.set_vec(2, r1);
    points.set_vec(3, red);
    get_points_on_bezier_curve_with_splitting(points, 0, tolerance, out_points);


    // do 2nd half
    points.set_vec(0, red);
    points.set_vec(1, r2);
    points.set_vec(2, q3);
    points.set_vec(3, p4);
    get_points_on_bezier_curve_with_splitting(points, 0, tolerance, out_points);
}


fn get_points_on_bezier_curve(p1 : Vec2<f32>, p2 : Vec2<f32>, p3 : Vec2<f32>, p4 : Vec2<f32>, tolerance : f32) -> Vec2Buffer<f32> {
    let mut points = Vec2Buffer::new();
    points.push_vec(p1);
    points.push_vec(p2);
    points.push_vec(p3);
    points.push_vec(p4);
    let mut new_points = Vec2Buffer::new();
    let num_segments = (points.len() - 1) / 3;
    for i in 0 .. num_segments {
        let offset = i * 3;
        get_points_on_bezier_curve_with_splitting(&mut points, offset, tolerance, &mut new_points);
    }
    new_points.push_vec(p4);
    return new_points;
}


fn simplify_points(points : &Vec2Buffer<f32>, start : usize, end : usize, epsilon : f32, out_points : &mut Vec2Buffer<f32>) {
    // find the furthest point from the endpoints
    let s = points.get(start);
    let e = points.get(end - 1);
    let mut max_dist_sq : f32 = 0.0;
    let mut max_ndx = 1;
    for i in start + 1 .. end - 1 {
        let dist_sq = distance_to_segment_sq(points.get(i), s, e);
        if dist_sq > max_dist_sq {
            max_dist_sq = dist_sq;
            max_ndx = i;
        }   
    }

    // if that point is too far
    if max_dist_sq.sqrt() > epsilon {
        // split
        simplify_points(points, start, max_ndx + 1, epsilon, out_points);
        simplify_points(points, max_ndx, end, epsilon, out_points);
    } else {
        // add the 2 end points
        out_points.push_vec(s);
        // out_points.push_vec(e);
    }
}

static TO_BEZIER_BASIS : Matrix4 = Matrix4::new([
    1.0,  0.0,  0.0, 0.0,
   -3.0,  3.0,  0.0, 0.0,
    3.0, -6.0,  3.0, 0.0,
   -1.0,  3.0, -3.0, 1.0
]);

static FROM_BEZIER_BASIS : Matrix4 = Matrix4::new([
    1.0, 0.0, 0.0, 0.0,
    1.0, 1.0/3.0, 0.0, 0.0,
    1.0, 2.0/3.0, 1.0/3.0, 0.0,
    1.0, 1.0, 1.0, 1.0
]);  

// Vec4 { x: -0.011685343, y: 0.00004250087, z: -0.037542794, w: 1.0 }, 
// r2 : Vec4 { x: -0.11227082, y: 0.0012596224, z: 0.068105265, w: 1.0 }, 
// r3 : Vec4 { x: 0.10104364, y: 0.037332144, z: -0.1235477, w: 1.0 }
// r1: (4) [-0.011685325408087255, 0.00004250071971324798, -0.037542755781704684, 1]
// r2: (4) [-0.11227077352868109, 0.0012596193405952489, 0.06810521791767168, 1]
// r3: (4) [0.10104369617581305, 0.03733209446585055, -0.12354768878937478, 1]

fn classify_bezier(p0 : Vec2<f32>, p1 : Vec2<f32>, p2 : Vec2<f32>, p3 : Vec2<f32>) -> (Vec4<f32>, Vec4<f32>, Vec4<f32>) {
    log_str(&format!("classify_bezier : {:?}", p3));
    let Vec2 { x : p0x, y : p0y} = p0;
    let Vec2 { x : p1x, y : p1y} = p1;
    let Vec2 { x : p2x, y : p2y} = p2;
    let Vec2 { x : p3x, y : p3y} = p3;
    let b_matrix = Matrix4::new([
        p0x, p0y, 0.0, 1.0,
        p1x, p1y, 0.0, 1.0,
        p2x, p2y, 0.0, 1.0,
        p3x, p3y, 0.0, 1.0
    ]);
    let c_matrix = TO_BEZIER_BASIS * b_matrix;
    let [ 
        q0x, q0y, _, _,
        q1x, q1y, _, _,
        q2x, q2y, _, _,
        q3x, q3y, _, _,
    ] = c_matrix.data;
    
    // let m0 = Matrix3::new([
    //     q3x, q3y, 1.0,
    //     q2x, q2y, 1.0,
    //     q1x, q1y, 1.0,
    // ]);
    let m1 = Matrix3::new([
        q3x, q3y, 1.0,
        q2x, q2y, 1.0,
        q0x, q0y, 1.0,
    ]);
    let m2 = Matrix3::new([
        q3x, q3y, 1.0,
        q1x, q1y, 1.0,
        q0x, q0y, 1.0,
    ]);
    let m3 = Matrix3::new([
        q2x, q2y, 1.0,
        q1x, q1y, 1.0,
        q0x, q0y, 1.0,
    ]);

    // let d0 = m0.det(); // d0 should always be 0
    let d1 = -m1.det();
    let d2 = m2.det();
    let d3 = -m3.det();
    
    // let discr = 3 * delta2 * delta2 - 4 * delta1 * delta3;
    let discr = 3.0 * d2 * d2 - 4.0 * d1 * d3;
    
    let f_matrix;
    match discr.partial_cmp(&0.0) {
        None => {
            unreachable!();
        }
        Some(Ordering::Equal) => {
            todo!();
        }
        Some(Ordering::Greater) => {
            // Type = CurveTypes.Serpentine;

            // "A somewhat more stable quadratic solution technique should be used"
            let mut sl = 2.0 * d1;
            let mut tl = d2 + ((1.0 / f32::sqrt(3.0)) * discr.sqrt());

            let l_magnitude = (sl*sl + tl*tl).sqrt();
            sl /= l_magnitude;
            tl /= l_magnitude;

            let mut sm = 2.0 * d1;
            let mut tm = d2 - ((1.0 / f32::sqrt(3.0)) * discr.sqrt());
            let m_magnitude = (sm*sm + tm*tm).sqrt();
            sm /= m_magnitude;
            tm /= m_magnitude;
            
            f_matrix = Matrix4::new([
                tl * tm, tl.powi(3), tm.powi(3), 1.0,
                -sm * tl - sl *  tm, -3.0 * sl * tl * tl, -3.0 * sm *  tm *  tm, 0.0,
                sl * sm, 3.0 * sl * sl * tl, 3.0 * sm * sm *  tm, 0.0,
                0.0, -sl * sl * sl, -sm * sm * sm, 0.0,
            ]);
        }
        Some(Ordering::Less) => {
            // Type = CurveTypes.Smooth;
            let mut td = d2 + (-discr).sqrt();
            let mut sd = 2.0 * d1;
            let d_magnitude = (sd*sd + td*td).sqrt();
            sd /= d_magnitude;
            td /= d_magnitude;
        
            let mut te = d2 - (-discr).sqrt();
            let mut se = 2.0 * d1;
            let e_magnitude = (se*se + te*te).sqrt();
            se /= e_magnitude;
            te /= e_magnitude;        
            f_matrix = Matrix4::new([
                td * te,                   td * td * te,                               td * te * te,                           1.0,      
                -se * td - sd * te,      -se * td * td - 2.0 * sd * te * td,     -sd * te * te - 2.0 * se * td * te,           0.0,
                sd * se,                   te * sd * sd + 2.0 * se * td * sd,           td * se * se + 2.0 * sd * te * se,     0.0,
                0.0,                         -sd * sd * se,                              -sd * se * se,                        0.0
            ]);
        }
    }
    let result = FROM_BEZIER_BASIS * f_matrix;
    let r1 = result.row(0);
    let r2 = result.row(1);
    let r3 = result.row(2);
    return (r1, r2, r3);
}


pub struct CubicBezierShader {
    pub shader : Shader,
    vertices : Vec2Buffer<f32>,
    bezier_helper_coords : Vec4Buffer<f32>,
}

impl CubicBezierShader {
    pub fn new(context : WebGl2RenderingContext) -> Result<Self, JsValue> {
        let mut shader = Shader::new(
            context,
            // vertexShader : 
            r#"#version 300 es
                in vec2 aVertexPosition;
                in vec4 aBezierParameter;
                out vec4 vBezierParameter;
                uniform mat3 uTransformationMatrix;
                void main() {
                    vBezierParameter = aBezierParameter;
                    // gl_Position = vec4(uTransformationMatrix * vec3(aVertexPosition, 1.0), 0.0).xywz;
                    gl_Position = vec4(vec3(aVertexPosition, 1.0), 0.0).xywz;
                }
            "#,
            // fragmentShader :
            r#"#version 300 es
                precision highp float;
                uniform vec4 uColor;
                in vec4 vBezierParameter;
                out vec4 outColor;
                void main() {
                    vec4 P0 = vBezierParameter;
                    float k0 = P0.x;
                    float l0 = P0.y;
                    float m0 = P0.z;
                    float n0 = P0.w;
                    float C0 = pow(k0, 3.) - l0 * m0 * n0;
                    float thickness = 2.;
                    vec2 scaledGradient = ((C0 > 0.) ? -1. : 1.) * thickness * normalize(vec2(
                        3. * k0 * k0 * dFdx(k0) - dFdx(l0) * m0 * n0 - l0 * dFdx(m0) * n0 - l0 * m0 * dFdx(n0),
                        3. * k0 * k0 * dFdy(k0) - dFdy(l0) * m0 * n0 - l0 * dFdy(m0) * n0 - l0 * m0 * dFdy(n0)
                    ));
                    mat4 partials = mat4(dFdx(vBezierParameter), dFdy(vBezierParameter), vec4(0), vec4(0));
                    vec4 P1 = (vBezierParameter + partials * vec4(scaledGradient, 0, 0));
                    float k1 = P1.x;
                    float l1 = P1.y;
                    float m1 = P1.z;
                    float n1 = P1.w;
                    float C1 = pow(k1, 3.) - l1 * m1 * n1;

                    if(C0 < 0. && C1 < 0. || C0 > 0. && C1 > 0. ){
                        // discard;
                        outColor = vec4(1, 0,0,1);
                    } else {
                        // gl_FragColor = vec4(0.1, 0.1, 0.1, 1);
                        outColor = vec4(0, 0, 0, 1);
                    }
    
                    // Upper 4 bits: front faces
                    // Lower 4 bits: back faces
                    // gl_FragColor = vec4(C0, 0,0,1);//color * (gl_FrontFacing ? 16.0 / 255.0 : 1.0 / 255.0);
                }
            "#
        )?;
        shader.add_attribute_vec2f("aVertexPosition", false)?;
        shader.add_attribute_vec4f("aBezierParameter", false)?;
        Ok(Self {
            shader,
            vertices : Vec2Buffer::new(),
            bezier_helper_coords : Vec4Buffer::new()
        })
    }

    pub fn add_cubic_bezier(&mut self, q1 : Vec2<f32>, q2 : Vec2<f32>, q3 : Vec2<f32>, q4 : Vec2<f32>) {
        let tolerance = 0.001;
        let distance = 0.001;
        let p1 = q1 - q1;
        let p2 = q2 - q1;
        let p3 = q3 - q1;
        let p4 = q4 - q1;
        let (r1, r2, r3) = classify_bezier(p1, p2, p3, p4);
        
        let temp_points = get_points_on_bezier_curve(p1, p2, p3, p4, tolerance);
        let mut bezier_points = Vec2Buffer::new();
        simplify_points(&temp_points, 0, temp_points.len(), distance, &mut bezier_points);
        bezier_points.push_vec(p4);

        let mut normals = Vec2Buffer::new();
        for i in 0 .. bezier_points.len() - 1 {
            let Vec2 { x : x0, y : y0 } = bezier_points.get(i);
            let Vec2 { x : x1, y : y1 } = bezier_points.get(i+1);
            let dx = x1 - x0;
            let dy = y1 - y0;
            let magnitude = f32::sqrt(dx * dx + dy * dy);
            let scale = 0.1;
            let n = Vec2::new(-dy * scale / magnitude, dx * scale / magnitude);
            normals.push_vec(n);
        }
        normals.push_vec(normals.get(normals.len() - 1));

        let mut vertices = Vec::new();
        for i in 0 ..  bezier_points.len() {
            let p = bezier_points.get(i);
            let n = normals.get(i);
            vertices.push([p + n, p - n]);
        }
    
        let helper_coords : Vec<_> = vertices.iter().map(|&[pp, pm]| 
            [
                barycentric_average([p1, p2, p3], [r1, r2, r3], pp),
                barycentric_average([p1, p2, p3], [r1, r2, r3], pm),
            ]
        ).collect();
    
        for i in 0 .. bezier_points.len() - 1 {
            for &(idx, pm) in &[(i, 0), (i, 1), (i+1, 1), (i, 0), (i + 1, 0), (i+1, 1)] {
                self.vertices.push_vec(vertices[idx][pm] + q1);
                self.bezier_helper_coords.push_vec(helper_coords[idx][pm])
            }
        }
    }

    pub fn draw(&self) -> Result<(), JsValue> {
        self.shader.use_program();
        let mut geometry = self.shader.create_geometry()?;
        self.shader.set_attribute_data(&mut geometry, "aVertexPosition", &*self.vertices)?;
        self.shader.set_attribute_data(&mut geometry,"aBezierParameter", &*self.bezier_helper_coords)?;
        self.shader.draw(&geometry)?;
        Ok(())
    }
}
