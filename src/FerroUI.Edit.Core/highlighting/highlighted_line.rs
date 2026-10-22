use super::{HighlightingColor, HighlightingState};

#[derive(Clone, Debug, PartialEq)]
pub struct HighlightedSection {
    pub offset: usize,
    pub length: usize,
    pub color: HighlightingColor,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HighlightedLine {
    pub line_number: usize,
    pub sections: Vec<HighlightedSection>,
    pub end_state: HighlightingState,
}
