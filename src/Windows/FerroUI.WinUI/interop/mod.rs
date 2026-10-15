//! The interfaces of the Windows Runtime the integration calls and
//! implements: WinUI 3 (`Microsoft.UI.Xaml`), its input, dispatching and
//! content namespaces (`Microsoft.UI.*`), and what it uses of the system
//! (`Windows.UI.Text.Core`, `Windows.ApplicationModel.DataTransfer`,
//! `Windows.Storage`, `Windows.Foundation`).
//!
//! The bindings are generated from `winui.idl` by the MicroCom generator in
//! the build script of the crate. They are values and vtable calls, so they
//! compile on every host; what calls the system is Windows only. Beside
//! them are the pieces the projections of the reference's runtime provide:
//! strings and activation (`win_rt`), derived classes (`composable`),
//! delegates (`event_handlers`), awaiting an operation (`async_operation`)
//! and collections (`collections`).

#![allow(non_camel_case_types)]

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod winui {
    include!(concat!(env!("OUT_DIR"), "/winui.rs"));
}

pub use winui::*;

/// What an object of the integration answers as an `IInspectable`: the
/// identifiers of the interfaces it names, its class name, base trust. The
/// reference's runtime finds both by reflection.
#[cfg(windows)]
macro_rules! inspectable {
    ($type:ty, $class_name:expr, [$($interface:ty),* $(,)?]) => {
        impl $crate::interop::IInspectableImpl for $type {
            fn get_iids(&self, iid_count: *mut u32) -> Result<*mut ::ferroui_microcom::Guid, ::ferroui_microcom::HResult> {
                // SAFETY: the out parameter of the call.
                unsafe {
                    $crate::interop::win_rt::iids(
                        &[$(<$interface as ::ferroui_microcom::Interface>::IID),*],
                        iid_count,
                    )
                }
            }

            fn get_runtime_class_name(&self) -> Result<isize, ::ferroui_microcom::HResult> {
                $crate::interop::win_rt::runtime_class_name($class_name)
            }

            fn get_trust_level(&self) -> Result<$crate::interop::TrustLevel, ::ferroui_microcom::HResult> {
                Ok($crate::interop::TrustLevel::BaseTrust)
            }
        }
    };
}

#[cfg(windows)]
pub(crate) mod async_operation;
#[cfg(windows)]
pub(crate) mod collections;
#[cfg(windows)]
pub(crate) mod composable;
#[cfg(windows)]
pub(crate) mod event_handlers;
pub(crate) mod win_rt;

#[cfg(test)]
mod interop_tests;
