mod highlighting_definition;
mod highlighting_engine;
mod highlighted_line;
mod document_highlighter;

pub use highlighting_definition::{HighlightingColor,HighlightingDefinition,HighlightingError,HighlightingRule,HighlightingRuleSet,HighlightingSpan};
pub use highlighting_engine::{HighlightingEngine,HighlightingState};
pub use highlighted_line::{HighlightedLine,HighlightedSection};
pub use document_highlighter::DocumentHighlighter;
