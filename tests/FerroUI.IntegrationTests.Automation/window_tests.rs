//! Port of `WindowTests.cs`.

use crate::infrastructure::session::SCREENSHOT_NOT_PROVIDED_ON_MAC_OS;
use crate::infrastructure::{
    Actions, By, Collection, DefaultAppFixture, Disposable, Element, Keys, Session, TestBase, TestClass, TestPlatforms,
    Tests, WebDriverError,
};
use ferroui_base::{PixelPoint, PixelRect, PixelSize, Size};
use ferroui_controls::{WindowStartupLocation, WindowState};
use std::time::Duration;

/// `ShowWindowMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowWindowMode {
    NonOwned,
    Owned,
    Modal,
}

const SHOW_WINDOW_MODES: [ShowWindowMode; 3] = [ShowWindowMode::NonOwned, ShowWindowMode::Owned, ShowWindowMode::Modal];
const WINDOW_STARTUP_LOCATIONS: [WindowStartupLocation; 3] =
    [WindowStartupLocation::Manual, WindowStartupLocation::CenterScreen, WindowStartupLocation::CenterOwner];
const WINDOW_STATES: [WindowState; 4] =
    [WindowState::Normal, WindowState::Minimized, WindowState::Maximized, WindowState::FullScreen];

/// `Enum.Parse<WindowState>(text)`.
pub fn parse_window_state(text: &str) -> WindowState {
    WINDOW_STATES
        .into_iter()
        .find(|state| format!("{state:?}") == text)
        .unwrap_or_else(|| panic!("Requested value '{text}' was not found."))
}

/// The options of `OpenWindow`, with upstream's defaults.
#[derive(Clone, Copy)]
pub struct OpenWindow {
    pub size: Option<Size>,
    pub mode: ShowWindowMode,
    pub location: WindowStartupLocation,
    pub state: WindowState,
    pub can_resize: bool,
    pub extend_client_area: bool,
    pub can_minimize: bool,
    pub can_maximize: bool,
}

impl Default for OpenWindow {
    fn default() -> Self {
        OpenWindow {
            size: None,
            mode: ShowWindowMode::NonOwned,
            location: WindowStartupLocation::Manual,
            state: WindowState::Normal,
            can_resize: true,
            extend_client_area: false,
            can_minimize: true,
            can_maximize: true,
        }
    }
}

/// `WindowInfo`.
#[derive(Clone, Debug)]
struct WindowInfo {
    client_size: Size,
    frame_size: Size,
    position: PixelPoint,
    owner_rect: Option<PixelRect>,
    screen_rect: PixelRect,
    scaling: f64,
    window_state: WindowState,
}

pub struct WindowTests {
    base: TestBase,
}

impl TestClass for WindowTests {}

/// `Assert.Equal(expected, actual, tolerance)`.
fn assert_within(expected: i32, actual: i32, tolerance: f64) {
    assert!(
        (f64::from(expected) - f64::from(actual)).abs() <= tolerance,
        "Assert.Equal() Failure: Values are not within tolerance {tolerance}: expected {expected}, actual {actual}"
    );
}

fn assert_close_enough(expected: PixelPoint, actual: PixelPoint) {
    if cfg!(windows) {
        // On win32, accurate frame information cannot be obtained until a window is shown but
        // WindowStartupLocation needs to be calculated before the window is shown, meaning that
        // the position of a centered window can be off by a bit. From initial testing, looks
        // like this shouldn't be more than 10 pixels.
        assert_within(expected.x, actual.x, 10.0);
        assert_within(expected.y, actual.y, 10.0);
    } else if cfg!(target_os = "macos") {
        assert_within(expected.x, actual.x, 15.0);
        assert_within(expected.y, actual.y, 15.0);
    } else {
        assert_eq!(expected, actual);
    }
}

fn size_text(size: Option<Size>) -> String {
    size.map_or("null".to_owned(), |s| format!("{}, {}", s.width, s.height))
}

impl WindowTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        WindowTests { base: TestBase::new(fixture, "Window") }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn startup_location(
        &self,
        size: Option<Size>,
        mode: ShowWindowMode,
        location: WindowStartupLocation,
        can_resize: bool,
    ) {
        let _window = self.open_window(OpenWindow { size, mode, location, can_resize, ..Default::default() });
        let info = self.get_window_info();

        if let Some(size) = size {
            assert_eq!(size, info.client_size);
        }

        assert!(info.frame_size.width >= info.client_size.width, "Expected frame width >= client width.");
        assert!(info.frame_size.height > info.client_size.height, "Expected frame height > client height.");

        let frame_rect = PixelRect::from_position_size(info.position, PixelSize::from_size(info.frame_size, info.scaling));

        match location {
            WindowStartupLocation::CenterScreen => {
                let expected = info.screen_rect.center_rect(frame_rect);
                assert_close_enough(expected.position(), frame_rect.position());
            }
            WindowStartupLocation::CenterOwner => {
                let owner_rect = info.owner_rect.expect("the rectangle of the owner");
                let expected = owner_rect.center_rect(frame_rect);
                assert_close_enough(expected.position(), frame_rect.position());
            }
            WindowStartupLocation::Manual => {}
        }
    }

    pub fn window_state(&self, size: Option<Size>, mode: ShowWindowMode, state: WindowState, can_resize: bool) {
        let _window = self.open_window(OpenWindow { size, mode, state, can_resize, ..Default::default() });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let info = self.get_window_info();

            assert_eq!(state, info.window_state);

            match state {
                WindowState::Normal => {
                    assert!(info.frame_size.width * info.scaling < f64::from(info.screen_rect.size().width));
                    assert!(info.frame_size.height * info.scaling < f64::from(info.screen_rect.size().height));
                }
                WindowState::Maximized | WindowState::FullScreen => {
                    assert!(info.frame_size.width * info.scaling >= f64::from(info.screen_rect.size().width));
                    assert!(info.frame_size.height * info.scaling >= f64::from(info.screen_rect.size().height));
                }
                WindowState::Minimized => {}
            }
        }));

        // finally
        if state == WindowState::FullScreen {
            let cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.session().find_element_by_accessibility_id("CurrentWindowState").send_click();
                self.session().find_element_by_accessibility_id("WindowStateNormal").send_click();

                // Wait for animations to run.
                if cfg!(target_os = "macos") {
                    std::thread::sleep(Duration::from_millis(1000));
                }
            }));
            // Ignore errors in cleanup
            let _ = cleanup;
        }
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    pub fn on_windows_docked_windows_retain_size_position_when_restored(&self) {
        let _window = self.open_window(OpenWindow {
            size: Some(Size::new(400.0, 400.0)),
            mode: ShowWindowMode::NonOwned,
            location: WindowStartupLocation::Manual,
            ..Default::default()
        });
        let window_state = self.session().find_element_by_accessibility_id("CurrentWindowState");

        assert_eq!("Normal", window_state.get_combo_box_value());

        let _window_element = self.session().find_elements(By::XPath("//Window".to_owned())).into_iter().next();

        Actions::new(self.session()).key_down(Keys::META).send_keys(Keys::LEFT).key_up(Keys::META).perform();

        let original = self.get_window_info();

        window_state.click();
        self.session().find_element_by_name("Minimized").send_click();

        Actions::new(self.session()).key_down(Keys::ALT).send_keys(Keys::TAB).key_up(Keys::ALT).perform();

        let current = self.get_window_info();

        assert_eq!(original.position, current.position);
        assert_eq!(original.frame_size, current.frame_size);
    }

    pub fn showing_window_with_size_larger_than_screen_measures_content_with_working_area(&self) {
        let _window = self.open_window(OpenWindow {
            size: Some(Size::new(4000.0, 2200.0)),
            mode: ShowWindowMode::NonOwned,
            location: WindowStartupLocation::Manual,
            ..Default::default()
        });
        let screen_rect_text_box = self.session().find_element_by_accessibility_id("CurrentClientSize");
        let measured_with_text_block = self.session().find_element_by_accessibility_id("CurrentMeasuredWithText");

        let measured_with_string = measured_with_text_block.text();
        let working_area_string = screen_rect_text_box.text();

        let working_area = Size::parse(&working_area_string).expect("the working area");
        let measured_with = Size::parse(&measured_with_string).expect("the size the content was measured with");

        assert_eq!(working_area, measured_with);
    }

    pub fn show_mode(&self, mode: ShowWindowMode) {
        let _window = self.open_window(OpenWindow { mode, location: WindowStartupLocation::Manual, ..Default::default() });
        let window_state = self.session().find_element_by_accessibility_id("CurrentWindowState");
        let original = self.get_window_info();

        assert_eq!("Normal", window_state.get_combo_box_value());

        window_state.click();
        self.session().find_element_by_accessibility_id("WindowStateMaximized").send_click();
        assert_eq!("Maximized", window_state.get_combo_box_value());

        window_state.click();
        self.session().find_element_by_accessibility_id("WindowStateNormal").send_click();

        let mut current = self.get_window_info();
        assert_eq!(original.position, current.position);
        assert_eq!(original.frame_size, current.frame_size);

        // On macOS, only non-owned windows can go fullscreen.
        if !cfg!(target_os = "macos") || mode == ShowWindowMode::NonOwned {
            window_state.click();
            self.session().find_element_by_accessibility_id("WindowStateFullScreen").send_click();
            assert_eq!("FullScreen", window_state.get_combo_box_value());

            current = self.get_window_info();
            let client_size = PixelSize::from_size(current.client_size, current.scaling);
            assert!(client_size.width >= current.screen_rect.width);
            assert!(client_size.height >= current.screen_rect.height);

            window_state.send_click();

            self.session().find_element_by_accessibility_id("WindowStateNormal").send_click();

            current = self.get_window_info();
            assert_eq!(original.position, current.position);
            assert_eq!(original.frame_size, current.frame_size);
        }
    }

    pub fn extended_client_window_shows_with_requested_size(&self) {
        let client_size = Size::new(400.0, 400.0);
        let _window = self.open_window(OpenWindow {
            size: Some(client_size),
            mode: ShowWindowMode::NonOwned,
            location: WindowStartupLocation::CenterScreen,
            extend_client_area: true,
            ..Default::default()
        });
        let _window_state = self.session().find_element_by_accessibility_id("CurrentWindowState");
        let current = self.get_window_info();

        assert_eq!(current.client_size, client_size);
    }

    pub fn transparent_window(&self) {
        let show_transparent_window = self.session().find_element_by_accessibility_id("ShowTransparentWindow");
        show_transparent_window.click();
        std::thread::sleep(Duration::from_millis(1000));

        let window = self.session().find_element_by_accessibility_id("TransparentWindow");
        let screenshot = window.get_screenshot();

        window.click();

        let top_left_color = screenshot.pixel(10, 10);
        let center_color = screenshot.pixel(screenshot.width / 2, screenshot.height / 2);

        assert_eq!((0, 128, 0), top_left_color);
        assert_eq!((255, 0, 0), center_color);
    }

    pub fn transparent_popup(&self) {
        let show_transparent_window = self.session().find_element_by_accessibility_id("ShowTransparentPopup");
        show_transparent_window.click();
        std::thread::sleep(Duration::from_millis(1000));

        let window = self.session().find_element_by_accessibility_id("TransparentPopupBackground");
        let container = window.find_element_by_accessibility_id("PopupContainer");
        let screenshot = container.get_screenshot();

        window.click();

        let top_left_color = screenshot.pixel(10, 10);
        let center_color = screenshot.pixel(screenshot.width / 2, screenshot.height / 2);

        assert_eq!((0, 128, 0), top_left_color);
        assert_eq!((255, 0, 0), center_color);
    }

    pub fn owned_window_should_appear_above_topmost_owner(&self) {
        let show_topmost_window = self.session().find_element_by_accessibility_id("ShowTopmostWindow");
        let _window = show_topmost_window.open_window_with_click();
        std::thread::sleep(Duration::from_millis(1000));
        let owner_window = self.session().get_window_by_id("OwnerWindow");
        let owned_window = self.session().get_window_by_id("OwnedWindow");

        fn get_position(window: &Element) -> PixelPoint {
            PixelPoint::parse(&window.find_element_by_accessibility_id("CurrentPosition").text()).expect("a position")
        }

        let owner_position = get_position(&owner_window);
        let owned_position = get_position(&owned_window);

        // Owned Window moves
        let move_button = owned_window.find_element_by_accessibility_id("MoveButton");
        move_button.click();
        std::thread::sleep(Duration::from_millis(1000));

        assert_eq!(get_position(&owner_window), owner_position);
        assert_ne!(get_position(&owned_window), owned_position);
    }

    pub fn window_has_disabled_maximize_button_when_can_resize_is_false(&self, mode: ShowWindowMode, extend_client_area: bool) {
        let _window = self.open_window(OpenWindow {
            mode,
            location: WindowStartupLocation::Manual,
            can_resize: false,
            extend_client_area,
            ..Default::default()
        });
        let secondary_window = self.session().get_window_by_id("SecondaryWindow");

        let maximize_button = if cfg!(windows) {
            if extend_client_area {
                secondary_window.find_element_by_xpath("//Button[@Name='Maximize']")
            } else {
                secondary_window.find_element_by_xpath("//TitleBar/Button[2]")
            }
        } else if mode == ShowWindowMode::NonOwned {
            secondary_window.find_element_by_accessibility_id("_XCUI:FullScreenWindow")
        } else {
            secondary_window.find_element_by_accessibility_id("_XCUI:ZoomWindow")
        };

        assert!(!maximize_button.enabled());
    }

    pub fn window_minimize_button_enabled_matches_can_minimize(&self, can_minimize: bool) {
        let _window = self.open_window(OpenWindow { can_minimize, ..Default::default() });
        let secondary_window = self.session().get_window_by_id("SecondaryWindow");
        let window_chrome = secondary_window.get_system_chrome_buttons();

        let minimize = window_chrome.minimize.expect("the minimize button");
        assert_eq!(can_minimize, minimize.enabled());
    }

    pub fn window_maximize_button_enabled_matches_can_maximize(&self, can_maximize: bool) {
        let _window = self.open_window(OpenWindow { can_maximize, ..Default::default() });
        let secondary_window = self.session().get_window_by_id("SecondaryWindow");
        let window_chrome = secondary_window.get_system_chrome_buttons();

        let maximize_button = window_chrome.full_screen.or(window_chrome.maximize).expect("the maximize button");
        assert_eq!(can_maximize, maximize_button.enabled());
    }

    pub fn changing_system_decorations_should_not_change_frame_size_and_position(&self) {
        let _window = self.open_window(OpenWindow {
            mode: ShowWindowMode::NonOwned,
            location: WindowStartupLocation::Manual,
            ..Default::default()
        });
        let info = self.get_window_info();

        self.session().find_element_by_accessibility_id("CurrentSystemDecorations").click();
        self.session().find_element_by_accessibility_id("SystemDecorationsNone").send_click();
        let mut updated_info = self.get_window_info();
        assert_eq!(info.frame_size, updated_info.frame_size);
        assert_eq!(info.position, updated_info.position);

        self.session().find_element_by_accessibility_id("CurrentSystemDecorations").click();
        self.session().find_element_by_accessibility_id("SystemDecorationsFull").send_click();
        updated_info = self.get_window_info();
        assert_eq!(info.frame_size, updated_info.frame_size);
        assert_eq!(info.position, updated_info.position);
    }

    pub fn changing_window_state_should_not_change_frame_size_and_position(&self) {
        let _window = self.open_window(OpenWindow::default());
        let info = self.get_window_info();

        self.session().find_element_by_accessibility_id("CurrentWindowState").send_click();
        self.session().find_element_by_accessibility_id("WindowStateMaximized").send_click();
        self.session().find_element_by_accessibility_id("CurrentWindowState").send_click();
        self.session().find_element_by_accessibility_id("WindowStateNormal").send_click();
        let updated_info = self.get_window_info();
        assert_eq!(info.frame_size, updated_info.frame_size);
        assert_eq!(info.position, updated_info.position);
    }

    pub fn changing_size_should_not_change_position(&self) {
        let _window = self.open_window(OpenWindow::default());
        let info = self.get_window_info();

        self.session().find_element_by_accessibility_id("AddToWidth").send_click();
        self.session().find_element_by_accessibility_id("AddToHeight").send_click();

        let updated_info = self.get_window_info();
        assert_eq!(info.position, updated_info.position);
        assert_eq!(
            Size::new(info.frame_size.width + 10.0, info.frame_size.height + 10.0),
            updated_info.frame_size
        );
    }

    /// `StartupLocationData()`.
    fn startup_location_data() -> Vec<(Option<Size>, ShowWindowMode, WindowStartupLocation, bool)> {
        let sizes = [None, Some(Size::new(400.0, 300.0))];
        let mut data = Vec::new();

        for size in sizes {
            for mode in SHOW_WINDOW_MODES {
                for location in WINDOW_STARTUP_LOCATIONS {
                    if !(location == WindowStartupLocation::CenterOwner && mode == ShowWindowMode::NonOwned) {
                        data.push((size, mode, location, true));
                        data.push((size, mode, location, false));
                    }
                }
            }
        }

        data
    }

    /// `WindowStateData()`.
    fn window_state_data() -> Vec<(Option<Size>, ShowWindowMode, WindowState, bool)> {
        let sizes = [None, Some(Size::new(400.0, 300.0))];
        let mut data = Vec::new();

        for size in sizes {
            for mode in SHOW_WINDOW_MODES {
                for state in WINDOW_STATES {
                    // Not sure how to handle testing minimized windows currently.
                    if state == WindowState::Minimized {
                        continue;
                    }

                    // Child/Modal windows cannot be fullscreen on macOS.
                    if cfg!(target_os = "macos") && state == WindowState::FullScreen && mode != ShowWindowMode::NonOwned {
                        continue;
                    }

                    data.push((size, mode, state, true));
                    data.push((size, mode, state, false));
                }
            }
        }

        data
    }

    fn open_window(&self, options: OpenWindow) -> Disposable {
        let session = self.session();
        let size_text_box = session.find_element_by_accessibility_id("ShowWindowSize");
        let mode_combo_box = session.find_element_by_accessibility_id("ShowWindowMode");
        let location_combo_box = session.find_element_by_accessibility_id("ShowWindowLocation");
        let state_combo_box = session.find_element_by_accessibility_id("ShowWindowState");
        let can_resize_check_box = session.find_element_by_accessibility_id("ShowWindowCanResize");
        let can_minimize_check_box = session.find_element_by_accessibility_id("ShowWindowCanMinimize");
        let can_maximize_check_box = session.find_element_by_accessibility_id("ShowWindowCanMaximize");
        let show_button = session.find_element_by_accessibility_id("ShowWindow");
        let extend_client_area_check_box =
            session.find_element_by_accessibility_id("ShowWindowExtendClientAreaToDecorationsHint");

        if let Some(size) = options.size {
            size_text_box.send_keys(&format!("{}, {}", size.width, size.height));
        }

        let mode = format!("{:?}", options.mode);
        if mode_combo_box.get_combo_box_value() != mode {
            mode_combo_box.click();
            session.find_element_by_name(&mode).send_click();
        }

        let location = format!("{:?}", options.location);
        if location_combo_box.get_combo_box_value() != location {
            location_combo_box.click();
            session.find_element_by_name(&location).send_click();
        }

        let state = format!("{:?}", options.state);
        if state_combo_box.get_combo_box_value() != state {
            state_combo_box.click();
            session.find_element_by_accessibility_id(&format!("ShowWindowState{state}")).send_click();
        }

        if can_resize_check_box.get_is_checked() != Some(options.can_resize) {
            can_resize_check_box.click();
        }

        if can_minimize_check_box.get_is_checked() != Some(options.can_minimize) {
            can_minimize_check_box.click();
        }

        if can_maximize_check_box.get_is_checked() != Some(options.can_maximize) {
            can_maximize_check_box.click();
        }

        if extend_client_area_check_box.get_is_checked() != Some(options.extend_client_area) {
            extend_client_area_check_box.click();
        }

        show_button.open_window_with_click()
    }

    fn get_window_info(&self) -> WindowInfo {
        let session = self.session();
        let text = |id: &str| -> Result<String, WebDriverError> { Ok(session.try_find_element_by_accessibility_id(id)?.text()) };
        let read_owner_rect = || -> Result<Option<PixelRect>, WebDriverError> {
            let text = text("CurrentOwnerRect")?;
            Ok(if !text.trim().is_empty() { Some(PixelRect::parse(&text).expect("the rectangle of the owner")) } else { None })
        };

        let mut retry = 0;

        loop {
            let result = (|| -> Result<WindowInfo, WebDriverError> {
                Ok(WindowInfo {
                    client_size: Size::parse(&text("CurrentClientSize")?).expect("the client size"),
                    frame_size: Size::parse(&text("CurrentFrameSize")?).expect("the frame size"),
                    position: PixelPoint::parse(&text("CurrentPosition")?).expect("the position"),
                    owner_rect: read_owner_rect()?,
                    screen_rect: PixelRect::parse(&text("CurrentScreenRect")?).expect("the rectangle of the screen"),
                    scaling: text("CurrentScaling")?.parse().expect("the scaling"),
                    window_state: parse_window_state(&text("CurrentWindowState")?),
                })
            })();
            match result {
                Ok(info) => return info,
                Err(WebDriverError::NoSuchElement(_)) if retry < 3 => {
                    retry += 1;
                    // MacOS sometimes seems to need a bit of time to get itself back in order after switching out
                    // of fullscreen.
                    std::thread::sleep(Duration::from_millis(1000));
                }
                Err(error) => panic!("{error}"),
            }
        }
    }
}

pub fn tests(tests: &mut Tests) {
    let mut class = tests.class("window_tests", Collection::Default, WindowTests::new);
    for (size, mode, location, can_resize) in WindowTests::startup_location_data() {
        class.fact(
            &format!("startup_location({}, {mode:?}, {location:?}, {can_resize})", size_text(size)),
            move |t: &WindowTests| t.startup_location(size, mode, location, can_resize),
        );
    }
    for (size, mode, state, can_resize) in WindowTests::window_state_data() {
        class.fact(
            &format!("window_state({}, {mode:?}, {state:?}, {can_resize})", size_text(size)),
            move |t: &WindowTests| t.window_state(size, mode, state, can_resize),
        );
    }
    class
        .platform_fact(
            "on_windows_docked_windows_retain_size_position_when_restored",
            TestPlatforms::WINDOWS,
            WindowTests::on_windows_docked_windows_retain_size_position_when_restored,
        )
        .fact(
            "showing_window_with_size_larger_than_screen_measures_content_with_working_area",
            WindowTests::showing_window_with_size_larger_than_screen_measures_content_with_working_area,
        );
    for mode in SHOW_WINDOW_MODES {
        class.fact(&format!("show_mode({mode:?})"), move |t: &WindowTests| t.show_mode(mode));
    }
    class
        .fact("extended_client_window_shows_with_requested_size", WindowTests::extended_client_window_shows_with_requested_size)
        .fact("transparent_window", WindowTests::transparent_window)
        .not_provided_on(TestPlatforms::MAC_OS, SCREENSHOT_NOT_PROVIDED_ON_MAC_OS)
        .fact("transparent_popup", WindowTests::transparent_popup)
        .not_provided_on(TestPlatforms::MAC_OS, SCREENSHOT_NOT_PROVIDED_ON_MAC_OS)
        .platform_fact(
            "owned_window_should_appear_above_topmost_owner",
            TestPlatforms::WINDOWS,
            WindowTests::owned_window_should_appear_above_topmost_owner,
        );
    for (mode, extend_client_area) in [
        (ShowWindowMode::NonOwned, true),
        (ShowWindowMode::Owned, true),
        (ShowWindowMode::Modal, true),
        (ShowWindowMode::NonOwned, false),
        (ShowWindowMode::Owned, false),
        (ShowWindowMode::Modal, false),
    ] {
        class.fact(
            &format!("window_has_disabled_maximize_button_when_can_resize_is_false({mode:?}, {extend_client_area})"),
            move |t: &WindowTests| t.window_has_disabled_maximize_button_when_can_resize_is_false(mode, extend_client_area),
        );
    }
    for value in [false, true] {
        class.fact(
            &format!("window_minimize_button_enabled_matches_can_minimize({value})"),
            move |t: &WindowTests| t.window_minimize_button_enabled_matches_can_minimize(value),
        );
    }
    for value in [false, true] {
        class.fact(
            &format!("window_maximize_button_enabled_matches_can_maximize({value})"),
            move |t: &WindowTests| t.window_maximize_button_enabled_matches_can_maximize(value),
        );
    }
    class
        .fact(
            "changing_system_decorations_should_not_change_frame_size_and_position",
            WindowTests::changing_system_decorations_should_not_change_frame_size_and_position,
        )
        .fact(
            "changing_window_state_should_not_change_frame_size_and_position",
            WindowTests::changing_window_state_should_not_change_frame_size_and_position,
        )
        .platform_fact(
            "changing_size_should_not_change_position",
            TestPlatforms::WINDOWS,
            WindowTests::changing_size_should_not_change_position,
        );
}
