//! Native FerroUI port of the pinned Microsoft WinUI Gallery catalog.
//!
//! The complete catalog and its original sample sources remain browsable. Executable
//! examples and migration diagnostics have separate states; browsing a page is never
//! represented as completing its WinUI behavior. See `README.md` and `port-status.json`.

#![forbid(unsafe_code)]

mod assets;
mod assets_data;
mod cat_data;
pub mod catalog;
mod catalog_data;
pub mod contrast;
mod control_samples;
pub mod examples;
mod frame_sample;
pub mod gallery;
mod input_samples;
pub mod navigation;
mod platform_samples;

#[cfg(any(feature = "desktop", feature = "capture"))]
mod app;

pub use catalog::{CatalogEntry, DocumentationLink, SampleDefinition, CATALOG, SOURCE_COMMIT};
pub use gallery::GalleryView;

#[cfg(any(feature = "desktop", feature = "capture"))]
pub use app::App;

#[cfg(test)]
mod tests;
