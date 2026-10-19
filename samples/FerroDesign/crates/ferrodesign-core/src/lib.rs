//! Renderer-independent vector-design document and editing engine.
//! Logical positions are parent-relative; rotation is about each node's center.
mod editor;
mod effects_edit;
mod components;
mod geometry;
mod image_edit;
mod image_info;
mod layout;
mod layout_edit;
mod layout_drag;
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
pub use effects_edit::*;
pub use components::*;
pub use geometry::*;
pub use image_edit::*;
pub use image_info::*;
pub use layout::*;
pub use layout_drag::*;
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
