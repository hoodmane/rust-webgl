use lyon::geom::math::{Point, point, Vector, vector, Transform};
use std::cmp::Ordering;

use wasm_bindgen::prelude::*;

use crate::vector::JsPoint;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BufferDimensions {
	width : i32,
	height : i32,
	density : f64
}

impl BufferDimensions {
	pub fn new(width : i32, height : i32, density : f64) -> Self {
		Self { width, height, density }
	}
	
	pub fn width(&self) -> i32 {
		self.width
	}

	pub fn height(&self) -> i32 {
		self.height 
	}

	pub fn density(&self) -> f64 {
		self.density
	}

	pub fn pixel_width(&self) -> i32 {
        (self.width as f64 * self.density) as i32
    }

    pub fn pixel_height(&self) -> i32 {
        (self.height as f64 * self.density) as i32
    }
}


#[derive(Clone, Copy, Debug)]
pub struct CoordinateSystem {
	pub(crate) origin : Point,
    pub(crate) scale : Vector,
    pub(crate) glyph_scale : f32,

    pub(crate) left_margin : i32,
    pub(crate) right_margin : i32,
    pub(crate) bottom_margin : i32,
    pub(crate) top_margin : i32,
    // pixel coordinates to WebGl coordinates [-1, 1] x [-1, 1]
    pub(crate) transform : Transform,
    // dimensions of screen
    pub(crate) buffer_dimensions : BufferDimensions,


    natural_scale_ratio : f32, 
    max_scale : f32, 
    min_xy_boundary : Point,
    max_xy_boundary : Point,
}

impl CoordinateSystem {
    pub fn new() -> Self {
        CoordinateSystem {
            // user affine coordinate transformation
            origin : point(0.0, 0.0),
            scale : vector(100.0, 100.0),

            // pixel coordinates to WebGl coordinates [-1, 1] x [-1, 1]
            transform : Transform::identity(),
            
            left_margin : 10,
            right_margin : 50,
            bottom_margin : 10,
            top_margin : 10,
            buffer_dimensions : BufferDimensions::new(1, 1, 0.0),

            // bounds on user affine coordinate transform
            natural_scale_ratio : 1.0,
            max_scale : 1000.0,
            min_xy_boundary : point(f32::NEG_INFINITY, f32::NEG_INFINITY),
            max_xy_boundary : point(f32::INFINITY, f32::INFINITY),
            glyph_scale : 1.0,
        }
    }


    pub fn transform_point(&self, point : Point) -> Point {
        let Point {x, y, ..} = point;
        Point::new(self.transform_x(x), self.transform_y(y))
    }

    pub fn transform_x(&self, x : f32) -> f32 {
        self.origin.x + x * self.scale.x
    }

    pub fn transform_y(&self, y : f32) -> f32 {
        self.origin.y - y * self.scale.y
    }

    pub fn inverse_transform_point(&self, point : Point) -> Point {
        let Point {x, y, ..} = point;
        Point::new(
            self.inverse_transform_x(x),
            self.inverse_transform_y(y)
        )
    }

    pub fn inverse_transform_x(&self, x : f32) -> f32 {
        (x - self.origin.x)/self.scale.x
    }

    pub fn inverse_transform_y(&self, y : f32) -> f32 {
        -(y - self.origin.y) / self.scale.y
    }

    pub fn set_margins(&mut self, 
        left_margin : i32,
        right_margin : i32,
        bottom_margin : i32,
        top_margin : i32,
    ) {
        self.left_margin = left_margin;
        self.right_margin = right_margin;
        self.bottom_margin = bottom_margin;
        self.top_margin = top_margin;
    }

    pub fn reset_transform(&mut self){
        self.transform = Transform::scale(2.0/ (self.buffer_dimensions.width() as f32), -2.0/(self.buffer_dimensions.height() as f32))
            .then_translate(vector(-1.0, 1.0));
    }

    pub(crate) fn screen_x_range(&self) -> (f32, f32) {
        (self.left_margin as f32, (self.buffer_dimensions.width() - self.right_margin) as f32)
    }

    pub(crate) fn screen_y_range(&self) -> (f32, f32) {
        (self.top_margin as f32, (self.buffer_dimensions.height() - self.bottom_margin) as f32)
    }

    pub(crate) fn current_min_xy(&self) -> Point {
        let (screen_x_min, _) = self.screen_x_range();
        let (_, screen_y_max) = self.screen_y_range();
        point(self.inverse_transform_x(screen_x_min), self.inverse_transform_y(screen_y_max))
    }

    pub(crate) fn current_max_xy(&self) -> Point {
        let (_, screen_x_max) = self.screen_x_range();
        let (screen_y_min, _) = self.screen_y_range();
        point(self.inverse_transform_x(screen_x_max), self.inverse_transform_y(screen_y_min))
    }

    pub fn set_current_xrange(&mut self, xmin : f32, xmax : f32){
        let (screen_x_min, screen_x_max) = self.screen_x_range();
        self.scale.x = (screen_x_max - screen_x_min) / (xmax - xmin);
        self.origin.x = screen_x_min - xmin * self.scale.x;
        self.natural_scale_ratio = self.scale.y / self.scale.x;
    }

    pub fn set_current_yrange(&mut self, ymin : f32, ymax : f32){
        let (screen_y_min, screen_y_max) = self.screen_y_range();
        self.scale.y = (screen_y_max - screen_y_min) / (ymax - ymin);
        self.origin.y = screen_y_min + ymax * self.scale.y;
        self.natural_scale_ratio = self.scale.y / self.scale.x;
    }

    pub fn set_max_xrange(&mut self, xmin : f32, xmax : f32){
        self.min_xy_boundary.x = xmin;
        self.max_xy_boundary.x = xmax;
    }

    pub fn set_max_yrange(&mut self, ymin : f32, ymax : f32){
        self.min_xy_boundary.y = ymin;
        self.max_xy_boundary.y = ymax;
    }



    pub fn translate(&mut self, delta : JsPoint) {
        let delta : Vector = delta.into();
        self.origin += delta;
        self.enforce_translation_bounds();
    }

    pub fn scale_around(&mut self, scale : f32, center : JsPoint) -> Result<(), JsValue> {
        // ensure maximum scale
        let mut scale = f32::min(scale, self.max_scale / f32::max(self.scale.x, self.scale.y));
        // Now if we scale in we have to ensure that we restore the natural aspect ratio before scaling both directions.
        if scale > 1.0 {
            let scale_ratio = self.scale.y / self.scale.x;
            match scale_ratio.partial_cmp(&self.natural_scale_ratio) {
                None => { return Err("NaN occurred somehow?".into()); },
                Some(Ordering::Equal) => {},
                Some(Ordering::Less) => { // stretched in the y direction
                    // How much would we have to scale by to correct the stretch?
                    let correction_ratio = self.natural_scale_ratio/scale_ratio;
                    let yscale = scale.min(correction_ratio);
                    self.scale_around_y_raw(yscale, center);
                    scale /= yscale;
                },
                Some(Ordering::Greater) => { // stretched in the x direction
                    let correction_ratio = scale_ratio/self.natural_scale_ratio;
                    let xscale = scale.min(correction_ratio);
                    self.scale_around_x_raw(xscale, center);
                    scale /= xscale;
                },
            }
        }
        self.scale_around_raw(scale, center);
        self.enforce_scale_out_bounds();
        Ok(())
    }


    fn scale_around_raw(&mut self, scale : f32, center : JsPoint){
        let center : Point = center.into();
        self.origin += (center - self.origin) * (1.0 - scale);
        self.scale *= scale;
    }

    fn scale_around_x_raw(&mut self, scale : f32, center : JsPoint){
        let y = self.origin.y;
        let yscale = self.scale.y;
        self.scale_around_raw(scale, center);
        self.origin.y = y;
        self.scale.y = yscale;
    }

    fn scale_around_y_raw(&mut self, scale : f32, center : JsPoint){
        let x = self.origin.x;
        let xscale = self.scale.x;
        self.scale_around_raw(scale, center);
        self.origin.x = x;
        self.scale.x = xscale;
    }


    // Ensure that we don't scroll off sides of region
    fn enforce_translation_bounds(&mut self){
        let cur_min = self.current_min_xy();
        let cur_max = self.current_max_xy();
        let bound_min = self.min_xy_boundary;
        let bound_max = self.max_xy_boundary;
        let max_correction = Vector::min(bound_max - cur_max, vector(0.0, 0.0));
        let min_correction = Vector::max(bound_min - cur_min, vector(0.0, 0.0));
        let mut correction = max_correction + min_correction;
        correction.x *= self.scale.x;
        correction.y *= -self.scale.y;
        self.origin -= correction;
    }

    fn enforce_scale_out_bounds(&mut self){
        // Fix scale before doing translation bounds to prevent thrashing / weird behavior when range is too big.
        let cur_xy_range = self.current_max_xy() - self.current_min_xy();
        let max_xy_range = self.max_xy_boundary - self.min_xy_boundary;
        if cur_xy_range.x > max_xy_range.x {
            self.set_current_xrange(self.min_xy_boundary.x, self.max_xy_boundary.x);
        }
        if cur_xy_range.y > max_xy_range.y {
            self.set_current_yrange(self.min_xy_boundary.y, self.max_xy_boundary.y);
        }
        self.enforce_translation_bounds();
    }

}

