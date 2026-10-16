//! The automation backend of macOS: the accessibility API of the system
//! (`AXUIElement` of the application services framework), from another
//! process, as the Mac2 driver of upstream's tests reads the application
//! through XCTest, and the input of the desktop through events of the window
//! server (`CGEventPost`).
//!
//! Both need the Accessibility permission of the process that runs the tests
//! (System Settings, Privacy & Security, Accessibility); [`Backend::check`]
//! reports its absence.
//!
//! The names of the tree are the ones of XCTest, which the tests are written
//! against: the element types are `XCUIElementType*` after the role, the
//! buttons of a window have the identifiers `_XCUI:CloseWindow` and the
//! others, and the attributes are `identifier`, `title`, `label`, `value`,
//! `enabled`, `selected` and `hittable`.

#![allow(non_upper_case_globals, non_snake_case)]

use super::super::keys::{Key, NamedKey};
use super::super::session::{
    By, ElementRect, MouseButton, Screenshot, WebDriverError, SCREENSHOT_NOT_PROVIDED_ON_MAC_OS,
};
use std::cell::{Cell, RefCell};
use std::ffi::{c_void, CStr};
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

// ---- the system interfaces ------------------------------------------------------------------------

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFArrayRef = *const c_void;
type CFIndex = isize;
type AXUIElementRef = *const c_void;
type AXError = i32;
type CGEventRef = *mut c_void;
type CGEventSourceRef = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

const kCFStringEncodingUTF8: u32 = 0x0800_0100;
const kCFNumberDoubleType: CFIndex = 13;
const kAXErrorSuccess: AXError = 0;
const kAXValueCGPointType: u32 = 1;
const kAXValueCGSizeType: u32 = 2;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    fn CFRelease(value: CFTypeRef);
    fn CFRetain(value: CFTypeRef) -> CFTypeRef;
    fn CFEqual(a: CFTypeRef, b: CFTypeRef) -> u8;
    fn CFGetTypeID(value: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFStringCreateWithBytes(
        allocator: CFTypeRef,
        bytes: *const u8,
        length: CFIndex,
        encoding: u32,
        external: u8,
    ) -> CFStringRef;
    fn CFStringGetLength(string: CFStringRef) -> CFIndex;
    fn CFStringGetMaximumSizeForEncoding(length: CFIndex, encoding: u32) -> CFIndex;
    fn CFStringGetCString(string: CFStringRef, buffer: *mut i8, size: CFIndex, encoding: u32) -> u8;
    fn CFNumberGetValue(number: CFTypeRef, kind: CFIndex, value: *mut c_void) -> u8;
    fn CFBooleanGetValue(boolean: CFTypeRef) -> u8;
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> CFTypeRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(element: AXUIElementRef, attribute: CFStringRef, value: *mut CFTypeRef) -> AXError;
    fn AXUIElementSetAttributeValue(element: AXUIElementRef, attribute: CFStringRef, value: CFTypeRef) -> AXError;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
    fn AXUIElementCopyActionNames(element: AXUIElementRef, names: *mut CFArrayRef) -> AXError;
    fn AXUIElementCopyElementAtPosition(application: AXUIElementRef, x: f32, y: f32, element: *mut AXUIElementRef) -> AXError;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, seconds: f32) -> AXError;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut c_void) -> u8;
    fn AXValueGetTypeID() -> usize;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state: i32) -> CGEventSourceRef;
    fn CGEventCreateMouseEvent(source: CGEventSourceRef, kind: u32, position: CGPoint, button: u32) -> CGEventRef;
    fn CGEventCreateKeyboardEvent(source: CGEventSourceRef, key: u16, down: u8) -> CGEventRef;
    fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);
    fn CGEventSetFlags(event: CGEventRef, flags: u64);
    fn CGEventSetType(event: CGEventRef, kind: u32);
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventPost(tap: u32, event: CGEventRef);
}

const kCGEventSourceStateHIDSystemState: i32 = 1;
const kCGHIDEventTap: u32 = 0;
const kCGMouseEventClickState: u32 = 1;
const kCGEventLeftMouseDown: u32 = 1;
const kCGEventLeftMouseUp: u32 = 2;
const kCGEventRightMouseDown: u32 = 3;
const kCGEventRightMouseUp: u32 = 4;
const kCGEventMouseMoved: u32 = 5;
const kCGEventLeftMouseDragged: u32 = 6;
const kCGEventRightMouseDragged: u32 = 7;
const kCGEventFlagsChanged: u32 = 12;
const kCGEventFlagMaskShift: u64 = 0x0002_0000;
const kCGEventFlagMaskControl: u64 = 0x0004_0000;
const kCGEventFlagMaskAlternate: u64 = 0x0008_0000;
const kCGEventFlagMaskCommand: u64 = 0x0010_0000;

// ---- values of Core Foundation ---------------------------------------------------------------------

/// A Core Foundation object of which this value owns one reference.
struct Owned(CFTypeRef);

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: a live object of which this value owns a reference.
            unsafe { CFRelease(self.0) }
        }
    }
}

impl Clone for Owned {
    fn clone(&self) -> Owned {
        if !self.0.is_null() {
            // SAFETY: a live object; the new value owns the new reference.
            unsafe { CFRetain(self.0) };
        }
        Owned(self.0)
    }
}

fn cf_string(text: &str) -> Owned {
    // SAFETY: UTF-8 bytes of the given length; the result is owned.
    Owned(unsafe {
        CFStringCreateWithBytes(std::ptr::null(), text.as_ptr(), text.len() as CFIndex, kCFStringEncodingUTF8, 0)
    })
}

fn type_of(value: CFTypeRef) -> usize {
    // SAFETY: a live object.
    unsafe { CFGetTypeID(value) }
}

fn string_of(value: CFTypeRef) -> Option<String> {
    // SAFETY: the type is checked before the string functions are called; the
    // buffer is as large as the system says the string can be.
    unsafe {
        if value.is_null() || type_of(value) != CFStringGetTypeID() {
            return None;
        }
        let size = CFStringGetMaximumSizeForEncoding(CFStringGetLength(value), kCFStringEncodingUTF8) + 1;
        let mut buffer = vec![0i8; size as usize];
        if CFStringGetCString(value, buffer.as_mut_ptr(), size, kCFStringEncodingUTF8) == 0 {
            return None;
        }
        Some(CStr::from_ptr(buffer.as_ptr()).to_string_lossy().into_owned())
    }
}

/// A number the way `NSNumber.stringValue` writes it: no fraction for a whole
/// number.
fn format_number(value: f64) -> String {
    format!("{value}")
}

/// The text of a value of an attribute: a string, a number, a boolean.
fn text_of(value: CFTypeRef) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: the type is checked before each accessor is called.
    unsafe {
        let kind = type_of(value);
        if kind == CFStringGetTypeID() {
            string_of(value)
        } else if kind == CFBooleanGetTypeID() {
            Some(if CFBooleanGetValue(value) != 0 { "1" } else { "0" }.to_owned())
        } else if kind == CFNumberGetTypeID() {
            let mut number = 0f64;
            CFNumberGetValue(value, kCFNumberDoubleType, (&mut number as *mut f64).cast());
            Some(format_number(number))
        } else {
            None
        }
    }
}

fn bool_of(value: CFTypeRef) -> Option<bool> {
    if value.is_null() {
        return None;
    }
    // SAFETY: the type is checked first.
    unsafe {
        let kind = type_of(value);
        if kind == CFBooleanGetTypeID() {
            Some(CFBooleanGetValue(value) != 0)
        } else if kind == CFNumberGetTypeID() {
            let mut number = 0f64;
            CFNumberGetValue(value, kCFNumberDoubleType, (&mut number as *mut f64).cast());
            Some(number != 0.0)
        } else {
            None
        }
    }
}

// ---- elements ----------------------------------------------------------------------------------------

/// An element of the accessibility tree of the application.
#[derive(Clone)]
pub struct NativeElement(Owned);

impl NativeElement {
    fn raw(&self) -> AXUIElementRef {
        self.0 .0
    }

    /// The value of an attribute, owned.
    fn copy(&self, attribute: &str) -> Option<Owned> {
        let name = cf_string(attribute);
        let mut value: CFTypeRef = std::ptr::null();
        // SAFETY: a live element, a live name and a place for the result,
        // which is owned by the caller.
        let error = unsafe { AXUIElementCopyAttributeValue(self.raw(), name.0, &mut value) };
        (error == kAXErrorSuccess && !value.is_null()).then(|| Owned(value))
    }

    fn string(&self, attribute: &str) -> Option<String> {
        self.copy(attribute).and_then(|value| string_of(value.0))
    }

    fn boolean(&self, attribute: &str) -> Option<bool> {
        self.copy(attribute).and_then(|value| bool_of(value.0))
    }

    fn element(&self, attribute: &str) -> Option<NativeElement> {
        self.copy(attribute).filter(|value| !value.0.is_null()).map(NativeElement)
    }

    fn elements(&self, attribute: &str) -> Vec<NativeElement> {
        let Some(array) = self.copy(attribute) else { return Vec::new() };
        // SAFETY: the type is checked; every item is retained for the
        // element made of it.
        unsafe {
            if type_of(array.0) != CFArrayGetTypeID() {
                return Vec::new();
            }
            (0..CFArrayGetCount(array.0))
                .map(|index| {
                    let item = CFArrayGetValueAtIndex(array.0, index);
                    CFRetain(item);
                    NativeElement(Owned(item))
                })
                .collect()
        }
    }

    fn set(&self, attribute: &str, value: CFTypeRef) -> bool {
        let name = cf_string(attribute);
        // SAFETY: a live element, name and value.
        unsafe { AXUIElementSetAttributeValue(self.raw(), name.0, value) == kAXErrorSuccess }
    }

    fn actions(&self) -> Vec<String> {
        let mut names: CFArrayRef = std::ptr::null();
        // SAFETY: a live element and a place for the owned result.
        if unsafe { AXUIElementCopyActionNames(self.raw(), &mut names) } != kAXErrorSuccess || names.is_null() {
            return Vec::new();
        }
        let names = Owned(names);
        // SAFETY: an array of strings.
        unsafe { (0..CFArrayGetCount(names.0)).filter_map(|i| string_of(CFArrayGetValueAtIndex(names.0, i))).collect() }
    }

    fn perform(&self, action: &str) -> bool {
        let name = cf_string(action);
        // SAFETY: a live element and name.
        unsafe { AXUIElementPerformAction(self.raw(), name.0) == kAXErrorSuccess }
    }

    fn point(&self, attribute: &str) -> Option<(f64, f64)> {
        let value = self.copy(attribute)?;
        let mut point = CGPoint::default();
        // SAFETY: the type is checked; the value is a point.
        let ok = unsafe {
            type_of(value.0) == AXValueGetTypeID()
                && AXValueGetValue(value.0, kAXValueCGPointType, (&mut point as *mut CGPoint).cast()) != 0
        };
        ok.then_some((point.x, point.y))
    }

    fn size(&self, attribute: &str) -> Option<(f64, f64)> {
        let value = self.copy(attribute)?;
        let mut size = CGSize::default();
        // SAFETY: the type is checked; the value is a size.
        let ok = unsafe {
            type_of(value.0) == AXValueGetTypeID()
                && AXValueGetValue(value.0, kAXValueCGSizeType, (&mut size as *mut CGSize).cast()) != 0
        };
        ok.then_some((size.width, size.height))
    }

    fn role(&self) -> String {
        self.string("AXRole").unwrap_or_default()
    }
}

/// The identifier XCTest gives the buttons of the title bar, by subrole.
fn xctest_identifier(subrole: &str) -> Option<&'static str> {
    Some(match subrole {
        "AXCloseButton" => "_XCUI:CloseWindow",
        "AXMinimizeButton" => "_XCUI:MinimizeWindow",
        "AXZoomButton" => "_XCUI:ZoomWindow",
        "AXFullScreenButton" => "_XCUI:FullScreenWindow",
        _ => return None,
    })
}

/// The `XCUIElementType` of a role.
fn element_type(role: &str, subrole: &str, in_extras_menu_bar: bool) -> &'static str {
    match role {
        "AXApplication" => "XCUIElementTypeApplication",
        "AXWindow" if subrole == "AXDialog" => "XCUIElementTypeDialog",
        "AXWindow" => "XCUIElementTypeWindow",
        "AXSheet" => "XCUIElementTypeSheet",
        "AXDrawer" => "XCUIElementTypeDrawer",
        "AXButton" => "XCUIElementTypeButton",
        "AXRadioButton" => "XCUIElementTypeRadioButton",
        "AXRadioGroup" => "XCUIElementTypeRadioGroup",
        "AXCheckBox" => "XCUIElementTypeCheckBox",
        "AXDisclosureTriangle" => "XCUIElementTypeDisclosureTriangle",
        "AXPopUpButton" => "XCUIElementTypePopUpButton",
        "AXComboBox" => "XCUIElementTypeComboBox",
        "AXMenuButton" => "XCUIElementTypeMenuButton",
        "AXMenuBar" => "XCUIElementTypeMenuBar",
        "AXMenuBarItem" if in_extras_menu_bar => "XCUIElementTypeStatusItem",
        "AXMenuBarItem" => "XCUIElementTypeMenuBarItem",
        "AXMenu" => "XCUIElementTypeMenu",
        "AXMenuItem" => "XCUIElementTypeMenuItem",
        "AXStaticText" => "XCUIElementTypeStaticText",
        "AXTextField" if subrole == "AXSecureTextField" => "XCUIElementTypeSecureTextField",
        "AXTextField" => "XCUIElementTypeTextField",
        "AXTextArea" => "XCUIElementTypeTextView",
        "AXGroup" => "XCUIElementTypeGroup",
        "AXScrollArea" => "XCUIElementTypeScrollView",
        "AXScrollBar" => "XCUIElementTypeScrollBar",
        "AXSlider" => "XCUIElementTypeSlider",
        "AXIncrementor" => "XCUIElementTypeStepper",
        "AXProgressIndicator" => "XCUIElementTypeProgressIndicator",
        "AXBusyIndicator" => "XCUIElementTypeActivityIndicator",
        "AXImage" => "XCUIElementTypeImage",
        "AXLink" => "XCUIElementTypeLink",
        "AXTabGroup" => "XCUIElementTypeTabGroup",
        "AXToolbar" => "XCUIElementTypeToolbar",
        "AXTable" => "XCUIElementTypeTable",
        "AXList" => "XCUIElementTypeTable",
        "AXOutline" => "XCUIElementTypeOutline",
        "AXRow" if subrole == "AXOutlineRow" => "XCUIElementTypeOutlineRow",
        "AXRow" => "XCUIElementTypeTableRow",
        "AXColumn" => "XCUIElementTypeTableColumn",
        "AXCell" => "XCUIElementTypeCell",
        "AXSplitGroup" => "XCUIElementTypeSplitGroup",
        "AXSplitter" => "XCUIElementTypeSplitter",
        "AXHandle" => "XCUIElementTypeHandle",
        "AXPopover" => "XCUIElementTypePopover",
        "AXColorWell" => "XCUIElementTypeColorWell",
        "AXGrid" => "XCUIElementTypeGrid",
        "AXLevelIndicator" => "XCUIElementTypeLevelIndicator",
        "AXValueIndicator" => "XCUIElementTypeValueIndicator",
        "AXLayoutArea" => "XCUIElementTypeLayoutArea",
        "AXLayoutItem" => "XCUIElementTypeLayoutItem",
        "AXHelpTag" => "XCUIElementTypeHelpTag",
        "AXMatte" => "XCUIElementTypeMatte",
        "AXRuler" => "XCUIElementTypeRuler",
        "AXRulerMarker" => "XCUIElementTypeRulerMarker",
        "AXBrowser" => "XCUIElementTypeBrowser",
        "AXDateField" => "XCUIElementTypeDatePicker",
        "AXHeading" => "XCUIElementTypeStaticText",
        _ => "XCUIElementTypeOther",
    }
}

/// The virtual key code of a key of the US layout, and whether it needs the
/// shift key, for a character.
fn key_code_of_char(c: char) -> Option<(u16, bool)> {
    const LETTERS: [u16; 26] = [
        0x00, 0x0B, 0x08, 0x02, 0x0E, 0x03, 0x05, 0x04, 0x22, 0x26, 0x28, 0x25, 0x2E, 0x2D, 0x1F, 0x23, 0x0C, 0x0F,
        0x01, 0x11, 0x20, 0x09, 0x0D, 0x07, 0x10, 0x06,
    ];
    const DIGITS: [u16; 10] = [0x1D, 0x12, 0x13, 0x14, 0x15, 0x17, 0x16, 0x1A, 0x1C, 0x19];
    Some(match c {
        'a'..='z' => (LETTERS[(c as u8 - b'a') as usize], false),
        'A'..='Z' => (LETTERS[(c as u8 - b'A') as usize], true),
        '0'..='9' => (DIGITS[(c as u8 - b'0') as usize], false),
        ' ' => (0x31, false),
        '!' => (0x12, true),
        '@' => (0x13, true),
        '#' => (0x14, true),
        '$' => (0x15, true),
        '%' => (0x17, true),
        '^' => (0x16, true),
        '&' => (0x1A, true),
        '*' => (0x1C, true),
        '(' => (0x19, true),
        ')' => (0x1D, true),
        '-' => (0x1B, false),
        '_' => (0x1B, true),
        '=' => (0x18, false),
        '+' => (0x18, true),
        '[' => (0x21, false),
        '{' => (0x21, true),
        ']' => (0x1E, false),
        '}' => (0x1E, true),
        '\\' => (0x2A, false),
        '|' => (0x2A, true),
        ';' => (0x29, false),
        ':' => (0x29, true),
        '\'' => (0x27, false),
        '"' => (0x27, true),
        ',' => (0x2B, false),
        '<' => (0x2B, true),
        '.' => (0x2F, false),
        '>' => (0x2F, true),
        '/' => (0x2C, false),
        '?' => (0x2C, true),
        '`' => (0x32, false),
        '~' => (0x32, true),
        '\n' => (0x24, false),
        '\t' => (0x30, false),
        _ => return None,
    })
}

/// The virtual key code of a named key and the flag it sets when it is a
/// modifier.
fn key_code_of_named(key: NamedKey) -> Option<(u16, u64)> {
    Some(match key {
        NamedKey::Shift => (0x38, kCGEventFlagMaskShift),
        NamedKey::Control => (0x3B, kCGEventFlagMaskControl),
        NamedKey::Alt => (0x3A, kCGEventFlagMaskAlternate),
        NamedKey::Meta => (0x37, kCGEventFlagMaskCommand),
        NamedKey::Backspace => (0x33, 0),
        NamedKey::Tab => (0x30, 0),
        NamedKey::Return | NamedKey::Enter => (0x24, 0),
        NamedKey::Escape | NamedKey::Cancel => (0x35, 0),
        NamedKey::Space => (0x31, 0),
        NamedKey::PageUp => (0x74, 0),
        NamedKey::PageDown => (0x79, 0),
        NamedKey::End => (0x77, 0),
        NamedKey::Home => (0x73, 0),
        NamedKey::Left => (0x7B, 0),
        NamedKey::Right => (0x7C, 0),
        NamedKey::Down => (0x7D, 0),
        NamedKey::Up => (0x7E, 0),
        NamedKey::Delete => (0x75, 0),
        NamedKey::Help | NamedKey::Insert => (0x72, 0),
        NamedKey::F(n) => {
            const F: [u16; 12] = [0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F];
            (*F.get(usize::from(n).wrapping_sub(1))?, 0)
        }
        NamedKey::Null | NamedKey::Clear | NamedKey::Pause => return None,
    })
}

// ---- the backend -------------------------------------------------------------------------------------

/// The application under test and the input of the desktop.
pub struct Backend {
    child: RefCell<Option<Child>>,
    application: NativeElement,
    source: CGEventSourceRef,
    /// The flags of the modifiers the tests hold down.
    flags: Cell<u64>,
    /// The mouse buttons the tests hold down.
    buttons: RefCell<Vec<MouseButton>>,
}

/// How long the input of a command is given to arrive and be handled before
/// the command returns.
const SETTLE: Duration = Duration::from_millis(250);

impl Backend {
    /// Whether the process may use the accessibility API of other
    /// applications and post input.
    pub fn check() -> Result<(), String> {
        // SAFETY: no arguments.
        if unsafe { AXIsProcessTrusted() } != 0 {
            Ok(())
        } else {
            Err("macOS refuses this process the accessibility API (AXIsProcessTrusted() is false): the process \
                 that runs the tests (the terminal, or the application that started it) needs the Accessibility \
                 permission, System Settings > Privacy & Security > Accessibility; the same permission lets it post \
                 the input of the mouse and the keyboard"
                .to_owned())
        }
    }

    /// Starts the application and waits for its main window.
    pub fn launch(app: &Path, arguments: &[&str]) -> Result<Backend, String> {
        let child = Command::new(app)
            .args(arguments)
            .spawn()
            .map_err(|error| format!("cannot start {}: {error}", app.display()))?;
        let pid = child.id() as i32;
        // SAFETY: a process identifier; the result is owned.
        let application = NativeElement(Owned(unsafe { AXUIElementCreateApplication(pid) }));
        // SAFETY: a live element.
        unsafe { AXUIElementSetMessagingTimeout(application.raw(), 10.0) };
        // SAFETY: the state of the event source of the system.
        let source = unsafe { CGEventSourceCreate(kCGEventSourceStateHIDSystemState) };
        let backend = Backend {
            child: RefCell::new(Some(child)),
            application,
            source,
            flags: Cell::new(0),
            buttons: RefCell::new(Vec::new()),
        };

        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let windows = backend.application.elements("AXWindows");
            if windows.iter().any(|window| window.string("AXIdentifier").as_deref() == Some("MainWindow")) {
                break;
            }
            if let Some(status) = backend.child.borrow_mut().as_mut().and_then(|child| child.try_wait().ok().flatten()) {
                return Err(format!("{} ended before it showed its main window: {status}", app.display()));
            }
            if Instant::now() > deadline {
                return Err(format!("{} showed no main window in 60 s", app.display()));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        // SAFETY: a live value.
        backend.application.set("AXFrontmost", unsafe { kCFBooleanTrue });
        std::thread::sleep(Duration::from_millis(500));
        Ok(backend)
    }

    /// The session of the desktop of Windows; macOS has none.
    pub fn desktop() -> Result<Backend, String> {
        Err("a session of the whole desktop exists on Windows only".to_owned())
    }

    /// Ends the application.
    pub fn quit(&self) {
        if let Some(mut child) = self.child.borrow_mut().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn settle(&self) {
        std::thread::sleep(SETTLE);
    }

    /// The root of a search from the session: the application.
    pub fn session_root(&self, _window: Option<&str>) -> Result<NativeElement, WebDriverError> {
        Ok(self.application.clone())
    }

    /// The windows of the application, by title (macOS has no handles; the
    /// tests use them on Windows only).
    pub fn window_handles(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn activate_window(&self, _handle: &str) {}

    pub fn close_window(&self, _handle: &str) {}

    /// The children of an element; the application also has the menu extras
    /// (its status items), which XCTest shows among its children.
    pub fn children(&self, element: &NativeElement) -> Vec<NativeElement> {
        let mut children = element.elements("AXChildren");
        if element.role() == "AXApplication" {
            if let Some(extras) = element.element("AXExtrasMenuBar") {
                children.push(extras);
            }
        }
        children
    }

    fn parent(&self, element: &NativeElement) -> Option<NativeElement> {
        element.element("AXParent")
    }

    pub fn same(&self, a: &NativeElement, b: &NativeElement) -> bool {
        // SAFETY: two live objects.
        unsafe { CFEqual(a.raw(), b.raw()) != 0 }
    }

    fn in_extras_menu_bar(&self, element: &NativeElement) -> bool {
        let extras = self.application.element("AXExtrasMenuBar");
        let parent = self.parent(element);
        matches!((extras, parent), (Some(extras), Some(parent)) if self.same(&extras, &parent))
    }

    pub fn tag_name(&self, element: &NativeElement) -> String {
        let role = element.role();
        let subrole = element.string("AXSubrole").unwrap_or_default();
        let extras = role == "AXMenuBarItem" && self.in_extras_menu_bar(element);
        element_type(&role, &subrole, extras).to_owned()
    }

    /// `identifier`: the identifier of the element, or the one XCTest gives a
    /// button of a title bar.
    pub fn automation_id(&self, element: &NativeElement) -> String {
        match element.string("AXIdentifier").filter(|id| !id.is_empty()) {
            Some(id) => id,
            None => element
                .string("AXSubrole")
                .and_then(|subrole| xctest_identifier(&subrole))
                .unwrap_or_default()
                .to_owned(),
        }
    }

    fn title(&self, element: &NativeElement) -> String {
        element.string("AXTitle").unwrap_or_default()
    }

    fn label(&self, element: &NativeElement) -> String {
        element.string("AXDescription").unwrap_or_default()
    }

    fn value(&self, element: &NativeElement) -> Option<String> {
        element.copy("AXValue").and_then(|value| text_of(value.0))
    }

    /// The name a search by name compares: the first non-empty of the
    /// identifier, the title and the label.
    pub fn name(&self, element: &NativeElement) -> String {
        [self.automation_id(element), self.title(element), self.label(element)]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or_default()
    }

    pub fn class_name(&self, element: &NativeElement) -> String {
        self.tag_name(element)
    }

    /// The text: the first non-empty of the value, the title and the label.
    pub fn text(&self, element: &NativeElement) -> String {
        [self.value(element).unwrap_or_default(), self.title(element), self.label(element)]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or_default()
    }

    pub fn attribute(&self, element: &NativeElement, name: &str) -> Option<String> {
        let flag = |value: bool| Some(if value { "true" } else { "false" }.to_owned());
        match name {
            "identifier" => Some(self.automation_id(element)),
            "title" => Some(self.title(element)),
            "label" => Some(self.label(element)),
            "value" => self.value(element),
            "placeholderValue" => element.string("AXPlaceholderValue"),
            "elementType" => Some(self.tag_name(element)),
            "enabled" => flag(self.is_enabled(element)),
            "selected" => flag(self.is_selected(element)),
            "focused" => flag(self.has_focus(element)),
            "hittable" => flag(self.is_hittable(element)),
            other => element.copy(other).and_then(|value| text_of(value.0)),
        }
    }

    pub fn is_enabled(&self, element: &NativeElement) -> bool {
        element.boolean("AXEnabled").unwrap_or(true)
    }

    pub fn is_selected(&self, element: &NativeElement) -> bool {
        element.boolean("AXSelected").unwrap_or(false)
    }

    pub fn has_focus(&self, element: &NativeElement) -> bool {
        element.boolean("AXFocused").unwrap_or(false)
    }

    /// Whether a click at the centre of the element would reach it: the
    /// element at that point of the screen is the element or below it, and a
    /// window is not minimized.
    fn is_hittable(&self, element: &NativeElement) -> bool {
        if element.boolean("AXMinimized") == Some(true) {
            return false;
        }
        let (x, y) = self.rect(element).center();
        let mut hit: AXUIElementRef = std::ptr::null();
        // SAFETY: a live element and a place for the owned result.
        let error = unsafe { AXUIElementCopyElementAtPosition(self.application.raw(), x as f32, y as f32, &mut hit) };
        if error != kAXErrorSuccess || hit.is_null() {
            return false;
        }
        let mut current = Some(NativeElement(Owned(hit)));
        for _ in 0..64 {
            match current {
                Some(node) if self.same(&node, element) => return true,
                Some(node) => current = self.parent(&node),
                None => return false,
            }
        }
        false
    }

    pub fn rect(&self, element: &NativeElement) -> ElementRect {
        let (x, y) = element.point("AXPosition").unwrap_or_default();
        let (width, height) = element.size("AXSize").unwrap_or_default();
        ElementRect { x, y, width, height }
    }

    /// The press action of the element (`accessibilityPerformPress`), where
    /// it has one.
    pub fn press(&self, element: &NativeElement) -> bool {
        element.actions().iter().any(|action| action == "AXPress") && element.perform("AXPress")
    }

    pub fn focus(&self, element: &NativeElement) {
        // SAFETY: a live value.
        element.set("AXFocused", unsafe { kCFBooleanTrue });
    }

    pub fn clear(&self, element: &NativeElement) {
        let empty = cf_string("");
        if !element.set("AXValue", empty.0) {
            // An element whose value cannot be set is cleared with the keyboard.
            self.focus(element);
            self.key(Key::Named(NamedKey::Meta), true);
            self.key(Key::Char('a'), true);
            self.key(Key::Char('a'), false);
            self.key(Key::Named(NamedKey::Meta), false);
            self.key(Key::Named(NamedKey::Backspace), true);
            self.key(Key::Named(NamedKey::Backspace), false);
        }
    }

    /// The elements below `root` (and `root` itself with `include_root`)
    /// that the locator finds, in the order of the tree.
    pub fn find_all(&self, root: &NativeElement, include_root: bool, by: &By) -> Vec<NativeElement> {
        let mut found = Vec::new();
        let mut stack: Vec<(NativeElement, bool)> = vec![(root.clone(), include_root)];
        let mut visited = 0usize;
        while let Some((element, check)) = stack.pop() {
            visited += 1;
            if visited > 50_000 {
                break;
            }
            if check && self.matches(&element, by) {
                found.push(element.clone());
            }
            let children = self.children(&element);
            stack.extend(children.into_iter().rev().map(|child| (child, true)));
        }
        found
    }

    fn matches(&self, element: &NativeElement, by: &By) -> bool {
        match by {
            By::AccessibilityId(id) => self.automation_id(element) == *id,
            By::Name(name) => self.name(element) == *name,
            By::ClassName(class) => self.class_name(element) == *class,
            By::TagName(tag) => self.tag_name(element) == *tag,
            By::XPath(_) => false,
        }
    }

    pub fn screenshot(&self, _rect: &ElementRect) -> Result<Screenshot, String> {
        Err(SCREENSHOT_NOT_PROVIDED_ON_MAC_OS.to_owned())
    }

    // ---- input -----------------------------------------------------------------------------------

    fn post(&self, event: CGEventRef) {
        if event.is_null() {
            return;
        }
        // SAFETY: a live event, which is released after it is posted.
        unsafe {
            CGEventSetFlags(event, self.flags.get());
            CGEventPost(kCGHIDEventTap, event);
            CFRelease(event as CFTypeRef);
        }
    }

    pub fn mouse_move(&self, x: f64, y: f64) {
        let buttons = self.buttons.borrow();
        let (kind, button) = if buttons.contains(&MouseButton::Left) {
            (kCGEventLeftMouseDragged, 0)
        } else if buttons.contains(&MouseButton::Right) {
            (kCGEventRightMouseDragged, 1)
        } else {
            (kCGEventMouseMoved, 0)
        };
        // SAFETY: the source of the backend; the event is posted and released.
        self.post(unsafe { CGEventCreateMouseEvent(self.source, kind, CGPoint { x, y }, button) });
    }

    pub fn mouse_button(&self, button: MouseButton, down: bool, x: f64, y: f64, click_count: u32) {
        let (kind, number) = match (button, down) {
            (MouseButton::Left, true) => (kCGEventLeftMouseDown, 0),
            (MouseButton::Left, false) => (kCGEventLeftMouseUp, 0),
            (MouseButton::Right, true) => (kCGEventRightMouseDown, 1),
            (MouseButton::Right, false) => (kCGEventRightMouseUp, 1),
        };
        // SAFETY: the source of the backend; the event is posted and released.
        unsafe {
            let event = CGEventCreateMouseEvent(self.source, kind, CGPoint { x, y }, number);
            if !event.is_null() {
                CGEventSetIntegerValueField(event, kCGMouseEventClickState, i64::from(click_count));
            }
            self.post(event);
        }
        let mut buttons = self.buttons.borrow_mut();
        if down {
            buttons.push(button);
        } else {
            buttons.retain(|b| *b != button);
        }
    }

    pub fn key(&self, key: Key, down: bool) {
        match key {
            Key::Named(named) => {
                let Some((code, flag)) = key_code_of_named(named) else { return };
                if flag != 0 {
                    let flags = if down { self.flags.get() | flag } else { self.flags.get() & !flag };
                    self.flags.set(flags);
                }
                // SAFETY: the source of the backend; the event is posted and released.
                unsafe {
                    let event = CGEventCreateKeyboardEvent(self.source, code, u8::from(down));
                    if flag != 0 && !event.is_null() {
                        CGEventSetType(event, kCGEventFlagsChanged);
                    }
                    self.post(event);
                }
            }
            Key::Char(c) => {
                let (code, shift) = key_code_of_char(c).unwrap_or((0, false));
                let saved = self.flags.get();
                if shift {
                    self.flags.set(saved | kCGEventFlagMaskShift);
                }
                let mut units = [0u16; 2];
                let units = c.encode_utf16(&mut units);
                // SAFETY: the source of the backend; the string lives through
                // the call; the event is posted and released.
                unsafe {
                    let event = CGEventCreateKeyboardEvent(self.source, code, u8::from(down));
                    if !event.is_null() {
                        CGEventKeyboardSetUnicodeString(event, units.len(), units.as_ptr());
                    }
                    self.post(event);
                }
                self.flags.set(saved);
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.quit();
        if !self.source.is_null() {
            // SAFETY: the source this backend created.
            unsafe { CFRelease(self.source as CFTypeRef) };
        }
    }
}

