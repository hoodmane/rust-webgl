use crate::vector::Vec2;
use crate::poly_line::PolyLine;
use crate::webgl_wrapper::WebGlWrapper;

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

pub fn normal_arrow(line_width : f32) -> PolyLine {
    let length = line_width * 2.2 + WebGlWrapper::point_to_pixels(1.6);
    let width = 2.096774 * length;
    let mut poly_line = PolyLine::new(Vec2::new(-length, width/2.0));
    poly_line.cubic_curve_to(
        Vec2::new(-0.81731 * length, 0.2 * width),
        Vec2::new(-0.41019 * length, 0.05833333 * width),
        Vec2::new(0.0, 0.0)
    );
    poly_line.cubic_curve_to(
        Vec2::new(-0.41019 * length, -0.05833333 * width),
        Vec2::new(-0.81731 * length, -0.2 * width),
        Vec2::new(-length, -width/2.0)
    );
    poly_line
}