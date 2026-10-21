//! Editor-only frame grids through public FerroUI drawing/clip APIs.
//!
//! The retained candidate set consumes the editor change feed. Panning never
//! rescans ordinary layers, and scoped edits update membership only for changed
//! IDs. Line counts follow the visible viewport, with dense axes suppressed.
use super::*;
use ferrodesign_core::{EditorRenderChanges, EditorRenderToken, LayoutGrid};
use std::collections::BTreeSet;

const MIN_GRID_SPACING: f64 = 4.0;
const MAX_AXIS_LINES: usize = 4_096;
const MAX_FRAME_LINES: usize = 32_768;

#[derive(Default)]
pub(super) struct LayoutGridOverlay {
    grids: BTreeSet<NodeId>,
    token: Option<EditorRenderToken>,
    #[cfg(test)]
    examined: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct GridOverlayStats {
    pub layers: usize,
    pub lines: usize,
    pub suppressed_axes: usize,
    pub budget_exhausted: bool,
}

impl LayoutGridOverlay {
    fn refresh(&mut self, editor: &Editor) {
        let token = editor.render_change_token();
        if self.token.as_ref() == Some(&token) { return; }
        let changes = self.token.as_ref().map_or(EditorRenderChanges::Full, |previous| editor.render_changes_since(previous));
        match changes {
            EditorRenderChanges::Full => {
                self.grids.clear();
                for node in editor.document.nodes.values() {
                    #[cfg(test)] { self.examined += 1; }
                    if node.layout_grid.is_some() { self.grids.insert(node.id); }
                }
            }
            EditorRenderChanges::Nodes { nodes, ancestors } => {
                for id in nodes.into_iter().chain(ancestors) {
                    #[cfg(test)] { self.examined += 1; }
                    if editor.document.nodes.get(&id).is_some_and(|node| node.layout_grid.is_some()) {
                        self.grids.insert(id);
                    } else { self.grids.remove(&id); }
                }
            }
        }
        self.token = Some(token);
    }

    /// Called in screen coordinates, outside the scene painter. PNG/SVG/XAML
    /// exports and prototype rendering never invoke this guide-only pass.
    pub(super) fn render(&mut self, ctx: &mut DrawingContext, editor: &Editor, viewport: Viewport) -> GridOverlayStats {
        self.refresh(editor);
        let mut stats = GridOverlayStats::default();
        if !viewport.zoom.is_finite() || viewport.zoom <= 0.0 || viewport.width <= 0.0 || viewport.height <= 0.0 { return stats; }
        let visible = viewport.visible_rect();
        let canvas_clip = ctx.push_clip(UiRect::new(0.0, 0.0, viewport.width, viewport.height));
        let camera = ctx.push_transform(Matrix::create_scale(viewport.zoom, viewport.zoom)
            * Matrix::create_translation(viewport.pan.x, viewport.pan.y));
        let document = &editor.document;
        for &id in &self.grids {
            let Some(node) = document.nodes.get(&id) else { continue; };
            let Some(grid) = node.layout_grid.filter(|grid| grid.visible && grid.color.a > 0 && grid.validate().is_ok()) else { continue; };
            if !LayoutGrid::supports(node.kind) || document.page_of(id) != Some(editor.active_page) || !document.effective_visible(id) { continue; }
            let Some(world) = document.world_transform(id) else { continue; };
            if !world.transform_rect(node.local_bounds()).intersects(visible) { continue; }
            let Some(inverse) = world.inverse() else { continue; };
            let Some(local) = inverse.transform_rect(visible).intersection(node.local_bounds()) else { continue; };
            if !local.is_finite() || local.width <= 0.0 || local.height <= 0.0 { continue; }
            let det = (world.a * world.d - world.b * world.c).abs();
            let sx = det / world.c.hypot(world.d) * viewport.zoom;
            let sy = det / world.a.hypot(world.b) * viewport.zoom;
            let vertical = axis_lines(local.x, local.right(), grid.size, sx);
            let horizontal = axis_lines(local.y, local.bottom(), grid.size, sy);
            stats.suppressed_axes += usize::from(vertical.is_none()) + usize::from(horizontal.is_none());
            let lines = vertical.map_or(0, |(_, count)| count) + horizontal.map_or(0, |(_, count)| count);
            if lines == 0 { continue; }
            if stats.lines + lines > MAX_FRAME_LINES { stats.budget_exhausted = true; break; }
            // Walk the actual ancestry, applying all authored local transforms
            // and rounded ancestor clips. Guide pixels cannot escape a clipped
            // rotated/sheared frame, or an ancestor that clips its descendants.
            let mut chain = Vec::new();
            let mut current = Some(id);
            while let Some(current_id) = current {
                let Some(ancestor) = document.nodes.get(&current_id) else { break; };
                chain.push(ancestor);
                current = ancestor.parent;
                if chain.len() >= core::MAX_DOCUMENT_DEPTH { break; }
            }
            let mut states = Vec::with_capacity(chain.len() * 2);
            for ancestor in chain.into_iter().rev() {
                let t = ancestor.local_transform();
                states.push(ctx.push_transform(Matrix::new(t.a, t.b, t.c, t.d, t.e, t.f)));
                if ancestor.clip_content || ancestor.id == id {
                    states.push(ctx.push_rounded_clip(crate::renderer::rounded_rect(ancestor)));
                }
            }
            let color = ((grid.color.a as u32) << 24) | ((grid.color.r as u32) << 16)
                | ((grid.color.g as u32) << 8) | grid.color.b as u32;
            if let Some((first, count)) = vertical {
                let stroke = pen(color, 1.0 / sx);
                for offset in 0..count {
                    let x = (first + offset as f64) * grid.size;
                    ctx.draw_line(&stroke, UiPoint::new(x, local.y), UiPoint::new(x, local.bottom()));
                }
            }
            if let Some((first, count)) = horizontal {
                let stroke = pen(color, 1.0 / sy);
                for offset in 0..count {
                    let y = (first + offset as f64) * grid.size;
                    ctx.draw_line(&stroke, UiPoint::new(local.x, y), UiPoint::new(local.right(), y));
                }
            }
            for state in states.into_iter().rev() { ctx.pop(state); }
            stats.layers += 1;
            stats.lines += lines;
        }
        ctx.pop(camera);
        ctx.pop(canvas_clip);
        stats
    }
}

/// Projected perpendicular spacing is the density criterion, including shear.
/// Do not silently change authored spacing by drawing a coarser substitute.
fn axis_lines(start: f64, end: f64, size: f64, screen_scale: f64) -> Option<(f64, usize)> {
    if !screen_scale.is_finite() || screen_scale <= 0.0 || size * screen_scale < MIN_GRID_SPACING { return None; }
    let first = (start / size).ceil();
    let last = (end / size).floor();
    let count = last - first + 1.0;
    if !first.is_finite() || !count.is_finite() || count <= 0.0 || count > MAX_AXIS_LINES as f64
        || (first + 1.0) * size <= first * size { return None; }
    Some((first, count as usize))
}

#[cfg(test)]
#[path = "grids_tests.rs"]
mod tests;
