//! A library of the system opened at run time: the table of its
//! functions, each resolved by name when the library is loaded.
//!
//! The reference declares platform invokes of `libinput.so.10`,
//! `libdrm.so.2` and `libgbm.so.1`, which its runtime resolves when one is
//! first called. Here a library is opened with `dlopen` and every function
//! of its table is resolved with `dlsym` when the table is loaded
//! (docs/porting/linux-framebuffer-platform.md, section 3): nothing links
//! the libraries, and a machine without one gets an error that names it.
//! An addition of the port; the declarations of each library are in the
//! file of the reference that declares them, as tables of the loader of the
//! base crate (`ferroui_base::native_functions!`), which the other Linux
//! backends share.

use ferroui_base::platform::native_library::{NativeLibrary, NativeLibraryError};
use std::ffi::CStr;

/// Opens the library of that name and resolves its table with `load` (the
/// `load` of a table of `ferroui_base::native_functions!`).
///
/// Every function of the library is bound when it is opened, and its
/// symbols stay out of the global namespace (`RTLD_NOW | RTLD_LOCAL`). The
/// library stays loaded for the life of the process. The error names the
/// library that cannot be loaded or the function it does not export.
pub(crate) fn load_library<T>(
    library: &'static CStr,
    load: impl FnOnce(&NativeLibrary) -> Result<T, NativeLibraryError>,
) -> Result<T, String> {
    let library = NativeLibrary::open_with_flags(library, libc::RTLD_NOW | libc::RTLD_LOCAL).map_err(message)?;
    load(&library).map_err(message)
}

/// The message of the backend for a library or a function that cannot be
/// had.
fn message(error: NativeLibraryError) -> String {
    match error {
        NativeLibraryError::EntryPoint { library, symbol } => format!("{library} does not export {symbol}"),
        error @ NativeLibraryError::Library { .. } => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    #[test]
    fn a_library_that_does_not_exist_is_an_error_with_its_name() {
        let error = load_library(c"libferroui-there-is-no-such-library.so.0", |_| Ok(())).unwrap_err();
        assert_eq!(
            error,
            "Unable to load shared library 'libferroui-there-is-no-such-library.so.0' or one of its dependencies."
        );
    }

    /// The C library is loaded in every process: a table of a function it
    /// has is loaded, one of a function it does not have is an error that
    /// names both.
    #[test]
    fn a_function_that_is_missing_is_an_error_with_its_name() {
        ferroui_base::native_functions! {
            struct Table(lib) {
                lib::getpid: unsafe extern "C" fn() -> libc::pid_t;
            }
        }
        let table = load_library(c"libc.so.6", Table::load).unwrap();
        // SAFETY: `getpid` takes nothing and always succeeds.
        assert_eq!(unsafe { (table.getpid)() } as u32, std::process::id());

        ferroui_base::native_functions! {
            struct Missing(lib) {
                lib::ferroui_no_such_function: unsafe extern "C" fn();
            }
        }
        let error = load_library(c"libc.so.6", Missing::load).err().unwrap();
        assert_eq!(error, "libc.so.6 does not export ferroui_no_such_function");
    }
}
