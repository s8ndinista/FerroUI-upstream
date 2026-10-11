//! A client of UI Automation for runs without a person: it reads the tree
//! of a window and drives its patterns through the client interface of the
//! system (`CUIAutomation`), as a test tool or a screen reader does. The
//! smoke run of the desktop example and the walk of the catalog use it, on
//! a thread of their own.
//!
//! Not from the reference, and no part of what an application uses of the
//! backend. The client interfaces are the ones of `UIAutomationClient.h`;
//! a method is called through its slot of the table of the interface, with
//! the slot numbers of that header (they were compared with the
//! declarations of the `windows` crate, which are generated from the
//! metadata of the Windows SDK).

#![allow(missing_docs)]

use crate::automation::interop::UiaControlTypeId;
use crate::automation::marshalling::{free_bstr, ptr_to_string_bstr, string_to_bstr};
use ferroui_microcom::{ComPtr, Guid, HResult};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize};

#[allow(clippy::all, non_snake_case, non_upper_case_globals, dead_code, unused_unsafe, unsafe_op_in_unsafe_fn)]
mod uia_client {
    include!(concat!(env!("OUT_DIR"), "/uia_client.rs"));
}

use uia_client::{IUIAutomationFocusChangedEventHandler, IUIAutomationFocusChangedEventHandlerImpl};

const CLSID_CUI_AUTOMATION: Guid = Guid::from_u128(0xff48dba4_60ef_4201_aa87_54103eef594e);
const IID_IUI_AUTOMATION: Guid = Guid::from_u128(0x30cbe57d_d9d0_452a_ab13_7ac5ac4825ee);

/// The pattern identifiers the client asks for (`UIA_*PatternId`).
pub const INVOKE_PATTERN: i32 = 10000;
pub const VALUE_PATTERN: i32 = 10002;
pub const RANGE_VALUE_PATTERN: i32 = 10003;
pub const TOGGLE_PATTERN: i32 = 10015;

const IID_INVOKE_PATTERN: Guid = Guid::from_u128(0xfb377fbe_8ea6_46d5_9c73_6499642d3059);
const IID_TOGGLE_PATTERN: Guid = Guid::from_u128(0x94cf8058_9b8d_4ab9_8bfd_4cd0a33c8c70);
const IID_VALUE_PATTERN: Guid = Guid::from_u128(0xa94cd8b1_0844_4cd6_9d2d_640537ab39e9);
const IID_RANGE_VALUE_PATTERN: Guid = Guid::from_u128(0x59213f4f_7346_49e5_b120_80555987a148);

// The slots of the methods the client calls, after the three of `IUnknown`.
mod slots {
    pub const RELEASE: usize = 2;
    // IUIAutomation
    pub const ELEMENT_FROM_HANDLE: usize = 6;
    pub const GET_FOCUSED_ELEMENT: usize = 8;
    pub const RAW_VIEW_WALKER: usize = 16;
    pub const ADD_FOCUS_CHANGED_EVENT_HANDLER: usize = 39;
    pub const REMOVE_ALL_EVENT_HANDLERS: usize = 41;
    // IUIAutomationElement
    pub const SET_FOCUS: usize = 3;
    pub const GET_CURRENT_PATTERN_AS: usize = 14;
    pub const CURRENT_CONTROL_TYPE: usize = 21;
    pub const CURRENT_NAME: usize = 23;
    pub const CURRENT_HAS_KEYBOARD_FOCUS: usize = 26;
    pub const CURRENT_IS_KEYBOARD_FOCUSABLE: usize = 27;
    pub const CURRENT_IS_ENABLED: usize = 28;
    pub const CURRENT_CLASS_NAME: usize = 30;
    pub const CURRENT_FRAMEWORK_ID: usize = 40;
    pub const CURRENT_BOUNDING_RECTANGLE: usize = 43;
    // IUIAutomationTreeWalker
    pub const GET_FIRST_CHILD_ELEMENT: usize = 4;
    pub const GET_NEXT_SIBLING_ELEMENT: usize = 6;
    // The patterns: the first method, and the first property.
    pub const PATTERN_FIRST: usize = 3;
    pub const PATTERN_SECOND: usize = 4;
}

/// The function in a slot of the table of a COM object.
///
/// # Safety
/// `object` is a live interface pointer whose table has a function of the
/// type `F` in the slot `index`.
unsafe fn slot<F: Copy>(object: *mut c_void, index: usize) -> F {
    debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
    // SAFETY: the contract of the function: an interface pointer points at
    // the pointer of its table, which is an array of function pointers.
    unsafe {
        let table = *object.cast::<*const *const c_void>();
        std::mem::transmute_copy::<*const c_void, F>(&*table.add(index))
    }
}

fn check(result: i32, what: &str) -> Result<(), String> {
    if result >= 0 {
        Ok(())
    } else {
        Err(format!("{what}: {}", HResult(result as u32)))
    }
}

type Method0 = unsafe extern "system" fn(*mut c_void) -> i32;
type Getter<T> = unsafe extern "system" fn(*mut c_void, *mut T) -> i32;

/// An interface pointer the client owns one reference of.
struct Owned(*mut c_void);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: a live interface pointer with a reference of this value.
        unsafe { slot::<unsafe extern "system" fn(*mut c_void) -> u32>(self.0, slots::RELEASE)(self.0) };
    }
}

impl Owned {
    /// A property of the object that is a number.
    fn number<T: Copy + Default>(&self, index: usize, what: &str) -> Result<T, String> {
        let mut value = T::default();
        // SAFETY: the slot of a property of the type `T` of this interface.
        check(unsafe { slot::<Getter<T>>(self.0, index)(self.0, &mut value) }, what)?;
        Ok(value)
    }

    /// A property of the object that is a string of the system.
    fn text(&self, index: usize, what: &str) -> Result<String, String> {
        let mut value: *mut u16 = std::ptr::null_mut();
        // SAFETY: the slot of a string property of this interface; the
        // string is owned by the caller, who frees it.
        unsafe {
            check(slot::<Getter<*mut u16>>(self.0, index)(self.0, &mut value), what)?;
            let text = ptr_to_string_bstr(value).unwrap_or_default();
            free_bstr(value);
            Ok(text)
        }
    }

    fn call(&self, index: usize, what: &str) -> Result<(), String> {
        // SAFETY: the slot of a method without arguments of this interface.
        check(unsafe { slot::<Method0>(self.0, index)(self.0) }, what)
    }
}

/// An element of the tree, as the client holds it.
pub struct Element(Owned);

/// What the client reads of an element.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementInfo {
    pub depth: usize,
    pub control_type: i32,
    pub name: String,
    pub class_name: String,
    pub framework_id: String,
    pub is_enabled: bool,
    pub is_keyboard_focusable: bool,
    /// Left, top, right, bottom, in pixels of the screen.
    pub bounding_rectangle: [i32; 4],
}

impl ElementInfo {
    /// The name of the control type (`Button` for 50000).
    pub fn control_type_name(&self) -> String {
        control_type_name(self.control_type)
    }
}

/// The name of a control type of the system.
pub fn control_type_name(control_type: i32) -> String {
    format!("{:?}", UiaControlTypeId(control_type))
}

impl Element {
    pub fn name(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_NAME, "IUIAutomationElement::get_CurrentName")
    }

    pub fn class_name(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_CLASS_NAME, "IUIAutomationElement::get_CurrentClassName")
    }

    pub fn framework_id(&self) -> Result<String, String> {
        self.0.text(slots::CURRENT_FRAMEWORK_ID, "IUIAutomationElement::get_CurrentFrameworkId")
    }

    pub fn control_type(&self) -> Result<i32, String> {
        self.0.number(slots::CURRENT_CONTROL_TYPE, "IUIAutomationElement::get_CurrentControlType")
    }

    pub fn is_enabled(&self) -> Result<bool, String> {
        Ok(self.0.number::<i32>(slots::CURRENT_IS_ENABLED, "IUIAutomationElement::get_CurrentIsEnabled")? != 0)
    }

    pub fn is_keyboard_focusable(&self) -> Result<bool, String> {
        let value: i32 =
            self.0.number(slots::CURRENT_IS_KEYBOARD_FOCUSABLE, "IUIAutomationElement::get_CurrentIsKeyboardFocusable")?;
        Ok(value != 0)
    }

    pub fn has_keyboard_focus(&self) -> Result<bool, String> {
        let value: i32 =
            self.0.number(slots::CURRENT_HAS_KEYBOARD_FOCUS, "IUIAutomationElement::get_CurrentHasKeyboardFocus")?;
        Ok(value != 0)
    }

    /// Left, top, right, bottom.
    pub fn bounding_rectangle(&self) -> Result<[i32; 4], String> {
        self.0.number(slots::CURRENT_BOUNDING_RECTANGLE, "IUIAutomationElement::get_CurrentBoundingRectangle")
    }

    pub fn set_focus(&self) -> Result<(), String> {
        self.0.call(slots::SET_FOCUS, "IUIAutomationElement::SetFocus")
    }

    pub fn info(&self, depth: usize) -> Result<ElementInfo, String> {
        Ok(ElementInfo {
            depth,
            control_type: self.control_type()?,
            name: self.name()?,
            class_name: self.class_name()?,
            framework_id: self.framework_id()?,
            is_enabled: self.is_enabled()?,
            is_keyboard_focusable: self.is_keyboard_focusable()?,
            bounding_rectangle: self.bounding_rectangle()?,
        })
    }

    /// The pattern of the element as its client interface; `None` when the
    /// element does not have the pattern.
    fn pattern(&self, pattern_id: i32, iid: &Guid, what: &str) -> Result<Option<Owned>, String> {
        type GetCurrentPatternAs = unsafe extern "system" fn(*mut c_void, i32, *const Guid, *mut *mut c_void) -> i32;
        let mut pattern = std::ptr::null_mut();
        // SAFETY: the slot of `GetCurrentPatternAs`, an identifier that
        // lives through the call and a place for the result.
        check(
            unsafe {
                slot::<GetCurrentPatternAs>(self.0 .0, slots::GET_CURRENT_PATTERN_AS)(self.0 .0, pattern_id, iid, &mut pattern)
            },
            what,
        )?;
        Ok((!pattern.is_null()).then(|| Owned(pattern)))
    }

    fn required_pattern(&self, pattern_id: i32, iid: &Guid, what: &str) -> Result<Owned, String> {
        self.pattern(pattern_id, iid, what)?.ok_or_else(|| format!("{what}: the element does not have the pattern"))
    }

    /// Whether the element has the pattern (one of the four of this
    /// module).
    pub fn has_pattern(&self, pattern_id: i32) -> Result<bool, String> {
        let iid = match pattern_id {
            INVOKE_PATTERN => &IID_INVOKE_PATTERN,
            VALUE_PATTERN => &IID_VALUE_PATTERN,
            RANGE_VALUE_PATTERN => &IID_RANGE_VALUE_PATTERN,
            TOGGLE_PATTERN => &IID_TOGGLE_PATTERN,
            other => return Err(format!("the pattern {other} is not one the client asks for")),
        };
        Ok(self.pattern(pattern_id, iid, "IUIAutomationElement::GetCurrentPatternAs")?.is_some())
    }

    /// `IUIAutomationInvokePattern::Invoke`.
    pub fn invoke(&self) -> Result<(), String> {
        self.required_pattern(INVOKE_PATTERN, &IID_INVOKE_PATTERN, "the invoke pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationInvokePattern::Invoke")
    }

    /// `IUIAutomationTogglePattern::Toggle`.
    pub fn toggle(&self) -> Result<(), String> {
        self.required_pattern(TOGGLE_PATTERN, &IID_TOGGLE_PATTERN, "the toggle pattern")?
            .call(slots::PATTERN_FIRST, "IUIAutomationTogglePattern::Toggle")
    }

    /// The toggle state: 0 off, 1 on, 2 indeterminate.
    pub fn toggle_state(&self) -> Result<i32, String> {
        self.required_pattern(TOGGLE_PATTERN, &IID_TOGGLE_PATTERN, "the toggle pattern")?
            .number(slots::PATTERN_SECOND, "IUIAutomationTogglePattern::get_CurrentToggleState")
    }

    /// `IUIAutomationValuePattern::SetValue`.
    pub fn set_value(&self, value: &str) -> Result<(), String> {
        let pattern = self.required_pattern(VALUE_PATTERN, &IID_VALUE_PATTERN, "the value pattern")?;
        let bstr = string_to_bstr(value);
        // SAFETY: the slot of `SetValue`, which takes a string of the
        // system; the string is freed after the call.
        unsafe {
            let result =
                slot::<unsafe extern "system" fn(*mut c_void, *mut u16) -> i32>(pattern.0, slots::PATTERN_FIRST)(pattern.0, bstr);
            free_bstr(bstr);
            check(result, "IUIAutomationValuePattern::SetValue")
        }
    }

    pub fn value(&self) -> Result<String, String> {
        self.required_pattern(VALUE_PATTERN, &IID_VALUE_PATTERN, "the value pattern")?
            .text(slots::PATTERN_SECOND, "IUIAutomationValuePattern::get_CurrentValue")
    }

    /// `IUIAutomationRangeValuePattern::SetValue`.
    pub fn set_range_value(&self, value: f64) -> Result<(), String> {
        let pattern = self.required_pattern(RANGE_VALUE_PATTERN, &IID_RANGE_VALUE_PATTERN, "the range value pattern")?;
        // SAFETY: the slot of `SetValue`, which takes a double.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, f64) -> i32>(pattern.0, slots::PATTERN_FIRST)(pattern.0, value)
            },
            "IUIAutomationRangeValuePattern::SetValue",
        )
    }

    pub fn range_value(&self) -> Result<f64, String> {
        self.required_pattern(RANGE_VALUE_PATTERN, &IID_RANGE_VALUE_PATTERN, "the range value pattern")?
            .number(slots::PATTERN_SECOND, "IUIAutomationRangeValuePattern::get_CurrentValue")
    }
}

/// The elements that got the focus, as the handler was told of them: interface pointers the
/// log holds a reference of, until the client reads them.
///
/// The handler does not ask an element anything. It is called by the automation core while
/// the provider that raised the event may still be inside the call that raised it, on its UI
/// thread; a question to the element is a call of the provider, which waits for that thread
/// (run 38101124412 did not end after the first event of a provider of the process of the
/// client). The client asks later, on its own thread ([`UiaClient::take_focus_events`]).
#[derive(Clone, Default)]
pub struct FocusLog {
    elements: Arc<Mutex<Vec<usize>>>,
}

impl FocusLog {
    pub fn new() -> FocusLog {
        FocusLog::default()
    }
}

struct FocusChangedHandler {
    log: FocusLog,
}

impl IUIAutomationFocusChangedEventHandlerImpl for FocusChangedHandler {
    fn handle_focus_changed_event(&self, sender: *mut c_void) -> Result<(), HResult> {
        if sender.is_null() {
            return Ok(());
        }
        // SAFETY: a live interface pointer of the caller; the log takes a reference of it
        // (`AddRef`, the second slot), which the client gives back when it has read it.
        unsafe { slot::<unsafe extern "system" fn(*mut c_void) -> u32>(sender, 1)(sender) };
        self.log.elements.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(sender as usize);
        Ok(())
    }
}

/// The client: COM on the calling thread (the multithreaded apartment, so
/// that the handlers of events are called without a message loop), the
/// automation object and its walker of the raw view. To be made, used and
/// dropped on one thread that is not the UI thread of the windows it reads:
/// a call waits for that thread to answer.
pub struct UiaClient {
    automation: Owned,
    walker: Owned,
    handlers: Vec<ComPtr<IUIAutomationFocusChangedEventHandler>>,
}

impl UiaClient {
    pub fn new() -> Result<UiaClient, String> {
        // SAFETY: the reserved argument is null; the flag is
        // `COINIT_MULTITHREADED`. The thread keeps COM until it ends.
        check(unsafe { CoInitializeEx(std::ptr::null(), 0) }, "CoInitializeEx")?;
        let mut automation = std::ptr::null_mut();
        // SAFETY: two identifiers that live through the call, no outer
        // object, an in-process server (`CLSCTX_INPROC_SERVER`), and a
        // place for the result.
        check(
            unsafe {
                CoCreateInstance(
                    (&CLSID_CUI_AUTOMATION as *const Guid).cast(),
                    std::ptr::null_mut(),
                    1,
                    (&IID_IUI_AUTOMATION as *const Guid).cast(),
                    &mut automation,
                )
            },
            "CoCreateInstance(CUIAutomation)",
        )?;
        let automation = Owned(automation);
        let mut walker = std::ptr::null_mut();
        // SAFETY: the slot of `get_RawViewWalker`.
        check(
            unsafe { slot::<Getter<*mut c_void>>(automation.0, slots::RAW_VIEW_WALKER)(automation.0, &mut walker) },
            "IUIAutomation::get_RawViewWalker",
        )?;
        Ok(UiaClient { automation, walker: Owned(walker), handlers: Vec::new() })
    }

    /// `IUIAutomation::ElementFromHandle`.
    pub fn element_from_handle(&self, hwnd: isize) -> Result<Element, String> {
        let mut element = std::ptr::null_mut();
        // SAFETY: the slot of `ElementFromHandle`, a window handle and a
        // place for the result.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, isize, *mut *mut c_void) -> i32>(
                    self.automation.0,
                    slots::ELEMENT_FROM_HANDLE,
                )(self.automation.0, hwnd, &mut element)
            },
            "IUIAutomation::ElementFromHandle",
        )?;
        if element.is_null() {
            return Err("IUIAutomation::ElementFromHandle: no element".to_owned());
        }
        Ok(Element(Owned(element)))
    }

    /// `IUIAutomation::GetFocusedElement`.
    pub fn focused_element(&self) -> Result<Option<Element>, String> {
        let mut element = std::ptr::null_mut();
        // SAFETY: the slot of `GetFocusedElement`.
        check(
            unsafe {
                slot::<Getter<*mut c_void>>(self.automation.0, slots::GET_FOCUSED_ELEMENT)(self.automation.0, &mut element)
            },
            "IUIAutomation::GetFocusedElement",
        )?;
        Ok((!element.is_null()).then(|| Element(Owned(element))))
    }

    fn step(&self, element: &Element, index: usize, what: &str) -> Result<Option<Element>, String> {
        let mut found = std::ptr::null_mut();
        // SAFETY: a slot of the walker that takes an element and a place
        // for the element it leads to.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> i32>(self.walker.0, index)(
                    self.walker.0,
                    element.0 .0,
                    &mut found,
                )
            },
            what,
        )?;
        Ok((!found.is_null()).then(|| Element(Owned(found))))
    }

    /// The children of an element in the raw view.
    pub fn children(&self, element: &Element) -> Result<Vec<Element>, String> {
        let mut result = Vec::new();
        let mut child = self.step(element, slots::GET_FIRST_CHILD_ELEMENT, "IUIAutomationTreeWalker::GetFirstChildElement")?;
        while let Some(current) = child {
            child = self.step(&current, slots::GET_NEXT_SIBLING_ELEMENT, "IUIAutomationTreeWalker::GetNextSiblingElement")?;
            result.push(current);
        }
        Ok(result)
    }

    /// The elements below `root`, depth first, each with its depth (the
    /// children of the root are at depth one).
    pub fn descendants(&self, root: &Element) -> Result<Vec<(usize, Element)>, String> {
        fn walk(client: &UiaClient, element: &Element, depth: usize, result: &mut Vec<(usize, Element)>) -> Result<(), String> {
            for child in client.children(element)? {
                let index = result.len();
                walk(client, &child, depth + 1, result)?;
                result.insert(index, (depth, child));
            }
            Ok(())
        }
        let mut result = Vec::new();
        walk(self, root, 1, &mut result)?;
        Ok(result)
    }

    /// The control type and the name of every element the log was told of since the last call,
    /// read now, on the thread of the client.
    pub fn take_focus_events(&self, log: &FocusLog) -> Vec<String> {
        let elements = std::mem::take(&mut *log.elements.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        elements
            .into_iter()
            .map(|element| {
                // The reference the handler took becomes the element's.
                let element = Element(Owned(element as *mut c_void));
                let name = element.name().unwrap_or_else(|error| format!("<{error}>"));
                let control_type = element.control_type().map(control_type_name).unwrap_or_default();
                format!("{control_type} {name:?}")
            })
            .collect()
    }

    /// Listens for changes of the focus: the log gets every element the
    /// system reports.
    pub fn add_focus_changed_event_handler(&mut self, log: FocusLog) -> Result<(), String> {
        let handler = IUIAutomationFocusChangedEventHandler::from_impl(FocusChangedHandler { log });
        // SAFETY: the slot of `AddFocusChangedEventHandler`, no cache
        // request and a live handler, of which the system takes a
        // reference.
        check(
            unsafe {
                slot::<unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> i32>(
                    self.automation.0,
                    slots::ADD_FOCUS_CHANGED_EVENT_HANDLER,
                )(self.automation.0, std::ptr::null_mut(), handler.as_ptr().cast())
            },
            "IUIAutomation::AddFocusChangedEventHandler",
        )?;
        self.handlers.push(handler);
        Ok(())
    }
}

impl Drop for UiaClient {
    fn drop(&mut self) {
        if !self.handlers.is_empty() {
            let _ = self.automation.call(slots::REMOVE_ALL_EVENT_HANDLERS, "IUIAutomation::RemoveAllEventHandlers");
        }
    }
}

/// The window of the desktop that has the foreground; 0 when none has.
pub fn foreground_window() -> isize {
    // SAFETY: no arguments.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize }
}

/// Ends COM on the calling thread, after every value of this module of the
/// thread was dropped.
pub fn uninitialize_com() {
    // SAFETY: pairs the initialisation of `UiaClient::new` on this thread;
    // the caller has dropped the client and its elements.
    unsafe { CoUninitialize() }
}

/// The count of the elements of each control type, most frequent first.
pub fn count_by_control_type(elements: &[ElementInfo]) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for element in elements {
        *counts.entry(element.control_type_name()).or_default() += 1;
    }
    let mut counts: Vec<(String, usize)> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}
