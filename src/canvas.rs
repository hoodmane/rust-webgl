use std::cmp::Ordering;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};
// use std::f32::consts::PI;


use crate::log;
use crate::error::convert_tessellation_error;
use crate::shader::{GlyphShader, EdgeShader, LineShader, DefaultShader, DefaultShaderIndexed};


use crate::webgl_wrapper::{WebGlWrapper};
use lyon::geom::math::{Point, point, Vector, vector, Angle, Transform};
use crate::vector::{JsPoint, Vec4};


use crate::arrow::normal_arrow;
use crate::edge::Edge;
use crate::rect::{Rect, BufferDimensions};


static BLACK : Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
static GRID_LIGHT_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 30.0 / 255.0);
static GRID_DARK_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 90.0 / 255.0);
static CONVEX_HULL_ANGLE_RESOLUTION : usize = 180;
static CONVEX_HULL_GLYPH_SCALE : f32 = 100.0;



use euclid::default::Rotation2D;
use lyon::path::builder::{PathBuilder, Build};
use lyon::tessellation::{
    VertexBuffers, geometry_builder, 
    StrokeTessellator, StrokeOptions,
    FillTessellator, FillOptions
};

#[wasm_bindgen]
pub struct Canvas {
    // user affine coordinate transformation
    origin : Point,
    scale : Vector,

    // bounds on user affine coordinate transform
    natural_scale_ratio : f32, 
    max_scale : f32, 
    min_xy_boundary : Point,
    max_xy_boundary : Point,

    left_margin : i32,
    right_margin : i32,
    bottom_margin : i32,
    top_margin : i32,
    // pixel coordinates to WebGl coordinates [-1, 1] x [-1, 1]
    transform : Transform,
    // dimensions of screen
    buffer_dimensions : BufferDimensions,

    // Webgl shaders
    webgl : WebGlWrapper,
    grid_shader : LineShader,
    axes_shader : LineShader,
    default_shader : DefaultShader,
    default_shader_indexed : DefaultShaderIndexed,
    glyph_shader : GlyphShader,
    edge_shader : EdgeShader,
}

#[wasm_bindgen]
pub struct JsBuffer {
    data : Vec<Point>
}
impl JsBuffer {
    fn new(data : Vec<Point>) -> Self {
        Self { data }
    }
}

#[wasm_bindgen]
impl Canvas {
    #[wasm_bindgen(constructor)]
    pub fn new(webgl_context : &WebGl2RenderingContext) -> Result<Canvas, JsValue> {
        let webgl = WebGlWrapper::new(webgl_context.clone());
        let grid_shader = LineShader::new(webgl.clone())?;
        let axes_shader = LineShader::new(webgl.clone())?;
        let default_shader = DefaultShader::new(webgl.clone())?;
        let default_shader_indexed = DefaultShaderIndexed::new(webgl.clone())?;
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let buffer_dimensions = BufferDimensions::new(1, 1, 0.0);
        let edge_shader = EdgeShader::new(webgl.clone())?;


        let mut result = Self {
            // user affine coordinate transformation
            origin : point(0.0, 0.0),
            scale : vector(100.0, 100.0),

            // bounds on user affine coordinate transform
            natural_scale_ratio : 1.0,
            max_scale : 1000.0,
            min_xy_boundary : point(f32::NEG_INFINITY, f32::NEG_INFINITY),
            max_xy_boundary : point(f32::INFINITY, f32::INFINITY),

            // pixel coordinates to WebGl coordinates [-1, 1] x [-1, 1]
            transform : Transform::identity(),
            buffer_dimensions,

            left_margin : 10,
            right_margin : 50,
            bottom_margin : 10,
            top_margin : 10,
            
            webgl,
            grid_shader,
            axes_shader,
            default_shader,
            default_shader_indexed,
            glyph_shader,
            edge_shader,
        };
        result.resize(result.webgl.dimensions()?)?;
        Ok(result)   
    }


    pub fn restore_context(&mut self, webgl_context : &WebGl2RenderingContext) -> Result<(), JsValue> {
        self.webgl = WebGlWrapper::new(webgl_context.clone());
        self.grid_shader = LineShader::new(self.webgl.clone())?;
        self.axes_shader = LineShader::new(self.webgl.clone())?;
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
        self.update_scissor();
        Ok(())
    }

    fn update_scissor(&self){
        let left = (self.left_margin as f64 * self.buffer_dimensions.density()) as i32;
        let bottom = (self.bottom_margin as f64 * self.buffer_dimensions.density()) as i32;
        let width = ((self.buffer_dimensions.width() - self.left_margin - self.right_margin) as f64  * self.buffer_dimensions.density()) as i32;
        let height = ((self.buffer_dimensions.height() - self.top_margin - self.bottom_margin) as f64  * self.buffer_dimensions.density()) as i32;
        self.webgl.scissor(left, bottom, width, height);
    }


    fn chart_region(&self) -> Rect {
        Rect::new(
            self.left_margin as f32, self.top_margin  as f32, 
            (self.buffer_dimensions.width() - self.left_margin - self.right_margin) as f32,
            (self.buffer_dimensions.height() - self.top_margin - self.bottom_margin) as f32
        )
    }

    fn enable_clip(&self){
        self.webgl.enable(WebGl2RenderingContext::SCISSOR_TEST);
    }

    fn disable_clip(&self){
        self.webgl.disable(WebGl2RenderingContext::SCISSOR_TEST);
    }

    fn reset_transform(&mut self){
        self.transform = Transform::scale(2.0/ (self.buffer_dimensions.width() as f32), -2.0/(self.buffer_dimensions.height() as f32))
            .then_translate(vector(-1.0, 1.0));
    }

    pub fn apply_transform(&self, p : JsPoint) -> JsPoint {
        self.transform.transform_point(p.into()).into()
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
        
        self.webgl.viewport(self.buffer_dimensions);
        self.update_scissor();
        Ok(())
    }

    fn screen_x_range(&self) -> (f32, f32) {
        (self.left_margin as f32, (self.buffer_dimensions.width() - self.right_margin) as f32)
    }

    fn screen_y_range(&self) -> (f32, f32) {
        (self.top_margin as f32, (self.buffer_dimensions.height() - self.bottom_margin) as f32)
    }

    fn current_min_xy(&self) -> Point {
        let (screen_x_min, _) = self.screen_x_range();
        let (_, screen_y_max) = self.screen_y_range();
        point(self.inverse_transform_x(screen_x_min), self.inverse_transform_y(screen_y_max))
    }

    fn current_max_xy(&self) -> Point {
        let (_, screen_x_max) = self.screen_x_range();
        let (screen_y_min, _) = self.screen_y_range();
        point(self.inverse_transform_x(screen_x_max), self.inverse_transform_y(screen_y_min))
    }

    pub fn set_current_xrange(&mut self, xmin : f32, xmax : f32){
        let (screen_x_min, screen_x_max) = self.screen_x_range();
        self.scale.x = (screen_x_max - screen_x_min) / (xmax - xmin);
        self.origin.x = screen_x_min - xmin * self.scale.x;
    }

    pub fn set_current_yrange(&mut self, ymin : f32, ymax : f32){
        let (screen_y_min, screen_y_max) = self.screen_y_range();
        self.scale.y = (screen_y_max - screen_y_min) / (ymax - ymin);
        self.origin.y = screen_y_min + ymax * self.scale.y;
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
                    scale = scale / yscale;
                },
                Some(Ordering::Greater) => { // stretched in the x direction
                    let correction_ratio = scale_ratio/self.natural_scale_ratio;
                    let xscale = scale.min(correction_ratio);
                    self.scale_around_x_raw(xscale, center);
                    scale = scale / xscale;
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

    pub fn transform_point(&self, point : JsPoint) -> JsPoint {
        let JsPoint {x, y} = point;
        Point::new(self.transform_x(x), self.transform_y(y)).into()
    }

    pub fn transform_x(&self, x : f32) -> f32 {
        self.origin.x + x * self.scale.x
    }

    pub fn transform_y(&self, y : f32) -> f32 {
        self.origin.y - y * self.scale.y
    }

    pub fn inverse_transform_point(&self, point : JsPoint) -> JsPoint {
        let JsPoint {x, y, ..} = point;
        Point::new(
            self.inverse_transform_x(x),
            self.inverse_transform_y(y)
        ).into()
    }

    pub fn inverse_transform_x(&self, x : f32) -> f32 {
        (x - self.origin.x)/self.scale.x
    }

    pub fn inverse_transform_y(&self, y : f32) -> f32 {
        -(y - self.origin.y) / self.scale.y
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

    pub fn test_edge_shader(&mut self, start : JsPoint, end : JsPoint, s1 : String, s2 : String, degrees : f32, scale : f32, thickness : f32) -> Result<(), JsValue> {
        use crate::glyph::{Glyph, GlyphInstance};
        
        let start : Point = start.into();
        let end : Point = end.into();
        let glyph1 = Glyph::from_stix(&s1);
        let glyph2 = Glyph::from_stix(&s2);
        let start_glyph = GlyphInstance::new(glyph1, start, scale,  Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));
        let end_glyph = GlyphInstance::new(glyph2, end, scale,  Vec4::new(0.0, 1.0, 0.0, 1.0), Vec4::new(0.0, 0.0, 1.0, 1.0));
        self.glyph_shader.clear_glyphs();
        self.glyph_shader.add_glyph(start_glyph.clone())?;
        self.glyph_shader.add_glyph(end_glyph.clone())?;

        self.glyph_shader.prepare()?;
        self.glyph_shader.draw(self.transform, self.origin, point(self.scale.x, -self.scale.y));

        let arrow = crate::arrow::test_arrow();
        self.edge_shader.clear();
        self.edge_shader.add_edge(start_glyph.clone(), end_glyph.clone(), Some(&arrow), Some(&arrow), Angle::degrees(degrees), thickness)?;
        self.edge_shader.prepare()?;
 
        // edge_shader.draw(self.transform, self.origin, point(self.scale.x, -self.scale.y));
        self.edge_shader.draw(self.transform, self.origin, point(self.scale.x, -self.scale.y));
        Ok(())
    }


    pub fn test_speed_setup(&mut self, s1 : String, s2 : String, xy_max : usize,  scale : f32, degrees : f32) -> Result<(), JsValue> {
        use crate::glyph::{Glyph, GlyphInstance};
        let glyph1 = Glyph::from_stix(&s1);
        let glyph2 = Glyph::from_stix(&s2);
        let mut glyph_instances = Vec::new();

        self.glyph_shader.clear_glyphs();
        self.edge_shader.clear();


        for x in 0..xy_max {
            for y in 0..xy_max {
                let s = if (x + y) % 2 == 1 { &glyph1 } else { &glyph2 };
                let r = x as f32 /  xy_max as f32;
                let b = y as f32 /  xy_max as f32;
                // let glyph_instance = GlyphInstance::new(s.clone(), point(x as f32, y as f32), scale, Vec4::new(r, 0.0, b, 1.0), Vec4::new(b, 0.0, r, 1.0));
                let glyph_instance = GlyphInstance::new(s.clone(), point(x as f32, y as f32), scale, Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::new(0.0, 0.0, 1.0, 1.0));
                self.glyph_shader.add_glyph(glyph_instance.clone())?;
                glyph_instances.push(glyph_instance);
            }
        }
        let x_max = xy_max;
        let y_max = xy_max;


        let arrow = crate::arrow::normal_arrow(2.0);
        let angle = Angle::degrees(degrees);

        for x in 1..x_max {
            for y in 0..y_max {
                let source = {
                    let y = 0;
                    glyph_instances[x * y_max + y].clone()
                };
                let target = {
                    let x = x - 1;
                    glyph_instances[x * y_max + y].clone()
                };
                self.edge_shader.add_edge(source, target, Some(&arrow), Some(&arrow), angle, 2.0)?;
            }
        }

        self.glyph_shader.prepare()?;
        self.edge_shader.prepare()?;
        self.test_speed();
        Ok(())
    }

    pub fn test_speed(&mut self) {
        self.glyph_shader.draw(self.transform, self.origin, point(self.scale.x, -self.scale.y));
        self.edge_shader.draw(self.transform, self.origin, point(self.scale.x, -self.scale.y));
    }


    // pub fn test_stix_math(&mut self, p : JsPoint, q : JsPoint, angle : f32, s : String) -> Result<(), JsValue> {
    //     use lyon::path::iterator::PathIterator;
    //     use crate::glyph::{Glyph, GlyphInstance};
    //     let glyph = Glyph::from_stix(&s);
    //     let start_instance = GlyphInstance::new(glyph.clone(), self.transform_point(p).into(), 30.0);
    //     let end_instance = GlyphInstance::new(glyph, self.transform_point(q).into(), 30.0);
    //     let edge = Edge::new(start_instance.clone(), end_instance.clone(), Angle::degrees(angle));


    //     let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    //     {
    //         let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
    //         // Create the tessellator.
    //         let mut stroke_tessellator = StrokeTessellator::new();
    //         let mut fill_tessellator = FillTessellator::new();
               
    //         edge.tessellate(&mut vertex_builder,
    //             &mut stroke_tessellator, &StrokeOptions::default(),
    //             &mut fill_tessellator,
    //         )?;
    //         start_instance.draw(&mut vertex_builder, &mut fill_tessellator)?;
    //         end_instance.draw(&mut vertex_builder, &mut fill_tessellator)?;
    //     }
    //     let transform = self.transform; //.pre_translate(self.transform_point((0.0, 0.0).into()).into());
    //     self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
    //     Ok(())
    // }

    pub fn get_test_buffer(&self) -> JsBuffer {
        let mut test_buffer = Vec::new();
        test_buffer.push(Point::new(0.0, 0.0));
        test_buffer.push(Point::new(100.0, 100.0));
        test_buffer.push(Point::new(300.0, 50.0));
        JsBuffer::new(test_buffer)
    }

    pub fn end_frame(&self){

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
			self.grid_shader.add_line(Point::new(tx, chart_region.top()), Point::new(tx, chart_region.bottom()), color, thickness)?;
		}

		// Horizontal grid lines
		for y in bottom .. top {
            let (color, thickness) = Self::gridline_color_and_thickness(y);
            let ty = self.transform_y((y as f32) * step);
			self.grid_shader.add_line(Point::new(chart_region.left(), ty), Point::new(chart_region.right(), ty), color, thickness)?;
        }

        // x axis
        self.axes_shader.add_line(
            Point::new(chart_region.left(), chart_region.bottom()), 
            Point::new(chart_region.right(), chart_region.bottom()), 
            BLACK, 0.5
        )?;

        // y axis
        self.axes_shader.add_line(
            Point::new(chart_region.left(), chart_region.top()), 
            Point::new(chart_region.left(), chart_region.bottom()), 
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

    // pub fn draw_triangle(&mut self, p1 : JsPoint, p2 : JsPoint, p3 : JsPoint) -> Result<(), JsValue> {
    //     let mut triangles : Vec<Point> = Vec::new();
    //     triangles.push(p1.into());
    //     triangles.push(p2.into());
    //     triangles.push(p3.into());
    //     self.default_shader.draw(Transform::identity(), triangles.as_slice(), WebGl2RenderingContext::TRIANGLES)?;
    //     Ok(())
    // }


}

