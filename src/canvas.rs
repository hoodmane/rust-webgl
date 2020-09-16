use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};
use std::f32::consts::PI;


use crate::log;
use crate::shader::{LineShader, StencilShader, GlyphShader, HorizontalAlignment, VerticalAlignment, DefaultShader, DefaultShaderIndexed};

use crate::font::{GlyphCompiler, Glyph, Font};

use crate::webgl_wrapper::{WebGlWrapper, Buffer};
use crate::matrix::Transform;
use crate::vector::{Vec2, Vec4};


use crate::arrow::normal_arrow;

use crate::rect::{Rect, BufferDimensions};

use crate::poly_line::{LineStyle, LineJoinStyle, LineCapStyle, Path};
use crate::arrow::Arrow;
use crate::convex_hull;

static BLACK : Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
static GRID_LIGHT_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 30.0 / 255.0);
static GRID_DARK_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 90.0 / 255.0);
static CONVEX_HULL_ANGLE_RESOLUTION : usize = 180;
static CONVEX_HULL_GLYPH_SCALE : f32 = 100.0;

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
    glyph_hull_buffer : Buffer,
    default_shader : DefaultShader,
    default_shader_indexed : DefaultShaderIndexed,
    buffer_dimensions : BufferDimensions,
    left_margin : i32,
    right_margin : i32,
    bottom_margin : i32,
    top_margin : i32,
    transform : Transform
}

#[wasm_bindgen]
pub struct JsBuffer {
    data : Vec<Vec2>
}
impl JsBuffer {
    fn new(data : Vec<Vec2>) -> Self {
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
        let default_shader_indexed = DefaultShaderIndexed::new(webgl.clone())?;
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let buffer_dimensions = BufferDimensions::new(1, 1, 0.0);

        let glyph_hull_buffer = webgl.new_buffer();

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
            glyph_hull_buffer,
            default_shader,
            default_shader_indexed,
            buffer_dimensions,
            left_margin : 10,
            right_margin : 50,
            bottom_margin : 10,
            top_margin : 10,
        };
        result.resize(result.webgl.dimensions()?)?;
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
            (self.buffer_dimensions.width() - self.left_margin - self.right_margin) as f32,
            (self.buffer_dimensions.height() - self.top_margin - self.bottom_margin) as f32
        )
    }

    fn enable_clip(&self){
        self.webgl.enable(WebGl2RenderingContext::STENCIL_TEST);
    }

    fn disable_clip(&self){
        self.webgl.disable(WebGl2RenderingContext::STENCIL_TEST);
    }

    fn reset_transform(&mut self){
        let mut transform = Transform::new();
        transform.translate(-1.0, 1.0);
        transform.scale(2.0/ (self.buffer_dimensions.width() as f32), -2.0/(self.buffer_dimensions.height() as f32));
        self.transform = transform;
    }

    fn resize(&mut self, new_dimensions : BufferDimensions) -> Result<(), JsValue> {
        if new_dimensions == self.buffer_dimensions {
            return Ok(());
        }
        self.buffer_dimensions = new_dimensions;
        let canvas = self.webgl.canvas()?;
        canvas.style().set_property("width", &format!("{}px", self.buffer_dimensions.width()))?;
        canvas.style().set_property("height", &format!("{}px", self.buffer_dimensions.height()))?;
        canvas.set_width(self.buffer_dimensions.pixel_width() as u32);
        canvas.set_height(self.buffer_dimensions.pixel_height() as u32);
        self.reset_transform();
        
        self.glyph_shader.resize_buffer(self.buffer_dimensions)?;
        self.webgl.viewport(self.buffer_dimensions);
        self.stencil_shader.set_stencil_rect(self.transform, self.chart_region())?;
        Ok(())
    }

    fn screen_x_range(&self) -> (f32, f32) {
        (self.left_margin as f32, (self.buffer_dimensions.width() - self.right_margin) as f32)
    }

    fn screen_y_range(&self) -> (f32, f32) {
        (self.top_margin as f32, (self.buffer_dimensions.height() - self.bottom_margin) as f32)
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
        self.resize(self.webgl.dimensions()?)?;
        self.webgl.clear_color(1.0, 1.0, 1.0, 1.0);
        self.webgl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        
        self.webgl.copy_blend_mode();
        self.webgl.render_to_canvas(self.buffer_dimensions);
        Ok(())
    }

    pub fn draw_box(&mut self, x : f32, y : f32, width : f32, height : f32) -> Result<(), JsValue> {
        let mut a = GlyphCompiler::new();
        a.move_to(Vec2::new(0.0, 0.0));
        a.line_to(Vec2::new(width, 0.0));
        a.line_to(Vec2::new(width, height));
        a.line_to(Vec2::new(0.0, height));
        a.move_to(Vec2::new(0.0, 0.0));       
        a.close();
        let glyph = a.end();
        self.glyph_shader.draw(&glyph, self.transform, Vec2::new(x, y), 1.0, HorizontalAlignment::Center, VerticalAlignment::Center, Vec4::new(0.0, 0.0, 0.0, 0.0))?;
        Ok(())
    }

    pub fn draw_letter(&mut self, 
        font : &Font, codepoint : u16,  
        pos : Vec2, scale : f32,
        horizontal_alignment : HorizontalAlignment, vertical_alignment : VerticalAlignment,
        color : Vec4
    ) -> Result<(), JsValue> {
        let glyph = font.glyph(codepoint)?.path();
        self.glyph_shader.draw(glyph, self.transform, pos, scale, horizontal_alignment, vertical_alignment, color)?;
        Ok(())
    }

    pub fn draw_js_buffer(&mut self, buffer : &JsBuffer, pos : Vec2, draw_triangles : bool ) -> Result<(), JsValue> {
        let mut transform = self.transform;
        transform.translate(self.transform_x(pos.x), self.transform_y(pos.y));
        log!("buffer.data : {:?}", buffer.data);
        self.default_shader.draw(transform, &buffer.data, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        Ok(())
    }

    pub fn draw_js_buffer_points(&mut self, buffer : &JsBuffer, pos : Vec2) -> Result<(), JsValue> {
        let mut transform = self.transform;
        transform.translate(self.transform_x(pos.x), self.transform_y(pos.y));
        log!("buffer.data : {:?}", buffer.data);
        self.default_shader.draw(transform, &buffer.data, 
            WebGl2RenderingContext::POINTS)?;
        Ok(())
    }

    pub fn draw_js_buffer_fan(&mut self, buffer : &JsBuffer, pos : Vec2) -> Result<(), JsValue> {
        let mut transform = self.transform;
        transform.translate(self.transform_x(pos.x), self.transform_y(pos.y));
        log!("buffer.data : {:?}", buffer.data);
        self.default_shader.draw(transform, &buffer.data, 
            WebGl2RenderingContext::TRIANGLE_FAN)?;
        Ok(())
    }

    fn glyph_convex_hull<'a>(&mut self, glyph : &'a Glyph) -> Result<&'a Vec<Vec2>, JsValue>{
        glyph.convex_hull.get_or_try_init(||{
            let scale = CONVEX_HULL_GLYPH_SCALE;
            let glyph_path = glyph.path();
    
            self.glyph_hull_buffer.resize(self.buffer_dimensions)?;
            self.webgl.render_to(&mut self.glyph_hull_buffer)?;
    
            let (letter_raster, width, height) = self.glyph_shader.get_raster(glyph_path, self.transform, scale, &mut self.glyph_hull_buffer)?;
            let mut letter_hull = convex_hull::convex_hull(&letter_raster, width as usize, height as usize, Vec2::new((width/2) as f32, (height/2) as f32), 2, 0.1);
            for v in &mut letter_hull {
                v.y *= -1.0;
                *v /= self.buffer_dimensions.density() as f32;
            }
    
            let (letter_hull_raster, width, height) = self.default_shader.get_raster(self.transform, &letter_hull, WebGl2RenderingContext::TRIANGLE_FAN, &mut self.glyph_hull_buffer)?;
            let channel = 3; // alpha channel
            let mut outline = convex_hull::sample_raster_outline(&letter_hull_raster, width as usize, height as usize, channel, Vec2::new((width/2) as f32, (height/2) as f32), CONVEX_HULL_ANGLE_RESOLUTION);
            for v in &mut outline {
                v.y *= -1.0;
                *v /= self.buffer_dimensions.density() as f32;
                *v /= scale;
            }
            Ok(outline)
        })
    }

    pub fn js_find_glyph_boundary_point(&mut self, font : &Font, codepoint : u16, angle : f32) -> Result<Vec2, JsValue> {
        let glyph = font.glyph(codepoint)?;
        self.find_glyph_boundary_point(glyph, angle)
    }

    fn find_glyph_boundary_point(&mut self, glyph : &Glyph, angle : f32) -> Result<Vec2, JsValue> {
        let convex_hull = self.glyph_convex_hull(glyph)?;
        let angle = angle.rem_euclid(2.0 * PI);
        let index = ((CONVEX_HULL_ANGLE_RESOLUTION as f32) * (angle / (2.0 * PI))) as usize;
        Ok(convex_hull[index])
    }

    pub fn js_draw_line_to_glyph(&mut self, start : Vec2, end : Vec2, font : &Font, codepoint : u16, glyph_scale : f32) -> Result<JsBuffer, JsValue> {
        let glyph = font.glyph(codepoint)?;
        self.draw_line_to_glyph(start, end, glyph, glyph_scale)
    }

    fn draw_line_to_glyph(&mut self, start : Vec2, end : Vec2, glyph : &Glyph, glyph_scale : f32) -> Result<JsBuffer, JsValue> {
        let start = self.transform_point(start);
        let end = self.transform_point(end);
        let angle = (start - end).angle();
        let boundary_point = self.find_glyph_boundary_point(glyph, -angle)? * glyph_scale * 1.02;
        let mut poly_line = Path::new(start);
        poly_line.line_to(end + boundary_point);
        let mut triangles = Vec::new();
        poly_line.get_triangles(&mut triangles, LineStyle::new(LineJoinStyle::Miter, LineCapStyle::Butt, 5.0, 10.0, 0.5));
        self.default_shader.draw(self.transform, &triangles, WebGl2RenderingContext::TRIANGLE_STRIP)?;
        self.glyph_shader.draw(glyph.path(), self.transform, end, glyph_scale, HorizontalAlignment::Center, VerticalAlignment::Center, Vec4::new(0.2, 0.5, 0.8, 1.0))?;
        Ok(JsBuffer::new(triangles))
    }

    // pub fn draw_arrow(&mut self, start : Vec2, end : Vec2, line_width : f32) {
    //     let mut poly_line = PolyLine::new(start);
    //     poly_line.line_to(end);

    // }

    pub fn get_letter_convex_hull(&mut self, 
        font : &Font, codepoint : u16,
        scale : f32,
    ) -> Result<JsBuffer, JsValue> {

        let glyph = font.glyph(codepoint)?.path();
        self.glyph_hull_buffer.resize(self.buffer_dimensions)?;
        self.webgl.render_to(&mut self.glyph_hull_buffer)?;
        let (letter, width, height) = self.glyph_shader.get_raster(glyph, self.transform, scale, &mut self.glyph_hull_buffer)?;
        self.webgl.render_to_canvas(self.buffer_dimensions);
        let mut image = convex_hull::convex_hull(&letter, width as usize, height as usize, Vec2::new((width/2) as f32, (height/2) as f32), 2, 0.1);
        for v in &mut image {
            v.y *= -1.0;
            *v /= self.buffer_dimensions.density() as f32;
        }
        Ok(JsBuffer::new(image))
    }


    pub fn get_test_buffer(&self) -> JsBuffer {
        let mut test_buffer = Vec::new();
        test_buffer.push(Vec2::new(0.0, 0.0));
        test_buffer.push(Vec2::new(100.0, 100.0));
        test_buffer.push(Vec2::new(300.0, 50.0));
        JsBuffer::new(test_buffer)
    }

    pub fn end_frame(&self){

    }

    pub fn test_polyline1(&mut self, line_width : f32, draw_triangles : bool) -> Result<JsBuffer, JsValue> {
        let mut path1 = Path::new(Vec2::new(0.0, 0.0));
        path1.line_to(self.transform_point(Vec2::new(50.0, 60.0)));
        path1.line_to(self.transform_point(Vec2::new(100.0, 50.0)));

        log!("get triangles 1");
        let mut triangles1 = Vec::new();
        path1.get_triangles(&mut triangles1, LineStyle::new(LineJoinStyle::Round, LineCapStyle::Butt, line_width, 10.0, 0.5));
        log!("triangles1 : {:?}", triangles1);
        

        let mut transform = self.transform;
        transform.translate(self.transform_x(0.0), self.transform_y(0.0));
        self.default_shader.draw(transform, &triangles1,  if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;

        Ok(JsBuffer::new(triangles1))
    }


    pub fn test_polyline2(&mut self, line_width : f32, draw_triangles : bool) -> Result<JsBuffer, JsValue> {
        let mut path2 = Path::new(Vec2::new(0.0, 0.0));
        // path1.arc_to(self.transform_point(Vec2::new(1.0, 1.0)), 90.0);
        path2.cubic_curve_to(Vec2::new(100.0, 10.0), Vec2::new(50.0, 50.0), Vec2::new(50.0, 100.0));

        // log!("get triangles 2");
        let mut triangles2 = Vec::new();
        path2.get_triangles(&mut triangles2, LineStyle::new(LineJoinStyle::Round, LineCapStyle::Round, line_width, 10.0, 0.5));
        // log!("triangles2 : {:?}", triangles2);


        let mut transform = self.transform;
        transform.translate(self.transform_x(1.0), self.transform_y(0.0));
        self.default_shader.draw(transform, &triangles2,  if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;

        Ok(JsBuffer::new(triangles2))
    }

    pub fn test_polyline3(&mut self, line_width : f32, draw_triangles : bool) -> Result<JsBuffer, JsValue> {
        let mut path3 = Path::new(Vec2::new(0.0, 0.0));
        path3.line_to(Vec2::new(50.0, 50.0));
        path3.cubic_curve_to(Vec2::new(100.0, 50.0), Vec2::new(50.0, 50.0), Vec2::new(50.0, 100.0));

        // log!("get triangles 3");
        let mut triangles3 = Vec::new();
        path3.get_triangles(&mut triangles3, LineStyle::new(LineJoinStyle::Round, LineCapStyle::Round, line_width, 10.0, 0.5));
        // log!("triangles3 : {:?}", triangles3);

        let mut transform = self.transform;
        transform.translate(self.transform_x(2.0), self.transform_y(0.0));
        self.default_shader.draw(transform, &triangles3,  if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;

        Ok(JsBuffer::new(triangles3))
    }

    pub fn test_arc(&mut self, line_width : f32, degrees : f32, draw_triangles : bool) -> Result<JsBuffer, JsValue> {
        let mut path3 = Path::new(Vec2::new(0.0, 0.0));
        path3.arc_to(Vec2::new(50.0, 50.0), degrees * PI / 180.0);

        // log!("get triangles 3");
        let mut triangles3 = Vec::new();
        path3.get_triangles(&mut triangles3, LineStyle::new(LineJoinStyle::Round, LineCapStyle::Round, line_width, 10.0, 0.5));
        // log!("triangles3 : {:?}", triangles3);

        let mut transform = self.transform;
        transform.translate(self.transform_x(2.0), self.transform_y(0.0));
        self.default_shader.draw(transform, &triangles3,  if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;

        Ok(JsBuffer::new(triangles3))
    }


    pub fn draw_arrow(&mut self, line_width : f32, x_offset : f32, draw_triangles : bool) -> Result<(), JsValue> {
        let arrow = normal_arrow(line_width);
        let poly_line = &arrow.path;
        let xpos = -1.0;
        let mut triangles = Vec::new();
        poly_line.get_triangles(&mut triangles, LineStyle::new(LineJoinStyle::Miter, LineCapStyle::Butt, line_width, 10.0, 0.5));
        log!("triangles : {:?}", triangles);
        let mut transform = self.transform;
        transform.translate(self.transform_x(xpos) + x_offset, self.transform_y(2.0));
        self.default_shader.draw(transform, &triangles, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        
        let mut triangles2 = Vec::new();
        poly_line.get_triangles(&mut triangles2, LineStyle::new(LineJoinStyle::Bevel, LineCapStyle::Rect, line_width, 10.0, 0.5));
        let mut transform2 = self.transform;
        transform2.translate(self.transform_x(xpos) + x_offset, self.transform_y(-1.0));
        self.default_shader.draw(transform2, &triangles2, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;


        let mut triangles3 = Vec::new();
        poly_line.get_triangles(&mut triangles3, LineStyle::new(LineJoinStyle::Round, LineCapStyle::Round, line_width, 10.0, 0.5));
        let mut transform3 = self.transform;
        transform3.translate(self.transform_x(xpos) + x_offset, self.transform_y(-4.0));
        self.default_shader.draw(transform3, &triangles3, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
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

        
    pub fn test_lyon2(&mut self, draw_triangles : bool) -> Result<(), JsValue> {
        let mut path = crate::lyon_path::Path::new((0.0, 0.0));
        path.arc_to((100.0, 100.0), PI/180.0 * 15.0);
        path.line_to((200.0, 0.0));
        path.cubic_curve_to((250.0, 100.0), (550.0, 200.0), (300.0, 200.0));
        let buffers = crate::lyon_tesselate::tesselate_path(&path)?;
        self.default_shader_indexed.draw(self.transform, &buffers.vertices, &buffers.indices, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        Ok(())
    }
}