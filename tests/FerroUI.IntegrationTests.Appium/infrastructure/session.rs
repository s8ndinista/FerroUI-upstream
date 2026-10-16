//! What upstream's tests use of an Appium session and of its elements
//! (`AppiumDriver`, `AppiumWebElement`, the `FindElementBy*` extensions of
//! `AppiumDriverEx.cs`), over the automation backend of the platform.
//!
//! The semantics are the ones of the drivers upstream runs against, where the
//! tests depend on them (docs/porting/integration-tests.md, section 3):
//!
//! - A search from the session covers the current window and the elements
//!   below it on Windows (WinAppDriver), the application and everything below
//!   it on macOS (the Mac2 driver). A search from an element covers the
//!   elements below it. A search for one element waits up to
//!   [`IMPLICIT_WAIT`] for it to appear.
//! - The accessibility id is the `AutomationId` on Windows and the identifier
//!   on macOS; the name is the `Name` on Windows and on macOS the first
//!   non-empty of the identifier, the title and the label.
//! - The text is, on Windows, the value of the value pattern, else the value
//!   of the range value pattern, else the name; on macOS the first non-empty
//!   of the value, the title and the label.

use super::backend::{Backend, NativeElement};
use super::keys::{key_of, Key, NamedKey};
use super::xpath;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How long a search for one element waits for it to appear.
pub const IMPLICIT_WAIT: Duration = Duration::from_millis(1000);

static DUMP_TREE: AtomicBool = AtomicBool::new(false);

/// Asks a search for one element that does not find it to print the tree it searched
/// (`--dump-tree` of the runner).
pub fn set_dump_tree(enabled: bool) {
    DUMP_TREE.store(enabled, Ordering::Relaxed);
}

fn dump_tree_enabled() -> bool {
    DUMP_TREE.load(Ordering::Relaxed) || std::env::var("FERROUI_AUTOMATION_DUMP_TREE").as_deref() == Ok("1")
}

/// What a command of the session failed with: the exceptions upstream's tests
/// catch are `WebDriverException` and its `NoSuchElementException`.
#[derive(Clone, Debug, PartialEq)]
pub enum WebDriverError {
    NoSuchElement(String),
    NoSuchWindow(String),
    Other(String),
}

impl fmt::Display for WebDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebDriverError::NoSuchElement(what) => write!(f, "no such element: {what}"),
            WebDriverError::NoSuchWindow(what) => write!(f, "no such window: {what}"),
            WebDriverError::Other(what) => f.write_str(what),
        }
    }
}

/// How an element is looked for (`MobileBy` and `By` of upstream).
#[derive(Clone, Debug, PartialEq)]
pub enum By {
    AccessibilityId(String),
    Name(String),
    XPath(String),
    ClassName(String),
    TagName(String),
}

/// Why the backend of macOS takes no pictures of the screen (`GetScreenshot`).
pub const SCREENSHOT_NOT_PROVIDED_ON_MAC_OS: &str = "the backend of macOS takes no pictures of the screen: that \
    needs the Screen Recording permission and ScreenCaptureKit, and the runs of this port do not capture the screen \
    of the host (docs/porting/integration-tests.md, section 5)";

/// The mouse buttons of the input of the tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

/// The size of an element (`System.Drawing.Size`): in pixels on Windows, in
/// points on macOS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElementSize {
    pub width: i32,
    pub height: i32,
}

/// The bounds of an element on the screen (`System.Drawing.Rectangle`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElementRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ElementRect {
    pub fn left(&self) -> f64 {
        self.x
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// A picture of the screen (`Screenshot` of upstream), in the pixels of the
/// display.
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// Red, green, blue, alpha, row by row from the top.
    pub rgba: Vec<u8>,
}

impl Screenshot {
    /// The red, green and blue of a pixel (`SKBitmap.GetPixel`).
    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let index = ((y * self.width + x) * 4) as usize;
        (self.rgba[index], self.rgba[index + 1], self.rgba[index + 2])
    }
}

struct SessionState {
    backend: Backend,
    /// The window of the session on Windows (`CurrentWindowHandle`); a
    /// session on macOS has the application as its root.
    current_window: RefCell<Option<String>>,
    /// Where the pointer of the actions is.
    pointer: Cell<(f64, f64)>,
    /// The mouse buttons the actions hold down.
    buttons: RefCell<Vec<MouseButton>>,
    /// The modifier keys the actions hold down.
    modifiers: RefCell<Vec<NamedKey>>,
}

/// A session of the application (`AppiumDriver`).
#[derive(Clone)]
pub struct Session(Rc<SessionState>);

/// An element of the application (`AppiumWebElement`).
#[derive(Clone)]
pub struct Element {
    session: Session,
    native: NativeElement,
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend = self.session.backend();
        write!(
            f,
            "{} {:?} ({:?})",
            backend.tag_name(&self.native),
            backend.automation_id(&self.native),
            backend.name(&self.native)
        )
    }
}

/// The tree of the backend, as the XPath evaluation reads it.
struct XPathTree<'a>(&'a Backend);

impl xpath::Tree for XPathTree<'_> {
    type Node = NativeElement;

    fn children(&self, node: &NativeElement) -> Vec<NativeElement> {
        self.0.children(node)
    }

    fn tag_name(&self, node: &NativeElement) -> String {
        self.0.tag_name(node)
    }

    fn attribute(&self, node: &NativeElement, name: &str) -> Option<String> {
        self.0.attribute(node, name)
    }
}

/// Where a search starts.
enum Scope<'a> {
    /// The root of the session, which the search includes.
    Session,
    /// The elements below an element.
    Element(&'a NativeElement),
}

impl Session {
    pub fn new(backend: Backend) -> Session {
        let current_window = backend.window_handles().into_iter().next();
        Session(Rc::new(SessionState {
            backend,
            current_window: RefCell::new(current_window),
            pointer: Cell::new((0.0, 0.0)),
            buttons: RefCell::new(Vec::new()),
            modifiers: RefCell::new(Vec::new()),
        }))
    }

    pub(crate) fn backend(&self) -> &Backend {
        &self.0.backend
    }

    fn wrap(&self, native: NativeElement) -> Element {
        Element { session: self.clone(), native }
    }

    /// The root element of a search from the session.
    fn root(&self) -> Result<NativeElement, WebDriverError> {
        let handle = self.0.current_window.borrow().clone();
        self.backend().session_root(handle.as_deref())
    }

    fn find_all(&self, scope: Scope<'_>, by: &By) -> Result<Vec<NativeElement>, WebDriverError> {
        let backend = self.backend();
        let (root, include_root) = match scope {
            Scope::Session => (self.root()?, true),
            Scope::Element(element) => (element.clone(), false),
        };
        match by {
            By::XPath(expression) => {
                xpath::evaluate(&XPathTree(backend), &root, expression).map_err(WebDriverError::Other)
            }
            _ => backend.find_all(&root, include_root, by).map_err(|error| WebDriverError::Other(format!("{by:?}: {error}"))),
        }
    }

    fn find_one(&self, scope: Scope<'_>, by: &By) -> Result<NativeElement, WebDriverError> {
        let deadline = Instant::now() + IMPLICIT_WAIT;
        loop {
            let found = match &scope {
                Scope::Session => self.find_all(Scope::Session, by),
                Scope::Element(element) => self.find_all(Scope::Element(element), by),
            };
            match found {
                Ok(found) => {
                    if let Some(first) = found.into_iter().next() {
                        return Ok(first);
                    }
                    if Instant::now() >= deadline {
                        self.dump_tree(&scope, by);
                        return Err(WebDriverError::NoSuchElement(format!("{by:?}")));
                    }
                }
                // The search itself failed (the session's window is gone, or the provider of
                // the application answered an error): reported once the wait is over.
                Err(error) => {
                    if Instant::now() >= deadline {
                        self.dump_tree(&scope, by);
                        return Err(error);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// With `--dump-tree` (or `FERROUI_AUTOMATION_DUMP_TREE=1`), prints the tree a search for
    /// one element did not find it in: a diagnostic of the runs, no command of the drivers.
    fn dump_tree(&self, scope: &Scope<'_>, by: &By) {
        if !dump_tree_enabled() {
            return;
        }
        let backend = self.backend();
        let root = match scope {
            Scope::Session => match self.root() {
                Ok(root) => root,
                Err(error) => {
                    eprintln!("-- the tree of the search for {by:?}: {error}");
                    return;
                }
            },
            Scope::Element(element) => (*element).clone(),
        };
        eprintln!("-- the tree of the search for {by:?}:");
        if let Some(text) = backend.search_diagnostic(&root) {
            eprint!("{text}");
        }
        let mut lines = 0usize;
        let mut stack: Vec<(NativeElement, usize)> = vec![(root, 0)];
        while let Some((element, depth)) = stack.pop() {
            lines += 1;
            if lines > 600 {
                eprintln!("   ...");
                break;
            }
            let rect = backend.rect(&element);
            eprintln!(
                "   {:indent$}{} id={:?} name={:?} rect=({}, {}, {}, {})",
                "",
                backend.tag_name(&element),
                backend.automation_id(&element),
                backend.name(&element),
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                indent = depth * 2
            );
            if depth < 40 {
                let children = backend.children(&element);
                stack.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
            }
        }
    }

    pub fn try_find_element(&self, by: By) -> Result<Element, WebDriverError> {
        self.find_one(Scope::Session, &by).map(|native| self.wrap(native))
    }

    pub fn find_element(&self, by: By) -> Element {
        self.try_find_element(by).unwrap_or_else(|error| panic!("{error}"))
    }

    pub fn find_elements(&self, by: By) -> Vec<Element> {
        let found = self.find_all(Scope::Session, &by).unwrap_or_else(|error| panic!("{error}"));
        found.into_iter().map(|native| self.wrap(native)).collect()
    }

    pub fn try_find_element_by_accessibility_id(&self, id: &str) -> Result<Element, WebDriverError> {
        self.try_find_element(By::AccessibilityId(id.to_owned()))
    }

    pub fn find_element_by_accessibility_id(&self, id: &str) -> Element {
        self.find_element(By::AccessibilityId(id.to_owned()))
    }

    pub fn find_elements_by_accessibility_id(&self, id: &str) -> Vec<Element> {
        self.find_elements(By::AccessibilityId(id.to_owned()))
    }

    pub fn find_element_by_name(&self, name: &str) -> Element {
        self.find_element(By::Name(name.to_owned()))
    }

    pub fn find_elements_by_name(&self, name: &str) -> Vec<Element> {
        self.find_elements(By::Name(name.to_owned()))
    }

    pub fn find_element_by_xpath(&self, expression: &str) -> Element {
        self.find_element(By::XPath(expression.to_owned()))
    }

    pub fn find_elements_by_xpath(&self, expression: &str) -> Vec<Element> {
        self.find_elements(By::XPath(expression.to_owned()))
    }

    pub fn find_elements_by_class_name(&self, class_name: &str) -> Vec<Element> {
        self.find_elements(By::ClassName(class_name.to_owned()))
    }

    /// `CurrentWindowHandle`.
    pub fn current_window_handle(&self) -> String {
        self.0.current_window.borrow().clone().unwrap_or_default()
    }

    /// `WindowHandles`: the top-level windows of the application.
    pub fn window_handles(&self) -> Vec<String> {
        self.backend().window_handles()
    }

    /// `SwitchTo().Window(handle)`: the window becomes the root of the
    /// searches and is brought to the front.
    pub fn switch_to_window(&self, handle: &str) {
        *self.0.current_window.borrow_mut() = Some(handle.to_owned());
        self.backend().activate_window(handle);
        self.backend().settle();
    }

    /// `Close()`: closes the current window.
    pub fn close(&self) {
        if let Some(handle) = self.0.current_window.borrow_mut().take() {
            self.backend().close_window(&handle);
            self.backend().settle();
        }
    }

    // ---- input -------------------------------------------------------------------------------

    pub(crate) fn pointer(&self) -> (f64, f64) {
        self.0.pointer.get()
    }

    /// Moves the pointer, in `duration`, in steps, so that a drag sees the
    /// moves between.
    pub(crate) fn move_pointer(&self, x: f64, y: f64, duration: Duration) {
        let (from_x, from_y) = self.0.pointer.get();
        let steps = (duration.as_millis() / 16).max(1) as u32;
        for step in 1..=steps {
            let t = f64::from(step) / f64::from(steps);
            self.backend().mouse_move(from_x + (x - from_x) * t, from_y + (y - from_y) * t);
            if steps > 1 {
                std::thread::sleep(duration / steps);
            }
        }
        self.0.pointer.set((x, y));
    }

    pub(crate) fn button_down(&self, button: MouseButton, click_count: u32) {
        let (x, y) = self.pointer();
        self.backend().mouse_button(button, true, x, y, click_count);
        self.0.buttons.borrow_mut().push(button);
    }

    pub(crate) fn button_up(&self, button: MouseButton, click_count: u32) {
        let (x, y) = self.pointer();
        self.backend().mouse_button(button, false, x, y, click_count);
        self.0.buttons.borrow_mut().retain(|b| *b != button);
    }

    pub(crate) fn any_button_down(&self) -> bool {
        !self.0.buttons.borrow().is_empty()
    }

    pub(crate) fn click_at(&self, x: f64, y: f64, button: MouseButton) {
        self.move_pointer(x, y, Duration::ZERO);
        self.button_down(button, 1);
        std::thread::sleep(Duration::from_millis(30));
        self.button_up(button, 1);
    }

    pub(crate) fn double_click_here(&self) {
        self.button_down(MouseButton::Left, 1);
        std::thread::sleep(Duration::from_millis(30));
        self.button_up(MouseButton::Left, 1);
        std::thread::sleep(Duration::from_millis(60));
        self.button_down(MouseButton::Left, 2);
        std::thread::sleep(Duration::from_millis(30));
        self.button_up(MouseButton::Left, 2);
    }

    pub(crate) fn key_down(&self, key: NamedKey) {
        self.backend().key(Key::Named(key), true);
        if key.is_modifier() {
            self.0.modifiers.borrow_mut().push(key);
        }
    }

    pub(crate) fn key_up(&self, key: NamedKey) {
        self.backend().key(Key::Named(key), false);
        self.0.modifiers.borrow_mut().retain(|k| *k != key);
    }

    /// Types a string of keys as the protocol does: a modifier is pressed by
    /// one occurrence and released by the next, `Null` releases every
    /// modifier, any other key is pressed and released. With
    /// `release_modifiers` the modifiers still down at the end are released
    /// (the command of an element); the actions keep them down until a key up.
    pub(crate) fn type_keys(&self, text: &str, release_modifiers: bool) {
        for c in text.chars() {
            match key_of(c) {
                Key::Named(NamedKey::Null) => self.release_modifiers(),
                Key::Named(key) if key.is_modifier() => {
                    if self.0.modifiers.borrow().contains(&key) {
                        self.key_up(key);
                    } else {
                        self.key_down(key);
                    }
                }
                key => {
                    self.backend().key(key, true);
                    self.backend().key(key, false);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if release_modifiers {
            self.release_modifiers();
        }
    }

    /// Releases every mouse button and modifier key the actions hold down.
    /// Not a command of upstream's sessions: the runner calls it after every
    /// test, so that a test that fails with a button or a key down does not
    /// leave it down for the next tests and for the desktop after the run.
    pub fn release_input(&self) {
        let buttons: Vec<MouseButton> = self.0.buttons.borrow().clone();
        for button in buttons {
            self.button_up(button, 1);
        }
        self.release_modifiers();
    }

    fn release_modifiers(&self) {
        let held: Vec<NamedKey> = self.0.modifiers.borrow().clone();
        for key in held.into_iter().rev() {
            self.key_up(key);
        }
    }

    pub(crate) fn settle(&self) {
        self.backend().settle();
    }
}

impl PartialEq for Element {
    /// Whether two elements are the same element of the user interface.
    fn eq(&self, other: &Element) -> bool {
        self.session.backend().same(&self.native, &other.native)
    }
}

impl Element {
    pub fn session(&self) -> &Session {
        &self.session
    }

    pub(crate) fn native(&self) -> &NativeElement {
        &self.native
    }

    fn backend(&self) -> &Backend {
        self.session.backend()
    }

    /// `Text`.
    pub fn text(&self) -> String {
        self.backend().text(&self.native)
    }

    /// `Enabled`.
    pub fn enabled(&self) -> bool {
        self.backend().is_enabled(&self.native)
    }

    /// `Selected`.
    pub fn selected(&self) -> bool {
        self.backend().is_selected(&self.native)
    }

    /// `Rect`.
    pub fn rect(&self) -> ElementRect {
        self.backend().rect(&self.native)
    }

    /// `Size`.
    pub fn size(&self) -> ElementSize {
        let rect = self.rect();
        ElementSize { width: rect.width.round() as i32, height: rect.height.round() as i32 }
    }

    /// `GetAttribute(name)`, with the names of the attributes of the
    /// platform's driver.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.backend().attribute(&self.native, name)
    }

    /// `Click()`: a click of the mouse at the centre of the element, after the
    /// element is brought into view, as both drivers do (WinAppDriver; the
    /// Mac2 driver calls `click` of XCTest, which synthesizes the events of
    /// the mouse).
    pub fn click(&self) {
        let session = &self.session;
        self.backend().scroll_into_view(&self.native);
        let (x, y) = self.rect().center();
        session.click_at(x, y, MouseButton::Left);
        session.settle();
    }

    /// `Clear()`.
    pub fn clear(&self) {
        self.backend().clear(&self.native);
        self.session.settle();
    }

    /// `SendKeys(text)`: gives the element the keyboard focus and types (on
    /// macOS, where the focus does not take, after a click at its centre).
    pub fn send_keys(&self, text: &str) {
        let backend = self.backend();
        backend.focus(&self.native);
        if Backend::CLICKS_WHEN_FOCUS_FAILS && !backend.has_focus(&self.native) {
            let (x, y) = self.rect().center();
            self.session.click_at(x, y, MouseButton::Left);
        }
        self.session.type_keys(text, true);
        self.session.settle();
    }

    /// `GetScreenshot()`: the pixels of the screen under the element.
    pub fn get_screenshot(&self) -> Screenshot {
        self.backend().screenshot(&self.rect()).unwrap_or_else(|error| panic!("GetScreenshot: {error}"))
    }

    pub fn try_find_element(&self, by: By) -> Result<Element, WebDriverError> {
        self.session.find_one(Scope::Element(&self.native), &by).map(|native| self.session.wrap(native))
    }

    pub fn find_element(&self, by: By) -> Element {
        self.try_find_element(by).unwrap_or_else(|error| panic!("{error}"))
    }

    pub fn find_elements(&self, by: By) -> Vec<Element> {
        let found = self.session.find_all(Scope::Element(&self.native), &by).unwrap_or_else(|error| panic!("{error}"));
        found.into_iter().map(|native| self.session.wrap(native)).collect()
    }

    pub fn find_element_by_accessibility_id(&self, id: &str) -> Element {
        self.find_element(By::AccessibilityId(id.to_owned()))
    }

    pub fn find_elements_by_accessibility_id(&self, id: &str) -> Vec<Element> {
        self.find_elements(By::AccessibilityId(id.to_owned()))
    }

    pub fn find_element_by_name(&self, name: &str) -> Element {
        self.find_element(By::Name(name.to_owned()))
    }

    pub fn find_elements_by_name(&self, name: &str) -> Vec<Element> {
        self.find_elements(By::Name(name.to_owned()))
    }

    pub fn find_element_by_xpath(&self, expression: &str) -> Element {
        self.find_element(By::XPath(expression.to_owned()))
    }

    pub fn find_elements_by_xpath(&self, expression: &str) -> Vec<Element> {
        self.find_elements(By::XPath(expression.to_owned()))
    }

    pub fn find_elements_by_tag_name(&self, tag_name: &str) -> Vec<Element> {
        self.find_elements(By::TagName(tag_name.to_owned()))
    }
}
