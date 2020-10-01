use uuid::Uuid;
use std::rc::Rc;

use wasm_bindgen::JsValue;

use lyon::geom::math::{point, Point};
use lyon::path::{Path, builder::{PathBuilder, Build}};

use lyon::tessellation::{
    geometry_builder, TessellationError,
    StrokeTessellator, StrokeOptions, LineCap, LineJoin,
    FillTessellator, FillOptions, VertexBuffers,
};

use crate::error::convert_tessellation_error;
use crate::webgl_wrapper::WebGlWrapper;
// pub struct ArrowSettings {
//     length : ArrowLength,
//     width : ArrowDimension,
//     inset : ArrowDimension,
//     scale_length : f32,
//     scale_width : f32,
//     arc : Angle,
//     reverse : bool,
//     harpoon : bool,
//     color : (),
//     fill_color : (),
//     line_cap : (),
//     line_join : (),
//     line_width : ArrowDimension,

// }

// impl ArrowSettings {
//     fn set_length(dim : f32, line_width_factor : f32){

//     }

//     fn set_width(dim : f32, line_width_factor : f32){

//     }
// }

// pub struct ArrowLength {
//     dimension : f32,
//     line_width_factor : f32
// }

// pub struct ArrowDimension {
//     dimension : f32,
//     line_width_factor : f32,
//     length_factor : f32,
// }

pub struct Arrow {
    pub(crate) tip_end : f32,
    pub(crate) back_end : f32,
    pub(crate) visual_tip_end : f32,
    pub(crate) visual_back_end : f32,
    pub(crate) line_end : f32,
    pub(crate) path : Rc<Path>, 
    pub(crate) stroke : Option<StrokeOptions>, 
    pub(crate) fill : Option<FillOptions>,
    pub(crate) uuid : Uuid,
}

impl Arrow {
    pub fn tesselate_into_buffers(&self, buffers : &mut VertexBuffers<Point, u16>) -> Result<(), JsValue> {
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut fill = FillTessellator::new();
        let mut stroke = StrokeTessellator::new();

        if let Some(fill_options) = &self.fill {
            fill.tessellate(self.path.iter(), fill_options, &mut vertex_builder).map_err(convert_tessellation_error)?;
        }
        if let Some(stroke_options) = &self.stroke {
            stroke.tessellate(self.path.iter(), stroke_options, &mut vertex_builder).map_err(convert_tessellation_error)?;
        }
        Ok(())
    }
}



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


// \ifpgfarrowroundjoin%
// \else%
//   \pgfmathdivide@{\pgf@sys@tonumber\pgfutil@tempdima}{\pgf@sys@tonumber\pgfutil@tempdimb}%
//   \let\pgf@temp@quot\pgfmathresult%
//   \pgf@x\pgfmathresult pt%
//   \pgf@x\pgfmathresult\pgf@x%
//   \pgf@x49.44662\pgf@x%
//   \advance\pgf@x by1pt%  \pgfarrowlinewidth^2 + (0.41019/0.0583333 \pgftempdim@a / \pgfutil@tempdimb) \pgfarrowlinewidth^2
//   \pgfmathsqrt@{\pgf@sys@tonumber\pgf@x}%
//   \pgf@xc\pgfmathresult\pgfarrowlinewidth% xc is front miter
//   \pgf@xc.5\pgf@xc
//   \pgf@xa\pgf@temp@quot\pgfarrowlinewidth% xa is extra harpoon miter
//   \pgf@xa3.51591\pgf@xa% xa is extra harpoon miter
// \fi%
// \pgfarrowssettipend{\ifpgfarrowroundjoin.5\pgfarrowlinewidth\else\pgf@xc\ifpgfarrowharpoon\advance\pgf@x by\pgf@xa\fi\fi}

pub fn normal_arrow(line_width : f32) -> Arrow {
    let length = line_width * 2.2 + WebGlWrapper::point_to_pixels(1.6);
    let width = 2.096774 * length;
    let length_m = length - line_width;
    let width_m = width - line_width;

    let round_join = true;
    let harpoon = false;
    let reversed = false;

    let tip_end = if round_join {
            line_width / 2.0
        } else {
            let miter = ((length_m / width_m) * (length_m / width_m) * 49.44662 + 1.0).sqrt() * line_width;
            if harpoon {
                let extra_harpoon_miter = 3.51591 * (length_m / width_m) * line_width;
                miter + extra_harpoon_miter
            } else {
                miter
            }
        };
    let visual_tip_end = tip_end;

    let visual_back_end = -line_width/2.0;
    let back_end = - length_m - line_width / 2.0;

//     \ifpgfarrowreversed%
//     \ifpgfarrowharpoon%
//       \pgfarrowssetlineend{.5\pgfarrowlinewidth}%
//     \else%
//       \pgfarrowssetlineend{-.5\pgfarrowlinewidth}%
//     \fi%
//   \else%
//     \pgfarrowssetlineend{-.5\pgfarrowlinewidth}%
//   \fi%
    let line_end = if reversed {
        if harpoon {
            line_width/2.0
        } else {
            -line_width/2.0
        }
    } else {
        - line_width/2.0
    };


    let mut path_builder = Path::builder();
    path_builder.move_to(point(-length_m, width_m/2.0));

    path_builder.cubic_bezier_to(
        point(-0.81731 * length_m, 0.2 * width_m),
        point(-0.41019 * length_m, 0.05833333 * width_m),
        point(0.0, 0.0)
    );
    path_builder.cubic_bezier_to(
        point(-0.41019 * length_m, -0.05833333 * width_m),
        point(-0.81731 * length_m, -0.2 * width_m),
        point(-length_m, -width_m/2.0)
    );
    let path = Rc::new(path_builder.build());

    let stroke_options = StrokeOptions::DEFAULT.with_line_cap(LineCap::Round).with_line_join(LineJoin::Round).with_line_width(line_width);

    Arrow {
        tip_end,
        back_end,
        visual_tip_end,
        visual_back_end,
        line_end,
        path,
        stroke : Some(stroke_options),
        fill : None,
        uuid : Uuid::new_v4()
    }
}

// defaults = {
//     length = +0.75pt 1.25,
//     width'  = +0pt 4 -1,
//     line width = +0pt 1 1,
//   },

// pub fn hook_arrow(line_width : f32, angle : Angle) -> Arrow {
//     let length = line_width * 1.25 + WebGlWrapper::point_to_pixels(0.75);
//     let width = 4 * length - line_width;
//     let length_m = length - line_width / 2.0;
//     let width_m = width - line_width;
//     let angle = angle.positive();

//     let round_join = true;
//     let round_cap = true;
//     let harpoon = false;
//     let reversed = false;

// //     % Adjust width and length: Take line thickness into account:
// //     \advance\pgfarrowlength by-.5\pgfarrowlinewidth
// //     \advance\pgfarrowwidth by-\pgfarrowlinewidth
// //     \ifpgfarrowreversed
// //       \ifpgfarrowroundjoin
// //         \pgfarrowssetbackend{-.5\pgfarrowlinewidth}
// //       \fi
// //     \fi
//     // if reversed && round_join {
//     //     back_end = -0.5 * line_width;
//     // }

//     let (sin_angle, cos_angle) = angle.sin_cos();
//     let tip_end = line_width / 2.0 + length_m * (if cos_angle > 0.0 { sin_angle } else { 1.0 });

//     //     % There are four different intervals for the values of
// //     % \pgfarrowsarc that give rise to four different settings of tip
// //     % ends and so on:
// //     %
// //     % Case 1: 0 <= Angle < 90
// //     %

//     let back_end = - line_width / 2.0 + if angle < Angle::frac_pi_2() {
//         if reversed && round_join {
//             0.0
//         } else {
//             line_width / 2.0
//         }
//     } else if angle < Angle::pi() {
//         if round_cap {
//             0.0
//         } else {
//             line_width / 2.0
//         }
//     } else if angle < Angle::frac_pi_2() * 3.0 {
//         sin_angle * length_m 
//     } else {
//         - length_m
//     }

// //     \ifdim\pgfarrowarc pt<90pt%
// //     \else\ifdim\pgfarrowarc pt<180pt%
// //       \ifpgfarrowroundcap\pgfarrowssetbackend{-.5\pgfarrowlinewidth}\fi%
// //     \else\ifdim\pgfarrowarc pt<270pt%
// //         % Back end is given by sin(pgfarrowarc)*length
// //         \pgfmathsin@{\pgfarrowarc}
// //         \pgfarrowssetbackend{\pgfmathresult\pgfarrowlength\advance\pgf@x by-.5\pgfarrowlinewidth}%
// //     \else%
// //       \pgfarrowssetbackend{-\pgfarrowlength\advance\pgf@x by-.5\pgfarrowlinewidth}%
// //     \fi\fi\fi%



// //     \ifpgfarrowreversed
// //       \pgfarrowssetlineend{.5\pgfarrowlinewidth}
// //     \else%
// //       \ifpgfarrowharpoon
// //         \pgfarrowssetlineend{0pt}
// //       \else
// //         \pgfarrowssetlineend{.25\pgfarrowlinewidth}
// //       \fi
// //     \fi

//     let line_end = if reversed {
//         line_width / 2.0
//     } else {
//         if harpoon {
//             0.0
//         } else {
//             line_width / 4.0
//         }
//     }


//     // \pgfsetdash{}{+0pt}
//     // \ifpgfarrowroundjoin\pgfsetroundjoin\else\pgfsetmiterjoin\fi
//     // \ifpgfarrowroundcap\pgfsetroundcap\else\pgfsetbuttcap\fi
//     // \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
//     // {%
//     //   \pgftransformxscale{+\pgfarrowlength}
//     //   \pgftransformyscale{+.25\pgfarrowwidth}
//     //   \pgfpathmoveto{\pgfpointpolar{+\pgfarrowarc}{+1pt}\advance\pgf@y by1pt}
//     //   \pgfpatharc{\pgfarrowarc}{+-90}{+1pt}
//     //   \ifpgfarrowharpoon
//     //   \else
//     //     \pgfpatharc{+90}{+-\pgfarrowarc}{+1pt}
//     //   \fi
//     // }
//     // \ifpgfarrowharpoon\ifpgfarrowreversed
//     // \pgfpathlineto{\pgfqpoint{\pgflinewidth}{0pt}}
//     // \fi\fi
//     // \pgfusepathqstroke


//     let mut path_builder = Path::builder();
//     path_builder.move_to(point(-length_m, width_m/2.0));

//     path_builder.cubic_bezier_to(
//         point(-0.81731 * length_m, 0.2 * width_m),
//         point(-0.41019 * length_m, 0.05833333 * width_m),
//         point(0.0, 0.0)
//     );
//     path_builder.cubic_bezier_to(
//         point(-0.41019 * length_m, -0.05833333 * width_m),
//         point(-0.81731 * length_m, -0.2 * width_m),
//         point(-length_m, -width_m/2.0)
//     );
//     let path = path_builder.build();

//     let stroke_options = StrokeOptions::DEFAULT.with_line_cap(LineCap::Round).with_line_join(LineJoin::Round).with_line_width(line_width);

//     Arrow {
//         tip_end,
//         back_end,
//         visual_tip_end,
//         visual_back_end,
//         line_end,
//         path,
//         stroke : Some(stroke_options),
//         fill : None
//     }
// }



//     % Adjust arc:
//     \pgf@x\pgfarrowarc pt%
//     \advance\pgf@x by-90pt%
//     \edef\pgfarrowarc{\pgf@sys@tonumber\pgf@x}%
//     % The following are needed in the code:
//     \pgfarrowssavethe\pgfarrowlinewidth
//     \pgfarrowssavethe\pgfarrowlength
//     \pgfarrowssavethe\pgfarrowwidth
//     \pgfarrowssave\pgfarrowarc
//   },
//   drawing code = {
//     \pgfsetdash{}{+0pt}
//     \ifpgfarrowroundjoin\pgfsetroundjoin\else\pgfsetmiterjoin\fi
//     \ifpgfarrowroundcap\pgfsetroundcap\else\pgfsetbuttcap\fi
//     \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
//     {%
//       \pgftransformxscale{+\pgfarrowlength}
//       \pgftransformyscale{+.25\pgfarrowwidth}
//       \pgfpathmoveto{\pgfpointpolar{+\pgfarrowarc}{+1pt}\advance\pgf@y by1pt}
//       \pgfpatharc{\pgfarrowarc}{+-90}{+1pt}
//       \ifpgfarrowharpoon
//       \else
//         \pgfpatharc{+90}{+-\pgfarrowarc}{+1pt}
//       \fi
//     }
//     \ifpgfarrowharpoon\ifpgfarrowreversed
//     \pgfpathlineto{\pgfqpoint{\pgflinewidth}{0pt}}
//     \fi\fi
//     \pgfusepathqstroke
//   },

pub fn test_arrow() -> Arrow {
    let length = 30.0;
    let width = 2.096774 * length;
    let mut path_builder = Path::builder();
    path_builder.move_to(point(-length, width/2.0));
    path_builder.line_to(point(0.0, 0.0));
    path_builder.line_to(point(-length, -width/2.0));
    path_builder.line_to(point(-length/2.0, 0.0));
    path_builder.close();
    let path = Rc::new(path_builder.build());
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
        path,
        // fill : Some(FillOptions::DEFAULT),
        fill : None,
        stroke : Some(StrokeOptions::DEFAULT),
        // stroke : None,
        uuid : Uuid::new_v4(),
    }
}