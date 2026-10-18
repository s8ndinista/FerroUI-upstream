use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}
pub type Alignment = Align;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distribution {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Select,
    Frame,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Polygon,
    Star,
    Pen,
    Pencil,
    Text,
    Hand,
    Comment,
    Slice,
    Section,
}
#[derive(Clone, Debug, PartialEq)]
struct Metadata {
    name: String,
    pages: Vec<Page>,
    styles: BTreeMap<StyleId, Style>,
    variables: BTreeMap<VariableId, Variable>,
    variable_mode: String,
    comments: Vec<CommentThread>,
    guides: BTreeMap<NodeId, Vec<Guide>>,
    next_id: u64,
}
impl Metadata {
    fn capture(d: &Document) -> Self {
        Self {
            name: d.name.clone(),
            pages: d.pages.clone(),
            styles: d.styles.clone(),
            variables: d.variables.clone(),
            variable_mode: d.variable_mode.clone(),
            comments: d.comments.clone(),
            guides: d.guides.clone(),
            next_id: d.next_id,
        }
    }
    fn restore(&self, d: &mut Document) {
        d.name = self.name.clone();
        d.pages = self.pages.clone();
        d.styles = self.styles.clone();
        d.variables = self.variables.clone();
        d.variable_mode = self.variable_mode.clone();
        d.comments = self.comments.clone();
        d.guides = self.guides.clone();
        d.next_id = self.next_id
    }
}
#[derive(Clone, Debug)]
struct HistoryEntry {
    label: String,
    nodes: BTreeMap<NodeId, (Option<Node>, Option<Node>)>,
    before: Metadata,
    after: Metadata,
    selection_before: Vec<NodeId>,
    selection_after: Vec<NodeId>,
    page_before: NodeId,
    page_after: NodeId,
}
#[derive(Clone, Debug)]
struct PendingTransaction {
    label: String,
    before: Document,
    selection: Vec<NodeId>,
    page: NodeId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clipboard {
    pub roots: Vec<NodeId>,
    pub nodes: BTreeMap<NodeId, Node>,
    pub bounds: Option<Rect>,
}
/// Committed history is a node delta. A gesture captures once, updates at pointer
/// frequency, and commits a single undo step with stable selection identities.
pub struct Editor {
    pub document: Document,
    pub selection: Vec<NodeId>,
    pub active_page: NodeId,
    pub scene: SceneIndex,
    pub revision: u64,
    pub viewport: Viewport,
    pub tool: Tool,
    pub clipboard: Option<Clipboard>,
    pub grid_size: f64,
    pub snap_to_grid: bool,
    pub snap_to_objects: bool,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    pending: Option<PendingTransaction>,
    previous_nodes: BTreeMap<NodeId, Node>,
    previous_page: NodeId,
    previous_roots: Vec<NodeId>,
    history_limit: usize,
    paste_count: u32,
}
impl Default for Editor {
    fn default() -> Self {
        Self::new(Document::default())
    }
}
impl Editor {
    pub fn new(document: Document) -> Self {
        let active_page = document.pages.first().map(|p| p.id).unwrap_or(1);
        let mut e = Self {
            document,
            selection: Vec::new(),
            active_page,
            scene: SceneIndex::default(),
            revision: 0,
            viewport: Viewport::default(),
            tool: Tool::Select,
            clipboard: None,
            grid_size: 8.0,
            snap_to_grid: false,
            snap_to_objects: true,
            undo: Vec::new(),
            redo: Vec::new(),
            pending: None,
            previous_nodes: BTreeMap::new(),
            previous_page: 0,
            previous_roots: Vec::new(),
            history_limit: 100,
            paste_count: 0,
        };
        e.refresh();
        e
    }
    pub fn demo() -> Self {
        Self::new(seeded_document())
    }
    pub fn refresh(&mut self) {
        self.selection
            .retain(|id| self.document.nodes.contains_key(id));
        if self.document.page(self.active_page).is_none() {
            self.active_page = self.document.pages.first().map(|p| p.id).unwrap_or(1)
        }
        let roots = self
            .document
            .page(self.active_page)
            .map(|p| p.children.as_slice())
            .unwrap_or_default();
        let mut structural_change = self.previous_page != self.active_page
            || self.previous_nodes.len() != self.document.nodes.len()
            || self.previous_roots != roots;
        let mut geometry_changed = Vec::new();
        self.revision = self.revision.wrapping_add(1);
        self.document.revision = self.revision;
        for (id, n) in &mut self.document.nodes {
            let content_changed = self
                .previous_nodes
                .get(id)
                .map(|old| {
                    structural_change |= n.parent != old.parent
                        || n.children != old.children
                        || n.visible != old.visible
                        || n.kind != old.kind
                        || n.clip_content != old.clip_content;
                    if n.x != old.x
                        || n.y != old.y
                        || n.width != old.width
                        || n.height != old.height
                        || n.rotation != old.rotation
                        || n.flip_x != old.flip_x
                        || n.flip_y != old.flip_y
                        || n.affine_transform != old.affine_transform
                        || n.effects != old.effects
                        || n.strokes != old.strokes
                    {
                        geometry_changed.push(*id);
                    }
                    let revision = n.revision;
                    n.revision = old.revision;
                    let changed = *n != *old;
                    n.revision = revision;
                    changed
                })
                .unwrap_or(true);
            if content_changed {
                n.revision = self.revision;
                self.previous_nodes.insert(*id, n.clone());
            }
        }
        if self.previous_nodes.len() != self.document.nodes.len() {
            self.previous_nodes
                .retain(|id, _| self.document.nodes.contains_key(id));
        }
        if structural_change {
            self.scene.rebuild(&self.document, self.active_page);
            self.previous_page = self.active_page;
            self.previous_roots = self
                .document
                .page(self.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default();
        } else if !geometry_changed.is_empty() {
            self.scene
                .update_nodes(&self.document, self.active_page, &geometry_changed);
        }
    }
    pub fn begin_transaction(&mut self, label: impl Into<String>) {
        if self.pending.is_none() {
            self.pending = Some(PendingTransaction {
                label: label.into(),
                before: self.document.clone(),
                selection: self.selection.clone(),
                page: self.active_page,
            })
        }
    }
    pub fn commit_transaction(&mut self) -> bool {
        let Some(mut p) = self.pending.take() else {
            return false;
        };
        let mut nodes = BTreeMap::new();
        let keys: BTreeSet<_> = p
            .before
            .nodes
            .keys()
            .chain(self.document.nodes.keys())
            .copied()
            .collect();
        for id in keys {
            let mut before = p.before.nodes.get_mut(&id);
            let after = self.document.nodes.get(&id);
            if let (Some(a), Some(b)) = (before.as_deref_mut(), after) {
                a.revision = b.revision;
            }
            let equal = match (before.as_deref(), after) {
                (Some(a), Some(b)) => a == b,
                (None, None) => true,
                _ => false,
            };
            if !equal {
                nodes.insert(id, (before.map(|n| n.clone()), after.cloned()));
            }
        }
        let before = Metadata::capture(&p.before);
        let after = Metadata::capture(&self.document);
        if nodes.is_empty() && before == after {
            self.refresh();
            return false;
        }
        self.undo.push(HistoryEntry {
            label: p.label,
            nodes,
            before,
            after,
            selection_before: p.selection,
            selection_after: self.selection.clone(),
            page_before: p.page,
            page_after: self.active_page,
        });
        if self.undo.len() > self.history_limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.refresh();
        true
    }
    pub fn cancel_transaction(&mut self) {
        if let Some(p) = self.pending.take() {
            self.document = p.before;
            self.selection = p.selection;
            self.active_page = p.page;
            self.refresh()
        }
    }
    pub fn transact<R>(
        &mut self,
        label: impl Into<String>,
        operation: impl FnOnce(&mut Document) -> R,
    ) -> R {
        let own = self.pending.is_none();
        if own {
            self.begin_transaction(label)
        }
        let r = operation(&mut self.document);
        if own {
            self.commit_transaction();
        } else {
            self.refresh()
        }
        r
    }
    fn edit<R>(&mut self, label: &str, operation: impl FnOnce(&mut Self) -> R) -> R {
        let own = self.pending.is_none();
        if own {
            self.begin_transaction(label)
        }
        let r = operation(self);
        if own {
            self.commit_transaction();
        } else {
            self.refresh()
        }
        r
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }
    pub fn history_len(&self) -> usize {
        self.undo.len()
    }
    pub fn undo(&mut self) -> bool {
        if self.pending.is_some() {
            self.cancel_transaction();
            return true;
        }
        let Some(e) = self.undo.pop() else {
            return false;
        };
        for (id, (before, _)) in &e.nodes {
            if let Some(n) = before {
                self.document.nodes.insert(*id, n.clone());
            } else {
                self.document.nodes.remove(id);
            }
        }
        e.before.restore(&mut self.document);
        self.selection = e.selection_before.clone();
        self.active_page = e.page_before;
        self.redo.push(e);
        self.refresh();
        true
    }
    pub fn redo(&mut self) -> bool {
        if self.pending.is_some() {
            self.commit_transaction();
        }
        let Some(e) = self.redo.pop() else {
            return false;
        };
        for (id, (_, after)) in &e.nodes {
            if let Some(n) = after {
                self.document.nodes.insert(*id, n.clone());
            } else {
                self.document.nodes.remove(id);
            }
        }
        e.after.restore(&mut self.document);
        self.selection = e.selection_after.clone();
        self.active_page = e.page_after;
        self.undo.push(e);
        self.refresh();
        true
    }
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.pending = None
    }
    pub fn set_history_limit(&mut self, n: usize) {
        self.history_limit = n.max(1);
        let excess = self.undo.len().saturating_sub(self.history_limit);
        self.undo.drain(..excess);
    }
    pub fn select(&mut self, id: NodeId, additive: bool) {
        if !self.document.nodes.contains_key(&id) {
            return;
        }
        if !additive {
            self.selection.clear()
        }
        if additive && self.selection.contains(&id) {
            self.selection.retain(|n| *n != id)
        } else if !self.selection.contains(&id) {
            self.selection.push(id)
        }
    }
    pub fn clear_selection(&mut self) {
        self.selection.clear()
    }
    pub fn select_all(&mut self) {
        self.selection = self
            .document
            .page(self.active_page)
            .map(|p| {
                p.children
                    .iter()
                    .filter(|id| {
                        !self.document.effective_locked(**id)
                            && self.document.effective_visible(**id)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn select_rect(&mut self, r: Rect, additive: bool) {
        if !additive {
            self.selection.clear()
        }
        let mut ids: Vec<_> = self
            .scene
            .spatial
            .query(r)
            .into_iter()
            .filter(|id| {
                !self.document.effective_locked(*id)
                    && self
                        .document
                        .world_bounds(*id)
                        .is_some_and(|b| r.contains_rect(b))
            })
            .collect();
        let set: BTreeSet<_> = ids.iter().copied().collect();
        ids.retain(|id| {
            !set.iter()
                .any(|p| p != id && self.document.is_ancestor(*p, *id))
        });
        for id in ids {
            if !self.selection.contains(&id) {
                self.selection.push(id)
            }
        }
    }
    pub fn hit_test(&self, p: Point, tolerance: f64) -> Option<NodeId> {
        self.scene.hit_test(&self.document, p, tolerance, false)
    }
    pub fn visible_nodes(&self, r: Rect) -> Vec<NodeId> {
        self.scene.visible_nodes(r)
    }
    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selection
            .iter()
            .filter_map(|id| self.document.world_bounds(*id))
            .reduce(|a, b| a.union(b))
    }
    pub fn root_selection(&self) -> Vec<NodeId> {
        self.selection
            .iter()
            .filter(|id| {
                !self.document.effective_locked(**id)
                    && !self
                        .selection
                        .iter()
                        .any(|p| p != *id && self.document.is_ancestor(*p, **id))
            })
            .copied()
            .collect()
    }
    pub fn set_active_page(&mut self, page: NodeId) -> bool {
        if self.document.page(page).is_none() {
            return false;
        }
        self.active_page = page;
        self.selection.clear();
        self.refresh();
        true
    }
    pub fn add_page(&mut self, name: impl Into<String>) -> NodeId {
        let name = name.into();
        self.edit("Add page", |e| {
            let id = e.document.add_page(name);
            e.active_page = id;
            e.selection.clear();
            id
        })
    }
    pub fn add_node(&mut self, kind: NodeKind, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.edit("Create layer", |e| {
            let id = e
                .document
                .insert_node(e.active_page, Node::new(kind, kind.label(), x, y, w, h))
                .expect("active page exists");
            e.selection = vec![id];
            id
        })
    }
    pub fn insert_node(&mut self, parent: NodeId, node: Node) -> Result<NodeId, DocumentError> {
        self.edit("Create layer", |e| {
            let id = e.document.insert_node(parent, node)?;
            e.selection = vec![id];
            Ok(id)
        })
    }
    pub fn update_node<R>(
        &mut self,
        id: NodeId,
        label: &str,
        update: impl FnOnce(&mut Node) -> R,
    ) -> Option<R> {
        self.edit(label, |e| Some(update(e.document.nodes.get_mut(&id)?)))
    }
    pub fn move_selection(&mut self, dx: f64, dy: f64) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.edit("Move layers", |e| {
            for id in e.root_selection() {
                e.move_world(id, Point::new(dx, dy))
            }
        })
    }
    fn move_world(&mut self, id: NodeId, delta: Point) {
        let p = self.document.nodes.get(&id).and_then(|n| n.parent);
        let delta = p
            .and_then(|id| self.document.world_transform(id))
            .and_then(|t| t.inverse())
            .map(|t| t.apply_vector(delta))
            .unwrap_or(delta);
        if let Some(n) = self.document.nodes.get_mut(&id) {
            n.x += delta.x;
            n.y += delta.y
        }
    }
    pub fn resize_node(&mut self, id: NodeId, w: f64, h: f64) {
        if !w.is_finite() || !h.is_finite() {
            return;
        }
        self.edit("Resize layer", |e| {
            if let Some(n) = e.document.nodes.get(&id) {
                let (ow, oh) = (n.width, n.height);
                let original_transform = n.local_transform();
                e.document
                    .apply_constraints(id, ow, oh, w.max(0.0), h.max(0.0));
                if let Some(n) = e.document.nodes.get_mut(&id) {
                    if n.kind == NodeKind::Vector {
                        let sx = if ow > GEOMETRY_EPSILON {
                            w.max(0.0) / ow
                        } else {
                            1.0
                        };
                        let sy = if oh > GEOMETRY_EPSILON {
                            h.max(0.0) / oh
                        } else {
                            1.0
                        };
                        n.path = n.path.transformed(Transform::scale(sx, sy));
                    }
                    n.width = w.max(0.0);
                    n.height = h.max(0.0);
                    // Resizing preserves the transformed local origin, including
                    // imported affine layers; dimensions change in local space.
                    let resized_transform = n.local_transform();
                    n.x += original_transform.e - resized_transform.e;
                    n.y += original_transform.f - resized_transform.f;
                }
                e.document.layout_node(id);
            }
        })
    }
    pub fn rotate_selection(&mut self, degrees: f64) {
        if !degrees.is_finite() {
            return;
        }
        self.edit("Rotate layers", |e| {
            for id in e.root_selection() {
                if let Some(n) = e.document.nodes.get_mut(&id) {
                    n.rotation = (n.rotation + degrees).rem_euclid(360.0)
                }
            }
        })
    }
    pub fn set_fill(&mut self, id: NodeId, c: Color) {
        self.update_node(id, "Change fill", |n| {
            n.fills = vec![Fill::solid(c)];
            n.overridden_properties.insert("fills".into());
        });
    }
    pub fn set_selection_fill(&mut self, c: Color) {
        self.edit("Change fill", |e| {
            for id in e.selection.clone() {
                if let Some(n) = e.document.nodes.get_mut(&id) {
                    n.fills = vec![Fill::solid(c)];
                    n.overridden_properties.insert("fills".into());
                }
            }
        })
    }
    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) {
        let text = text.into();
        self.update_node(id, "Edit text", |n| {
            n.text = text;
            n.text_runs.clear();
            n.overridden_properties.insert("text".into());
        });
    }
    pub fn toggle_visibility(&mut self, id: NodeId) {
        self.update_node(id, "Toggle visibility", |n| n.visible = !n.visible);
    }
    pub fn toggle_locked(&mut self, id: NodeId) {
        self.update_node(id, "Toggle lock", |n| n.locked = !n.locked);
    }
    pub fn delete_selection(&mut self) {
        self.edit("Delete layers", |e| {
            for id in e.root_selection() {
                e.document.remove_subtree(id);
            }
            e.selection.clear()
        })
    }
    pub fn copy_selection(&mut self) -> bool {
        let roots = self.root_selection();
        if roots.is_empty() {
            return false;
        }
        let mut nodes = BTreeMap::new();
        for id in &roots {
            for child in self.document.descendants(*id) {
                if let Some(n) = self.document.nodes.get(&child) {
                    nodes.insert(child, n.clone());
                }
            }
        }
        for id in &roots {
            if let Some(t) = self.document.world_transform(*id) {
                if let Some(n) = nodes.get_mut(id) {
                    assign_transform(n, t);
                    n.parent = None
                }
            }
        }
        self.clipboard = Some(Clipboard {
            roots,
            nodes,
            bounds: self.selection_bounds(),
        });
        self.paste_count = 0;
        true
    }
    pub fn cut_selection(&mut self) -> bool {
        if !self.copy_selection() {
            return false;
        }
        self.delete_selection();
        true
    }
    pub fn clipboard_json(&self) -> Option<String> {
        serde_json::to_string(self.clipboard.as_ref()?).ok()
    }
    pub fn paste(&mut self) -> Vec<NodeId> {
        let Some(c) = self.clipboard.clone() else {
            return Vec::new();
        };
        self.paste_count += 1;
        let d = 24.0 * self.paste_count as f64;
        self.paste_clipboard(&c, self.active_page, Point::new(d, d))
    }
    pub fn paste_at(&mut self, p: Point) -> Vec<NodeId> {
        let Some(c) = self.clipboard.clone() else {
            return Vec::new();
        };
        let origin = c.bounds.map(|b| Point::new(b.x, b.y)).unwrap_or_default();
        self.paste_clipboard(&c, self.active_page, p - origin)
    }
    pub fn paste_clipboard(&mut self, c: &Clipboard, parent: NodeId, offset: Point) -> Vec<NodeId> {
        self.edit("Paste layers", |e| {
            let roots = clone_tree_into(&mut e.document, &c.nodes, &c.roots, parent, offset, false);
            e.selection = roots.clone();
            roots
        })
    }
    /// Import editable SVG layers beside the existing page contents. Parsing and
    /// validation happen before the transaction; errors leave history, selection,
    /// and the document unchanged. The returned imported frame is selected.
    pub fn import_svg(&mut self, source: &str, name: &str) -> Result<Vec<NodeId>, DocumentError> {
        let imported = parse_svg(source, name)?;
        if self.document.nodes.len() + imported.document.nodes.len() > MAX_DOCUMENT_NODES {
            return Err(DocumentError::Invalid(
                "SVG import exceeds the document node limit".into(),
            ));
        }
        if self.document.page(self.active_page).is_none() {
            return Err(DocumentError::InvalidParent(self.active_page));
        }
        let right = self
            .document
            .page(self.active_page)
            .unwrap()
            .children
            .iter()
            .filter_map(|id| self.document.world_bounds(*id))
            .map(|b| b.right())
            .reduce(f64::max);
        let original = imported
            .document
            .world_bounds(imported.root)
            .unwrap_or_default();
        let offset = Point::new(
            right.map(|r| r + 64.0).unwrap_or(0.0) - original.x,
            -original.y,
        );
        Ok(self.edit("Import SVG", |editor| {
            let roots = clone_tree_into(
                &mut editor.document,
                &imported.document.nodes,
                &[imported.root],
                editor.active_page,
                offset,
                false,
            );
            editor.selection = roots.clone();
            roots
        }))
    }
    pub fn duplicate_selection(&mut self) -> Vec<NodeId> {
        self.edit("Duplicate layers", |e| {
            let roots = e.root_selection();
            let source = e.document.nodes.clone();
            let mut out = Vec::new();
            for id in roots {
                let Some(n) = source.get(&id) else { continue };
                let parent = n.parent.unwrap_or(e.active_page);
                let ids = clone_tree_into(
                    &mut e.document,
                    &source,
                    &[id],
                    parent,
                    Point::new(24.0, 24.0),
                    false,
                );
                if let Some(new) = ids.first() {
                    let siblings = e.document.children_mut(parent).unwrap();
                    siblings.retain(|i| i != new);
                    let index = siblings
                        .iter()
                        .position(|i| *i == id)
                        .map(|i| i + 1)
                        .unwrap_or(siblings.len());
                    siblings.insert(index, *new);
                }
                out.extend(ids)
            }
            e.selection = out.clone();
            out
        })
    }
    pub fn group_selection(&mut self) -> Option<NodeId> {
        self.edit("Group selection", |e| e.group_internal(NodeKind::Group))
    }
    pub fn frame_selection(&mut self) -> Option<NodeId> {
        self.edit("Frame selection", |e| e.group_internal(NodeKind::Frame))
    }
    fn group_internal(&mut self, kind: NodeKind) -> Option<NodeId> {
        let roots = self.root_selection();
        if roots.is_empty() {
            return None;
        }
        let first = self
            .document
            .nodes
            .get(&roots[0])?
            .parent
            .unwrap_or(self.active_page);
        let parent = if roots
            .iter()
            .all(|id| self.document.nodes.get(id).and_then(|n| n.parent) == Some(first))
        {
            first
        } else {
            self.active_page
        };
        let inv = self
            .document
            .world_transform(parent)
            .unwrap_or_default()
            .inverse()?;
        let bounds = roots
            .iter()
            .filter_map(|id| {
                let n = self.document.nodes.get(id)?;
                Some((inv * self.document.world_transform(*id)?).transform_rect(n.local_bounds()))
            })
            .reduce(|a, b| a.union(b))?;
        let transforms: Vec<_> = roots
            .iter()
            .filter_map(|id| Some((*id, self.document.world_transform(*id)?)))
            .collect();
        let index = self
            .document
            .children(parent)?
            .iter()
            .position(|id| roots.contains(id))
            .unwrap_or(self.document.children(parent)?.len());
        let mut n = Node::new(
            kind,
            kind.label(),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        );
        n.fills.clear();
        n.clip_content = false;
        let group = self.document.insert_node(parent, n).ok()?;
        {
            let siblings = self.document.children_mut(parent)?;
            siblings.retain(|id| *id != group);
            siblings.insert(index.min(siblings.len()), group)
        }
        let inverse = self.document.world_transform(group)?.inverse()?;
        for (id, world) in transforms {
            let _ = self.document.reparent(id, group, usize::MAX);
            if let Some(n) = self.document.nodes.get_mut(&id) {
                assign_transform(n, inverse * world)
            }
        }
        self.selection = vec![group];
        Some(group)
    }
    pub fn ungroup_selection(&mut self) -> Vec<NodeId> {
        self.edit("Ungroup selection", |e| {
            let mut out = Vec::new();
            for id in e.root_selection() {
                let Some(group) = e.document.nodes.get(&id).cloned() else {
                    continue;
                };
                if !matches!(
                    group.kind,
                    NodeKind::Group | NodeKind::Frame | NodeKind::Section
                ) {
                    out.push(id);
                    continue;
                }
                let parent = group.parent.unwrap_or(e.active_page);
                let Some(inv) = e
                    .document
                    .world_transform(parent)
                    .unwrap_or_default()
                    .inverse()
                else {
                    continue;
                };
                let index = e
                    .document
                    .children(parent)
                    .and_then(|c| c.iter().position(|n| *n == id))
                    .unwrap_or(0);
                for (i, child) in group.children.iter().enumerate() {
                    let Some(world) = e.document.world_transform(*child) else {
                        continue;
                    };
                    let _ = e.document.reparent(*child, parent, index + i);
                    if let Some(n) = e.document.nodes.get_mut(child) {
                        assign_transform(n, inv * world)
                    }
                    out.push(*child)
                }
                e.document.remove_subtree(id);
            }
            e.selection = out.clone();
            out
        })
    }
    pub fn align_selection(&mut self, a: Align) {
        self.edit("Align layers", |e| {
            let ids = e.root_selection();
            let Some(bounds) = ids
                .iter()
                .filter_map(|id| e.document.world_bounds(*id))
                .reduce(|a, b| a.union(b))
            else {
                return;
            };
            for id in ids {
                let Some(b) = e.document.world_bounds(id) else {
                    continue;
                };
                let delta = match a {
                    Align::Left => Point::new(bounds.x - b.x, 0.0),
                    Align::HorizontalCenter => Point::new(bounds.center().x - b.center().x, 0.0),
                    Align::Right => Point::new(bounds.right() - b.right(), 0.0),
                    Align::Top => Point::new(0.0, bounds.y - b.y),
                    Align::VerticalCenter => Point::new(0.0, bounds.center().y - b.center().y),
                    Align::Bottom => Point::new(0.0, bounds.bottom() - b.bottom()),
                };
                e.move_world(id, delta)
            }
        })
    }
    pub fn distribute_selection(&mut self, d: Distribution) {
        self.edit("Distribute layers", |e| {
            let mut items: Vec<_> = e
                .root_selection()
                .into_iter()
                .filter_map(|id| Some((id, e.document.world_bounds(id)?)))
                .collect();
            if items.len() < 3 {
                return;
            }
            let horizontal = d == Distribution::Horizontal;
            items.sort_by(|a, b| {
                (if horizontal { a.1.x } else { a.1.y })
                    .total_cmp(&(if horizontal { b.1.x } else { b.1.y }))
            });
            let first = items.first().unwrap().1;
            let last = items.last().unwrap().1;
            let start = if horizontal { first.x } else { first.y };
            let end = if horizontal {
                last.right()
            } else {
                last.bottom()
            };
            let sum: f64 = items
                .iter()
                .map(|(_, b)| if horizontal { b.width } else { b.height })
                .sum();
            let gap = (end - start - sum) / (items.len() - 1) as f64;
            let mut cursor = start;
            for (id, b) in items {
                let delta = cursor - if horizontal { b.x } else { b.y };
                e.move_world(
                    id,
                    if horizontal {
                        Point::new(delta, 0.0)
                    } else {
                        Point::new(0.0, delta)
                    },
                );
                cursor += if horizontal {
                    b.width + gap
                } else {
                    b.height + gap
                }
            }
        })
    }
    pub fn arrange_selection(&mut self, a: Arrange) {
        self.edit("Arrange layers", |e| {
            let selection: BTreeSet<_> = e.root_selection().into_iter().collect();
            let parents: BTreeSet<_> = selection
                .iter()
                .filter_map(|id| e.document.nodes.get(id).and_then(|n| n.parent))
                .collect();
            for parent in parents {
                let Some(c) = e.document.children_mut(parent) else {
                    continue;
                };
                match a {
                    Arrange::BringToFront => {
                        let moved: Vec<_> = c
                            .iter()
                            .filter(|id| selection.contains(id))
                            .copied()
                            .collect();
                        c.retain(|id| !selection.contains(id));
                        c.extend(moved)
                    }
                    Arrange::SendToBack => {
                        let mut moved: Vec<_> = c
                            .iter()
                            .filter(|id| selection.contains(id))
                            .copied()
                            .collect();
                        c.retain(|id| !selection.contains(id));
                        moved.append(c);
                        *c = moved
                    }
                    Arrange::BringForward => {
                        for i in (0..c.len().saturating_sub(1)).rev() {
                            if selection.contains(&c[i]) && !selection.contains(&c[i + 1]) {
                                c.swap(i, i + 1)
                            }
                        }
                    }
                    Arrange::SendBackward => {
                        for i in 1..c.len() {
                            if selection.contains(&c[i]) && !selection.contains(&c[i - 1]) {
                                c.swap(i, i - 1)
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn set_auto_layout(&mut self, layout: AutoLayout) -> Option<NodeId> {
        self.edit("Add auto layout", |e| {
            let id = if e.selection.len() == 1
                && e.document
                    .nodes
                    .get(&e.selection[0])
                    .is_some_and(|n| n.kind.is_container())
            {
                e.selection[0]
            } else {
                e.group_internal(NodeKind::Frame)?
            };
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.layout = Some(layout);
                if n.kind == NodeKind::Group {
                    n.kind = NodeKind::Frame
                }
            }
            e.document.layout_all();
            Some(id)
        })
    }
    pub fn remove_auto_layout(&mut self, id: NodeId) {
        self.update_node(id, "Remove auto layout", |n| n.layout = None);
    }
    pub fn create_component(&mut self) -> Option<NodeId> {
        self.edit("Create component", |e| {
            let id = if e.selection.len() == 1
                && e.document
                    .nodes
                    .get(&e.selection[0])
                    .is_some_and(|n| n.kind.is_container())
            {
                e.selection[0]
            } else {
                e.group_internal(NodeKind::Component)?
            };
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.kind = NodeKind::Component;
                n.component_id = None;
                n.component_source = None
            }
            Some(id)
        })
    }
    pub fn create_instance(&mut self, component: NodeId, x: f64, y: f64) -> Option<NodeId> {
        self.edit("Create instance", |e| {
            let source = e.document.nodes.clone();
            let n = source.get(&component)?;
            if n.kind != NodeKind::Component {
                return None;
            }
            let offset = Point::new(x - n.x, y - n.y);
            let roots = clone_tree_into(
                &mut e.document,
                &source,
                &[component],
                e.active_page,
                offset,
                true,
            );
            let id = *roots.first()?;
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.kind = NodeKind::Instance;
                n.component_id = Some(component);
                n.component_source = Some(component)
            }
            e.selection = vec![id];
            Some(id)
        })
    }
    pub fn detach_instance(&mut self, id: NodeId) -> bool {
        self.edit("Detach instance", |e| {
            if !e
                .document
                .nodes
                .get(&id)
                .is_some_and(|n| n.kind == NodeKind::Instance)
            {
                return false;
            }
            for child in e.document.descendants(id) {
                if let Some(n) = e.document.nodes.get_mut(&child) {
                    n.component_id = None;
                    n.component_source = None;
                    n.overridden_properties.clear();
                    if n.kind == NodeKind::Instance {
                        n.kind = NodeKind::Frame
                    }
                }
            }
            true
        })
    }
    pub fn sync_instances(&mut self, component: NodeId) -> usize {
        self.edit("Update component instances", |e| {
            sync_component_instances(&mut e.document, component)
        })
    }
    pub fn add_interaction(
        &mut self,
        source: NodeId,
        target: NodeId,
        transition: Transition,
    ) -> bool {
        self.edit("Add prototype interaction", |e| {
            if !e.document.nodes.contains_key(&target) {
                return false;
            }
            let Some(n) = e.document.nodes.get_mut(&source) else {
                return false;
            };
            n.interactions.push(Interaction {
                trigger: Trigger::Click,
                action: PrototypeAction::NavigateTo { target },
                transition,
                delay_ms: 0,
                key: None,
            });
            true
        })
    }
    pub fn add_comment(
        &mut self,
        node: Option<NodeId>,
        position: Point,
        author: impl Into<String>,
        body: impl Into<String>,
        created_at: impl Into<String>,
    ) -> u64 {
        let author = author.into();
        let body = body.into();
        let created_at = created_at.into();
        self.edit("Add comment", |e| {
            let id = e.document.allocate_id();
            e.document.comments.push(CommentThread {
                id,
                node,
                position,
                resolved: false,
                messages: vec![CommentMessage {
                    author,
                    body,
                    created_at,
                }],
            });
            id
        })
    }
    pub fn resolve_comment(&mut self, id: u64, resolved: bool) {
        self.transact("Resolve comment", |d| {
            if let Some(c) = d.comments.iter_mut().find(|c| c.id == id) {
                c.resolved = resolved
            }
        })
    }
    pub fn add_fill_style(&mut self, name: impl Into<String>, color: Color) -> StyleId {
        let name = name.into();
        self.transact("Create color style", |d| {
            let id = d.allocate_id();
            d.styles.insert(
                id,
                Style {
                    id,
                    name,
                    description: String::new(),
                    value: StyleValue::Fill {
                        fills: vec![Fill::solid(color)],
                    },
                },
            );
            id
        })
    }
    pub fn apply_style(&mut self, id: NodeId, style: StyleId) -> bool {
        self.transact("Apply style", |d| {
            let Some(s) = d.styles.get(&style).cloned() else {
                return false;
            };
            let Some(n) = d.nodes.get_mut(&id) else {
                return false;
            };
            match s.value {
                StyleValue::Fill { fills } => {
                    n.fills = fills;
                    n.fill_style = Some(style)
                }
                StyleValue::Text {
                    family,
                    size,
                    weight,
                    line_height,
                    letter_spacing,
                } => {
                    n.font_family = family;
                    n.font_size = size;
                    n.font_weight = weight;
                    n.line_height = line_height;
                    n.letter_spacing = letter_spacing;
                    n.text_style = Some(style)
                }
                StyleValue::Effect { effects } => {
                    n.effects = effects;
                    n.effect_style = Some(style)
                }
                StyleValue::Grid { .. } => return false,
            }
            true
        })
    }
    pub fn set_variable_mode(&mut self, mode: impl Into<String>) {
        let mode = mode.into();
        self.transact("Change variable mode", |d| {
            d.variable_mode = mode;
            d.apply_variables();
            d.layout_all();
        });
    }
    pub fn to_json(&self) -> Result<String, DocumentError> {
        self.document.to_json()
    }
    pub fn load_json(&mut self, json: &str) -> Result<(), DocumentError> {
        let d = Document::from_json(json)?;
        self.edit("Open document", |e| {
            e.active_page = d.pages[0].id;
            e.document = d;
            e.selection.clear()
        });
        Ok(())
    }
    pub fn export_svg(&self) -> String {
        self.document.export_svg(self.active_page, &[])
    }
    pub fn export_selection_svg(&self) -> String {
        self.document.export_svg(self.active_page, &self.selection)
    }
    pub fn snap_point(&self, p: Point) -> Point {
        if self.snap_to_grid {
            let s = self.grid_size.max(1.0);
            Point::new((p.x / s).round() * s, (p.y / s).round() * s)
        } else {
            p
        }
    }
    pub fn smart_snap(&self, bounds: Rect, threshold: f64, exclude: &[NodeId]) -> SnapResult {
        let mut result = SnapResult::default();
        let mut bx = threshold;
        let mut by = threshold;
        let sx = [bounds.x, bounds.center().x, bounds.right()];
        let sy = [bounds.y, bounds.center().y, bounds.bottom()];
        for id in self.scene.spatial.query(bounds.inflate(threshold * 8.0)) {
            if exclude.contains(&id) || exclude.iter().any(|p| self.document.is_ancestor(*p, id)) {
                continue;
            }
            let Some(t) = self.document.world_bounds(id) else {
                continue;
            };
            for x in [t.x, t.center().x, t.right()] {
                for source in sx {
                    let d = x - source;
                    if d.abs() < bx {
                        bx = d.abs();
                        result.delta.x = d;
                        result.vertical_guide = Some(x)
                    }
                }
            }
            for y in [t.y, t.center().y, t.bottom()] {
                for source in sy {
                    let d = y - source;
                    if d.abs() < by {
                        by = d.abs();
                        result.delta.y = d;
                        result.horizontal_guide = Some(y)
                    }
                }
            }
        }
        result
    }
    pub fn boolean_selection(&mut self, op: BooleanOperation) -> Option<NodeId> {
        self.edit("Boolean selection", |e| {
            let roots = e.root_selection();
            if roots.len() < 2 {
                return None;
            }
            let fills = e
                .document
                .nodes
                .get(&roots[0])
                .map(|n| n.fills.clone())
                .unwrap_or_else(|| vec![Fill::solid(Color::rgb(168, 132, 255))]);
            let id = e.group_internal(NodeKind::Boolean)?;
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.boolean_operation = op;
                n.fills = fills;
                n.name = match op {
                    BooleanOperation::Union => "Union",
                    BooleanOperation::Subtract => "Subtract",
                    BooleanOperation::Intersect => "Intersect",
                    BooleanOperation::Exclude => "Exclude",
                }
                .into()
            }
            Some(id)
        })
    }
    pub fn flatten_selection(&mut self) -> Option<NodeId> {
        self.edit("Flatten selection", |e| {
            let roots = e.root_selection();
            if roots.is_empty() {
                return None;
            }
            let all: Vec<_> = roots
                .iter()
                .flat_map(|id| e.document.descendants(*id))
                .collect();
            if all.iter().any(|id| {
                e.document
                    .nodes
                    .get(id)
                    .is_some_and(|n| matches!(n.kind, NodeKind::Text | NodeKind::Boolean))
            }) {
                return None;
            }
            let b = roots
                .iter()
                .filter_map(|id| e.document.world_bounds(*id))
                .reduce(|a, b| a.union(b))?;
            let mut segments = Vec::new();
            let origin = Transform::translation(-b.x, -b.y);
            for id in all {
                let n = e.document.nodes.get(&id)?;
                if n.kind.is_container() && n.fills.is_empty() {
                    continue;
                }
                let t = origin * e.document.world_transform(id)?;
                for s in outline_path(n).segments {
                    segments.push(match s {
                        PathSegment::MoveTo { point } => PathSegment::MoveTo {
                            point: t.apply(point),
                        },
                        PathSegment::LineTo { point } => PathSegment::LineTo {
                            point: t.apply(point),
                        },
                        PathSegment::QuadraticTo { control, point } => PathSegment::QuadraticTo {
                            control: t.apply(control),
                            point: t.apply(point),
                        },
                        PathSegment::CubicTo {
                            control1,
                            control2,
                            point,
                        } => PathSegment::CubicTo {
                            control1: t.apply(control1),
                            control2: t.apply(control2),
                            point: t.apply(point),
                        },
                        PathSegment::Close => PathSegment::Close,
                    })
                }
            }
            let fills = e.document.nodes.get(&roots[0])?.fills.clone();
            let mut n = Node::new(NodeKind::Vector, "Vector", b.x, b.y, b.width, b.height);
            n.path = VectorPath {
                segments,
                fill_rule: FillRule::NonZero,
            };
            n.fills = fills;
            let id = e.document.insert_node(e.active_page, n).ok()?;
            for root in roots {
                e.document.remove_subtree(root);
            }
            e.selection = vec![id];
            Some(id)
        })
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SnapResult {
    pub delta: Point,
    pub vertical_guide: Option<f64>,
    pub horizontal_guide: Option<f64>,
}
fn assign_transform(n: &mut Node, t: Transform) {
    n.set_local_transform(t)
}
fn clone_tree_into(
    d: &mut Document,
    source: &BTreeMap<NodeId, Node>,
    roots: &[NodeId],
    parent: NodeId,
    offset: Point,
    instance: bool,
) -> Vec<NodeId> {
    if d.children(parent).is_none() {
        return Vec::new();
    }
    let mut ordered = Vec::new();
    let mut stack: Vec<_> = roots.iter().rev().copied().collect();
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(n) = source.get(&id) {
            ordered.push(id);
            stack.extend(n.children.iter().rev().copied())
        }
    }
    if d.nodes.len() + ordered.len() > MAX_DOCUMENT_NODES {
        return Vec::new();
    }
    let remap: BTreeMap<_, _> = ordered.iter().map(|id| (*id, d.allocate_id())).collect();
    for old in ordered {
        let Some(original) = source.get(&old) else {
            continue;
        };
        let mut n = original.clone();
        n.id = remap[&old];
        n.revision = 0;
        n.children = n
            .children
            .iter()
            .filter_map(|id| remap.get(id).copied())
            .collect();
        n.parent = if roots.contains(&old) {
            Some(parent)
        } else {
            original.parent.and_then(|id| remap.get(&id).copied())
        };
        if roots.contains(&old) {
            n.x += offset.x;
            n.y += offset.y
        }
        if instance {
            n.component_source = Some(old);
            n.overridden_properties.clear()
        } else if let Some(component) = n.component_id {
            if let Some(new) = remap.get(&component) {
                n.component_id = Some(*new)
            }
        }
        for i in &mut n.interactions {
            match &mut i.action {
                PrototypeAction::NavigateTo { target }
                | PrototypeAction::OpenOverlay { target, .. }
                | PrototypeAction::ScrollTo { target } => {
                    if let Some(id) = remap.get(target) {
                        *target = *id
                    }
                }
                _ => {}
            }
        }
        d.nodes.insert(n.id, n);
    }
    let ids: Vec<_> = roots
        .iter()
        .filter_map(|id| remap.get(id).copied())
        .collect();
    if let Some(c) = d.children_mut(parent) {
        c.extend(ids.iter().copied())
    }
    ids
}
/// Reconciles materialized instance children by source identity. Overrides are
/// applied independently, preserving unrelated property updates from the master.
pub fn sync_component_instances(d: &mut Document, component: NodeId) -> usize {
    let source = d.nodes.clone();
    let Some(master) = source.get(&component) else {
        return 0;
    };
    if master.kind != NodeKind::Component {
        return 0;
    }
    let instances: Vec<_> = source
        .values()
        .filter(|n| n.kind == NodeKind::Instance && n.component_id == Some(component))
        .map(|n| n.id)
        .collect();
    fn sync(
        d: &mut Document,
        source: &BTreeMap<NodeId, Node>,
        target: NodeId,
        source_id: NodeId,
        root: bool,
    ) {
        let Some(master) = source.get(&source_id) else {
            return;
        };
        let Some(old) = d.nodes.get(&target).cloned() else {
            return;
        };
        let mut n = master.clone();
        n.id = target;
        n.parent = old.parent;
        n.component_source = Some(source_id);
        n.component_id = old.component_id;
        n.overridden_properties = old.overridden_properties.clone();
        n.revision = old.revision;
        n.children = old.children.clone();
        if root {
            n.kind = NodeKind::Instance;
            n.x = old.x;
            n.y = old.y;
            n.rotation = old.rotation;
            n.flip_x = old.flip_x;
            n.flip_y = old.flip_y;
            n.affine_transform = old.affine_transform
        }
        for p in &old.overridden_properties {
            match p.as_str() {
                "text" => {
                    n.text = old.text.clone();
                    n.text_runs = old.text_runs.clone()
                }
                "fills" => n.fills = old.fills.clone(),
                "strokes" => n.strokes = old.strokes.clone(),
                "effects" => n.effects = old.effects.clone(),
                "visible" => n.visible = old.visible,
                "opacity" => n.opacity = old.opacity,
                "width" => n.width = old.width,
                "height" => n.height = old.height,
                "position" => {
                    n.x = old.x;
                    n.y = old.y
                }
                "font_size" => n.font_size = old.font_size,
                "name" => n.name = old.name.clone(),
                _ => {}
            }
        }
        d.nodes.insert(target, n);
        let old_children: Vec<_> = old
            .children
            .iter()
            .filter_map(|id| d.nodes.get(id).map(|n| (*id, n.component_source)))
            .collect();
        let mut children = Vec::new();
        for source_id in &master.children {
            if let Some((child, _)) = old_children
                .iter()
                .find(|(_, sid)| *sid == Some(*source_id))
            {
                sync(d, source, *child, *source_id, false);
                children.push(*child)
            } else {
                children.extend(clone_tree_into(
                    d,
                    source,
                    &[*source_id],
                    target,
                    Point::default(),
                    true,
                ))
            }
        }
        for (child, sid) in old_children {
            if !sid.is_some_and(|id| master.children.contains(&id)) {
                d.remove_subtree(child);
            }
        }
        if let Some(n) = d.nodes.get_mut(&target) {
            n.children = children
        }
    }
    for id in &instances {
        sync(d, &source, *id, component, true)
    }
    instances.len()
}
/// Deterministic local prototype state. The host owns timers, animation and URL
/// launching; this engine owns navigation, overlays, history and variables.
#[derive(Clone, Debug, Default)]
pub struct PrototypePlayer {
    pub current: Option<NodeId>,
    pub overlays: Vec<(NodeId, Point)>,
    pub history: Vec<NodeId>,
    pub last_transition: Transition,
    pub pending_url: Option<String>,
    pub scroll_target: Option<NodeId>,
}
impl PrototypePlayer {
    pub fn start(&mut self, d: &Document, page: NodeId) -> bool {
        self.current = d.page(page).and_then(|p| {
            p.prototype_start.or_else(|| {
                p.children
                    .iter()
                    .find(|id| d.nodes.get(id).is_some_and(|n| n.kind == NodeKind::Frame))
                    .copied()
            })
        });
        self.overlays.clear();
        self.history.clear();
        self.current.is_some()
    }
    pub fn trigger(&mut self, d: &mut Document, node: NodeId, trigger: Trigger) -> bool {
        let Some(i) = d
            .nodes
            .get(&node)
            .and_then(|n| n.interactions.iter().find(|i| i.trigger == trigger))
            .cloned()
        else {
            return false;
        };
        self.last_transition = i.transition;
        match i.action {
            PrototypeAction::NavigateTo { target } => {
                if !d.nodes.contains_key(&target) {
                    return false;
                }
                if let Some(current) = self.current {
                    self.history.push(current)
                }
                self.current = Some(target);
                self.overlays.clear()
            }
            PrototypeAction::OpenOverlay { target, position } => {
                if !d.nodes.contains_key(&target) {
                    return false;
                }
                self.overlays.push((target, position))
            }
            PrototypeAction::CloseOverlay => {
                self.overlays.pop();
            }
            PrototypeAction::Back => {
                if let Some(previous) = self.history.pop() {
                    self.current = Some(previous)
                }
            }
            PrototypeAction::SetVariable { variable, value } => {
                if let Some(v) = d.variables.get_mut(&variable) {
                    v.values.insert(d.variable_mode.clone(), value);
                    d.apply_variables();
                }
            }
            PrototypeAction::OpenUrl { url } => self.pending_url = Some(url),
            PrototypeAction::ScrollTo { target } => {
                if !d.nodes.contains_key(&target) {
                    return false;
                }
                self.scroll_target = Some(target)
            }
        }
        true
    }
}
