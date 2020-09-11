// Note: We've been using the pixel screen coordinate system where (0, 0) is the top left hand corner of the screen
// and y is measured down. This is a left-handed coordinate system so the clockwise / counterclockwise calculations
// are a bit screwy =/

// Stolen from: https://github.com/pixijs/pixi.js/blob/3dde07c107d20720dfeeae8d8b2795bfcf86f8ee/packages/graphics/src/utils/

use std::f32::consts::PI;

use crate::log::log_str;
use crate::vector::{Vec2, Vec2Buffer};
use wasm_bindgen::JsValue;


#[derive(Clone, Copy, Debug)]
pub enum LineJoinStyle {
    Bevel,
    Round,
    Miter
}

#[derive(Clone, Copy, Debug)]
pub enum LineCapStyle {
    Round,
    Rect,
    Butt
}

#[derive(Clone, Copy, Debug)]
pub struct LineStyle {
    join : LineJoinStyle,
    cap : LineCapStyle,
    width : f32,
    miter_limit : f32,
    alignment : f32
}

impl LineStyle {
    pub fn new(join : LineJoinStyle, cap : LineCapStyle, width : f32, miter_limit : f32, alignment : f32) -> Self{
        Self {
            join, cap, width, miter_limit, alignment
        }
    }
}


pub struct PolyLine {
    points : Vec2Buffer,
}

impl PolyLine {
    pub fn new(start_pt : Vec2) -> Self {
        let mut points = Vec2Buffer::new();
        points.push_vec(start_pt);
        PolyLine {
            points
        }
    }

    pub fn line_to(&mut self, to : Vec2){
        self.points.push_vec(to);
    }

    pub fn cubic_curve_to(&mut self, cp1 : Vec2, cp2 : Vec2, to : Vec2) {
        let from = self.points.last();
        let n = segments_count(cubic_curve_length(from, cp1, cp2, to));
        for i in 1 ..= n {
            let t = (i as f32) / (n as f32);
            self.points.push_vec(cubic_bezier_point(from, cp1, cp2, to, t));
        }
    }

    pub fn quadratic_curve_to(&mut self, cp : Vec2, to : Vec2) {
        let from = self.points.last();
        let n = segments_count(quadratic_curve_length(from, cp, to));
        for i in 1 ..= n {
            let t = (i as f32) / (n as f32);
            self.points.push_vec(quadratic_bezier_point(from, cp, to, t));
        }
    }

    pub fn arc_to(&mut self, q : Vec2, theta : f32) -> Result<(), JsValue> {
        if theta == 0.0 {
            return Err(JsValue::from_str(&"Theta should be nonzero."));
        }
        let p = self.points.last();
        let pq = q - p;
        // half of distance between p and q
        let d = pq.magnitude() * 0.5;
        if d == 0.0 {
            return Err(JsValue::from_str(&"Two points should not be equal."));
        }
        // distance from (p1 + p0)/2 to center (negative if we're left-handed)
        let e = d/f32::tan(theta);
        let radius = d/f32::sin(theta);
        let pq_perp = Vec2::new(pq.y, -pq.x).normalize() * e;
        let center = (p + q) * 0.5 + pq_perp;


        let theta0 = (p - center).angle();
        let sweep = 2.0 * theta;
        let n = segments_count(f32::abs(sweep) * radius);

        for i in 1 ..= n {
            let angle = theta0 + sweep * (i as f32 / n as f32);
            self.points.push_vec(center + Vec2::direction(angle) * radius)
        }
        Ok(())
    }

    pub fn get_triangles(&self, output : &mut Vec2Buffer, style : LineStyle) {
        let mut builder = PolyLineTriangleBuilder::new(output, style);
        let closed_shape = false;
        builder.start_line(self.points.get(0), self.points.get(1), closed_shape);
        for i in 0 .. self.points.len() - 2 {
            builder.line_join(self.points.get(i), self.points.get(i + 1), self.points.get(i + 2));
        }
        builder.end_line(self.points.get(self.points.len() - 2), self.points.get(self.points.len() - 1), closed_shape);
    }
}

struct PolyLineTriangleBuilder<'a> {
    outputs : &'a mut Vec2Buffer,
    style : LineStyle
}

impl<'a> PolyLineTriangleBuilder<'a> {
    fn new(outputs : &'a mut Vec2Buffer, style : LineStyle) -> Self {
        Self {
            outputs,
            style
        }
    }

    fn inner_weight(&self) -> f32 {
        (1.0 - self.style.alignment) * 2.0
    }

    fn outer_weight(&self) -> f32 {
        self.style.alignment * 2.0
    }




    fn start_line(&mut self, p0 : Vec2, p1 : Vec2, closed_shape : bool){
        let perp = (p0 - p1).perp().normalize() * self.style.width;
        let start = true;
        if !closed_shape {
            self.line_cap(p0, perp, start);
        }
        self.line_end(p0, perp);
    }

    fn end_line(&mut self, p0 : Vec2, p1 : Vec2, closed_shape : bool){
        let perp = (p0 - p1).perp().normalize() * self.style.width;
        let start = false;
        self.line_end(p1, perp);
        if !closed_shape {
            self.line_cap(p1, perp, start);
        }
    }

    fn line_end(&mut self, p : Vec2, perp : Vec2){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();        
        self.outputs.push_vec(p - perp * inner_weight);
        self.outputs.push_vec(p + perp * outer_weight);
    }

    fn line_cap(&mut self, p : Vec2, perp : Vec2, start : bool){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();        
        match self.style.cap {
            LineCapStyle::Round => self.arc(
                p - perp * ((inner_weight - outer_weight) * 0.5),
                p - perp * inner_weight,
                p + perp * outer_weight,
                !start,
            ),
            LineCapStyle::Rect => self.rect(p, perp, start),
            LineCapStyle::Butt => ()
        };
    }


    fn line_join(&mut self, p0 : Vec2, p1 : Vec2, p2 : Vec2){
        let width = self.style.width;
        let perp0 = (p0 - p1).perp().normalize() * width;
        let perp1 = (p1 - p2).perp().normalize() * width;
        
        // Going nearly straight?
        let cross = Vec2::cross(p0 - p1, p1 - p2);
        let clockwise = cross > 0.0;
        if f32::abs(cross) < 0.1 {
            self.outputs.push_vec(p1 - perp0 * self.inner_weight());
            self.outputs.push_vec(p1 + perp0 * self.outer_weight());
            return;
        }

        let (miter_point, inner_miter, outer_miter) = Self::compute_miter_points(
            p0, p1, p2, perp0, perp1, self.inner_weight(), self.outer_weight()
        );

        match self.style.join {
            LineJoinStyle::Bevel => {
                self.bevel_join(clockwise, p1, inner_miter, outer_miter, perp0, perp1);
            }
            LineJoinStyle::Round => {
                self.round_join(clockwise, p1, inner_miter, outer_miter, perp0, perp1);
            }
            LineJoinStyle::Miter => {
                let pdist_sq = (miter_point - p1).magnitude_sq();
                let threshold = (width * width) * (self.style.miter_limit * self.style.miter_limit);
                if pdist_sq > threshold {
                    self.bevel_join(clockwise, p1, inner_miter, outer_miter, perp0, perp1);
                } else {
                    self.outputs.push_vec(inner_miter);
                    self.outputs.push_vec(outer_miter);
                }
            }
        }
    }


    fn compute_miter_points(
        p0 : Vec2, p1 : Vec2, p2 : Vec2, 
        perp0 : Vec2, perp1 : Vec2,
        inner_weight : f32, outer_weight : f32
    ) -> (Vec2, Vec2, Vec2) {  
        // positive if internal angle counterclockwise, negative if internal angle clockwise. 
        let cross = Vec2::cross(p1 - p2, p0 - p1);
        let clockwise = cross < 0.0;
    
        // p[x|y] is the miter point. pdist is the distance between miter point and p1. 
        let c1 = Vec2::cross(p0 - perp0, p1 - perp0);
        let c2 = Vec2::cross(p2 - perp1, p1 - perp1);
        let d0 = p0 - p1;
        let d1 = p1 - p2;

        let miter_point = Vec2::new( 
             ((-d0.x * c2) - (d1.x * c1)) / cross,
            -((d0.y * c2) - (-d1.y * c1)) / cross
        );    

        let mut inner_miter = p1 + (miter_point - p1) * inner_weight; 
        let mut outer_miter = p1 - (miter_point - p1) * outer_weight;
    

        let p0_inner = p0 - perp0 * inner_weight;
        let p2_inner = p2 - perp1 * inner_weight;
        
        let p0_outer = p0 + perp0 * outer_weight;
        let p2_outer = p2 + perp1 * outer_weight;


        /* Check if inner miter point is on same side as p1 w.r.t vector p02 */
        // Take normal to v02
        let n02 = (p0 - p2).perp();
        let dot_p1 = Vec2::dot(n02, p1 - p0_inner);
    
        if clockwise {
            let dot_im = Vec2::dot(n02, inner_miter - p0_inner);
    
            // Not on same side? make inner miter point the mid-point instead
            if f32::abs(dot_p1 - dot_im) > 0.1 && dot_p1.is_sign_positive() != dot_im.is_sign_positive()
            {
                inner_miter = (p0_inner + p2_inner) * 0.5;
            }
        }
        else
        {
            let dot_om = Vec2::dot(n02, outer_miter - p0_inner);
            // Not on same side? make outer miter point the mid-point instead
            if f32::abs(dot_p1 - dot_om) > 0.1 && dot_p1.is_sign_positive() != dot_om.is_sign_positive()
            {
                outer_miter = (p0_outer + p2_outer) * 0.5;
            }
        }
        (miter_point, inner_miter, outer_miter)
    }

    fn bevel_join(&mut self, clockwise : bool, p1 : Vec2, inner_miter : Vec2, outer_miter : Vec2, perp0 : Vec2, perp1 : Vec2){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();
        let points = if clockwise {[ // rotating inward
                inner_miter, p1 + perp0 * outer_weight, 
                inner_miter, p1 + perp1 * outer_weight
            ]} else {[// rotating outward
                p1 - perp0 * inner_weight, outer_miter,
                p1 - perp1 * inner_weight, outer_miter
            ]};
        for &point in &points {
            self.outputs.push_vec(point);
        }
    }

    fn round_join(&mut self, clockwise : bool, p1 : Vec2, inner_miter : Vec2, outer_miter : Vec2, perp0 : Vec2, perp1 : Vec2){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();
        if clockwise { // arc is outside
            let start = p1 + perp0 * outer_weight;
            let end = p1 + perp1 * outer_weight;
            self.outputs.push_vec(inner_miter);
            self.outputs.push_vec(start);

            self.arc(p1, start, end, !clockwise);

            self.outputs.push_vec(inner_miter);
            self.outputs.push_vec(end);
        } else { // arc is inside
            let start = p1 - perp0 * inner_weight;
            let end = p1 - perp1 * inner_weight;
            self.outputs.push_vec(start);
            self.outputs.push_vec(outer_miter);
            self.arc(p1, start, end, !clockwise);
            self.outputs.push_vec(end);
            self.outputs.push_vec(outer_miter);
        }
    }


    fn arc(&mut self, 
        center : Vec2, start : Vec2, end : Vec2, clockwise: bool, // if not cap, then clockwise is turn of joint, otherwise rotation from angle0 to angle1
    ){
        let mut angle0 = (start - center).angle();
        let mut angle1 = (end - center).angle();
    
        if clockwise && angle0 < angle1 {
            angle0 += 2.0 * PI;
        }
        if !clockwise && angle0 > angle1 {
            angle1 += 2.0 * PI;
        }
    
        let radius = (start - center).magnitude();
        let seg_count = segments_count(f32::abs(angle1 - angle0) * radius);
        let angle_increment = (angle1 - angle0) / (seg_count as f32);
    
        if !clockwise {
            self.outputs.push_vec(center);
        }

        for i in 0 .. seg_count {
            let angle = angle0 + (i as f32) * angle_increment;
            self.outputs.push_vec(center + Vec2::direction(angle) * radius);
            self.outputs.push_vec(center);
        }
        self.outputs.push_vec(end);
        if clockwise {
            self.outputs.push_vec(center);
        }
    }


    fn rect(&mut self, 
        p : Vec2, perp : Vec2, clockwise : bool, // rotation for square (true at left end, false at right end) 
    ){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();
        let inner = p - perp * inner_weight;
        let outer = p + perp * outer_weight;
        let extension = perp.perp() * if clockwise { -1.0 } else { 1.0 };
        /* Square itself must be inserted clockwise*/
        self.outputs.push_vec(inner + extension);
        self.outputs.push_vec(outer + extension);
    }
}




static MIN_SEGMENTS : usize = 8;
static MAX_SEGMENTS : usize = 2048;
static SEGMENT_LENGTH : f32 = 10.0;

fn segments_count(length : f32) -> usize {
    let result = f32::ceil(length / SEGMENT_LENGTH) as usize;
    usize::min(usize::max(result, MIN_SEGMENTS), MAX_SEGMENTS)
}

pub fn cubic_bezier_point(p0 : Vec2, p1 : Vec2, p2 : Vec2, p3 : Vec2, t : f32) -> Vec2 {
    let t_squared = t * t;
    let t_cubed = t_squared * t;
    let nt = 1.0 - t;
    let nt_squared = nt * nt;
    let nt_cubed = nt_squared * nt;

    p0 * nt_cubed + p1 * (3.0 * nt_squared * t) + p2 * (3.0 * nt * t_squared) + p3 * t_cubed
}

pub fn cubic_curve_length(from : Vec2, cp1 : Vec2, cp2 : Vec2, to : Vec2) -> f32 {
    let n = 10;
    let mut result = 0.0;
    let mut prev = from;

    for i in 1 ..= n {
        let t = (i as f32) / (n as f32);
        let pt = cubic_bezier_point(from, cp1, cp2, to, t);
        result += (prev - pt).magnitude();
        prev = pt;
    }
    result
}




pub fn quadratic_bezier_point(p : Vec2, control : Vec2, q : Vec2, t : f32) -> Vec2 {
    let v = p + (control - p) * t;
    v + (control + (q - control) * t - v) * t
}

pub fn quadratic_curve_length(from : Vec2, cp : Vec2, to : Vec2) -> f32 {
    let v1 = from - cp * 2.0 + to;
    let v2 = (cp - from) * 2.0;
    let a = Vec2::dot(v1, v1); // their a = 4*oura
    let b = Vec2::dot(v1, v2); // their b = 4*ourb
    let c = Vec2::dot(v2, v2);

    let s = f32::sqrt(4.0 * a + 4.0 * b + c); // same
    let a_root = f32::sqrt(a); // their a2 = 2*a_root
    let a_to_3_2 = a * a_root; // a32 = 16 * a_to_3_2
    let c_root = f32::sqrt(c); // c2 = 2 * c_root
    let b_over_root_a = b / a_root; // ba = 2 * b_over_root_a

    let denominator = 4.0 * a_to_3_2;
    let numerator = (2.0 * a_to_3_2 * s) + (a_root * b * (s - c_root)) + 
        ((c * a) - (b * b)) * f32::ln((2.0 * a_root + b_over_root_a + s) / (b_over_root_a + c_root));
    numerator / denominator
}
