use crate::log::log_str;
use crate::line_shader::LineShader;
use crate::webgl_wrapper::WebGlWrapper;
use crate::matrix::Transform;
use crate::vector::{Vec2, Vec4};
use wasm_bindgen::prelude::*;

static BLACK : Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
static GRID_LIGHT_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 31.0 / 255.0);
static GRID_DARK_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 127.0 / 255.0);

#[wasm_bindgen]
pub struct Canvas {
    webgl : WebGlWrapper,
    origin : Vec2,
    xscale : f32,
    yscale : f32,
    line_shader : LineShader,
    width : i32,
    height : i32,
    density : f64,
    left_margin : i32,
    right_margin : i32,
    bottom_margin : i32,
    top_margin : i32,
    transform : Transform
}

impl Canvas {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let line_shader = LineShader::new(webgl.clone())?;
        let width = webgl.width();
        let height = webgl.height();
        let density = WebGlWrapper::density();
        let mut result = Self {
            webgl,
            origin : Vec2::new(0.0, 0.0),
            xscale : 100.0,
            yscale : 100.0,
            transform : Transform::new(),
            line_shader,
            width,
            height,
            density,
            left_margin : 10,
            right_margin : 10,
            bottom_margin : 10,
            top_margin : 10,
        };
        result.resize(width, height, density)?;
        Ok(result)   
    }
}


#[wasm_bindgen]
impl Canvas {
    pub fn pixel_width(&self) -> i32 {
        (self.width as f64 * self.density) as i32
    }

    pub fn pixel_height(&self) -> i32 {
        (self.height as f64 * self.density) as i32
    }

    fn reset_transform(&mut self){
        let mut transform = Transform::new();
        transform.translate(-1.0, 1.0);
        transform.scale(2.0/ (self.width as f32), -2.0/(self.height as f32));
        self.transform = transform;
    }

    pub fn resize(&mut self, width : i32, height : i32, density : f64) -> Result<(), JsValue> {
        self.width = width;
        self.height = height;
        self.density = density;
        let canvas = self.webgl.canvas()?;
        canvas.style().set_property("width", &format!("{}px", self.width))?;
        canvas.style().set_property("height", &format!("{}px", self.height))?;
        canvas.set_width(self.pixel_width() as u32);
        canvas.set_height(self.pixel_height() as u32);
        self.reset_transform();
        Ok(())
    }

    fn screen_x_range() -> (f32, f32) {
        (self.left_margin as f32, (self.width - self.right_margin) as f32)
    }

    fn screen_y_range() -> (f32, f32) {
        (self.top_margin as f32, (self.height - self.bottom_margin) as f32)
    }


    pub fn set_xrange(&mut self, xmin : f32, xmax : f32){
        let (screen_x_min, screen_x_max) = self.screen_x_range();
        self.xscale = (screen_x_max - screen_x_min) / (xmax - xmin);
        self.origin.x = screen_x_min - xmin * self.xscale;
    }

    pub fn set_yrange(&mut self, ymin : f32, ymax : f32){
        let (screen_y_min, screen_y_max) = self.screen_y_range();
        self.yscale = (screen_y_max - screen_y_min) / (ymax - ymin);
        self.origin.y = screen_y_min + ymax * self.yscale;
    }

    pub fn translate(&mut self, delta : Vec2) {
        self.origin += delta;
    }

    pub fn scale_around(&mut self, scale : f32, center : Vec2){
        self.origin += (center - self.origin) * (1.0 - scale);
        self.xscale *= scale;
        self.yscale *= scale;
    }

    pub fn transform_point(&self, point : Vec2) -> Vec2 {
        let Vec2 {x, y} = point;
        Vec2 {
            x : self.transform_x(x),
            y : self.transform_y(y)
        }
    }

    pub fn transform_x(&self, x : f32) -> f32 {
        self.origin.x + x * self.xscale
    }

    pub fn transform_y(&self, y : f32) -> f32 {
        self.origin.y - y * self.yscale
    }

    pub fn inverse_transform_point(&self, point : Vec2) -> Vec2 {
        let Vec2 {x, y} = point;
        Vec2 {
            x : self.inverse_transform_x(x),
            y : self.inverse_transform_y(y)
        }
    }


    pub fn inverse_transform_x(&self, x : f32) -> f32 {
        (x - self.origin.x)/self.xscale
    }

    pub fn inverse_transform_y(&self, y : f32) -> f32 {
        -(y - self.origin.y) / self.yscale
    }

    fn gridline_color_and_thickness(t : i32) -> (Vec4, f32) {
        if t == 0 {
            (BLACK, 2.0)
        } else if t % 10 == 0 {
            (GRID_DARK_COLOR, 1.0)
        } else {
            (GRID_LIGHT_COLOR, 1.0)   
        }
    }

    pub fn start_frame(&mut self) {
        self.line_shader.clear();
        self.webgl.viewport(0, 0, self.pixel_width(), self.pixel_height());
    }

    pub fn end_frame(&self){

    }

    pub fn add_line(&mut self, p : Vec2, q : Vec2, color : Vec4, thickness : f32) -> Result<(), JsValue> {
        self.line_shader.add_line(p, q, color, thickness)?;
        Ok(())
    }

    pub fn draw_grid(&mut self) -> Result<(), JsValue> {
        let (screen_x_min, screen_x_max) = self.screen_x_range();
        let (screen_y_min, screen_y_max) = self.screen_y_range();        
		// Grid lines
		let step = 2.0; //f32::powf(10.0, f32::round(f32::log10(scale / 64.0)));
        let left = f32::floor(self.inverse_transform_x(screen_x_min) / step) as i32;
        let right = f32::ceil(self.inverse_transform_x(screen_x_max) / step) as i32;
        let bottom =  f32::ceil(self.inverse_transform_y(screen_y_min) / step) as i32;
        let top =  f32::floor(self.inverse_transform_y(screen_y_max) / step) as i32;

		// Vertical grid lines
		for x in left .. right {
            let (color, thickness) = Self::gridline_color_and_thickness(x);
            let tx = self.transform_x((x as f32) * step);
			self.line_shader.add_line(Vec2::new(tx, self.top_margin), Vec2::new(tx, height), color, thickness)?;
		}

		// Horizontal grid lines
		for y in top .. bottom {
            let (color, thickness) = Self::gridline_color_and_thickness(y);
            let ty = self.transform_y((y as f32) * step);
			self.line_shader.add_line(Vec2::new(self.left_margin, ty), Vec2::new(width - self.right_margin, ty), color, thickness)?;
        }
        Ok(())
    }
    
    pub fn render(&self) -> Result<(), JsValue> {
        self.line_shader.draw(self.transform)?;
        Ok(())
    }
}