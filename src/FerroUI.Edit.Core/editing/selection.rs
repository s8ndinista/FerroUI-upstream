use crate::document::{AnchorMovementType, DocumentChangeEventArgs, SimpleSegment};

/// A directed UTF-16 selection. `anchor` stays fixed while `caret` moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionSegment {
    pub anchor: usize,
    pub caret: usize,
}

impl SelectionSegment {
    pub const fn new(anchor: usize, caret: usize) -> Self { Self { anchor, caret } }
    pub const fn empty(offset: usize) -> Self { Self::new(offset, offset) }
    pub fn start_offset(self) -> usize { self.anchor.min(self.caret) }
    pub fn end_offset(self) -> usize { self.anchor.max(self.caret) }
    pub fn length(self) -> usize { self.end_offset() - self.start_offset() }
    pub fn is_empty(self) -> bool { self.anchor == self.caret }
    pub fn contains(self, offset: usize) -> bool { self.start_offset() <= offset && offset <= self.end_offset() }
    pub fn segment(self) -> SimpleSegment { SimpleSegment::new(self.start_offset(), self.length()) }

    pub fn update_on_document_change(self, change: &DocumentChangeEventArgs) -> Self {
        if self.is_empty() {
            return Self::empty(change.get_new_offset(self.caret, AnchorMovementType::AfterInsertion));
        }
        let start = change.get_new_offset(self.start_offset(), AnchorMovementType::BeforeInsertion);
        let end = change.get_new_offset(self.end_offset(), AnchorMovementType::AfterInsertion);
        if self.anchor <= self.caret { Self::new(start, end) } else { Self::new(end, start) }
    }
}

/// A nonempty, ordered collection of nonoverlapping selections.
/// Overlapping ranges and duplicate carets are coalesced before editing, so a
/// character is never replaced twice by a multicaret operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    segments: Vec<SelectionSegment>,
    primary: usize,
}

impl Default for Selection {
    fn default() -> Self { Self::empty(0) }
}

impl Selection {
    pub fn empty(offset: usize) -> Self { Self { segments: vec![SelectionSegment::empty(offset)], primary: 0 } }
    pub fn simple(anchor: usize, caret: usize) -> Self { Self { segments: vec![SelectionSegment::new(anchor, caret)], primary: 0 } }

    pub fn from_segments(segments: impl IntoIterator<Item = SelectionSegment>, primary: usize) -> Self {
        let mut input: Vec<_> = segments.into_iter().collect();
        if input.is_empty() { return Self::default(); }
        let primary_segment = input[primary.min(input.len() - 1)];
        input.sort_by_key(|s| (s.start_offset(), s.end_offset()));
        let mut output: Vec<SelectionSegment> = Vec::with_capacity(input.len());
        for s in input {
            if let Some(last) = output.last_mut() {
                // Adjacent nonempty selections stay separate; a caret at a
                // selection boundary is covered by that selection.
                if s.start_offset() < last.end_offset()
                    || (s.start_offset() == last.end_offset() && (s.is_empty() || last.is_empty())) {
                    let start = last.start_offset();
                    let end = last.end_offset().max(s.end_offset());
                    let backwards = last.anchor > last.caret || s == primary_segment && s.anchor > s.caret;
                    *last = if backwards { SelectionSegment::new(end, start) } else { SelectionSegment::new(start, end) };
                    continue;
                }
            }
            output.push(s);
        }
        let primary = output.iter().position(|s| s.contains(primary_segment.caret)).unwrap_or(0);
        Self { segments: output, primary }
    }

    pub fn segments(&self) -> &[SelectionSegment] { &self.segments }
    pub fn primary_index(&self) -> usize { self.primary }
    pub fn primary(&self) -> SelectionSegment { self.segments[self.primary] }
    pub fn is_empty(&self) -> bool { self.segments.iter().all(|s| s.is_empty()) }
    pub fn length(&self) -> usize { self.segments.iter().map(|s| s.length()).sum() }
    pub fn surrounding_segment(&self) -> SimpleSegment {
        let start = self.segments[0].start_offset();
        SimpleSegment::new(start, self.segments.last().unwrap().end_offset() - start)
    }
    pub fn update_on_document_change(&self, change: &DocumentChangeEventArgs) -> Self {
        Self::from_segments(self.segments.iter().map(|s| s.update_on_document_change(change)), self.primary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coalesces_overlap_and_duplicate_carets_but_preserves_adjacent_ranges() {
        let selection = Selection::from_segments([
            SelectionSegment::new(8, 3), SelectionSegment::new(4, 9),
            SelectionSegment::empty(4), SelectionSegment::new(9, 12),
            SelectionSegment::empty(20), SelectionSegment::empty(20),
        ], 0);
        assert_eq!(selection.segments(), &[SelectionSegment::new(9, 3), SelectionSegment::new(9, 12), SelectionSegment::empty(20)]);
        assert_eq!(selection.primary().caret, 3);
    }
}
