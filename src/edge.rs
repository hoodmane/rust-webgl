use wasm_bindgen::JsValue;

use lyon::geom::math::{Angle};
use lyon::tessellation::{
    geometry_builder::SimpleBuffersBuilder, TessellationError,
    StrokeTessellator, StrokeOptions,
    FillTessellator, FillOptions,
};


use crate::log;
use crate::glyph::GlyphInstance;
use crate::path::Path;


pub struct Edge {
    source : GlyphInstance,
    target : GlyphInstance,
    bend : Angle,
    color : ()
}

impl Edge {
    pub fn new(source : GlyphInstance, target : GlyphInstance, bend : Angle) -> Self {
        Self {
            source,
            target,
            bend,
            color : ()
        }
    }


    pub fn tessellate(&self,
        vertex_builder : &mut SimpleBuffersBuilder, 
        stroke : &mut StrokeTessellator, stroke_options : &StrokeOptions,
        fill : &mut FillTessellator,
    ) -> Result<(), JsValue> {
        let tolerance = StrokeOptions::DEFAULT_TOLERANCE;
        let mut path = Path::new(self.source.center());
        path.arc_to(self.target.center(), self.bend.signed().get());
        path.shorten_start_to_boundary(&self.source, tolerance);
        path.shorten_end_to_boundary(&self.target, tolerance);
        path.add_end_arrow(tolerance, crate::arrow::normal_arrow(1.0));
        path.draw(vertex_builder,
            stroke, stroke_options,
            fill,
        )?;
        Ok(())
    }
}