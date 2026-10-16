//! Port of `ElementExtensions.cs`: what the tests read of elements and
//! windows, written once for both platforms.

use super::actions::Actions;
use super::keys::Keys;
use super::session::{By, Element, Session};
use std::time::Duration;

/// `WindowChrome`: the buttons of the title bar of a window, and the bar.
pub struct WindowChrome {
    pub close: Option<Element>,
    pub minimize: Option<Element>,
    pub maximize: Option<Element>,
    pub full_screen: Option<Element>,
    pub title_bar: Option<Element>,
}

impl WindowChrome {
    pub fn is_any_button_enabled(&self) -> bool {
        (self.title_bar.as_ref().is_none_or(|t| t.enabled()))
            && (self.close.as_ref().is_some_and(|b| b.enabled())
                || self.minimize.as_ref().is_some_and(|b| b.enabled())
                || self.maximize.as_ref().is_some_and(|b| b.enabled())
                || self.full_screen.as_ref().is_some_and(|b| b.enabled()))
    }

    pub fn title_bar_height(&self) -> i32 {
        self.title_bar.as_ref().map_or(-1, |t| t.size().height)
    }

    pub fn max_button_height(&self) -> i32 {
        let height = |button: &Option<Element>| button.as_ref().map_or(-1, |b| b.size().height);
        height(&self.close).max(height(&self.minimize)).max(height(&self.maximize).max(height(&self.full_screen)))
    }
}

/// Runs an action when it is dropped: the `IDisposable` that
/// `OpenWindowWithClick` returns. The action runs when the scope of the
/// value ends, also when the test fails; a failure of the action while a
/// failure of the test unwinds is reported and does not hide that failure.
pub struct Disposable(Option<Box<dyn FnOnce()>>);

impl Disposable {
    pub fn new(action: impl FnOnce() + 'static) -> Disposable {
        Disposable(Some(Box::new(action)))
    }

    pub fn empty() -> Disposable {
        Disposable(None)
    }

    /// `Dispose()`.
    pub fn dispose(mut self) {
        if let Some(action) = self.0.take() {
            action();
        }
    }
}

impl Drop for Disposable {
    fn drop(&mut self) {
        if let Some(action) = self.0.take() {
            if std::thread::panicking() {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).is_err() {
                    eprintln!("the clean-up of the test failed too");
                }
            } else {
                action();
            }
        }
    }
}

impl Element {
    /// `GetChildren()`: the elements of `*/*` below the element.
    pub fn get_children(&self) -> Vec<Element> {
        self.find_elements_by_xpath("*/*")
    }

    /// `GetSystemChromeButtons()`.
    pub fn get_system_chrome_buttons(&self) -> WindowChrome {
        if cfg!(target_os = "macos") {
            let close_button = self.find_elements_by_accessibility_id("_XCUI:CloseWindow").into_iter().next();
            let fullscreen_button = self.find_elements_by_accessibility_id("_XCUI:FullScreenWindow").into_iter().next();
            let minimize_button = self.find_elements_by_accessibility_id("_XCUI:MinimizeWindow").into_iter().next();
            let zoom_button = self.find_elements_by_accessibility_id("_XCUI:ZoomWindow").into_iter().next();
            return WindowChrome {
                close: close_button,
                minimize: minimize_button,
                maximize: zoom_button,
                full_screen: fullscreen_button,
                title_bar: None,
            };
        }

        if cfg!(windows) {
            let titlebar = self.find_elements_by_tag_name("TitleBar").into_iter().next();
            let close_button = titlebar.as_ref().map(|t| t.find_element_by_name("Close"));
            let minimize_button = titlebar.as_ref().map(|t| t.find_element_by_name("Minimize"));
            let maximize_button = titlebar.as_ref().map(|t| t.find_element_by_name("Maximize"));
            return WindowChrome {
                close: close_button,
                minimize: minimize_button,
                maximize: maximize_button,
                full_screen: None,
                title_bar: titlebar,
            };
        }

        panic!("GetChromeButtons not supported on this platform.");
    }

    /// `GetClientChromeButtons()`.
    pub fn get_client_chrome_buttons(&self) -> WindowChrome {
        let chrome = self.find_elements_by_accessibility_id("FerroWindowChrome").into_iter().next();
        let first = |id: &str| chrome.as_ref().and_then(|c| c.find_elements_by_accessibility_id(id).into_iter().next());
        let titlebar = first("FerroTitleBar");
        let close_button = first("Close");
        let minimize_button = first("Minimize");
        let maximize_button = first("Maximize");
        let fullscreen_button = first("Fullscreen");
        WindowChrome {
            close: close_button,
            minimize: minimize_button,
            maximize: maximize_button,
            full_screen: fullscreen_button,
            title_bar: titlebar,
        }
    }

    /// `GetComboBoxValue()`.
    pub fn get_combo_box_value(&self) -> String {
        if cfg!(windows) {
            self.text()
        } else {
            self.get_attribute("value").unwrap_or_default()
        }
    }

    /// `GetName()`.
    pub fn get_name(&self) -> String {
        get_attribute(self, "Name", "title").unwrap_or_default()
    }

    /// `GetIsChecked()`.
    pub fn get_is_checked(&self) -> Option<bool> {
        match get_attribute(self, "Toggle.ToggleState", "value").as_deref() {
            Some("0") => Some(false),
            Some("1") => Some(true),
            Some("2") => None,
            other => panic!("Unexpected IsChecked value: {other:?}."),
        }
    }

    /// `GetIsFocused()`.
    pub fn get_is_focused(&self) -> bool {
        if cfg!(windows) {
            self.get_attribute("HasKeyboardFocus").as_deref() == Some("True")
        } else {
            // https://stackoverflow.com/questions/71807788/check-if-element-is-focused-in-appium
            panic!("Couldn't work out how to check if an element is focused on mac.");
        }
    }

    /// `OpenWindowWithClick(delay)`: clicks a button which is expected to open
    /// a new window; the value closes the window when it is dropped.
    pub fn open_window_with_click(&self) -> Disposable {
        self.open_window_with_click_delay(None)
    }

    pub fn open_window_with_click_delay(&self, delay: Option<Duration>) -> Disposable {
        let session = self.session().clone();

        if cfg!(windows) {
            let old_handle = session.current_window_handle();
            let old_handles = session.window_handles();
            let old_child_windows = session.find_elements(By::XPath("//Window".to_owned()));

            self.click();

            if let Some(delay) = delay {
                std::thread::sleep(delay);
            }

            let mut new_handle: Option<String> = None;
            let mut new_child_window: Option<Element> = None;

            for _ in 0..10 {
                let added: Vec<String> =
                    session.window_handles().into_iter().filter(|h| !old_handles.contains(h)).collect();
                new_handle = single_or_default(added);
                if new_handle.is_some() {
                    break;
                }

                // If a new window handle hasn't been added to the session then it's likely
                // that a child window was opened. These don't appear in session.WindowHandles
                // so we have to use an XPath query to get hold of it.
                let new_child_windows = session.find_elements(By::XPath("//Window".to_owned()));
                let added: Vec<Element> =
                    new_child_windows.into_iter().filter(|w| !old_child_windows.contains(w)).collect();
                new_child_window = single_or_default(added);
                if new_child_window.is_some() {
                    break;
                }

                std::thread::sleep(Duration::from_millis(100));
            }

            if let Some(new_handle) = new_handle {
                // A new top-level window was opened. We need to switch to it.
                session.switch_to_window(&new_handle);

                return Disposable::new(move || {
                    session.close();
                    session.switch_to_window(&old_handle);
                });
            }

            if let Some(new_child_window) = new_child_window {
                return Disposable::new(move || {
                    new_child_window.send_keys(&format!("{}{}{}", Keys::ALT, Keys::F4, Keys::ALT));
                });
            }

            panic!("Could not find the newly opened window");
        } else {
            let windows_path = "/XCUIElementTypeApplication/XCUIElementTypeWindow";
            let old_windows = session.find_elements_by_xpath(windows_path);
            let old_window_titles: Vec<String> = old_windows.iter().map(|x| x.text()).collect();

            self.click();

            // Wait for animations to run.
            std::thread::sleep(Duration::from_millis(1000));

            let new_windows = session.find_elements_by_xpath(windows_path);

            // Try to find the new window by looking for a window with a title that didn't exist before the button
            // was clicked. Sometimes it seems that when a window becomes fullscreen, all other windows in the
            // application lose their titles, so filter out windows with no title (this may have started happening
            // with macOS 13.1?)
            let mut new_window_titles: Vec<String> =
                new_windows.iter().map(|x| x.text()).filter(|text| !text.is_empty()).collect();
            new_window_titles.dedup();
            let added: Vec<String> = new_window_titles.into_iter().filter(|t| !old_window_titles.contains(t)).collect();
            assert_eq!(added.len(), 1, "expected a single new window, found {added:?}");
            let new_window_title = added.into_iter().next().unwrap();

            Disposable::new(move || {
                // TODO: We should be able to use Cmd+W here but the apps of the framework don't seem to have this
                // shortcut set up by default.
                let windows = session.find_elements_by_xpath(windows_path);
                let _text: Vec<String> = windows.iter().map(|x| x.text()).collect();
                let new_window = session
                    .find_elements_by_xpath(windows_path)
                    .into_iter()
                    .find(|x| x.text() == new_window_title)
                    .expect("the window the click opened");
                let close = new_window.find_element_by_accessibility_id("_XCUI:CloseWindow");
                close.click();
                std::thread::sleep(Duration::from_millis(1000));
            })
        }
    }

    /// `SendClick()`: the click of a mouse at the centre of the element.
    ///
    /// The Click() method seems to correspond to accessibilityPerformPress on macOS but certain controls
    /// such as list items don't support this action, so instead simulate a physical click as VoiceOver
    /// does. On Windows, Click() seems to fail with the WindowState checkbox for some reason.
    pub fn send_click(&self) {
        Actions::new(self.session()).move_to_element(self).click().perform();
    }

    /// `SendDoubleClick()`.
    pub fn send_double_click(&self) {
        Actions::new(self.session()).move_to_element(self).double_click().perform();
    }

    /// `MovePointerOver()`.
    pub fn move_pointer_over(&self) {
        Actions::new(self.session()).move_to_element(self).perform();
    }
}

impl Session {
    /// `GetCurrentSingleWindow()`.
    pub fn get_current_single_window(&self) -> Element {
        if cfg!(target_os = "macos") {
            // The accessibility tree of the framework currently exposes two nested Window elements, this is a
            // bug and should be fixed but in the meantime use the `parent::' selector to return the parent "real"
            // window.
            self.find_element_by_xpath("XCUIElementTypeWindow//*/parent::XCUIElementTypeWindow")
        } else {
            self.find_element_by_xpath("//Window")
        }
    }

    /// `GetWindowById(identifier)`.
    pub fn get_window_by_id(&self, identifier: &str) -> Element {
        if cfg!(target_os = "macos") {
            self.find_element_by_xpath(&format!("XCUIElementTypeWindow[@identifier='{identifier}']"))
        } else {
            self.find_element_by_xpath(&format!("//Window[@AutomationId='{identifier}']"))
        }
    }
}

/// `GetAttribute(element, windows, macOS)`.
pub fn get_attribute(element: &Element, windows: &str, mac_os: &str) -> Option<String> {
    element.get_attribute(if cfg!(windows) { windows } else { mac_os })
}

/// `SingleOrDefault()`: the one item, none for none; more than one fails.
fn single_or_default<T>(items: Vec<T>) -> Option<T> {
    assert!(items.len() <= 1, "Sequence contains more than one element");
    items.into_iter().next()
}
