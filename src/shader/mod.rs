mod shader;
mod default_shader;
mod line_shader;
// mod glyph_shader;

mod shader_indexed;
mod default_shader_indexed;


pub mod new_glyph_shader;
pub mod edge_shader;

pub mod edge_shader_test;
pub mod edge_shader_test2;
mod attributes;
mod data_texture;

pub use shader::{Shader, Geometry};
pub use default_shader::DefaultShader;
pub use line_shader::LineShader;
// pub use glyph_shader::{GlyphShader, HorizontalAlignment, VerticalAlignment};

// pub use new_glyph_shader::GlyphShader;

pub use shader_indexed::{ShaderIndexed, GeometryIndexed};
pub use default_shader_indexed::DefaultShaderIndexed;