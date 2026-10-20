//! ferroui-view
//!
//! A declarative way to write FerroUI user interfaces in Rust, without XAML:
//! elements built with chained setters (`button().content("Add").on_click(..)`)
//! or with the [`rsx!`] macro, state held in signals that the properties of
//! controls follow, and children that follow state.
//!
//! The pieces (`docs/declarative-ui.md` has the design):
//!
//! - [`Element`]: a description of controls, their property values,
//!   handlers and children. It is data, compared by value, so that a later
//!   reconciler can compare two trees and patch controls; mounting makes the
//!   controls ([`Element::mount`]).
//! - Builders: a function per control class (`button()`, `stack_panel()`)
//!   and a trait per class with a setter per property and a handler per routed
//!   event, generated from the sources (`scripts/generate_view_builders.py`).
//! - Reactivity: [`Signal`], [`Memo`], [`effect`], owned by an [`Owner`].
//!   A setter given a signal or a closure makes the property follow it.
//! - Children that follow state: [`for_each`] (keyed), [`show`], [`dynamic`].
//! - With the feature `desktop`: [`launch`] runs an application whose main
//!   window is an element.
//!
//! ```ignore
//! use ferroui_view::prelude::*;
//!
//! fn counter() -> Element {
//!     let count = signal(0);
//!     stack_panel()
//!         .spacing(8.0)
//!         .child(text_block().text(move || format!("Count: {}", count.get())))
//!         .child(button().content("Increment").on_click(move |_| count.update(|c| *c += 1)))
//!         .into()
//! }
//! ```

// The expansions of the macros name the crate; its own tests use them too.
extern crate self as ferroui_view;

pub mod builder;
mod control_flow;
mod conversions;
pub mod element;
mod extras;
pub mod reactive;

#[cfg(feature = "desktop")]
mod app;

/// The generated builders (see the module documentation).
pub mod generated {
    pub mod builders;
}

pub use builder::{Bind, Builder, El, ElementBuilder, FromValue, IntoProp, IntoValue, TypedProperty, ViewClass};
pub use control_flow::{dynamic, for_each, show, show_else, NodeRef};
pub use element::{Element, IntoChild, IntoChildren, IntoElement, Key};
pub use extras::GridDefinitions;
pub use reactive::{batch, effect, memo, on_cleanup, signal, untrack, Memo, Owner, Signal, TwoWay};

#[cfg(feature = "desktop")]
pub use app::{launch, mount_window, open_window};

pub use ferroui_view_macros::{component, rsx};

/// Everything an application written with the builders or the macro uses.
pub mod prelude {
    pub use crate::generated::builders::*;
    pub use crate::{
        batch, component, dynamic, effect, for_each, memo, on_cleanup, rsx, show, show_else, signal, untrack, Bind, El,
        Element, ElementBuilder, GridDefinitions, IntoChild, IntoChildren, IntoElement, Key, Memo, NodeRef, Owner,
        Signal, TwoWay,
    };
    #[cfg(feature = "desktop")]
    pub use crate::{launch, mount_window, open_window};
}

#[cfg(test)]
mod tests;
