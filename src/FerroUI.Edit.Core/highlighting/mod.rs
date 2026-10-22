mod document_highlighter;
mod highlighted_line;
mod highlighting_definition;
mod highlighting_engine;
mod highlighting_manager;

pub use document_highlighter::DocumentHighlighter;
pub use highlighted_line::{HighlightedLine, HighlightedSection};
pub use highlighting_definition::{
    HighlightingColor, HighlightingDefinition, HighlightingError, HighlightingRule,
    HighlightingRuleSet, HighlightingSpan,
};
pub use highlighting_engine::{HighlightingEngine, HighlightingState};
pub use highlighting_manager::HighlightingManager;
