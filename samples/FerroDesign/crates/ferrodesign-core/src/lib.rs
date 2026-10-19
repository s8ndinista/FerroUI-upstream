//! Renderer-independent vector-design document and editing engine.
//! Logical positions are parent-relative; rotation is about each node's center.
mod editor;
mod geometry;
mod layout;
mod model;
mod prototype;
mod seed;
mod spatial;
mod svg;
mod svg_import;
mod text_edit;
mod variable_edit;
mod vector_edit;
pub use editor::*;
pub use geometry::*;
pub use layout::*;
pub use model::*;
pub use prototype::*;
pub use seed::*;
pub use spatial::*;
pub use svg::*;
pub use svg_import::*;
pub use text_edit::*;
pub use variable_edit::*;
pub use vector_edit::*;
#[cfg(test)]
mod svg_import_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vector_edit_tests;
