//! Block layout and reusable page continuations over the framework text formatter.
//! Source/dependency mapping and validation: docs/porting/winui-rich-text.

use crate::documents::{
    Block, EmbeddedControlRun, LineStackingStrategy, Paragraph, RichTextDocument,
};
use crate::text_block::InlinesTextSource;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink,
    InterWordJustification, TextEndOfParagraph, TextFormatter, TextFormatterImpl, TextLine,
    TextLineBreak, TextLineImpl, TextParagraphProperties, TextRunCache, TextRunProperties,
};
use ferroui_base::media::{BaselineAlignment, CharacterHit, TextAlignment, TextWrapping, Typeface};
use ferroui_base::{Point, Rect, Ref, Size, Vector};
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
}

impl RichTextSource {
    pub fn new(
        document: &RichTextDocument,
        empty_properties: Rc<dyn TextRunProperties>,
    ) -> Rc<Self> {
        let mut blocks = document.blocks().snapshot().to_vec();
        // The empty editor still has a line with a measurable caret.
        let empty = blocks.is_empty();
        if empty {
            blocks.push(Paragraph::new().upcast());
        }
        let mut offset = 0;
        let mut paragraphs = Vec::with_capacity(blocks.len());
        for block in blocks {
            let mut runs = Vec::new();
            block.build_text_runs(&mut runs);
            let length = block.text_length();
            let properties: Rc<dyn TextRunProperties> = if empty {
                empty_properties.clone()
            } else {
                Rc::new(GenericTextRunProperties::with_all(
                    Typeface::with_style(
                        block.font_family(),
                        block.font_style(),
                        block.font_weight(),
                        block.font_stretch(),
                    ),
                    block.font_size(),
                    None,
                    block.foreground(),
                    block.background(),
                    BaselineAlignment::Baseline,
                    None,
                    block.font_features(),
                ))
            };
            paragraphs.push(RichParagraphSource {
                block,
                source: InlinesTextSource::new(Rc::new(runs), None),
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
        })
    }
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
}

impl RichTextLine {
    pub fn start(&self) -> usize {
        self.paragraph_offset + self.line.first_text_source_index().max(0) as usize
    }

    pub fn end(&self) -> usize {
        (self.start() + self.line.length().max(0) as usize)
            .min(self.paragraph_offset + self.paragraph_length)
    }
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
}

impl RichTextLayout {
    pub fn new(
        source: Rc<RichTextSource>,
        constraint: Size,
        wrapping: TextWrapping,
        max_lines: usize,
        start: &RichTextBreak,
    ) -> Rc<Self> {
        let formatter = <dyn TextFormatter>::current();
        let mut result = Self {
            lines: Vec::new(),
            source,
            size: Size::default(),
            next: start.clone(),
            has_overflow: false,
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
            let margin = block.margin();
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
            let mut previous_baseline: Option<f64> = None;

            loop {
                let indent = if local == 0 {
                    block
                        .to_ref()
                        .cast::<Paragraph>()
                        .map_or(0.0, |p| p.text_indent())
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
                        if strategy == LineStackingStrategy::BlockLineHeight {
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
                let line = formatted.unwrap_or_else(|| {
                    TextFormatterImpl::create_empty_text_line(index, line_width, &properties)
                });
                let advance = match strategy {
                    LineStackingStrategy::BlockLineHeight
                    | LineStackingStrategy::BaselineToBaseline
                        if line_height.is_finite() && line_height > 0.0 =>
                    {
                        line_height
                    }
                    _ if line_height.is_finite() => line.height().max(line_height),
                    _ => line.height(),
                }
                .max(f64::MIN_POSITIVE);
                let mut origin_y = y;
                if strategy == LineStackingStrategy::BaselineToBaseline && line_height.is_finite() {
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
        (entry.paragraph_offset + position)
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
        let local = local.min(i32::MAX as usize) as i32;
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
            let a = start.max(entry.start());
            let b = end.min(entry.end());
            if a < b {
                for bounds in entry
                    .line
                    .get_text_bounds((a - entry.paragraph_offset) as i32, (b - a) as i32)
                {
                    result.push(
                        bounds
                            .rectangle()
                            .translate(Vector::new(entry.origin.x, entry.origin.y)),
                    );
                }
            }
            let paragraph_end = entry.paragraph_offset + entry.paragraph_length;
            if start <= paragraph_end && end > paragraph_end && entry.end() == paragraph_end {
                let caret = self.caret_rect(paragraph_end);
                result.push(Rect::new(caret.x, caret.y, 4.0, caret.height));
            }
        }
        result
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
