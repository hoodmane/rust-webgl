use crate::log; 

use lazy_static::lazy_static;
use arrayvec::ArrayVec;

use std::rc::Rc;

use wasm_bindgen::JsValue;
use euclid::default::Box2D;
use fonterator::{self as font, Font}; // For parsing font file.
use footile::{Pt, PathOp};
use lyon::geom::math::{point, Point, Vector, Angle, Transform};
use lyon::path::{PathEvent, iterator::PathIterator};
use lyon::tessellation::{
    geometry_builder::SimpleBuffersBuilder, TessellationError,
    StrokeTessellator, StrokeOptions,
    FillTessellator, FillOptions,
};

use footile::{PathBuilder, Plotter, FillRule};
use pix::matte::Matte8;
use pix::Raster;

use crate::convex_hull::ConvexHull;


use crate::rect::{RectBuilder, Rect};

const FONT_SIZE: f32 = 32.0;


lazy_static!{
    static ref STIX_FONT : Font<'static> = {
        font::Font::new().push(include_bytes!("../fonts/STIX2Math.otf") as &[u8]).expect("Failed to parse font file")
    };
}

fn pt_to_euclid(p : Pt) -> Point {
    point(p.0, p.1)
}

fn copy_pathop(path_op : &PathOp) -> PathOp {
    match path_op {
        PathOp::Close() => PathOp::Close(),
        PathOp::Move(to) => PathOp::Move(*to),
        PathOp::Line(to) => PathOp::Line(*to),
        PathOp::Quad(ctrl, to) => PathOp::Quad(*ctrl, *to),
        PathOp::Cubic(ctrl1, ctrl2, to) => PathOp::Cubic(*ctrl1, *ctrl2, *to),
        PathOp::PenWidth(width) => PathOp::PenWidth(*width)
    }
}

fn pathop_bounding_box<'a, T : Iterator<Item=&'a PathOp>>(path : T) -> Box2D<f32> {
    Box2D::from_points(path.flat_map(|path_op|{
        let mut result = ArrayVec::<[_; 3]>::new();
        match path_op {
            PathOp::Close() => {},
            PathOp::Move(to) => result.push(pt_to_euclid(*to)),
            PathOp::Line(to) => result.push(pt_to_euclid(*to)),
            PathOp::Quad(ctrl, to) => {
                result.push(pt_to_euclid(*ctrl));
                result.push(pt_to_euclid(*to));
            }
            PathOp::Cubic(ctrl1, ctrl2, to) =>{
                result.push(pt_to_euclid(*ctrl1));
                result.push(pt_to_euclid(*ctrl2));
                result.push(pt_to_euclid(*to));
            } 
            PathOp::PenWidth(_) => {}
        };
        result.into_iter()
    }))
}

fn convert_path<T : Iterator<Item=PathOp>>(path : T) -> Vec<PathEvent> {
    let mut first = point(0.0, 0.0); 
    let mut from = point(0.0, 0.0);
    path.filter_map(move |path_op| {
        let result; //= None;
        match path_op {
            PathOp::Close() => {
                result = Some(PathEvent::End { last : from, first, close : true});
            }
            PathOp::Move(to) => {
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Begin { at : to });
                first = to;
                from = to;
            }
            PathOp::Line(to) => {
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Line { from, to });
                from = to;
            }
            PathOp::Quad(ctrl, to) => {
                let ctrl = pt_to_euclid(ctrl);
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Quadratic { from, ctrl, to });
                from = to;
            }
            PathOp::Cubic(ctrl1, ctrl2, to) => {
                let ctrl1 = pt_to_euclid(ctrl1);
                let ctrl2 = pt_to_euclid(ctrl2);
                let to = pt_to_euclid(to);
                result = Some(PathEvent::Cubic { from, ctrl1, ctrl2, to });
                from = to;
            }
            PathOp::PenWidth(_) => {unimplemented!()}
        }
        result
    }).collect()
}


pub struct Glyph {
    path : Vec<PathEvent>,
    convex_hull : ConvexHull,
}

impl Glyph {
    pub fn from_stix(character : &str) -> Rc<Self> {
        let path : Vec<_> = STIX_FONT.render(
            character,
            (512.0 - 64.0) / FONT_SIZE,
            font::TextAlign::Center
        ).0.collect();
        let bounding_box = pathop_bounding_box(path.iter());
        Rc::new(Self {
            path : convert_path(path.iter().map(|a| copy_pathop(a))),
            convex_hull : ConvexHull::from_path(path, bounding_box)
        })
    }
}


#[derive(Clone)]
pub struct GlyphInstance {
    glyph : Rc<Glyph>,
    center : Point,
    scale : f32
}


impl GlyphInstance {
    pub fn new(glyph : Rc<Glyph>, center : Point, scale : f32) -> Self {
        Self {
            glyph,
            center,
            scale
        }
    }

    pub fn center(&self) -> Point {
        self.center
    }

    pub fn draw(&self,
        vertex_builder : &mut SimpleBuffersBuilder, fill : &mut FillTessellator,
    ) -> Result<(), JsValue> {
        let transform = Transform::identity().then_translate(- self.glyph.convex_hull.center().to_vector()).then_scale(self.scale, self.scale).then_translate(self.center.to_vector());
        let path = self.glyph.path.iter().map(|e| *e).transformed(&transform);
        fill.tessellate(path, &FillOptions::default(), vertex_builder).map_err(convert_error)?;
        Ok(())
    }

    fn into_local_coords(&self, point : Point) -> Vector {
        (point - self.center) / self.scale
    }

    fn from_local_coords(&self, point : Vector) -> Point {
        self.center + point * self.scale
    }

    pub fn find_boundary_distance_toward(&self, p : Point) -> f32 {
        (self.find_boundary_point((p - self.center).angle_from_x_axis()) - self.center).length()
    }

    pub fn find_boundary_toward(&self, p : Point) -> Point {
        self.find_boundary_point((p - self.center).angle_from_x_axis())
    }

	pub fn find_boundary_point(&self, angle : Angle) -> Point {
        self.from_local_coords(self.glyph.convex_hull.find_boundary_point(angle))
	}


    pub fn contains_point(&self, point : Point) -> bool {
        self.glyph.convex_hull.contains_point(self.into_local_coords(point))
    }

    // pub fn point_toward(&self, point : Point) -> Point {
    //     self.glyph.convex_hull.find_boundar
    // }
}


fn convert_error(err : TessellationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}


// pub fn test_stix(stix : &str) -> Result<(Vec<PathEvent>, ConvexHull), JsValue> {
//     let font = font::Font::new().push(include_bytes!("../fonts/STIX2Math.otf") as &[u8]).ok_or("Failed to parse font file")?;
//     let path : Vec<_> = font.render(
//         stix,
//         (512.0 - 64.0) / FONT_SIZE,
//         font::TextAlign::Center
//     ).0.collect();
//     let bounding_box = pathop_bounding_box(path.iter());
//     Ok((
//         convert_path(path.iter().map(|a| copy_pathop(a))),
//         ConvexHull::from_path(path, bounding_box)
//     ))
// }

// pub fn test() -> Vec<PathEvent> {
//     let font = font::monospace_font();
//     let english = "Raster Text With Font";
//     let path = font.render(
//         english,
//         (512.0 - 64.0) / FONT_SIZE,
//         font::TextAlign::Left
//     ).0;
//     convert_path(path)
// }

