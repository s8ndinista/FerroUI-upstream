//! Port of `TestBase.cs`: every test class selects its page of the
//! application before a test.

use super::default_app_fixture::DefaultAppFixture;
use super::session::{By, Session, WebDriverError};
use std::time::Duration;

pub struct TestBase {
    session: Session,
}

impl TestBase {
    pub fn new(fixture: &DefaultAppFixture, page_name: &str) -> TestBase {
        let session = fixture.session().clone();

        let mut retry = 0;

        loop {
            let result = (|| -> Result<(), WebDriverError> {
                let pager = session.try_find_element_by_accessibility_id("Pager")?;
                let page = pager.try_find_element(By::Name(page_name.to_owned()))?;
                page.click();

                // If the mouse was captured, the first click might have just released the capture, try again
                if !page.selected() {
                    page.click();
                }

                Ok(())
            })();
            match result {
                Ok(()) => break,
                Err(_) if retry < 3 => {
                    retry += 1;
                    // MacOS sometimes seems to need a bit of time to get itself back in order after switching out
                    // of fullscreen.
                    std::thread::sleep(Duration::from_millis(1000));
                }
                Err(error) => panic!("{error}"),
            }
        }

        TestBase { session }
    }

    /// `Session`.
    pub fn session(&self) -> &Session {
        &self.session
    }
}

/// A test class of upstream: its `Dispose`, which runs after every test of
/// the class (its constructor runs before, as the registration of the test
/// names it).
pub trait TestClass {
    fn dispose(&self) {}
}
