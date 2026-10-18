//! Renderer-independent vector-design document and editing engine.
//! Logical positions are parent-relative; rotation is about each node's center.
mod editor;
mod geometry;
mod layout;
mod model;
mod seed;
mod spatial;
mod svg;
mod svg_import;
pub use editor::*;
pub use geometry::*;
pub use layout::*;
pub use model::*;
pub use seed::*;
pub use spatial::*;
pub use svg::*;
pub use svg_import::*;
#[cfg(test)]
mod svg_import_tests;
#[cfg(test)]
mod tests;
