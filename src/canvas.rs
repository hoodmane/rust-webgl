
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext};
// use std::f32::consts::PI;


#[allow(unused_imports)]
use crate::log;

use crate::shader::{GridShader, GlyphShader, EdgeShader};


use crate::webgl_wrapper::{WebGlWrapper};
use lyon::geom::math::{Point, point, Angle};
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
    glyph_shader : GlyphShader,
    edge_shader : EdgeShader,
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
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let edge_shader = EdgeShader::new(webgl.clone())?;
        
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
            glyph_shader,
            edge_shader,
        };
        result.resize(result.webgl.dimensions()?)?;
        Ok(result)   
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
        
        self.webgl.viewport(new_dimensions);
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

    pub fn test_edge_shader(&mut self, start : JsPoint, end : JsPoint, s1 : String, s2 : String, degrees : f32, scale : f32, thickness : f32) -> Result<(), JsValue> {
        use crate::glyph::{Glyph, GlyphInstance};
        
        let start : Point = start.into();
        let end : Point = end.into();
        let glyph1 = Glyph::from_stix(&s1);
        let glyph2 = Glyph::from_stix(&s2);
        let start_glyph = GlyphInstance::new(glyph1, start, scale,  Vec4::new(0.0, 0.0, 0.0, 0.5), Vec4::new(1.0, 0.0, 0.0, 0.5));
        let end_glyph = GlyphInstance::new(glyph2, end, scale,  Vec4::new(0.0, 1.0, 0.0, 0.5), Vec4::new(0.0, 0.0, 1.0, 0.5));
        self.glyph_shader.clear_glyphs();
        self.glyph_shader.add_glyph(start_glyph.clone())?;
        self.glyph_shader.add_glyph(end_glyph.clone())?;

        self.glyph_shader.prepare()?;
        self.glyph_shader.draw(self.coordinate_system);

        let arrow = crate::arrow::test_arrow();
        self.edge_shader.clear();
        self.edge_shader.add_edge(start_glyph.clone(), end_glyph.clone(), Some(&arrow), Some(&arrow), Angle::degrees(degrees), thickness)?;
        self.edge_shader.prepare()?;
 
        // edge_shader.draw(self.transform, self.origin, point(self.scale.x, -self.scale.y));
        self.edge_shader.draw(self.coordinate_system);
        Ok(())
    }


    pub fn test_speed_setup(&mut self, s1 : String, s2 : String, xy_max : usize,  scale : f32, degrees : f32, thickness : f32) -> Result<(), JsValue> {
        use crate::glyph::{Glyph, GlyphInstance};
        let glyph1 = Glyph::from_stix(&s1);
        let glyph2 = Glyph::from_stix(&s2);
        let mut glyph_instances = Vec::new();

        self.glyph_shader.clear_glyphs();
        self.edge_shader.clear();


        for x in 0..xy_max {
            for y in 0..xy_max {
                // let s = if (x + y) % 2 == 1 { &glyph1 } else { &glyph2 };
                // let r = x as f32 /  xy_max as f32;
                // let b = y as f32 /  xy_max as f32;
                // let glyph_instance = GlyphInstance::new(s.clone(), point(x as f32, y as f32), scale, Vec4::new(r, 0.0, b, 1.0), Vec4::new(b, 0.0, r, 1.0));

                let s = if (x + y) % 2 == 1 { &glyph1 } else { &glyph2 };
                let glyph_instance = GlyphInstance::new(s.clone(), point(x as f32, y as f32), scale, Vec4::new(0.0, 0.0, 0.0, 0.5), Vec4::new(0.0, 0.0, 1.0, 0.5));
                self.glyph_shader.add_glyph(glyph_instance.clone())?;
                glyph_instances.push(glyph_instance);
            }
        }
        let x_max = xy_max;
        let y_max = xy_max;


        let arrow = crate::arrow::normal_arrow(thickness);
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
                self.edge_shader.add_edge(source, target, Some(&arrow), Some(&arrow), angle, thickness)?;
            }
        }

        self.glyph_shader.prepare()?;
        self.edge_shader.prepare()?;
        self.test_speed();
        Ok(())
    }

    pub fn test_speed(&mut self) {
        self.glyph_shader.draw(self.coordinate_system);
        self.edge_shader.draw(self.coordinate_system);
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

    
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.webgl.premultiplied_blend_mode();
        self.disable_clip();
        // self.axes_shader.draw(self.transform)?;
        self.enable_clip();
        self.minor_grid_shader.draw(self.coordinate_system)?;
        self.major_grid_shader.draw(self.coordinate_system)?;
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

