//! The tests of the content of the sample (not ports: the upstream sample has no tests).
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>`
//! loads the document through the run-time loader and `class_<name>` constructs its class and
//! shows it in a window.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.
//! `view` shows the embedded view in a window of the headless shell, as the panel shows it, and
//! drives its handlers: the button, the slider, hover and ink.

mod compiled_markup;
mod support;
mod view;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
