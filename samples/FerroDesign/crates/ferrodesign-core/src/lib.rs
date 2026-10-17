//! Renderer-independent vector-design document and editing engine.
//! Logical positions are parent-relative; rotation is about each node's center.
mod geometry;
mod model;
mod layout;
mod spatial;
mod editor;
mod svg;
mod seed;
pub use geometry::*;
pub use model::*;
pub use layout::*;
pub use spatial::*;
pub use editor::*;
pub use svg::*;
pub use seed::*;
#[cfg(test)] mod tests;

