use crate::log::log_str;
use crate::shader::{LineShader, StencilShader, GlyphShader, HorizontalAlignment, VerticalAlignment, DefaultShader};

use crate::font::{GlyphCompiler, GlyphPath, Font};

use crate::webgl_wrapper::WebGlWrapper;
use crate::matrix::Transform;
use crate::vector::{Vec2, Vec4, Vec2Buffer};
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};

use crate::arrow::normal_arrow;

use crate::rect::Rect;

use crate::poly_line::{PolyLine, LineStyle, LineJoinStyle, LineCapStyle};


static BLACK : Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
static GRID_LIGHT_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 30.0 / 255.0);
static GRID_DARK_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 90.0 / 255.0);

#[wasm_bindgen]
pub struct Canvas {
    webgl : WebGlWrapper,
    origin : Vec2,
    xscale : f32,
    yscale : f32,
    stencil_shader : StencilShader,
    grid_shader : LineShader,
    axes_shader : LineShader,
    glyph_shader : GlyphShader,
    default_shader : DefaultShader,
    width : i32,
    height : i32,
    density : f64,
    left_margin : i32,
    right_margin : i32,
    bottom_margin : i32,
    top_margin : i32,
    transform : Transform
}

#[wasm_bindgen]
pub struct JsBuffer {
    data : Vec2Buffer
}
impl JsBuffer {
    fn new(data : Vec2Buffer) -> Self {
        Self { data }
    }
}

#[wasm_bindgen]
impl Canvas {
    #[wasm_bindgen(constructor)]
    pub fn new(webgl_context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
        let webgl = WebGlWrapper::new(webgl_context.clone());
        let stencil_shader = StencilShader::new(webgl.clone())?;
        let grid_shader = LineShader::new(webgl.clone())?;
        let axes_shader = LineShader::new(webgl.clone())?;
        let default_shader = DefaultShader::new(webgl.clone())?;
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let (width, height) = webgl.width_and_height()?;
        let density = WebGlWrapper::pixel_density();
        let mut result = Self {
            webgl,
            origin : Vec2::new(0.0, 0.0),
            xscale : 100.0,
            yscale : 100.0,
            transform : Transform::new(),
            stencil_shader,
            grid_shader,
            axes_shader,
            glyph_shader,
            default_shader,
            width,
            height,
            density,
            left_margin : 10,
            right_margin : 50,
            bottom_margin : 10,
            top_margin : 10,
        };
        result.resize(width, height, density)?;
        Ok(result)   
    }


    pub fn restore_context(&mut self, webgl_context : &WebGl2RenderingContext) -> Result<(), JsValue> {
        self.webgl = WebGlWrapper::new(webgl_context.clone());
        self.stencil_shader = StencilShader::new(self.webgl.clone())?;
        self.grid_shader = LineShader::new(self.webgl.clone())?;
        self.axes_shader = LineShader::new(self.webgl.clone())?;
        self.glyph_shader = GlyphShader::new(self.webgl.clone())?;
        Ok(())
    }


    pub fn set_margins(&mut self, 
        left_margin : i32,
        right_margin : i32,
        bottom_margin : i32,
        top_margin : i32,
    ) -> Result<(), JsValue> {
        self.left_margin = left_margin;
        self.right_margin = right_margin;
        self.bottom_margin = bottom_margin;
        self.top_margin = top_margin;
        self.stencil_shader.set_stencil_rect(self.transform, self.chart_region())?;
        Ok(())
    }

    fn chart_region(&self) -> Rect {
        Rect::new(
            self.left_margin as f32, self.top_margin  as f32, 
            (self.width - self.left_margin - self.right_margin) as f32,
            (self.height - self.top_margin - self.bottom_margin) as f32
        )
    }

    fn enable_clip(&self){
        self.webgl.enable(WebGl2RenderingContext::STENCIL_TEST);
    }

    fn disable_clip(&self){
        self.webgl.disable(WebGl2RenderingContext::STENCIL_TEST);
    }

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
        log_str(&format!("resize... old width : {}, new width : {}", self.width, width));
        log_str(&format!("resize... old height : {}, new height : {}", self.height, height));
        log_str(&format!("resize... old density : {}, new density : {}", self.density, density));
        self.width = width;
        self.height = height;
        self.density = density;
        let canvas = self.webgl.canvas()?;
        canvas.style().set_property("width", &format!("{}px", self.width))?;
        canvas.style().set_property("height", &format!("{}px", self.height))?;
        canvas.set_width(self.pixel_width() as u32);
        canvas.set_height(self.pixel_height() as u32);
        self.reset_transform();
        
        self.glyph_shader.resize_buffer(self.pixel_width(), self.pixel_height())?;
        self.webgl.viewport(0, 0, self.pixel_width(), self.pixel_height());
        self.stencil_shader.set_stencil_rect(self.transform, self.chart_region())?;
        Ok(())
    }

    fn screen_x_range(&self) -> (f32, f32) {
        (self.left_margin as f32, (self.width - self.right_margin) as f32)
    }

    fn screen_y_range(&self) -> (f32, f32) {
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
        if t % 10 == 0 {
            (GRID_DARK_COLOR, 0.5)
        } else {
            (GRID_LIGHT_COLOR, 0.5)   
        }
    }

    pub fn start_frame(&mut self) -> Result<(), JsValue> {
        let (new_width, new_height) = self.webgl.width_and_height()?;
        let new_density = WebGlWrapper::pixel_density();
        if new_width != self.width || new_height != self.height || new_density != self.density {
            self.resize(new_width, new_height, new_density)?;
        }
        self.webgl.clear_color(1.0, 1.0, 1.0, 1.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        
        // self.webgl.copy_blend_mode();
        // self.webgl.render_to_canvas();
        Ok(())
    }

    pub fn draw_box(&mut self, x : f32, y : f32, width : f32, height : f32) -> Result<(), JsValue> {
        let mut a = GlyphCompiler::new();
        a.move_to(Vec2::new(0.0, 0.0));
        a.line_to(Vec2::new(width, 0.0));
        a.line_to(Vec2::new(width, height));
        a.line_to(Vec2::new(0.0, height));
        a.move_to(Vec2::new(0.0, 0.0));
        // a.move_to(Vec2::new(x, y));
        // a.line_to(Vec2::new(x + width, y));
        // a.line_to(Vec2::new(x + width, y + height));
        // a.line_to(Vec2::new(x, y + height));
        // a.move_to(Vec2::new(x, y));        
        a.close();
        let glyph = a.end();
        self.glyph_shader.draw(&glyph, self.transform, Vec2::new(x, y), 1.0, HorizontalAlignment::Center, VerticalAlignment::Center)?;
        Ok(())
    }

    pub fn draw_letter(&mut self, 
        font : &Font, codepoint : u16,  
        pos : Vec2, scale : f32,
        horizontal_alignment : HorizontalAlignment, vertical_alignment : VerticalAlignment
    ) -> Result<(), JsValue> {
        let glyph = font.glyph(codepoint)?.path();
        self.glyph_shader.draw(glyph, self.transform, pos, scale, horizontal_alignment, vertical_alignment)?;
        Ok(())
    }

    pub fn draw_js_buffer(&mut self, buffer : &JsBuffer, pos : Vec2, draw_triangles : bool ) -> Result<(), JsValue> {
        let mut transform = self.transform;
        transform.translate(self.transform_x(pos.x), self.transform_y(pos.y));
        log_str(&format!("buffer.data : {:?}", buffer.data));
        self.default_shader.draw(transform, &buffer.data, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        Ok(())
    }

    pub fn draw_js_buffer_points(&mut self, buffer : &JsBuffer, pos : Vec2) -> Result<(), JsValue> {
        let mut transform = self.transform;
        transform.translate(self.transform_x(pos.x), self.transform_y(pos.y));
        log_str(&format!("buffer.data : {:?}", buffer.data));
        self.default_shader.draw(transform, &buffer.data, 
            WebGl2RenderingContext::POINTS)?;
        Ok(())
    }

    pub fn draw_js_buffer_fan(&mut self, buffer : &JsBuffer, pos : Vec2) -> Result<(), JsValue> {
        let mut transform = self.transform;
        transform.translate(self.transform_x(pos.x), self.transform_y(pos.y));
        log_str(&format!("buffer.data : {:?}", buffer.data));
        self.default_shader.draw(transform, &buffer.data, 
            WebGl2RenderingContext::TRIANGLE_FAN)?;
        Ok(())
    }

    pub fn get_test_buffer(&self) -> JsBuffer {
        let mut test_buffer = Vec2Buffer::new();
        test_buffer.push_vec(Vec2::new(0.0, 0.0));
        test_buffer.push_vec(Vec2::new(100.0, 100.0));
        test_buffer.push_vec(Vec2::new(300.0, 50.0));
        JsBuffer::new(test_buffer)
    }

    pub fn get_letter_convex_hull(&mut self, 
        font : &Font, codepoint : u16,  
         scale : f32, //draw_triangles : bool
    ) -> Result<JsBuffer, JsValue> {

        let glyph = font.glyph(codepoint)?.path();
        let (letter, width, height) = self.glyph_shader.draw_to_fit(glyph, self.transform, scale)?;
        let image = crate::convex_hull::convex_hull(&letter, width as usize, height as usize, Vec2::new((width/2) as f32, (height/2) as f32), 2, 0.1);
        let mut image_buffer = Vec2Buffer::new();
        for &v in &image {
            let mut v = v;
            v.y *= -1.0;
            image_buffer.push_vec(v);
        }
        // log_str(&format!("image_buffer : {:?}", image_buffer));
        Ok(JsBuffer::new(image_buffer))
        // // log_str(&format!("image : {:?}", image));
        // let mut transform = self.transform;
        // // transform.translate(self.transform_x(-3.0), self.transform_y(4.0) + 200.0);
        // // log_str(&format!("image : {:?}", image));
        // self.default_shader.draw(transform, &image_buffer, 
        //     if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;      
    }

    pub fn end_frame(&self){

    }

    // pub fn add_line(&mut self, p : Vec2, q : Vec2, color : Vec4, thickness : f32) -> Result<(), JsValue> {
    //     self.line_shader.add_line(p, q, color, thickness)?;
    //     Ok(())
    // }

    pub fn draw_arrow(&mut self, line_width : f32, draw_triangles : bool) -> Result<(), JsValue> {
        let poly_line = normal_arrow(line_width);
        let xpos = -1.0;
        let mut triangles = Vec2Buffer::new();
        poly_line.get_triangles(&mut triangles, LineStyle::new(LineJoinStyle::Miter, LineCapStyle::Butt, line_width, 10.0, 0.5));
        log_str(&format!("triangles : {:?}", triangles));
        let mut transform = self.transform;
        transform.translate(self.transform_x(xpos), self.transform_y(2.0));
        self.default_shader.draw(transform, &triangles, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        
        let mut triangles2 = Vec2Buffer::new();
        poly_line.get_triangles(&mut triangles2, LineStyle::new(LineJoinStyle::Bevel, LineCapStyle::Rect, line_width, 10.0, 0.5));
        let mut transform2 = self.transform;
        transform2.translate(self.transform_x(xpos), self.transform_y(-1.0));
        self.default_shader.draw(transform2, &triangles2, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;


        let mut triangles3 = Vec2Buffer::new();
        poly_line.get_triangles(&mut triangles3, LineStyle::new(LineJoinStyle::Round, LineCapStyle::Round, line_width, 10.0, 0.5));
        let mut transform3 = self.transform;
        transform3.translate(self.transform_x(xpos), self.transform_y(-4.0));
        self.default_shader.draw(transform3, &triangles3, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;




        // for &(xpos, x) in &[(-1.0, -100.0), (3.0, 100.0)] {
        //     let mut poly_line = PolyLine::new(Vec2::new(x, -y/2.0));
        //     poly_line.line_to(Vec2::new(0.0, 0.0));
        //     poly_line.line_to(Vec2::new(x, y/2.0));
        //     let mut triangles = Vec2Buffer::new();
        //     poly_line.get_triangles(&mut triangles, LineStyle::new(LineJoinStyle::Miter, line_width, 10.0, 0.5));
        //     log_str(&format!("triangles : {:?}", triangles));
        //     let mut transform = self.transform;
        //     transform.translate(self.transform_x(xpos), self.transform_y(2.0));
        //     self.default_shader.draw(transform, &triangles, 
        //         if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
            
        //     let mut triangles2 = Vec2Buffer::new();
        //     poly_line.get_triangles(&mut triangles2, LineStyle::new(LineJoinStyle::Bevel, line_width, 10.0, 0.5));
        //     let mut transform2 = self.transform;
        //     transform2.translate(self.transform_x(xpos), self.transform_y(-2.0));
        //     self.default_shader.draw(transform2, &triangles2, 
        //         if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;


        //     let mut triangles3 = Vec2Buffer::new();
        //     poly_line.get_triangles(&mut triangles3, LineStyle::new(LineJoinStyle::Round, line_width, 10.0, 0.5));
        //     let mut transform3 = self.transform;
        //     transform3.translate(self.transform_x(xpos), self.transform_y(-4.0));
        //     self.default_shader.draw(transform3, &triangles3, 
        //         if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        // }
        Ok(())
    }

    pub fn draw_grid(&mut self) -> Result<(), JsValue> {
        self.axes_shader.clear();
        self.grid_shader.clear();

        let chart_region = self.chart_region();
		// Grid lines
		let step = 1.0; // 1 / f32::powf(10.0, f32::round(f32::log10(scale / 64.0)));
        let left = f32::floor(self.inverse_transform_x(chart_region.left()) / step - 1.0) as i32;
        let right = f32::ceil(self.inverse_transform_x(chart_region.right()) / step + 1.0) as i32;
        let bottom =  f32::floor(self.inverse_transform_y(chart_region.bottom()) / step - 1.0) as i32;
        let top =  f32::ceil(self.inverse_transform_y(chart_region.top()) / step + 1.0) as i32;

		// Vertical grid lines
		for x in left .. right {
            let (color, thickness) = Self::gridline_color_and_thickness(x);
            let tx = self.transform_x((x as f32) * step);
			self.grid_shader.add_line(Vec2::new(tx, chart_region.top()), Vec2::new(tx, chart_region.bottom()), color, thickness)?;
		}

		// Horizontal grid lines
		for y in bottom .. top {
            let (color, thickness) = Self::gridline_color_and_thickness(y);
            let ty = self.transform_y((y as f32) * step);
			self.grid_shader.add_line(Vec2::new(chart_region.left(), ty), Vec2::new(chart_region.right(), ty), color, thickness)?;
        }

        // x axis
        self.axes_shader.add_line(
            Vec2::new(chart_region.left(), chart_region.bottom()), 
            Vec2::new(chart_region.right(), chart_region.bottom()), 
            BLACK, 0.5
        )?;

        // y axis
        self.axes_shader.add_line(
            Vec2::new(chart_region.left(), chart_region.top()), 
            Vec2::new(chart_region.left(), chart_region.bottom()), 
            BLACK, 0.5
        )?;
        Ok(())
    }
    
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.webgl.premultiplied_blend_mode();
        self.disable_clip();
        self.axes_shader.draw(self.transform)?;
        self.enable_clip();
        self.grid_shader.draw(self.transform)?;
        Ok(())
    }
}