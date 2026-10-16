//! The native text box the embedding page hosts (namespace `IntegrationTestApp.Embedding`):
//! one module per upstream file.
//!
//! Ported: the control ([`NativeTextBox`]), the contract of its platform specific part
//! ([`INativeTextBoxFactory`], [`INativeTextBoxImpl`]), the AppKit part
//! (`MacOSTextBoxFactory`, `MacOSViewHandle`, `MacHelper`) and the part of the Windows platform
//! (`Win32TextBoxFactory`, `Win32WindowControlHandle`, `WinApi`). The managed original reaches
//! AppKit through a binding package; here the few Objective-C runtime calls the text box needs are
//! in `objc.rs`, which is not a port. The calls of the Windows API are the ones of `windows-sys`.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod i_native_text_box_factory;
mod native_text_box;

#[cfg(target_os = "macos")]
mod mac_helper;
#[cfg(target_os = "macos")]
mod mac_os_text_box_factory;
#[cfg(target_os = "macos")]
mod mac_os_view_handle;
#[cfg(target_os = "macos")]
pub(crate) mod objc;

#[cfg(windows)]
mod win32_text_box_factory;
#[cfg(windows)]
mod win32_window_control_handle;
#[cfg(windows)]
mod win_api;

pub use i_native_text_box_factory::{INativeTextBoxFactory, INativeTextBoxImpl};
pub use native_text_box::NativeTextBox;

#[cfg(target_os = "macos")]
pub use mac_helper::MacHelper;
#[cfg(target_os = "macos")]
pub use mac_os_text_box_factory::MacOSTextBoxFactory;
#[cfg(target_os = "macos")]
pub use mac_os_view_handle::MacOSViewHandle;

#[cfg(windows)]
pub use win32_text_box_factory::Win32TextBoxFactory;
#[cfg(windows)]
pub use win32_window_control_handle::Win32WindowControlHandle;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[NativeTextBox::TYPE];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[];

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
pub(crate) fn register_lists() {}
