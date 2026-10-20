//! TOM logical navigation. Native Word/Sentence/Paragraph definitions come from
//! the pinned Microsoft.UI.Text.xml; visual line units use FerroUI's live layout.
use super::*;
use crate::documents::EmbeddedControlRun;
use crate::rich_text_layout::RichTextLayout;
use ferroui_base::media::text_formatting::{LogicalTextRunEnumerator, ShapedTextRun};
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
        let mutation = document.mutation_version();
        let text = document.text();
        let length = text.encode_utf16().count();
        let display_unit = matches!(unit, TextRangeUnit::Line | TextRangeUnit::Cluster);
        let view = if display_unit {
            editor.text_view()
        } else {
            None
        };
        let ranges = match unit {
            // Native positions count UTF-16. The safe Rust range model never
            // exposes half a surrogate; one Character traverses a whole scalar.
            TextRangeUnit::Character => scalar_ranges(&text),
            TextRangeUnit::Word => word_ranges(&text),
            TextRangeUnit::Sentence => sentence_ranges(&text),
            TextRangeUnit::Paragraph => paragraph_ranges(&text),
            TextRangeUnit::Line => match &view {
                Some(view) => line_ranges(&document, &text, view.clone())?,
                None => paragraph_ranges(&text),
            },
            TextRangeUnit::Story => vec![0..length],
            TextRangeUnit::HardParagraph => hard_paragraph_ranges(&document.snapshot()),
            TextRangeUnit::Cluster => match &view {
                Some(view) => cluster_ranges(&document, &text, view.clone())?,
                None => {
                    return Err(DocumentError::InvalidFormat(
                        "Cluster navigation requires an associated shaped text view.",
                    )
                    .into());
                }
            },
        };
        // Formatting a view can raise layout-property callbacks. Do not use a
        // stale boundary snapshot if an application edited during those callbacks.
        // Public revision events can be deferred by an outer document batch.
        // The internal mutation identity also detects edits inside such a batch.
        if document.mutation_version() != mutation || display_unit && editor.text_view() != view {
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
    let utf16: Vec<_> = text.encode_utf16().collect();
    let mut result = Vec::new();
    for layout in story_layouts(document, first)? {
        for line in &layout.lines {
            let start = line.start();
            let end = line.end();
            if start > end || end > utf16.len() {
                return Err(EditError::ReentrantEdit);
            }
            for position in [start, end] {
                if !is_scalar_boundary(&utf16, position) {
                    return Err(DocumentError::InvalidUtf16Boundary { offset: position }.into());
                }
            }
            result.push(start..end);
        }
    }
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

/// Capture one coherent layout chain before reading its runs. Public document
/// revisions do not detect inherited presentation changes or view replacement.
/// Checking cached identity after all layout callbacks detects those changes
/// without reshaping or allocating glyph runs on the ordinary cached path.
fn story_layouts(
    document: &RichTextDocument,
    first: Ref<RichTextBlock>,
) -> Result<Vec<Rc<RichTextLayout>>, EditError> {
    let mut current = Some(first);
    let mut visited = HashSet::new();
    let mut snapshots = Vec::new();
    while let Some(view) = current {
        if !visited.insert(&*view as *const RichTextBlock as usize) {
            return Err(
                DocumentError::InvalidFormat("The text view chain contains a cycle.").into(),
            );
        }
        ensure_view_document(document, &view)?;
        let layout = view.page_layout();
        ensure_view_document(document, &view)?;
        if layout.is_trimmed {
            return Err(DocumentError::InvalidFormat(
                "Text-unit navigation requires a complete, untrimmed text layout.",
            )
            .into());
        }
        current = view.overflow_content_target().map(Ref::upcast);
        snapshots.push((view, layout));
    }
    for (view, layout) in &snapshots {
        ensure_view_document(document, view)?;
        if !Rc::ptr_eq(layout, &view.page_layout()) {
            return Err(EditError::ReentrantEdit);
        }
    }
    Ok(snapshots.into_iter().map(|(_, layout)| layout).collect())
}

/// Uses the same logical run order and cluster origins as the live formatter.
/// A visual-order scan would reverse RTL units, while grapheme segmentation
/// cannot recover font-dependent ligatures or complex-script shaping clusters.
fn cluster_ranges(
    document: &RichTextDocument,
    text: &str,
    first: Ref<RichTextBlock>,
) -> Result<Vec<Range<usize>>, EditError> {
    let utf16: Vec<_> = text.encode_utf16().collect();
    let mut result = Vec::new();
    let mut covered = 0;
    let mut has_line = false;
    for layout in story_layouts(document, first)? {
        for displayed in &layout.lines {
            let start = displayed.start();
            let end = displayed.end();
            if (!has_line && start != 0) || start < covered || start > end || end > utf16.len() {
                return Err(DocumentError::InvalidFormat(
                    "The text view cluster ranges are not contiguous.",
                )
                .into());
            }
            has_line = true;
            // Paragraph objects have one source mark outside their shaped runs.
            append_break_clusters(&utf16, covered..start, &mut result)?;
            let mut cursor = start;
            let mut runs = LogicalTextRunEnumerator::new(&*displayed.line);
            while let Some(run) = runs.move_next() {
                // The formatter's final paragraph marker and empty caret glyph
                // have no addressable source character in the document model.
                if cursor == end {
                    break;
                }
                let length = usize::try_from(run.length()).map_err(|_| {
                    DocumentError::InvalidFormat("A shaped run has a negative length.")
                })?;
                let run_end = cursor.checked_add(length).filter(|run_end| *run_end <= end);
                let run_end = run_end.ok_or(DocumentError::InvalidFormat(
                    "A shaped run exceeds its source line.",
                ))?;
                if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                    append_shaped_clusters(&utf16, cursor..run_end, shaped, &mut result)?;
                } else if run.is::<EmbeddedControlRun>() && utf16[cursor..run_end] == [0xfffc] {
                    result.push(cursor..run_end);
                } else if run.as_text_end_of_line().is_some() {
                    append_break_clusters(&utf16, cursor..run_end, &mut result)?;
                } else if length != 0 {
                    return Err(DocumentError::InvalidFormat(
                        "Cluster navigation requires shaped text or a known structural run.",
                    )
                    .into());
                }
                cursor = run_end;
            }
            if cursor != end {
                return Err(DocumentError::InvalidFormat(
                    "The shaped runs do not cover their source line.",
                )
                .into());
            }
            covered = end;
        }
    }
    if !has_line || covered != utf16.len() {
        return Err(DocumentError::InvalidFormat(
            "Cluster navigation requires layout for the complete story.",
        )
        .into());
    }
    Ok(result)
}

fn append_shaped_clusters(
    text: &[u16],
    range: Range<usize>,
    run: &ShapedTextRun,
    result: &mut Vec<Range<usize>>,
) -> Result<(), EditError> {
    let buffer = run.shaped_buffer();
    if buffer.text().span() != &text[range.clone()] {
        return Err(EditError::ReentrantEdit);
    }
    if range.is_empty() {
        return Ok(());
    }
    let glyphs = buffer.glyph_infos();
    let Some(origin) = glyphs.iter().map(|glyph| glyph.glyph_cluster).min() else {
        // Some shapers omit newline glyphs altogether. Those explicit source
        // separators remain navigable; missing ordinary text is not guessed.
        return append_break_clusters(text, range, result);
    };
    if origin < 0 {
        return Err(DocumentError::InvalidFormat("A glyph cluster has a negative origin.").into());
    }
    let mut starts = Vec::with_capacity(glyphs.len() + 2);
    starts.push(range.start);
    starts.push(range.end);
    for glyph in glyphs.iter() {
        // A wrapped/split shaped buffer retains its original cluster values.
        // Normalize its first logical cluster to this run's source position.
        let offset = usize::try_from(i64::from(glyph.glyph_cluster) - i64::from(origin))
            .ok()
            .filter(|offset| *offset < range.len())
            .ok_or(DocumentError::InvalidFormat(
                "A glyph cluster exceeds its source run.",
            ))?;
        starts.push(range.start + offset);
    }
    // A line-ending glyph can be absent even when the preceding run has glyphs.
    // Keep explicit separators separate from the last rendered cluster.
    let mut offset = range.start;
    while offset < range.end {
        if is_cluster_break(text[offset]) {
            starts.push(offset);
            offset += 1;
            if text[offset - 1] == 0x0d && text.get(offset) == Some(&0x0a) && offset < range.end {
                offset += 1;
            }
            starts.push(offset);
        } else {
            offset += 1;
        }
    }
    starts.sort_unstable();
    starts.dedup();
    if starts
        .iter()
        .any(|&position| !is_scalar_boundary(text, position))
    {
        return Err(DocumentError::InvalidFormat(
            "A glyph cluster splits a UTF-16 surrogate pair.",
        )
        .into());
    }
    result.extend(starts.windows(2).map(|pair| pair[0]..pair[1]));
    Ok(())
}

fn is_scalar_boundary(text: &[u16], position: usize) -> bool {
    position == 0
        || position == text.len()
        || !(0xd800..=0xdbff).contains(&text[position - 1])
        || !(0xdc00..=0xdfff).contains(&text[position])
}

fn is_cluster_break(unit: u16) -> bool {
    is_paragraph_separator(unit) || unit == 0x2028
}

fn append_break_clusters(
    text: &[u16],
    range: Range<usize>,
    result: &mut Vec<Range<usize>>,
) -> Result<(), EditError> {
    let mut start = range.start;
    while start < range.end {
        if !is_cluster_break(text[start]) {
            return Err(DocumentError::InvalidFormat(
                "The text view omits nonstructural source characters.",
            )
            .into());
        }
        let mut end = start + 1;
        if text[start] == 0x0d && end < range.end && text[end] == 0x0a {
            end += 1;
        }
        result.push(start..end);
        start = end;
    }
    Ok(())
}

#[cfg(test)]
#[path = "rich_edit_text_units_tests.rs"]
mod tests;
