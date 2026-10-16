//! The tests of the gallery.
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>` loads
//! the document through the run-time loader and `class_<name>` constructs its class and shows it
//! in a window, under the theme of the gallery.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.
//! `pages` checks the table of the pages and the native pages beside them, and shows every page
//! with the checks of the capture mode (templates, forced states).

mod compiled_markup;
mod pages;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
