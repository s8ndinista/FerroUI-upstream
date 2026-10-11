//! The input backends of the platform.

pub mod ev_dev;
mod i_input_backend;
mod i_screen_info_provider;
pub mod null_input;
mod raw_event_grouping_threading_helper;

pub use i_input_backend::IInputBackend;
pub use i_screen_info_provider::IScreenInfoProvider;
pub use raw_event_grouping_threading_helper::RawEventGroupingThreadingHelper;

/// The input backend over `libinput` (`LibInputBackend` of the
/// reference), the default when no backend is given and evdev is not
/// asked for.
///
/// # Panics
/// Always: the backend is stage 2 of
/// docs/porting/linux-framebuffer-platform.md and is not built yet.
pub(crate) fn lib_input_backend_is_not_built() -> ! {
    panic!(
        "The libinput backend is not built yet (stage 2 of docs/porting/linux-framebuffer-platform.md). \
         Pass an input backend, or set FERROUI_USE_EVDEV=1 and name the devices with \
         FERROUI_EVDEV_DEVICE_{{name}} variables"
    )
}
