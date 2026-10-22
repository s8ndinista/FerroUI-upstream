mod document_change_event_args;
mod document_line;
mod offset_change_map;
mod text_anchor;
mod text_anchor_tree;
mod text_document;
mod text_location;
mod text_source;
mod text_source_version_provider;
mod undo_stack;

pub use crate::utils::DocumentSubscription;
pub use document_change_event_args::{DocumentChangeEventArgs, TextChangeEventArgs};
pub use document_line::DocumentLine;
pub use offset_change_map::{
    AnchorMovementType, OffsetChangeMap, OffsetChangeMapEntry, OffsetChangeMappingType,
};
pub use text_anchor::{AnchorSegment, TextAnchor};
pub use text_document::{DocumentEdit, DocumentError, TextDocument, UpdateGuard};
pub use text_location::{ISegment, SimpleSegment, TextLocation};
pub use text_source::{ITextSource, StringTextSource, TextSnapshot};
pub use text_source_version_provider::{DocumentVersion, TextSourceVersionProvider};
pub use undo_stack::UndoStack;

#[cfg(test)]
mod tests;
