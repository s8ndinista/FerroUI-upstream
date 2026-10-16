//! What upstream's tests use of Appium and of their own helpers, over the
//! automation backend of the platform (docs/porting/integration-tests.md).

// The layer is the API upstream's tests are written against; the tests of one platform use a part of it.
#![allow(dead_code)]

pub mod actions;
pub mod backend;
pub mod default_app_fixture;
pub mod element_extensions;
pub mod keys;
pub mod platform_fact;
pub mod registry;
pub mod session;
pub mod test_base;
pub mod xpath;

pub use actions::{ActionBuilder, Actions, CoordinateOrigin, PointerInputDevice};
pub use default_app_fixture::{Collection, DefaultAppFixture};
pub use element_extensions::{Disposable, WindowChrome};
pub use keys::Keys;
pub use platform_fact::TestPlatforms;
pub use registry::Tests;
pub use session::{By, Element, MouseButton, Session, WebDriverError};
pub use test_base::{TestBase, TestClass};
