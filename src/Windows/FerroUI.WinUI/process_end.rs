//! The end of the process of a WinUI application: what the runtime of the
//! reference does once its generated `Main` has returned from
//! `Application.Start`.
//!
//! When `main` of a Rust program returns on Windows, the C runtime calls
//! `ExitProcess` on the main thread. The system first ends every other
//! thread of the process, wherever each was, and then, on the main thread,
//! runs the callbacks of the end of the process: the thread-local
//! destructors of that thread (the TLS callback of the standard library)
//! and the `DLL_PROCESS_DETACH` of the libraries. The main thread of a WinUI
//! application is its UI thread: its thread-local values are the framework
//! (the dispatcher, the services, the platform, the objects of the Windows
//! App SDK the integration and the application hold) and the per-thread
//! state of ANGLE and of XAML. A destructor there that takes a lock an ended
//! thread held never returns: a lock of the Rust standard library or a
//! slim reader/writer lock has no owner the system could see is gone
//! (`docs/porting/win32-platform.md`, section 11.3, for the Windows backend;
//! `docs/porting/winui.md`, section 7.7, for this integration: the process
//! of ControlGallery.WinUI printed `main returns` and then stayed with one
//! thread, waiting on such a lock, until the step of CI ended it).
//!
//! The runtime of the reference never destroys those values: its statics
//! and thread statics are not finalized when the process ends, and neither
//! are the objects of the Windows Runtime its wrappers hold; its threads are
//! ended by the system as these are. [`end_process`] does the same for a
//! Rust program: it hands the end of the process to a thread of its own,
//! which holds none of the values of the UI thread and calls `exit` of the C
//! runtime (the handlers registered with `atexit`, the destructors of the
//! static objects of the C++ libraries, then `ExitProcess`, which runs the
//! detach callbacks of the libraries on that thread). The UI thread waits,
//! holding nothing, until the system ends it; its values are never
//! destroyed.

/// A line of the list of the threads of the process: the identifier, the
/// mark of the calling thread, the description the thread was given and
/// where it started (module and offset, or the bare address).
fn thread_line(id: u32, current: bool, name: &str, start: Option<&str>) -> String {
    let mut line = format!("thread {id}");
    if current {
        line.push_str(" (this thread)");
    }
    if !name.is_empty() {
        line.push_str(&format!(" \"{name}\""));
    }
    if let Some(start) = start {
        line.push_str(&format!(", started in {start}"));
    }
    line
}

/// The file name of a module path and the offset of an address in it, as
/// the lines of the threads print a start address.
fn module_offset(module_path: &str, address: usize, base: usize) -> String {
    let file = module_path.rsplit(['\\', '/']).next().unwrap_or(module_path);
    format!("{file}+{:#x}", address.wrapping_sub(base))
}

#[cfg(windows)]
pub use imp::{end_process, process_threads};

#[cfg(windows)]
mod imp {
    use super::{module_offset, thread_line};
    use std::ffi::c_void;
    use std::io::Write;
    use std::time::Duration;
    use windows_sys::Wdk::System::Threading::{NtQueryInformationThread, ThreadQuerySetWin32StartAddress};
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::WriteFile;
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, GetCurrentThreadId, GetThreadDescription, OpenThread, THREAD_QUERY_INFORMATION,
    };

    // The C runtime the standard library links on Windows.
    unsafe extern "C" {
        fn atexit(callback: extern "C" fn()) -> i32;
        fn exit(code: i32) -> !;
        fn fflush(stream: *mut c_void) -> i32;
    }

    /// How long after [`end_process`] the watchdog of a traced run first
    /// lists the threads, and then again each time.
    const WATCHDOG_PERIOD: Duration = Duration::from_secs(15);

    /// Whether the end of the process is traced: the switch of the
    /// integration's diagnostics (`FERROUI_WINUI_TRACE`, which the smoke runs
    /// of the samples set).
    fn traced() -> bool {
        std::env::var_os("FERROUI_WINUI_TRACE").is_some()
    }

    /// Writes a line to the standard output handle with one call of the
    /// system, without the lock of the standard library's `stdout`: a line
    /// of the end of the process may be written after the system has ended a
    /// thread that held that lock.
    fn raw_line(text: &str) {
        let line = format!("{text}\n");
        // SAFETY: the handle of the process's standard output (it may be
        // null or invalid, then nothing is written) and a buffer of this
        // frame.
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return;
            }
            let mut written = 0u32;
            WriteFile(handle, line.as_ptr(), line.len() as u32, &mut written, std::ptr::null_mut());
        }
    }

    /// The threads of the process, a line each: the identifier, the
    /// description and the module and offset of the start address
    /// (Toolhelp32 and `ThreadQuerySetWin32StartAddress`). What a run that
    /// does not end prints, so that the log names the thread that is waited
    /// for.
    pub fn process_threads() -> Vec<String> {
        let mut list = Vec::new();
        // SAFETY (the block): calls of the system with values and buffers of
        // this function; every handle that is opened is closed, and the
        // description is freed.
        unsafe {
            let (process, current) = (GetCurrentProcessId(), GetCurrentThreadId());
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return list;
            }
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = size_of::<THREADENTRY32>() as u32;
            let mut more = Thread32First(snapshot, &mut entry);
            while more != 0 {
                if entry.th32OwnerProcessID == process {
                    let id = entry.th32ThreadID;
                    let mut name = String::new();
                    let mut start = None;
                    let thread = OpenThread(THREAD_QUERY_INFORMATION, 0, id);
                    if !thread.is_null() {
                        let mut description: *mut u16 = std::ptr::null_mut();
                        if GetThreadDescription(thread, &mut description) >= 0 && !description.is_null() {
                            let mut length = 0;
                            while *description.add(length) != 0 {
                                length += 1;
                            }
                            name = String::from_utf16_lossy(std::slice::from_raw_parts(description, length));
                            LocalFree(description.cast());
                        }
                        let mut address: usize = 0;
                        let status = NtQueryInformationThread(
                            thread,
                            ThreadQuerySetWin32StartAddress,
                            (&raw mut address).cast(),
                            size_of::<usize>() as u32,
                            std::ptr::null_mut(),
                        );
                        if status >= 0 {
                            let mut module = std::ptr::null_mut();
                            let found = GetModuleHandleExW(
                                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                                address as *const u16,
                                &mut module,
                            );
                            start = Some(if found != 0 {
                                let mut file = [0u16; 512];
                                let length = GetModuleFileNameW(module, file.as_mut_ptr(), file.len() as u32) as usize;
                                module_offset(&String::from_utf16_lossy(&file[..length]), address, module as usize)
                            } else {
                                format!("{address:#x}")
                            });
                        }
                        CloseHandle(thread);
                    }
                    list.push(thread_line(id, id == current, &name, start.as_deref()));
                }
                more = Thread32Next(snapshot, &mut entry);
            }
            CloseHandle(snapshot);
        }
        list
    }

    fn print_threads(heading: &str) {
        let threads = process_threads();
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "[exit] {heading}: {} thread(s)", threads.len());
        for thread in &threads {
            let _ = writeln!(out, "[exit]   {thread}");
        }
        let _ = out.flush();
    }

    /// The handler of the C runtime, registered by a traced
    /// [`end_process`]: `exit` runs it on the thread that ends the process,
    /// before `ExitProcess`, while every thread still runs.
    extern "C" fn at_exit() {
        print_threads("exit of the C runtime: the handlers registered with atexit run");
    }

    /// A value of the thread that ends the process: the standard library
    /// destroys it in the TLS callback of the executable with
    /// `DLL_PROCESS_DETACH`, that is inside `ExitProcess`, after the system
    /// has ended the other threads.
    struct DetachMarker;

    impl Drop for DetachMarker {
        fn drop(&mut self) {
            raw_line("[exit] ExitProcess: the other threads are ended, the detach callbacks run");
        }
    }

    ferroui_base::ferro_thread_local! {
        static DETACH_MARKER: DetachMarker = const { DetachMarker };
    }

    /// Ends the process with `code` as the runtime of the reference ends it
    /// once `Main` has returned (see [the module](super)): the standard
    /// output is flushed, a thread of its own calls `exit` of the C runtime,
    /// and the calling thread (the UI thread) waits until the system ends
    /// it, so that its thread-local values are never destroyed.
    ///
    /// Call it from the UI thread after `Application.Start` has returned,
    /// once what has to be done before the end is done (the windows let go
    /// of, `MddBootstrapShutdown`). With `FERROUI_WINUI_TRACE` set the end is
    /// traced on the standard output (`[exit]` lines): the threads alive
    /// when it starts, the handlers of the C runtime (with the threads alive
    /// then), the destructor of the ending thread's values inside
    /// `ExitProcess`, and a watchdog that lists the threads, with where each
    /// started, every 15 seconds while the process still runs before
    /// `ExitProcess` (inside it the system has ended the watchdog too; what
    /// waits there is named by the step of CI from outside,
    /// `scripts/winui/smoke-run.ps1`).
    pub fn end_process(code: i32) -> ! {
        let trace = traced();
        if trace {
            print_threads(&format!("main hands the end of the process (code {code}) to a thread of its own"));
            // SAFETY: a function of this crate that takes nothing and
            // returns.
            unsafe { atexit(at_exit) };
            let _ = std::thread::Builder::new().name("process end watchdog".to_owned()).spawn(|| {
                let mut waited = Duration::ZERO;
                loop {
                    std::thread::sleep(WATCHDOG_PERIOD);
                    waited += WATCHDOG_PERIOD;
                    print_threads(&format!("the process still runs {} s after main handed its end over", waited.as_secs()));
                }
            });
        }

        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        // SAFETY: null flushes every stream of the C runtime.
        unsafe { fflush(std::ptr::null_mut()) };

        let ending = std::thread::Builder::new().name("process end".to_owned()).spawn(move || {
            if trace {
                DETACH_MARKER.with(|_| {});
            }
            // SAFETY: ends the process; nothing of this program runs after
            // it but the handlers of the C runtime and the destructors of
            // this thread's values.
            unsafe { exit(code) }
        });
        if ending.is_err() {
            // No thread could be started: the end of a program whose `main`
            // returns.
            std::process::exit(code);
        }
        loop {
            std::thread::park();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference ends its process through its runtime.
    use super::*;

    #[test]
    fn a_thread_line_names_the_thread_and_where_it_started() {
        assert_eq!(thread_line(7716, false, "", None), "thread 7716");
        assert_eq!(
            thread_line(12, true, "process end", Some("app.exe+0x10")),
            "thread 12 (this thread) \"process end\", started in app.exe+0x10"
        );
    }

    #[test]
    fn a_start_address_is_printed_as_module_and_offset() {
        assert_eq!(
            module_offset(r"C:\Windows\System32\ntdll.dll", 0x7ff8_0001_2340, 0x7ff8_0000_0000),
            "ntdll.dll+0x12340"
        );
        assert_eq!(module_offset("libGLESv2.dll", 0x1010, 0x1000), "libGLESv2.dll+0x10");
    }
}
