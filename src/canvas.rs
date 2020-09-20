use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};
// use std::f32::consts::PI;


use crate::log;
use crate::shader::{StencilShader, LineShader, DefaultShader, DefaultShaderIndexed};


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
    webgl : WebGlWrapper,
    origin : Point,
    xscale : f32,
    yscale : f32,
    stencil_shader : StencilShader,
    grid_shader : LineShader,
    axes_shader : LineShader,
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
        // TODO: stencil_shader ==> scissor
        let stencil_shader = StencilShader::new(webgl.clone())?;
        let grid_shader = LineShader::new(webgl.clone())?;
        let axes_shader = LineShader::new(webgl.clone())?;
        let default_shader = DefaultShader::new(webgl.clone())?;
        let default_shader_indexed = DefaultShaderIndexed::new(webgl.clone())?;
        let buffer_dimensions = BufferDimensions::new(1, 1, 0.0);


        let mut result = Self {
            webgl,
            origin : Point::new(0.0, 0.0),
            xscale : 100.0,
            yscale : 100.0,
            transform : Transform::identity(),
            stencil_shader,
            grid_shader,
            axes_shader,
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
        self.transform = Transform::scale(2.0/ (self.buffer_dimensions.width() as f32), -2.0/(self.buffer_dimensions.height() as f32))
            .then_translate(vector(-1.0, 1.0));
        log!("buffer_dimensions : {:?}", self.buffer_dimensions);
        log!("transform : {:?}", self.transform);
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

    pub fn translate(&mut self, delta : JsPoint) {
        let delta : Vector = delta.into();
        self.origin += delta;
    }

    pub fn scale_around(&mut self, scale : f32, center : JsPoint){
        let center : Point = center.into();
        self.origin += (center - self.origin) * (1.0 - scale);
        self.xscale *= scale;
        self.yscale *= scale;
    }

    pub fn transform_point(&self, point : JsPoint) -> JsPoint {
        let JsPoint {x, y} = point;
        Point::new(self.transform_x(x), self.transform_y(y)).into()
    }

    pub fn transform_x(&self, x : f32) -> f32 {
        self.origin.x + x * self.xscale
    }

    pub fn transform_y(&self, y : f32) -> f32 {
        self.origin.y - y * self.yscale
    }

    pub fn inverse_transform_point(&self, point : JsPoint) -> JsPoint {
        let JsPoint {x, y, ..} = point;
        Point::new(
            self.inverse_transform_x(x),
            self.inverse_transform_y(y)
        ).into()
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

    pub fn draw_js_buffer(&mut self, buffer : &JsBuffer, pos : JsPoint, draw_triangles : bool ) -> Result<(), JsValue> {
        let transform = self.transform.pre_translate(vector(self.transform_x(pos.x), self.transform_y(pos.y)));
        log!("buffer.data : {:?}", buffer.data);
        self.default_shader.draw(transform, buffer.data.as_slice(), 
            if draw_triangles { WebGl2RenderingContext::TRIANGLE_STRIP } else { WebGl2RenderingContext::LINE_STRIP })?;
        Ok(())
    }

    pub fn draw_js_buffer_points(&mut self, buffer : &JsBuffer, pos : JsPoint) -> Result<(), JsValue> {
        let transform = self.transform.pre_translate(vector(self.transform_x(pos.x), self.transform_y(pos.y)));
        log!("buffer.data : {:?}", buffer.data);
        self.default_shader.draw(transform, buffer.data.as_slice(), 
            WebGl2RenderingContext::POINTS)?;
        Ok(())
    }

    pub fn draw_js_buffer_fan(&mut self, buffer : &JsBuffer, pos : JsPoint) -> Result<(), JsValue> {
        let transform = self.transform.pre_translate(vector(self.transform_x(pos.x), self.transform_y(pos.y)));
        log!("buffer.data : {:?}", buffer.data);
        self.default_shader.draw(transform, buffer.data.as_slice(), 
            WebGl2RenderingContext::TRIANGLE_FAN)?;
        Ok(())
    }


    pub fn test_speed(&mut self, s : String, angle : f32) -> Result<(), JsValue> {
        use lyon::path::iterator::PathIterator;
        use crate::glyph::{Glyph, GlyphInstance};
        let glyph = Glyph::from_stix(&s);
        let x_max = 20;
        let y_max = 20;
        let mut glyph_instances = Vec::with_capacity(x_max * y_max);
        for x in 0..x_max {
            for y in 0..y_max {
                glyph_instances.push(GlyphInstance::new(glyph.clone(), self.transform_point((x as f32, y as f32).into()).into(), 15.0));
            }
        }
        let mut edge_instances = Vec::with_capacity(x_max * y_max);
        for x in 1..x_max {
            for y in 0..y_max {
                let source = {
                    let y = 0;
                    &glyph_instances[x * y_max + y]
                };
                let target = {
                    let x = x - 1;
                    &glyph_instances[x * y_max + y]
                };
                edge_instances.push(Edge::new(source.clone(), target.clone(), Angle::degrees(angle)));
            }
        }
        // let start_instance = GlyphInstance::new(glyph.clone(), self.transform_point(p).into(), 30.0);
        // let end_instance = GlyphInstance::new(glyph, self.transform_point(q).into(), 30.0);
        // let edge = Edge::new(start_instance.clone(), end_instance.clone(), Angle::degrees(angle));


        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
        {
            let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
            // Create the tessellator.
            let mut stroke_tessellator = StrokeTessellator::new();
            let mut fill_tessellator = FillTessellator::new();
            for edge in &edge_instances {
                edge.tessellate(&mut vertex_builder,
                    &mut stroke_tessellator, &StrokeOptions::default(),
                    &mut fill_tessellator,
                )?;
            }   
            for instance in &glyph_instances {
                instance.draw(&mut vertex_builder, &mut fill_tessellator)?;
            }
        }
        let transform = self.transform; //.pre_translate(self.transform_point((0.0, 0.0).into()).into());
        self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
        Ok(())
    }


    pub fn test_stix_math(&mut self, p : JsPoint, q : JsPoint, angle : f32, s : String) -> Result<(), JsValue> {
        use lyon::path::iterator::PathIterator;
        use crate::glyph::{Glyph, GlyphInstance};
        let glyph = Glyph::from_stix(&s);
        let start_instance = GlyphInstance::new(glyph.clone(), self.transform_point(p).into(), 30.0);
        let end_instance = GlyphInstance::new(glyph, self.transform_point(q).into(), 30.0);
        let edge = Edge::new(start_instance.clone(), end_instance.clone(), Angle::degrees(angle));


        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
        {
            let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
            // Create the tessellator.
            let mut stroke_tessellator = StrokeTessellator::new();
            let mut fill_tessellator = FillTessellator::new();
               
            edge.tessellate(&mut vertex_builder,
                &mut stroke_tessellator, &StrokeOptions::default(),
                &mut fill_tessellator,
            )?;
            start_instance.draw(&mut vertex_builder, &mut fill_tessellator)?;
            end_instance.draw(&mut vertex_builder, &mut fill_tessellator)?;
        }
        let transform = self.transform; //.pre_translate(self.transform_point((0.0, 0.0).into()).into());
        self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
        Ok(())
    }

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

    pub fn draw_triangle(&mut self, p1 : JsPoint, p2 : JsPoint, p3 : JsPoint) -> Result<(), JsValue> {
        let mut triangles : Vec<Point> = Vec::new();
        triangles.push(p1.into());
        triangles.push(p2.into());
        triangles.push(p3.into());
        self.default_shader.draw(Transform::identity(), triangles.as_slice(), WebGl2RenderingContext::TRIANGLES)?;
        Ok(())
    }

       
    pub fn test_lyon(&mut self, draw_triangles : bool) -> Result<(), JsValue> {
        let mut path_builder = lyon::path::Path::builder();
        path_builder.move_to(point(0.0, 0.0));
        // path_builder.line_to(point(100.0, 200.0));
        // path_builder.line_to(point(200.0, 0.0));
        // path_builder.line_to(point(100.0, 100.0));
        // path_builder.close();

        path_builder.line_to(point(100.0, 100.0));
        path_builder.line_to(point(200.0, 0.0));
        path_builder.cubic_bezier_to(point(250.0, 100.0), point(550.0, 200.0), point(300.0, 200.0));
        path_builder.move_to(point(0.0, 100.0));
        path_builder.line_to(point(200.0, 100.0));
        path_builder.line_to(point(200.0, 200.0));
        let path = path_builder.build();

        let test : Vec<_> = path.iter().collect();
        log!("test : {:?}", test);
        
        // Create the destination vertex and index buffers.
        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
        
        {
            // Create the destination vertex and index buffers.
            let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
        
            // vertex_builder.begin_geometry();
            // {
            //     let mut attrib_buffer: Vec<f32> = Vec::new();
            //     let mut stroker = StrokeBuilder::new(&StrokeOptions::default(), &(), &mut attrib_buffer, &mut vertex_builder);
    
            //     for &evt in &[
            //         PathEvent::Begin { at: point(0.0, 0.0) }, 
            //         PathEvent::Line { from: point(0.0, 0.0), to: point(100.0, 100.0) }, 
            //         PathEvent::Line { from: point(100.0, 100.0), to: point(100.0, 50.0) }
            //     ] {
            //         stroker.path_event(evt);
            //     }
    
            //     stroker.build().unwrap();
            // }
            // vertex_builder.end_geometry();

            // Create the tessellator.
            let mut tessellator = StrokeTessellator::new();
        
            // Compute the tessellation.
            tessellator.tessellate(
                &path,
                &StrokeOptions::default(),
                &mut vertex_builder
            ).unwrap();
        }
        // log!("buffers : {:?}", buffers);
        let transform = self.transform.pre_translate(vector(self.transform_x(0.0), self.transform_y(0.0)));
        self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLES } else { WebGl2RenderingContext::LINE_STRIP })?;

        Ok(())
    }


    pub fn test_lyon3(&mut self, translate : JsPoint, _shorten : f32, _rotate_degrees : f32, draw_triangles : bool) -> Result<(), JsValue> {
        // let mut path = crate::path::Path::new((0.0, 0.0));
        // path.cubic_curve_to((50.0, 100.0), (350.0, 200.0), (100.0, 200.0));

        let mut path = crate::path::Path::new((self.transform_x(0.0), self.transform_y(0.0)));
        path.line_to((self.transform_x(1.0), self.transform_y(2.0)));
        // path.cubic_curve_to((self.transform_x(0.5), self.transform_y(1.0)), (self.transform_x(3.5), self.transform_y(2.0)), (self.transform_x(1.0), self.transform_y(2.0)));
        // path.shorten_start(StrokeOptions::DEFAULT_TOLERANCE, shorten);
        // path.shorten_end(StrokeOptions::DEFAULT_TOLERANCE, shorten);
        // path.add_end_arrow(StrokeOptions::DEFAULT_TOLERANCE, crate::arrow::test_arrow());

        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
            
        {
            // Create the destination vertex and index buffers.
            let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
    
            // Create the tessellator.
            let mut stroke_tessellator = StrokeTessellator::new();
            let mut fill_tessellator = FillTessellator::new();
    
            path.draw(&mut vertex_builder,
                &mut stroke_tessellator, &StrokeOptions::default(),
                &mut fill_tessellator,
            )?;
        }

        let transform = self.transform.pre_translate(translate.into());
        self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, 
            if draw_triangles { WebGl2RenderingContext::TRIANGLES } else { WebGl2RenderingContext::LINE_STRIP })?;
        Ok(())
    }    
}

use lyon::tessellation::TessellationError;
fn convert_error(err : TessellationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}
