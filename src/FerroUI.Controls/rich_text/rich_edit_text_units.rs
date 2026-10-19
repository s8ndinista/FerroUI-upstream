//! TOM logical navigation. Native Word/Sentence/Paragraph definitions come from
//! the pinned Microsoft.UI.Text.xml; visual line units use FerroUI's live layout.
use super::*;
use ferroui_base::Ref;
use std::collections::HashSet;

pub(super) struct TextUnits {
    ranges: Vec<Range<usize>>,
    boundaries: Vec<usize>,
    length: usize,
    terminal_caret_in_last_unit: bool,
}

impl TextUnits {
    pub(super) fn resolve(editor: &RichTextEditor, unit: TextRangeUnit) -> Result<Self, EditError> {
        let document = editor.document();
        let revision = document.revision();
        let text = document.text();
        let length = text.encode_utf16().count();
        let ranges = match unit {
            // Native positions count UTF-16. The safe Rust range model never
            // exposes half a surrogate; one Character traverses a whole scalar.
            TextRangeUnit::Character => scalar_ranges(&text),
            TextRangeUnit::Word => word_ranges(&text),
            TextRangeUnit::Sentence => sentence_ranges(&text),
            TextRangeUnit::Paragraph => paragraph_ranges(&text),
            TextRangeUnit::Line => match editor.text_view() {
                Some(view) => line_ranges(&document, &text, view)?,
                None => paragraph_ranges(&text),
            },
            TextRangeUnit::Story => vec![0..length],
            TextRangeUnit::HardParagraph => hard_paragraph_ranges(&document.snapshot()),
        };
        // Formatting a view can raise layout-property callbacks. Do not use a
        // stale boundary snapshot if an application edited during those callbacks.
        if document.revision() != revision {
            return Err(EditError::ReentrantEdit);
        }
        let mut boundaries: Vec<_> = ranges
            .iter()
            .flat_map(|range| [range.start, range.end])
            .collect();
        boundaries.push(0);
        boundaries.push(length);
        boundaries.sort_unstable();
        boundaries.dedup();
        Ok(Self {
            ranges,
            boundaries,
            length,
            terminal_caret_in_last_unit: matches!(
                unit,
                TextRangeUnit::Paragraph
                    | TextRangeUnit::HardParagraph
                    | TextRangeUnit::Line
                    | TextRangeUnit::Story
            ),
        })
    }

    fn containing(&self, position: usize) -> Option<&Range<usize>> {
        if position == self.length && self.terminal_caret_in_last_unit {
            return self.ranges.last();
        }
        let index = self
            .ranges
            .partition_point(|range| range.end <= position && !range.is_empty());
        self.ranges
            .get(index)
            .filter(|range| range.start <= position && (position < range.end || range.is_empty()))
    }

    pub(super) fn expanded(&self, range: Range<usize>) -> Range<usize> {
        let start = self
            .containing(range.start)
            .map_or(range.start, |unit| unit.start);
        let active = if range.is_empty() {
            range.end
        } else {
            range.end - 1
        };
        let end = self.containing(active).map_or(range.end, |unit| unit.end);
        start..end
    }

    pub(super) fn start_of(&self, position: usize) -> usize {
        self.containing(position)
            .map_or(position, |unit| unit.start)
    }

    pub(super) fn end_of(&self, position: usize) -> usize {
        // An insertion point between units stays on that boundary. Position zero
        // is the native exception: it advances to the end of the first unit.
        let active = position.saturating_sub(1);
        self.containing(active)
            .map_or(position, |unit| unit.end)
            .max(position)
    }

    pub(super) fn index(&self, position: usize) -> usize {
        if position == self.length {
            return self.ranges.len();
        }
        let preceding = self.ranges.partition_point(|range| range.end <= position);
        if preceding < self.ranges.len() {
            preceding + 1
        } else {
            0
        }
    }

    pub(super) fn indexed(&self, index: i32) -> Result<Range<usize>, EditError> {
        let offset = if index > 0 {
            i128::from(index) - 1
        } else if index < 0 {
            self.ranges.len() as i128 + i128::from(index)
        } else {
            -1
        };
        usize::try_from(offset)
            .ok()
            .and_then(|index| self.ranges.get(index))
            .cloned()
            .ok_or_else(|| {
                DocumentError::InvalidFormat("The text unit index is out of range.").into()
            })
    }

    pub(super) fn move_range(&self, range: Range<usize>, count: i32) -> (usize, i32) {
        if count == 0 {
            return (range.start, 0);
        }
        if range.is_empty() {
            return self.move_position(range.start, count);
        }
        let (position, remaining, collapse) = if count > 0 {
            let next = self
                .boundaries
                .partition_point(|boundary| *boundary < range.end);
            (self.boundaries[next], count - 1, 1)
        } else {
            let previous = self
                .boundaries
                .partition_point(|boundary| *boundary <= range.start);
            (self.boundaries[previous - 1], count + 1, -1)
        };
        let (position, moved) = self.move_position(position, remaining);
        (position, moved + collapse)
    }

    pub(super) fn move_position(&self, position: usize, count: i32) -> (usize, i32) {
        if count == 0 {
            return (position, 0);
        }
        // A single boundary lookup and a clamped displacement also handle extreme
        // i32::MIN/MAX requests without repeated scans or signed abs overflow.
        let requested = count.unsigned_abs() as usize;
        if count < 0 {
            let before = self
                .boundaries
                .partition_point(|boundary| *boundary < position);
            let moved = requested.min(before);
            if moved == 0 {
                return (position, 0);
            }
            (self.boundaries[before - moved], (-(moved as i64)) as i32)
        } else {
            let after = self
                .boundaries
                .partition_point(|boundary| *boundary <= position);
            let moved = requested.min(self.boundaries.len() - after);
            if moved == 0 {
                return (position, 0);
            }
            (self.boundaries[after + moved - 1], moved as i32)
        }
    }
}

pub(super) fn signed_displacement(to: usize, from: usize) -> Result<isize, EditError> {
    isize::try_from(to as i128 - from as i128).map_err(|_| {
        DocumentError::InvalidFormat("The text movement exceeds the signed offset range.").into()
    })
}

fn scalar_ranges(text: &str) -> Vec<Range<usize>> {
    let mut offset = 0;
    text.chars()
        .map(|character| {
            let start = offset;
            offset += character.len_utf16();
            start..offset
        })
        .collect()
}

fn word_ranges(text: &str) -> Vec<Range<usize>> {
    let utf16: Vec<_> = text.encode_utf16().collect();
    let mut result: Vec<Range<usize>> = Vec::new();
    for segment in WordBreakEnumerator::new(&utf16) {
        let range = segment.offset()..segment.offset() + segment.length();
        if utf16[range.clone()]
            .iter()
            .all(|unit| matches!(*unit, 0x09 | 0x20))
        {
            if let Some(previous) = result.last_mut() {
                previous.end = range.end;
                continue;
            }
        }
        result.push(range);
    }
    result
}

fn sentence_ranges(text: &str) -> Vec<Range<usize>> {
    let utf16: Vec<_> = text.encode_utf16().collect();
    let terminator = |unit| matches!(unit, 0x2e | 0x3f | 0x21);
    let whitespace = |unit| matches!(unit, 0x09..=0x0d | 0x20 | 0x2029);
    let mut result = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < utf16.len() {
        if terminator(utf16[index]) && utf16.get(index + 1).is_some_and(|unit| whitespace(*unit)) {
            let mut end = index + 1;
            while end < utf16.len() && whitespace(utf16[end]) {
                end += 1;
            }
            // Every later sentence must start after a qualifying terminator and
            // cannot itself start with punctuation that terminates a sentence.
            if end == utf16.len() || !terminator(utf16[end]) {
                result.push(start..end);
                start = end;
            }
            index = end;
        } else {
            index += 1;
        }
    }
    if start < utf16.len() {
        result.push(start..utf16.len());
    }
    result
}

fn is_paragraph_separator(unit: u16) -> bool {
    matches!(unit, 0x0a..=0x0d | 0x2029)
}

fn paragraph_ranges(text: &str) -> Vec<Range<usize>> {
    let utf16: Vec<_> = text.encode_utf16().collect();
    let mut ends = Vec::new();
    let mut index = 0;
    while index < utf16.len() {
        let unit = utf16[index];
        index += 1;
        if unit == 0x0d && utf16.get(index) == Some(&0x0a) {
            index += 1;
        }
        if is_paragraph_separator(unit) {
            ends.push(index);
        }
    }
    ranges_ending_at(ends, utf16.len())
}

fn hard_paragraph_ranges(snapshot: &DocumentSnapshot) -> Vec<Range<usize>> {
    fn collect(
        inlines: &[InlineSnapshot],
        offset: &mut usize,
        ends: &mut Vec<usize>,
        previous_cr: &mut bool,
    ) {
        for inline in inlines {
            if let Some(children) = inline.children() {
                collect(children, offset, ends, previous_cr);
            } else if let InlineSnapshotKind::Run(text) = &inline.kind {
                let utf16: Vec<_> = text.encode_utf16().collect();
                // A CR/LF may cross a character-format boundary between Run
                // objects. It is still one hard paragraph terminator.
                if *previous_cr && utf16.first() == Some(&0x0a) {
                    if let Some(end) = ends.last_mut() {
                        *end += 1;
                    }
                }
                let mut index = 0;
                while index < utf16.len() {
                    let unit = utf16[index];
                    index += 1;
                    if unit == 0x0d {
                        if utf16.get(index) == Some(&0x0a) {
                            index += 1;
                        }
                        ends.push(*offset + index);
                    }
                }
                *offset += utf16.len();
                if let Some(last) = utf16.last() {
                    *previous_cr = *last == 0x0d;
                }
            } else {
                // Inline LineBreak is a soft break even on platforms whose text
                // projection uses CR/LF. Paragraph objects remain hard boundaries.
                *offset += inline.text_length();
                if inline.text_length() > 0 {
                    *previous_cr = false;
                }
            }
        }
    }
    let mut ends = Vec::new();
    let mut offset = 0;
    let mut previous_cr = false;
    for (index, paragraph) in snapshot.paragraphs.iter().enumerate() {
        collect(&paragraph.inlines, &mut offset, &mut ends, &mut previous_cr);
        if index + 1 < snapshot.paragraphs.len() {
            offset += 1;
            ends.push(offset);
            previous_cr = false;
        }
    }
    ranges_ending_at(ends, offset)
}

fn ranges_ending_at(ends: Vec<usize>, length: usize) -> Vec<Range<usize>> {
    let mut start = 0;
    let mut ranges = Vec::with_capacity(ends.len() + 1);
    for end in ends {
        ranges.push(start..end);
        start = end;
    }
    // The final paragraph exists even when empty. Its terminal mark is implicit
    // in the model rather than an addressable extra UTF-16 character.
    ranges.push(start..length);
    ranges
}

fn line_ranges(
    document: &RichTextDocument,
    text: &str,
    first: Ref<RichTextBlock>,
) -> Result<Vec<Range<usize>>, EditError> {
    let length = text.encode_utf16().count();
    let mut current = Some(first);
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    while let Some(view) = current {
        if !visited.insert(&*view as *const RichTextBlock as usize) {
            return Err(
                DocumentError::InvalidFormat("The text view chain contains a cycle.").into(),
            );
        }
        ensure_view_document(document, &view)?;
        let metrics = view.line_metrics();
        if view.is_text_trimmed() {
            return Err(DocumentError::InvalidFormat(
                "Line navigation requires a complete, untrimmed text layout.",
            )
            .into());
        }
        ensure_view_document(document, &view)?;
        for line in metrics {
            let end = line
                .start
                .checked_add(line.length)
                .ok_or(DocumentError::InvalidFormat(
                    "A text line exceeds the document coordinate range.",
                ))?;
            if end > length {
                return Err(EditError::ReentrantEdit);
            }
            document.validate_range(line.start..end)?;
            result.push(line.start..end);
        }
        current = view.overflow_content_target().map(Ref::upcast);
    }
    let utf16: Vec<_> = text.encode_utf16().collect();
    if result.first().is_none_or(|range| range.start != 0)
        || result.last().is_none_or(|range| range.end != utf16.len())
    {
        return Err(DocumentError::InvalidFormat(
            "Line navigation requires layout for the complete story.",
        )
        .into());
    }
    for index in 0..result.len().saturating_sub(1) {
        let next_start = result[index + 1].start;
        let end = result[index].end;
        if end > next_start
            || !utf16[end..next_start]
                .iter()
                .all(|unit| is_paragraph_separator(*unit))
        {
            return Err(DocumentError::InvalidFormat(
                "The text view line ranges are not contiguous.",
            )
            .into());
        }
        // The formatter omits a structural paragraph mark from line metrics.
        // Assign it to the preceding display line, keeping every position covered.
        result[index].end = next_start;
    }
    Ok(result)
}

#[cfg(test)]
#[path = "rich_edit_text_units_tests.rs"]
mod tests;
