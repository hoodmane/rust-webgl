use crate::rect::{Rect, RectBuilder};
use crate::convex_hull::ConvexHull;

use std::rc::Rc;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use once_cell::unsync::OnceCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use js_sys::{ArrayBuffer, DataView};
use web_sys::Response;

use lyon::geom::math::{Point, Vector};
use crate::vector::{Vec4};

enum PathCommand {
    MoveTo = 0,
    LineTo = 1,
    CurveTo = 2,
    Close = 3,
}


impl TryFrom<u8> for PathCommand {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == PathCommand::MoveTo as u8 => Ok(PathCommand::MoveTo),
            x if x == PathCommand::LineTo as u8 => Ok(PathCommand::LineTo),
            x if x == PathCommand::CurveTo as u8 => Ok(PathCommand::CurveTo),
            x if x == PathCommand::Close as u8 => Ok(PathCommand::Close),
            _ => Err(()),
        }
    }
}

struct DataReader<'a> {
    data_view : &'a DataView,
    offset : usize,
}

impl<'a> DataReader<'a> {
    pub fn new(data_view : &'a DataView) -> Self {
        Self { 
            data_view,
            offset : 0
        }
    }

    fn read_u8(&mut self) -> u8 {
        let result = self.data_view.get_uint8(self.offset);
        self.offset += 1;
        result
    }

    fn read_u16(&mut self) -> u16 {
        let result = self.data_view.get_uint16_endian(self.offset, true);
        self.offset += 2;
        result
    }    

    fn read_i16(&mut self) -> i16 {
        let result = self.data_view.get_int16_endian(self.offset, true);
        self.offset += 2;
        result
    }

    fn read_i32(&mut self) -> i32 {
        let result = self.data_view.get_int32_endian(self.offset, true);
        self.offset += 4;
        result
    }

    fn seek_to(&mut self, offset : usize){
        self.offset = offset;
    }
}


pub async fn fetch_font() -> Result<DataView, JsValue> {
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_str("fonts/fonts.bin")).await?;
    
    // `resp_value` is a `Response` object.
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let array_buffer_jsvalue = JsFuture::from(resp.array_buffer()?).await?;
    let array_buffer : ArrayBuffer = array_buffer_jsvalue.dyn_into().unwrap();
    let data_view = DataView::new(&array_buffer, 0, array_buffer.byte_length() as usize);
    Ok(data_view)
}

#[wasm_bindgen]
pub async fn read_font() -> Result<Font, JsValue> {
    let data_reader = fetch_font().await?;
    Ok(Font::new(data_reader))
}

#[wasm_bindgen]
pub struct Font {
    data : DataView,
    scale : f32,
    ascender : f32,
    descender : f32,
    glyphs : HashMap<u16, Rc<Glyph>>,
    italic_glyphs : HashMap<u16, Rc<Glyph>>,
}

impl Font {
    pub fn new(data : DataView) -> Self {
        let mut glyphs = HashMap::new();
        let mut italic_glyphs = HashMap::new();
        let mut data_reader = DataReader::new(&data);
        let scale = 1.0 / (data_reader.read_i16() as f32);
        let ascender = (data_reader.read_i16() as f32) * scale;
        let descender = (data_reader.read_i16() as f32) * scale;
        let num_glyphs = data_reader.read_i16();
        for _ in 0..num_glyphs {
            let code_point = data_reader.read_u16();
            let advance_width = (data_reader.read_i16() as f32) * scale;
            let byte_offset = data_reader.read_i32() as usize;
            let byte_length = data_reader.read_i16() as usize;
            let glyph = Rc::new(Glyph {
                code_point : code_point & 0x7FFF,
                advance_width,
                byte_offset,
                byte_length,
                path : OnceCell::new(),
                convex_hull : OnceCell::new(),
            });
            if code_point & 0x8000 == 0 {
                glyphs.insert(code_point, glyph);
            } else {
                italic_glyphs.insert(code_point, glyph);
            }
        }
        Font {
            data,
            scale,
            ascender,
            descender,
            glyphs,
            italic_glyphs
        }
    }

    fn read_point(&self, data_reader : &mut DataReader) -> Point {
        let x = (data_reader.read_i16() as f32) * self.scale;
        let y = (data_reader.read_i16() as f32) * self.scale; // + self.ascender;
        Point::new(x, y)
    }    


    pub fn glyph(&self, code_point : u16) -> Result<&Rc<Glyph>, JsValue> {
        let glyph = self.glyphs.get(&code_point).ok_or("unknown glyph")?;
        glyph.path.get_or_try_init::<_, JsValue>(|| {
            let end = glyph.byte_offset + glyph.byte_length;
            let mut data_reader = DataReader::new(&self.data);
            data_reader.seek_to(glyph.byte_offset);
            let mut glyph_compiler = GlyphCompiler::new();
            while data_reader.offset < end {
                match data_reader.read_u8().try_into().or_else(|_| Err(JsValue::from_str(&"Invalid path command in font data")))? {
                    PathCommand::MoveTo => glyph_compiler.move_to(self.read_point(&mut data_reader)),
                    PathCommand::LineTo => glyph_compiler.line_to(self.read_point(&mut data_reader)),
                    PathCommand::CurveTo => glyph_compiler.curve_to(self.read_point(&mut data_reader), self.read_point(&mut data_reader)),
                    PathCommand::Close => glyph_compiler.close(),
                }
            }
            Ok(glyph_compiler.end())
        })?;
        Ok(glyph)
    }
}


pub struct Glyph {
    code_point : u16,
    advance_width : f32,
    byte_offset : usize,
    byte_length : usize,
    path : OnceCell<GlyphPath>,
    pub convex_hull : OnceCell<ConvexHull>
}

impl Glyph {
    pub fn path(&self) -> &GlyphPath {
        &self.path.get().unwrap()
    }

    pub fn vertices(&self) -> &Vec<Vec4> {
        &self.path().vertices
    }

    pub fn bounding_box(&self) -> Rect {
        self.path().bounding_box
    }
}


#[derive(Debug)]
pub struct GlyphPath {
    pub vertices : Vec<Vec4>,
    pub bounding_box : Rect
} 


pub struct GlyphCompiler {
	// const _pool GPU.BufferPool
	vertices : Vec<Vec4>,
	first : Point,
	current : Point,
    contour_count : u32,
	bounding_box_builder : RectBuilder,
}

impl GlyphCompiler {
	pub fn new() -> Self {
        Self {
            vertices : Vec::new(),
            first : Point::new(0.0, 0.0),
            current : Point::new(0.0, 0.0),
            contour_count : 0,
            bounding_box_builder : RectBuilder::new()
        }
    }

	pub fn move_to(&mut self, p : Point) {
        // log_str(&format!("move_to {:?}", p));
        self.first = p;
        self.current = p;
		self.contour_count = 0
	}

	pub fn line_to(&mut self, p : Point) {
        // log_str(&format!("line_to {:?}", p));
        self.contour_count += 1;
		if self.contour_count >= 2 {
			self.append_triangle(self.first, self.current, p)
		}

		self.current = p;
	}

	pub fn curve_to(&mut self, c : Point, p : Point) {
        // log_str(&format!("curve_to {:?}, {:?}", c, p));
        self.contour_count += 1;
        if self.contour_count >= 2 {
			self.append_triangle(self.first, self.current, p)
		}
        self.append_curve(self.current, c, p);
		self.current = p;
	}

	pub fn close(&mut self) {
        // log_str(&format!("close"));
        self.current = self.first;
		self.contour_count = 0;
	}

	pub fn end(self) -> GlyphPath {
        // log_str(&format!("Vertices ::\n\n {:?}", self.vertices));
		GlyphPath {
            vertices : self.vertices,
            bounding_box : self.bounding_box_builder.build()
        } 
	}

	fn append_triangle(&mut self, a : Point, b : Point, c : Point) {
        self.append_vertex(a, 0.0, 1.0);
        self.append_vertex(b, 0.0, 1.0);
        self.append_vertex(c, 0.0, 1.0);
    }

    fn append_curve(&mut self, a : Point, b : Point, c : Point) {
        self.append_vertex(a, 0.0, 0.0);
        self.append_vertex(b, 0.5, 0.0);
        self.append_vertex(c, 1.0, 1.0);
	}

	fn append_vertex(&mut self, p : Point, s : f32, t : f32) {
		self.bounding_box_builder.include(p);
		self.vertices.push(Vec4::new(p.x, p.y, s, t));
	}
}