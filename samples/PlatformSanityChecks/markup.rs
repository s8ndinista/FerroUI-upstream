//! The document of the sample. `App.xaml` names no class (`x:Class`), so it is not the document
//! of a class: the table of the classes with a document is empty.

use crate::register_types::ASSEMBLY;
use sample_support::Sample;

pub use sample_support::XamlClass;

// `ASSETS`, `DOCUMENT_ASSETS` (a test build), `DOCUMENTS` and `EXCLUDED` (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

// `COMPILED`: the documents the build compiled with a class (none here; `build.rs`).
include!(concat!(env!("OUT_DIR"), "/compiled_classes.rs"));

/// What the sample states about its documents and assets.
pub static SAMPLE: Sample = Sample {
    assembly: &ASSEMBLY,
    register_types: crate::register_types::register_types,
    classes: &[],
    compiled: COMPILED,
    assets: ASSETS,
    #[cfg(test)]
    document_assets: DOCUMENT_ASSETS,
    #[cfg(not(test))]
    document_assets: &[],
    documents: DOCUMENTS,
};
