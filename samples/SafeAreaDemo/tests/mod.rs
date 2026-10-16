//! The tests of the sample (not ports: the upstream sample has no tests).
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>`
//! loads the document through the run-time loader and `class_<name>` constructs its class and
//! shows it in a window.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.
//! `view` drives the main view headless: it shows the main window, lays out, renders frames
//! through the compositor with Skia, and drives the view model with an insets manager and an
//! input pane of the test in place of the ones of a mobile platform.

mod compiled_markup;
mod support;
mod view;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
