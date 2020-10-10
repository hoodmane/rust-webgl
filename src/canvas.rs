
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};
// use std::f32::consts::PI;


#[allow(unused_imports)]
use crate::log;

use crate::glyph::{Glyph, GlyphInstance};

use crate::shader::{GridShader, ChartShaders, EdgeOptions};


use crate::webgl_wrapper::WebGlWrapper;
use lyon::geom::math::{Point, point};
use crate::vector::{JsPoint, Vec4};


use crate::coordinate_system::{CoordinateSystem, BufferDimensions};

#[allow(dead_code)]
static BLACK : Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
#[allow(dead_code)]
static GRID_LIGHT_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 30.0 / 255.0);
#[allow(dead_code)]
static GRID_DARK_COLOR : Vec4 = Vec4::new(0.0, 0.0, 0.0, 90.0 / 255.0);



#[wasm_bindgen]
pub struct Canvas {
    // user affine coordinate transformation
    coordinate_system : CoordinateSystem,

    // Webgl shaders
    webgl : WebGlWrapper,
    minor_grid_shader : GridShader,
    major_grid_shader : GridShader,
    // axes_shader : LineShader,
    chart_shaders : ChartShaders
}

#[wasm_bindgen]
#[allow(dead_code)]
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
        let chart_shaders = ChartShaders::new(webgl.clone())?;

        let mut minor_grid_shader = GridShader::new(webgl.clone())?;
        minor_grid_shader.thickness(0.5);
        minor_grid_shader.color(GRID_LIGHT_COLOR);
        minor_grid_shader.grid_step(2, 2);

        let mut major_grid_shader = GridShader::new(webgl.clone())?;
        major_grid_shader.thickness(0.5);
        major_grid_shader.color(GRID_DARK_COLOR);
        major_grid_shader.grid_step(10, 10);

        let coordinate_system = CoordinateSystem::new();

        let mut result = Self {
            coordinate_system,
            webgl,
            minor_grid_shader,
            major_grid_shader,
            chart_shaders,
        };
        result.resize(result.webgl.dimensions()?)?;
        Ok(result)   
    }

    // Returns : [xNearest, yNearest, distance]
    pub fn nearest_gridpoint(&self, point : &JsPoint) -> Vec<f32> {
        let pt = point.into();
        let nearest = self.coordinate_system.transform_point(self.coordinate_system.inverse_transform_point(pt).round());
        vec![nearest.x, nearest.y, nearest.distance_to(pt)]
    }


    pub fn transform_point(&self, point : &JsPoint) -> JsPoint {
        self.coordinate_system.transform_point(point.into()).into()
    }

    pub fn inverse_transform_point(&self, point : &JsPoint) -> JsPoint {
        self.coordinate_system.inverse_transform_point(point.into()).into()
    }

    pub fn restore_context(&mut self, webgl_context : &WebGl2RenderingContext) -> Result<(), JsValue> {
        self.webgl = WebGlWrapper::new(webgl_context.clone());
        // self.grid_shader = LineShader::new(self.webgl.clone())?;
        Ok(())
    }


    pub fn set_margins(&mut self, 
        left_margin : i32,
        right_margin : i32,
        bottom_margin : i32,
        top_margin : i32,
    ) -> Result<(), JsValue> {
        self.coordinate_system.set_margins(left_margin, right_margin, bottom_margin, top_margin);
        self.update_scissor();
        Ok(())
    }

    fn update_scissor(&self){
        let coord_system = self.coordinate_system;
        let left = (coord_system.left_margin as f64 * coord_system.buffer_dimensions.density()) as i32;
        let bottom = (coord_system.bottom_margin as f64 * coord_system.buffer_dimensions.density()) as i32;
        let width = ((coord_system.buffer_dimensions.width() - coord_system.left_margin - coord_system.right_margin) as f64  * coord_system.buffer_dimensions.density()) as i32;
        let height = ((coord_system.buffer_dimensions.height() - coord_system.top_margin - coord_system.bottom_margin) as f64  * coord_system.buffer_dimensions.density()) as i32;
        self.webgl.scissor(left, bottom, width, height);
    }

    pub fn set_current_xrange(&mut self, xmin: f32, xmax: f32) {
        self.coordinate_system.set_current_xrange(xmin, xmax);
    }

    pub fn set_current_yrange(&mut self, ymin: f32, ymax: f32) {
        self.coordinate_system.set_current_yrange(ymin, ymax);
    }

    pub fn set_max_xrange(&mut self, xmin: f32, xmax: f32) {
        self.coordinate_system.set_max_xrange(xmin, xmax);
    }

    pub fn set_max_yrange(&mut self, ymin: f32, ymax: f32) {
        self.coordinate_system.set_max_yrange(ymin, ymax);
    }

    pub fn translate(&mut self, delta : JsPoint) {
        self.coordinate_system.translate(delta);
    }

    pub fn scale_around(&mut self, scale: f32, center: JsPoint) -> Result<(), JsValue> {
        self.coordinate_system.scale_around(scale, center)?;
        Ok(())
    }

    pub fn set_glyph_scale(&mut self, glyph_scale : f32){
        self.coordinate_system.glyph_scale = glyph_scale;
    }


    fn enable_clip(&self){
        self.webgl.enable(WebGl2RenderingContext::SCISSOR_TEST);
    }

    fn disable_clip(&self){
        self.webgl.disable(WebGl2RenderingContext::SCISSOR_TEST);
    }

    pub fn apply_transform(&self, p : JsPoint) -> JsPoint {
        self.coordinate_system.transform.transform_point(p.into()).into()
    }

    fn resize(&mut self, new_dimensions : BufferDimensions) -> Result<(), JsValue> {
        if new_dimensions == self.coordinate_system.buffer_dimensions {
            return Ok(());
        }
        self.coordinate_system.buffer_dimensions = new_dimensions;
        let canvas = self.webgl.canvas()?;
        canvas.style().set_property("width", &format!("{}px", new_dimensions.width()))?;
        canvas.style().set_property("height", &format!("{}px", new_dimensions.height()))?;
        canvas.set_width(new_dimensions.pixel_width() as u32);
        canvas.set_height(new_dimensions.pixel_height() as u32);
        self.coordinate_system.reset_transform();
        
        self.webgl.viewport_dimensions(new_dimensions);
        self.update_scissor();
        Ok(())
    }


    #[allow(dead_code)]
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
        self.webgl.render_to_canvas(self.coordinate_system.buffer_dimensions);
        Ok(())
    }

    pub fn clear(&mut self){
        self.clear_glyphs();
        self.clear_edges();
    }

    pub fn clear_glyphs(&mut self) {
        self.chart_shaders.clear_glyphs();
    }

    pub fn clear_edges(&mut self) {
        self.chart_shaders.clear_edges();
    }

    pub fn add_glyph(&mut self, point : &JsPoint, glyph : &Glyph, scale : f32,  &stroke_color : &Vec4,  &fill_color : &Vec4 ) -> Result<(), JsValue>  {
        self.chart_shaders.add_glyph_instance(GlyphInstance::new(glyph.clone(), point.into(), scale,  stroke_color, fill_color))?;
        Ok(())
    }



    pub fn test_edge_shader(&mut self, 
        start_point : &JsPoint, end_point : &JsPoint, 
        start_glyph : &Glyph, end_glyph : &Glyph, 
        scale : f32,
        edge_options : &EdgeOptions
    ) -> Result<(), JsValue> {        
        self.clear();
        let start : Point = start_point.into();
        let end : Point = end_point.into();
        let start_glyph = GlyphInstance::new(start_glyph.clone(), start, scale,  Vec4::new(0.0, 0.0, 0.0, 0.5), Vec4::new(1.0, 0.0, 0.0, 0.5));
        let end_glyph = GlyphInstance::new(end_glyph.clone(), end, scale,  Vec4::new(0.0, 1.0, 0.0, 0.5), Vec4::new(0.0, 0.0, 1.0, 0.5));
        self.chart_shaders.add_glyph_instance(start_glyph.clone())?;
        self.chart_shaders.add_glyph_instance(end_glyph.clone())?;

        self.chart_shaders.add_edge(start_glyph, end_glyph, edge_options)?;
 
        Ok(())
    }


    pub fn test_speed_setup(&mut self, 
        glyph1 : &Glyph, glyph2 : &Glyph, 
        xy_max : usize,  scale : f32, 
        edge_options : &EdgeOptions
    ) -> Result<(), JsValue> {
        self.clear();
        let mut glyph_instances = Vec::new();

        for x in 0..xy_max {
            for y in 0..xy_max {
                // let s = if (x + y) % 2 == 1 { &glyph1 } else { &glyph2 };
                // let r = x as f32 /  xy_max as f32;
                // let b = y as f32 /  xy_max as f32;
                // let glyph_instance = GlyphInstance::new(s.clone(), point(x as f32, y as f32), scale, Vec4::new(r, 0.0, b, 1.0), Vec4::new(b, 0.0, r, 1.0));

                let glyph = if (x + y) % 2 == 1 { glyph1 } else { glyph2 };
                let glyph_instance = GlyphInstance::new(glyph.clone(), point(x as f32, y as f32), scale, Vec4::new(0.0, 0.0, 0.0, 0.5), Vec4::new(0.0, 0.0, 1.0, 0.5));
                self.chart_shaders.add_glyph_instance(glyph_instance.clone())?;
                glyph_instances.push(glyph_instance);
            }
        }
        let x_max = xy_max;
        let y_max = xy_max;

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
                self.chart_shaders.add_edge(source, target, edge_options)?;
            }
        }
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

    
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.webgl.premultiplied_blend_mode();
        self.disable_clip();
        self.enable_clip();
        self.minor_grid_shader.draw(self.coordinate_system)?;
        self.major_grid_shader.draw(self.coordinate_system)?;
        self.chart_shaders.draw(self.coordinate_system)?;
        Ok(())
    }

    pub fn object_underneath_pixel(&self,  p : JsPoint) -> Result<Option<u32>, JsValue> {
        self.chart_shaders.object_underneath_pixel(self.coordinate_system, p.into())
    }
}

