#![allow(dead_code)]

use crate::vector::Vec2;


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
	x : f32,
	y : f32,
	width : f32,
	height : f32
}

impl Rect {
    pub fn new(x : f32, y : f32, width : f32, height : f32) -> Self {
        Self {
            x, y, width, height
        }
    }

	pub fn left(&self) -> f32 {
		self.x
	}

	pub fn top(&self) -> f32 {
		self.y
	}

	pub fn right(&self) -> f32 {
		self.x + self.width
	}

	pub fn bottom(&self) -> f32 {
		self.y + self.height
	}

	pub fn is_empty(&self) -> bool {
		self.width == 0.0 || self.height == 0.0
	}
}


pub struct RectBuilder {
	min_x : f32,
	min_y : f32,
	max_x : f32,
	max_y : f32
}

impl RectBuilder {
	pub fn new() -> Self {
		RectBuilder {
            min_x : f32::INFINITY,
            min_y : f32::INFINITY,
            max_x : f32::NEG_INFINITY,
            max_y : f32::NEG_INFINITY
        }
	}

	pub fn reset(&mut self) {
		self.min_x = f32::INFINITY;
        self.min_y = f32::INFINITY;
		self.max_x = f32::NEG_INFINITY;
		self.max_y = f32::NEG_INFINITY;        
	}

	pub fn build(&self) -> Rect {
		Rect::new(self.min_x, self.min_y, self.max_x - self.min_x, self.max_y - self.min_y)
	}

	pub fn include(&mut self,  p : Vec2) {
        let Vec2 {x, y} = p;
		self.min_x = self.min_x.min(x);
		self.min_y = self.min_y.min(y);
		self.max_x = self.max_x.max(x);
		self.max_y = self.max_y.max(y);
	}
}

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