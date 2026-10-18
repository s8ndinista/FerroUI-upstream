//! Interactive retained-mode surface. Artwork uses only FerroUI's public DrawingContext API.
use crate::{
    icons::brush,
    renderer::{RenderStats, ScenePainter},
};
use ferrodesign_core::{
    self as core, Color, Editor, Node, NodeId, NodeKind, PathSegment, Point, Rect, VectorPath,
    Viewport,
};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IPen, Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point as UiPoint,
    Rect as UiRect, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Frame,
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Star,
    Pen,
    Pencil,
    Text,
    Hand,
    Comment,
    Section,
}
impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Frame => "Frame",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Polygon => "Polygon",
            Self::Star => "Star",
            Self::Pen => "Pen",
            Self::Pencil => "Pencil",
            Self::Text => "Text",
            Self::Hand => "Hand",
            Self::Comment => "Comment",
            Self::Section => "Section",
        }
    }
    fn kind(self) -> NodeKind {
        match self {
            Self::Frame => NodeKind::Frame,
            Self::Ellipse => NodeKind::Ellipse,
            Self::Line => NodeKind::Line,
            Self::Polygon => NodeKind::Polygon,
            Self::Star => NodeKind::Star,
            Self::Pen | Self::Pencil => NodeKind::Vector,
            Self::Text => NodeKind::Text,
            Self::Section => NodeKind::Section,
            _ => NodeKind::Rectangle,
        }
    }
}
#[derive(Clone)]
pub enum CanvasEvent {
    Selection,
    Document,
    Camera,
    Tool,
    EditText(NodeId),
    Comment(Point),
    ContextMenu,
    PrototypeClick(NodeId),
    ExitPresentation,
    Action(&'static str),
}
enum Gesture {
    None,
    Pan {
        start: Point,
        original: Point,
    },
    Move {
        start: Point,
        nodes: Vec<Node>,
        bounds: Option<Rect>,
        exclude: Vec<NodeId>,
        moved: bool,
    },
    Resize {
        node: Node,
        corner: usize,
    },
    Rotate {
        node: Node,
        center: Point,
        angle: f64,
    },
    Marquee {
        start: Point,
        current: Point,
        additive: bool,
    },
    Draw {
        start: Point,
        id: NodeId,
    },
    Pencil {
        id: NodeId,
        origin: Point,
    },
}
#[repr(C)]
pub struct DesignCanvas {
    base: Control,
    pub editor: Rc<RefCell<Editor>>,
    pub viewport: Cell<Viewport>,
    pub tool: Cell<Tool>,
    gesture: RefCell<Gesture>,
    painter: RefCell<ScenePainter>,
    callback: RefCell<Option<Rc<dyn Fn(CanvasEvent)>>>,
    hover: Cell<Option<NodeId>>,
    space: Cell<bool>,
    initialized: Cell<bool>,
    pub show_grid: Cell<bool>,
    pub show_rulers: Cell<bool>,
    pub dark: Cell<bool>,
    pub presentation: Cell<Option<NodeId>>,
    pub overlays: RefCell<Vec<(NodeId, Point)>>,
    pub frames: Cell<u64>,
    pub snap: Cell<bool>,
    snap_guides: Cell<(Option<f64>, Option<f64>)>,
    pen_node: Cell<Option<NodeId>>,
}
ferro_class!(DesignCanvas:Control);
ferro_impl_classes!(DesignCanvas:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
impl DesignCanvas {
    pub fn new(editor: Rc<RefCell<Editor>>) -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            editor,
            viewport: Cell::new(Viewport::default()),
            tool: Cell::new(Tool::Select),
            gesture: RefCell::new(Gesture::None),
            painter: RefCell::new(ScenePainter::default()),
            callback: RefCell::new(None),
            hover: Cell::new(None),
            space: Cell::new(false),
            initialized: Cell::new(false),
            show_grid: Cell::new(false),
            show_rulers: Cell::new(false),
            dark: Cell::new(false),
            presentation: Cell::new(None),
            overlays: RefCell::new(Vec::new()),
            frames: Cell::new(0),
            snap: Cell::new(true),
            snap_guides: Cell::new((None, None)),
            pen_node: Cell::new(None),
        });
        this.set_focusable(true);
        this.set_clip_to_bounds(true);
        this
    }
    pub fn on_change(&self, callback: impl Fn(CanvasEvent) + 'static) {
        *self.callback.borrow_mut() = Some(Rc::new(callback));
    }
    fn emit(&self, event: CanvasEvent) {
        let callback = self.callback.borrow().clone();
        if let Some(f) = callback {
            f(event)
        }
    }
    pub fn set_tool(&self, tool: Tool) {
        self.finish_pen();
        self.tool.set(tool);
        self.focus();
        self.invalidate_visual();
        self.emit(CanvasEvent::Tool);
    }
    pub fn reset_fit(&self) {
        self.initialized.set(false);
        self.invalidate_visual();
    }
    pub fn zoom(&self, factor: f64) {
        let mut v = self.viewport.get();
        v.zoom_at(Point::new(v.width / 2.0, v.height / 2.0), v.zoom * factor);
        self.viewport.set(v);
        self.invalidate_visual();
        self.emit(CanvasEvent::Camera);
    }
    pub fn zoom_to_selection(&self) {
        let editor = self.editor.borrow();
        let bounds = editor
            .selection
            .iter()
            .filter_map(|&id| editor.document.world_bounds(id))
            .reduce(|a, b| a.union(b));
        if let Some(bounds) = bounds {
            let mut v = self.viewport.get();
            v.fit(bounds, 90.0);
            self.viewport.set(v);
            self.invalidate_visual();
        }
        drop(editor);
        self.emit(CanvasEvent::Camera);
    }
    pub fn render_stats(&self) -> RenderStats {
        self.painter.borrow().stats()
    }
    pub fn register_image_data(&self, href: &str) -> std::io::Result<(f64, f64)> {
        self.painter.borrow_mut().register_image_data(href)
    }
    pub fn export_png(&self) -> Result<Vec<u8>, String> {
        use ferroui_base::media::imaging::{
            BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap,
        };
        use ferroui_base::PixelSize;
        let editor = self.editor.borrow();
        let roots = if editor.selection.is_empty() {
            editor
                .document
                .page(editor.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        } else {
            editor.root_selection()
        };
        let bounds = self
            .painter
            .borrow_mut()
            .rendered_bounds(&editor.document, &roots)
            .ok_or_else(|| "There are no layers to export.".to_string())?;
        let width = bounds.width.ceil().max(1.0);
        let height = bounds.height.ceil().max(1.0);
        if width > 16384.0 || height > 16384.0 || width * height > 32_000_000.0 {
            return Err("Select a smaller region; PNG export supports up to 32 megapixels.".into());
        }
        let bitmap = RenderTargetBitmap::new(PixelSize::new(width as i32, height as i32));
        {
            let mut context = bitmap.create_drawing_context();
            let transform =
                context.push_transform(Matrix::create_translation(-bounds.x, -bounds.y));
            self.painter
                .borrow_mut()
                .render(&mut context, &editor.document, &roots, bounds);
            context.pop(transform);
            context.dispose();
        }
        let mut bytes = Vec::new();
        let result = bitmap.save(
            &mut bytes,
            &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default()),
        );
        bitmap.dispose();
        result.map_err(|e| e.to_string())?;
        Ok(bytes)
    }
    pub fn invalidate_scene(&self) {
        self.painter.borrow_mut().invalidate();
        self.invalidate_visual();
    }
    pub fn finish_pen(&self) {
        if let Some(id) = self.pen_node.take() {
            let mut editor = self.editor.borrow_mut();
            normalize_drawn_vector(&mut editor, id);
            editor.commit_transaction();
            drop(editor);
            self.invalidate_visual();
            self.emit(CanvasEvent::Document);
        }
    }
    pub fn cancel(&self) {
        self.editor.borrow_mut().cancel_transaction();
        self.pen_node.set(None);
        *self.gesture.borrow_mut() = Gesture::None;
        self.snap_guides.set((None, None));
        self.space.set(false);
        self.invalidate_visual();
        self.emit(CanvasEvent::Document);
    }
    fn world(&self, p: UiPoint) -> Point {
        self.viewport.get().screen_to_world(Point::new(p.x, p.y))
    }
    fn snap_point(&self, p: Point) -> Point {
        if self.snap.get() {
            Point::new(p.x.round(), p.y.round())
        } else {
            p
        }
    }
    fn pick(&self, p: Point) -> Option<NodeId> {
        self.editor
            .borrow()
            .hit_test(p, 3.0 / self.viewport.get().zoom)
    }
    fn handle_at(&self, p: Point) -> Option<usize> {
        let editor = self.editor.borrow();
        if editor.selection.len() != 1 {
            return None;
        }
        let id = editor.selection[0];
        let node = editor.document.nodes.get(&id)?;
        if node.locked {
            return None;
        }
        let transform = editor.document.world_transform(id)?;
        let corners = node.local_bounds().corners().map(|p| transform.apply(p));
        let tolerance = 8.0 / self.viewport.get().zoom;
        for (i, c) in corners.iter().enumerate() {
            if c.distance(p) < tolerance {
                return Some(i);
            }
        }
        let top = (corners[0] + corners[1]) * 0.5;
        let center = transform.apply(Point::new(node.width / 2.0, node.height / 2.0));
        let direction = top - center;
        let length = direction.distance(Point::new(0.0, 0.0)).max(0.001);
        if (top + direction * (24.0 / self.viewport.get().zoom / length)).distance(p) < tolerance {
            return Some(4);
        }
        None
    }
    fn new_node(&self, tool: Tool, p: Point) -> NodeId {
        let mut editor = self.editor.borrow_mut();
        editor.begin_transaction(format!("Draw {}", tool.name()));
        let mut node = Node::new(tool.kind(), tool.name(), p.x, p.y, 1.0, 1.0)
            .with_fill(Color::rgb(123, 97, 255));
        if tool == Tool::Frame {
            node = node.with_fill(Color::WHITE);
            node.clip_content = true;
        }
        if tool == Tool::Section {
            node = node.with_fill(Color::rgba(216, 209, 255, 48));
            node.width = 400.0;
            node.height = 300.0;
        }
        if tool == Tool::Text {
            node = node.with_text("Text", 24.0, Color::rgb(30, 30, 30));
            node.width = 160.0;
            node.height = 36.0;
        }
        if matches!(tool, Tool::Line | Tool::Pen | Tool::Pencil) {
            node.fills.clear();
            node.strokes = vec![core::Stroke::new(Color::rgb(30, 30, 30), 2.0)];
        }
        if tool == Tool::Star {
            node.polygon_sides = 5;
        }
        let parent = editor.active_page;
        let id = editor
            .document
            .insert_node(parent, node)
            .expect("valid page");
        editor.selection = vec![id];
        editor.refresh();
        id
    }
}
impl InputElementImpl for DesignCanvas {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        this.focus();
        e.set_handled(true);
        let screen = e.get_position(Some(this));
        let screen = Point::new(screen.x, screen.y);
        let world = this.viewport.get().screen_to_world(screen);
        if e.properties().is_right_button_pressed {
            this.emit(CanvasEvent::ContextMenu);
            return;
        }
        e.pointer()
            .capture(Some(&this.to_ref().upcast::<InputElement>()));
        if this.presentation.get().is_some() {
            let mut point = world;
            let editor = this.editor.borrow();
            let mut root = this.presentation.get().unwrap();
            if let Some(current_bounds) = editor.document.world_bounds(root) {
                for &(id, position) in this.overlays.borrow().iter().rev() {
                    if let Some(bounds) = editor.document.world_bounds(id) {
                        let dx = current_bounds.x + position.x - bounds.x;
                        let dy = current_bounds.y + position.y - bounds.y;
                        let mapped = Point::new(world.x - dx, world.y - dy);
                        if bounds.contains(mapped) {
                            point = mapped;
                            root = id;
                            break;
                        }
                    }
                }
            }
            let id = editor
                .hit_test(point, 3.0 / this.viewport.get().zoom)
                .filter(|&id| id == root || editor.document.is_ancestor(root, id));
            drop(editor);
            if let Some(id) = id {
                this.emit(CanvasEvent::PrototypeClick(id));
            }
            return;
        }
        if this.space.get()
            || this.tool.get() == Tool::Hand
            || e.properties().is_middle_button_pressed
        {
            *this.gesture.borrow_mut() = Gesture::Pan {
                start: screen,
                original: this.viewport.get().pan,
            };
            return;
        }
        let additive = e.key_modifiers().contains(KeyModifiers::SHIFT);
        match this.tool.get() {
            Tool::Comment => this.emit(CanvasEvent::Comment(world)),
            Tool::Select => {
                if let Some(corner) = this.handle_at(world) {
                    let mut editor = this.editor.borrow_mut();
                    let id = editor.selection[0];
                    let node = editor.document.nodes[&id].clone();
                    if corner == 4 {
                        let center = editor
                            .document
                            .world_transform(id)
                            .unwrap()
                            .apply(Point::new(node.width / 2.0, node.height / 2.0));
                        let angle = (world.y - center.y).atan2(world.x - center.x).to_degrees();
                        editor.begin_transaction("Rotate");
                        *this.gesture.borrow_mut() = Gesture::Rotate {
                            node,
                            center,
                            angle,
                        };
                    } else {
                        editor.begin_transaction("Resize");
                        *this.gesture.borrow_mut() = Gesture::Resize { node, corner };
                    }
                } else if let Some(id) = this.pick(world) {
                    let mut editor = this.editor.borrow_mut();
                    if additive {
                        if editor.selection.contains(&id) {
                            editor.selection.retain(|x| *x != id);
                        } else {
                            editor.selection.push(id);
                        }
                    } else if !editor.selection.contains(&id) {
                        editor.selection = vec![id];
                    }
                    if e.click_count() == 2 && editor.document.nodes[&id].kind == NodeKind::Text {
                        drop(editor);
                        this.emit(CanvasEvent::EditText(id));
                        return;
                    }
                    let exclude = editor.root_selection();
                    let bounds = exclude
                        .iter()
                        .filter_map(|&id| editor.document.world_bounds(id))
                        .reduce(|a, b| a.union(b));
                    let nodes = exclude
                        .iter()
                        .filter_map(|id| editor.document.nodes.get(id).cloned())
                        .collect();
                    editor.begin_transaction("Move layers");
                    *this.gesture.borrow_mut() = Gesture::Move {
                        start: world,
                        nodes,
                        bounds,
                        exclude,
                        moved: false,
                    };
                    drop(editor);
                    this.emit(CanvasEvent::Selection);
                } else {
                    if !additive {
                        this.editor.borrow_mut().selection.clear();
                    }
                    *this.gesture.borrow_mut() = Gesture::Marquee {
                        start: world,
                        current: world,
                        additive,
                    };
                    this.emit(CanvasEvent::Selection);
                }
            }
            Tool::Pen => {
                let p = this.snap_point(world);
                let id = if let Some(id) = this.pen_node.get() {
                    id
                } else {
                    let id = this.new_node(Tool::Pen, p);
                    this.pen_node.set(Some(id));
                    id
                };
                let mut close = false;
                {
                    let mut editor = this.editor.borrow_mut();
                    let node = editor.document.nodes.get_mut(&id).unwrap();
                    let local = Point::new(p.x - node.x, p.y - node.y);
                    if let Some(PathSegment::MoveTo { point }) = node.path.segments.first() {
                        if node.path.segments.len() > 2
                            && local.distance(*point) < 8.0 / this.viewport.get().zoom
                        {
                            node.path.segments.push(PathSegment::Close);
                            node.fills = vec![core::Fill::solid(Color::rgb(123, 97, 255))];
                            close = true;
                        } else {
                            node.path
                                .segments
                                .push(PathSegment::LineTo { point: local });
                        }
                    } else {
                        node.path = VectorPath::default();
                        node.path
                            .segments
                            .push(PathSegment::MoveTo { point: local });
                    }
                    node.width = node.width.max(local.x);
                    node.height = node.height.max(local.y);
                    editor.refresh();
                }
                if close {
                    this.finish_pen();
                    this.tool.set(Tool::Select);
                }
                this.emit(CanvasEvent::Selection);
            }
            tool => {
                let start = this.snap_point(world);
                let id = this.new_node(tool, start);
                if tool == Tool::Text {
                    this.editor.borrow_mut().commit_transaction();
                    this.tool.set(Tool::Select);
                    this.emit(CanvasEvent::Document);
                    this.emit(CanvasEvent::EditText(id));
                } else if tool == Tool::Pencil {
                    this.editor
                        .borrow_mut()
                        .document
                        .nodes
                        .get_mut(&id)
                        .unwrap()
                        .path
                        .segments
                        .push(PathSegment::MoveTo {
                            point: Point::new(0.0, 0.0),
                        });
                    *this.gesture.borrow_mut() = Gesture::Pencil { id, origin: start };
                } else {
                    *this.gesture.borrow_mut() = Gesture::Draw { start, id };
                }
                this.emit(CanvasEvent::Selection);
            }
        }
        this.invalidate_visual();
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let screen = e.get_position(Some(this));
        let world = this.world(screen);
        let shift = e.key_modifiers().contains(KeyModifiers::SHIFT);
        let mut camera_changed = false;
        let mut gesture = this.gesture.borrow_mut();
        match &mut *gesture {
            Gesture::None => {
                let hover = this.pick(world);
                if this.hover.replace(hover) != hover {
                    this.invalidate_visual();
                }
                return;
            }
            Gesture::Pan { start, original } => {
                let mut v = this.viewport.get();
                v.pan = *original + Point::new(screen.x - start.x, screen.y - start.y);
                this.viewport.set(v);
                camera_changed = true;
            }
            Gesture::Move {
                start,
                nodes,
                bounds,
                exclude,
                moved,
            } => {
                let mut delta = world - *start;
                if delta.distance(Point::new(0.0, 0.0)) < 0.5 / this.viewport.get().zoom && !*moved
                {
                    return;
                }
                if shift {
                    if delta.x.abs() > delta.y.abs() {
                        delta.y = 0.0
                    } else {
                        delta.x = 0.0
                    }
                }
                if this.snap.get() {
                    delta.x = delta.x.round();
                    delta.y = delta.y.round();
                }
                let mut editor = this.editor.borrow_mut();
                this.snap_guides.set((None, None));
                if this.snap.get() {
                    if let Some(bounds) = bounds {
                        let proposed = Rect::new(
                            bounds.x + delta.x,
                            bounds.y + delta.y,
                            bounds.width,
                            bounds.height,
                        );
                        let snap =
                            editor.smart_snap(proposed, 5.0 / this.viewport.get().zoom, exclude);
                        // An axis-constrained drag must stay on its original axis.
                        let snap_x = !shift || delta.x != 0.0;
                        let snap_y = !shift || delta.y != 0.0;
                        if snap_x {
                            delta.x += snap.delta.x;
                        }
                        if snap_y {
                            delta.y += snap.delta.y;
                        }
                        this.snap_guides.set((
                            if snap_x { snap.vertical_guide } else { None },
                            if snap_y { snap.horizontal_guide } else { None },
                        ));
                    }
                }
                for original in nodes.iter() {
                    let parent = original
                        .parent
                        .and_then(|id| editor.document.world_transform(id))
                        .unwrap_or(core::Transform::IDENTITY);
                    let local = parent
                        .inverse()
                        .unwrap_or(core::Transform::IDENTITY)
                        .apply_vector(delta);
                    if let Some(node) = editor.document.nodes.get_mut(&original.id) {
                        node.x = original.x + local.x;
                        node.y = original.y + local.y;
                    }
                }
                editor.refresh();
                *moved = true;
            }
            Gesture::Resize { node, corner } => {
                let mut editor = this.editor.borrow_mut();
                let parent = node
                    .parent
                    .and_then(|id| editor.document.world_transform(id))
                    .unwrap_or(core::Transform::IDENTITY);
                let transform = parent * node.local_transform();
                let local = transform
                    .inverse()
                    .unwrap_or(core::Transform::IDENTITY)
                    .apply(world);
                let opposite = match *corner {
                    0 => Point::new(node.width, node.height),
                    1 => Point::new(0.0, node.height),
                    2 => Point::new(0.0, 0.0),
                    _ => Point::new(node.width, 0.0),
                };
                let mut width = (local.x - opposite.x).abs().max(1.0);
                let mut height = (local.y - opposite.y).abs().max(1.0);
                if shift {
                    let ratio = node.width / node.height.max(0.001);
                    if width / height > ratio {
                        height = width / ratio
                    } else {
                        width = height * ratio
                    }
                }
                if this.snap.get() {
                    width = width.round().max(1.0);
                    height = height.round().max(1.0);
                }
                let new_opposite = match *corner {
                    0 => Point::new(width, height),
                    1 => Point::new(0.0, height),
                    2 => Point::new(0.0, 0.0),
                    _ => Point::new(width, 0.0),
                };
                let mut updated = node.clone();
                updated.x = 0.0;
                updated.y = 0.0;
                updated.width = width;
                updated.height = height;
                let anchor_in_parent = node.local_transform().apply(opposite);
                let offset = updated.local_transform().apply(new_opposite);
                editor.resize_node(node.id, width, height);
                if let Some(n) = editor.document.nodes.get_mut(&node.id) {
                    n.x = anchor_in_parent.x - offset.x;
                    n.y = anchor_in_parent.y - offset.y;
                }
                editor.refresh();
            }
            Gesture::Rotate {
                node,
                center,
                angle,
            } => {
                let mut rotation = node.rotation
                    + (world.y - center.y).atan2(world.x - center.x).to_degrees()
                    - *angle;
                if shift {
                    rotation = (rotation / 15.0).round() * 15.0
                }
                let mut editor = this.editor.borrow_mut();
                if let Some(n) = editor.document.nodes.get_mut(&node.id) {
                    n.rotation = rotation;
                }
                editor.refresh();
            }
            Gesture::Marquee { current, .. } => *current = world,
            Gesture::Draw { start, id } => {
                let mut end = this.snap_point(world);
                if shift {
                    let size = (end.x - start.x).abs().max((end.y - start.y).abs());
                    end = Point::new(
                        start.x + size * (end.x - start.x).signum(),
                        start.y + size * (end.y - start.y).signum(),
                    );
                }
                let bounds = Rect::from_points(*start, end);
                let mut editor = this.editor.borrow_mut();
                if let Some(n) = editor.document.nodes.get_mut(id) {
                    n.x = bounds.x;
                    n.y = bounds.y;
                    n.width = bounds.width.max(1.0);
                    n.height = bounds.height.max(1.0);
                    if n.kind == NodeKind::Line {
                        n.flip_y = (end.x - start.x) * (end.y - start.y) < 0.0;
                    }
                }
                editor.refresh();
            }
            Gesture::Pencil { id, origin } => {
                let local = world - *origin;
                let mut editor = this.editor.borrow_mut();
                if let Some(n) = editor.document.nodes.get_mut(id) {
                    // Sample at approximately one screen pixel to avoid redundant path segments.
                    let last = n.path.segments.last().and_then(|s| match s {
                        PathSegment::MoveTo { point } | PathSegment::LineTo { point } => {
                            Some(*point)
                        }
                        _ => None,
                    });
                    if last.is_none_or(|p| p.distance(local) > 0.8 / this.viewport.get().zoom) {
                        n.path.segments.push(PathSegment::LineTo { point: local });
                        n.width = n.width.max(local.x);
                        n.height = n.height.max(local.y);
                    }
                }
                editor.refresh();
            }
        }
        drop(gesture);
        this.invalidate_visual();
        if camera_changed {
            this.emit(CanvasEvent::Camera);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        e.pointer().capture(None);
        this.snap_guides.set((None, None));
        let gesture = std::mem::replace(&mut *this.gesture.borrow_mut(), Gesture::None);
        match gesture {
            Gesture::Marquee {
                start,
                current,
                additive,
            } => {
                this.editor
                    .borrow_mut()
                    .select_rect(Rect::from_points(start, current), additive);
                this.emit(CanvasEvent::Selection);
            }
            Gesture::Move { moved: false, .. } => {
                this.editor.borrow_mut().cancel_transaction();
            }
            Gesture::Pencil { id, .. } => {
                let mut editor = this.editor.borrow_mut();
                normalize_drawn_vector(&mut editor, id);
                editor.commit_transaction();
                drop(editor);
                this.tool.set(Tool::Select);
                this.emit(CanvasEvent::Document);
            }
            Gesture::Move { .. }
            | Gesture::Resize { .. }
            | Gesture::Rotate { .. }
            | Gesture::Draw { .. } => {
                this.editor.borrow_mut().commit_transaction();
                this.tool.set(Tool::Select);
                this.emit(CanvasEvent::Document);
            }
            _ => {}
        }
        this.invalidate_visual();
    }
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        e.set_handled(true);
        let mut v = this.viewport.get();
        if e.key_modifiers()
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
        {
            let p = e.get_position(Some(this));
            v.zoom_at(Point::new(p.x, p.y), v.zoom * (e.delta().y * 0.16).exp());
        } else {
            v.pan = v.pan + Point::new(e.delta().x * 36.0, e.delta().y * 36.0);
        }
        this.viewport.set(v);
        this.invalidate_visual();
        this.emit(CanvasEvent::Camera);
    }
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.key == Key::Space {
            this.space.set(true);
            e.set_handled(true);
            return;
        }
        if e.key == Key::Escape {
            if this.presentation.get().is_some() {
                this.emit(CanvasEvent::ExitPresentation)
            } else {
                this.cancel()
            }
            e.set_handled(true);
            return;
        }
        if e.key == Key::Enter && this.pen_node.get().is_some() {
            this.finish_pen();
            e.set_handled(true);
            return;
        }
        let command = e
            .key_modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);
        let alt = e.key_modifiers.contains(KeyModifiers::ALT);
        let action = if command {
            match e.key {
                Key::Z => Some(if shift { "redo" } else { "undo" }),
                Key::Y => Some("redo"),
                Key::C => Some("copy"),
                Key::X => Some("cut"),
                Key::V => Some("paste"),
                Key::D => Some("duplicate"),
                Key::A => Some("select-all"),
                Key::G => Some(if alt {
                    "frame-selection"
                } else if shift {
                    "ungroup"
                } else {
                    "group"
                }),
                Key::S => Some("save"),
                Key::O => Some("open"),
                Key::K => Some(if shift {
                    "place-image"
                } else if alt {
                    "component"
                } else {
                    "actions"
                }),
                _ => None,
            }
        } else {
            match e.key {
                Key::Delete | Key::Back => Some("delete"),
                Key::V => Some("select"),
                Key::F => Some(if shift { "section" } else { "frame" }),
                Key::R => Some("rectangle"),
                Key::O => Some("ellipse"),
                Key::L => Some("line"),
                Key::P => Some(if shift { "pencil" } else { "pen" }),
                Key::T => Some("text"),
                Key::H => Some("hand"),
                Key::C => Some("comment"),
                Key::A if shift => Some("auto-layout"),
                Key::S if shift => Some("section"),
                Key::D1 => Some("fit"),
                Key::D2 => Some("fit-selection"),
                Key::K => Some("actions"),
                _ => None,
            }
        };
        if let Some(action) = action {
            this.emit(CanvasEvent::Action(action));
            e.set_handled(true);
            return;
        }
        let step = if shift { 10.0 } else { 1.0 };
        let delta = match e.key {
            Key::Left => Some((-step, 0.0)),
            Key::Right => Some((step, 0.0)),
            Key::Up => Some((0.0, -step)),
            Key::Down => Some((0.0, step)),
            _ => None,
        };
        if let Some((x, y)) = delta {
            this.editor.borrow_mut().move_selection(x, y);
            this.emit(CanvasEvent::Document);
            this.invalidate_visual();
            e.set_handled(true);
        }
    }
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);
        if e.key == Key::Space {
            this.space.set(false);
            e.set_handled(true);
        }
    }
}
fn normalize_drawn_vector(editor: &mut Editor, id: NodeId) {
    let Some(node) = editor.document.nodes.get_mut(&id) else {
        return;
    };
    let Some(bounds) = node.path.bounds() else {
        return;
    };
    // Freehand strokes and pen vertices may extend left/up from their first
    // point. Normalize once when committing so handles and spatial hits cover
    // the complete vector without moving the artwork in document space.
    node.x += bounds.x;
    node.y += bounds.y;
    node.width = bounds.width.max(1.0);
    node.height = bounds.height.max(1.0);
    for segment in &mut node.path.segments {
        let translate = |point: &mut Point| {
            point.x -= bounds.x;
            point.y -= bounds.y;
        };
        match segment {
            PathSegment::MoveTo { point } | PathSegment::LineTo { point } => translate(point),
            PathSegment::QuadraticTo { control, point } => {
                translate(control);
                translate(point);
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                translate(control1);
                translate(control2);
                translate(point);
            }
            PathSegment::Close => {}
        }
    }
    editor.refresh();
}
fn ui(p: Point) -> UiPoint {
    UiPoint::new(p.x, p.y)
}
fn rect(r: Rect) -> UiRect {
    UiRect::new(r.x, r.y, r.width, r.height)
}
fn pen(color: u32, width: f64) -> Rc<dyn IPen> {
    Rc::new(ImmutablePen::from_uint32(color, width))
}
fn label(ctx: &mut DrawingContext, text: &str, point: Point, size: f64, color: &str) {
    let text = FormattedText::new(
        text,
        CultureInfo::invariant_culture(),
        FlowDirection::LeftToRight,
        Typeface::from_name("fonts:Inter#Inter"),
        size,
        Some(brush(color)),
    );
    ctx.draw_text(&text, ui(point));
}
impl VisualImpl for DesignCanvas {
    fn render(this: &Self, ctx: &mut DrawingContext) {
        let bounds = this.bounds();
        let mut v = this.viewport.get();
        v.width = bounds.width;
        v.height = bounds.height;
        if v.width <= 0.0 || v.height <= 0.0 {
            return;
        }
        let editor = this.editor.borrow();
        let doc = &editor.document;
        let roots: Vec<NodeId> = if let Some(root) = this.presentation.get() {
            vec![root]
        } else {
            doc.page(editor.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        };
        if !this.initialized.replace(true) {
            if let Some(bounds) = roots
                .iter()
                .filter_map(|&id| doc.world_bounds(id))
                .reduce(|a, b| a.union(b))
            {
                v.fit(bounds, 65.0);
                if this.presentation.get().is_none() {
                    v.pan.y -= 22.0;
                }
            }
        }
        this.viewport.set(v);
        this.frames.set(this.frames.get() + 1);
        let page_color = doc
            .page(editor.active_page)
            .map(|p| p.background)
            .unwrap_or(Color::rgb(229, 229, 229));
        let page_hex = format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            page_color.a, page_color.r, page_color.g, page_color.b
        );
        let background = brush(if this.presentation.get().is_some() {
            "#181818"
        } else if this.dark.get() {
            "#252525"
        } else {
            &page_hex
        });
        ctx.draw_rectangle(
            Some(&background),
            None,
            UiRect::new(0.0, 0.0, v.width, v.height),
            0.0,
            0.0,
            &BoxShadows::default(),
        );
        if this.show_grid.get() && this.presentation.get().is_none() {
            let step = (32.0 * v.zoom).max(8.0);
            let grid_pen = pen(
                if this.dark.get() {
                    0xFF333333
                } else {
                    0xFFD5D5D5
                },
                0.5,
            );
            let mut x = v.pan.x.rem_euclid(step);
            while x < v.width {
                ctx.draw_line(&grid_pen, UiPoint::new(x, 0.0), UiPoint::new(x, v.height));
                x += step;
            }
            let mut y = v.pan.y.rem_euclid(step);
            while y < v.height {
                ctx.draw_line(&grid_pen, UiPoint::new(0.0, y), UiPoint::new(v.width, y));
                y += step;
            }
        }
        let camera = ctx.push_transform(
            Matrix::create_scale(v.zoom, v.zoom) * Matrix::create_translation(v.pan.x, v.pan.y),
        );
        this.painter
            .borrow_mut()
            .render(ctx, doc, &roots, v.visible_rect());
        ctx.pop(camera);
        if let Some(current) = this.presentation.get() {
            for &(id, position) in this.overlays.borrow().iter() {
                if let (Some(current_bounds), Some(overlay_bounds)) =
                    (doc.world_bounds(current), doc.world_bounds(id))
                {
                    let dim = brush("#66000000");
                    ctx.draw_rectangle(
                        Some(&dim),
                        None,
                        UiRect::new(0.0, 0.0, v.width, v.height),
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                    let dx = current_bounds.x + position.x - overlay_bounds.x;
                    let dy = current_bounds.y + position.y - overlay_bounds.y;
                    let transform = ctx.push_transform(
                        Matrix::create_translation(dx, dy)
                            * Matrix::create_scale(v.zoom, v.zoom)
                            * Matrix::create_translation(v.pan.x, v.pan.y),
                    );
                    let visible = v.visible_rect();
                    this.painter.borrow_mut().render(
                        ctx,
                        doc,
                        &[id],
                        Rect::new(
                            visible.x - dx,
                            visible.y - dy,
                            visible.width,
                            visible.height,
                        ),
                    );
                    ctx.pop(transform);
                }
            }
            label(
                ctx,
                "Esc to return to editor",
                Point::new(20.0, v.height - 30.0),
                11.0,
                "#AAAAAA",
            );
            return;
        }
        let blue = pen(0xFF0D99FF, 1.0);
        let (vertical, horizontal) = this.snap_guides.get();
        if vertical.is_some() || horizontal.is_some() {
            let guide = pen(0xFFDD498B, 1.0);
            if let Some(x) = vertical {
                let x = v.world_to_screen(Point::new(x, 0.0)).x;
                ctx.draw_line(&guide, UiPoint::new(x, 0.0), UiPoint::new(x, v.height));
            }
            if let Some(y) = horizontal {
                let y = v.world_to_screen(Point::new(0.0, y)).y;
                ctx.draw_line(&guide, UiPoint::new(0.0, y), UiPoint::new(v.width, y));
            }
        }
        for &id in &roots {
            if let Some(n) = doc.nodes.get(&id) {
                if n.kind.is_container() && n.visible {
                    if let Some(bounds) = doc.world_bounds(id) {
                        let p = v.world_to_screen(Point::new(bounds.x, bounds.y));
                        if p.x > -400.0 && p.x < v.width && p.y > -30.0 && p.y < v.height {
                            label(
                                ctx,
                                &n.name,
                                Point::new(p.x, p.y - 20.0),
                                11.0,
                                if matches!(n.kind, NodeKind::Component | NodeKind::Instance) {
                                    "#9747FF"
                                } else if this.dark.get() {
                                    "#999999"
                                } else {
                                    "#777777"
                                },
                            );
                        }
                    }
                }
            }
        }
        if let Some(id) = this.hover.get() {
            if !editor.selection.contains(&id) {
                if let Some(b) = doc.world_bounds(id) {
                    let a = v.world_to_screen(Point::new(b.x, b.y));
                    ctx.draw_rectangle_outline(
                        &blue,
                        UiRect::new(a.x, a.y, b.width * v.zoom, b.height * v.zoom),
                        0.0,
                    );
                }
            }
        }
        let white = brush("#FFFFFF");
        for &id in &editor.selection {
            let Some(n) = doc.nodes.get(&id) else {
                continue;
            };
            let Some(t) = doc.world_transform(id) else {
                continue;
            };
            let corners = n
                .local_bounds()
                .corners()
                .map(|p| v.world_to_screen(t.apply(p)));
            for i in 0..4 {
                ctx.draw_line(&blue, ui(corners[i]), ui(corners[(i + 1) % 4]));
            }
            if editor.selection.len() == 1 {
                for p in corners {
                    ctx.draw_rectangle(
                        Some(&white),
                        Some(&blue),
                        UiRect::new(p.x - 3.0, p.y - 3.0, 6.0, 6.0),
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                }
                let top = (corners[0] + corners[1]) * 0.5;
                let center = (corners[0] + corners[2]) * 0.5;
                let direction = top - center;
                let length = direction.distance(Point::new(0.0, 0.0)).max(0.001);
                let rotate = top + direction * (24.0 / length);
                ctx.draw_line(&blue, ui(top), ui(rotate));
                ctx.draw_ellipse_at(Some(&white), Some(&blue), ui(rotate), 3.0, 3.0);
                if let Some(b) = doc.world_bounds(id) {
                    let p = v.world_to_screen(Point::new(b.x + b.width / 2.0, b.bottom()));
                    let value = format!("{} × {}", n.width.round(), n.height.round());
                    let width = value.len() as f64 * 6.2 + 16.0;
                    let fill = brush("#0D99FF");
                    ctx.draw_rectangle(
                        Some(&fill),
                        None,
                        UiRect::new(p.x - width / 2.0, p.y + 9.0, width, 22.0),
                        4.0,
                        4.0,
                        &BoxShadows::default(),
                    );
                    label(
                        ctx,
                        &value,
                        Point::new(p.x - width / 2.0 + 8.0, p.y + 13.0),
                        10.0,
                        "#FFFFFF",
                    );
                }
            }
        }
        if let Gesture::Marquee { start, current, .. } = &*this.gesture.borrow() {
            let a = v.world_to_screen(*start);
            let b = v.world_to_screen(*current);
            let fill = brush("#200D99FF");
            ctx.draw_rectangle(
                Some(&fill),
                Some(&blue),
                rect(Rect::from_points(a, b)),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
        }
        for comment in &doc.comments {
            if !comment.resolved && doc.comment_page(comment) == Some(editor.active_page) {
                let p = v.world_to_screen(comment.position);
                let fill = brush("#FFCD29");
                ctx.draw_ellipse_at(Some(&fill), None, ui(p), 10.0, 10.0);
                label(ctx, "C", Point::new(p.x - 3.5, p.y - 6.0), 10.0, "#2A2200");
            }
        }
        if this.show_rulers.get() {
            let fill = brush("#F5F5F5");
            ctx.draw_rectangle(
                Some(&fill),
                None,
                UiRect::new(0.0, 0.0, v.width, 20.0),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            let step = 100.0 * if v.zoom < 0.3 { 5.0 } else { 1.0 };
            let mut x = (v.visible_rect().x / step).floor() * step;
            let tick = pen(0xFFBBBBBB, 1.0);
            while x < v.visible_rect().right() {
                let px = v.world_to_screen(Point::new(x, 0.0)).x;
                ctx.draw_line(&tick, UiPoint::new(px, 14.0), UiPoint::new(px, 20.0));
                label(
                    ctx,
                    &format!("{x:.0}"),
                    Point::new(px + 3.0, 3.0),
                    9.0,
                    "#777777",
                );
                x += step;
            }
        }
    }
}
