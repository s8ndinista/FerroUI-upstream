//! Directed movement over the editor's model and real shaped display lines.
//!
//! Contracts: the pinned Microsoft.UI.Text.xml ITextSelection.MoveUp/MoveDown,
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-moveup
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-movedown
use super::*;
use ferroui_base::media::CharacterHit;
use units::{resolve_display_lines, DisplayLine};

pub(super) struct PublishedSelection {
    pub(super) accepted: EditorSelection,
    pub(super) uncontested: bool,
}

impl RichEditTextSelection {
    /// Moves upward by actual display lines or logical paragraphs. A negative
    /// count moves downward. Extension retains the directed anchor. Collapsing
    /// starts at the directional selection edge, then moves the requested units;
    /// a collapse with no further movement still reports one unit.
    ///
    /// Line movement requires a complete, untrimmed shaped view and retains the
    /// desired horizontal coordinate across short lines. Paragraph works without
    /// a view. Screen and Window units are not represented by TextRangeUnit.
    pub fn move_up(&self, unit: TextRangeUnit, count: i32, extend: bool) -> Result<i32, EditError> {
        self.move_vertically(unit, count, extend, false)
    }

    /// Moves downward by display lines or logical paragraphs. Negative counts
    /// reverse the direction. The return value counts accepted units relative
    /// to this command's direction, rather than UTF-16 positions.
    pub fn move_down(
        &self,
        unit: TextRangeUnit,
        count: i32,
        extend: bool,
    ) -> Result<i32, EditError> {
        self.move_vertically(unit, count, extend, true)
    }

    pub(super) fn move_logically(
        &self,
        unit: TextRangeUnit,
        count: i32,
        extend: bool,
    ) -> Result<i32, EditError> {
        if count == 0 {
            return Ok(0);
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        // Layout can change selection. Capture the directed endpoints only once
        // resolution has completed, then fence publication against model edits.
        let before = self.selection();
        let version = self.editor.document().mutation_version();
        let (position, moved) = if extend {
            units.move_position(before.active, count)
        } else {
            units.move_range(before.range(), count)
        };
        let requested =
            EditorSelection::new(if extend { before.anchor } else { position }, position);
        let published = self.publish_navigation(before, requested, version)?;
        unit_count(accepted_count(
            before,
            requested,
            published.accepted,
            !extend,
            count.signum(),
            i128::from(moved),
            |forward| {
                units
                    .move_range(before.range(), if forward { 1 } else { -1 })
                    .0
            },
            |from, to| units.crossed_boundaries(from, to),
        ))
    }

    fn move_vertically(
        &self,
        unit: TextRangeUnit,
        count: i32,
        extend: bool,
        down: bool,
    ) -> Result<i32, EditError> {
        if !matches!(unit, TextRangeUnit::Line | TextRangeUnit::Paragraph) {
            return Err(DocumentError::InvalidFormat(
                "Vertical selection navigation supports Line and Paragraph units.",
            )
            .into());
        }
        if count == 0 {
            return Ok(0);
        }
        // Widen before negation: MoveUp(i32::MIN) is a valid downward request.
        let logical_count = if down {
            i64::from(count)
        } else {
            -i64::from(count)
        };
        let direction = logical_count.signum() as i32;
        if unit == TextRangeUnit::Paragraph {
            let units = TextUnits::resolve(&self.editor, unit)?;
            let before = self.selection();
            let version = self.editor.document().mutation_version();
            let base = vertical_base(before, logical_count, extend);
            let (position, moved) = units.move_position_wide(base, logical_count);
            let requested =
                EditorSelection::new(if extend { before.anchor } else { position }, position);
            let published = self.publish_navigation(before, requested, version)?;
            let accepted = accepted_vertical_count(
                before,
                requested,
                published.accepted,
                !extend,
                direction,
                collapsed_vertical_count(before, extend, direction, i128::from(moved)),
                |from, to| units.crossed_boundaries(from, to),
            );
            return unit_count(if down { accepted } else { -accepted });
        }

        let lines = resolve_display_lines(&self.editor)?;
        let before = self.selection();
        let version = self.editor.document().mutation_version();
        let base = vertical_base(before, logical_count, extend);
        let first = line_index(&lines, base);
        let retained_x = if extend || before.is_empty() {
            self.editor.vertical_navigation_x()
        } else {
            None
        };
        let x = match retained_x {
            Some(x) => x,
            None => caret_x(&lines[first], base)?,
        };
        if !x.is_finite() {
            return Err(DocumentError::InvalidFormat(
                "The text view returned an invalid caret distance.",
            )
            .into());
        }
        let distance = usize::try_from(logical_count.unsigned_abs()).unwrap_or(usize::MAX);
        let last = if logical_count >= 0 {
            first.saturating_add(distance).min(lines.len() - 1)
        } else {
            first.saturating_sub(distance)
        };
        let mut position = if last == first {
            base
        } else {
            caret_at_x(&lines[last], x)?
        };
        let mut moved = last as i128 - first as i128;
        if extend && distance > last.abs_diff(first) {
            // Native extension at the first/last display line can reach the
            // story boundary in one more unit. The Rust story has no synthetic
            // terminal EOP: only an observable movement within model positions
            // is counted at its end.
            let boundary = if direction > 0 {
                lines.last().unwrap().range.end
            } else {
                0
            };
            if position != boundary {
                position = boundary;
                moved += i128::from(direction);
            }
        }
        // A handler may redirect an extension to a story edge on the same
        // display line. Resolve the projected edge carets before publication so
        // the accepted count uses the same extra boundary unit as direct moves.
        let edge_positions = if extend {
            Some((
                caret_at_x(&lines[0], x)?,
                caret_at_x(lines.last().unwrap(), x)?,
            ))
        } else {
            None
        };
        let requested =
            EditorSelection::new(if extend { before.anchor } else { position }, position);
        let published = self.publish_navigation(before, requested, version)?;
        let accepted = accepted_vertical_count(
            before,
            requested,
            published.accepted,
            !extend,
            direction,
            collapsed_vertical_count(before, extend, direction, moved),
            |from, to| {
                let mut units = line_index(&lines, to) as i128 - line_index(&lines, from) as i128;
                if let Some((first, last)) = edge_positions {
                    let end = lines.last().unwrap().range.end;
                    if to < from && to == 0 && first != 0 {
                        units -= 1;
                    } else if to > from && to == end && last != end {
                        units += 1;
                    }
                }
                units
            },
        );
        if published.uncontested {
            self.editor.remember_vertical_navigation_x(x);
        }
        unit_count(if down { accepted } else { -accepted })
    }

    /// Do not let a callback-owned document mutation be acknowledged as a stale
    /// movement, even when an outer application batch defers revision events.
    pub(super) fn publish_navigation(
        &self,
        before: EditorSelection,
        requested: EditorSelection,
        version: u64,
    ) -> Result<PublishedSelection, EditError> {
        let document = self.editor.document();
        if document.mutation_version() != version {
            self.editor.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.selection() != before {
            return Err(EditError::ReentrantEdit);
        }
        let checkpoint = self.editor.navigation_checkpoint();
        let generation = self.editor.navigation_generation();
        let result = self.editor.set_selection(requested);
        if document.mutation_version() != version {
            self.editor.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        result?;
        let accepted = self.selection();
        Ok(PublishedSelection {
            accepted,
            // A view replacement or nested command can choose different caret
            // affinity at the same offsets. Its independent generation prevents
            // this enclosing request from overwriting that accepted state.
            uncontested: accepted == requested
                && self.editor.navigation_generation() == generation
                && self.editor.navigation_checkpoint()
                    == checkpoint.wrapping_add(u64::from(requested != before)),
        })
    }
}

fn vertical_base(before: EditorSelection, count: i64, extend: bool) -> usize {
    if extend || before.is_empty() {
        before.active
    } else if count > 0 {
        before.range().end
    } else {
        before.range().start
    }
}

fn collapsed_vertical_count(
    before: EditorSelection,
    extend: bool,
    direction: i32,
    moved: i128,
) -> i128 {
    if moved == 0 && !extend && !before.is_empty() {
        i128::from(direction)
    } else {
        moved
    }
}

fn line_index(lines: &[DisplayLine], position: usize) -> usize {
    // Shared wrapped boundaries belong to the following line. Structural marks
    // have no glyph, but remain covered by the preceding source range.
    lines
        .partition_point(|line| line.range.start <= position)
        .saturating_sub(1)
}

fn caret_x(line: &DisplayLine, position: usize) -> Result<f64, EditError> {
    let local = position
        .min(line.caret_end)
        .saturating_sub(line.paragraph_offset);
    let local = i32::try_from(local).map_err(|_| {
        DocumentError::InvalidFormat(
            "The caret position exceeds the text formatter's source-index range.",
        )
    })?;
    Ok(line.origin.x
        + line
            .line
            .get_distance_from_character_hit(CharacterHit::new(local)))
}

fn caret_at_x(line: &DisplayLine, x: f64) -> Result<usize, EditError> {
    let hit = line.line.get_character_hit_from_distance(x - line.origin.x);
    let local = hit
        .first_character_index()
        .checked_add(hit.trailing_length())
        .and_then(|position| usize::try_from(position).ok())
        .ok_or(DocumentError::InvalidFormat(
            "The text view returned an invalid character hit.",
        ))?;
    let position = line
        .paragraph_offset
        .checked_add(local)
        .ok_or(DocumentError::InvalidFormat(
            "The text view character hit exceeds the story.",
        ))?;
    Ok(position.clamp(line.range.start, line.caret_end))
}

fn unit_count(value: i128) -> Result<i32, EditError> {
    i32::try_from(value).map_err(|_| {
        DocumentError::InvalidFormat("The accepted movement exceeds the signed unit-count range.")
            .into()
    })
}

/// Callback redirection is measured in the same units as the command. A
/// cancelled selection is zero. An accepted collapse costs one unit; further
/// movement counts only boundaries beyond that directional collapsed endpoint.
fn accepted_count(
    before: EditorSelection,
    requested: EditorSelection,
    accepted: EditorSelection,
    collapse: bool,
    direction: i32,
    requested_count: i128,
    collapse_position: impl Fn(bool) -> usize,
    distance: impl Fn(usize, usize) -> i128,
) -> i128 {
    if accepted == before {
        return 0;
    }
    if accepted == requested {
        return requested_count;
    }
    if !collapse || before.is_empty() || !accepted.is_empty() {
        return distance(before.active, accepted.active);
    }
    let range = before.range();
    let forward = if accepted.active < range.start {
        false
    } else if accepted.active > range.end {
        true
    } else {
        direction > 0
    };
    let additional = distance(collapse_position(forward), accepted.active);
    if forward {
        1 + additional.max(0)
    } else {
        -1 + additional.min(0)
    }
}

/// Vertical collapse does not consume the requested distance. If a handler
/// redirects it, report the accepted units from the directional collapse edge;
/// the accepted collapse itself is one unit when no further unit was crossed.
fn accepted_vertical_count(
    before: EditorSelection,
    requested: EditorSelection,
    accepted: EditorSelection,
    collapse: bool,
    direction: i32,
    requested_count: i128,
    distance: impl Fn(usize, usize) -> i128,
) -> i128 {
    if accepted == before {
        return 0;
    }
    if accepted == requested {
        return requested_count;
    }
    if !collapse || before.is_empty() || !accepted.is_empty() {
        return distance(before.active, accepted.active);
    }
    let range = before.range();
    let forward = if accepted.active < range.start {
        false
    } else if accepted.active > range.end {
        true
    } else {
        direction > 0
    };
    if forward {
        distance(range.end, accepted.active).max(1)
    } else {
        distance(range.start, accepted.active).min(-1)
    }
}

#[cfg(test)]
#[path = "rich_edit_text_selection_movement_tests.rs"]
mod tests;
