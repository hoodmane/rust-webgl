use std::iter::{Iterator, Flatten as FlattenIterator};

use euclid::default::Point2D as Point;
use euclid::default::Rotation2D as Rotation;
use euclid::default::Vector2D as Vector;
use num_traits::{Float, cast};

use lyon::math::{point, vector};
use lyon::geom::euclid::{Angle};
use lyon::geom::{
    Scalar,
    LineSegment,
    Line,
    QuadraticBezierSegment,
    CubicBezierSegment,
    Arc,
    cubic_to_quadratic::single_curve_approximation,
    quadratic_bezier::FlattenedT as FlattenedQuadraticT
};
use lyon::path::PathEvent;


pub enum PathSegment<S : Scalar> {
    Linear(LineSegment<S>),
    Quadratic(QuadraticBezierSegment<S>),
    Cubic(CubicBezierSegment<S>),
    Arc(Arc<S>),
    NoOp
}

impl<S : Scalar> PathSegment<S> {
    pub fn line(from : Point<S>, to : Point<S>) -> Self {
        Self::Linear(LineSegment { from, to })
    }

    pub fn quadratic(from : Point<S>, ctrl : Point<S>, to : Point<S>) -> Self{
        Self::Quadratic(QuadraticBezierSegment {
            from, ctrl, to
        })
    }

    pub fn cubic(from : Point<S>, ctrl1 : Point<S>, ctrl2 : Point<S>, to : Point<S>) -> Self {
        Self::Cubic(CubicBezierSegment {
            from, ctrl1, ctrl2, to
        })
    }

    
    pub fn arc( center: Point<S>, radii : Vector<S>, start_angle: Angle<S>, sweep_angle: Angle<S>, x_rotation: Angle<S>) -> Self {
        Self::Arc(Arc {
            center,
            radii,
            start_angle,
            sweep_angle,
            x_rotation
        })
    }

    pub fn from(&self) -> Point<S> {
        match self {
            Self::Arc(s) => s.from(),
            Self::Cubic(s) => s.from(),
            Self::Quadratic(s) => s.from(),
            Self::Linear(s) => s.from(),
            Self::NoOp => panic!()
        }
    }

    pub fn to(&self) -> Point<S> {
        match self {
            Self::Arc(s) => s.to(),
            Self::Cubic(s) => s.to(),
            Self::Quadratic(s) => s.to(),
            Self::Linear(s) => s.to(),
            Self::NoOp => panic!()
        }
    }
    
    pub fn flip(&self) -> Self {
        match self {
            Self::Arc(s) => Self::Arc(s.flip()),
            Self::Cubic(s) => Self::Cubic(s.flip()),
            Self::Quadratic(s) => Self::Quadratic(s.flip()),
            Self::Linear(s) => Self::Linear(s.flip()),
            Self::NoOp => panic!()
        }
    }

    pub fn after_split(&self, t : S) -> Self {
        match self {
            Self::Arc(s) => Self::Arc(s.after_split(t)),
            Self::Cubic(s) => Self::Cubic(s.after_split(t)),
            Self::Quadratic(s) => Self::Quadratic(s.after_split(t)),
            Self::Linear(s) => Self::Linear(s.after_split(t)),
            Self::NoOp => panic!()
        }
    }

    pub fn find_time_for_arclength(&self, tolerance : S, len : S) -> Result<S, S> {
        match self {
            Self::NoOp => Err(len),
            Self::Linear(s) => {
                let line_length = s.length();
                let remaining = len - line_length;
                if remaining < S::ZERO {
                    Ok(len / line_length)
                } else {
                    Err(remaining)
                }
            },
            _ => {
                let tolerance = tolerance/S::value(20.0);
                let mut from = self.from();
                let mut len = len;
                for t in self.flatten_with_t(tolerance){
                    let to = self.sample(t);
                    len -= (to - from).length();
                    if len < S::ZERO {
                        return Ok(t);
                    }
                    from = to;
                }
                Err(len)   
            }
        }
    }

    pub fn find_time_for_distance_from_start(&self, tolerance : S, len : S) -> Result<S, S> {
        match self {
            Self::NoOp | Self::Linear(_) => self.find_time_for_arclength(tolerance, len), 
            _ => {
                let tolerance = tolerance/S::value(20.0);
                let from = self.from();
                for t in self.flatten_with_t(tolerance){
                    if (self.sample(t) - from).length() > len {
                        return Ok(t);
                    }
                }
                Err(len)   
            }
        }
    }

    pub fn shorten(&self, tolerance : S, shorten : S) -> Result<PathSegment<S>, S> {
        Ok(self.after_split(self.find_time_for_arclength(tolerance, shorten)?))
    }

    pub fn flatten_with_t(&self, tolerance : S) -> FlattenedPathSegmentT<S> {
        match self {
            Self::Linear(_s) => unimplemented!(),
            Self::Quadratic(s) => FlattenedPathSegmentT::Quadratic(flatten_quadratic_with_t(s, tolerance)),
            Self::Cubic(s) => FlattenedPathSegmentT::Cubic(flatten_cubic_with_t(s, tolerance)),
            Self::Arc(s) => FlattenedPathSegmentT::Arc(flatten_arc_with_t(s, tolerance)),
            Self::NoOp => unimplemented!()
        }
    }

    pub fn sample(&self, t : S) -> Point<S> {
        match self {
            Self::Linear(s) => s.sample(t),
            Self::Quadratic(s) => s.sample(t),
            Self::Cubic(s) => s.sample(t),
            Self::Arc(s) => s.sample(t),
            Self::NoOp => panic!("Can't sample noop")
        }
    }
    
}

impl PathSegment<f32> {
    pub fn iter(&self) -> PathSegmentIterator {
        match self {
            Self::Arc(s) => PathSegmentIterator::arc(s),
            Self::Cubic(s) => PathSegmentIterator::cubic(s),
            Self::Quadratic(s) => PathSegmentIterator::quadratic(s),
            Self::Linear(s) => PathSegmentIterator::linear(s),
            Self::NoOp => PathSegmentIterator::empty()
        }
    }
}

pub enum FlattenedPathSegmentT<'a, S : Scalar> {
    // Linear(LineSegment<S>),
    Quadratic(FlattenedQuadraticT<S>),
    Cubic(FlattenedCubicT<'a, S>),
    Arc(FlattenedArcT<S>),
    // NoOp
}

impl<'a, S : Scalar> Iterator for FlattenedPathSegmentT<'a, S> {
    type Item = S;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FlattenedPathSegmentT::Quadratic(it) => it.next(),
            FlattenedPathSegmentT::Cubic(it) => it.next(),
            FlattenedPathSegmentT::Arc(it) => it.next(),
        }
    }
}


fn num_quadratics<S: Scalar>(curve: &CubicBezierSegment<S>, tolerance: S) -> S {
    debug_assert!(tolerance > S::ZERO);

    let x = curve.from.x - S::THREE * curve.ctrl1.x + S::THREE * curve.ctrl2.x - curve.to.x;
    let y = curve.from.y - S::THREE * curve.ctrl1.y + S::THREE * curve.ctrl2.y - curve.to.y;

    let err = x * x + y * y;

    (err / (S::value(432.0) * tolerance * tolerance)).powf(S::ONE / S::SIX).ceil().max(S::ONE)
}


fn flatten_quadratic_with_t<S : Scalar>(curve : &QuadraticBezierSegment<S>, tolerance : S) -> FlattenedQuadraticT<S> {
    curve.flattened_t(tolerance)
}


pub struct FlattenedCubicT<'a, S : Scalar> {
    inner : FlattenIterator<FlattenedCubicTInner<'a, S>>
}

impl<'a, S : Scalar> FlattenedCubicT<'a, S> {
    fn new(inner : FlattenedCubicTInner<'a, S>) -> Self {
        Self {
            inner : inner.flatten()
        }
    }
}

impl<'a, S : Scalar> Iterator for FlattenedCubicT<'a, S> {
    type Item = S;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}


struct FlattenedCubicTInner<'a, S> {
    flattening_tolerance : S,
    num_quadratics : u32,
    step : S,
    t0 : S,
    curve : &'a CubicBezierSegment<S>,
}

impl<'a, S : Scalar> Iterator for FlattenedCubicTInner<'a, S> {
    type Item = std::iter::Map<std::iter::Zip<FlattenedQuadraticT<S>, std::iter::Repeat<(S, S)>>, fn((S, (S, S))) -> S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.num_quadratics == 0 {
            return None;
        }
        self.num_quadratics -= 1;
        let t0 = self.t0;
        self.t0 += self.step;
        let t1 = self.t0;
        let quadratic = single_curve_approximation(&self.curve.split_range(t0..t1));
        fn f<S : Scalar>((t_sub, (t0, step)): (S, (S, S))) -> S {
            t0 + step * t_sub
        }
        // Some(quadratic.flattened_t(self.flattening_tolerance).map(|t_sub| t0 + step * t_sub))
        Some(quadratic.flattened_t(self.flattening_tolerance).zip(std::iter::repeat((t0, self.step))).map(f as fn(v: (S, (S, S))) -> S))
    }
}

fn flatten_cubic_with_t<'a, S : Scalar> (curve : &'a CubicBezierSegment<S>, tolerance : S) -> FlattenedCubicT<'a, S> {
    debug_assert!(tolerance >= S::EPSILON);
    let quadratics_tolerance = tolerance * S::value(0.2);
    let flattening_tolerance = tolerance * S::value(0.8);

    let num_quadratics = num_quadratics(&curve, quadratics_tolerance);
    let step = S::ONE / num_quadratics;

    let t0 = S::ZERO;
    FlattenedCubicT::new(FlattenedCubicTInner {
        flattening_tolerance,
        num_quadratics : num_quadratics.to_u32().unwrap(),
        step,
        t0,
        curve
    })
}


fn arc_flattening_step<S : Scalar>(arc : &Arc<S>, tolerance: S) -> S {
    // cos(theta) = (r - tolerance) / r
    // angle = 2 * theta
    // s = angle / sweep

    // Here we make the approximation that for small tolerance values we consider
    // the radius to be constant over each approximated segment.
    let r = (arc.from() - arc.center).length();
    let a = S::TWO * S::acos((r - tolerance) / r);
    let result = S::min(a / arc.sweep_angle.radians, S::ONE);

    if result < S::EPSILON {
        return S::ONE;
    }

    result
}

fn flatten_arc_with_t<S : Scalar>(arc : &Arc<S>, tolerance: S) -> FlattenedArcT<S> {
    let end = arc.to();
    let iter = *arc;
    let t0 = S::ZERO;
    FlattenedArcT {
        tolerance,
        end,
        iter,
        t0,
        finished : false
    }
}


pub struct FlattenedArcT<S : Scalar> {
    tolerance : S,
    end : Point<S>,
    iter : Arc<S>,
    t0 : S,
    finished : bool,
}

impl<S : Scalar> Iterator for FlattenedArcT<S> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let step = arc_flattening_step(&self.iter, self.tolerance);
        if step >= S::ONE {
            self.finished = true;
            return Some(S::ONE);
        }
        self.iter = self.iter.after_split(step);
        self.t0 += step * (S::ONE - self.t0);
        Some(self.t0)
    }
}

pub struct ArcQuadraticBezierIterator<S : Scalar> {
    arc : Arc<S>,
    sign : S,
    sweep_angle : S,
    n_steps : i32,
    step : Angle<S>,
    i : i32
}

impl<S : Scalar> ArcQuadraticBezierIterator<S> {
    pub fn new(arc: &Arc<S>) -> Self {
        let sign = arc.sweep_angle.get().signum();
        let sweep_angle = S::abs(arc.sweep_angle.get()).min(S::PI() * S::TWO);
    
        let n_steps = S::ceil(sweep_angle / S::FRAC_PI_4());
        let step = Angle::radians(sweep_angle / n_steps * sign);
        Self {
            arc : *arc,
            sign,
            sweep_angle,
            n_steps : cast::<S, i32>(n_steps).unwrap(),
            step,
            i : -1
        }
    }
}

fn sample_ellipse<S: Scalar>(radii: Vector<S>, x_rotation: Angle<S>, angle: Angle<S>) -> Point<S> {
    Rotation::new(x_rotation).transform_point(point(
        radii.x * Float::cos(angle.get()),
        radii.y * Float::sin(angle.get()),
    ))
}

#[inline]
fn tangent_at_angle<S : Scalar>(arc : &Arc<S>, angle: Angle<S>) -> Vector<S> {
    let a = angle.get();
    Rotation::new(arc.x_rotation).transform_vector(vector(
        -arc.radii.x * Float::sin(a),
        arc.radii.y * Float::cos(a),
    ))
}

impl<S : Scalar> Iterator for ArcQuadraticBezierIterator<S> {
    type Item = QuadraticBezierSegment<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;
        let i = self.i;
        if i == self.n_steps {
            return None;
        }
        let arc = &self.arc;
        let step = self.step;
        let a1 = arc.start_angle + step * cast(i).unwrap();
        let a2 = arc.start_angle + step * cast(i + 1).unwrap();

        let v1 = sample_ellipse(arc.radii, arc.x_rotation, a1).to_vector();
        let v2 = sample_ellipse(arc.radii, arc.x_rotation, a2).to_vector();
        let from = arc.center + v1;
        let to = arc.center + v2;
        let l1 = Line {
            point: from,
            vector: tangent_at_angle(&arc, a1),
        };
        let l2 = Line {
            point: to,
            vector: tangent_at_angle(&arc, a2),
        };
        let ctrl = l2.intersection(&l1).unwrap_or(from);
        Some(QuadraticBezierSegment { from, ctrl, to })
    }
}



pub enum PathSegmentIterator {
    Arc(std::iter::Map<ArcQuadraticBezierIterator<f32>, fn(QuadraticBezierSegment<f32>) -> PathEvent>),
    Once(std::iter::Once<PathEvent>),
    Empty(std::iter::Empty<PathEvent>)
}



impl PathSegmentIterator {
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

impl Iterator for PathSegmentIterator {

    type Item = PathEvent;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Arc(it) => it.next(),
            Self::Once(it) => it.next(),
            Self::Empty(it) => it.next()
        }
    }
}
