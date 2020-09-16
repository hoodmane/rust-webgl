use std::convert::Into;

use lyon::geom::math::{Point, point, vector, Angle,  Vector};
use lyon::geom::{Arc, CubicBezierSegment, QuadraticBezierSegment, LineSegment};
use lyon::path::PathEvent;

use crate::flattened_curve::{find_linear_inital_segment, find_quadratic_initial_segment, find_cubic_initial_segment, find_arc_initial_segment, ArcQuadraticBezierIterator};



enum PathSegment {
    Linear(LineSegment<f32>),
    Quadratic(QuadraticBezierSegment<f32>),
    Cubic(CubicBezierSegment<f32>),
    Arc(Arc<f32>),
    NoOp
}

impl PathSegment {
    pub fn line(from : Point, to : Point) -> Self {
        Self::Linear(LineSegment { from, to })
    }

    pub fn quadratic(from : Point, ctrl : Point, to : Point) -> Self{
        Self::Quadratic(QuadraticBezierSegment {
            from, ctrl, to
        })
    }

    pub fn cubic(from : Point, ctrl1 : Point, ctrl2 : Point, to : Point) -> Self {
        Self::Cubic(CubicBezierSegment {
            from, ctrl1, ctrl2, to
        })
    }

    
    pub fn arc( center: Point, radii : Vector, start_angle: Angle, sweep_angle: Angle, x_rotation: Angle) -> Self {
        Self::Arc(Arc {
            center,
            radii,
            start_angle,
            sweep_angle,
            x_rotation
        })
    }

    fn from(&self) -> Point {
        match self {
            Self::Arc(s) => s.from(),
            Self::Cubic(s) => s.from(),
            Self::Quadratic(s) => s.from(),
            Self::Linear(s) => s.from(),
            Self::NoOp => panic!()
        }
    }

    fn to(&self) -> Point {
        match self {
            Self::Arc(s) => s.to(),
            Self::Cubic(s) => s.to(),
            Self::Quadratic(s) => s.to(),
            Self::Linear(s) => s.to(),
            Self::NoOp => panic!()
        }
    }
    
    fn flip(&self) -> Self {
        match self {
            Self::Arc(s) => Self::Arc(s.flip()),
            Self::Cubic(s) => Self::Cubic(s.flip()),
            Self::Quadratic(s) => Self::Quadratic(s.flip()),
            Self::Linear(s) => Self::Linear(s.flip()),
            Self::NoOp => panic!()
        }
    }

    fn after_split(&self, t : f32) -> Self {
        match self {
            Self::Arc(s) => Self::Arc(s.after_split(t)),
            Self::Cubic(s) => Self::Cubic(s.after_split(t)),
            Self::Quadratic(s) => Self::Quadratic(s.after_split(t)),
            Self::Linear(s) => Self::Linear(s.after_split(t)),
            Self::NoOp => panic!()
        }
    }

    fn find_initial_segment(&self, tolerance : f32, len : f32) -> Result<f32, f32> {
        match self {
            Self::Arc(s) => find_arc_initial_segment(s, tolerance, len),
            Self::Cubic(s) => find_cubic_initial_segment(s, tolerance, len),
            Self::Quadratic(s) => find_quadratic_initial_segment(s, tolerance, len),
            Self::Linear(s) => find_linear_inital_segment(s, tolerance, len),
            Self::NoOp => Err(len),
        }
    }

    fn shorten(&self, tolerance : f32, shorten : f32) -> Result<PathSegment, f32> {
        Ok(self.after_split(self.find_initial_segment(tolerance, shorten)?))
    }

    fn iter(&self) -> PathSegmentEventIterator {
        match self {
            Self::Arc(s) => PathSegmentEventIterator::arc(s),
            Self::Cubic(s) => PathSegmentEventIterator::cubic(s),
            Self::Quadratic(s) => PathSegmentEventIterator::quadratic(s),
            Self::Linear(s) => PathSegmentEventIterator::linear(s),
            Self::NoOp => PathSegmentEventIterator::empty()
        }
    }
    
}

pub enum PathSegmentEventIterator {
    Arc(std::iter::Map<ArcQuadraticBezierIterator<f32>, fn(QuadraticBezierSegment<f32>) -> PathEvent>),
    Once(std::iter::Once<PathEvent>),
    Empty(std::iter::Empty<PathEvent>)
}



impl PathSegmentEventIterator {
    fn once(event : PathEvent) -> Self {
        Self::Once(std::iter::once(event))
    }

    fn empty() -> Self {
        Self::Empty(std::iter::empty())
    }

    fn arc(arc : &Arc<f32>) -> Self {
        fn to_events(curve : QuadraticBezierSegment<f32>) -> PathEvent {
            PathEvent::Quadratic {
                from : curve.from,
                ctrl : curve.ctrl,
                to : curve.to
            }
        }
        Self::Arc(ArcQuadraticBezierIterator::new(arc).map(to_events as fn(c : QuadraticBezierSegment<f32>) -> PathEvent))
    }

    fn linear(curve : &LineSegment<f32>) -> Self {
        Self::once(PathEvent::Line  {
            from : curve.from,
            to : curve.to
        })
    }

    fn quadratic(curve : &QuadraticBezierSegment<f32>) -> Self {
        Self::once(PathEvent::Quadratic {
            from : curve.from,
            ctrl : curve.ctrl,
            to : curve.to
        })
    }

    fn cubic(curve : &CubicBezierSegment<f32>) -> Self {
        Self::once(PathEvent::Cubic {
            from : curve.from,
            ctrl1 : curve.ctrl1,
            ctrl2 : curve.ctrl2,
            to : curve.to
        })
    }
}

impl Iterator for PathSegmentEventIterator {

    type Item = PathEvent;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Arc(it) => it.next(),
            Self::Once(it) => it.next(),
            Self::Empty(it) => it.next()
        }
    }
}


pub struct Path {
    start : Point,
    path : Vec<PathSegment>
}

impl Path {
    pub fn new<T : Into<Point>>(start : T) -> Self {
        Self {
            start : start.into(), 
            path : Vec::new()
        }
    }

    pub fn previous_point(&self, command_index : usize) -> Point {
        match command_index {
            0 => self.start,
            _ => self.path[command_index - 1].to()
        }
    }

    
    pub fn last_point(&self) -> Point {
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
        for segment in self.path.iter_mut().rev() {
            let flipped_segment = segment.flip();
            match flipped_segment.shorten(tolerance, shorten) {
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

    pub fn event_iterator(&self ) -> impl Iterator<Item = PathEvent> + '_ {
        fn path_segment_iter(seg : &PathSegment) -> PathSegmentEventIterator {
            seg.iter()
        }
        std::iter::once(PathEvent::Begin { at : self.start}).chain(
            self.path.iter().flat_map(path_segment_iter as fn(seg : &PathSegment) -> PathSegmentEventIterator)
        ).chain(
            std::iter::once(PathEvent::End { first : self.start, last : self.last_point(), close : false })
        )
    }
}