//! The window of the comparison as the system has it: its handle (found by its title, which
//! names the process), its scale, the work area of its screen, its client size, and a picture of
//! its client area.
//!
//! The picture is the window printing itself (`PrintWindow` with the flag that includes what the
//! desktop window manager composes of it: the content of WinUI, which is composed, is there).
//! Nothing but the window is captured: no part of the desktop or of another window. The system
//! does not draw what of a window lies outside the desktop, so the run places the window inside
//! the work area of its screen first.

use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// `MONITORINFO`.
#[repr(C)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

/// `BITMAPINFOHEADER`, which is all of a `BITMAPINFO` for 32 bits a pixel without compression.
#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pels_per_meter: i32,
    y_pels_per_meter: i32,
    clr_used: u32,
    clr_important: u32,
}

#[link(name = "user32", kind = "raw-dylib")]
extern "system" {
    fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
    fn GetDpiForWindow(hwnd: isize) -> u32;
    fn MonitorFromWindow(hwnd: isize, flags: u32) -> isize;
    fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
    fn GetClientRect(hwnd: isize, rect: *mut Rect) -> i32;
    fn SetWindowPos(hwnd: isize, insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn GetDC(hwnd: isize) -> isize;
    fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
    fn PrintWindow(hwnd: isize, dc: isize, flags: u32) -> i32;
}

#[link(name = "gdi32", kind = "raw-dylib")]
extern "system" {
    fn CreateCompatibleDC(dc: isize) -> isize;
    fn CreateDIBSection(
        dc: isize,
        info: *const BitmapInfoHeader,
        usage: u32,
        bits: *mut *mut c_void,
        section: isize,
        offset: u32,
    ) -> isize;
    fn SelectObject(dc: isize, object: isize) -> isize;
    fn DeleteObject(object: isize) -> i32;
    fn DeleteDC(dc: isize) -> i32;
    fn GdiFlush() -> i32;
}

/// `MONITOR_DEFAULTTONEAREST`.
const MONITOR_DEFAULT_TO_NEAREST: u32 = 2;
/// `SWP_NOZORDER | SWP_NOACTIVATE`.
const SWP_NO_Z_ORDER_NO_ACTIVATE: u32 = 0x0004 | 0x0010;
/// `PW_CLIENTONLY`: the client area, without the frame.
const PW_CLIENTONLY: u32 = 1;
/// `PW_RENDERFULLCONTENT`: with what the desktop window manager composes of the window.
const PW_RENDERFULLCONTENT: u32 = 2;

/// The top-level window with the title `title`, if there is one.
pub fn find_window(title: &str) -> Option<isize> {
    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a null class name and a string of this frame ending with a null.
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    (hwnd != 0).then_some(hwnd)
}

/// The scale of the window: its DPI over 96.
pub fn scale(hwnd: isize) -> f64 {
    // SAFETY: takes a handle; 0 for a handle that is not a window.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        1.0
    } else {
        f64::from(dpi) / 96.0
    }
}

/// The work area of the screen of the window (the screen without the task bar), in pixels.
pub fn work_area(hwnd: isize) -> Option<Rect> {
    let mut info = MonitorInfo { size: std::mem::size_of::<MonitorInfo>() as u32, monitor: Rect::default(), work: Rect::default(), flags: 0 };
    // SAFETY: takes a handle and fills a structure of this frame whose size it is given.
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULT_TO_NEAREST);
        if monitor == 0 || GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
    }
    Some(info.work)
}

/// The client area of the window, in pixels (its origin is 0, 0).
pub fn client_rect(hwnd: isize) -> Option<Rect> {
    let mut rect = Rect::default();
    // SAFETY: fills a rectangle of this frame.
    (unsafe { GetClientRect(hwnd, &mut rect) } != 0).then_some(rect)
}

/// Places the window with its top-left corner at `x`, `y` and a client area of `width` by
/// `height` pixels. Returns the size of the whole window.
pub fn place(hwnd: isize, x: i32, y: i32, width: i32, height: i32) -> Result<(i32, i32), String> {
    let mut window = Rect::default();
    let mut client = Rect::default();
    // SAFETY: fill rectangles of this frame.
    if unsafe { GetWindowRect(hwnd, &mut window) } == 0 || unsafe { GetClientRect(hwnd, &mut client) } == 0 {
        return Err("the rectangles of the window could not be read".to_string());
    }
    let frame_width = width + (window.width() - client.width());
    let frame_height = height + (window.height() - client.height());
    // SAFETY: takes a handle and numbers.
    if unsafe { SetWindowPos(hwnd, 0, x, y, frame_width, frame_height, SWP_NO_Z_ORDER_NO_ACTIVATE) } == 0 {
        return Err("SetWindowPos failed".to_string());
    }
    Ok((frame_width, frame_height))
}

/// The pixels of the client area of a window.
pub struct Capture {
    /// Four bytes a pixel, blue first, the fourth byte 255; rows from the top, `width * 4` bytes
    /// each.
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

/// Captures the client area of the window, or says what failed.
pub fn capture_client_area(hwnd: isize) -> Result<Capture, String> {
    let rect = client_rect(hwnd).ok_or_else(|| "GetClientRect failed".to_string())?;
    let (width, height) = (rect.width(), rect.height());
    if width < 1 || height < 1 {
        return Err(format!("the client area is empty ({width} by {height})"));
    }

    let header = BitmapInfoHeader {
        size: std::mem::size_of::<BitmapInfoHeader>() as u32,
        width,
        // A negative height: the rows run from the top.
        height: -height,
        planes: 1,
        bit_count: 32,
        compression: 0,
        size_image: 0,
        x_pels_per_meter: 0,
        y_pels_per_meter: 0,
        clr_used: 0,
        clr_important: 0,
    };

    // SAFETY: every object created here is released before the function returns. The bitmap
    // section is `width * height * 4` bytes that the system owns until the bitmap is deleted;
    // they are copied out, after the drawing of the system has been flushed, while the bitmap
    // lives.
    unsafe {
        let window_dc = GetDC(hwnd);
        if window_dc == 0 {
            return Err("GetDC failed".to_string());
        }
        let memory_dc = CreateCompatibleDC(window_dc);
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = if memory_dc == 0 { 0 } else { CreateDIBSection(memory_dc, &header, 0, &mut bits, 0, 0) };
        let result = if memory_dc == 0 || bitmap == 0 || bits.is_null() {
            Err("the bitmap of the capture could not be created".to_string())
        } else {
            let previous = SelectObject(memory_dc, bitmap);
            let printed = PrintWindow(hwnd, memory_dc, PW_CLIENTONLY | PW_RENDERFULLCONTENT) != 0;
            GdiFlush();
            let mut pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
            SelectObject(memory_dc, previous);
            if printed {
                // The fourth byte is not written by every kind of drawing.
                for pixel in pixels.chunks_exact_mut(4) {
                    pixel[3] = 0xFF;
                }
                Ok(Capture { pixels, width, height })
            } else {
                Err("PrintWindow failed".to_string())
            }
        };
        if bitmap != 0 {
            DeleteObject(bitmap);
        }
        if memory_dc != 0 {
            DeleteDC(memory_dc);
        }
        ReleaseDC(hwnd, window_dc);
        result
    }
}

/// The number of different colours among a grid of samples of a picture: 1 for a picture of one
/// colour, which is what a capture that shows nothing looks like.
pub fn sampled_colors(pixels: &[u8], width: i32, height: i32) -> usize {
    let mut colors = std::collections::HashSet::new();
    let (step_x, step_y) = ((width / 64).max(1), (height / 64).max(1));
    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            let at = ((y * width + x) * 4) as usize;
            if let Some(bgr) = pixels.get(at..at + 3) {
                colors.insert([bgr[0], bgr[1], bgr[2]]);
            }
            x += step_x;
        }
        y += step_y;
    }
    colors.len()
}
