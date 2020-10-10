use std::collections::{BTreeMap, btree_map};

use wasm_bindgen::JsValue;

use lyon::geom::math::{Vector};

#[allow(unused_imports)]
use crate::log;
use crate::webgl_wrapper::WebGlWrapper;

use crate::vector::JsPoint;
use crate::coordinate_system::CoordinateSystem;
use crate::glyph::{Glyph, GlyphUuid, GlyphInstance};

use crate::shader::attributes::{Format, Type, NumChannels};
use crate::shader::{GlyphShader, HitCanvasShader, EdgeShader, EdgeOptions, DataTexture};



// struct NodeId(usize);
// struct EdgeId(usize);
pub struct ChartShaders {
    glyph_map : BTreeMap<GlyphUuid, usize>,

    glyph_convex_hulls : DataTexture<Vector>,
    
    pub glyph_shader : GlyphShader,
    pub edge_shader : EdgeShader,
    pub hit_canvas_shader : HitCanvasShader,

}

impl ChartShaders {
    pub fn new(webgl : WebGlWrapper) -> Result<Self, JsValue> {
        let glyph_convex_hulls = DataTexture::new(webgl.clone(), Format(Type::F32, NumChannels::Four));
        let glyph_shader = GlyphShader::new(webgl.clone())?;
        let hit_canvas_shader = HitCanvasShader::new(webgl.clone())?;
        let edge_shader = EdgeShader::new(webgl.clone())?;
        Ok(Self { 
            glyph_map : BTreeMap::new(),
            glyph_convex_hulls,
            glyph_shader,
            hit_canvas_shader,
            edge_shader,
        })
    }

    pub fn clear_glyphs(&mut self) {
        self.glyph_map.clear();
        self.hit_canvas_shader.clear_glyphs();
        self.glyph_shader.clear_glyphs();
    }

    pub fn clear_edges(&mut self) {
        self.edge_shader.clear();
    }

    fn glyph_index(&mut self, glyph : &Glyph) -> Result<usize, JsValue>{
        let next_index = self.glyph_map.len();
        let entry = self.glyph_map.entry(glyph.uuid);
        Ok(match entry {
            btree_map::Entry::Occupied(oe) => *oe.get(),
            btree_map::Entry::Vacant(ve) => {
                self.glyph_shader.add_glyph_data(glyph)?;
                self.glyph_convex_hulls.append(glyph.boundary().iter().copied());
                *ve.insert(next_index)
            }
        })
    }

    pub fn add_glyph_instance(&mut self, glyph_instance : GlyphInstance) -> Result<(), JsValue> {
        let glyph_index = self.glyph_index(&glyph_instance.glyph)?;
        self.glyph_shader.add_glyph_instance(glyph_instance.clone(), glyph_index);
        self.hit_canvas_shader.add_glyph(glyph_instance, glyph_index)?;
        Ok(())
    }

    pub fn add_edge(&mut self, start : GlyphInstance, end : GlyphInstance, options : &EdgeOptions) -> Result<(), JsValue> {
        let start_glyph_index = self.glyph_index(&start.glyph)?;
        let end_glyph_index = self.glyph_index(&end.glyph)?;
        self.edge_shader.add_edge(start, end, start_glyph_index, end_glyph_index, options)?;
        Ok(())
    }

    pub fn object_underneath_pixel(&self, coordinate_system : CoordinateSystem, p : JsPoint) -> Result<Option<u32>, JsValue> {
        self.hit_canvas_shader.object_underneath_pixel(coordinate_system, p.into())
    }

    pub fn draw(&mut self, coordinate_system : CoordinateSystem) -> Result<(), JsValue> {
        self.glyph_shader.draw(coordinate_system)?;
        self.edge_shader.draw(coordinate_system, &mut self.glyph_convex_hulls)?;
        self.hit_canvas_shader.draw(coordinate_system, &mut self.glyph_convex_hulls)?;
        Ok(())
    }
}


impl ChartShaders {
    // fn glyph_data(&mut self, glyph : &Glyph) -> Result<ShaderGlyphHeader, JsValue> {
    //     let entry = self.glyph_map.entry(glyph.uuid);
    //     // If btree_map::Entry had a method "or_try_insert(f : K -> Result<V, E>) -> Result<&V, E>" we could use that instead.
    //     match entry {
    //         btree_map::Entry::Occupied(oe) => Ok(*oe.get()),
    //         btree_map::Entry::Vacant(ve) => {
    //             let index = self.vertices_data.len() / 3;
    //             let index : Result<u16, _> = index.try_into();
    //             let index = index.map_err(|_| "Too many total glyph vertices : max number of triangles in all glyphs is 65535.")?;

    //             let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    //             let scale = 100.0;
                
    //             glyph.tessellate_fill(&mut buffers, scale)?;
    //             let num_fill_triangles = buffers.indices.len()  / 3;
    //             self.vertices_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                
    //             buffers.vertices.clear();
    //             buffers.indices.clear();

    //             glyph.tessellate_stroke(&mut buffers, scale)?;
    //             let num_stroke_triangles = buffers.indices.len() / 3;
    //             self.vertices_data.append(buffers.indices.iter().map(|&i| buffers.vertices[i as usize]));
                
    //             self.max_glyph_num_triangles = self.max_glyph_num_triangles.max(num_fill_triangles + num_stroke_triangles);

    //             let num_fill_triangles = num_fill_triangles.try_into().unwrap();
    //             let num_stroke_triangles  = num_stroke_triangles.try_into().unwrap();
    //             Ok(*ve.insert(ShaderGlyphHeader {
    //                 index, 
    //                 num_fill_triangles, 
    //                 num_stroke_triangles,
    //                 padding : 0
    //             }))
    //         }
    //     }
    // }
}

// #[derive(Clone)]
// pub struct Node {
//     pub(crate) glyph : Glyph,
//     pub(crate) center : Point,
//     pub(crate) scale : f32,
//     pub(crate) stroke_color : Vec4,
//     pub(crate) fill_color : Vec4,
// }

// #[derive(Clone, Copy, Debug)]
// #[repr(C)]
// struct GlyphShaderNodeHeader {
//     index : u16,
//     num_fill_triangles : u16,
//     num_stroke_triangles : u16,
//     padding : u16,
// }

// #[derive(Clone, Copy, Debug)]
// #[repr(C)]
// struct GlyphShaderNodeInstance {
//     position : Point,
//     scale : f32,
//     fill_color : [u16;2],
//     stroke_color : [u16;2],
    
//     // aGlyphData
//     glyph : GlyphShaderGlyphHeader
// }
