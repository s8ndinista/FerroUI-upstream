mod rich_edit_text_document;
mod rich_text_editor;
mod rich_text_transfer;
mod rtf;

pub(crate) use rich_edit_text_document::unit_ranges as rich_text_unit_ranges;
pub(crate) use rich_text_editor::utf16_prefix;
pub use rich_edit_text_document::{
    RichEditTextDocument, RichEditTextRange, RichEditTextSelection, TextFindOptions,
    TextFormatValue, TextRangeUnit,
};
pub use rich_text_editor::{EditError, EditKind, EditorSelection, RichTextEditor};
pub use rich_text_transfer::{RichEditClipboardFormat, RichTextTransfer};
pub use rtf::{RtfError, RtfLimits, RtfReader, RtfWriter};
