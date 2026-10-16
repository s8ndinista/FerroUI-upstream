//! Port of `DefaultAppFixture.cs` and `OverlayPopupsAppFixture.cs`: the
//! application a collection of tests drives, started once for the
//! collection and ended after it.
//!
//! Upstream asks an Appium server to start the application (the executable
//! on Windows, the application bundle by its identifier on macOS). Here the
//! runner starts the executable of the sample itself: the one named by
//! `FERROUI_INTEGRATION_TEST_APP`, else `integration-test-app` in the
//! directory of the build the test binary belongs to (`cargo build -p
//! integration-test-app` builds it there).

use super::backend::Backend;
use super::session::Session;
use std::path::PathBuf;

/// The collections of upstream (`CollectionDefinitions.cs`): each has an
/// application of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Collection {
    Default,
    WindowDecorations,
    OverlayPopups,
}

impl Collection {
    /// The arguments the application of the collection is started with.
    fn arguments(self) -> &'static [&'static str] {
        match self {
            Collection::Default | Collection::WindowDecorations => &[],
            Collection::OverlayPopups => &["--overlayPopups"],
        }
    }
}

/// The executable of the sample.
pub fn test_app_path() -> PathBuf {
    if let Some(path) = std::env::var_os("FERROUI_INTEGRATION_TEST_APP") {
        return PathBuf::from(path);
    }
    let name = if cfg!(windows) { "integration-test-app.exe" } else { "integration-test-app" };
    let executable = std::env::current_exe().expect("the path of the test binary");
    // The test binary is in `<target>/<profile>/deps`.
    let profile_directory = executable.parent().and_then(|deps| deps.parent()).expect("the directory of the build");
    profile_directory.join(name)
}

/// `DefaultAppFixture`: the running application and its session.
pub struct DefaultAppFixture {
    session: Session,
}

impl DefaultAppFixture {
    pub fn new(collection: Collection) -> Result<DefaultAppFixture, String> {
        let app = test_app_path();
        if !app.exists() {
            return Err(format!(
                "{} does not exist: build it with `cargo build -p integration-test-app` or name it with \
                 FERROUI_INTEGRATION_TEST_APP",
                app.display()
            ));
        }
        let backend = Backend::launch(&app, collection.arguments())?;
        Ok(DefaultAppFixture { session: Session::new(backend) })
    }

    /// `Session`.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// `CreateNestedSession(appName)`: a session of another application; the
    /// name `Root` is the desktop of Windows, the only one the tests use.
    pub fn create_nested_session(&self, app_name: &str) -> Session {
        assert_eq!(app_name, "Root", "the tests open a nested session of the desktop only");
        Session::new(Backend::desktop().unwrap_or_else(|error| panic!("{error}")))
    }
}

impl Drop for DefaultAppFixture {
    /// `Dispose`: ends the application.
    fn drop(&mut self) {
        self.session.backend().quit();
    }
}
