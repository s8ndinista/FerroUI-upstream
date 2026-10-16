//! Port of `Embedding/WinApi.cs`: the calls of the Windows API the native text
//! box of the Windows platform makes, behind safe names (the declarations are
//! the ones of `windows-sys`).

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TRACKMOUSEEVENT};
use windows_sys::Win32::UI::WindowsAndMessaging as wm;

pub const GWL_WNDPROC: i32 = -4;
pub const TME_HOVER: u32 = 1;
pub const TME_LEAVE: u32 = 2;
pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const WM_MOUSEHOVER: u32 = 0x02A1;
pub const WM_MOUSEMOVE: u32 = 0x0200;

pub const WS_BORDER: u32 = 0x0080_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;

/// `WndProcDelegate`.
pub type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `CreateWindowEx` of a child window of the class `class_name`; null when the
/// system refuses.
pub fn create_window_ex(ex_style: u32, class_name: &str, window_name: &str, style: u32, parent: HWND) -> HWND {
    let class_name = wide(class_name);
    let window_name = wide(window_name);
    // SAFETY: two NUL-terminated strings that live through the call, a parent
    // window, no menu, the module of the process and no creation data.
    unsafe {
        wm::CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            window_name.as_ptr(),
            style,
            0,
            0,
            1,
            1,
            parent,
            std::ptr::null_mut(),
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    }
}

/// `SetWindowLongPtr(handle, GWL_WNDPROC, procedure)`: the procedure the window
/// had.
///
/// # Safety
/// `procedure` handles every message of the window and passes what it does not
/// handle to the procedure this returns.
pub unsafe fn set_window_procedure(handle: HWND, procedure: WndProc) -> isize {
    // SAFETY: the contract of the function.
    unsafe { wm::SetWindowLongPtrW(handle, GWL_WNDPROC, procedure as usize as isize) }
}

/// `CallWindowProc`.
///
/// # Safety
/// `previous` is the procedure `set_window_procedure` returned for the window.
pub unsafe fn call_window_proc(previous: isize, handle: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    // SAFETY: the contract of the function: a window procedure of the system or
    // of a class.
    unsafe {
        let previous: WndProc = std::mem::transmute::<isize, WndProc>(previous);
        wm::CallWindowProcW(Some(previous), handle, msg, w_param, l_param)
    }
}

/// `DestroyWindow`.
pub fn destroy_window(handle: HWND) -> bool {
    // SAFETY: a window handle; a stale one makes the call fail.
    unsafe { wm::DestroyWindow(handle) != 0 }
}

/// `GetWindowText` with a buffer of `capacity` characters.
pub fn get_window_text(handle: HWND, capacity: usize) -> String {
    let mut buffer = vec![0u16; capacity];
    // SAFETY: a buffer of the given capacity.
    let length = unsafe { wm::GetWindowTextW(handle, buffer.as_mut_ptr(), capacity as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

/// `SetWindowText`.
pub fn set_window_text(handle: HWND, text: &str) -> bool {
    let text = wide(text);
    // SAFETY: a NUL-terminated string that lives through the call.
    unsafe { wm::SetWindowTextW(handle, text.as_ptr()) != 0 }
}

/// `TrackMouseEvent`.
pub fn track_mouse_event(event: &mut TRACKMOUSEEVENT) -> bool {
    // SAFETY: a structure of the size its first member states.
    unsafe { TrackMouseEvent(event) != 0 }
}
