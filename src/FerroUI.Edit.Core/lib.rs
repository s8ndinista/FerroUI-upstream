//! Renderer-independent editor services. Public document offsets count UTF-16
//! code units; logical line and column numbers are one-based.
//!
//! Documents are shared handles and snapshots are immutable. Editing services
//! operate on the same document/change contract used by the UI control.

pub mod document;
pub mod utils;
pub mod editing;
pub mod search;
pub mod folding;
pub mod completion;
pub mod snippets;
pub mod highlighting;

pub use document::{TextDocument, TextLocation};
