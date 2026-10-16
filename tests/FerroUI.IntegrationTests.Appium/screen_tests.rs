//! Port of `ScreenTests.cs`.

use crate::infrastructure::{Collection, DefaultAppFixture, Session, TestBase, TestClass, Tests};
use ferroui_base::Rect;
use ferroui_controls::platform::ScreenOrientation;

pub struct ScreenTests {
    base: TestBase,
}

impl TestClass for ScreenTests {}

/// `Enum.Parse<ScreenOrientation>(text)`.
fn parse_screen_orientation(text: &str) -> ScreenOrientation {
    match text {
        "None" => ScreenOrientation::None,
        "Landscape" => ScreenOrientation::Landscape,
        "Portrait" => ScreenOrientation::Portrait,
        "LandscapeFlipped" => ScreenOrientation::LandscapeFlipped,
        "PortraitFlipped" => ScreenOrientation::PortraitFlipped,
        other => panic!("Requested value '{other}' was not found."),
    }
}

/// `bool.Parse(text)`.
fn parse_bool(text: &str) -> bool {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" => true,
        "false" => false,
        _ => panic!("String '{text}' was not recognized as a valid Boolean."),
    }
}

impl ScreenTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        ScreenTests { base: TestBase::new(fixture, "Screens") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn can_read_current_screen_info(&self) {
        let refresh_button = self.session().find_element_by_accessibility_id("ScreenRefresh");
        refresh_button.send_click();

        let screen_name = self.session().find_element_by_accessibility_id("ScreenName").text();
        let screen_handle = self.session().find_element_by_accessibility_id("ScreenHandle").text();
        let screen_bounds = Rect::parse(&self.session().find_element_by_accessibility_id("ScreenBounds").text())
            .expect("the bounds of the screen");
        let screen_work_area = Rect::parse(&self.session().find_element_by_accessibility_id("ScreenWorkArea").text())
            .expect("the working area of the screen");
        let screen_scaling: f64 = self
            .session()
            .find_element_by_accessibility_id("ScreenScaling")
            .text()
            .parse()
            .expect("the scaling of the screen");
        let screen_orientation =
            parse_screen_orientation(&self.session().find_element_by_accessibility_id("ScreenOrientation").text());

        // `Assert.NotNull`: an element's text is never null.
        let _ = (&screen_name, &screen_handle);
        assert!(screen_bounds.width > 0.0 && screen_bounds.height > 0.0);
        assert!(screen_work_area.width > 0.0 && screen_work_area.height > 0.0);
        assert!(screen_bounds.width >= screen_work_area.width);
        assert!(screen_bounds.height >= screen_work_area.height);
        assert!(screen_scaling > 0.0);
        assert!(screen_orientation != ScreenOrientation::None);
    }

    pub fn returns_the_same_screen_instance(&self) {
        let refresh_button = self.session().find_element_by_accessibility_id("ScreenRefresh");
        refresh_button.send_click();

        let screen_name1 = self.session().find_element_by_accessibility_id("ScreenName").text();
        let screen_handle1 = self.session().find_element_by_accessibility_id("ScreenHandle").text();

        refresh_button.send_click();

        let screen_name2 = self.session().find_element_by_accessibility_id("ScreenName").text();
        let screen_handle2 = self.session().find_element_by_accessibility_id("ScreenHandle").text();
        let screen_same_reference = parse_bool(&self.session().find_element_by_accessibility_id("ScreenSameReference").text());

        assert_eq!(screen_name1, screen_name2);
        assert_eq!(screen_handle1, screen_handle2);
        assert!(screen_same_reference);
    }
}

pub fn tests(tests: &mut Tests) {
    tests
        .class("screen_tests", Collection::Default, ScreenTests::new)
        .fact("can_read_current_screen_info", ScreenTests::can_read_current_screen_info)
        .fact("returns_the_same_screen_instance", ScreenTests::returns_the_same_screen_instance);
}
