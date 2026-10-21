//! Characterization of the system MSFTEDIT engine, not WinUI's text engine.
//!
//! This probe prints native HomeKey/EndKey and MoveUp/MoveDown destinations,
//! active-end flags, HRESULTs and signed deltas. Its assertions validate setup and unambiguous
//! invariants; the first-run destination/count observations are not Rust parity
//! assertions. Keep the engine version with any fixture derived from its output.
//!
//! ABI and constants verified against Microsoft's Windows SDK projections:
//! https://github.com/microsoft/windows-rs/blob/32c3144490c016fe496a0aed769bce60987a2e9d/crates/libs/windows/src/Windows/Win32/UI/Controls/RichEdit/mod.rs
//! https://learn.microsoft.com/windows/win32/controls/em-getoleinterface
//! https://learn.microsoft.com/windows/win32/controls/text-object-model
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-getflags
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-homekey
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-endkey
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-moveup
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-movedown

use crate::automation::marshalling::{free_bstr, ptr_to_string_bstr};
use crate::interop::unmanaged_methods::rtl_get_version;
use ferroui_microcom::{ComPtr, Guid, IUnknown, IUnknownVtbl};
use std::ffi::c_void;
use std::mem::{offset_of, size_of};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, HMODULE, HWND};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleW, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
};
use windows_sys::Win32::System::Ole::{OleInitialize, OleUninitialize};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SendMessageW, SetWindowTextW, ES_AUTOHSCROLL, ES_AUTOVSCROLL,
    ES_MULTILINE, WS_POPUP,
};

type TestResult<T = ()> = Result<T, String>;
type Getter = unsafe extern "system" fn(*mut c_void, *mut i32) -> i32;
type BoundaryMethod = unsafe extern "system" fn(*mut c_void, i32, i32, *mut i32) -> i32;
type MoveMethod = unsafe extern "system" fn(*mut c_void, i32, i32, i32, *mut i32) -> i32;

const EM_GETOLEINTERFACE: u32 = 1084;
const IID_TEXT_DOCUMENT: Guid = Guid::from_u128(0x8cc497c0_a1df_11ce_8098_00aa0047be5d);
const TOM_SEL_START_ACTIVE: i32 = 1;

// Typed prefixes of the SDK's dual interfaces. IDispatch contributes four
// slots after IUnknown; ITextRange contributes 51 more before ITextSelection.
// Uncalled slots retain their ABI width without declaring unused interfaces.
#[repr(C)]
struct TextDocumentVTable {
    _unknown: IUnknownVtbl,
    _dispatch: [*const c_void; 4],
    _get_name: *const c_void,
    get_selection: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
}

#[repr(C)]
struct TextSelectionVTable {
    _unknown: IUnknownVtbl,
    _dispatch: [*const c_void; 4],
    get_text: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32,
    _set_text_through_set_formatted_text: [*const c_void; 6],
    get_start: Getter,
    _set_start: *const c_void,
    get_end: Getter,
    _set_end: *const c_void,
    _font_and_paragraph: [*const c_void; 4],
    get_story_length: Getter,
    _get_story_type_through_set_index: [*const c_void; 5],
    set_range: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    _in_range_through_get_embedded_object: [*const c_void; 29],
    get_flags: Getter,
    _set_flags: *const c_void,
    _get_type_and_horizontal_movement: [*const c_void; 3],
    move_up: MoveMethod,
    move_down: MoveMethod,
    home_key: BoundaryMethod,
    end_key: BoundaryMethod,
}

const _: () = {
    let slot = size_of::<*const c_void>();
    assert!(offset_of!(TextDocumentVTable, get_selection) == 8 * slot);
    assert!(offset_of!(TextSelectionVTable, get_text) == 7 * slot);
    assert!(offset_of!(TextSelectionVTable, get_start) == 14 * slot);
    assert!(offset_of!(TextSelectionVTable, get_end) == 16 * slot);
    assert!(offset_of!(TextSelectionVTable, get_story_length) == 22 * slot);
    assert!(offset_of!(TextSelectionVTable, set_range) == 28 * slot);
    assert!(offset_of!(TextSelectionVTable, get_flags) == 58 * slot);
    assert!(offset_of!(TextSelectionVTable, move_up) == 63 * slot);
    assert!(offset_of!(TextSelectionVTable, move_down) == 64 * slot);
    assert!(offset_of!(TextSelectionVTable, home_key) == 65 * slot);
    assert!(offset_of!(TextSelectionVTable, end_key) == 66 * slot);
};

fn check_hresult(result: i32, operation: &str) -> TestResult {
    if result < 0 {
        Err(format!("{operation}: HRESULT 0x{:08X}", result as u32))
    } else {
        Ok(())
    }
}

fn last_error(operation: &str) -> String {
    // SAFETY: GetLastError reads this thread's last-error value.
    let error = unsafe { GetLastError() };
    format!("{operation}: Win32 error {error}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

struct OleApartment;

impl OleApartment {
    fn new() -> TestResult<Self> {
        // SAFETY: the reserved argument is null; the probe owns its thread.
        check_hresult(unsafe { OleInitialize(null()) }, "OleInitialize")?;
        Ok(Self)
    }
}

impl Drop for OleApartment {
    fn drop(&mut self) {
        // SAFETY: exactly balances this thread's successful OleInitialize.
        unsafe { OleUninitialize() };
    }
}

struct SystemRichEditLibrary(HMODULE);

impl SystemRichEditLibrary {
    fn load() -> TestResult<Self> {
        // SAFETY: a terminated name, null reserved handle, documented search flag.
        let module = unsafe {
            LoadLibraryExW(
                wide("Msftedit.dll").as_ptr(),
                null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if module.is_null() {
            Err(last_error("LoadLibraryExW(Msftedit.dll, System32)"))
        } else {
            Ok(Self(module))
        }
    }

    fn describe(&self) -> TestResult {
        let mut path = vec![0u16; 32768];
        // SAFETY: a loaded module and a writable UTF-16 buffer of the given size.
        let count = unsafe { GetModuleFileNameW(self.0, path.as_mut_ptr(), path.len() as u32) };
        if count == 0 {
            return Err(last_error("GetModuleFileNameW(Msftedit.dll)"));
        }
        if count as usize >= path.len() {
            return Err("Msftedit.dll module path was truncated".into());
        }
        path.truncate(count as usize + 1);
        let version = file_version(&path)?;
        println!(
            "SYSTEM_MSFTEDIT_ENGINE windows={:?} dll={:?} file_version={}.{}.{}.{}",
            rtl_get_version(),
            String::from_utf16_lossy(&path[..count as usize]),
            version.0,
            version.1,
            version.2,
            version.3,
        );
        Ok(())
    }
}

impl Drop for SystemRichEditLibrary {
    fn drop(&mut self) {
        // SAFETY: this owns one LoadLibraryExW reference; all controls are gone.
        unsafe { FreeLibrary(self.0) };
    }
}

fn file_version(path: &[u16]) -> TestResult<(u32, u32, u32, u32)> {
    let mut ignored = 0;
    // SAFETY: path is the terminated module filename returned above.
    let size = unsafe { GetFileVersionInfoSizeW(path.as_ptr(), &mut ignored) };
    if size == 0 {
        return Err(last_error("GetFileVersionInfoSizeW(Msftedit.dll)"));
    }
    // A u32 buffer supplies the alignment of VS_FIXEDFILEINFO as well as its
    // storage. The API's byte count remains the exact requested size.
    let mut data = vec![0u32; (size as usize).div_ceil(size_of::<u32>())];
    // SAFETY: a terminated path and writable storage for at least size bytes.
    if unsafe { GetFileVersionInfoW(path.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return Err(last_error("GetFileVersionInfoW(Msftedit.dll)"));
    }
    let mut root = null_mut();
    let mut length = 0;
    // SAFETY: a successful version-info block and valid output parameters.
    if unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            wide("\\").as_ptr(),
            &mut root,
            &mut length,
        )
    } == 0
        || root.is_null()
        || length < size_of::<VS_FIXEDFILEINFO>() as u32
    {
        return Err("Msftedit.dll has no complete fixed version information".into());
    }
    // SAFETY: VerQueryValueW returned this structure inside the still-live block.
    let info = unsafe { root.cast::<VS_FIXEDFILEINFO>().read_unaligned() };
    if info.dwSignature != 0xfeef04bd {
        return Err("Msftedit.dll fixed version information has an invalid signature".into());
    }
    Ok((
        info.dwFileVersionMS >> 16,
        info.dwFileVersionMS & 0xffff,
        info.dwFileVersionLS >> 16,
        info.dwFileVersionLS & 0xffff,
    ))
}

struct HiddenRichEdit(HWND);

impl HiddenRichEdit {
    fn new() -> TestResult<Self> {
        // SAFETY: null names the current executable module and does not AddRef it.
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return Err(last_error("GetModuleHandleW"));
        }
        // SAFETY: MSFTEDIT registered its class when the DLL was loaded. All
        // strings remain live through this synchronous call. The unshown popup
        // has no parent, menu or creation data. AutoHScroll excludes soft wraps.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                wide("RICHEDIT50W").as_ptr(),
                wide("").as_ptr(),
                WS_POPUP | ES_MULTILINE as u32 | ES_AUTOHSCROLL as u32 | ES_AUTOVSCROLL as u32,
                0,
                0,
                1000,
                500,
                null_mut(),
                null_mut(),
                instance,
                null(),
            )
        };
        if hwnd.is_null() {
            Err(last_error("CreateWindowExW(RICHEDIT50W)"))
        } else {
            Ok(Self(hwnd))
        }
    }

    fn set_text(&self, text: &str) -> TestResult {
        // SAFETY: a live window and a terminated string for this synchronous call.
        if unsafe { SetWindowTextW(self.0, wide(text).as_ptr()) } == 0 {
            Err(last_error("SetWindowTextW(RICHEDIT50W)"))
        } else {
            Ok(())
        }
    }

    fn selection(&self) -> TestResult<NativeSelection> {
        let mut ole = null_mut::<c_void>();
        // SAFETY: EM_GETOLEINTERFACE writes one owned interface reference to ole.
        if unsafe {
            SendMessageW(
                self.0,
                EM_GETOLEINTERFACE,
                0,
                (&mut ole as *mut *mut c_void) as isize,
            )
        } == 0
        {
            return Err("EM_GETOLEINTERFACE failed".into());
        }
        // SAFETY: the successful message returns one AddRef'd IRichEditOle,
        // whose first interface slots are IUnknown.
        let ole = unsafe { ComPtr::<IUnknown>::from_raw(ole.cast()) }
            .ok_or("EM_GETOLEINTERFACE returned null")?;
        let mut document = null_mut();
        // SAFETY: an exact SDK IID and a writable interface-pointer output.
        let result = unsafe { ole.query_interface_raw(&IID_TEXT_DOCUMENT, &mut document) };
        check_hresult(result as i32, "QueryInterface(ITextDocument)")?;
        // SAFETY: successful QueryInterface transfers one owned reference.
        let document = unsafe { ComPtr::<IUnknown>::from_raw(document.cast()) }
            .ok_or("QueryInterface(ITextDocument) returned null")?;
        // SAFETY: QueryInterface established the SDK ITextDocument vtable prefix.
        let table = unsafe { &**document.as_ptr().cast::<*const TextDocumentVTable>() };
        let mut selection = null_mut();
        // SAFETY: GetSelection is slot 8 of ITextDocument and writes an owned
        // ITextSelection reference. The document stays alive through the call.
        check_hresult(
            unsafe { (table.get_selection)(document.as_ptr().cast(), &mut selection) },
            "ITextDocument::GetSelection",
        )?;
        // SAFETY: successful GetSelection transfers one owned reference. The
        // returned selection and the window keep the underlying story alive.
        unsafe { ComPtr::<IUnknown>::from_raw(selection.cast()) }
            .map(NativeSelection)
            .ok_or_else(|| "ITextDocument::GetSelection returned null".into())
    }
}

impl Drop for HiddenRichEdit {
    fn drop(&mut self) {
        // SAFETY: the window was created and is destroyed on the probe thread.
        unsafe { DestroyWindow(self.0) };
    }
}

struct Bstr(*mut u16);

impl Drop for Bstr {
    fn drop(&mut self) {
        // SAFETY: this owns the BSTR returned by GetText; null is also allowed.
        unsafe { free_bstr(self.0) };
    }
}

struct NativeSelection(ComPtr<IUnknown>);

impl NativeSelection {
    fn table(&self) -> &TextSelectionVTable {
        // SAFETY: GetSelection returned this owned ITextSelection reference,
        // whose SDK vtable prefix is checked above. It outlives this borrow.
        unsafe { &**self.0.as_ptr().cast::<*const TextSelectionVTable>() }
    }

    fn number(&self, getter: Getter, name: &str) -> TestResult<i32> {
        let mut value = 0;
        // SAFETY: callers pass a typed integer getter from this interface's table.
        check_hresult(unsafe { getter(self.0.as_ptr().cast(), &mut value) }, name)?;
        Ok(value)
    }

    fn state(&self) -> TestResult<SelectionState> {
        let table = self.table();
        Ok(SelectionState {
            start: self.number(table.get_start, "ITextRange::GetStart")?,
            end: self.number(table.get_end, "ITextRange::GetEnd")?,
            flags: self.number(table.get_flags, "ITextSelection::GetFlags")?,
        })
    }

    fn set_range(&self, anchor: i32, active: i32) -> TestResult {
        // SAFETY: the SDK SetRange ABI accepts two signed character positions.
        check_hresult(
            unsafe { (self.table().set_range)(self.0.as_ptr().cast(), anchor, active) },
            "ITextRange::SetRange",
        )
    }

    fn text(&self) -> TestResult<String> {
        let mut text = Bstr(null_mut());
        // SAFETY: GetText writes an owned BSTR; the guard frees it on all exits.
        check_hresult(
            unsafe { (self.table().get_text)(self.0.as_ptr().cast(), &mut text.0) },
            "ITextRange::GetText",
        )?;
        // SAFETY: the successful getter returned a live BSTR or null for empty.
        Ok(unsafe { ptr_to_string_bstr(text.0) }.unwrap_or_default())
    }

    fn navigate(&self, unit: Unit, extend: bool, end: bool) -> TestResult<(i32, i32)> {
        let method = if end {
            self.table().end_key
        } else {
            self.table().home_key
        };
        let mut delta = i32::MIN;
        // SAFETY: both SDK methods take Unit, Extend and a writable LONG delta.
        let result = unsafe {
            method(
                self.0.as_ptr().cast(),
                unit as i32,
                i32::from(extend),
                &mut delta,
            )
        };
        check_hresult(result, if end { "EndKey" } else { "HomeKey" })?;
        Ok((result, delta))
    }

    fn move_vertical(
        &self,
        unit: Unit,
        count: i32,
        extend: bool,
        down: bool,
    ) -> TestResult<(i32, i32)> {
        let method = if down {
            self.table().move_down
        } else {
            self.table().move_up
        };
        let mut delta = i32::MIN;
        // SAFETY: the SDK methods take Unit, Count, Extend and a writable LONG.
        let result = unsafe {
            method(
                self.0.as_ptr().cast(),
                unit as i32,
                count,
                i32::from(extend),
                &mut delta,
            )
        };
        check_hresult(result, if down { "MoveDown" } else { "MoveUp" })?;
        Ok((result, delta))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionState {
    start: i32,
    end: i32,
    flags: i32,
}

impl SelectionState {
    fn directed(self) -> (i32, i32) {
        if self.flags & TOM_SEL_START_ACTIVE != 0 {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        }
    }
}

// These are Win32 TOM constants, not the differently numbered WinUI enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum Unit {
    Paragraph = 4,
    Line = 5,
    Story = 6,
}

fn observe(
    selection: &NativeSelection,
    fixture: &str,
    text_length: i32,
    story_length: i32,
    anchor: i32,
    active: i32,
    unit: Unit,
    extend: bool,
    end: bool,
) -> TestResult {
    selection.set_range(anchor, active)?;
    let before = selection.state()?;
    assert_eq!(
        before.directed(),
        (anchor, active),
        "{fixture}: native fixture direction"
    );
    let (result, delta) = selection.navigate(unit, extend, end)?;
    let after = selection.state()?;
    let (accepted_anchor, accepted_active) = after.directed();
    println!(
        "SYSTEM_MSFTEDIT_HOME_END fixture={fixture} supplied_utf16_length={text_length} native_story_length={story_length} unit={unit:?} extend={extend} operation={} before={before:?} after={after:?} anchor={accepted_anchor} active={accepted_active} delta={delta} active_displacement={} hresult=0x{:08X}",
        if end { "End" } else { "Home" }, accepted_active - active, result as u32,
    );
    assert!(0 <= after.start && after.start <= after.end && after.end <= story_length);
    if extend {
        assert_eq!(
            accepted_anchor, anchor,
            "extension retains the native anchor"
        );
    } else {
        assert_eq!(after.start, after.end, "collapse leaves a native caret");
    }
    if unit == Unit::Story && !end {
        assert_eq!(accepted_active, 0);
    }
    // A second identical command is an unambiguous no-op in these unwrapped
    // fixtures. Record S_OK/S_FALSE and flags as well as the expected zero delta.
    let (repeat_result, repeat_delta) = selection.navigate(unit, extend, end)?;
    let repeated = selection.state()?;
    println!(
        "SYSTEM_MSFTEDIT_HOME_END_REPEAT fixture={fixture} unit={unit:?} extend={extend} operation={} after={repeated:?} delta={repeat_delta} hresult=0x{:08X}",
        if end { "End" } else { "Home" }, repeat_result as u32,
    );
    assert_eq!(repeated.directed(), after.directed());
    assert_eq!(repeat_delta, 0);
    Ok(())
}

fn characterize() -> TestResult {
    // Locals drop in reverse declaration order: selection -> HWND -> DLL -> OLE,
    // including any early return or assertion failure.
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let window = HiddenRichEdit::new()?;
    let selection = window.selection()?;
    for (name, text, ranges) in [
        ("multiline", "abc\rdefg\rhij", &[(1, 6), (6, 1)][..]),
        ("utf16", "A😀B\rxy", &[(1, 6), (6, 1)][..]),
        ("empty_lines", "ab\r\rxy\r", &[(3, 3), (7, 7)][..]),
        ("empty_story", "", &[(0, 0)][..]),
    ] {
        window.set_text(text)?;
        let text_length = text.encode_utf16().count() as i32;
        selection.set_range(0, text_length)?;
        assert_eq!(
            selection.text()?,
            text,
            "{name}: native text was installed exactly"
        );
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!("SYSTEM_MSFTEDIT_FIXTURE name={name} text={text:?} utf16_length={text_length} native_story_length={story_length}");
        assert!(story_length >= text_length);
        for &(anchor, active) in ranges {
            for unit in [Unit::Line, Unit::Story] {
                for extend in [false, true] {
                    for end in [false, true] {
                        observe(
                            &selection,
                            name,
                            text_length,
                            story_length,
                            anchor,
                            active,
                            unit,
                            extend,
                            end,
                        )?;
                    }
                }
            }
        }
    }
    // Explicit boundary inputs supplement the idempotence checks above.
    window.set_text("abc\rdefg\rhij")?;
    let story_length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
    println!("SYSTEM_MSFTEDIT_FIXTURE name=boundary text=\"abc\\rdefg\\rhij\" utf16_length=12 native_story_length={story_length}");
    for (unit, end, position) in [
        (Unit::Line, false, 0),
        (Unit::Line, true, 3),
        (Unit::Story, false, 0),
        (Unit::Story, true, 12),
    ] {
        observe(
            &selection,
            "boundary",
            12,
            story_length,
            position,
            position,
            unit,
            false,
            end,
        )?;
    }
    println!("SYSTEM_MSFTEDIT_PROBE_COMPLETE commands=60 repeated_noops=60");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_home_end_characterization() {
    // A dedicated thread makes apartment ownership independent of other tests
    // on a reused harness worker. No window is shown and no focus is requested.
    std::thread::Builder::new()
        .name("system-msftedit-tom".into())
        .spawn(characterize)
        .expect("start the native TOM probe thread")
        .join()
        .expect("the native TOM probe thread panicked")
        .expect("the native TOM probe failed");
}

fn observe_vertical(
    selection: &NativeSelection,
    fixture: &str,
    story_length: i32,
    unit: Unit,
    count: i32,
    extend: bool,
    down: bool,
) -> TestResult {
    let before = selection.state()?;
    let (anchor, active) = before.directed();
    let (result, delta) = selection.move_vertical(unit, count, extend, down)?;
    let after = selection.state()?;
    let (accepted_anchor, accepted_active) = after.directed();
    println!(
        "SYSTEM_MSFTEDIT_VERTICAL fixture={fixture} unit={unit:?} count={count} extend={extend} operation={} before={before:?} after={after:?} anchor={accepted_anchor} active={accepted_active} delta={delta} active_displacement={} hresult=0x{:08X}",
        if down { "Down" } else { "Up" }, accepted_active - active, result as u32,
    );
    assert!(0 <= after.start && after.start <= after.end && after.end <= story_length);
    if extend {
        assert_eq!(
            accepted_anchor, anchor,
            "extension retains the native anchor"
        );
    } else if count != 0 {
        assert_eq!(after.start, after.end, "movement leaves a native caret");
    }
    // Count=0, collapse-edge choice, signed unit counts, paragraph transitions,
    // boundary no-ops and desired-column retention are observations. Do not
    // turn the initial run into assertions derived from the Rust implementation.
    Ok(())
}

fn characterize_vertical() -> TestResult {
    // Same ownership order as the Home/End probe; all COM work stays on this
    // thread and no HWND is shown or focused.
    let _apartment = OleApartment::new()?;
    let library = SystemRichEditLibrary::load()?;
    library.describe()?;
    let window = HiddenRichEdit::new()?;
    let selection = window.selection()?;
    let mut commands = 0;
    for (name, text, ranges) in [
        (
            "vertical_multiline",
            "abcdef\rxy\rabcdef\rpqrs",
            &[(2, 12), (12, 2), (5, 5)][..],
        ),
        ("vertical_utf16", "A😀B\rxy\rA😀B", &[(1, 6), (6, 1)][..]),
        ("vertical_empty_lines", "ab\r\rxy\r", &[(3, 3), (7, 7)][..]),
        ("vertical_empty_story", "", &[(0, 0)][..]),
    ] {
        window.set_text(text)?;
        let length = text.encode_utf16().count() as i32;
        selection.set_range(0, length)?;
        assert_eq!(selection.text()?, text, "{name}: exact native setup");
        let story_length =
            selection.number(selection.table().get_story_length, "GetStoryLength")?;
        println!("SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name={name} text={text:?} supplied_utf16_length={length} native_story_length={story_length}");
        for &(anchor, active) in ranges {
            for unit in [Unit::Line, Unit::Paragraph] {
                for extend in [false, true] {
                    for down in [false, true] {
                        for count in [0, 1, 2, -1] {
                            selection.set_range(anchor, active)?;
                            assert_eq!(selection.state()?.directed(), (anchor, active));
                            observe_vertical(
                                &selection,
                                name,
                                story_length,
                                unit,
                                count,
                                extend,
                                down,
                            )?;
                            commands += 1;
                        }
                    }
                }
            }
        }
    }
    let text = "abcdef\rx\rabcdef";
    window.set_text(text)?;
    selection.set_range(0, text.encode_utf16().count() as i32)?;
    assert_eq!(
        selection.text()?,
        text,
        "column fixture: exact native setup"
    );
    let story_length = selection.number(selection.table().get_story_length, "GetStoryLength")?;
    println!("SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name=retained_column text={text:?} supplied_utf16_length={} native_story_length={story_length}", text.encode_utf16().count());
    // Unlike matrix rows, these calls deliberately retain native navigation state.
    selection.set_range(5, 5)?;
    for down in [true, true, false, false] {
        observe_vertical(
            &selection,
            "retained_column",
            story_length,
            Unit::Line,
            1,
            false,
            down,
        )?;
        commands += 1;
    }
    println!("SYSTEM_MSFTEDIT_VERTICAL_FIXTURE name=vertical_boundary text={text:?} supplied_utf16_length={} native_story_length={story_length}", text.encode_utf16().count());
    for (position, down) in [(0, false), (text.encode_utf16().count() as i32, true)] {
        for unit in [Unit::Line, Unit::Paragraph] {
            selection.set_range(position, position)?;
            observe_vertical(
                &selection,
                "vertical_boundary",
                story_length,
                unit,
                1,
                false,
                down,
            )?;
            commands += 1;
        }
    }
    println!("SYSTEM_MSFTEDIT_VERTICAL_PROBE_COMPLETE commands={commands}");
    Ok(())
}

#[test]
fn rich_text_tom_oracle_system_msftedit_vertical_characterization() {
    std::thread::Builder::new()
        .name("system-msftedit-vertical".into())
        .spawn(characterize_vertical)
        .expect("start the native vertical TOM probe thread")
        .join()
        .expect("the native vertical TOM probe thread panicked")
        .expect("the native vertical TOM probe failed");
}
