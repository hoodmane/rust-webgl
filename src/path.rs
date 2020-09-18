use std::convert::Into;

use lyon::geom::math::{Point, point, vector, Vector, Angle, Transform};
use lyon::geom::{Arc, CubicBezierSegment, QuadraticBezierSegment, LineSegment};
use lyon::path::{PathEvent, iterator::PathIterator};

use crate::path_segment::{PathSegment, PathSegmentIterator};
use crate::arrow::Arrow;

pub struct Path {
    start : Point,
    path : Vec<PathSegment<f32>>,
    end_arrow : Option<(Arrow, Transform)>
}

impl Path {
    pub fn new<T : Into<Point>>(start : T) -> Self {
        Self {
            start : start.into(), 
            path : Vec::new(),
            end_arrow : None
        }
    }

    fn previous_point(&self, command_index : usize) -> Point {
        match command_index {
            0 => self.start,
            _ => self.path[command_index - 1].to()
        }
    }

    
    fn last_point(&self) -> Point {
        self.previous_point(self.path.len())
    }

    pub fn line_to<T : Into<Point>>(&mut self, to : T){
        let from = self.last_point();
        self.path.push(PathSegment::line(from, to.into()))
    }


    pub fn arc_to<T : Into<Point>>(&mut self, q : T, theta : f32) {
        if theta == 0.0 {
            self.line_to(q);
            return;
        }
        let p = self.last_point();
        let q : Point = q.into();
        let pq = q - p;
        // half of distance between p and q
        let d = pq.length() * 0.5;
        if d == 0.0 {
            return;
        }
        // distance from (p1 + p0)/2 to center (negative if we're left-handed)
        let e = d/f32::tan(theta);
        let radius = d/f32::sin(theta);
        let pq_perp = pq.yx().reflect(vector(1.0, 0.0)).normalize() * -e;
        let center = (p + q.to_vector()) * 0.5 + pq_perp;
        let start_angle = ((p - center) * theta.signum()).angle_from_x_axis();
        let end_angle = start_angle - Angle::radians(2.0 * theta);
        // log!("start_angle : {} end_angle : {}", start_angle, end_angle);
        // center: Point, radii : Vector, start_angle: Angle, sweep_angle: Angle, x_rotation: Angle
        self.path.push(PathSegment::arc(center, vector(radius, radius), start_angle, end_angle - start_angle, Angle::radians(0.0)));
    }

    pub fn quadratic_curve_to<T : Into<Point>>(&mut self, ctrl : T, to : T){
        let from = self.last_point();
        self.path.push(PathSegment::quadratic(from, ctrl.into(), to.into()));
    }

    pub fn cubic_curve_to<T : Into<Point>>(&mut self, ctrl1 : T, ctrl2 : T, to : T){
        let from = self.last_point();
        self.path.push(PathSegment::cubic(from, ctrl1.into(), ctrl2.into(), to.into()));
    }


    pub fn shorten_start(&mut self, tolerance : f32, mut shorten : f32) {
        if shorten <= 0.0 {
            return;
        }
        for segment in &mut self.path {
            match segment.shorten(tolerance, shorten) {
                Ok(seg) => {
                    self.start = seg.from();
                    *segment = seg;
                    return;
                }
                Err(remaining) => {
                    shorten = remaining;
                    *segment = PathSegment::NoOp;
                }
            }

        }
    }

    pub fn shorten_end(&mut self, tolerance : f32, mut shorten : f32) {
        if shorten <= 0.0 {
            return;
        }
        for segment in self.path.iter_mut().rev() {
            let flipped_segment = segment.flip();
            match flipped_segment.shorten(tolerance/20.0, shorten) {
                Ok(flip_seg) => {
                    self.start = flip_seg.to();
                    *segment = flip_seg.flip();
                    return;
                }
                Err(remaining) => {
                    shorten = remaining;
                    *segment = PathSegment::NoOp;
                }
            }

        }
    }

    fn sample_start(&self, time : f32) -> Point {
        self.path[0].sample(time)
    }

    fn sample_end(&self, time : f32) -> Point {
        self.path.last().unwrap().sample(1.0 - time)
    }

    fn find_time_for_distance_from_start(&self, tolerance : f32, distance : f32) -> Result<f32, f32> {
        self.path.get(0).ok_or(distance)?.find_time_for_distance_from_start(tolerance, distance)
    }

    fn find_time_for_distance_from_end(&self, tolerance : f32, distance : f32) -> Result<f32, f32> {
        self.path.last().ok_or(distance)?.flip().find_time_for_distance_from_start(tolerance, distance)
    }

    pub fn event_iterator(&self) -> impl Iterator<Item = PathEvent> + '_ {
        fn path_segment_iter(seg : &PathSegment<f32>) -> PathSegmentIterator {
            seg.iter()
        }
        std::iter::once(PathEvent::Begin { at : self.start}).chain(
            self.path.iter().flat_map(path_segment_iter as fn(seg : &PathSegment<f32>) -> PathSegmentIterator)
        ).chain(
            std::iter::once(PathEvent::End { first : self.start, last : self.last_point(), close : false })
        ).chain(
            self.end_arrow_event_iterator().into_iter().flatten()
        )
    }

    pub fn add_end_arrow(&mut self, tolerance : f32, arrow : Arrow) {
        let line_setback = arrow.visual_tip_end - arrow.line_end;
        let visual_end_setback = arrow.visual_tip_end - arrow.visual_back_end;
        let visual_end_time = self.find_time_for_distance_from_end(tolerance, visual_end_setback).unwrap();
        let visual_end_point = self.sample_end(visual_end_time);
        let end_point = self.last_point();
        let angle = (end_point - visual_end_point).angle_from_x_axis();
        let transform = Transform::translation(-arrow.visual_tip_end, 0.0).then_rotate(angle).then_translate(end_point.to_vector());
        self.end_arrow = Some((arrow, transform));
    }

    fn end_arrow_event_iterator(&self)  -> Option<impl Iterator<Item = PathEvent> + '_>  {
        self.end_arrow.as_ref().map(|(arrow, transform)|
            arrow.path.iter().transformed(transform)
        )
    }
}