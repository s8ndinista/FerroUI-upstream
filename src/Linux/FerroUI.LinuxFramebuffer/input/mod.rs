//! The input backends of the platform.

pub mod ev_dev;
mod i_input_backend;
mod i_screen_info_provider;
pub mod lib_input;
pub mod null_input;
mod raw_event_grouping_threading_helper;

pub use i_input_backend::IInputBackend;
pub use i_screen_info_provider::IScreenInfoProvider;
pub use raw_event_grouping_threading_helper::RawEventGroupingThreadingHelper;
