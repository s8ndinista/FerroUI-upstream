use crate::*;
use std::collections::BTreeSet;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutResult {
    pub changed: Vec<NodeId>,
    pub warnings: Vec<String>,
}
/// Layout reads only geometry and sizing rules. Retaining an entire `Node`
/// here would copy text, vector paths, images, and paint stacks on every pass.
#[derive(Clone, Copy)]
struct LayoutItem {
    id: NodeId,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    layout_child: LayoutChild,
}
fn clamp_size(v: f64, min: f64, max: Option<f64>) -> f64 {
    v.max(min.max(0.0))
        .min(max.unwrap_or(f64::MAX).max(min.max(0.0)))
}
fn flex_sizes(total: f64, items: &[(f64, f64, Option<f64>)]) -> Vec<f64> {
    let mut sizes = vec![0.0; items.len()];
    let mut active: BTreeSet<usize> = (0..items.len()).collect();
    let mut remaining = total;
    while !active.is_empty() {
        let weight: f64 = active.iter().map(|i| items[*i].0.max(0.001)).sum();
        let mut saturated = Vec::new();
        for i in &active {
            let (w, min, max) = items[*i];
            let share = remaining.max(0.0) * w.max(0.001) / weight;
            let clamped = clamp_size(share, min, max);
            if (share - clamped).abs() > 1e-7 {
                saturated.push((*i, clamped))
            }
        }
        if saturated.is_empty() {
            for i in active {
                sizes[i] = remaining.max(0.0) * items[i].0.max(0.001) / weight
            }
            break;
        }
        for (i, size) in saturated {
            sizes[i] = size;
            remaining -= size;
            active.remove(&i);
        }
    }
    sizes
}
fn alignment_offset(a: LayoutAlign, extra: f64) -> f64 {
    match a {
        LayoutAlign::Center => extra.max(0.0) / 2.0,
        LayoutAlign::End => extra.max(0.0),
        _ => 0.0,
    }
}
impl Document {
    pub fn apply_constraints(
        &mut self,
        parent: NodeId,
        old_width: f64,
        old_height: f64,
        new_width: f64,
        new_height: f64,
    ) {
        let Some(p) = self.nodes.get(&parent) else {
            return;
        };
        let has_layout = p.layout.is_some();
        let children = p.children.clone();
        fn axis(p: &mut f64, s: &mut f64, old: f64, new: f64, c: Constraint) {
            let delta = new - old;
            match c {
                Constraint::Start => {}
                Constraint::End => *p += delta,
                Constraint::Center => *p += delta / 2.0,
                Constraint::Stretch => *s = (*s + delta).max(0.0),
                Constraint::Scale => {
                    if old.abs() > GEOMETRY_EPSILON {
                        *p *= new / old;
                        *s = (*s * new / old).max(0.0)
                    }
                }
            }
        }
        for id in children {
            if let Some(c) = self.nodes.get_mut(&id) {
                if has_layout && !c.layout_child.absolute {
                    continue;
                }
                axis(
                    &mut c.x,
                    &mut c.width,
                    old_width,
                    new_width,
                    c.constraints.horizontal,
                );
                axis(
                    &mut c.y,
                    &mut c.height,
                    old_height,
                    new_height,
                    c.constraints.vertical,
                )
            }
        }
    }
    pub fn layout_node(&mut self, id: NodeId) -> LayoutResult {
        let mut result = LayoutResult::default();
        let Some(parent) = self.nodes.get(&id) else {
            return result;
        };
        let Some(layout) = parent.layout.clone() else {
            return result;
        };
        let mut children: Vec<LayoutItem> = parent
            .children
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .filter(|n| n.visible && !n.layout_child.absolute)
            .map(|node| LayoutItem {
                id: node.id,
                x: node.x,
                y: node.y,
                width: node.width,
                height: node.height,
                layout_child: node.layout_child,
            })
            .collect();
        if layout.reverse {
            children.reverse()
        }
        let p = layout.padding;
        let mut pw = parent.width;
        let mut ph = parent.height;
        let gap = layout.gap.max(0.0);
        let cross_gap = layout.cross_gap.max(0.0);
        if children.is_empty() {
            if layout.width == SizingMode::Hug {
                pw = p.left + p.right
            }
            if layout.height == SizingMode::Hug {
                ph = p.top + p.bottom
            }
        } else if layout.axis == LayoutAxis::Grid {
            let columns = layout.columns.max(1).min(4096).min(children.len());
            let rows = children.len().div_ceil(columns);
            if layout.width == SizingMode::Hug {
                pw = p.left
                    + p.right
                    + (0..columns)
                        .map(|col| {
                            children
                                .iter()
                                .skip(col)
                                .step_by(columns)
                                .map(|n| n.width)
                                .fold(0.0, f64::max)
                        })
                        .sum::<f64>()
                    + gap * (columns - 1) as f64
            }
            let cell_width =
                ((pw - p.left - p.right - gap * (columns - 1) as f64) / columns as f64).max(0.0);
            let mut heights = vec![0.0f64; rows];
            for (i, c) in children.iter_mut().enumerate() {
                if c.layout_child.width == SizingMode::Fill
                    || layout.cross_align == LayoutAlign::Stretch
                {
                    c.width = clamp_size(
                        cell_width,
                        c.layout_child.min_width,
                        c.layout_child.max_width,
                    )
                }
                heights[i / columns] = heights[i / columns].max(c.height)
            }
            if layout.height == SizingMode::Hug {
                ph = p.top + p.bottom + heights.iter().sum::<f64>() + cross_gap * (rows - 1) as f64
            } else if layout.cross_align == LayoutAlign::Stretch {
                heights.fill(
                    ((ph - p.top - p.bottom - cross_gap * (rows - 1) as f64) / rows as f64)
                        .max(0.0),
                )
            }
            let mut y = p.top;
            for row in 0..rows {
                for col in 0..columns {
                    let Some(c) = children.get_mut(row * columns + col) else {
                        continue;
                    };
                    if c.layout_child.height == SizingMode::Fill {
                        c.height = clamp_size(
                            heights[row],
                            c.layout_child.min_height,
                            c.layout_child.max_height,
                        )
                    }
                    c.x = p.left
                        + col as f64 * (cell_width + gap)
                        + alignment_offset(layout.main_align, cell_width - c.width);
                    c.y = y + alignment_offset(layout.cross_align, heights[row] - c.height)
                }
                y += heights[row] + cross_gap
            }
        } else {
            let row = layout.axis == LayoutAxis::Row;
            let (mb, ma, cb, ca) = if row {
                (p.left, p.right, p.top, p.bottom)
            } else {
                (p.top, p.bottom, p.left, p.right)
            };
            let main_hug = (if row { layout.width } else { layout.height }) == SizingMode::Hug;
            let cross_hug = (if row { layout.height } else { layout.width }) == SizingMode::Hug;
            let mut main = if row { pw } else { ph };
            let mut cross = if row { ph } else { pw };
            let cm = |n: &LayoutItem| if row { n.width } else { n.height };
            let cc = |n: &LayoutItem| if row { n.height } else { n.width };
            if main_hug {
                main = mb
                    + ma
                    + children.iter().map(cm).sum::<f64>()
                    + gap * children.len().saturating_sub(1) as f64;
                if children.iter().any(|n| {
                    (if row {
                        n.layout_child.width
                    } else {
                        n.layout_child.height
                    }) == SizingMode::Fill
                }) {
                    result.warnings.push(format!(
                        "Node {id}: fill children use intrinsic sizes on a hug main axis"
                    ))
                }
            }
            let available = (main - mb - ma).max(0.0);
            let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
            let mut used = 0.0;
            for (i, c) in children.iter().enumerate() {
                let needed = cm(c)
                    + if lines.last().unwrap().is_empty() {
                        0.0
                    } else {
                        gap
                    };
                if layout.wrap
                    && !main_hug
                    && !lines.last().unwrap().is_empty()
                    && used + needed > available + GEOMETRY_EPSILON
                {
                    lines.push(Vec::new());
                    used = 0.0
                }
                if !lines.last().unwrap().is_empty() {
                    used += gap
                }
                used += cm(c);
                lines.last_mut().unwrap().push(i)
            }
            let mut cross_sizes = Vec::new();
            for line in &lines {
                let flexible: Vec<usize> = line
                    .iter()
                    .copied()
                    .filter(|i| {
                        !main_hug
                            && (if row {
                                children[*i].layout_child.width
                            } else {
                                children[*i].layout_child.height
                            }) == SizingMode::Fill
                    })
                    .collect();
                if !flexible.is_empty() {
                    let fixed: f64 = line
                        .iter()
                        .filter(|i| {
                            (if row {
                                children[**i].layout_child.width
                            } else {
                                children[**i].layout_child.height
                            }) != SizingMode::Fill
                        })
                        .map(|i| cm(&children[*i]))
                        .sum();
                    let space =
                        (available - fixed - gap * line.len().saturating_sub(1) as f64).max(0.0);
                    let items: Vec<_> = flexible
                        .iter()
                        .map(|i| {
                            let c = children[*i].layout_child;
                            if row {
                                (c.grow, c.min_width, c.max_width)
                            } else {
                                (c.grow, c.min_height, c.max_height)
                            }
                        })
                        .collect();
                    for (i, size) in flexible.iter().zip(flex_sizes(space, &items)) {
                        if row {
                            children[*i].width = size
                        } else {
                            children[*i].height = size
                        }
                    }
                }
                cross_sizes.push(line.iter().map(|i| cc(&children[*i])).fold(0.0, f64::max));
            }
            if cross_hug {
                cross = cb
                    + ca
                    + cross_sizes.iter().sum::<f64>()
                    + cross_gap * lines.len().saturating_sub(1) as f64
            } else if lines.len() == 1 {
                cross_sizes[0] = (cross - cb - ca).max(0.0)
            }
            let mut cross_cursor = cb;
            for (line, line_cross) in lines.iter().zip(cross_sizes) {
                let content: f64 = line.iter().map(|i| cm(&children[*i])).sum();
                let remaining =
                    (available - content - gap * line.len().saturating_sub(1) as f64).max(0.0);
                let effective_gap =
                    if layout.main_align == LayoutAlign::SpaceBetween && line.len() > 1 {
                        gap + remaining / (line.len() - 1) as f64
                    } else {
                        gap
                    };
                let mut main_cursor = mb + alignment_offset(layout.main_align, remaining);
                for i in line {
                    let c = &mut children[*i];
                    let fill_cross = (if row {
                        c.layout_child.height
                    } else {
                        c.layout_child.width
                    }) == SizingMode::Fill;
                    if fill_cross || layout.cross_align == LayoutAlign::Stretch {
                        if row {
                            c.height = clamp_size(
                                line_cross,
                                c.layout_child.min_height,
                                c.layout_child.max_height,
                            )
                        } else {
                            c.width = clamp_size(
                                line_cross,
                                c.layout_child.min_width,
                                c.layout_child.max_width,
                            )
                        }
                    }
                    let offset = alignment_offset(layout.cross_align, line_cross - cc(c));
                    if row {
                        c.x = main_cursor;
                        c.y = cross_cursor + offset
                    } else {
                        c.y = main_cursor;
                        c.x = cross_cursor + offset
                    }
                    main_cursor += cm(c) + effective_gap
                }
                cross_cursor += line_cross + cross_gap;
            }
            if row {
                pw = main;
                ph = cross
            } else {
                ph = main;
                pw = cross
            }
        }
        for c in children {
            if let Some(n) = self.nodes.get_mut(&c.id) {
                if n.x != c.x || n.y != c.y || n.width != c.width || n.height != c.height {
                    n.x = c.x;
                    n.y = c.y;
                    n.width = c.width;
                    n.height = c.height;
                    result.changed.push(n.id)
                }
            }
        }
        if let Some(n) = self.nodes.get_mut(&id) {
            if n.width != pw || n.height != ph {
                n.width = pw.max(0.0);
                n.height = ph.max(0.0);
                result.changed.push(id)
            }
        }
        result
    }
    /// Bottom-up measure and top-down arrange, with bounded convergence for nested wrapping.
    pub fn layout_all(&mut self) -> LayoutResult {
        let order: Vec<_> = self
            .pages
            .iter()
            .flat_map(|p| self.draw_order(p.id))
            .collect();
        let mut changed = BTreeSet::new();
        let mut warnings = BTreeSet::new();
        for _ in 0..6 {
            let mut count = 0;
            for id in order.iter().rev().chain(order.iter()) {
                let r = self.layout_node(*id);
                count += r.changed.len();
                changed.extend(r.changed);
                warnings.extend(r.warnings)
            }
            if count == 0 {
                break;
            }
        }
        LayoutResult {
            changed: changed.into_iter().collect(),
            warnings: warnings.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn dense_mixed_fill_layout_preserves_payloads_and_excluded_children() {
        let mut document = Document::new("Dense layout");
        let page = document.pages[0].id;
        let mut row = Node::new(NodeKind::Frame, "Row", 20.0, 30.0, 10_000.0, 80.0);
        row.layout = Some(AutoLayout {
            gap: 1.0,
            padding: Padding::all(4.0),
            height: SizingMode::Fixed,
            ..AutoLayout::default()
        });
        let row = document.insert_node(page, row).unwrap();
        let mut originals = Vec::new();
        for index in 0..1024 {
            let mut node = Node::new(NodeKind::Text, format!("Text {index}"),
                7.0, 9.0, 2.0, 24.0)
                .with_text("Layout retains café Ω 🚀. ".repeat(32), 16.0, Color::BLACK);
            if index % 4 != 0 {
                node.layout_child.width = SizingMode::Fill;
            }
            let id = document.insert_node(row, node).unwrap();
            originals.push(document.nodes[&id].clone());
        }
        for absolute in [false, true] {
            let mut node = Node::new(NodeKind::Rectangle, "Excluded", 700.0, 800.0, 99.0, 100.0);
            node.visible = absolute;
            node.layout_child.absolute = absolute;
            let id = document.insert_node(row, node).unwrap();
            originals.push(document.nodes[&id].clone());
        }
        let result = document.layout_all();
        assert!(result.warnings.is_empty());
        let fill_width = (10_000.0 - 8.0 - 1023.0 - 256.0 * 2.0) / 768.0;
        let mut x = 4.0;
        for (index, original) in originals.iter().enumerate() {
            let actual = &document.nodes[&original.id];
            if index < 1024 {
                let width = if index % 4 == 0 { 2.0 } else { fill_width };
                assert!((actual.x - x).abs() < 1e-6);
                assert!((actual.width - width).abs() < 1e-6);
                assert_eq!(actual.y, 4.0);
                x += width + 1.0;
                let mut payload = actual.clone();
                payload.x = original.x;
                payload.y = original.y;
                payload.width = original.width;
                payload.height = original.height;
                assert_eq!(&payload, original);
            } else {
                assert_eq!(actual, original);
                assert!(!result.changed.contains(&original.id));
            }
        }
        assert!((x - 1.0 - 9996.0).abs() < 1e-6);
        assert!(document.layout_all().changed.is_empty());
        document.validate().unwrap();
    }

    #[test]
    fn no_layout_document_retains_all_content_and_geometry() {
        let mut document = Document::new("No layout");
        let page = document.pages[0].id;
        for index in 0..128 {
            document.insert_node(page,
                Node::new(NodeKind::Text, format!("Text {index}"),
                    index as f64 * 24.0, 90.0, 200.0, 40.0)
                    .with_text("Unchanged rich content Ω 🚀 ".repeat(64), 18.0, Color::BLACK))
                .unwrap();
        }
        let before = document.to_json().unwrap();
        assert_eq!(document.layout_all(), LayoutResult::default());
        assert_eq!(document.to_json().unwrap(), before);
    }
}
