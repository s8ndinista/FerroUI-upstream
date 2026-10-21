//! Portable inking controls and services, adapted from the WinUI control contracts.
//! See `docs/porting/winui-inking.md` for source mappings and interoperability limits.

mod ink_events;
mod markup;
pub(crate) use markup::register_markup;
mod ink_platform_services;
mod ink_clipboard;
mod ink_point;
mod ink_drawing_attributes;
mod ink_stroke;
mod ink_stroke_builder;
mod ink_stroke_container;
mod persistence;
mod ink_input;
mod ink_presenter;
mod ink_presenter_stencil;
mod ink_stencil_input;
mod ink_synchronizer;
mod ink_canvas;
mod ink_canvas_automation_peer;
mod color_names;
mod ink_toolbar_types;
mod ink_toolbar_buttons;
mod ink_toolbar_configuration;
mod ink_toolbar;
mod ink_toolbar_automation_peer;

pub use ink_point::{InkError, InkPoint};
pub use ink_platform_services::*;
pub use ink_clipboard::InkClipboard;
pub use ink_drawing_attributes::*;
pub use ink_stroke::InkStroke;
pub use ink_stroke_builder::InkStrokeBuilder;
pub use ink_stroke_container::InkStrokeContainer;
pub use persistence::{InkPersistenceLimits, INK_MIME_TYPE};
pub use ink_input::*;
pub use ink_presenter::*;
pub use ink_presenter_stencil::*;
pub use ink_synchronizer::InkSynchronizer;
pub use ink_canvas::InkCanvas;
pub use ink_canvas_automation_peer::InkCanvasAutomationPeer;
pub use ink_toolbar_types::*;
pub use ink_toolbar_buttons::*;
pub use ink_toolbar_configuration::*;
pub use ink_toolbar::*;
pub use ink_toolbar_automation_peer::*;

#[cfg(test)]
mod inking_tests;

#[cfg(test)]
mod ink_canvas_tests;

#[cfg(test)]
mod ink_parity_tests;

#[cfg(test)]
mod ink_stencil_tests;
