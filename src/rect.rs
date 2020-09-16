use lyon::geom::math::Point;


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
	min : Point,
	max : Point
}

impl RectBuilder {
	pub fn new() -> Self {
		RectBuilder {
			min : Point::new(f32::INFINITY, f32::INFINITY),
			max : Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY)
        }
	}

	pub fn build(self) -> Rect {
		Rect::new(self.min.x, self.min.y, self.max.x - self.min.x, self.max.y - self.min.y)
	}

	pub fn include(&mut self,  p : Point) {
		self.min = self.min.min(p);
		self.max = self.max.max(p);
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