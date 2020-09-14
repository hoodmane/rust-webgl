// Note: We've been using the pixel screen coordinate system where (0, 0) is the top left hand corner of the screen
// and y is measured down. This is a left-handed coordinate system so the clockwise / counterclockwise calculations
// are a bit screwy =/

// Inspired by: https://github.com/pixijs/pixi.js/blob/3dde07c107d20720dfeeae8d8b2795bfcf86f8ee/packages/graphics/src/utils/

use std::f32::consts::PI;
use std::rc::Rc;
use wasm_bindgen::JsValue;


use crate::vector::Vec2;
use crate::matrix::Transform;
use crate::arrow::Arrow;
use crate::log;

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


// pub struct PolyLineSegment {
//     points : Vec<Vec2>,
//     arrow_spec : ArrowSpec
// }

// struct ArrowSpec {
//     ???
// }


#[derive(Clone, Copy, Debug)]
enum PathCommand {
    LineTo(Vec2),
    ArcTo { end : Vec2, center : Vec2, radius : f32, start_angle : f32, end_angle : f32 },
    QuadraticCurveTo(Vec2, Vec2),
    CubicCurveTo(Vec2, Vec2, Vec2),
    NoOp(Vec2),
}

impl PathCommand {
    fn end_point(self) -> Vec2 {
        match self {
            PathCommand::LineTo(v) => v,
            PathCommand::ArcTo { end , .. } => end,
            PathCommand::QuadraticCurveTo(_c, e) => e,
            PathCommand::CubicCurveTo(_c1, _c2, e) => e,
            PathCommand::NoOp(e) => e
        }
    }
    
}

pub struct Path {
    start : Vec2,
    path : Vec<PathCommand>,
    end_arrow : Option<Rc<Arrow>>
}

impl Path {
    pub fn new(start : Vec2) -> Self {
        Path {
            start,
            path : vec![],
            end_arrow : None
        }
    }

    pub fn line_to(&mut self, to : Vec2) {
        self.path.push(PathCommand::LineTo(to));
    }

    pub fn quadratic_curve_to(&mut self, c1 : Vec2, to : Vec2) {
        self.path.push(PathCommand::QuadraticCurveTo(c1, to));
    }

    pub fn cubic_curve_to(&mut self, c1 : Vec2, c2 : Vec2, to : Vec2) {
        self.path.push(PathCommand::CubicCurveTo(c1, c2, to));
    }

    pub fn arc_to(&mut self, q : Vec2, theta : f32) {
        if theta == 0.0 {
            self.line_to(q);
            return;
        }
        let p = self.last_point();
        let pq = q - p;
        // half of distance between p and q
        let d = pq.magnitude() * 0.5;
        if d == 0.0 {
            return;
        }
        // distance from (p1 + p0)/2 to center (negative if we're left-handed)
        let e = d/f32::tan(theta);
        let radius = d/f32::sin(theta);
        let pq_perp = pq.perp().normalize() * -e;
        let center = (p + q) * 0.5 + pq_perp;
        let start_angle = ((p - center) * theta.signum()).angle();
        let end_angle = start_angle - 2.0 * theta;
        log!("start_angle : {} end_angle : {}", start_angle, end_angle);

        self.path.push(PathCommand::ArcTo { end : q, center, radius, start_angle, end_angle });
    }

    fn previous_point(&self, command_index : usize) -> Vec2 {
        match command_index {
            0 => self.start,
            _ => self.path[command_index - 1].end_point()
        }
    }

    fn last_point(&self) -> Vec2 {
        self.previous_point(self.path.len())
    }

    // fn segment_length(&self, command_index : usize) -> f32 {
    //     let previous_point = self.previous_point(command_index);
    //     match self.path[command_index] {
    //         PathCommand::LineTo(end_point) => (end_point - previous_point).magnitude(),
    //         PathCommand::ArcTo(end)
    //     }
    // }

    fn segment_start_end_distance(&self, command_index : usize) -> f32 {
        let start_point = self.previous_point(command_index);
        let end_point = self.path[command_index].end_point();
        (end_point - start_point).magnitude()
    }

    fn delete_first_command(&mut self) {
        let first_command_endpoint = self.path[0].end_point();
        self.start = first_command_endpoint;
        self.path[0] = PathCommand::NoOp(first_command_endpoint);
    }

    fn delete_last_command(&mut self) {
        self.path.pop();
    }

    pub fn shorten_start(&mut self, shorten : f32){
        // TODO: what if shorten > length?
        let idx = 0;
        match self.path[idx] {
            PathCommand::LineTo(end_point) => { 
                let segment_length = self.segment_start_end_distance(idx);
                if shorten > segment_length {
                    self.delete_first_command();
                    return;
                }
                self.start += (end_point - self.start).normalize() * shorten;
            }
            PathCommand::ArcTo { end, center, radius, mut start_angle, end_angle } => {
                if shorten > 2.0 * radius {
                    if (end_angle - start_angle).abs() > PI {
                        start_angle += PI * (end_angle - start_angle).signum();
                    } else {
                        self.delete_first_command();
                        return;
                    }
                } else {
                    start_angle += 2.0 * f32::asin(shorten / (2.0 * radius));
                }
                self.path[idx] = PathCommand::ArcTo { end, center, radius, start_angle, end_angle};
                self.start = center + Vec2::direction(start_angle) * radius;
            }
            PathCommand::NoOp(_) => {}
            _ => unimplemented!()
        }
    }

    pub fn shorten_end(&mut self, shorten : f32){
        let idx = self.path.len() - 1;
        let previous_point = self.previous_point(idx);
        match self.path[idx] {
            PathCommand::LineTo(mut end_point) => { 
                let segment_length = self.segment_start_end_distance(idx);
                if shorten > segment_length {
                    self.delete_last_command();
                    return;
                }
                end_point -= (end_point - previous_point).normalize() * shorten;
            }
            PathCommand::ArcTo { end : _, center, radius, start_angle, mut end_angle } => {
                if shorten > 2.0 * radius {
                    if (start_angle - end_angle).abs() > PI {
                        end_angle -= PI * (start_angle - end_angle).signum();
                    } else {
                        self.delete_last_command();
                        return;
                    }
                } else {
                    end_angle -= 2.0 * f32::asin(shorten / (2.0 * radius));
                }
                let end = center + Vec2::direction(end_angle) * radius;
                self.path[0] = PathCommand::ArcTo { end, center, radius, start_angle, end_angle};
            }
            PathCommand::NoOp(_) => {}
            _ => unimplemented!()
        }
    }


    fn get_points(&self) -> impl Iterator<Item = Vec2>  + '_ {
        std::iter::once(self.start).chain(self.path.iter().enumerate().flat_map(move |(idx, &command)|{
            let from = self.previous_point(idx);
            match command {
                PathCommand::LineTo(to) => PathCommandIterator::line(to),
                PathCommand::QuadraticCurveTo(cp, to) => PathCommandIterator::quadratic(from, cp, to),
                PathCommand::CubicCurveTo(cp1, cp2, to) => PathCommandIterator::cubic(from, cp1, cp2, to),
                PathCommand::ArcTo {end : _, center, radius, start_angle, end_angle } => PathCommandIterator::arc(center, radius, start_angle, end_angle),
                PathCommand::NoOp(_) => PathCommandIterator::noop()
            }
        }))
    }

    fn get_points_with_transform(&self, offset : Vec2, angle : f32) -> impl Iterator<Item = Vec2>  + '_ {
        let mut t = Transform::new();
        t.translate_vec(offset);
        t.rotate(angle);
        std::iter::once(self.start).chain(self.path.iter().enumerate().flat_map(move |(idx, &command)|{
            let from = self.previous_point(idx);
            match command {
                PathCommand::LineTo(to) => PathCommandIterator::line(t.transform_point(to)),
                PathCommand::QuadraticCurveTo(cp, to) => PathCommandIterator::quadratic(t.transform_point(from), t.transform_point(cp), t.transform_point(to)),
                PathCommand::CubicCurveTo(cp1, cp2, to) => PathCommandIterator::cubic(t.transform_point(from), t.transform_point(cp1), t.transform_point(cp2), t.transform_point(to)),
                PathCommand::ArcTo {end : _, center, radius, start_angle, end_angle } => {
                    let new_center = t.transform_point(center);
                    let new_start_angle = start_angle + angle;
                    let new_end_angle = end_angle + angle;
                    PathCommandIterator::arc(new_center, radius, new_start_angle, new_end_angle)
                },
                PathCommand::NoOp(_) => PathCommandIterator::noop()
            }
        }))
    }


    pub fn get_triangles(&self, output : &mut Vec<Vec2>, style : LineStyle) {
        self.get_triangles_helper(output, style);
    }

    // A helper function so that we can use ?.
    fn get_triangles_helper(&self, output : &mut Vec<Vec2>, style : LineStyle) -> Option<()> {
        let mut builder = PolyLineTriangleBuilder::new(output, style);
        let closed_shape = false;
        let mut points = self.get_points();
        let mut p0 = points.next()?;
        let mut p1 = points.next()?;

        // log!("p0 : {:?}", p0);
        // log!("p1 : {:?}", p1);
        builder.start_line(p0, p1, !closed_shape);
        // let start_idx = builder.setup_move();
        for p2 in points {
            // log!("new point : {:?}", p2);
            builder.line_join(p0, p1, p2);
            p0 = p1;
            p1 = p2;
        }
        builder.end_line(p0, p1, !closed_shape && self.end_arrow.is_none());
        // builder.finish_move(start_idx);
        // if let Some(arrow) = self.end_arrow {
        //     builder.add_end_arrow(arrow);
        // }
        Some(())
    }

}

enum PathCommandIterator {
    Quadratic(QuadraticCurveIterator),
    Cubic(CubicCurveIterator),
    Arc(ArcIterator),
    Empty(std::iter::Empty<Vec2>),
    Once(std::iter::Once<Vec2>),
}

impl PathCommandIterator {
    fn noop() -> Self {
        Self::Empty(std::iter::empty())
    }
    
    fn line(to : Vec2) -> Self {
        Self::Once(std::iter::once(to))
    }

    fn quadratic(from : Vec2, cp : Vec2, to : Vec2) -> Self {
        Self::Quadratic(QuadraticCurveIterator::new(from, cp, to))
    }

    fn cubic(from : Vec2, cp1 : Vec2, cp2 : Vec2, to : Vec2) -> Self {
        Self::Cubic(CubicCurveIterator::new(from, cp1, cp2, to))
    }

    fn arc(center : Vec2, radius : f32, start_angle : f32, end_angle : f32) -> Self {
        Self::Arc(ArcIterator::new(center, radius, start_angle, end_angle))
    }
}

impl Iterator for PathCommandIterator {
    type Item = Vec2;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PathCommandIterator::Quadratic(it) => it.next(),
            PathCommandIterator::Cubic(it) => it.next(),
            PathCommandIterator::Arc(it) => it.next(),
            PathCommandIterator::Empty(it) => it.next(),
            PathCommandIterator::Once(it) => it.next(),
        }
    }
}

struct QuadraticCurveIterator { from : Vec2, cp : Vec2, to : Vec2, i : usize, n : usize }

impl QuadraticCurveIterator {
    fn new(from : Vec2, cp : Vec2, to : Vec2) -> Self {
        let i = 0;
        let n = segments_count(quadratic_curve_length(from, cp, to));
        Self { from, cp, to, i, n}
    }
}

impl Iterator for QuadraticCurveIterator {
    type Item = Vec2;
    
    fn next(&mut self) -> Option<Self::Item>{
        self.i += 1;
        if self.i > self.n {
            return None;
        }
        let t = (self.i as f32) / (self.n as f32);
        Some(quadratic_bezier_point(self.from, self.cp, self.to, t))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (n, Some(n))
    }
}

struct CubicCurveIterator { from : Vec2, cp1 : Vec2, cp2 : Vec2, to : Vec2, i : usize, n : usize }

impl CubicCurveIterator {
    fn new(from : Vec2, cp1 : Vec2, cp2 : Vec2, to : Vec2) -> Self {
        let i = 0;
        let n = segments_count(cubic_curve_length(from, cp1, cp2, to));
        Self { from, cp1, cp2, to, i, n}
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (n, Some(n))
    }
}

impl Iterator for CubicCurveIterator {
    type Item = Vec2;
    
    fn next(&mut self) -> Option<Self::Item>{
        self.i += 1;
        if self.i > self.n {
            return None;
        }
        let t = (self.i as f32) / (self.n as f32);
        Some(cubic_bezier_point(self.from, self.cp1, self.cp2, self.to, t))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (n, Some(n))
    }
}

struct ArcIterator { center : Vec2, radius : f32, start_angle : f32, sweep : f32, i : usize, n : usize }

impl ArcIterator {
    fn new(center : Vec2, radius : f32, start_angle : f32, end_angle : f32) -> Self {
        let sweep = end_angle - start_angle;
        let i = 0;
        let n = segments_count(f32::abs(sweep) * radius);
        Self { center, radius, start_angle, sweep, i, n}
    }
}

impl Iterator for ArcIterator {
    type Item = Vec2;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;
        if self.i > self.n {
            return None;
        }
        let t = (self.i as f32) / (self.n as f32);
        Some(self.center + Vec2::direction(self.start_angle + self.sweep * t) * self.radius)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (n, Some(n))
    }
}




struct PolyLineTriangleBuilder<'a> {
    outputs : &'a mut Vec<Vec2>,
    style : LineStyle
}

impl<'a> PolyLineTriangleBuilder<'a> {
    fn new(outputs : &'a mut Vec<Vec2>, style : LineStyle) -> Self {
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
    
    // We are going to print these triangles with TRIANGLE_STRIP primitive.
    // In order to make two disconnected strips, we double up the last vertex of the first strip
    // and the first vertex of the second strip, connecting the two strips by two degenerate triangles.
    // We use setup_move and finish_move to handle this.
    fn setup_move(&mut self) -> usize {
        // If there are prior vertices we need to do a move
        if let Some(&last) = self.outputs.last() {
            // Double up last vertex of first strip
            self.outputs.push(last);
            let result = self.outputs.len();
            // Leave a gap to double the first vertex of the next strip. Will replace this with a call to 
            // finish_move later.
            self.outputs.push(Vec2::new(0.0, 0.0));
            result
        } else {
            0
        }
    }

    fn finish_move(&mut self, move_idx : usize) {
        if move_idx > 0 && self.outputs.len() > move_idx + 1 {
            self.outputs[move_idx] = self.outputs[move_idx + 1];
        }
    }



    fn start_line(&mut self, p0 : Vec2, p1 : Vec2, needs_line_cap : bool){
        log!("start_line : p0 : {:?} -- p1 : {:?}", p0, p1);
        let perp = (p0 - p1).perp().normalize() * self.style.width;
        let start = true;
        if needs_line_cap {
            self.line_cap(p0, perp, start);
        }
        self.line_end(p0, perp);
    }

    fn end_line(&mut self, p0 : Vec2, p1 : Vec2, needs_line_cap : bool){
        log!("end_line : p0 : {:?} -- p1 : {:?}", p0, p1);
        let perp = (p0 - p1).perp().normalize() * self.style.width;
        let start = false;
        self.line_end(p1, perp);
        if needs_line_cap {
            self.line_cap(p1, perp, start);
        }
    }

    fn line_end(&mut self, p : Vec2, perp : Vec2){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();        
        self.outputs.push(p - perp * inner_weight);
        self.outputs.push(p + perp * outer_weight);
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

    fn start_arrow(&mut self, ){

    }


    fn line_join(&mut self, p0 : Vec2, p1 : Vec2, p2 : Vec2){
        let width = self.style.width;
        let perp0 = (p0 - p1).perp().normalize() * width;
        let perp1 = (p1 - p2).perp().normalize() * width;
        
        // Going nearly straight?
        let cross = Vec2::cross(p0 - p1, p1 - p2);
        let clockwise = cross > 0.0;
        if f32::abs(cross) < 0.1 {
            self.outputs.push(p1 - perp0 * self.inner_weight());
            self.outputs.push(p1 + perp0 * self.outer_weight());
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
                    self.outputs.push(inner_miter);
                    self.outputs.push(outer_miter);
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
            self.outputs.push(point);
        }
    }

    fn round_join(&mut self, clockwise : bool, p1 : Vec2, inner_miter : Vec2, outer_miter : Vec2, perp0 : Vec2, perp1 : Vec2){
        let inner_weight = self.inner_weight();
        let outer_weight = self.outer_weight();
        if clockwise { // arc is outside
            let start = p1 + perp0 * outer_weight;
            let end = p1 + perp1 * outer_weight;
            self.outputs.push(inner_miter);
            self.outputs.push(start);

            self.arc(p1, start, end, !clockwise);

            self.outputs.push(inner_miter);
            self.outputs.push(end);
        } else { // arc is inside
            let start = p1 - perp0 * inner_weight;
            let end = p1 - perp1 * inner_weight;
            self.outputs.push(start);
            self.outputs.push(outer_miter);
            self.arc(p1, start, end, !clockwise);
            self.outputs.push(end);
            self.outputs.push(outer_miter);
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
            self.outputs.push(center);
        }

        for i in 0 .. seg_count {
            let angle = angle0 + (i as f32) * angle_increment;
            self.outputs.push(center + Vec2::direction(angle) * radius);
            self.outputs.push(center);
        }
        self.outputs.push(end);
        if clockwise {
            self.outputs.push(center);
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
        self.outputs.push(inner + extension);
        self.outputs.push(outer + extension);
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
