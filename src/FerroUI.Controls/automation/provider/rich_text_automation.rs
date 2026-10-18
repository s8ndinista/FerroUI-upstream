use super::{
    ITextProvider, ITextRangeProvider, ProviderAdapter, ProviderError, SupportedTextSelection,
    TextAttribute, TextAttributeValue, TextPatternRangeEndpoint, TextProviderError, TextUnit,
};
use crate::automation::ElementNotEnabledException;
use crate::automation::peers::{AutomationPeer, ControlAutomationPeer};
use crate::documents::{
    CharacterFormat, InlineSnapshot, InlineSnapshotKind, Paragraph, ParagraphFormat,
    RichTextDocument, TextGravity,
};
use crate::rich_text::{RichEditTextDocument, RichEditTextRange, TextFindOptions, TextRangeUnit};
use crate::{Control, RichEditBox, RichTextBlock, ScrollViewer};
use ferroui_base::media::{
    BaselineAlignment, Color, FlowDirection, FontStyle, IBrush, TextAlignment,
    TextDecorationLocation,
};
use ferroui_base::reactive::{CompositeDisposable, IDisposable, SerialDisposable};
use ferroui_base::{Matrix, ObjectType, Point, Rect, Ref, Vector, Visual, WeakRef};
use std::any::Any;
use std::collections::HashSet;
use std::ops::Range;
use std::rc::Rc;

/// Both rich peers expose the same document/range implementation. The owner is
/// weak because accessibility clients can retain a peer after its control dies.
pub(crate) trait RichTextAutomationPeer: ObjectType {
    fn text_owner(&self) -> Option<Ref<RichTextBlock>>;
    fn text_peer(&self) -> Ref<AutomationPeer>;
}

#[derive(Clone)]
struct TextContext {
    view: WeakRef<RichTextBlock>,
    peer: WeakRef<AutomationPeer>,
}

impl TextContext {
    fn from_peer<T: RichTextAutomationPeer>(peer: &T) -> Result<Self, ProviderError> {
        let view = peer
            .text_owner()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        Ok(Self {
            view: view.downgrade(),
            peer: peer.text_peer().downgrade(),
        })
    }

    fn view(&self) -> Result<Ref<RichTextBlock>, ProviderError> {
        self.view
            .upgrade()
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.view()?;
        self.peer
            .upgrade()
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn make_range(&self, range: Range<usize>) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let view = self.view()?;
        let service = view
            .cast::<RichEditBox>()
            .map(|editor| editor.text_document())
            .unwrap_or_else(|| RichEditTextDocument::new(view.document()));
        Ok(Rc::new(RichTextAutomationRange {
            context: self.clone(),
            range: service.get_range(range.start, range.end)?,
        }))
    }
}

impl<T: RichTextAutomationPeer> ITextProvider for ProviderAdapter<T> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.text_peer()
    }

    fn document_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let length = context.view()?.document().text_length();
        context.make_range(0..length)
    }

    fn get_selection(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        if !view.is_text_selection_enabled() {
            return Ok(Vec::new());
        }
        Ok(vec![context.make_range(selection(&view))?])
    }

    fn get_visible_ranges(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let mut visible: Vec<Range<usize>> = Vec::new();
        if let Some(geometry) = ViewGeometry::new(&view) {
            for line in view.line_metrics() {
                let bounds = geometry.visible(line.bounds);
                if bounds.is_empty() {
                    continue;
                }
                let range = line.start..line.start + line.length;
                if let Some(previous) = visible.last_mut().filter(|last| last.end == range.start) {
                    previous.end = range.end;
                } else {
                    visible.push(range);
                }
            }
        }
        // UIA requires a degenerate range when the viewport has no visible text.
        if visible.is_empty() {
            let start = view.content_start().min(view.document().text_length());
            visible.push(start..start);
        }
        visible
            .into_iter()
            .map(|range| context.make_range(range))
            .collect()
    }

    fn range_from_child(
        &self,
        child: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let children = embedded_children(&view.document());
        let mut candidate = Some(child.clone());
        while let Some(peer) = candidate {
            for (range, control) in &children {
                if ControlAutomationPeer::create_peer_for_element(control) == peer {
                    return context.make_range(range.clone());
                }
            }
            if peer == context.peer()? {
                break;
            }
            candidate = peer.get_parent();
        }
        Err(TextProviderError::InvalidArgument.into())
    }

    fn range_from_point(&self, point: Point) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let root = view
            .visual_root()
            .ok_or(TextProviderError::GeometryUnavailable)?;
        let transform = root
            .transform_to_visual(&view)
            .ok_or(TextProviderError::GeometryUnavailable)?;
        let position = view.hit_test_text_position(point.transform(transform));
        context.make_range(position..position)
    }

    fn supported_text_selection(&self) -> Result<SupportedTextSelection, ProviderError> {
        Ok(
            if TextContext::from_peer(&*self.0)?
                .view()?
                .is_text_selection_enabled()
            {
                SupportedTextSelection::Single
            } else {
                SupportedTextSelection::None
            },
        )
    }

    fn text_changed(&self, handler: Rc<dyn Fn()>) -> Result<Rc<dyn IDisposable>, ProviderError> {
        let context = TextContext::from_peer(&*self.0)?;
        let view = context.view()?;
        let document_subscription = Rc::new(SerialDisposable::new());
        let callback = handler.clone();
        document_subscription.set(Some(view.document().changed(move |_| callback())));
        let current = document_subscription.clone();
        let weak = view.downgrade();
        let property_subscription = view.property_changed(move |event| {
            if event.property() == RichTextBlock::document_property().as_property() {
                if let Some(view) = weak.upgrade() {
                    let callback = handler.clone();
                    current.set(Some(view.document().changed(move |_| callback())));
                    handler();
                }
            }
        });
        Ok(Rc::new(CompositeDisposable::from_disposables([
            property_subscription,
            document_subscription as Rc<dyn IDisposable>,
        ])))
    }

    fn text_selection_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError> {
        Ok(TextContext::from_peer(&*self.0)?
            .view()?
            .selection_changed(move |_| handler()))
    }
}

struct RichTextAutomationRange {
    context: TextContext,
    range: RichEditTextRange,
}

impl RichTextAutomationRange {
    fn view(&self) -> Result<Ref<RichTextBlock>, ProviderError> {
        let view = self.context.view()?;
        if view.document() != self.range.document() {
            return Err(TextProviderError::ElementNotAvailable.into());
        }
        Ok(view)
    }

    fn compatible<'a>(&self, other: &'a dyn ITextRangeProvider) -> Result<&'a Self, ProviderError> {
        self.view()?;
        let other = other
            .as_any()
            .downcast_ref::<Self>()
            .ok_or(TextProviderError::IncompatibleRange)?;
        other.view()?;
        if self.context.peer()? != other.context.peer()?
            || self.range.document() != other.range.document()
        {
            return Err(TextProviderError::IncompatibleRange.into());
        }
        Ok(other)
    }

    fn endpoint(&self, endpoint: TextPatternRangeEndpoint) -> usize {
        match endpoint {
            TextPatternRangeEndpoint::Start => self.range.start_position(),
            TextPatternRangeEndpoint::End => self.range.end_position(),
        }
    }

    fn set_endpoint(
        &self,
        endpoint: TextPatternRangeEndpoint,
        position: usize,
    ) -> Result<(), ProviderError> {
        match endpoint {
            TextPatternRangeEndpoint::Start => self
                .range
                .set_range(position, self.range.end_position().max(position))?,
            TextPatternRangeEndpoint::End => self
                .range
                .set_range(self.range.start_position().min(position), position)?,
        }
        Ok(())
    }

    fn select_range(&self, range: Range<usize>) -> Result<(), ProviderError> {
        let view = self.view()?;
        if !view.is_effectively_enabled() {
            return Err(Box::new(ElementNotEnabledException::new()));
        }
        if !view.is_text_selection_enabled() {
            return Err(TextProviderError::SelectionNotSupported.into());
        }
        view.select(range.start, range.end);
        Ok(())
    }

    fn units(&self, unit: TextUnit) -> Result<Vec<Range<usize>>, ProviderError> {
        let view = self.view()?;
        let document = self.range.document();
        let text = document.text();
        Ok(match unit {
            TextUnit::Character => {
                crate::rich_text::rich_text_unit_ranges(&text, TextRangeUnit::Character)
            }
            TextUnit::Word => crate::rich_text::rich_text_unit_ranges(&text, TextRangeUnit::Word),
            TextUnit::Paragraph => paragraph_units(&text),
            TextUnit::Format => format_spans(&view)
                .into_iter()
                .map(|span| span.range)
                .collect(),
            TextUnit::Line | TextUnit::Page => {
                let mut lines = Vec::new();
                let mut pages = Vec::new();
                let mut next = Some(view);
                while let Some(page) = next {
                    let metrics = page.line_metrics();
                    if !metrics.is_empty() {
                        pages.push(page.content_start()..page.content_end());
                        lines.extend(
                            metrics
                                .into_iter()
                                .map(|line| line.start..line.start + line.length),
                        );
                    }
                    next = page.overflow_content_target().map(Ref::upcast);
                }
                let complete = pages.first().is_some_and(|page| page.start == 0)
                    && pages
                        .last()
                        .is_some_and(|page| page.end == document.text_length());
                if unit == TextUnit::Line && complete {
                    lines
                } else if unit == TextUnit::Page && complete && pages.len() > 1 {
                    pages
                } else if unit == TextUnit::Line {
                    // Text not laid out in a viewport has no line boundaries. UIA
                    // promotes that unit to the next larger supported unit.
                    paragraph_units(&text)
                } else {
                    vec![0..document.text_length()]
                }
            }
            TextUnit::Document => vec![0..document.text_length()],
        })
    }
}

impl ITextRangeProvider for RichTextAutomationRange {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.view()?;
        self.context.peer()
    }

    fn clone_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        self.view()?;
        Ok(Rc::new(Self {
            context: self.context.clone(),
            range: self.range.get_clone()?,
        }))
    }

    fn compare(&self, range: &dyn ITextRangeProvider) -> Result<bool, ProviderError> {
        Ok(self.range.range() == self.compatible(range)?.range.range())
    }

    fn compare_endpoints(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<i32, ProviderError> {
        let left = self.endpoint(endpoint);
        let right = self.compatible(range)?.endpoint(target_endpoint);
        Ok((left as i128 - right as i128).clamp(i32::MIN as i128, i32::MAX as i128) as i32)
    }

    fn expand_to_enclosing_unit(&self, unit: TextUnit) -> Result<(), ProviderError> {
        let units = self.units(unit)?;
        if let Some(unit) = enclosing_unit(&units, self.range.start_position()) {
            self.range.set_range(unit.start, unit.end)?;
        }
        Ok(())
    }

    fn find_attribute(
        &self,
        attribute: TextAttribute,
        value: &TextAttributeValue,
        backwards: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        let view = self.view()?;
        if matches!(attribute, TextAttribute::Unknown(_)) {
            return Err(TextProviderError::UnsupportedAttribute.into());
        }
        if matches!(
            value,
            TextAttributeValue::Mixed | TextAttributeValue::NotSupported
        ) {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let range = self.range.range();
        let mut found: Vec<Range<usize>> = Vec::new();
        for span in format_spans(&view) {
            let selected = span.range.start.max(range.start)..span.range.end.min(range.end);
            if selected.is_empty() {
                continue;
            }
            let actual = attribute_value(&view, selected.start, false, attribute);
            if actual == TextAttributeValue::NotSupported {
                return Err(TextProviderError::UnsupportedAttribute.into());
            }
            if &actual == value {
                if let Some(previous) = found
                    .last_mut()
                    .filter(|previous| previous.end == selected.start)
                {
                    previous.end = selected.end;
                } else {
                    found.push(selected);
                }
            }
        }
        let result = if backwards {
            found.pop()
        } else {
            found.into_iter().next()
        };
        result
            .map(|range| self.context.make_range(range))
            .transpose()
    }

    fn find_text(
        &self,
        text: &str,
        backwards: bool,
        ignore_case: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        self.view()?;
        if text.is_empty() {
            return Err(TextProviderError::InvalidArgument.into());
        }
        let mut options = TextFindOptions::empty();
        options.set(TextFindOptions::BACKWARDS, backwards);
        options.set(TextFindOptions::MATCH_CASE, !ignore_case);
        self.range
            .find(text, options)?
            .map(|range| {
                Ok(Rc::new(Self {
                    context: self.context.clone(),
                    range,
                }) as Rc<dyn ITextRangeProvider>)
            })
            .transpose()
    }

    fn get_attribute_value(
        &self,
        attribute: TextAttribute,
    ) -> Result<TextAttributeValue, ProviderError> {
        let view = self.view()?;
        let range = self.range.range();
        let first = attribute_value(&view, range.start, range.is_empty(), attribute);
        if range.is_empty() || first == TextAttributeValue::NotSupported {
            return Ok(first);
        }
        for span in format_spans(&view) {
            if span.range.start > range.start && span.range.start < range.end {
                let next = attribute_value(&view, span.range.start, false, attribute);
                if next == TextAttributeValue::NotSupported {
                    return Ok(next);
                }
                if next != first {
                    return Ok(TextAttributeValue::Mixed);
                }
            }
        }
        Ok(first)
    }

    fn get_bounding_rectangles(&self) -> Result<Vec<Rect>, ProviderError> {
        let view = self.view()?;
        let Some(geometry) = ViewGeometry::new(&view) else {
            return Ok(Vec::new());
        };
        Ok(view
            .get_rects_for_range(self.range.range())
            .into_iter()
            .map(|rect| geometry.visible(rect))
            .filter(|rect| !rect.is_empty())
            .collect())
    }

    fn get_enclosing_element(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        let view = self.view()?;
        let selected = self.range.range();
        if !selected.is_empty() {
            for (range, child) in embedded_children(&view.document()) {
                if range.start <= selected.start && selected.end <= range.end {
                    return Ok(ControlAutomationPeer::create_peer_for_element(&child));
                }
            }
        }
        self.context.peer()
    }

    fn get_text(&self, max_length: Option<usize>) -> Result<String, ProviderError> {
        self.view()?;
        let text = self.range.text()?;
        let Some(max_length) = max_length else {
            return Ok(text);
        };
        let mut length = 0;
        Ok(text
            .chars()
            .take_while(|ch| {
                length += ch.len_utf16();
                length <= max_length
            })
            .collect())
    }

    fn move_by(&self, unit: TextUnit, count: i32) -> Result<i32, ProviderError> {
        let units = self.units(unit)?;
        if count == 0 || units.is_empty() {
            return Ok(0);
        }
        if self.range.is_degenerate() {
            let (position, moved) = move_position(&units, self.range.start_position(), count);
            self.range.set_range(position, position)?;
            return Ok(moved);
        }
        let nonempty: Vec<_> = units.into_iter().filter(|unit| !unit.is_empty()).collect();
        let Some(current) = enclosing_unit_index(&nonempty, self.range.start_position()) else {
            return Ok(0);
        };
        let target = (current as i128 + count as i128)
            .clamp(0, nonempty.len().saturating_sub(1) as i128) as usize;
        let moved = target as i128 - current as i128;
        if moved != 0 {
            self.range
                .set_range(nonempty[target].start, nonempty[target].end)?;
        }
        Ok(moved as i32)
    }

    fn move_endpoint_by_unit(
        &self,
        endpoint: TextPatternRangeEndpoint,
        unit: TextUnit,
        count: i32,
    ) -> Result<i32, ProviderError> {
        let units = self.units(unit)?;
        let (position, moved) = move_position(&units, self.endpoint(endpoint), count);
        self.set_endpoint(endpoint, position)?;
        Ok(moved)
    }

    fn move_endpoint_by_range(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<(), ProviderError> {
        let position = self.compatible(range)?.endpoint(target_endpoint);
        self.set_endpoint(endpoint, position)
    }

    fn select(&self) -> Result<(), ProviderError> {
        self.select_range(self.range.range())
    }

    fn add_to_selection(&self) -> Result<(), ProviderError> {
        let view = self.view()?;
        let current = selection(&view);
        let added = self.range.range();
        if current.is_empty() || added.is_empty() {
            return self.select_range(added);
        }
        if added.end < current.start || current.end < added.start {
            return Err(TextProviderError::MultipleSelectionNotSupported.into());
        }
        self.select_range(current.start.min(added.start)..current.end.max(added.end))
    }

    fn remove_from_selection(&self) -> Result<(), ProviderError> {
        let view = self.view()?;
        let current = selection(&view);
        let removed = self.range.range();
        if removed.is_empty() {
            return self.select_range(removed);
        }
        let remaining =
            if current.is_empty() || removed.end <= current.start || current.end <= removed.start {
                current
            } else if removed.start <= current.start && current.end <= removed.end {
                current.start..current.start
            } else if removed.start <= current.start {
                removed.end..current.end
            } else if current.end <= removed.end {
                current.start..removed.start
            } else {
                return Err(TextProviderError::MultipleSelectionNotSupported.into());
            };
        self.select_range(remaining)
    }

    fn scroll_into_view(&self, align_to_top: bool) -> Result<(), ProviderError> {
        let host = self.view()?;
        let position = if align_to_top {
            self.range.start_position()
        } else {
            self.range.end_position()
        };
        let view = host.text_view_for_position(position).unwrap_or(host);
        let rectangle = view.get_rect_from_character_index(position);
        if rectangle.height <= 0.0 || !view.is_arrange_valid() {
            return Err(TextProviderError::GeometryUnavailable.into());
        }
        let mut ancestor = view.visual_parent();
        let mut viewport = None;
        while let Some(parent) = ancestor {
            if parent
                .cast::<crate::presenters::ScrollContentPresenter>()
                .is_some()
            {
                viewport = Some(parent.clone());
            }
            if let Some(scroller) = parent.cast::<ScrollViewer>() {
                let target = viewport.as_ref().unwrap_or(&parent);
                let transform = view
                    .transform_to_visual(target)
                    .ok_or(TextProviderError::GeometryUnavailable)?;
                let rect = rectangle.transform_to_aabb(transform);
                let delta = if align_to_top {
                    rect.y
                } else {
                    rect.bottom() - scroller.viewport().height
                };
                let offset = scroller.offset();
                scroller.set_offset(Vector::new(offset.x, offset.y + delta));
                break;
            }
            ancestor = parent.visual_parent();
        }
        view.bring_into_view_rect(rectangle);
        Ok(())
    }

    fn get_children(&self) -> Result<Vec<Ref<AutomationPeer>>, ProviderError> {
        let view = self.view()?;
        let selected = self.range.range();
        Ok(embedded_children(&view.document())
            .into_iter()
            .filter(|(range, _)| {
                !selected.is_empty() && range.start < selected.end && selected.start < range.end
            })
            .map(|(_, control)| ControlAutomationPeer::create_peer_for_element(&control))
            .collect())
    }
}

fn selection(view: &RichTextBlock) -> Range<usize> {
    view.selection_start().min(view.selection_end())
        ..view.selection_start().max(view.selection_end())
}

fn paragraph_units(text: &str) -> Vec<Range<usize>> {
    crate::rich_text::rich_text_unit_ranges(text, TextRangeUnit::Paragraph)
}

fn enclosing_unit_index(units: &[Range<usize>], position: usize) -> Option<usize> {
    units
        .iter()
        .position(|unit| unit.start <= position && (position < unit.end || unit.is_empty()))
        .or_else(|| units.len().checked_sub(1))
}

fn enclosing_unit(units: &[Range<usize>], position: usize) -> Option<&Range<usize>> {
    enclosing_unit_index(units, position).map(|index| &units[index])
}

fn move_position(units: &[Range<usize>], position: usize, count: i32) -> (usize, i32) {
    if count == 0 {
        return (position, 0);
    }
    let mut boundaries: Vec<_> = units
        .iter()
        .flat_map(|range| [range.start, range.end])
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();
    let remaining = count.unsigned_abs() as usize;
    if count > 0 {
        let first = boundaries.partition_point(|value| *value <= position);
        let moved = remaining.min(boundaries.len() - first);
        if moved == 0 {
            (position, 0)
        } else {
            (boundaries[first + moved - 1], moved as i32)
        }
    } else {
        let end = boundaries.partition_point(|value| *value < position);
        let moved = remaining.min(end);
        if moved == 0 {
            (position, 0)
        } else {
            (boundaries[end - moved], (-(moved as i64)) as i32)
        }
    }
}

struct ViewGeometry {
    transform: Matrix,
    clip: Rect,
}
impl ViewGeometry {
    fn new(view: &RichTextBlock) -> Option<Self> {
        if !view.is_effectively_visible() {
            return None;
        }
        let root = view.visual_root()?;
        let transform = view.transform_to_visual(&root)?;
        let mut clip = Rect::from_size(root.bounds().size());
        let mut current = Some(view.to_ref().upcast::<Visual>());
        while let Some(visual) = current {
            let to_root = visual.transform_to_visual(&root)?;
            if visual.clip_to_bounds() {
                clip = clip
                    .intersect(Rect::from_size(visual.bounds().size()).transform_to_aabb(to_root));
            }
            if let Some(geometry) = visual.clip() {
                clip = clip.intersect(geometry.bounds().transform_to_aabb(to_root));
            }
            current = visual.visual_parent();
        }
        Some(Self { transform, clip })
    }
    fn visible(&self, local: Rect) -> Rect {
        local.transform_to_aabb(self.transform).intersect(self.clip)
    }
}

fn embedded_children(document: &RichTextDocument) -> Vec<(Range<usize>, Ref<Control>)> {
    fn visit(
        inlines: &[InlineSnapshot],
        offset: &mut usize,
        output: &mut Vec<(Range<usize>, Ref<Control>)>,
    ) {
        for inline in inlines {
            if let InlineSnapshotKind::Embedded(Some(control)) = &inline.kind {
                output.push((*offset..*offset + 1, control.clone()));
            }
            if let Some(children) = inline.children() {
                visit(children, offset, output)
            } else {
                *offset += inline.text_length()
            }
        }
    }
    let mut output = Vec::new();
    let mut offset = 0;
    for (index, paragraph) in document.snapshot().paragraphs.iter().enumerate() {
        if index > 0 {
            offset += 1
        }
        visit(&paragraph.inlines, &mut offset, &mut output);
    }
    output
}

struct FormatSpan {
    range: Range<usize>,
    character: CharacterFormat,
    paragraph: ParagraphFormat,
}

fn format_spans(view: &RichTextBlock) -> Vec<FormatSpan> {
    fn visit(
        inlines: &[InlineSnapshot],
        offset: &mut usize,
        boundaries: &mut Vec<usize>,
        barriers: &mut HashSet<usize>,
    ) {
        for inline in inlines {
            boundaries.push(*offset);
            let barrier = matches!(
                inline.kind,
                InlineSnapshotKind::Embedded(_) | InlineSnapshotKind::Hyperlink { .. }
            );
            if barrier {
                barriers.insert(*offset);
            }
            if let Some(children) = inline.children() {
                visit(children, offset, boundaries, barriers)
            } else {
                *offset += inline.text_length()
            }
            if barrier {
                barriers.insert(*offset);
            }
            boundaries.push(*offset);
        }
    }
    let mut boundaries = vec![0];
    let mut barriers = HashSet::new();
    let mut offset = 0;
    let document = view.document();
    for (index, paragraph) in document.snapshot().paragraphs.iter().enumerate() {
        if index > 0 {
            boundaries.push(offset);
            offset += 1;
            boundaries.push(offset);
        }
        visit(
            &paragraph.inlines,
            &mut offset,
            &mut boundaries,
            &mut barriers,
        );
    }
    boundaries.push(document.text_length());
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut spans: Vec<FormatSpan> = Vec::new();
    for pair in boundaries.windows(2) {
        let character = effective_format(view, pair[0], false);
        let paragraph = effective_paragraph(view, pair[0]);
        if let Some(previous) = spans.last_mut().filter(|span| {
            span.range.end == pair[0]
                && !barriers.contains(&pair[0])
                && span.character == character
                && span.paragraph == paragraph
        }) {
            previous.range.end = pair[1];
        } else {
            spans.push(FormatSpan {
                range: pair[0]..pair[1],
                character,
                paragraph,
            });
        }
    }
    if spans.is_empty() {
        spans.push(FormatSpan {
            range: 0..0,
            character: effective_format(view, 0, true),
            paragraph: effective_paragraph(view, 0),
        });
    }
    spans
}

fn effective_format(view: &RichTextBlock, position: usize, caret: bool) -> CharacterFormat {
    view.document()
        .element_at(
            position,
            if caret {
                TextGravity::Backward
            } else {
                TextGravity::Forward
            },
        )
        .map(|element| CharacterFormat::effective(&element))
        .unwrap_or_else(|| CharacterFormat {
            font_family: Some(view.font_family()),
            font_size: Some(view.font_size()),
            font_style: Some(view.font_style()),
            font_weight: Some(view.font_weight()),
            foreground: Some(view.foreground()),
            background: Some(view.background()),
            ..Default::default()
        })
}

fn effective_paragraph(view: &RichTextBlock, position: usize) -> ParagraphFormat {
    let mut offset = 0;
    for block in view.document().blocks().snapshot().iter() {
        if let Some(paragraph) = block.cast::<Paragraph>() {
            let length = paragraph.text_length();
            if offset <= position && position <= offset + length {
                return ParagraphFormat {
                    text_alignment: Some(paragraph.text_alignment()),
                    text_indent: Some(
                        if paragraph.is_set(Paragraph::text_indent_property().as_property()) {
                            paragraph.text_indent()
                        } else {
                            view.text_indent()
                        },
                    ),
                    margin: Some(paragraph.margin()),
                    flow_direction: Some(paragraph.flow_direction()),
                    ..Default::default()
                };
            }
            offset += length + 1;
        }
    }
    ParagraphFormat {
        text_alignment: Some(view.text_alignment()),
        flow_direction: Some(view.flow_direction()),
        ..Default::default()
    }
}

fn attribute_value(
    view: &RichTextBlock,
    position: usize,
    caret: bool,
    attribute: TextAttribute,
) -> TextAttributeValue {
    use TextAttribute as A;
    use TextAttributeValue as V;
    let format = effective_format(view, position, caret);
    let paragraph = effective_paragraph(view, position);
    let decoration = |location| {
        format
            .text_decorations
            .as_ref()
            .and_then(Option::as_ref)
            .is_some_and(|collection| {
                collection
                    .list()
                    .snapshot()
                    .iter()
                    .any(|item| item.location() == location)
            })
    };
    match attribute {
        A::FontName => format
            .font_family
            .map_or(V::NotSupported, |font| V::String(font.name().to_owned())),
        A::FontSize => format
            .font_size
            .map_or(V::NotSupported, |size| V::Number(size * 72.0 / 96.0)),
        A::FontWeight => format
            .font_weight
            .map_or(V::NotSupported, |weight| V::Integer(weight.0)),
        A::IsItalic => V::Bool(
            format
                .font_style
                .is_some_and(|style| style != FontStyle::Normal),
        ),
        A::IsReadOnly => V::Bool(
            view.to_ref()
                .cast::<RichEditBox>()
                .is_none_or(|editor| editor.is_read_only()),
        ),
        A::IsHidden => V::Bool(!view.is_effectively_visible()),
        A::ForegroundColor => brush_color(format.foreground.as_ref().and_then(Option::as_ref)),
        A::BackgroundColor => brush_color(format.background.as_ref().and_then(Option::as_ref)),
        A::UnderlineStyle => V::Integer(i32::from(decoration(TextDecorationLocation::Underline))),
        A::StrikethroughStyle => {
            V::Integer(i32::from(decoration(TextDecorationLocation::Strikethrough)))
        }
        A::IsSubscript => V::Bool(format.baseline_alignment == Some(BaselineAlignment::Subscript)),
        A::IsSuperscript => {
            V::Bool(format.baseline_alignment == Some(BaselineAlignment::Superscript))
        }
        A::HorizontalTextAlignment => {
            let rtl = paragraph.flow_direction == Some(FlowDirection::RightToLeft);
            V::Integer(match paragraph.text_alignment.unwrap_or_default() {
                TextAlignment::Center => 1,
                TextAlignment::Right => 2,
                TextAlignment::Justify => 3,
                TextAlignment::Start if rtl => 2,
                TextAlignment::End if !rtl => 2,
                _ => 0,
            })
        }
        A::IndentationFirstLine => {
            V::Number(paragraph.text_indent.unwrap_or_default() * 72.0 / 96.0)
        }
        A::IndentationLeading | A::IndentationTrailing => {
            let margin = paragraph.margin.unwrap_or_default();
            let rtl = paragraph.flow_direction == Some(FlowDirection::RightToLeft);
            let left = (attribute == A::IndentationLeading) != rtl;
            V::Number(if left { margin.left } else { margin.right } * 72.0 / 96.0)
        }
        A::Unknown(_) => V::NotSupported,
    }
}

fn brush_color(brush: Option<&Rc<dyn IBrush>>) -> TextAttributeValue {
    brush.and_then(|brush| brush.as_solid_color_brush()).map_or(
        TextAttributeValue::NotSupported,
        |brush| {
            let color = brush.color();
            TextAttributeValue::Color(Color::from_rgb(color.r, color.g, color.b))
        },
    )
}
