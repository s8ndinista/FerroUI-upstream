//! Thread-local variables that do not take a key of the C library each.
//!
//! [`ferro_thread_local!`](macro@crate::ferro_thread_local) is declared and used
//! exactly as `std::thread_local!` is, and on every target with thread-local
//! statics it *is* `std::thread_local!`: the declaration is handed on
//! unchanged, so nothing differs in behaviour or cost.
//!
//! The Android targets of Rust have no thread-local statics: the standard
//! library keeps each `thread_local!` variable under a key of its own
//! (`pthread_key_create`), and bionic has 128 keys for a whole process. The
//! class model of the port keeps its definitions per thread, a cell per
//! property, routed event and class, which is far more than that
//! (docs/porting/android-platform.md, section 5.1). On Android the macro
//! therefore declares a slot of one table per thread instead ([`LocalKey`]):
//!
//! - Every declaration takes a number once, from a static counter, the first
//!   time it is used on any thread. The number is its index in the table.
//! - The table of a thread is a vector of boxed values, created with the
//!   first use on the thread and found through **one** key of the C library
//!   for the whole process. A value is created by its initialiser the first
//!   time it is used on the thread, as with `thread_local!`.
//! - No borrow of the table is held while an initialiser or a destructor
//!   runs, so an initialiser may use other variables (and its own: as with
//!   `thread_local!`, the value the outer initialiser returns then replaces
//!   the one the inner one stored). A value is boxed, so the references a
//!   `with` hands out stay valid while the table grows.
//! - When the thread ends, the values whose type has a destructor are
//!   destroyed in the reverse order of their creation; a value created while
//!   that runs is destroyed too. A destroyed variable cannot be used again
//!   on the thread: [`LocalKey::try_with`] returns an [`AccessError`] and
//!   [`LocalKey::with`] panics with the message of the standard library. A
//!   variable whose type has no destructor stays usable until the last
//!   destructor has run, as a variable of such a type does with
//!   `thread_local!` on targets with thread-local statics, and its memory is
//!   freed last.
//!
//! The table is compiled on hosts as well, for its tests, and with
//! `--cfg ferro_tls_table` every `ferro_thread_local!` of the build uses it
//! there (on Linux and macOS), so that the test suites of the framework can
//! run against it without a device.

#[cfg(any(
    target_os = "android",
    ferro_tls_table,
    all(test, any(target_os = "linux", target_os = "macos"))
))]
#[doc(hidden)]
pub mod table;

/// The type of a variable [`ferro_thread_local!`](macro@crate::ferro_thread_local) declares, and
/// the error of its `try_with`: those of the standard library where the macro is
/// `std::thread_local!`, those of the table on Android. Code that names the type (a function that
/// takes `&'static LocalKey<T>`) names this one.
#[cfg(any(target_os = "android", ferro_tls_table))]
pub use table::{AccessError, LocalKey};
#[cfg(not(any(target_os = "android", ferro_tls_table)))]
pub use std::thread::{AccessError, LocalKey};

/// Declares thread-local variables: `std::thread_local!` on targets with
/// thread-local statics, a slot of the table of the thread on Android (see
/// [the module](mod@crate::ferro_thread_local)).
///
/// ```
/// use std::cell::{Cell, RefCell};
///
/// ferroui_base::ferro_thread_local! {
///     static COUNT: Cell<u32> = const { Cell::new(0) };
///     static NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
/// }
///
/// COUNT.set(COUNT.get() + 1);
/// NAMES.with_borrow_mut(|names| names.push("one".to_owned()));
/// assert_eq!(COUNT.get(), 1);
/// ```
///
/// New code of the workspace declares its thread-local variables with this
/// macro and never with `thread_local!` (the conventions job checks it).
#[cfg(not(any(target_os = "android", ferro_tls_table)))]
#[macro_export]
macro_rules! ferro_thread_local {
    ($($declarations:tt)*) => {
        ::std::thread_local! { $($declarations)* }
    };
}

/// Declares thread-local variables: `std::thread_local!` on targets with
/// thread-local statics, a slot of the table of the thread on Android (see
/// [the module](mod@crate::ferro_thread_local)).
///
/// New code of the workspace declares its thread-local variables with this
/// macro and never with `thread_local!` (the conventions job checks it).
#[cfg(any(target_os = "android", ferro_tls_table))]
#[macro_export]
macro_rules! ferro_thread_local {
    ($($declarations:tt)*) => {
        $crate::__ferro_thread_local_table! { $($declarations)* }
    };
}

/// The declarations of [`ferro_thread_local!`](macro@crate::ferro_thread_local)
/// as slots of the table of the thread, whatever the target: what the macro
/// expands to on Android, and what the tests of the table declare on a host.
/// The grammar is that of `std::thread_local!`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_thread_local_table {
    (@key $(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $init:expr) => {
        $(#[$attr])* $vis static $name: $crate::ferro_thread_local::table::LocalKey<$t> = {
            #[inline]
            fn __init() -> $t {
                $init
            }
            $crate::ferro_thread_local::table::LocalKey::new(__init)
        };
    };
    () => {};
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block; $($rest:tt)*) => {
        $crate::__ferro_thread_local_table!(@key $(#[$attr])* $vis $name, $t, {
            const INIT_EXPR: $t = $init;
            INIT_EXPR
        });
        $crate::__ferro_thread_local_table!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block) => {
        $crate::__ferro_thread_local_table!(@key $(#[$attr])* $vis $name, $t, {
            const INIT_EXPR: $t = $init;
            INIT_EXPR
        });
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => {
        $crate::__ferro_thread_local_table!(@key $(#[$attr])* $vis $name, $t, $init);
        $crate::__ferro_thread_local_table!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => {
        $crate::__ferro_thread_local_table!(@key $(#[$attr])* $vis $name, $t, $init);
    };
}

