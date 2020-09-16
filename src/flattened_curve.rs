use std::iter::{Iterator, Flatten as FlattenIterator};

use euclid::default::Point2D as Point;
use euclid::default::Rotation2D as Rotation;
use euclid::default::Vector2D as Vector;
use num_traits::{Float, cast};

use lyon::math::{point};
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

#[inline]
pub fn vector<S>(x: S, y: S) -> Vector<S> {
    Vector::new(x, y)
}


fn num_quadratics<S: Scalar>(curve: &CubicBezierSegment<S>, tolerance: S) -> S {
    debug_assert!(tolerance > S::ZERO);

    let x = curve.from.x - S::THREE * curve.ctrl1.x + S::THREE * curve.ctrl2.x - curve.to.x;
    let y = curve.from.y - S::THREE * curve.ctrl1.y + S::THREE * curve.ctrl2.y - curve.to.y;

    let err = x * x + y * y;

    (err / (S::value(432.0) * tolerance * tolerance)).powf(S::ONE / S::SIX).ceil().max(S::ONE)
}


pub struct FlattenedCubicT<'a, S> {
    flattening_tolerance : S,
    num_quadratics : u32,
    step : S,
    t0 : S,
    curve : &'a CubicBezierSegment<S>,
}

impl<'a, S : Scalar> Iterator for FlattenedCubicT<'a, S> {
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

// pub enum FlattenedCurve<'a, S : Scalar> {
//     Quadratic(FlattenedQuadraticT<S>),
//     Cubic(FlattenIterator<FlattenedCubicT<'a, S>>),
// }

// impl<'a, S : Scalar> Iterator for FlattenedCurve<'a, S> {
//     type Item = S;
//     fn next(&mut self) -> Option<Self::Item> {
//         match self {
//             Self::Quadratic(it) => it.next(),
//             Self::Cubic(it) => it.next()
//         }
//     }
// }


pub fn flatten_cubic_with_t<'a, S : Scalar> (curve : &'a CubicBezierSegment<S>, tolerance : S) -> FlattenIterator<FlattenedCubicT<'a, S>> {
    debug_assert!(tolerance >= S::EPSILON);
    let quadratics_tolerance = tolerance * S::value(0.2);
    let flattening_tolerance = tolerance * S::value(0.8);

    let num_quadratics = num_quadratics(&curve, quadratics_tolerance);
    let step = S::ONE / num_quadratics;

    let t0 = S::ZERO;
    FlattenedCubicT {
        flattening_tolerance,
        num_quadratics : num_quadratics.to_u32().unwrap(),
        step,
        t0,
        curve
    }.flatten()

    // for _ in 0..num_quadratics.to_u32().unwrap() {
    //     let t1 = t0 + step;

    //     let quadratic = single_curve_approximation(&curve.split_range(t0..t1));
    //     quadratic.for_each_flattened_with_t(flattening_tolerance, &mut |point, t_sub| {
    //         let t = t0 + step * t_sub;
    //         callback(point, t);
    //     });

    //     t0 = t1;
    // }
}

pub fn flatten_quadratic_with_t<S : Scalar>(curve : &QuadraticBezierSegment<S>, tolerance : S) -> FlattenedQuadraticT<S> {
    curve.flattened_t(tolerance)
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

pub fn flatten_arc_with_t<S : Scalar>(arc : &Arc<S>, tolerance: S) -> FlattenedArcT<S> {
    let end = arc.to();
    let iter = arc.clone();
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

pub fn find_linear_inital_segment<S : Scalar>(curve : &LineSegment<S>, _tolerance : S, len : S) -> Result<S, S> {
    let line_length = curve.length();
    let remaining = len - line_length;
    if remaining < S::ZERO {
        Ok(len / line_length)
    } else {
        Err(remaining)
    }
}


pub fn find_quadratic_initial_segment<S : Scalar>(curve : &QuadraticBezierSegment<S>, tolerance : S, mut len : S) -> Result<S, S> {
    let mut from = curve.from;
    for t in flatten_quadratic_with_t(curve, tolerance){
        let to = curve.sample(t);
        len -= (to - from).length();
        if len < S::ZERO {
            return Ok(t);
        }
        from = to;
    }
    Err(len)
}


pub fn find_cubic_initial_segment<S : Scalar>(curve : &CubicBezierSegment<S>, tolerance : S, mut len : S) -> Result<S, S> {
    let mut from = curve.from;
    for t in flatten_cubic_with_t(curve, tolerance){
        let to = curve.sample(t);
        len -= (to - from).length();
        if len < S::ZERO {
            return Ok(t);
        }
        from = to;
    }
    Err(len)
}

pub fn find_arc_initial_segment<S : Scalar>(curve : &Arc<S>, tolerance : S, mut len : S) -> Result<S, S> {
    let mut from = curve.from();
    for t in flatten_arc_with_t(curve, tolerance){
        let to = curve.sample(t);
        len -= (to - from).length();
        if len < S::ZERO {
            return Ok(t);
        }
        from = to;
    }
    Err(len)
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
            arc : arc.clone(),
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