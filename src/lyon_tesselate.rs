use crate::log;
use crate::lyon_path::Path;

use lyon::geom::math::Point;
use lyon::tessellation::{TessellationError, StrokeTessellator, VertexBuffers, StrokeOptions, geometry_builder::simple_builder};

use wasm_bindgen::JsValue;

fn convert_error(err : TessellationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

pub fn tesselate_path(path : &Path) -> Result<VertexBuffers<Point, u16>, JsValue> {
    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    {
        // Create the destination vertex and index buffers.
        let mut vertex_builder = simple_builder(&mut buffers);

        // Create the tessellator.
        let mut tessellator = StrokeTessellator::new();

        // Compute the tessellation.
        tessellator.tessellate(
            path.event_iterator(),
            &StrokeOptions::default(),
            &mut vertex_builder
        ).map_err(convert_error)?;
    }
    // log!("buffers : {:?}", buffers);
    Ok(buffers)
}