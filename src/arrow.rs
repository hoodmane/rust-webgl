use crate::webgl_wrapper::WebGlWrapper;



use lyon::geom::math::{point, Point};
use lyon::path::{Path, builder::{PathBuilder, Build}};
// use crate::lyon_path::Path;

// length = +1.6pt 2.2, // 1.6pt + 2.2 * line_width
// width' = +0pt 2.096774, // 2.096774 * length
// line width = 0pt 1 1, // line width is normal line width
// round

// \pgfpathmoveto
// {\pgfqpoint{-\pgfutil@tempdima}{.5\pgfutil@tempdimb}}
// \pgfpathcurveto
// {\pgfqpoint{-0.81731\pgfutil@tempdima}{.2\pgfutil@tempdimb}}
// {\pgfqpoint{-0.41019\pgfutil@tempdima}{0.05833333\pgfutil@tempdimb}}
// {\pgfpointorigin}
// \pgfpathcurveto
// {\pgfqpoint{-0.41019\pgfutil@tempdima}{-0.05833333\pgfutil@tempdimb}}
// {\pgfqpoint{-0.81731\pgfutil@tempdima}{-.2\pgfutil@tempdimb}}
// {\pgfqpoint{-\pgfutil@tempdima}{-.5\pgfutil@tempdimb}}

pub struct Arrow {
    pub(crate) tip_end : f32,
    pub(crate) back_end : f32,
    pub(crate) visual_tip_end : f32,
    pub(crate) visual_back_end : f32,
    pub(crate) line_end : f32,
    pub(crate) stroke_path : Option<Path>,
    pub(crate) fill_path : Option<Path>,
}


pub fn normal_arrow(line_width : f32) -> Arrow {
    let length = line_width * 4.2 + WebGlWrapper::point_to_pixels(1.6);
    let width = 2.096774 * length;
    let mut path_builder = Path::builder();
    path_builder.move_to(point(-length, width/2.0));

    path_builder.cubic_bezier_to(
        point(-0.81731 * length, 0.2 * width),
        point(-0.41019 * length, 0.05833333 * width),
        point(0.0, 0.0)
    );
    path_builder.cubic_bezier_to(
        point(-0.41019 * length, -0.05833333 * width),
        point(-0.81731 * length, -0.2 * width),
        point(-length, -width/2.0)
    );
    let path = path_builder.build();
    let _tip_end = 0.0;
    let _back_end = 20.0;
    let _line_end = 10.0;
    Arrow {
        tip_end : 0.0,
        back_end : 20.0,
        visual_tip_end : 0.0,
        visual_back_end : 15.0,
        line_end : 10.0,
        fill_path : None,
        stroke_path : Some(path)
    }
}

pub fn test_arrow() -> Arrow {
    let length = 30.0;
    let width = 2.096774 * length;
    let mut path_builder = Path::builder();
    path_builder.move_to(point(-length, width/2.0));
    path_builder.line_to(point(0.0, 0.0));
    path_builder.line_to(point(-length, -width/2.0));
    path_builder.line_to(point(-length/2.0, 0.0));
    path_builder.close();
    let path = path_builder.build();
    let tip_end = 0.0;
    let visual_tip_end = 0.0;
    let back_end = -length;
    let visual_back_end = - length/2.0;
    let line_end = -length/3.0;
    Arrow {
        tip_end,
        back_end,
        visual_tip_end,
        visual_back_end,
        line_end,
        fill_path : Some(path),
        stroke_path : None
    }
}