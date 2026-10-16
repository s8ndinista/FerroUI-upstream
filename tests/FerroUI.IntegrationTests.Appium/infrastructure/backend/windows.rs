//! The automation backend of Windows: UI Automation, from another process,
//! through the client of the Windows backend (`ferroui_win32::automation::
//! testing`, extended for this harness), as WinAppDriver reads the
//! application for upstream's tests, and the input of the desktop through
//! `SendInput`.
//!
//! The process is aware of the scale of every monitor, so that the bounding
//! rectangles of UI Automation and the positions of the input are both in
//! pixels of the screen. The names of the tree are the ones of WinAppDriver:
//! the element types are the control types (`Window`, `Button`, `TitleBar`),
//! the attributes are the names of the properties (`AutomationId`, `Name`,
//! `ClassName`, `HasKeyboardFocus`, `AcceleratorKey`) and of the properties of
//! the patterns (`Toggle.ToggleState`).

use super::super::keys::{Key, NamedKey};
use super::super::session::{By, ElementRect, MouseButton, Screenshot, WebDriverError};
use ferroui_win32::automation::testing::{
    control_type_name, Condition, Element, UiaClient, VariantValue, RANGE_VALUE_PATTERN, SCROLL_ITEM_PATTERN,
    TREE_SCOPE_CHILDREN, TREE_SCOPE_DESCENDANTS, TREE_SCOPE_SUBTREE, VALUE_PATTERN,
};
use std::cell::RefCell;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, SRCCOPY,
};
use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, VIRTUAL_KEY,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, PostMessageW, SetForegroundWindow, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WM_CLOSE,
};

// The properties of UI Automation the harness reads (`UIA_*PropertyId`).
const PROPERTY_PROCESS_ID: i32 = 30002;
const PROPERTY_CONTROL_TYPE: i32 = 30003;
const PROPERTY_NAME: i32 = 30005;
const PROPERTY_AUTOMATION_ID: i32 = 30011;
const PROPERTY_CLASS_NAME: i32 = 30012;
const CONTROL_TYPE_WINDOW: i32 = 50032;

/// The attributes of WinAppDriver and the properties they read.
const ATTRIBUTES: &[(&str, i32)] = &[
    ("BoundingRectangle", 30001),
    ("ProcessId", 30002),
    ("ControlType", 30003),
    ("LocalizedControlType", 30004),
    ("Name", 30005),
    ("AcceleratorKey", 30006),
    ("AccessKey", 30007),
    ("HasKeyboardFocus", 30008),
    ("IsKeyboardFocusable", 30009),
    ("IsEnabled", 30010),
    ("AutomationId", 30011),
    ("ClassName", 30012),
    ("HelpText", 30013),
    ("IsControlElement", 30016),
    ("IsContentElement", 30017),
    ("IsPassword", 30019),
    ("NativeWindowHandle", 30020),
    ("IsOffscreen", 30022),
    ("Orientation", 30023),
    ("FrameworkId", 30024),
    ("ItemStatus", 30026),
    ("Value.Value", 30045),
    ("Value.IsReadOnly", 30046),
    ("RangeValue.Value", 30047),
    ("RangeValue.Minimum", 30049),
    ("RangeValue.Maximum", 30050),
    ("RangeValue.LargeChange", 30051),
    ("RangeValue.SmallChange", 30052),
    ("ExpandCollapse.ExpandCollapseState", 30070),
    ("Window.CanMaximize", 30073),
    ("Window.CanMinimize", 30074),
    ("Window.WindowVisualState", 30075),
    ("Window.IsModal", 30077),
    ("Window.IsTopmost", 30078),
    ("SelectionItem.IsSelected", 30079),
    ("Toggle.ToggleState", 30086),
];

/// The text of a value of a property, as WinAppDriver writes it.
fn text_of(value: &VariantValue) -> String {
    match value {
        VariantValue::Bool(value) => if *value { "True" } else { "False" }.to_owned(),
        VariantValue::Bstr(value) => value.clone(),
        VariantValue::I2(value) => value.to_string(),
        VariantValue::I4(value) => value.to_string(),
        VariantValue::I8(value) => value.to_string(),
        VariantValue::I1(value) => value.to_string(),
        VariantValue::UI1(value) => value.to_string(),
        VariantValue::UI2(value) => value.to_string(),
        VariantValue::UI4(value) => value.to_string(),
        VariantValue::UI8(value) => value.to_string(),
        VariantValue::R4(value) => value.to_string(),
        VariantValue::R8(value) => value.to_string(),
        other => format!("{other:?}"),
    }
}

fn hwnd_of(handle: &str) -> Option<isize> {
    isize::from_str_radix(handle.trim_start_matches("0x"), 16).ok()
}

fn handle_text(hwnd: isize) -> String {
    format!("0x{hwnd:08X}")
}

/// An element of the tree of UI Automation.
#[derive(Clone)]
pub struct NativeElement(Element);

/// The application under test (or the desktop), the client of UI Automation
/// and the input of the desktop.
pub struct Backend {
    client: UiaClient,
    /// The process of the application; none for the session of the desktop.
    child: RefCell<Option<Child>>,
    process_id: u32,
}

/// How long the input of a command is given to arrive and be handled before
/// the command returns.
const SETTLE: Duration = Duration::from_millis(150);

impl Backend {
    /// Whether `SendKeys` clicks an element whose focus could not be set.
    /// WinAppDriver sets the focus and types: a click would close a menu
    /// that has the keyboard (`MenuTests.Select_Child_With_Alt_Access_Keys`
    /// sends the access keys to the window while its menu is open).
    pub const CLICKS_WHEN_FOCUS_FAILS: bool = false;

    /// UI Automation and `SendInput` need the desktop of an interactive
    /// session, which the process has when it runs on Windows at all; there
    /// is no permission to ask for.
    pub fn check() -> Result<(), String> {
        UiaClient::new().map(|_| ())
    }

    fn new(child: Option<Child>) -> Result<Backend, String> {
        // SAFETY: a context the system defines; the call fails harmlessly when
        // the awareness of the process is set already.
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let process_id = child.as_ref().map_or(0, Child::id);
        let client = UiaClient::new()?;
        // A call to an application that stopped responding fails after 5 s instead of 20.
        let _ = client.set_timeouts(2000, 5000);
        Ok(Backend { client, child: RefCell::new(child), process_id })
    }

    /// Starts the application and waits for its main window, which it brings
    /// to the front (upstream does the same after WinAppDriver started it).
    pub fn launch(app: &Path, arguments: &[&str]) -> Result<Backend, String> {
        let child = Command::new(app)
            .args(arguments)
            .spawn()
            .map_err(|error| format!("cannot start {}: {error}", app.display()))?;
        let backend = Backend::new(Some(child))?;
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let main = backend.top_level_windows().into_iter().find(|window| {
                window.automation_id().as_deref() == Ok("MainWindow")
            });
            if let Some(main) = main {
                let hwnd = main.native_window_handle().unwrap_or(0);
                // SAFETY: a window handle the system reported.
                unsafe { SetForegroundWindow(hwnd as HWND) };
                std::thread::sleep(Duration::from_millis(500));
                return Ok(backend);
            }
            if let Some(status) = backend.child.borrow_mut().as_mut().and_then(|child| child.try_wait().ok().flatten()) {
                return Err(format!("{} ended before it showed its main window: {status}", app.display()));
            }
            if Instant::now() > deadline {
                return Err(format!("{} showed no main window in 60 s", app.display()));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// The session of the whole desktop (`Root` of WinAppDriver).
    pub fn desktop() -> Result<Backend, String> {
        Backend::new(None)
    }

    pub fn quit(&self) {
        if let Some(mut child) = self.child.borrow_mut().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn settle(&self) {
        std::thread::sleep(SETTLE);
    }

    fn condition(&self, property_id: i32, value: VariantValue) -> Condition {
        self.client.property_condition(property_id, &value).unwrap_or_else(|error| panic!("{error}"))
    }

    /// The top-level windows of the application, in the order of the desktop.
    fn top_level_windows(&self) -> Vec<Element> {
        let Ok(desktop) = self.client.root_element() else { return Vec::new() };
        let process = self.condition(PROPERTY_PROCESS_ID, VariantValue::I4(self.process_id as i32));
        let window = self.condition(PROPERTY_CONTROL_TYPE, VariantValue::I4(CONTROL_TYPE_WINDOW));
        let both = self.client.and_condition(&process, &window).unwrap_or_else(|error| panic!("{error}"));
        desktop.find_all(TREE_SCOPE_CHILDREN, &both).unwrap_or_default()
    }

    /// `WindowHandles`: the top-level windows of the application.
    pub fn window_handles(&self) -> Vec<String> {
        if self.child.borrow().is_none() {
            return Vec::new();
        }
        self.top_level_windows()
            .iter()
            .filter_map(|window| window.native_window_handle().ok().filter(|hwnd| *hwnd != 0))
            .map(handle_text)
            .collect()
    }

    /// The root of a search from the session: the current window; the
    /// desktop for the session of the desktop.
    pub fn session_root(&self, window: Option<&str>) -> Result<NativeElement, WebDriverError> {
        if self.child.borrow().is_none() {
            return self.client.root_element().map(NativeElement).map_err(WebDriverError::Other);
        }
        let handle = window.ok_or_else(|| WebDriverError::NoSuchWindow("the session has no current window".to_owned()))?;
        let hwnd = hwnd_of(handle).ok_or_else(|| WebDriverError::NoSuchWindow(handle.to_owned()))?;
        self.client
            .element_from_handle(hwnd)
            .map(NativeElement)
            .map_err(|error| WebDriverError::NoSuchWindow(format!("{handle}: {error}")))
    }

    pub fn activate_window(&self, handle: &str) {
        if let Some(hwnd) = hwnd_of(handle) {
            // SAFETY: a window handle; a stale one makes the call fail.
            unsafe { SetForegroundWindow(hwnd as HWND) };
        }
    }

    /// Closes a window: through its window pattern, else with `WM_CLOSE`.
    pub fn close_window(&self, handle: &str) {
        let Some(hwnd) = hwnd_of(handle) else { return };
        let closed = self.client.element_from_handle(hwnd).and_then(|window| window.close()).is_ok();
        if !closed {
            // SAFETY: a window handle; a stale one makes the call fail.
            unsafe { PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0) };
        }
    }

    pub fn children(&self, element: &NativeElement) -> Vec<NativeElement> {
        self.client.control_view_children(&element.0).unwrap_or_default().into_iter().map(NativeElement).collect()
    }

    pub fn same(&self, a: &NativeElement, b: &NativeElement) -> bool {
        self.client.compare_elements(&a.0, &b.0).unwrap_or(false)
    }

    /// The control type, as XPath names it.
    pub fn tag_name(&self, element: &NativeElement) -> String {
        element.0.control_type().map(control_type_name).unwrap_or_default()
    }

    pub fn automation_id(&self, element: &NativeElement) -> String {
        element.0.automation_id().unwrap_or_default()
    }

    pub fn name(&self, element: &NativeElement) -> String {
        element.0.name().unwrap_or_default()
    }

    pub fn class_name(&self, element: &NativeElement) -> String {
        element.0.class_name().unwrap_or_default()
    }

    /// The text: the value of the value pattern, else the value of the range
    /// value pattern, else the name.
    pub fn text(&self, element: &NativeElement) -> String {
        let element = &element.0;
        if element.has_pattern(VALUE_PATTERN).unwrap_or(false) {
            return element.value().unwrap_or_default();
        }
        if element.has_pattern(RANGE_VALUE_PATTERN).unwrap_or(false) {
            return element.range_value().map(|value| format!("{value}")).unwrap_or_default();
        }
        element.name().unwrap_or_default()
    }

    pub fn attribute(&self, element: &NativeElement, name: &str) -> Option<String> {
        let property_id = ATTRIBUTES.iter().find(|(attribute, _)| *attribute == name).map(|(_, id)| *id)?;
        element.0.property_value(property_id).ok().flatten().map(|value| text_of(&value))
    }

    pub fn is_enabled(&self, element: &NativeElement) -> bool {
        element.0.is_enabled().unwrap_or(false)
    }

    pub fn is_selected(&self, element: &NativeElement) -> bool {
        element.0.is_selected().unwrap_or(false)
    }

    pub fn has_focus(&self, element: &NativeElement) -> bool {
        element.0.has_keyboard_focus().unwrap_or(false)
    }

    pub fn rect(&self, element: &NativeElement) -> ElementRect {
        let [left, top, right, bottom] = element.0.bounding_rectangle().unwrap_or_default();
        ElementRect {
            x: f64::from(left),
            y: f64::from(top),
            width: f64::from(right - left),
            height: f64::from(bottom - top),
        }
    }

    /// WinAppDriver clicks every element with the mouse: nothing is pressed.
    pub fn press(&self, _element: &NativeElement) -> bool {
        false
    }

    /// Brings the element into view before a click at its centre (the
    /// "scroll into view" of the element click of WebDriver): where the
    /// centre is outside the bounds of one of the ancestors of the element
    /// up to its window (a list scrolled away from it, a scroll viewer), or
    /// the element says it is off the screen, the scroll item pattern
    /// scrolls it into view. Upstream's runs need none (their desktop has a
    /// resolution of 4K, where the main window shows every item of its list
    /// of pages); a smaller desktop leaves the last pages below the end of
    /// the list.
    pub fn scroll_into_view(&self, element: &NativeElement) {
        if self.is_in_view(element) {
            return;
        }
        if element.0.has_pattern(SCROLL_ITEM_PATTERN).unwrap_or(false) && element.0.scroll_into_view().is_ok() {
            self.settle();
        }
    }

    /// Whether the centre of the element is inside every ancestor of it up
    /// to its window, and the element is not off the screen.
    fn is_in_view(&self, element: &NativeElement) -> bool {
        if element.0.is_offscreen().unwrap_or(false) {
            return false;
        }
        let (x, y) = self.rect(element).center();
        let inside = |rect: &ElementRect| {
            rect.width <= 0.0
                || rect.height <= 0.0
                || (x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height)
        };
        let mut current = element.0.clone();
        for _ in 0..64 {
            if current.control_type().ok() == Some(CONTROL_TYPE_WINDOW) {
                break;
            }
            let Ok(Some(parent)) = self.client.control_view_parent(&current) else { break };
            let parent_rect = self.rect(&NativeElement(parent.clone()));
            if !inside(&parent_rect) {
                return false;
            }
            current = parent;
        }
        true
    }

    pub fn focus(&self, element: &NativeElement) {
        let _ = element.0.set_focus();
    }

    pub fn clear(&self, element: &NativeElement) {
        if element.0.set_value("").is_err() {
            self.focus(element);
            self.key(Key::Named(NamedKey::Control), true);
            self.key(Key::Char('a'), true);
            self.key(Key::Char('a'), false);
            self.key(Key::Named(NamedKey::Control), false);
            self.key(Key::Named(NamedKey::Delete), true);
            self.key(Key::Named(NamedKey::Delete), false);
        }
    }

    /// The elements below `root` (and `root` itself with `include_root`)
    /// that the locator finds, in the order of the tree: one search of UI
    /// Automation in the control view.
    pub fn find_all(
        &self,
        root: &NativeElement,
        include_root: bool,
        by: &By,
        thorough: bool,
    ) -> Result<Vec<NativeElement>, String> {
        let condition = match by {
            By::AccessibilityId(id) => self.condition(PROPERTY_AUTOMATION_ID, VariantValue::Bstr(id.clone())),
            By::Name(name) => self.condition(PROPERTY_NAME, VariantValue::Bstr(name.clone())),
            By::ClassName(class) => self.condition(PROPERTY_CLASS_NAME, VariantValue::Bstr(class.clone())),
            By::TagName(tag) => {
                let Some(control_type) = (50000..=50040).find(|id| control_type_name(*id) == *tag) else {
                    return Ok(Vec::new());
                };
                self.condition(PROPERTY_CONTROL_TYPE, VariantValue::I4(control_type))
            }
            By::XPath(_) => return Ok(Vec::new()),
        };
        let control_view = self.client.control_view_condition()?;
        let condition = self.client.and_condition(&condition, &control_view)?;
        let scope = if include_root { TREE_SCOPE_SUBTREE } else { TREE_SCOPE_DESCENDANTS };
        let found: Vec<NativeElement> = root.0.find_all(scope, &condition)?.into_iter().map(NativeElement).collect();
        if !found.is_empty() || !thorough {
            return Ok(found);
        }
        // One search of UI Automation does not reach the elements after a native window hosted
        // in the tree (the native text box of the embedding page): having visited the window of
        // the native control below its host, it goes on from the parent that window has among
        // the windows of the system, a holder window that is a child of the main window, and
        // so through the window again and again (2717 elements for a tree of 66, and not the
        // button after the host). The walk of the control view, which goes from the host to its
        // next sibling, finds them, as WinAppDriver does in upstream's `EmbeddingTests`.
        Ok(self.walk_find(root, include_root, by))
    }

    /// The elements below `root` (and `root` itself with `include_root`) that the locator
    /// finds, in the order of a walk of the control view.
    fn walk_find(&self, root: &NativeElement, include_root: bool, by: &By) -> Vec<NativeElement> {
        let matches = |element: &NativeElement| match by {
            By::AccessibilityId(id) => self.automation_id(element) == *id,
            By::Name(name) => self.name(element) == *name,
            By::ClassName(class) => self.class_name(element) == *class,
            By::TagName(tag) => self.tag_name(element) == *tag,
            By::XPath(_) => false,
        };
        let mut found = Vec::new();
        let mut stack: Vec<(NativeElement, bool)> = vec![(root.clone(), include_root)];
        let mut visited = 0usize;
        while let Some((element, check)) = stack.pop() {
            visited += 1;
            if visited > 20_000 {
                break;
            }
            if check && matches(&element) {
                found.push(element.clone());
            }
            let children = self.children(&element);
            stack.extend(children.into_iter().rev().map(|child| (child, true)));
        }
        found
    }

    /// Whether the application answers: its main window handles a message
    /// within 5 s (`SendMessageTimeout` of `WM_NULL`, which gives up at once
    /// for a window the system reports as hung). The runner starts the
    /// application again when it does not.
    pub fn is_responsive(&self) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_NULL};
        if self.child.borrow_mut().as_mut().is_some_and(|child| child.try_wait().ok().flatten().is_some()) {
            return false;
        }
        let Some(main) = self.top_level_windows().into_iter().find(|w| w.automation_id().as_deref() == Ok("MainWindow"))
        else {
            return false;
        };
        let Ok(hwnd) = main.native_window_handle() else { return false };
        let mut result = 0usize;
        // SAFETY: a window handle the system reported and a place for the result.
        unsafe { SendMessageTimeoutW(hwnd as HWND, WM_NULL, 0, 0, SMTO_ABORTIFHUNG, 5000, &mut result) != 0 }
    }

    /// A diagnostic of `--dump-tree` after a test that failed: the top-level
    /// windows of the application in the z-order of the desktop (front
    /// first), with their owner, whether they are topmost and visible, the
    /// foreground window, and the title of each.
    pub fn windows_diagnostic(&self) -> Option<String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetTopWindow, GetWindow, GetWindowLongPtrW, GetWindowThreadProcessId,
            InternalGetWindowText, IsWindowVisible, GWL_EXSTYLE, GW_HWNDNEXT, GW_OWNER, WS_EX_TOPMOST,
        };
        if self.process_id == 0 {
            return None;
        }
        let mut text = String::new();
        // SAFETY: queries of the window manager on handles it returned; a
        // handle that went away makes a query answer zero.
        unsafe {
            text.push_str(&format!("   foreground window: {}\n", handle_text(GetForegroundWindow() as isize)));
            let mut hwnd = GetTopWindow(std::ptr::null_mut());
            let mut index = 0usize;
            while !hwnd.is_null() && index < 10_000 {
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid == self.process_id {
                    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                    let owner = GetWindow(hwnd, GW_OWNER) as isize;
                    // The title without a message to the window (a window that does not
                    // respond would hold `GetWindowText`).
                    let mut title = [0u16; 128];
                    let length = InternalGetWindowText(hwnd, title.as_mut_ptr(), title.len() as i32).max(0) as usize;
                    let id = String::from_utf16_lossy(&title[..length.min(title.len())]);
                    text.push_str(&format!(
                        "   z {index}: {} title={id:?} owner={} topmost={} visible={}\n",
                        handle_text(hwnd as isize),
                        handle_text(owner),
                        ex_style & WS_EX_TOPMOST != 0,
                        IsWindowVisible(hwnd) != 0
                    ));
                }
                hwnd = GetWindow(hwnd, GW_HWNDNEXT);
                index += 1;
            }
        }
        Some(text)
    }

    /// A diagnostic of `--dump-tree`: the elements one search of UI Automation
    /// returns for the whole subtree of `root` (the control view, then the
    /// raw view), in its order, to compare with the walk of the tree.
    pub fn search_diagnostic(&self, root: &NativeElement) -> Option<String> {
        let mut text = String::new();
        let views = [("control view", self.client.control_view_condition()), ("raw view", self.client.true_condition())];
        for (view, condition) in views {
            let found = condition.and_then(|condition| root.0.find_all(TREE_SCOPE_SUBTREE, &condition));
            match found {
                Ok(found) => {
                    text.push_str(&format!("   FindAll of the subtree, {view}: {} elements\n", found.len()));
                    for element in found {
                        let element = NativeElement(element);
                        text.push_str(&format!(
                            "     {} id={:?} name={:?}\n",
                            self.tag_name(&element),
                            self.automation_id(&element),
                            self.name(&element)
                        ));
                    }
                }
                Err(error) => text.push_str(&format!("   FindAll of the subtree, {view}: {error}\n")),
            }
        }
        Some(text)
    }

    /// The pixels of the screen in the rectangle (`GetScreenshot` of
    /// WinAppDriver, which copies the screen too).
    pub fn screenshot(&self, rect: &ElementRect) -> Result<Screenshot, String> {
        let (x, y) = (rect.x.round() as i32, rect.y.round() as i32);
        let (width, height) = (rect.width.round() as i32, rect.height.round() as i32);
        if width <= 0 || height <= 0 {
            return Err(format!("the element has no area: {rect:?}"));
        }
        // SAFETY: the device contexts and the bitmap are created, selected and
        // released in pairs; the buffer has the size the header describes.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let memory = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, width, height);
            let previous = SelectObject(memory, bitmap);
            let copied = BitBlt(memory, 0, 0, width, height, screen, x, y, SRCCOPY | CAPTUREBLT);
            SelectObject(memory, previous);
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bgra = vec![0u8; (width * height * 4) as usize];
            let lines = GetDIBits(memory, bitmap, 0, height as u32, bgra.as_mut_ptr().cast(), &mut info, DIB_RGB_COLORS);
            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);
            if copied == 0 || lines == 0 {
                return Err("the screen could not be copied".to_owned());
            }
            for pixel in bgra.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                pixel[3] = 255;
            }
            Ok(Screenshot { width: width as u32, height: height as u32, rgba: bgra })
        }
    }

    // ---- input -----------------------------------------------------------------------------------

    fn send(&self, inputs: &[INPUT]) {
        // SAFETY: an array of inputs of the given length and size.
        unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    fn mouse_input(dx: i32, dy: i32, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
        }
    }

    pub fn mouse_move(&self, x: f64, y: f64) {
        // SAFETY: metrics of the system.
        let (left, top, width, height) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        // The absolute coordinates of `SendInput` span the virtual screen from 0 to 65535.
        let normalize = |value: f64, origin: i32, extent: i32| -> i32 {
            ((value - f64::from(origin)) * 65535.0 / f64::from((extent - 1).max(1))).round() as i32
        };
        let dx = normalize(x.round(), left, width);
        let dy = normalize(y.round(), top, height);
        self.send(&[Self::mouse_input(dx, dy, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK)]);
    }

    pub fn mouse_button(&self, button: MouseButton, down: bool, _x: f64, _y: f64, _click_count: u32) {
        let flags = match (button, down) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
        };
        self.send(&[Self::mouse_input(0, 0, flags)]);
    }

    fn key_input(vk: VIRTUAL_KEY, scan: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
        }
    }

    fn virtual_key(&self, vk: VIRTUAL_KEY, extended: bool, down: bool) {
        // SAFETY: a translation of the keyboard layout of the thread.
        let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u16;
        let mut flags = if down { 0 } else { KEYEVENTF_KEYUP };
        if extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        self.send(&[Self::key_input(vk, scan, flags)]);
    }

    pub fn key(&self, key: Key, down: bool) {
        match key {
            Key::Named(named) => {
                let (vk, extended): (VIRTUAL_KEY, bool) = match named {
                    NamedKey::Shift => (0x10, false),
                    NamedKey::Control => (0x11, false),
                    NamedKey::Alt => (0x12, false),
                    NamedKey::Meta => (0x5B, true),
                    NamedKey::Cancel => (0x03, false),
                    NamedKey::Help => (0x2F, false),
                    NamedKey::Backspace => (0x08, false),
                    NamedKey::Tab => (0x09, false),
                    NamedKey::Clear => (0x0C, false),
                    NamedKey::Return | NamedKey::Enter => (0x0D, false),
                    NamedKey::Pause => (0x13, false),
                    NamedKey::Escape => (0x1B, false),
                    NamedKey::Space => (0x20, false),
                    NamedKey::PageUp => (0x21, true),
                    NamedKey::PageDown => (0x22, true),
                    NamedKey::End => (0x23, true),
                    NamedKey::Home => (0x24, true),
                    NamedKey::Left => (0x25, true),
                    NamedKey::Up => (0x26, true),
                    NamedKey::Right => (0x27, true),
                    NamedKey::Down => (0x28, true),
                    NamedKey::Insert => (0x2D, true),
                    NamedKey::Delete => (0x2E, true),
                    NamedKey::F(n) => (0x6F + VIRTUAL_KEY::from(n), false),
                    NamedKey::Null => return,
                };
                self.virtual_key(vk, extended, down);
            }
            Key::Char(c) => {
                let mut units = [0u16; 2];
                let units = c.encode_utf16(&mut units);
                // SAFETY: a translation of the keyboard layout of the thread.
                let scan = if units.len() == 1 { unsafe { VkKeyScanW(units[0]) } } else { -1 };
                if scan == -1 {
                    // A character without a key of the layout: typed as text.
                    let flags = KEYEVENTF_UNICODE | if down { 0 } else { KEYEVENTF_KEYUP };
                    let inputs: Vec<INPUT> = units.iter().map(|unit| Self::key_input(0, *unit, flags)).collect();
                    self.send(&inputs);
                    return;
                }
                let vk = (scan & 0xFF) as VIRTUAL_KEY;
                let shift = scan & 0x100 != 0;
                if shift && down {
                    self.virtual_key(0x10, false, true);
                }
                self.virtual_key(vk, false, down);
                if shift && !down {
                    self.virtual_key(0x10, false, false);
                }
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.quit();
    }
}

