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
//! file of the reference that declares them.

/// Declares a table of the functions of a library and its `load`.
///
/// ```ignore
/// native_library! {
///     /// The functions of libfoo.
///     pub struct LibFoo("libfoo.so.1") {
///         foo_create: fn(c_int) -> *mut c_void;
///     }
/// }
/// ```
///
/// Each field is the function of the same name, with the C signature the
/// declaration gives; it is called as `(table.foo_create)(1)` in an
/// `unsafe` block whose safety argument is the contract of the C function.
/// The library stays loaded for the life of the process.
macro_rules! native_library {
    (
        $(#[$meta:meta])*
        $vis:vis struct $table:ident($library:literal) {
            $($name:ident: fn($($arg:ty),* $(,)?) $(-> $ret:ty)?;)*
        }
    ) => {
        $(#[$meta])*
        #[allow(non_snake_case, dead_code)]
        $vis struct $table {
            $(pub(crate) $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }

        impl $table {
            /// Opens the library and resolves its functions.
            pub fn load() -> Result<Self, String> {
                let library_name = ::std::ffi::CString::new($library).expect("a library name has no NUL");
                // SAFETY: the name is a valid C string; the handle is kept for the life of
                // the process, so what the library exports stays valid.
                let library = unsafe { ::libc::dlopen(library_name.as_ptr(), ::libc::RTLD_NOW | ::libc::RTLD_LOCAL) };
                if library.is_null() {
                    return Err(format!("Unable to load shared library '{}' or one of its dependencies.", $library));
                }
                Ok(Self {
                    $($name: {
                        let name = ::std::ffi::CString::new(stringify!($name)).expect("an identifier has no NUL");
                        // SAFETY: a valid handle and a valid C string.
                        let symbol = unsafe { ::libc::dlsym(library, name.as_ptr()) };
                        if symbol.is_null() {
                            return Err(format!("{} does not export {}", $library, stringify!($name)));
                        }
                        // SAFETY: the symbol is the C function of this name, whose signature
                        // is the declared one (the header of the library).
                        unsafe {
                            ::std::mem::transmute::<*mut ::std::ffi::c_void, unsafe extern "C" fn($($arg),*) $(-> $ret)?>(symbol)
                        }
                    },)*
                })
            }
        }
    };
}

pub(crate) use native_library;
