//! The tests of the sample (not ports: the upstream sample has no tests).
//!
//! `documents` holds the generated per-document tests (`sample-build`); `compiled_markup`
//! compares each class populated by its compiled markup with the same class populated by the
//! run-time loader; `shell` shows the main window headless and looks at what it drew.

mod compiled_markup;
mod shell;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
