mod document_change_event_args;
mod document_line;
mod document_text_writer;
mod i_undoable_operation;
mod offset_change_map;
mod text_anchor;
mod text_anchor_tree;
mod text_document;
mod text_location;
mod text_segment;
mod text_segment_collection;
mod text_source;
mod text_source_version_provider;
mod text_utilities;
mod undo_stack;

pub use crate::utils::DocumentSubscription;
pub use document_change_event_args::{DocumentChangeEventArgs, TextChangeEventArgs};
pub use document_line::DocumentLine;
pub use document_text_writer::DocumentTextWriter;
pub use i_undoable_operation::{IUndoableOperation, IUndoableOperationWithContext};
pub use offset_change_map::{
    AnchorMovementType, OffsetChangeMap, OffsetChangeMapEntry, OffsetChangeMappingType,
};
pub use text_anchor::{AnchorSegment, TextAnchor};
pub use text_document::{DocumentEdit, DocumentError, TextDocument, UpdateGuard};
pub use text_location::{ISegment, SimpleSegment, TextLocation};
pub use text_segment::{ITextSegment, SegmentError, TextSegment};
pub use text_segment_collection::TextSegmentCollection;
pub use text_source::{ITextSource, StringTextSource, TextSnapshot};
pub use text_source_version_provider::{DocumentVersion, TextSourceVersionProvider};
pub use text_utilities::{CaretPositioningMode, CharacterClass, LogicalDirection, TextUtilities};
pub use undo_stack::UndoStack;

#[cfg(test)]
mod tests;
