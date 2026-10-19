//! Native pointer ownership and public DrawingContext insertion adorners.
//! A retained core session keeps pointer updates independent of document size.
use super::*;

const DRAG_THRESHOLD: f64 = 4.0;

pub(super) struct LayoutDragGesture {
    session: core::LayoutDragSession,
    start_world: Point,
    start_screen: Point,
    current_world: Point,
    active: bool,
    target: Option<core::LayoutDropTarget>,
}

impl LayoutDragGesture {
    fn new(session: core::LayoutDragSession, world: Point, screen: Point) -> Self {
        Self {
            session,
            start_world: world,
            start_screen: screen,
            current_world: world,
            active: false,
            target: None,
        }
    }

    fn update(&mut self, world: Point, screen: Point) {
        self.current_world = world;
        self.active |= self.start_screen.distance(screen) >= DRAG_THRESHOLD;
        if self.active {
            self.target = self.session.target(world);
        }
    }

    fn owns(&self, editor: &Editor) -> bool {
        editor.document.revision == self.session.revision()
            && editor.revision == self.session.revision()
            && editor.selection.as_slice() == [self.session.child()]
            && !editor.has_active_transaction()
    }
}

impl DesignCanvas {
    /// State of the actual retained native pointer gesture, for diagnostics and
    /// browser regression probes. Reading it never borrows or edits the model.
    pub fn layout_drag_state(&self) -> Option<serde_json::Value> {
        self.layout_drag.borrow().as_ref().map(|gesture| {
            serde_json::json!({
                "child": gesture.session.child(), "parent": gesture.session.parent(),
                "active": gesture.active, "target": gesture.target,
            })
        })
    }

    /// A flow-owned child must not fall through to arbitrary x/y movement when
    /// it is pinned, alone, or inside an instance. Absolute children retain the
    /// ordinary free-move gesture; multi-selection remains its existing mode.
    pub(super) fn layout_drag_press(
        &self,
        editor: &Editor,
        child: NodeId,
        world: Point,
        screen: Point,
    ) -> bool {
        if editor.selection.as_slice() != [child] {
            return false;
        }
        let Some(node) = editor.document.nodes.get(&child) else {
            return false;
        };
        if !node.visible || node.layout_child.absolute {
            return false;
        }
        if !node
            .parent
            .and_then(|parent| editor.document.nodes.get(&parent))
            .is_some_and(|parent| parent.layout.is_some())
        {
            return false;
        }
        if !editor.has_active_transaction() {
            if let Ok(session) = core::LayoutDragSession::new(&editor.document, child) {
                *self.layout_drag.borrow_mut() =
                    Some(LayoutDragGesture::new(session, world, screen));
                self.hover.set(None);
                self.snap_guides.set((None, None));
            }
        }
        true
    }

    pub fn cancel_layout_drag(&self) -> bool {
        if self.layout_drag.borrow_mut().take().is_none() {
            return false;
        }
        // No document or transaction ever belonged to this deferred preview.
        self.invalidate_visual();
        true
    }

    pub(super) fn layout_drag_move(&self, world: Point, screen: Point) -> bool {
        let mut active = self.layout_drag.borrow_mut();
        let Some(gesture) = active.as_mut() else {
            return false;
        };
        let valid = gesture.owns(&self.editor.borrow())
            && self.tool.get() == Tool::Select
            && self.presentation.get().is_none();
        if valid {
            gesture.update(world, screen);
        } else {
            *active = None;
        }
        drop(active);
        self.invalidate_visual();
        true
    }

    /// Take ownership before releasing capture or raising callbacks. A late
    /// release after Escape, Undo, a camera/tool change, or capture loss cannot
    /// use an old session to write a new history entry.
    pub(super) fn layout_drag_release(&self, world: Point, screen: Point) -> bool {
        let Some(mut gesture) = self.layout_drag.borrow_mut().take() else {
            return false;
        };
        gesture.update(world, screen);
        let result = {
            let mut editor = self.editor.borrow_mut();
            if gesture.owns(&editor) && gesture.active {
                gesture
                    .target
                    .map(|target| editor.commit_layout_drag(&gesture.session, target))
            } else {
                None
            }
        };
        self.invalidate_visual();
        if let Some(result) = result {
            match result {
                Ok(result) => {
                    // A nontrivial order change can retain identical bounds,
                    // so use the target index rather than changed geometry.
                    if gesture
                        .target
                        .is_some_and(|target| gesture.session.changes_order(target))
                    {
                        self.emit(CanvasEvent::Document);
                    }
                    if !result.warnings.is_empty() {
                        self.emit(CanvasEvent::Notice(result.warnings.join("\n")));
                    }
                }
                Err(error) => self.emit(CanvasEvent::Notice(error.to_string())),
            }
        }
        true
    }

    pub(super) fn render_layout_drag(&self, context: &mut DrawingContext, viewport: Viewport) {
        let active = self.layout_drag.borrow();
        let Some(gesture) = active.as_ref().filter(|gesture| gesture.active) else {
            return;
        };
        let delta = gesture.current_world - gesture.start_world;
        let corners = gesture
            .session
            .source_corners()
            .map(|point| viewport.world_to_screen(point + delta));
        let outline = pen(0xA60D99FF, 1.0);
        for index in 0..4 {
            context.draw_line(&outline, ui(corners[index]), ui(corners[(index + 1) % 4]));
        }
        let Some(target) = gesture.target else {
            return;
        };
        let mut start = viewport.world_to_screen(target.marker.start);
        let mut end = viewport.world_to_screen(target.marker.end);
        let direction = end - start;
        let length = direction.distance(Point::default());
        let direction = if length > 1e-6 {
            direction * (1.0 / length)
        } else {
            Point::new(0.0, 1.0)
        };
        // Very small/zero-size children still have a discoverable insertion
        // edge. All adorner dimensions are screen pixels at any canvas zoom.
        if length < 12.0 {
            let center = (start + end) * 0.5;
            start = center - direction * 6.0;
            end = center + direction * 6.0;
        }
        let normal = Point::new(-direction.y, direction.x) * 4.0;
        let halo = pen(0xFFFFFFFF, 4.0);
        let guide = pen(0xFF0D99FF, 2.0);
        context.draw_line(&halo, ui(start), ui(end));
        context.draw_line(&guide, ui(start), ui(end));
        context.draw_line(&guide, ui(start - normal), ui(start + normal));
        context.draw_line(&guide, ui(end - normal), ui(end + normal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gesture() -> (Editor, LayoutDragGesture, Point) {
        let mut document = core::Document::new("Pointer ownership");
        let page = document.pages[0].id;
        let mut frame = Node::new(NodeKind::Frame, "Row", 10.0, 20.0, 300.0, 120.0);
        frame.layout = Some(core::AutoLayout::default());
        let parent = document.insert_node(page, frame).unwrap();
        let children: Vec<_> = (0..3)
            .map(|index| {
                document
                    .insert_node(
                        parent,
                        Node::new(
                            NodeKind::Rectangle,
                            format!("Child {index}"),
                            0.0,
                            0.0,
                            50.0,
                            40.0,
                        ),
                    )
                    .unwrap()
            })
            .collect();
        document.layout_all();
        let mut editor = Editor::new(document);
        editor.selection = vec![children[0]];
        let start = editor.document.world_bounds(children[0]).unwrap().center();
        let last = editor.document.world_bounds(children[2]).unwrap();
        let end = Point::new(last.right() - 1.0, last.center().y);
        let session = core::LayoutDragSession::new(&editor.document, children[0]).unwrap();
        (editor, LayoutDragGesture::new(session, start, start), end)
    }

    #[test]
    fn native_drag_threshold_and_release_position_do_not_need_intermediate_pointer_events() {
        let (mut editor, mut gesture, end) = gesture();
        gesture.update(
            gesture.start_world + Point::new(3.0, 0.0),
            gesture.start_screen + Point::new(3.0, 0.0),
        );
        assert!(!gesture.active);
        assert!(gesture.target.is_none());
        gesture.update(end, end);
        assert!(gesture.active && gesture.owns(&editor));
        let target = gesture.target.unwrap();
        assert_eq!(target.index, 2);
        assert!(!editor.can_undo());
        editor.commit_layout_drag(&gesture.session, target).unwrap();
        assert_eq!(editor.history_len(), 1);
        assert!(!gesture.owns(&editor));
    }

    #[test]
    fn deferred_gesture_does_not_own_unrelated_pending_history_or_replaced_selection() {
        let (mut editor, gesture, _) = gesture();
        assert!(gesture.owns(&editor));
        editor.begin_transaction("Another editor");
        assert!(!gesture.owns(&editor));
        editor.cancel_transaction();
        assert!(
            !gesture.owns(&editor),
            "Restoration changed the document revision"
        );
        let session = core::LayoutDragSession::new(&editor.document, editor.selection[0]).unwrap();
        let gesture = LayoutDragGesture::new(session, Point::default(), Point::default());
        editor.selection.clear();
        assert!(!gesture.owns(&editor));
    }

    #[test]
    fn actual_native_capture_loss_discards_preview_and_late_release_without_history() {
        use ferroui_base::input::{IPointer, Pointer as NativePointer, PointerType};

        for transfer in [false, true] {
            let (editor, mut gesture, end) = gesture();
            let before = editor.document.to_json().unwrap();
            let revision = editor.revision;
            gesture.update(end, end);
            let editor = Rc::new(RefCell::new(editor));
            let canvas = DesignCanvas::new(editor.clone());
            *canvas.layout_drag.borrow_mut() = Some(gesture);
            let pointer =
                NativePointer::new(NativePointer::get_next_free_id(), PointerType::Mouse, true);
            pointer.capture(Some(&canvas.clone().upcast::<InputElement>()));
            if transfer {
                pointer.capture(Some(&InputElement::new()));
            } else {
                pointer.platform_capture_lost();
            }
            assert!(canvas.layout_drag_state().is_none());
            assert!(!canvas.layout_drag_move(end + Point::new(10.0, 0.0), end));
            assert!(!canvas.layout_drag_release(end, end));
            assert_eq!(editor.borrow().document.to_json().unwrap(), before);
            assert_eq!(editor.borrow().revision, revision);
            assert!(!editor.borrow().can_undo());
            assert!(!editor.borrow().has_active_transaction());
        }
    }

    #[test]
    fn successful_drop_clears_preview_before_synchronous_capture_loss_can_cancel_it() {
        use ferroui_base::input::{IPointer, Pointer as NativePointer, PointerType};

        let (editor, gesture, end) = gesture();
        let parent = gesture.session.parent();
        let child = gesture.session.child();
        let editor = Rc::new(RefCell::new(editor));
        let canvas = DesignCanvas::new(editor.clone());
        *canvas.layout_drag.borrow_mut() = Some(gesture);
        let pointer =
            NativePointer::new(NativePointer::get_next_free_id(), PointerType::Mouse, true);
        pointer.capture(Some(&canvas.clone().upcast::<InputElement>()));
        assert!(canvas.layout_drag_release(end, end));
        pointer.capture(None);
        assert!(canvas.layout_drag_state().is_none());
        assert_eq!(
            editor.borrow().document.nodes[&parent].children.last(),
            Some(&child)
        );
        assert_eq!(editor.borrow().history_len(), 1);
        assert!(!editor.borrow().has_active_transaction());
    }

    #[test]
    fn escape_cancellation_does_not_restore_or_commit_an_unrelated_pending_edit() {
        let (editor, gesture, _) = gesture();
        let child = gesture.session.child();
        let editor = Rc::new(RefCell::new(editor));
        let canvas = DesignCanvas::new(editor.clone());
        *canvas.layout_drag.borrow_mut() = Some(gesture);
        editor.borrow_mut().begin_transaction("External edit");
        editor
            .borrow_mut()
            .document
            .nodes
            .get_mut(&child)
            .unwrap()
            .name = "External draft".into();
        canvas.cancel();
        assert!(canvas.layout_drag_state().is_none());
        assert!(editor.borrow().has_active_transaction());
        assert_eq!(
            editor.borrow().document.nodes[&child].name,
            "External draft"
        );
        assert!(!editor.borrow().can_undo());
        editor.borrow_mut().cancel_transaction();
        assert_ne!(
            editor.borrow().document.nodes[&child].name,
            "External draft"
        );
    }
}
