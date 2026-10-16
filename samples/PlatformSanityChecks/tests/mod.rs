//! The tests of the sample (not ports: the upstream sample has no tests).
//!
//! `documents` holds the generated per-document test (`sample-build`): `document_app` loads
//! `App.xaml` through the run-time loader. The checks of the program need the X11 platform and
//! an X server; they are run by the program itself (`program.rs`).

mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}

use crate::{App, SAMPLE};
use ferroui_controls::ApplicationImpl;

/// `App.xaml` names no class: it is not compiled as the document of a class, and the table of
/// the classes with a document is empty.
#[test]
fn the_document_names_no_class() {
    crate::register_types();
    assert_eq!(SAMPLE.documents, &[("/App.xaml", None)]);
    assert!(!SAMPLE.is_compiled("/App.xaml"));
    assert_eq!(SAMPLE.xaml_classes().count(), 0);
}

/// The load of the application from its document fails, as the load by the XAML loader of the
/// reference does for a class without precompiled markup.
#[test]
#[should_panic(expected = "/App.xaml")]
fn the_application_does_not_load_its_document() {
    let app = App::new();
    <App as ApplicationImpl>::initialize(&app);
}
