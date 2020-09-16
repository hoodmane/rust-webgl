// Stolen from: https://github.com/jobtalle/ConvexHull/tree/master/src/convexHull
use std::f32::consts::PI;
use std::cmp::Ordering;

use crate::vector::Vec2;


pub fn convex_hull(raster : &[u8], width : usize, height : usize, pivot : Vec2, spacing : usize, precision : f32) -> Vec<Vec2> {
	let point_count = ray_count(width, height, spacing);
	let channel = 2; // blue
	let mut convex_hull = sample_raster_outline(raster, width, height, channel, pivot, point_count);
	average_nearby_points(&mut convex_hull, precision);
	graham_scan(&mut convex_hull);
	// convex_hull.shrink_to_fit();
	convex_hull
}


fn ray_count(_width : usize, _height : usize, _spacing : usize) -> usize {
	180 // (width + height) / (spacing >> 1)
}


fn scan_ray_for_nontransparent_pixel(raster : &[u8], width : usize, channel : usize,  start_position : Vec2, direction : Vec2, radius : i32) -> Vec2 {
	// Scan for pixel with nonzero value on color channel "channel"
	for i in 0 .. radius {
		let current_position = start_position - direction * i as f32;
		let x = current_position.x as usize;
		let y = current_position.y as usize;
		// Check channel
		if raster[(x + y * width) * 4 + channel ] != 0 {
			return current_position;
		}
	}
	start_position - direction * radius as f32
}


pub fn sample_raster_outline(raster : &[u8], width : usize, height : usize, channel : usize, pivot : Vec2, point_count : usize) -> Vec<Vec2> {
	let angle_step = 2.0 * PI / (point_count as f32);
	let half_dim = Vec2::new((width/2) as f32, (height/2) as f32);

	let mut result = Vec::with_capacity(point_count);
	for i in 0 .. point_count {
		let angle = angle_step * (i as f32);
		let direction = Vec2::direction(angle);

		// Create edge points
		let abscos = direction.x.abs();
		let abssin = direction.y.abs();

		let radius = f32::min(half_dim.x / abscos, half_dim.y / abssin) - 1.0;
		let position = scan_ray_for_nontransparent_pixel(raster, width, channel, direction * radius + half_dim, direction, f32::ceil(radius) as i32);

		result.push(position - pivot);
	}
	result
}


// Average together collections of nearby points. In place.
fn average_nearby_points(points : &mut Vec<Vec2>, trim_distance : f32) {
	let mut input_idx = 0;
	let mut output_idx = 0;
	while input_idx < points.len() {
		// Average the current point with as many later points as are closer than trim_distance to it.
		let current = points[input_idx];
		let (total, num_points) = points[input_idx + 1 ..].iter().take_while(|&&p| {
			(p - current).magnitude() < trim_distance
		}).fold((current, 1), |(total, num_points), &point| (total + point, num_points + 1));
		let average = total / (num_points as f32);
		// Put new average into input list
		points[output_idx] = average;
		output_idx += 1;
		input_idx += num_points;
	}
	// Shrink list to new length (panic if somehow output_idx > points.len())
	points.resize_with(output_idx, || unreachable!());
}

fn orientation(p : Vec2, q : Vec2, r : Vec2) -> f32 {
	Vec2::cross(r - q, q - p)
}

fn compare_magnitudes(p : Vec2, q : Vec2) -> Ordering {
	p.magnitude_sq().partial_cmp(&q.magnitude_sq()).unwrap()
}

#[derive(PartialEq,PartialOrd)]
struct NonNan(f32);

impl NonNan {
    fn new(val: f32) -> Option<NonNan> {
        if val.is_nan() {
            None
        } else {
            Some(NonNan(val))
        }
    }
}

impl Eq for NonNan {}

impl Ord for NonNan {
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}


fn graham_scan(points : &mut Vec<Vec2>) {
	
	// Find minimum Y
	let (min_idx, _) = points.iter().enumerate().min_by_key(|(_idx, v)| NonNan::new(v.y).unwrap()).unwrap();

	// Put minimum at zero
	points.swap(0, min_idx);

	let compare_point = points[0];
	points.sort_by(	
		move |&p1, &p2| // sort first by the handedness of (compare_point, p1, p2) then by distance from compare_pt.
		orientation(compare_point, p1, p2).partial_cmp(&0.0).unwrap().then_with(|| compare_magnitudes(p1 - compare_point, p2 - compare_point))
	);

	// Create & initialize stack
	let mut stack_length : usize = 3;
	for i in 3 .. points.len() {
		// Seems like this could lead to an infinite loop here...
		// Luckily, Rust will panic if stack_index becomes less than 2
		while orientation(points[stack_length - 2], points[stack_length - 1], points[i]) >= 0.0 {
			stack_length -= 1;
		}
		points[stack_length] = points[i];
		stack_length += 1;
	}
	// Shrink list to new length (panic if somehow output_idx > points.len())
	points.resize_with(stack_length, || unreachable!());
}