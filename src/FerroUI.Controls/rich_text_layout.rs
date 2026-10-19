//! Block layout and reusable page continuations over the framework text formatter.
//! Source/dependency mapping and validation: docs/porting/winui-rich-text.

use crate::documents::{
    Block, EmbeddedControlRun, Inline, LineSpacingRule, LineStackingStrategy, Paragraph,
    RichTextDocument, TextHighlightRegion,
};
use crate::text_block::InlinesTextSource;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink,
    InterWordJustification, LogicalTextRunEnumerator, TextCollapsingProperties, TextEndOfParagraph,
    TextFormatter, TextFormatterImpl, TextLine, TextLineBreak, TextLineImpl,
    TextParagraphProperties, TextRunCache, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, CharacterHit, TextAlignment, TextCollapsingCreateInfo, TextTrimming,
    TextWrapping, Typeface,
};
use ferroui_base::utilities::ValueSpan;
use ferroui_base::{Point, Rect, Ref, Size, Vector};
use std::ops::Range;
use std::rc::Rc;

pub(crate) struct RichParagraphSource {
    pub block: Ref<Block>,
    pub source: InlinesTextSource,
    pub offset: usize,
    pub length: usize,
    pub properties: Rc<dyn TextRunProperties>,
    pub cache: TextRunCache,
}

/// A revision of the document's formatter input, shared by all overflow views.
pub(crate) struct RichTextSource {
    pub paragraphs: Vec<RichParagraphSource>,
    pub length: usize,
    pub default_indent: f64,
    pub highlights: Vec<TextHighlightRegion>,
}

impl RichTextSource {
    pub fn new(document: &RichTextDocument, view: &crate::RichTextBlock) -> Rc<Self> {
        let mut blocks = document.blocks().snapshot().to_vec();
        // The empty editor still has a line with a measurable caret.
        let empty = blocks.is_empty();
        if empty {
            let paragraph = Paragraph::new();
            paragraph.set_flow_direction(view.flow_direction());
            paragraph.set_text_alignment(view.text_alignment());
            paragraph.set_line_height(view.line_height());
            paragraph.set_line_stacking_strategy(view.line_stacking_strategy());
            paragraph.set_letter_spacing(view.letter_spacing());
            blocks.push(paragraph.upcast());
        }
        let mut offset = 0;
        let highlights = view.highlight_regions();
        let mut paragraphs = Vec::with_capacity(blocks.len());
        for block in blocks {
            let mut runs = Vec::new();
            block.build_text_runs(&mut runs);
            let length = block.text_length();
            let properties: Rc<dyn TextRunProperties> = if empty {
                Rc::new(GenericTextRunProperties::with_all(
                    Typeface::with_style(
                        view.font_family(),
                        view.font_style(),
                        view.font_weight(),
                        view.font_stretch(),
                    ),
                    view.font_size(),
                    view.text_decorations(),
                    view.foreground(),
                    None,
                    BaselineAlignment::Baseline,
                    None,
                    view.font_features(),
                ))
            } else {
                Rc::new(GenericTextRunProperties::with_all(
                    Typeface::with_style(
                        block.font_family(),
                        block.font_style(),
                        block.font_weight(),
                        block.font_stretch(),
                    ),
                    block.font_size(),
                    block.get_value(Inline::text_decorations_property()),
                    block.foreground(),
                    block.background(),
                    BaselineAlignment::Baseline,
                    None,
                    block.font_features(),
                ))
            };
            let modifiers = formatting_overrides(&runs, offset, &highlights, view);
            paragraphs.push(RichParagraphSource {
                block,
                source: InlinesTextSource::new(Rc::new(runs), modifiers),
                offset,
                length,
                properties,
                cache: TextRunCache::new(),
            });
            offset += length + 1;
        }
        Rc::new(Self {
            paragraphs,
            length: document.text_length(),
            default_indent: view.text_indent(),
            highlights,
        })
    }
}

fn formatting_overrides(
    runs: &[Rc<dyn TextRun>],
    paragraph_offset: usize,
    highlights: &[TextHighlightRegion],
    view: &crate::RichTextBlock,
) -> Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> {
    let selected = view.selection_start().min(view.selection_end())
        ..view.selection_start().max(view.selection_end());
    let selection = view.is_text_selection_enabled() && !selected.is_empty();
    if highlights.is_empty() && !selection {
        return None;
    }
    let mut modifiers = Vec::new();
    let mut position = paragraph_offset;
    for run in runs {
        let end = position + run.length().max(0) as usize;
        if let Some(properties) = run.properties() {
            let mut boundaries = vec![position, end];
            let regions: Vec<_> = highlights
                .iter()
                .filter(|region| region.range.start < end && region.range.end > position)
                .collect();
            for region in &regions {
                boundaries.push(region.range.start.max(position));
                boundaries.push(region.range.end.min(end));
            }
            if selection && selected.start < end && selected.end > position {
                boundaries.push(selected.start.max(position));
                boundaries.push(selected.end.min(end));
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            for boundary in boundaries.windows(2) {
                let start = boundary[0];
                let end = boundary[1];
                let highlight = regions
                    .iter()
                    .find(|region| region.range.start <= start && region.range.end >= end);
                let selected = selection && start >= selected.start && end <= selected.end;
                if highlight.is_none() && !selected {
                    continue;
                }
                let foreground = if selected {
                    view.selection_foreground_brush()
                } else {
                    None
                }
                .or_else(|| highlight.and_then(|region| region.foreground.clone()))
                .or_else(|| properties.foreground_brush().cloned());
                let background = if highlight.is_some()
                    || selected && view.selection_highlight_color().is_some()
                {
                    None
                } else {
                    properties.background_brush().cloned()
                };
                let override_properties: Rc<dyn TextRunProperties> =
                    Rc::new(GenericTextRunProperties::with_all(
                        properties.typeface().clone(),
                        properties.font_rendering_em_size(),
                        properties.text_decorations().cloned(),
                        foreground,
                        background,
                        properties.baseline_alignment(),
                        properties.culture_info().cloned(),
                        properties.font_features().cloned(),
                    ));
                let (Ok(start), Ok(length)) = (
                    i32::try_from(start - paragraph_offset),
                    i32::try_from(end - start),
                ) else {
                    continue;
                };
                modifiers.push(ValueSpan::new(start, length, override_properties));
            }
        }
        position = end;
    }
    (!modifiers.is_empty()).then(|| modifiers.into())
}

impl Drop for RichTextSource {
    fn drop(&mut self) {
        for paragraph in &self.paragraphs {
            paragraph.cache.dispose();
        }
    }
}

/// State at a page boundary. The break keeps the shaped, bidi-resolved remainder
/// of the paragraph; each reflow consumes a fork, never the stored continuation.
#[derive(Clone, Default)]
pub(crate) struct RichTextBreak {
    pub paragraph: usize,
    pub local_position: usize,
    pub position: usize,
    pub previous_line_break: Option<Rc<TextLineBreak>>,
}

/// Public metrics in document UTF-16 coordinates and view-local pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct RichTextLineMetrics {
    pub start: usize,
    pub length: usize,
    pub bounds: Rect,
    pub baseline: f64,
}

pub(crate) struct RichTextLine {
    pub line: Rc<dyn TextLine>,
    pub paragraph_offset: usize,
    pub paragraph_length: usize,
    pub origin: Point,
    pub advance: f64,
    right_margin: f64,
    source_spans: Option<Vec<LineSourceSpan>>,
}

/// Retained source positions and their compacted formatter indices. The
/// synthetic ellipsis is deliberately absent from these spans.
struct LineSourceSpan {
    source: Range<usize>,
    display: Range<usize>,
}

impl RichTextLine {
    pub fn start(&self) -> usize {
        self.paragraph_offset + self.line.first_text_source_index().max(0) as usize
    }

    pub fn end(&self) -> usize {
        (self.start() + self.line.length().max(0) as usize)
            .min(self.paragraph_offset + self.paragraph_length)
    }

    fn visible_ranges(&self) -> Vec<Range<usize>> {
        match &self.source_spans {
            None => vec![self.start()..self.end()],
            Some(spans) => spans
                .iter()
                .filter_map(|span| {
                    let start = (self.paragraph_offset + span.source.start).max(self.start());
                    let end = (self.paragraph_offset + span.source.end).min(self.end());
                    (start < end).then_some(start..end)
                })
                .collect(),
        }
    }

    fn source_position(&self, display: usize) -> usize {
        let Some(spans) = &self.source_spans else {
            return display;
        };
        for span in spans {
            if display >= span.display.start && display <= span.display.end {
                return span.source.start + display - span.display.start;
            }
        }
        spans
            .iter()
            .rev()
            .find(|span| span.display.end <= display)
            .map(|span| span.source.end)
            .or_else(|| spans.first().map(|span| span.source.start))
            .unwrap_or(self.line.first_text_source_index().max(0) as usize)
    }

    fn display_position(&self, source: usize) -> usize {
        let Some(spans) = &self.source_spans else {
            return source;
        };
        for span in spans {
            if source >= span.source.start && source <= span.source.end {
                return span.display.start + source - span.source.start;
            }
        }
        spans
            .iter()
            .rev()
            .find(|span| span.source.end <= source)
            .map(|span| span.display.end)
            .or_else(|| spans.first().map(|span| span.display.start))
            .unwrap_or(self.line.first_text_source_index().max(0) as usize)
    }
}

fn collapse_line(
    line: Rc<dyn TextLine>,
    properties: Rc<dyn TextCollapsingProperties>,
) -> (Rc<dyn TextLine>, Option<Vec<LineSourceSpan>>) {
    let mut originals = Vec::new();
    let mut cursor = line.first_text_source_index().max(0) as usize;
    let mut enumerator = LogicalTextRunEnumerator::new(&*line);
    while let Some(run) = enumerator.move_next() {
        let end = cursor + run.length().max(0) as usize;
        originals.push((cursor..end, run.text().clone(), run));
        cursor = end;
    }
    drop(enumerator);
    // The formatter transfers retained shaped-run ownership during collapse,
    // just as BaseTextLayout does. Disposing the old line would release it twice.
    let collapsed = line.collapse(&[Some(properties)]);
    if !collapsed.has_collapsed() {
        return (collapsed, None);
    }
    let mut spans = Vec::new();
    let mut cursor = collapsed.first_text_source_index().max(0) as usize;
    let mut enumerator = LogicalTextRunEnumerator::new(&*collapsed);
    while let Some(run) = enumerator.move_next() {
        let length = run.length().max(0) as usize;
        let text = run.text();
        let original = originals.iter().find_map(|(range, memory, original)| {
            if Rc::ptr_eq(original, &run) {
                return Some(range.start..range.start + length);
            }
            if length > 0
                && !text.is_empty()
                && memory.shares_owner_with(&text)
                && text.offset_in_owner() >= memory.offset_in_owner()
            {
                let skip = text.offset_in_owner() - memory.offset_in_owner();
                if skip + length <= range.len() {
                    return Some(range.start + skip..range.start + skip + length);
                }
            }
            None
        });
        if let Some(source) = original {
            spans.push(LineSourceSpan {
                source,
                display: cursor..cursor + length,
            });
        }
        cursor += length;
    }
    drop(enumerator);
    (collapsed, Some(spans))
}

impl Drop for RichTextLine {
    fn drop(&mut self) {
        self.line.dispose();
    }
}

pub(crate) struct RichTextLayout {
    // Retain input and caches until the last line that uses them is dropped.
    pub lines: Vec<RichTextLine>,
    pub source: Rc<RichTextSource>,
    pub size: Size,
    pub next: RichTextBreak,
    pub has_overflow: bool,
    pub is_trimmed: bool,
}

impl RichTextLayout {
    pub fn new(
        source: Rc<RichTextSource>,
        constraint: Size,
        wrapping: TextWrapping,
        max_lines: usize,
        trimming: &Rc<dyn TextTrimming>,
        terminal: bool,
        start: &RichTextBreak,
    ) -> Rc<Self> {
        let formatter = <dyn TextFormatter>::current();
        let mut result = Self {
            lines: Vec::new(),
            source,
            size: Size::default(),
            next: start.clone(),
            has_overflow: false,
            is_trimmed: false,
        };
        let width = if constraint.width.is_nan() {
            f64::INFINITY
        } else {
            constraint.width.max(0.0)
        };
        let height = if constraint.height.is_nan() {
            f64::INFINITY
        } else {
            constraint.height.max(0.0)
        };
        let mut y = 0.0_f64;
        let mut widest = 0.0_f64;

        'paragraphs: for paragraph_index in start.paragraph..result.source.paragraphs.len() {
            let paragraph = &result.source.paragraphs[paragraph_index];
            let block = &paragraph.block;
            let mut margin = block.margin();
            if let Some(paragraph) = block.to_ref().cast::<Paragraph>() {
                margin.left += paragraph.left_indent();
                margin.right += paragraph.right_indent();
            }
            let mut local = if paragraph_index == start.paragraph {
                start.local_position
            } else {
                0
            };
            let mut previous = if paragraph_index == start.paragraph {
                start.previous_line_break.clone()
            } else {
                None
            };
            let mut previous_is_shared = previous.is_some();
            if local == 0 {
                y += margin.top;
            }
            let paragraph_width = (width - margin.left - margin.right).max(0.0);
            let line_height = block.line_height();
            let strategy = block.line_stacking_strategy();
            let (spacing_rule, spacing) = block
                .to_ref()
                .cast::<Paragraph>()
                .map(|paragraph| (paragraph.line_spacing_rule(), paragraph.line_spacing()))
                .unwrap_or((LineSpacingRule::Undefined, 1.0));
            let mut previous_baseline: Option<f64> = None;

            loop {
                let indent = if local == 0 {
                    block
                        .to_ref()
                        .cast::<Paragraph>()
                        .filter(|p| p.is_set(Paragraph::text_indent_property().as_property()))
                        .map_or(result.source.default_indent, |p| p.text_indent())
                } else {
                    0.0
                };
                // Indent is placed outside the formatter so both wrapping and
                // embedded controls use the same available width and origin.
                let line_width = (paragraph_width - indent).max(0.0);
                let properties: Rc<dyn TextParagraphProperties> =
                    Rc::new(GenericTextParagraphProperties::with_all(
                        block.flow_direction(),
                        block.text_alignment(),
                        local == 0,
                        false,
                        paragraph.properties.clone(),
                        wrapping,
                        if spacing_rule == LineSpacingRule::Exactly {
                            spacing
                        } else if spacing_rule == LineSpacingRule::Undefined
                            && strategy == LineStackingStrategy::BlockLineHeight
                        {
                            line_height
                        } else {
                            f64::NAN
                        },
                        0.0,
                        block.letter_spacing(),
                    ));
                let input_break = previous.as_ref().map(|value| value.fork());
                let Some(index) = i32::try_from(local).ok() else {
                    result.has_overflow = true;
                    break 'paragraphs;
                };
                let formatted = formatter.format_line_with_cache(
                    &paragraph.source,
                    index,
                    line_width.max(f64::MIN_POSITIVE),
                    &properties,
                    input_break.as_ref(),
                    Some(&paragraph.cache),
                );
                let empty_line = formatted.is_none();
                let mut line = formatted.unwrap_or_else(|| {
                    TextFormatterImpl::create_empty_text_line(index, line_width, &properties)
                });
                let advance = if spacing_rule != LineSpacingRule::Undefined {
                    spacing_rule.advance(line.height(), spacing)
                } else {
                    match strategy {
                        LineStackingStrategy::BlockLineHeight
                        | LineStackingStrategy::BaselineToBaseline
                            if line_height.is_finite() && line_height > 0.0 =>
                        {
                            line_height
                        }
                        _ if line_height.is_finite() => line.height().max(line_height),
                        _ => line.height(),
                    }
                }
                .max(f64::MIN_POSITIVE);
                let mut origin_y = y;
                if spacing_rule == LineSpacingRule::Undefined
                    && strategy == LineStackingStrategy::BaselineToBaseline
                    && line_height.is_finite()
                {
                    if let Some(baseline) = previous_baseline {
                        origin_y = baseline + line_height - line.baseline();
                    }
                }
                // A first oversized line is accepted to guarantee forward progress
                // through columns smaller than a font's metrics. The control clips it.
                if !result.lines.is_empty()
                    && ((height.is_finite() && origin_y + advance > height + 1e-8)
                        || (max_lines > 0 && result.lines.len() >= max_lines))
                {
                    line.dispose();
                    result.next = RichTextBreak {
                        paragraph: paragraph_index,
                        local_position: local,
                        position: paragraph.offset + local,
                        previous_line_break: previous,
                    };
                    result.has_overflow = true;
                    break 'paragraphs;
                }

                if !previous_is_shared {
                    if let Some(previous) = &previous {
                        previous.release_remaining_runs();
                    }
                }

                let line_break = line.text_line_break();
                let end_of_paragraph = empty_line
                    || line_break.as_ref().is_some_and(|b| {
                        b.text_end_of_line()
                            .is_some_and(|run| run.is::<TextEndOfParagraph>())
                    });
                if block.text_alignment() == TextAlignment::Justify
                    && line_width.is_finite()
                    && !end_of_paragraph
                    && line.new_line_length() == 0
                {
                    line.justify(&InterWordJustification::new(line_width));
                }
                let consumed = line.length().max(0) as usize;
                let mut source_spans = None;
                if line.has_overflowed() && !Rc::ptr_eq(trimming, &<dyn TextTrimming>::none()) {
                    let collapsing =
                        trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
                            line_width,
                            paragraph.properties.clone(),
                            block.flow_direction(),
                        ));
                    (line, source_spans) = collapse_line(line, collapsing);
                }
                widest = widest.max(
                    margin.left
                        + indent
                        + line.start()
                        + line.width_including_trailing_whitespace()
                        + margin.right,
                );
                previous_baseline = Some(origin_y + line.baseline());
                result.lines.push(RichTextLine {
                    line,
                    paragraph_offset: paragraph.offset,
                    paragraph_length: paragraph.length,
                    origin: Point::new(margin.left + indent, origin_y),
                    advance,
                    right_margin: margin.right,
                    source_spans,
                });
                y = origin_y + advance;
                local = local.saturating_add(consumed).min(paragraph.length);
                previous = line_break;
                previous_is_shared = false;
                if end_of_paragraph || consumed == 0 {
                    y += margin.bottom;
                    result.next = RichTextBreak {
                        paragraph: paragraph_index + 1,
                        local_position: 0,
                        position: (paragraph.offset + paragraph.length + 1)
                            .min(result.source.length),
                        previous_line_break: None,
                    };
                    break;
                }
            }
        }
        if result.has_overflow && terminal && !Rc::ptr_eq(trimming, &<dyn TextTrimming>::none()) {
            if let Some(entry) = result.lines.last_mut() {
                if !entry.line.has_collapsed() {
                    let paragraph = result
                        .source
                        .paragraphs
                        .iter()
                        .find(|paragraph| paragraph.offset == entry.paragraph_offset)
                        .unwrap();
                    let collapsing =
                        trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
                            entry.line.width_including_trailing_whitespace(),
                            paragraph.properties.clone(),
                            paragraph.block.flow_direction(),
                        ));
                    let (line, spans) = collapse_line(entry.line.clone(), collapsing);
                    entry.line = line;
                    entry.source_spans = spans;
                }
            }
        }
        result.is_trimmed = result.lines.iter().any(|entry| entry.line.has_collapsed());
        widest = result
            .lines
            .iter()
            .map(|entry| {
                entry.origin.x
                    + entry.line.start()
                    + entry.line.width_including_trailing_whitespace()
                    + entry.right_margin
            })
            .fold(0.0_f64, f64::max);
        result.size = Size::new(widest.max(0.0), y.max(0.0));
        Rc::new(result)
    }

    pub fn draw(&self, sink: &mut dyn ITextDrawingSink, origin: Point) {
        for entry in &self.lines {
            entry.line.draw(
                sink,
                Point::new(origin.x + entry.origin.x, origin.y + entry.origin.y),
            );
        }
    }

    pub fn metrics(&self, origin: Point) -> Vec<RichTextLineMetrics> {
        self.lines
            .iter()
            .map(|entry| RichTextLineMetrics {
                start: entry.start(),
                length: entry.end().saturating_sub(entry.start()),
                bounds: Rect::new(
                    origin.x + entry.origin.x + entry.line.start(),
                    origin.y + entry.origin.y,
                    entry.line.width_including_trailing_whitespace(),
                    entry.advance,
                ),
                baseline: origin.y + entry.origin.y + entry.line.baseline(),
            })
            .collect()
    }

    pub fn hit_test(&self, point: Point) -> usize {
        let Some(entry) = self
            .lines
            .iter()
            .find(|entry| point.y < entry.origin.y + entry.advance)
            .or_else(|| self.lines.last())
        else {
            return self.next.position;
        };
        let hit = entry
            .line
            .get_character_hit_from_distance(point.x - entry.origin.x);
        let position = hit
            .first_character_index()
            .saturating_add(hit.trailing_length())
            .max(0) as usize;
        (entry.paragraph_offset + entry.source_position(position))
            .min(entry.end())
            .min(self.source.length)
    }

    pub fn caret_rect(&self, position: usize) -> Rect {
        self.caret_rect_with_gravity(position, crate::documents::TextGravity::Forward)
    }

    pub fn caret_rect_with_gravity(
        &self,
        position: usize,
        gravity: crate::documents::TextGravity,
    ) -> Rect {
        let Some(entry) = self
            .lines
            .iter()
            .find(|entry| position >= entry.start() && position < entry.end())
            .or_else(|| {
                self.lines
                    .iter()
                    .rev()
                    .find(|entry| position >= entry.start())
            })
            .or_else(|| self.lines.first())
        else {
            return Rect::default();
        };
        let local = position
            .saturating_sub(entry.paragraph_offset)
            .min(entry.paragraph_length);
        let local = entry.display_position(local).min(i32::MAX as usize) as i32;
        let leading = CharacterHit::new(local);
        let hit = if gravity == crate::documents::TextGravity::Backward && position > entry.start()
        {
            let previous = entry.line.get_previous_caret_character_hit(leading);
            CharacterHit::with_trailing_length(
                previous.first_character_index(),
                local.saturating_sub(previous.first_character_index()),
            )
        } else {
            leading
        };
        let x = entry.line.get_distance_from_character_hit(hit);
        Rect::new(entry.origin.x + x, entry.origin.y, 1.0, entry.advance)
    }

    pub fn selection_rects(&self, start: usize, end: usize) -> Vec<Rect> {
        let mut result = Vec::new();
        for entry in &self.lines {
            for visible in entry.visible_ranges() {
                let a = start.max(visible.start);
                let b = end.min(visible.end);
                if a < b {
                    let a = entry.display_position(a - entry.paragraph_offset);
                    let b = entry.display_position(b - entry.paragraph_offset);
                    for bounds in entry.line.get_text_bounds(a as i32, (b - a) as i32) {
                        result.push(
                            bounds
                                .rectangle()
                                .translate(Vector::new(entry.origin.x, entry.origin.y)),
                        );
                    }
                }
            }
            let paragraph_end = entry.paragraph_offset + entry.paragraph_length;
            if start <= paragraph_end
                && end > paragraph_end
                && entry.end() == paragraph_end
                && (entry.source_spans.is_none()
                    || entry
                        .visible_ranges()
                        .iter()
                        .any(|range| range.end == paragraph_end))
            {
                let caret = self.caret_rect(paragraph_end);
                result.push(Rect::new(caret.x, caret.y, 4.0, caret.height));
            }
        }
        result
    }

    pub fn visible_text_ranges(&self) -> Vec<Range<usize>> {
        let mut ranges: Vec<_> = self
            .lines
            .iter()
            .flat_map(RichTextLine::visible_ranges)
            .collect();
        ranges.sort_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(last) = merged.last_mut() {
                if range.start <= last.end {
                    last.end = last.end.max(range.end);
                    continue;
                }
            }
            merged.push(range);
        }
        merged
    }

    pub fn embedded_controls(&self, origin: Point) -> Vec<(Ref<crate::Control>, Rect)> {
        let mut result = Vec::new();
        for entry in &self.lines {
            let mut x = origin.x + entry.origin.x + entry.line.start();
            for run in entry.line.text_runs().iter() {
                if let Some(drawable) = run.as_drawable() {
                    if let Some(embedded) = run.downcast_ref::<EmbeddedControlRun>() {
                        let child = embedded.control().clone();
                        let y = origin.y
                            + entry.origin.y
                            + TextLineImpl::get_baseline_offset(&*entry.line, drawable);
                        result.push((
                            child.clone(),
                            Rect::from_position_size(Point::new(x, y), child.desired_size()),
                        ));
                    }
                    x += drawable.size().width;
                }
            }
        }
        result
    }
}
