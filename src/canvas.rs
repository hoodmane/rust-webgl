use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};
use std::f32::consts::PI;


use crate::log;
use crate::shader::{LineShader, StencilShader, GlyphShader, HorizontalAlignment, VerticalAlignment, DefaultShader, DefaultShaderIndexed};

use crate::font::{GlyphCompiler, Glyph, Font};
use crate::node::Node;

use crate::webgl_wrapper::{WebGlWrapper, Buffer};
use lyon::geom::math::{Point, point, Vector, vector, Angle, Transform};
use crate::vector::{JsPoint, Vec4};


use crate::arrow::normal_arrow;

use crate::rect::{Rect, BufferDimensions};

use crate::arrow::Arrow;
use crate::convex_hull::{self, ConvexHull};

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
            origin : Point::new(0.0, 0.0),
            xscale : 100.0,
            yscale : 100.0,
            transform : Transform::identity(),
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

    pub fn draw_box(&mut self, x : f32, y : f32, width : f32, height : f32) -> Result<(), JsValue> {
        let mut a = GlyphCompiler::new();
        a.move_to(Point::new(0.0, 0.0));
        a.line_to(Point::new(width, 0.0));
        a.line_to(Point::new(width, height));
        a.line_to(Point::new(0.0, height));
        a.move_to(Point::new(0.0, 0.0));       
        a.close();
        let glyph = a.end();
        self.glyph_shader.draw(&glyph, self.transform, Point::new(x, y), 1.0, HorizontalAlignment::Center, VerticalAlignment::Center, Vec4::new(0.0, 0.0, 0.0, 0.0))?;
        Ok(())
    }

    pub fn draw_letter(&mut self, 
        font : &Font, codepoint : u16,  
        pos : JsPoint, scale : f32,
        horizontal_alignment : HorizontalAlignment, vertical_alignment : VerticalAlignment,
        color : Vec4
    ) -> Result<(), JsValue> {
        let glyph = font.glyph(codepoint)?.path();
        // log!("glyph : {:?}", glyph);
        self.glyph_shader.draw(glyph, self.transform, pos.into(), scale, horizontal_alignment, vertical_alignment, color)?;
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

    // fn glyph_convex_hull<'a>(&mut self, glyph : &'a Glyph) -> Result<&'a ConvexHull, JsValue>{
    //     log!("get_or_try_init");
    //     glyph.convex_hull.get_or_try_init(||{
    //         log!("Computing convex hull");
    //         let scale = CONVEX_HULL_GLYPH_SCALE;
    //         let glyph_path = glyph.path();
    
    //         self.glyph_hull_buffer.resize(self.buffer_dimensions)?;
    //         self.webgl.render_to(&mut self.glyph_hull_buffer)?;
    
    //         let (letter_raster, width, height) = self.glyph_shader.get_raster(glyph_path, self.transform, scale, &mut self.glyph_hull_buffer)?;
    //         let mut letter_hull = convex_hull::raster_to_convex_hull_polygon(&letter_raster, width as usize, height as usize, Point::new((width/2) as f32, (height/2) as f32), 2, 0.1);
    //         for v in &mut letter_hull {
    //             v.y *= -1.0;
    //             *v /= self.buffer_dimensions.density() as f32;
    //         }
    
    //         let (letter_hull_raster, width, height) = self.default_shader.get_raster(self.transform, &letter_hull, WebGl2RenderingContext::TRIANGLE_FAN, &mut self.glyph_hull_buffer)?;
    //         let channel = 3; // alpha channel
    //         let scale = self.buffer_dimensions.density() as f32 * scale;
    //         Ok(convex_hull::convex_raster_to_convex_hull(letter_hull_raster, width as usize, height as usize, scale, channel, CONVEX_HULL_ANGLE_RESOLUTION))
    //     })
    // }

    // pub fn js_find_glyph_boundary_point(&mut self, font : &Font, codepoint : u16, angle : f32) -> Result<JsPoint, JsValue> {
    //     let glyph = font.glyph(codepoint)?;
    //     Ok(self.find_glyph_boundary_point(glyph, Angle::degrees(angle))?.into())
    // }

    // fn find_glyph_boundary_point(&mut self, glyph : &Glyph, angle : Angle) -> Result<Vector, JsValue> {
    //     let convex_hull = self.glyph_convex_hull(glyph)?;
    //     Ok(convex_hull.find_boundary_point(angle))
    // }
    
    // pub fn js_glyph_hull(&mut self, font : &Font, codepoint : u16) -> Result<(), JsValue> {
    //     self.glyph_convex_hull(font.glyph(codepoint)?)?;
    //     Ok(())
    // }

    pub fn js_draw_line_to_glyph(&mut self, start : JsPoint, end : JsPoint, font : &Font, codepoint : u16, glyph_scale : f32) -> Result<() /*JsBuffer*/, JsValue> {
        let glyph = font.glyph(codepoint)?;
        let start = self.transform_point(start);
        let end = self.transform_point(end);
        let node = Node::new(glyph.clone(), end.into(), glyph_scale);
        self.draw_line_to_glyph(start.into(), &node)
    }

    // fn draw_line_to_glyph(&mut self, start : Point, node : &Node) -> Result<(), JsValue> {

    //     let mut path = crate::path::Path::new((self.transform_x(0.0), self.transform_y(0.0)));
    //     path.line_to((self.transform_x(3.0), self.transform_y(1.0)));

    //     let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
            
    //     {
    //         // Create the destination vertex and index buffers.
    //         let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
    
    //         // Create the tessellator.
    //         let mut stroke_tessellator = StrokeTessellator::new();
    //         let mut fill_tessellator = FillTessellator::new();
    
    //         path.draw(&mut vertex_builder,
    //             &mut stroke_tessellator, &StrokeOptions::default(),
    //             &mut fill_tessellator, &FillOptions::default(),
    //         )?;
    //     }

    //     let transform = self.transform;
    //     self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
    //     Ok(())
    // }    


    fn draw_line_to_glyph(&mut self, start : Point, node : &Node) -> Result<() /*JsBuffer*/, JsValue> {
        let mut path = crate::path::Path::new(start);
        path.line_to(node.center);
        // log!("node.find_boundary_distance_toward(start) : {}", node.find_boundary_distance_toward(start));
        path.shorten_end(lyon::tessellation::StrokeOptions::DEFAULT_TOLERANCE, node.find_boundary_distance_toward(start));

        // log!("p1 : {:?}", (self.transform_x(0.0), self.transform_y(0.0)));
        // log!("p2 : {:?}", (self.transform_x(3.0), self.transform_y(1.0)));


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

        let transform = self.transform; //.pre_translate(translate.into());
        self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
        // node.draw(&mut self.glyph_shader, self.transform, Vec4::new(0.2, 0.5, 0.8, 1.0))?;

        Ok(()) // Ok(JsBuffer::new(triangles))
    }

    // pub fn draw_text(&mut self) -> Result<(), JsValue> {
    //     use lyon::path::iterator::PathIterator;
    //     let path = crate::fonterator_test::test();
        
    //     let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    //     {
    //         let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
    //         let mut tessellator = FillTessellator::new();
    //         tessellator.tessellate(
    //             path.iter().map(|e| *e).transformed(&Transform::scale(30.0, 30.0)),
    //             &FillOptions::default(),
    //             &mut vertex_builder
    //         ).unwrap();
    //     }
    //     let transform = self.transform.pre_translate(self.transform_point((0.0, 0.0).into()).into());
    //     self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
    //     Ok(())
    // }


    pub fn test_stix_math(&mut self, p : JsPoint, s : String) -> Result<(), JsValue> {
        use lyon::path::iterator::PathIterator;
        use crate::glyph::{Glyph, GlyphInstance};
        let glyph = Glyph::from_stix(&s);
        let instance = GlyphInstance::new(glyph, self.transform_point((2.0, 1.0).into()).into(), 30.0);

        let start = self.transform_point(p.into()).into();
        let end = instance.find_boundary_toward(start);

        let mut path = crate::path::Path::new(start);
        path.line_to(end);
        path.add_end_arrow(StrokeOptions::DEFAULT_TOLERANCE, crate::arrow::normal_arrow(1.0));



        let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
        {
            let mut vertex_builder = geometry_builder::simple_builder(&mut buffers);
            // Create the tessellator.
            let mut stroke_tessellator = StrokeTessellator::new();
            let mut fill_tessellator = FillTessellator::new();
               
            path.draw(&mut vertex_builder,
                &mut stroke_tessellator, &StrokeOptions::default(),
                &mut fill_tessellator,
            )?;
            instance.draw(&mut vertex_builder, &mut fill_tessellator)?;
        }
        let transform = self.transform; //.pre_translate(self.transform_point((0.0, 0.0).into()).into());
        self.default_shader_indexed.draw(transform, &buffers.vertices, &buffers.indices, WebGl2RenderingContext::TRIANGLES)?;
        Ok(())
    }

    // pub fn draw_arrow(&mut self, start : Point, end : Point, line_width : f32) {
    //     let mut poly_line = PolyLine::new(start);
    //     poly_line.line_to(end);

    // }

    // pub fn get_letter_convex_hull(&mut self, 
    //     font : &Font, codepoint : u16,
    //     scale : f32,
    // ) -> Result<JsBuffer, JsValue> {

    //     let glyph = font.glyph(codepoint)?.path();
    //     self.glyph_hull_buffer.resize(self.buffer_dimensions)?;
    //     self.webgl.render_to(&mut self.glyph_hull_buffer)?;
    //     let (letter, width, height) = self.glyph_shader.get_raster(glyph, self.transform, scale, &mut self.glyph_hull_buffer)?;
    //     self.webgl.render_to_canvas(self.buffer_dimensions);
    //     let mut image = convex_hull::convex_hull(&letter, width as usize, height as usize, Point::new((width/2) as f32, (height/2) as f32), 2, 0.1);
    //     for v in &mut image {
    //         v.y *= -1.0;
    //         *v /= self.buffer_dimensions.density() as f32;
    //     }
    //     Ok(JsBuffer::new(image))
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

    pub fn draw_line(&mut self, p1 : JsPoint, p2 : JsPoint, p3 : JsPoint) -> Result<(), JsValue> {
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
