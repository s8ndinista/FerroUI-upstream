use super::{caret_navigation::line_ranges, Selection, SelectionSegment};

/// Rectangular selection expressed in zero-based lines and visual columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectangleSelection {
    pub anchor_line: usize,
    pub anchor_column: usize,
    pub caret_line: usize,
    pub caret_column: usize,
}

impl RectangleSelection {
    pub fn to_selection(self, text: &str, tab_size: usize) -> Selection {
        let lines = line_ranges(text);
        let units: Vec<_> = text.encode_utf16().collect();
        let first = self.anchor_line.min(self.caret_line).min(lines.len() - 1);
        let last = self.anchor_line.max(self.caret_line).min(lines.len() - 1);
        let segments = (first..=last).map(|line| {
            let (start, end, _) = lines[line];
            let content = &units[start..end];
            SelectionSegment::new(
                start + offset_at_column(content, self.anchor_column, tab_size),
                start + offset_at_column(content, self.caret_column, tab_size),
            )
        });
        Selection::from_segments(
            segments,
            if self.caret_line < self.anchor_line {
                0
            } else {
                last - first
            },
        )
    }
}

pub(crate) fn offset_at_column(text: &[u16], column: usize, tab_size: usize) -> usize {
    let tab_size = tab_size.max(1);
    let (mut visual, mut offset) = (0, 0);
    while offset < text.len() && visual < column {
        if text[offset] == 9 {
            visual += tab_size - visual % tab_size;
            offset += 1;
        } else {
            visual += 1;
            let surrogate = (0xD800..=0xDBFF).contains(&text[offset])
                && text
                    .get(offset + 1)
                    .is_some_and(|c| (0xDC00..=0xDFFF).contains(c));
            offset += if surrogate { 2 } else { 1 };
        }
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tabs_short_lines_and_surrogates() {
        let selection = RectangleSelection {
            anchor_line: 0,
            anchor_column: 4,
            caret_line: 2,
            caret_column: 6,
        }
        .to_selection("\tabc\n😀abc\nx", 4);
        assert_eq!(
            selection.segments(),
            &[
                SelectionSegment::new(1, 3),
                SelectionSegment::empty(10),
                SelectionSegment::empty(12)
            ]
        );
    }
}
