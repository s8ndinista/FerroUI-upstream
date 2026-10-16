//! Shared helpers of the tests: the application scope and the bodies of the generated
//! per-document tests.

use crate::{register_types, SAMPLE};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Application;
use sample_testing::TestGlobalClock;
use std::rc::Rc;

/// Starts an application of the framework as the application of the test, with the test
/// services of the samples in place of the services of a platform. Not the application of the
/// sample: [`crate::App`] does not load its document (`App.xaml` names no class), and the
/// program never initializes it.
pub fn start_application() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start_with(sample_testing::services(Rc::new(TestGlobalClock::default())), Application::new)
}

/// The body of the generated test `document_<name>`.
pub fn document_loads(path: &str) {
    let _app = start_application();
    sample_testing::documents::document_loads(&SAMPLE, path);
}

/// The body of the generated test `class_<name>` (none: the document names no class).
#[allow(dead_code)]
pub fn class_constructs(path: &str) {
    let _app = start_application();
    sample_testing::documents::class_constructs(&SAMPLE, path);
}
