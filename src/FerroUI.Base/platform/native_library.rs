//! A shared library that is opened at run time and asked for its functions
//! by name (no upstream file: the reference declares the functions of a
//! library as platform invokes, which its runtime resolves this way when a
//! function is first called).
//!
//! The one loader of the backends of Unix systems: the X11 crate opens
//! GLib, GTK, the session management library and the ICE library with it,
//! the Wayland crate GBM and DRM, and the Linux framebuffer crate libinput,
//! libdrm and libgbm. Nothing of those libraries is linked: a crate builds
//! and starts without them, and a library that is missing shows as an
//! error value where the reference gets an exception
//! (docs/porting/wayland-platform.md, section 5.1).

use std::ffi::{c_int, c_void, CStr, CString};

/// A library that was opened. It is never closed, so what it exports stays
/// valid for the life of the process.
#[derive(Clone, Copy, Debug)]
pub struct NativeLibrary {
    /// The handle of the dynamic loader, as a number.
    handle: usize,
    name: &'static CStr,
}

/// A library or a function that cannot be had.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeLibraryError {
    /// The dynamic loader cannot load the library or one of its
    /// dependencies.
    Library {
        /// The name the library was asked for by.
        library: String,
    },
    /// The library has no function of that name.
    EntryPoint {
        /// The name the library was opened by.
        library: String,
        /// The name of the function.
        symbol: String,
    },
}

impl std::fmt::Display for NativeLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Library { library } => {
                write!(f, "Unable to load shared library '{library}' or one of its dependencies.")
            }
            Self::EntryPoint { library, symbol } => {
                write!(f, "Unable to find an entry point named '{symbol}' in shared library '{library}'.")
            }
        }
    }
}

impl std::error::Error for NativeLibraryError {}

impl NativeLibrary {
    /// Opens the library of that name, as the dynamic loader finds it: its
    /// functions are bound when first called, and its symbols are available
    /// to the libraries loaded after it (`RTLD_LAZY | RTLD_GLOBAL`).
    pub fn open(name: &'static CStr) -> Result<Self, NativeLibraryError> {
        Self::open_with_flags(name, libc::RTLD_LAZY | libc::RTLD_GLOBAL)
    }

    /// Opens the library of that name with the flags of `dlopen` given
    /// (`RTLD_NOW | RTLD_LOCAL`, for one).
    pub fn open_with_flags(name: &'static CStr, flags: c_int) -> Result<Self, NativeLibraryError> {
        // SAFETY: the name is a terminated string. The initializers of the library run, as
        // they do for any library a process loads; the handle is kept for ever.
        let handle = unsafe { libc::dlopen(name.as_ptr(), flags) };
        if handle.is_null() {
            return Err(NativeLibraryError::Library { library: name.to_string_lossy().into_owned() });
        }
        Ok(Self { handle: handle as usize, name })
    }

    /// The name the library was opened by.
    pub fn name(&self) -> &'static CStr {
        self.name
    }

    /// The address of the function of that name.
    pub fn symbol(&self, symbol: &str) -> Result<*const c_void, NativeLibraryError> {
        let missing = || NativeLibraryError::EntryPoint {
            library: self.name.to_string_lossy().into_owned(),
            symbol: symbol.to_owned(),
        };
        let c_symbol = CString::new(symbol).map_err(|_| missing())?;
        // SAFETY: the handle is the one `dlopen` returned and the library is still loaded;
        // the name is a terminated string that lives for the call.
        let address = unsafe { libc::dlsym(self.handle as *mut c_void, c_symbol.as_ptr()) };
        if address.is_null() {
            return Err(missing());
        }
        Ok(address.cast_const())
    }
}

/// Declares the functions the port calls of one or more libraries as a
/// table of function pointers, with a constructor that resolves every one
/// of them or fails with the name of the first that is missing.
///
/// Each entry names the library (an argument of `load`) the function is
/// in, as the `DllImport` attribute of the reference does, and the
/// exported name when the field is named otherwise (`as "Name"`):
///
/// ```ignore
/// ferroui_base::native_functions! {
///     /// The functions of libfoo.
///     pub struct LibFoo(lib) {
///         lib::foo_create: unsafe extern "C" fn(c_int) -> *mut c_void;
///     }
/// }
///
/// let table = LibFoo::load(&NativeLibrary::open(c"libfoo.so.1")?)?;
/// ```
///
/// Each field is the function of that name, with the C signature the
/// declaration gives; it is called as `(table.foo_create)(1)` in an
/// `unsafe` block whose safety argument is the contract of the C function.
#[macro_export]
#[doc(hidden)]
macro_rules! native_functions {
    (@symbol $f:ident) => { stringify!($f) };
    (@symbol $f:ident $symbol:literal) => { $symbol };
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ($($lib:ident),+ $(,)?) {
            $( $(#[$fmeta:meta])* $flib:ident :: $f:ident $(as $symbol:literal)? : $ty:ty ; )*
        }
    ) => {
        $(#[$meta])*
        #[allow(non_snake_case, dead_code)]
        $vis struct $name {
            $( $(#[$fmeta])* pub(crate) $f: $ty, )*
        }

        impl $name {
            /// Resolves every function of the table.
            #[allow(dead_code, non_snake_case)]
            $vis fn load(
                $($lib: &$crate::platform::native_library::NativeLibrary),+
            ) -> Result<Self, $crate::platform::native_library::NativeLibraryError> {
                Ok(Self {
                    $(
                        // SAFETY: the address is that of the exported function of this name,
                        // whose C declaration is the type of the field; a function pointer
                        // has the size and the validity of a non-null address.
                        $f: unsafe {
                            std::mem::transmute::<*const std::ffi::c_void, $ty>(
                                $flib.symbol($crate::native_functions!(@symbol $f $($symbol)?))?,
                            )
                        },
                    )*
                })
            }
        }
    };
}
pub use native_functions;

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    /// The C library, which is loaded in every process.
    #[cfg(target_os = "linux")]
    const LIBC: &CStr = c"libc.so.6";
    #[cfg(target_os = "macos")]
    const LIBC: &CStr = c"libSystem.B.dylib";
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    const LIBC: &CStr = c"libc.so";

    #[test]
    fn a_library_that_does_not_exist_is_an_error_with_its_name() {
        let error = NativeLibrary::open(c"libferroui-there-is-no-such-library.so.0").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Unable to load shared library 'libferroui-there-is-no-such-library.so.0' or one of its dependencies."
        );
        assert_eq!(
            error,
            NativeLibraryError::Library { library: "libferroui-there-is-no-such-library.so.0".to_string() }
        );
    }

    /// The C library is loaded in every process: a function it has is
    /// found, one it does not have is an error that names both.
    #[test]
    fn a_function_is_found_by_name_and_a_missing_one_is_an_error() {
        let Ok(library) = NativeLibrary::open(LIBC) else {
            return;
        };
        assert!(library.symbol("getpid").is_ok());
        let error = library.symbol("ferroui_no_such_function").unwrap_err();
        assert!(error.to_string().starts_with("Unable to find an entry point named 'ferroui_no_such_function'"));
        assert_eq!(
            error,
            NativeLibraryError::EntryPoint {
                library: LIBC.to_string_lossy().into_owned(),
                symbol: "ferroui_no_such_function".to_string(),
            }
        );

        native_functions! {
            struct Table(lib) {
                lib::getpid: unsafe extern "C" fn() -> libc::pid_t;
            }
        }
        let table = Table::load(&library).unwrap();
        // SAFETY: `getpid` takes nothing and always succeeds.
        assert_eq!(unsafe { (table.getpid)() } as u32, std::process::id());

        native_functions! {
            struct Missing(lib) {
                lib::ferroui_no_such_function: unsafe extern "C" fn();
            }
        }
        assert!(Missing::load(&library).is_err());
    }

    /// A library opened with flags of its own is the same library.
    #[test]
    fn a_library_is_opened_with_the_flags_given() {
        let Ok(library) = NativeLibrary::open_with_flags(LIBC, libc::RTLD_NOW | libc::RTLD_LOCAL) else {
            return;
        };
        assert_eq!(library.name(), LIBC);
        assert!(library.symbol("getpid").is_ok());
        assert!(NativeLibrary::open_with_flags(c"libferroui-there-is-no-such-library.so.0", libc::RTLD_NOW).is_err());
    }
}
