//! The XAML shell binds real FerroUI controls to the document command layer.
use crate::{
    canvas::{CanvasEvent, DesignCanvas, Tool},
    icons, platform,
};
use ferrodesign_core::{
    self as core, Align, Arrange, AutoLayout, BooleanOperation, Color, Distribution, Editor,
    Effect, Fill, GradientStop, LayoutAxis, Node, NodeId, NodeKind, Padding, Paint, Point, Stroke,
    TextAlign,
};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{ObjectType, Ref, Thickness};
use ferroui_controls::{
    Border, Button, CheckBox, ColumnDefinitions, ComboBox, Control, Grid, StackPanel, TextBlock,
    TextBox, UserControl,
};
use ferroui_markup_xaml::FerroXamlLoader;
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

pub struct EditorShell {
    pub root: Ref<UserControl>,
    pub canvas: Ref<DesignCanvas>,
    pub editor: Rc<RefCell<Editor>>,
    updating: Cell<bool>,
    expanded: RefCell<BTreeSet<NodeId>>,
    assets: Cell<bool>,
    inspector_tab: Cell<u8>,
    prototype_targets: RefCell<Vec<NodeId>>,
    prototype_history: RefCell<Vec<NodeId>>,
    prototype: RefCell<core::PrototypePlayer>,
}
impl EditorShell {
    pub fn new() -> Rc<Self> {
        let uri =
            Uri::new("ferres://FerroDesign/ui/Editor.xaml", UriKind::Absolute).expect("editor URI");
        let value = FerroXamlLoader::load(&uri, None).expect("compiled editor XAML");
        let root = from_markup_value::<Ref<UserControl>>(&Some(value))
            .expect("EditorRoot is a UserControl");
        let mut editor = Editor::demo();
        let saved = platform::load_document();
        if !saved.is_empty() {
            let _ = editor.load_json(&saved);
        }
        let expanded = editor
            .document
            .page(editor.active_page)
            .and_then(|p| p.children.first())
            .copied()
            .into_iter()
            .collect();
        let editor = Rc::new(RefCell::new(editor));
        let canvas = DesignCanvas::new(editor.clone());
        root.get_control::<Border>("CanvasHost")
            .set_child(canvas.clone());
        let this = Rc::new(Self {
            root,
            canvas,
            editor,
            updating: Cell::new(false),
            expanded: RefCell::new(expanded),
            assets: Cell::new(false),
            inspector_tab: Cell::new(0),
            prototype_targets: RefCell::new(Vec::new()),
            prototype_history: RefCell::new(Vec::new()),
            prototype: RefCell::new(core::PrototypePlayer::default()),
        });
        this.wire();
        this.refresh();
        this
    }
    fn control<T: ObjectType>(&self, name: &str) -> Ref<T> {
        self.root.get_control::<T>(name)
    }
    fn button(&self, name: &str) -> Ref<Button> {
        self.control(name)
    }
    fn field(&self, name: &str) -> Ref<TextBox> {
        self.control(name)
    }
    fn text(&self, name: &str, text: &str) {
        self.control::<TextBlock>(name).set_text(Some(text));
    }
    fn visible(&self, name: &str, value: bool) {
        self.control::<Control>(name).set_is_visible(value);
    }
    fn hook(self: &Rc<Self>, name: &str, action: &'static str) {
        let weak = Rc::downgrade(self);
        self.button(name).click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                s.command(action);
            }
        });
    }
    fn icon(&self, name: &str, glyph: &str) {
        self.button(name)
            .set_content(Some(Control::boxed(icons::icon(glyph, "#303030", 17.0))));
    }
    fn wire(self: &Rc<Self>) {
        for (name, icon) in [
            ("MenuButton", "logo"),
            ("FileRailButton", "file"),
            ("AssetsRailButton", "assets"),
            ("ToolsRailButton", "tools"),
            ("VariablesRailButton", "variables"),
            ("CommentsRailButton", "comment"),
            ("HistoryRailButton", "history"),
            ("ThemeButton", "theme"),
            ("PresentButton", "play"),
            ("VisibleButton", "eye"),
            ("LockedButton", "lock"),
            ("SelectTool", "pointer"),
            ("FrameTool", "frame"),
            ("RectTool", "rect"),
            ("EllipseTool", "ellipse"),
            ("PenTool", "pen"),
            ("TextTool", "text"),
            ("HandTool", "hand"),
            ("CommentTool", "comment"),
            ("ActionsButton", "actions"),
            ("AlignLeftButton", "align-left"),
            ("AlignCenterButton", "align-center"),
            ("AlignRightButton", "align-right"),
            ("AlignTopButton", "align-top"),
            ("AlignMiddleButton", "align-middle"),
            ("AlignBottomButton", "align-bottom"),
            ("CreateComponentButton", "component"),
        ] {
            self.icon(name, icon);
        }
        for (name, command) in [
            ("MenuButton", "menu"),
            ("FileActionsButton", "menu"),
            ("FileRailButton", "layers"),
            ("AssetsRailButton", "assets"),
            ("ToolsRailButton", "actions"),
            ("VariablesRailButton", "variables"),
            ("CommentsRailButton", "comments"),
            ("HistoryRailButton", "history"),
            ("ThemeButton", "theme"),
            ("PresentButton", "present"),
            ("ShareButton", "share"),
            ("SelectTool", "select"),
            ("FrameTool", "frame"),
            ("RectTool", "rectangle"),
            ("EllipseTool", "ellipse"),
            ("PenTool", "pen"),
            ("TextTool", "text"),
            ("HandTool", "hand"),
            ("CommentTool", "comment"),
            ("ActionsButton", "actions"),
            ("DesignTab", "design-tab"),
            ("PrototypeTab", "prototype-tab"),
            ("InspectTab", "inspect-tab"),
            ("AddPageButton", "add-page"),
            ("CollapseLayersButton", "collapse-layers"),
            ("VisibleButton", "toggle-visible"),
            ("LockedButton", "toggle-lock"),
            ("FlipButton", "flip"),
            ("AlignLeftButton", "align-left"),
            ("AlignCenterButton", "align-center"),
            ("AlignRightButton", "align-right"),
            ("AlignTopButton", "align-top"),
            ("AlignMiddleButton", "align-middle"),
            ("AlignBottomButton", "align-bottom"),
            ("AddLayoutButton", "auto-layout"),
            ("LayoutRowButton", "layout-row"),
            ("LayoutColumnButton", "layout-column"),
            ("LayoutGridButton", "layout-grid"),
            ("AddFillButton", "add-fill"),
            ("FillSwatch", "palette"),
            ("GradientButton", "gradient"),
            ("AddStrokeButton", "add-stroke"),
            ("AddEffectButton", "add-effect"),
            ("EffectButton", "effects"),
            ("BoldButton", "bold"),
            ("TextLeftButton", "text-left"),
            ("TextCenterButton", "text-center"),
            ("TextRightButton", "text-right"),
            ("CreateComponentButton", "component"),
            ("CreateInstanceButton", "instance"),
            ("DetachButton", "detach"),
            ("ConnectButton", "connect"),
            ("StartFrameButton", "flow-start"),
            ("PresentFlowButton", "present"),
            ("CopyCodeButton", "copy-css"),
            ("ExportXamlButton", "export-xaml"),
            ("ExportSvgButton", "export-svg"),
            ("ExportPngButton", "export-png"),
            ("ZoomOutButton", "zoom-out"),
            ("ZoomValueButton", "fit"),
            ("ZoomInButton", "zoom-in"),
            ("CloseModalButton", "close-modal"),
        ] {
            self.hook(name, command);
        }
        let weak = Rc::downgrade(self);
        self.canvas.on_change(move |event| {
            if let Some(s) = weak.upgrade() {
                s.canvas_event(event)
            }
        });
        let weak = Rc::downgrade(self);
        self.field("LayerSearch").text_changed(move |_, _| {
            if let Some(s) = weak.upgrade() {
                if !s.updating.get() {
                    s.refresh_layers();
                }
            }
        });
        for name in [
            "NameField",
            "XField",
            "YField",
            "RotationField",
            "WidthField",
            "HeightField",
            "GapField",
            "PaddingField",
            "OpacityField",
            "RadiusField",
            "FillField",
            "StrokeField",
            "StrokeWidthField",
            "TextField",
            "FontFamilyField",
            "FontSizeField",
        ] {
            let weak = Rc::downgrade(self);
            self.field(name)
                .add_handler(InputElement::lost_focus_event(), move |_, _| {
                    if let Some(s) = weak.upgrade() {
                        s.commit_field(name);
                    }
                });
            let weak = Rc::downgrade(self);
            self.field(name)
                .add_handler(InputElement::key_down_event(), move |_, e| {
                    if e.key == Key::Enter && name != "TextField" {
                        if let Some(s) = weak.upgrade() {
                            s.commit_field(name);
                            s.canvas.focus();
                        }
                        e.set_handled(true);
                    }
                });
        }
        let weak = Rc::downgrade(self);
        self.control::<CheckBox>("ClipCheck")
            .is_checked_changed(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    if !s.updating.get() {
                        let value = s.control::<CheckBox>("ClipCheck").is_checked() == Some(true);
                        s.modify("Clip content", move |n| n.clip_content = value);
                        s.changed();
                    }
                }
            });
        let palette = self.control::<StackPanel>("ColorPalette");
        for row in [
            [
                "#FFFFFF", "#E5E5E5", "#6B7280", "#202020", "#000000", "#FF6154",
            ],
            [
                "#FFB547", "#FFE97A", "#9EDF9C", "#3BBD93", "#66B7FF", "#7B61FF",
            ],
            [
                "#F5EFFF", "#E0ECFF", "#E7F7EE", "#FFF4D7", "#FFE7EC", "#F09CDC",
            ],
        ] {
            let panel = StackPanel::new();
            panel.set_orientation(Orientation::Horizontal);
            panel.set_spacing(7.0);
            for color in row {
                let button = Button::new();
                button.set_width(30.0);
                button.set_height(28.0);
                button.set_background(Some(icons::brush(color)));
                let weak = Rc::downgrade(self);
                button.click(move |_, _| {
                    if let Some(s) = weak.upgrade() {
                        s.editor
                            .borrow_mut()
                            .set_selection_fill(Color::from_hex(color).unwrap());
                        s.changed();
                    }
                });
                panel.children().add(button);
            }
            palette.children().add(panel);
        }
    }
    fn canvas_event(self: &Rc<Self>, event: CanvasEvent) {
        match event {
            CanvasEvent::Document => self.changed(),
            CanvasEvent::Selection => self.refresh(),
            CanvasEvent::Camera => self.sync_zoom(),
            CanvasEvent::Tool => self.sync_tools(),
            CanvasEvent::EditText(id) => {
                self.editor.borrow_mut().selection = vec![id];
                self.inspector_tab.set(0);
                self.refresh();
                self.field("TextField").focus();
                self.field("TextField").select_all();
            }
            CanvasEvent::Comment(position) => self.comment_dialog(position),
            CanvasEvent::ContextMenu => self.action_dialog(
                "Layer actions",
                "Edit, arrange, and organize the selected layers.",
                LAYER_ACTIONS,
            ),
            CanvasEvent::PrototypeClick(id) => self.prototype_click(id),
            CanvasEvent::ExitPresentation => self.set_presentation(None),
            CanvasEvent::Action(command) => {
                self.command(command);
            }
        }
    }
    pub fn document_json(&self) -> String {
        self.editor.borrow().to_json().unwrap_or_default()
    }
    pub fn state_json(&self) -> String {
        let editor = self.editor.borrow();
        let v = self.canvas.viewport.get();
        let stats = self.canvas.render_stats();
        let selection_bounds = editor
            .selection
            .iter()
            .filter_map(|&id| editor.document.world_bounds(id))
            .reduce(|a, b| a.union(b));
        let canvas = self.canvas.bounds();
        json!({
            "revision":editor.revision,"documentRevision":editor.document.revision,"nodeCount":editor.document.nodes.len(),
            "selectionCount":editor.selection.len(),"selection":editor.selection,"selectionBounds":selection_bounds,
            "tool":self.canvas.tool.get().name(),"zoom":v.zoom,"pan":{"x":v.pan.x,"y":v.pan.y},
            "canvas":{"x":canvas.x,"y":canvas.y,"width":canvas.width,"height":canvas.height},
            "renderer":"FerroUI Vello","pageCount":editor.document.pages.len(),"activePage":editor.active_page,
            "renderedFrames":self.canvas.frames.get(),"visibleNodes":stats.visible_nodes,"culledNodes":stats.culled_nodes,
            "cachedNodes":stats.cached_nodes,"missingImages":stats.missing_images,"unsupportedEffects":stats.unsupported_effects,
            "canUndo":editor.undo_label().is_some(),"canRedo":editor.redo_label().is_some(),"presentation":self.canvas.presentation.get(),
        }).to_string()
    }
    pub fn import_document(self: &Rc<Self>, text: &str, _name: &str) -> bool {
        self.canvas.cancel();
        let result = self.editor.borrow_mut().load_json(text);
        match result {
            Ok(_) => {
                self.expanded.borrow_mut().clear();
                self.canvas.reset_fit();
                self.canvas.invalidate_scene();
                self.changed();
                self.close_modal();
                true
            }
            Err(error) => {
                self.toast(&format!("Could not open design: {error}"));
                false
            }
        }
    }
    fn changed(self: &Rc<Self>) {
        let saved = platform::save_document(&self.document_json());
        self.text(
            "DocumentStatus",
            if saved {
                "All changes saved locally"
            } else {
                "Storage full · Export to save your work"
            },
        );
        self.refresh();
        self.canvas.invalidate_visual();
    }
    fn sync_zoom(&self) {
        self.button("ZoomValueButton")
            .set_content(Some(Rc::new(format!(
                "{}%",
                (self.canvas.viewport.get().zoom * 100.0).round()
            ))));
    }
    fn sync_tools(&self) {
        let active = self.canvas.tool.get();
        for (name, tool) in [
            ("SelectTool", Tool::Select),
            ("FrameTool", Tool::Frame),
            ("RectTool", Tool::Rectangle),
            ("EllipseTool", Tool::Ellipse),
            ("PenTool", Tool::Pen),
            ("TextTool", Tool::Text),
            ("HandTool", Tool::Hand),
            ("CommentTool", Tool::Comment),
        ] {
            self.button(name).classes().set("active", active == tool);
        }
        self.sync_zoom();
    }
    fn refresh(self: &Rc<Self>) {
        if self.updating.replace(true) {
            return;
        }
        self.sync_tools();
        self.refresh_pages();
        self.refresh_layers();
        self.refresh_inspector();
        self.updating.set(false);
    }
    fn refresh_pages(self: &Rc<Self>) {
        let host = self.control::<StackPanel>("PagesHost");
        host.children().clear();
        let editor = self.editor.borrow();
        self.text("FileTitle", &editor.document.name);
        for page in &editor.document.pages {
            let id = page.id;
            let button = Button::new();
            button.classes().add("layer");
            button.classes().set("active", id == editor.active_page);
            button.set_content(Some(Rc::new(format!(
                "{}  {}",
                if id == editor.active_page { "✓" } else { " " },
                page.name
            ))));
            let weak = Rc::downgrade(self);
            button.click(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    {
                        let mut editor = s.editor.borrow_mut();
                        editor.active_page = id;
                        editor.selection.clear();
                        editor.refresh();
                    }
                    s.canvas.reset_fit();
                    s.refresh();
                    s.canvas.focus();
                }
            });
            host.children().add(button);
        }
    }
    fn refresh_layers(self: &Rc<Self>) {
        let host = self.control::<StackPanel>("LayersHost");
        host.children().clear();
        self.text(
            "LeftTitle",
            if self.assets.get() {
                "Local components"
            } else {
                "Layers"
            },
        );
        self.button("FileRailButton")
            .classes()
            .set("active", !self.assets.get());
        self.button("AssetsRailButton")
            .classes()
            .set("active", self.assets.get());
        let filter = self
            .field("LayerSearch")
            .text()
            .unwrap_or_default()
            .to_lowercase();
        let editor = self.editor.borrow();
        let roots = if self.assets.get() {
            editor
                .document
                .nodes
                .values()
                .filter(|n| n.kind == NodeKind::Component)
                .map(|n| n.id)
                .collect::<Vec<_>>()
        } else {
            editor
                .document
                .page(editor.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        };
        for &id in roots.iter().rev() {
            self.layer_row(&host, &editor, id, 0, &filter);
        }
        if roots.is_empty() {
            let text = TextBlock::new();
            text.set_text(Some(if self.assets.get() {
                "Create a component to reuse it here."
            } else {
                "Draw a frame or shape to get started."
            }));
            text.set_text_wrapping(TextWrapping::Wrap);
            text.set_margin(Thickness::uniform(12.0));
            text.set_foreground(Some(icons::brush("#888888")));
            host.children().add(text);
        }
    }
    fn layer_row(
        self: &Rc<Self>,
        host: &Ref<StackPanel>,
        editor: &Editor,
        id: NodeId,
        depth: usize,
        filter: &str,
    ) {
        let Some(node) = editor.document.nodes.get(&id) else {
            return;
        };
        let matches = filter.is_empty() || node.name.to_lowercase().contains(filter);
        let descendant_matches = !filter.is_empty()
            && editor.document.descendants(id).iter().any(|id| {
                editor
                    .document
                    .nodes
                    .get(id)
                    .is_some_and(|n| n.name.to_lowercase().contains(filter))
            });
        if !matches && !descendant_matches {
            return;
        }
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("20,*,24").unwrap());
        grid.set_margin(Thickness::new((depth.min(10) * 12) as f64, 0.0, 0.0, 0.0));
        let expanded = self.expanded.borrow().contains(&id) || !filter.is_empty();
        if !node.children.is_empty() {
            let collapse = Button::new();
            collapse.set_padding(Thickness::uniform(0.0));
            collapse.set_content(Some(Rc::new(if expanded {
                "⌄".to_string()
            } else {
                "›".to_string()
            })));
            let weak = Rc::downgrade(self);
            collapse.click(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    {
                        let mut expanded = s.expanded.borrow_mut();
                        if !expanded.remove(&id) {
                            expanded.insert(id);
                        }
                    }
                    s.refresh_layers();
                }
            });
            grid.children().add(collapse);
        }
        let button = Button::new();
        button.classes().add("layer");
        button
            .classes()
            .set("active", editor.selection.contains(&id));
        if !node.visible {
            button.set_opacity(0.4);
        }
        let content = StackPanel::new();
        content.set_orientation(Orientation::Horizontal);
        content.set_spacing(7.0);
        let glyph = match node.kind {
            NodeKind::Frame => "frame",
            NodeKind::Group => "group",
            NodeKind::Text => "text",
            NodeKind::Ellipse => "ellipse",
            NodeKind::Vector => "pen",
            NodeKind::Line => "line",
            NodeKind::Polygon => "polygon",
            NodeKind::Star => "star",
            NodeKind::Component | NodeKind::Instance => "component",
            NodeKind::Section => "section",
            _ => "rect",
        };
        content.children().add(icons::icon(
            glyph,
            if matches!(node.kind, NodeKind::Component | NodeKind::Instance) {
                "#9747FF"
            } else {
                "#777777"
            },
            12.0,
        ));
        let name = TextBlock::new();
        name.set_text(Some(&node.name));
        name.set_vertical_alignment(VerticalAlignment::Center);
        name.set_font_size(11.0);
        content.children().add(name);
        button.set_content(Some(Control::boxed(content)));
        Grid::set_column(&button, 1);
        let weak = Rc::downgrade(self);
        let assets = self.assets.get();
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                if assets {
                    let point = s
                        .canvas
                        .viewport
                        .get()
                        .screen_to_world(Point::new(100.0, 100.0));
                    s.editor.borrow_mut().create_instance(id, point.x, point.y);
                    s.changed();
                } else {
                    s.editor.borrow_mut().selection = vec![id];
                    s.refresh();
                    s.canvas.invalidate_visual();
                }
                s.canvas.focus();
            }
        });
        grid.children().add(button);
        let eye = Button::new();
        eye.set_padding(Thickness::uniform(5.0));
        eye.set_content(Some(Control::boxed(icons::icon(
            if node.locked { "lock" } else { "eye" },
            "#999999",
            11.0,
        ))));
        Grid::set_column(&eye, 2);
        let weak = Rc::downgrade(self);
        eye.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                s.editor.borrow_mut().toggle_visibility(id);
                s.changed();
            }
        });
        grid.children().add(eye);
        host.children().add(grid);
        if expanded && !self.assets.get() {
            for &child in node.children.iter().rev() {
                self.layer_row(host, editor, child, depth + 1, filter);
            }
        }
    }
    fn refresh_inspector(&self) {
        let editor = self.editor.borrow();
        let node = editor
            .selection
            .first()
            .and_then(|id| editor.document.nodes.get(id));
        self.visible("DesignInspector", self.inspector_tab.get() == 0);
        self.visible("PrototypeInspector", self.inspector_tab.get() == 1);
        self.visible("InspectInspector", self.inspector_tab.get() == 2);
        for (name, index) in [("DesignTab", 0), ("PrototypeTab", 1), ("InspectTab", 2)] {
            self.button(name)
                .classes()
                .set("active", self.inspector_tab.get() == index);
        }
        self.text(
            "SelectionTitle",
            &if editor.selection.len() > 1 {
                format!("{} layers", editor.selection.len())
            } else {
                node.map(|n| n.kind.label().to_string())
                    .unwrap_or_else(|| "Page".into())
            },
        );
        let set = |name: &str, value: String| self.field(name).set_text(Some(&value));
        set(
            "NameField",
            node.map(|n| n.name.clone()).unwrap_or_else(|| {
                editor
                    .document
                    .page(editor.active_page)
                    .map(|p| p.name.clone())
                    .unwrap_or_default()
            }),
        );
        for (name, value) in [
            ("XField", node.map(|n| n.x)),
            ("YField", node.map(|n| n.y)),
            ("WidthField", node.map(|n| n.width)),
            ("HeightField", node.map(|n| n.height)),
            ("RotationField", node.map(|n| n.rotation)),
            ("RadiusField", node.map(|n| n.corner_radius)),
            ("OpacityField", node.map(|n| n.opacity * 100.0)),
            ("FontSizeField", node.map(|n| n.font_size)),
            (
                "StrokeWidthField",
                node.map(|n| n.strokes.first().map_or(0.0, |s| s.width)),
            ),
            (
                "GapField",
                node.and_then(|n| n.layout.as_ref().map(|l| l.gap)),
            ),
            (
                "PaddingField",
                node.and_then(|n| n.layout.as_ref().map(|l| l.padding.left)),
            ),
        ] {
            set(name, value.map(number).unwrap_or_default());
        }
        let fill = node.map(|n| n.fill_color()).unwrap_or_else(|| {
            editor
                .document
                .page(editor.active_page)
                .map(|p| p.background)
                .unwrap_or(Color::WHITE)
        });
        set("FillField", fill.hex().trim_start_matches('#').to_string());
        self.button("FillSwatch")
            .set_background(Some(icons::brush(&fill.hex())));
        set(
            "StrokeField",
            node.and_then(|n| n.strokes.first())
                .map(|s| s.paint.representative_color().hex())
                .unwrap_or_else(|| "000000".into()),
        );
        set(
            "TextField",
            node.map(|n| n.text.clone()).unwrap_or_default(),
        );
        set(
            "FontFamilyField",
            node.map(|n| n.font_family.clone())
                .unwrap_or_else(|| "Inter".into()),
        );
        self.control::<CheckBox>("ClipCheck")
            .set_is_checked(Some(node.is_some_and(|n| n.clip_content)));
        self.visible("LayoutOptions", node.is_some_and(|n| n.layout.is_some()));
        self.visible(
            "TextOptions",
            node.is_some_and(|n| n.kind == NodeKind::Text),
        );
        self.button("LockedButton")
            .classes()
            .set("active", node.is_some_and(|n| n.locked));
        self.button("BoldButton")
            .classes()
            .set("active", node.is_some_and(|n| n.font_weight >= 600));
        self.button("EffectButton").set_content(Some(Rc::new(
            node.map(|n| {
                format!(
                    "{} effect{}",
                    n.effects.len(),
                    if n.effects.len() == 1 { "" } else { "s" }
                )
            })
            .unwrap_or_else(|| "Add an effect".into()),
        )));
        set(
            "CodeField",
            node.map(css)
                .unwrap_or_else(|| "Select a layer to inspect its properties.".into()),
        );
        if self.inspector_tab.get() == 1 {
            let target = self.control::<ComboBox>("PrototypeTarget");
            let previous = target.selected_index();
            target.items().clear();
            let mut ids = Vec::new();
            for n in editor.document.nodes.values().filter(|n| {
                n.kind == NodeKind::Frame
                    && editor.document.page_of(n.id) == Some(editor.active_page)
            }) {
                target.items().add(Some(Rc::new(n.name.clone())));
                ids.push(n.id);
            }
            target.set_selected_index(if previous >= 0 && (previous as usize) < ids.len() {
                previous
            } else if ids.is_empty() {
                -1
            } else {
                0
            });
            *self.prototype_targets.borrow_mut() = ids;
            self.text(
                "PrototypeSummary",
                &node
                    .map(|n| {
                        format!(
                            "{} interaction(s). Click the play button to test your flow.",
                            n.interactions.len()
                        )
                    })
                    .unwrap_or_default(),
            );
        }
    }
    fn modify(self: &Rc<Self>, label: &str, mut operation: impl FnMut(&mut Node)) {
        let ids = self.editor.borrow().selection.clone();
        self.editor.borrow_mut().transact(label, |doc| {
            for id in ids {
                if let Some(n) = doc.nodes.get_mut(&id) {
                    operation(n);
                }
            }
            doc.layout_all();
        });
    }
    fn commit_field(self: &Rc<Self>, name: &str) {
        if self.updating.get() {
            return;
        }
        let text = self.field(name).text().unwrap_or_default();
        let id = self.editor.borrow().selection.first().copied();
        if id.is_none() {
            if name == "NameField" || name == "FillField" {
                let mut editor = self.editor.borrow_mut();
                let page = editor.active_page;
                editor.transact("Edit page", |doc| {
                    if let Some(page) = doc.page_mut(page) {
                        if name == "NameField" {
                            page.name = text.clone();
                        } else if let Some(color) = Color::from_hex(&text) {
                            page.background = color;
                        }
                    }
                });
                drop(editor);
                self.changed();
            }
            return;
        }
        let number = text
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && v.abs() <= 1.0e7);
        match name {
            "NameField" => self.modify("Rename layer", |n| n.name = text.clone()),
            "TextField" => self.modify("Edit text", |n| n.text = text.clone()),
            "FontFamilyField" => self.modify("Font family", |n| n.font_family = text.clone()),
            "FillField" => {
                if let Some(color) = Color::from_hex(&text) {
                    self.editor.borrow_mut().set_selection_fill(color);
                } else {
                    self.toast("Use a hex color, for example 7B61FF.");
                    self.refresh();
                    return;
                }
            }
            "StrokeField" => {
                if let Some(color) = Color::from_hex(&text) {
                    self.modify("Stroke color", |n| {
                        if n.strokes.is_empty() {
                            n.strokes.push(Stroke::new(color, 1.0));
                        } else {
                            n.strokes[0].paint = Paint::Solid(color);
                        }
                    });
                } else {
                    self.refresh();
                    return;
                }
            }
            _ => {
                let Some(value) = number else {
                    self.refresh();
                    return;
                };
                if matches!(name, "WidthField" | "HeightField") {
                    let mut editor = self.editor.borrow_mut();
                    editor.begin_transaction("Resize layers");
                    for id in editor.selection.clone() {
                        if let Some(n) = editor.document.nodes.get(&id) {
                            let (width, height) = if name == "WidthField" {
                                (value.max(1.0), n.height)
                            } else {
                                (n.width, value.max(1.0))
                            };
                            editor.resize_node(id, width, height);
                        }
                    }
                    editor.commit_transaction();
                } else {
                    self.modify("Edit property", |n| match name {
                        "XField" => n.x = value,
                        "YField" => n.y = value,
                        "RotationField" => n.rotation = value,
                        "OpacityField" => n.opacity = (value / 100.0).clamp(0.0, 1.0),
                        "RadiusField" => n.corner_radius = value.max(0.0),
                        "FontSizeField" => n.font_size = value.clamp(1.0, 4096.0),
                        "StrokeWidthField" => {
                            if n.strokes.is_empty() {
                                n.strokes.push(Stroke::new(Color::BLACK, value.max(0.0)));
                            } else {
                                n.strokes[0].width = value.max(0.0);
                            }
                        }
                        "GapField" => {
                            if let Some(layout) = &mut n.layout {
                                layout.gap = value.max(0.0);
                            }
                        }
                        "PaddingField" => {
                            if let Some(layout) = &mut n.layout {
                                layout.padding = Padding::all(value.max(0.0));
                            }
                        }
                        _ => {}
                    });
                }
            }
        }
        self.changed();
    }

    pub fn command(self: &Rc<Self>, command: &str) -> bool {
        self.visible("Toast", false);
        let tool = match command {
            "select" => Some(Tool::Select),
            "frame" => Some(Tool::Frame),
            "rectangle" => Some(Tool::Rectangle),
            "ellipse" => Some(Tool::Ellipse),
            "line" => Some(Tool::Line),
            "polygon" => Some(Tool::Polygon),
            "star" => Some(Tool::Star),
            "pen" => Some(Tool::Pen),
            "pencil" => Some(Tool::Pencil),
            "text" => Some(Tool::Text),
            "hand" => Some(Tool::Hand),
            "comment" => Some(Tool::Comment),
            "section" => Some(Tool::Section),
            _ => None,
        };
        if let Some(tool) = tool {
            self.close_modal();
            self.canvas.set_tool(tool);
            return true;
        }
        match command {
            "menu" => {
                self.action_dialog(
                    "FerroDesign",
                    "Design, prototype, and inspect your local files.",
                    FILE_ACTIONS,
                );
                return true;
            }
            "actions" => {
                self.action_dialog("Actions", "Choose a tool or run a command.", ACTIONS);
                return true;
            }
            "close-modal" => {
                self.close_modal();
                return true;
            }
            "layers" | "assets" => {
                self.assets.set(command == "assets");
                self.refresh_layers();
                return true;
            }
            "design-tab" | "prototype-tab" | "inspect-tab" => {
                self.inspector_tab.set(if command == "design-tab" {
                    0
                } else if command == "prototype-tab" {
                    1
                } else {
                    2
                });
                self.refresh();
                return true;
            }
            "palette" => {
                let panel = self.control::<StackPanel>("ColorPalette");
                panel.set_is_visible(!panel.is_visible());
                return true;
            }
            "zoom-in" => {
                self.canvas.zoom(1.25);
                return true;
            }
            "zoom-out" => {
                self.canvas.zoom(0.8);
                return true;
            }
            "fit" => {
                self.canvas.reset_fit();
                self.canvas.focus();
                return true;
            }
            "fit-selection" => {
                self.canvas.zoom_to_selection();
                self.canvas.focus();
                return true;
            }
            "theme" => {
                self.canvas.dark.set(!self.canvas.dark.get());
                self.canvas.invalidate_visual();
                return true;
            }
            "grid" => {
                self.canvas.show_grid.set(!self.canvas.show_grid.get());
                self.canvas.invalidate_visual();
                self.close_modal();
                return true;
            }
            "rulers" => {
                self.canvas.show_rulers.set(!self.canvas.show_rulers.get());
                self.canvas.invalidate_visual();
                self.close_modal();
                return true;
            }
            "snapping" => {
                self.canvas.snap.set(!self.canvas.snap.get());
                self.close_modal();
                return true;
            }
            "collapse-layers" => {
                let mut expanded = self.expanded.borrow_mut();
                if expanded.is_empty() {
                    expanded.extend(self.editor.borrow().document.nodes.keys().copied());
                } else {
                    expanded.clear();
                }
                drop(expanded);
                self.refresh_layers();
                return true;
            }
            "share" => {
                self.action_dialog("Share this design","Save a portable design file to share all editable layers, components, variables, and prototype links.",&[
                ("Download editable design","export-json"),("Export selected layers as SVG","export-svg"),("Export FerroUI XAML","export-xaml")]);
                return true;
            }
            "history" => {
                let editor = self.editor.borrow();
                let description = format!(
                    "{} document revisions. Next undo: {}. Next redo: {}.",
                    editor.revision,
                    editor.undo_label().unwrap_or("None"),
                    editor.redo_label().unwrap_or("None")
                );
                drop(editor);
                self.action_dialog(
                    "History",
                    &description,
                    &[("Undo", "undo"), ("Redo", "redo")],
                );
                return true;
            }
            "variables" => {
                self.variables_dialog();
                return true;
            }
            "comments" => {
                self.comments_dialog();
                return true;
            }
            "effects" => {
                self.effects_dialog();
                return true;
            }
            "advanced" => {
                self.advanced_dialog();
                return true;
            }
            "present" => {
                let editor = self.editor.borrow();
                let root = editor
                    .selection
                    .first()
                    .copied()
                    .or_else(|| {
                        editor
                            .document
                            .page(editor.active_page)
                            .and_then(|p| p.prototype_start)
                    })
                    .or_else(|| {
                        editor
                            .document
                            .page(editor.active_page)
                            .and_then(|p| p.children.first())
                            .copied()
                    });
                let root = root.map(|mut id| {
                    while let Some(parent) = editor.document.nodes.get(&id).and_then(|n| n.parent) {
                        if !editor.document.nodes.contains_key(&parent) {
                            break;
                        }
                        id = parent;
                    }
                    id
                });
                drop(editor);
                self.set_presentation(root);
                return root.is_some();
            }
            "prototype-back" => {
                let previous = self.prototype_history.borrow_mut().pop();
                if previous.is_some() {
                    self.set_presentation(previous);
                }
                return true;
            }
            "connect" => {
                return self.connect_prototype();
            }
            "flow-start" => {
                let mut editor = self.editor.borrow_mut();
                let id = editor.selection.first().copied();
                let page = editor.active_page;
                editor.transact("Set prototype start", |doc| {
                    if let Some(p) = doc.page_mut(page) {
                        p.prototype_start = id;
                    }
                });
                drop(editor);
                self.changed();
                return true;
            }
            "save" | "export-json" => {
                platform::download(
                    "FerroDesign.ferrodesign",
                    &self.document_json(),
                    "application/json",
                );
                self.close_modal();
                return true;
            }
            "open" => {
                #[cfg(target_os = "emscripten")]
                platform::pick_file();
                #[cfg(not(target_os = "emscripten"))]
                self.import_dialog();
                return true;
            }
            "import-json" => {
                self.import_dialog();
                return true;
            }
            "export-svg" => {
                let svg = self.editor.borrow().export_selection_svg();
                platform::download("FerroDesign.svg", &svg, "image/svg+xml");
                self.close_modal();
                return true;
            }
            "export-png" => {
                match self.canvas.export_png() {
                    Ok(bytes) => {
                        if !platform::download_bytes("FerroDesign.png", &bytes, "image/png") {
                            self.toast("Could not save the PNG.");
                        }
                    }
                    Err(message) => self.toast(&message),
                }
                return true;
            }
            "copy-css" => {
                let code = self.field("CodeField").text().unwrap_or_default();
                platform::copy(&code);
                return true;
            }
            "export-xaml" => {
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
                let xaml = export_xaml(&editor.document, &roots);
                drop(editor);
                platform::download("Design.xaml", &xaml, "application/xml");
                self.close_modal();
                return true;
            }
            _ => {}
        }
        self.canvas.finish_pen();
        match command {
            "undo" => {
                self.editor.borrow_mut().undo();
            }
            "redo" => {
                self.editor.borrow_mut().redo();
            }
            "copy" => {
                self.editor.borrow_mut().copy_selection();
            }
            "cut" => {
                self.editor.borrow_mut().cut_selection();
            }
            "paste" => {
                self.editor.borrow_mut().paste();
            }
            "duplicate" => {
                self.editor.borrow_mut().duplicate_selection();
            }
            "delete" => self.editor.borrow_mut().delete_selection(),
            "select-all" => {
                let mut editor = self.editor.borrow_mut();
                editor.selection = editor
                    .document
                    .page(editor.active_page)
                    .map(|p| p.children.clone())
                    .unwrap_or_default();
            }
            "group" => {
                self.editor.borrow_mut().group_selection();
            }
            "frame-selection" => {
                self.editor.borrow_mut().frame_selection();
            }
            "ungroup" => {
                self.editor.borrow_mut().ungroup_selection();
            }
            "union" | "subtract" | "intersect" | "exclude" => {
                self.editor.borrow_mut().boolean_selection(match command {
                    "subtract" => BooleanOperation::Subtract,
                    "intersect" => BooleanOperation::Intersect,
                    "exclude" => BooleanOperation::Exclude,
                    _ => BooleanOperation::Union,
                });
            }
            "flatten" => {
                self.editor.borrow_mut().flatten_selection();
            }
            "align-left" | "align-center" | "align-right" | "align-top" | "align-middle"
            | "align-bottom" => {
                self.editor.borrow_mut().align_selection(match command {
                    "align-left" => Align::Left,
                    "align-center" => Align::HorizontalCenter,
                    "align-right" => Align::Right,
                    "align-top" => Align::Top,
                    "align-middle" => Align::VerticalCenter,
                    _ => Align::Bottom,
                });
            }
            "distribute-horizontal" | "distribute-vertical" => {
                self.editor.borrow_mut().distribute_selection(
                    if command == "distribute-horizontal" {
                        Distribution::Horizontal
                    } else {
                        Distribution::Vertical
                    },
                );
            }
            "front" | "forward" | "backward" | "back" => {
                self.editor.borrow_mut().arrange_selection(match command {
                    "front" => Arrange::BringToFront,
                    "forward" => Arrange::BringForward,
                    "backward" => Arrange::SendBackward,
                    _ => Arrange::SendToBack,
                });
            }
            "toggle-visible" | "toggle-lock" => {
                let ids = self.editor.borrow().selection.clone();
                let mut editor = self.editor.borrow_mut();
                editor.begin_transaction("Toggle layer state");
                for id in ids {
                    if command == "toggle-visible" {
                        editor.toggle_visibility(id)
                    } else {
                        editor.toggle_locked(id)
                    }
                }
                editor.commit_transaction();
            }
            "flip" => self.modify("Flip horizontal", |n| n.flip_x = !n.flip_x),
            "add-page" => {
                let mut editor = self.editor.borrow_mut();
                let name = format!("Page {}", editor.document.pages.len() + 1);
                let id = editor.transact("Add page", |doc| doc.add_page(name));
                editor.active_page = id;
                editor.selection.clear();
                editor.refresh();
                drop(editor);
                self.canvas.reset_fit();
            }
            "new" => {
                let document = core::Document::new("Untitled");
                *self.editor.borrow_mut() = Editor::new(document);
                self.canvas.reset_fit();
                self.expanded.borrow_mut().clear();
            }
            "auto-layout" => {
                self.editor
                    .borrow_mut()
                    .set_auto_layout(AutoLayout::default());
            }
            "remove-layout" => {
                let id = self.editor.borrow().selection.first().copied();
                if let Some(id) = id {
                    self.editor.borrow_mut().remove_auto_layout(id);
                }
            }
            "layout-row" | "layout-column" | "layout-grid" => {
                self.modify("Change layout direction", |n| {
                    if let Some(layout) = &mut n.layout {
                        layout.axis = match command {
                            "layout-column" => LayoutAxis::Column,
                            "layout-grid" => LayoutAxis::Grid,
                            _ => LayoutAxis::Row,
                        };
                    }
                });
            }
            "add-fill" => self.modify("Add fill", |n| {
                n.fills.push(Fill::solid(Color::rgb(217, 217, 217)))
            }),
            "gradient" => self.modify("Change fill", |n| {
                let color = n.fill_color();
                let paint = if n
                    .fills
                    .first()
                    .is_some_and(|f| matches!(f.paint, Paint::LinearGradient { .. }))
                {
                    Paint::Solid(color)
                } else {
                    Paint::LinearGradient {
                        start: Point::new(0.0, 0.0),
                        end: Point::new(1.0, 1.0),
                        stops: vec![
                            GradientStop { offset: 0.0, color },
                            GradientStop {
                                offset: 1.0,
                                color: Color::rgb(241, 165, 255),
                            },
                        ],
                    }
                };
                if n.fills.is_empty() {
                    n.fills.push(Fill::solid(color));
                }
                n.fills[0].paint = paint;
            }),
            "add-stroke" => self.modify("Add stroke", |n| {
                n.strokes.push(Stroke::new(Color::BLACK, 1.0))
            }),
            "add-effect" => self.modify("Add drop shadow", |n| {
                n.effects.push(Effect::shadow(
                    Color::rgba(0, 0, 0, 40),
                    Point::new(0.0, 8.0),
                    24.0,
                ))
            }),
            "bold" => self.modify("Text weight", |n| {
                n.font_weight = if n.font_weight >= 600 { 400 } else { 700 }
            }),
            "text-left" | "text-center" | "text-right" => self.modify("Text alignment", |n| {
                n.text_align = match command {
                    "text-center" => TextAlign::Center,
                    "text-right" => TextAlign::Right,
                    _ => TextAlign::Left,
                }
            }),
            "component" => {
                self.editor.borrow_mut().create_component();
            }
            "instance" => {
                let editor = self.editor.borrow();
                let component = editor
                    .selection
                    .first()
                    .and_then(|id| editor.document.nodes.get(id))
                    .and_then(|n| {
                        if n.kind == NodeKind::Component {
                            Some(n.id)
                        } else {
                            n.component_source.or(n.component_id)
                        }
                    })
                    .or_else(|| {
                        editor
                            .document
                            .nodes
                            .values()
                            .find(|n| n.kind == NodeKind::Component)
                            .map(|n| n.id)
                    });
                let point = self
                    .canvas
                    .viewport
                    .get()
                    .screen_to_world(Point::new(160.0, 120.0));
                drop(editor);
                if let Some(id) = component {
                    self.editor
                        .borrow_mut()
                        .create_instance(id, point.x, point.y);
                }
            }
            "detach" => {
                let ids = self.editor.borrow().selection.clone();
                for id in ids {
                    self.editor.borrow_mut().detach_instance(id);
                }
            }
            "sync-instances" => {
                let ids = self.editor.borrow().selection.clone();
                for id in ids {
                    self.editor.borrow_mut().sync_instances(id);
                }
            }
            _ => return false,
        }
        self.close_modal();
        self.changed();
        self.canvas.focus();
        true
    }
    fn toast(&self, message: &str) {
        self.text("ToastText", message);
        self.visible("Toast", true);
    }
    fn close_modal(&self) {
        self.visible("ModalBackdrop", false);
        self.canvas.focus();
    }
    fn modal(&self, title: &str, description: &str) {
        self.text("ModalTitle", title);
        self.text("ModalDescription", description);
        self.control::<StackPanel>("ModalContent")
            .children()
            .clear();
        self.control::<StackPanel>("ModalActions")
            .children()
            .clear();
        self.visible("ModalBackdrop", true);
    }
    fn modal_button(self: &Rc<Self>, caption: &str, action: &'static str) {
        let button = Button::new();
        button.set_content(Some(Rc::new(caption.to_string())));
        button.classes().add("primary");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                s.command(action);
            }
        });
        self.control::<StackPanel>("ModalActions")
            .children()
            .add(button);
    }
    fn action_dialog(
        self: &Rc<Self>,
        title: &str,
        description: &str,
        actions: &[(&str, &'static str)],
    ) {
        self.modal(title, description);
        let host = self.control::<StackPanel>("ModalContent");
        let search = TextBox::new();
        search.set_watermark(Some("Search actions…"));
        host.children().add(search.clone());
        let rows = StackPanel::new();
        rows.set_spacing(3.0);
        host.children().add(rows.clone());
        let mut buttons = Vec::new();
        for &(caption, command) in actions {
            let button = Button::new();
            button.classes().add("layer");
            button.set_height(34.0);
            button.set_content(Some(Rc::new(caption.to_string())));
            let weak = Rc::downgrade(self);
            button.click(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    s.command(command);
                }
            });
            rows.children().add(button.clone());
            buttons.push((caption.to_lowercase(), button));
        }
        let weak = search.downgrade();
        search.text_changed(move |_, _| {
            if let Some(search) = weak.upgrade() {
                let value = search.text().unwrap_or_default().to_lowercase();
                for (name, button) in &buttons {
                    button.set_is_visible(name.contains(&value));
                }
            }
        });
        search.focus();
    }
    fn import_dialog(self: &Rc<Self>) {
        self.modal("Import design", "Paste the JSON from a FerroDesign file.");
        let text = TextBox::new();
        text.set_accepts_return(true);
        text.set_min_height(300.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        self.control::<StackPanel>("ModalContent")
            .children()
            .add(text.clone());
        let button = Button::new();
        button.set_content(Some(Rc::new("Import".to_string())));
        button.classes().add("primary");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                s.import_document(&text.text().unwrap_or_default(), "Imported design");
            }
        });
        self.control::<StackPanel>("ModalActions")
            .children()
            .add(button);
    }
    fn effects_dialog(self: &Rc<Self>) {
        let id = self.editor.borrow().selection.first().copied();
        let Some(id) = id else {
            self.toast("Select a layer first.");
            return;
        };
        let value = serde_json::to_string_pretty(&self.editor.borrow().document.nodes[&id].effects)
            .unwrap_or_default();
        self.modal("Effects","Edit the effect stack. Radius, spread, offset, color, and visibility are preserved in the design.");
        let text = TextBox::new();
        text.set_accepts_return(true);
        text.set_min_height(300.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text(Some(&value));
        self.control::<StackPanel>("ModalContent")
            .children()
            .add(text.clone());
        let button = Button::new();
        button.set_content(Some(Rc::new("Apply effects".to_string())));
        button.classes().add("primary");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                match serde_json::from_str::<Vec<Effect>>(&text.text().unwrap_or_default()) {
                    Ok(effects)
                        if effects.iter().all(|e| {
                            e.radius.is_finite()
                                && e.radius >= 0.0
                                && e.radius <= 10000.0
                                && e.spread.is_finite()
                                && e.offset.x.is_finite()
                                && e.offset.y.is_finite()
                        }) =>
                    {
                        s.editor
                            .borrow_mut()
                            .update_node(id, "Edit effects", |n| n.effects = effects);
                        s.close_modal();
                        s.changed();
                    }
                    _ => s.toast("Check the effect values and JSON format."),
                }
            }
        });
        self.control::<StackPanel>("ModalActions")
            .children()
            .add(button);
    }
    fn advanced_dialog(self: &Rc<Self>) {
        let editor = self.editor.borrow();
        let Some(&id) = editor.selection.first() else {
            drop(editor);
            self.toast("Select a layer first.");
            return;
        };
        let value = serde_json::to_string_pretty(&editor.document.nodes[&id]).unwrap_or_default();
        drop(editor);
        self.modal("Layer data","Edit constraints, rich text runs, paint stacks, vector segments, export settings, or component properties.");
        let text = TextBox::new();
        text.set_accepts_return(true);
        text.set_min_height(360.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text(Some(&value));
        self.control::<StackPanel>("ModalContent")
            .children()
            .add(text.clone());
        let button = Button::new();
        button.set_content(Some(Rc::new("Apply layer".to_string())));
        button.classes().add("primary");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                match serde_json::from_str::<Node>(&text.text().unwrap_or_default()) {
                    Ok(mut node) => {
                        let mut editor = s.editor.borrow_mut();
                        let mut candidate = editor.document.clone();
                        if let Some(old) = candidate.nodes.get(&id) {
                            node.id = id;
                            node.parent = old.parent;
                            node.children = old.children.clone();
                        }
                        candidate.nodes.insert(id, node.clone());
                        if candidate.validate().is_ok() {
                            editor.update_node(id, "Edit layer data", |n| *n = node);
                            drop(editor);
                            s.close_modal();
                            s.changed();
                        } else {
                            drop(editor);
                            s.toast("Layer values failed document validation.");
                        }
                    }
                    Err(error) => s.toast(&format!("Check the JSON: {error}")),
                }
            }
        });
        self.control::<StackPanel>("ModalActions")
            .children()
            .add(button);
    }

    fn variables_dialog(self: &Rc<Self>) {
        let editor = self.editor.borrow();
        let value = serde_json::to_string_pretty(&editor.document.variables).unwrap_or_default();
        let mode = editor.document.variable_mode.clone();
        drop(editor);
        self.modal("Local variables","Edit the variable collection and switch its active mode. Bindings are applied to the document when saved.");
        let mode_field = TextBox::new();
        mode_field.set_text(Some(&mode));
        mode_field.set_watermark(Some("Mode"));
        let text = TextBox::new();
        text.set_accepts_return(true);
        text.set_min_height(320.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text(Some(&value));
        let host = self.control::<StackPanel>("ModalContent");
        host.children().add(mode_field.clone());
        host.children().add(text.clone());
        let button = Button::new();
        button.set_content(Some(Rc::new("Apply variables".to_string())));
        button.classes().add("primary");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                let parsed = serde_json::from_str(&text.text().unwrap_or_default());
                match parsed {
                    Ok(variables) => {
                        let mode = mode_field.text().unwrap_or_default();
                        let mut editor = s.editor.borrow_mut();
                        let mut candidate = editor.document.clone();
                        candidate.variables = variables;
                        candidate.variable_mode = mode;
                        if candidate.validate().is_ok() {
                            editor.transact("Edit variables", |doc| {
                                doc.variables = candidate.variables;
                                doc.variable_mode = candidate.variable_mode;
                                doc.apply_variables();
                            });
                            drop(editor);
                            s.close_modal();
                            s.changed();
                        } else {
                            drop(editor);
                            s.toast("Variable data failed document validation.");
                        }
                    }
                    Err(error) => s.toast(&format!("Check the variable JSON: {error}")),
                }
            }
        });
        self.control::<StackPanel>("ModalActions")
            .children()
            .add(button);
    }

    fn comment_dialog(self: &Rc<Self>, position: Point) {
        self.modal("Add a comment", "Comments are saved with this design.");
        let text = TextBox::new();
        text.set_accepts_return(true);
        text.set_min_height(120.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_watermark(Some("Leave a comment…"));
        self.control::<StackPanel>("ModalContent")
            .children()
            .add(text.clone());
        let button = Button::new();
        button.set_content(Some(Rc::new("Post comment".to_string())));
        button.classes().add("primary");
        let weak = Rc::downgrade(self);
        button.click(move |_, _| {
            if let Some(s) = weak.upgrade() {
                let body = text.text().unwrap_or_default();
                if body.trim().is_empty() {
                    return;
                }
                let id = s.editor.borrow().selection.first().copied();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
                    .to_string();
                s.editor
                    .borrow_mut()
                    .add_comment(id, position, "WS", body, timestamp);
                s.close_modal();
                s.canvas.set_tool(Tool::Select);
                s.changed();
            }
        });
        self.control::<StackPanel>("ModalActions")
            .children()
            .add(button);
        text.focus();
    }
    fn comments_dialog(self: &Rc<Self>) {
        self.modal("Comments", "Review comments saved in this file.");
        let comments = self.editor.borrow().document.comments.clone();
        let host = self.control::<StackPanel>("ModalContent");
        if comments.is_empty() {
            let label = TextBlock::new();
            label.set_text(Some(
                "Select the comment tool and click the canvas to start a thread.",
            ));
            label.set_text_wrapping(TextWrapping::Wrap);
            host.children().add(label);
        }
        for comment in comments {
            let panel = StackPanel::new();
            panel.set_spacing(6.0);
            panel.set_margin(Thickness::new(0.0, 6.0, 0.0, 12.0));
            for message in comment.messages {
                let author = TextBlock::new();
                author.set_text(Some(&message.author));
                author.set_font_weight(FontWeight::SEMI_BOLD);
                panel.children().add(author);
                let body = TextBlock::new();
                body.set_text(Some(&message.body));
                body.set_text_wrapping(TextWrapping::Wrap);
                panel.children().add(body);
            }
            let button = Button::new();
            button.set_content(Some(Rc::new(if comment.resolved {
                "Reopen".to_string()
            } else {
                "Resolve".to_string()
            })));
            let weak = Rc::downgrade(self);
            let id = comment.id;
            let resolved = comment.resolved;
            button.click(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    s.editor.borrow_mut().resolve_comment(id, !resolved);
                    s.changed();
                    s.comments_dialog();
                }
            });
            panel.children().add(button);
            host.children().add(panel);
        }
        self.modal_button("Add comment", "comment");
    }
    fn connect_prototype(self: &Rc<Self>) -> bool {
        let source = self.editor.borrow().selection.first().copied();
        let index = self.control::<ComboBox>("PrototypeTarget").selected_index();
        let target = if index >= 0 {
            self.prototype_targets.borrow().get(index as usize).copied()
        } else {
            None
        };
        if let (Some(source), Some(target)) = (source, target) {
            self.editor
                .borrow_mut()
                .add_interaction(source, target, core::Transition::default());
            self.changed();
            true
        } else {
            self.toast("Select a layer and a destination frame.");
            false
        }
    }
    fn set_presentation(self: &Rc<Self>, root: Option<NodeId>) {
        self.close_modal();
        self.canvas.presentation.set(root);
        self.prototype.borrow_mut().current = root;
        if root.is_none() {
            self.prototype.borrow_mut().overlays.clear();
        }
        *self.canvas.overlays.borrow_mut() = self.prototype.borrow().overlays.clone();
        let present = root.is_some();
        for name in [
            "NavigationRail",
            "LeftPanel",
            "RightPanel",
            "Toolbar",
            "ZoomPanel",
            "CanvasCaption",
        ] {
            self.visible(name, !present);
        }
        self.control::<Grid>("Workspace").set_column_definitions(
            ColumnDefinitions::parse(if present { "0,0,*,0" } else { "48,248,*,288" }).unwrap(),
        );
        self.canvas.reset_fit();
        self.canvas.focus();
    }
    fn prototype_click(self: &Rc<Self>, id: NodeId) {
        let mut editor = self.editor.borrow_mut();
        let mut player = self.prototype.borrow_mut();
        let previous = player.current;
        let mut source = Some(id);
        let mut triggered = false;
        while let Some(id) = source {
            if player.trigger(&mut editor.document, id, core::Trigger::Click) {
                triggered = true;
                break;
            }
            source = editor.document.nodes.get(&id).and_then(|n| n.parent);
        }
        let current = player.current;
        let overlays = player.overlays.clone();
        if triggered {
            editor.refresh();
        }
        drop(player);
        drop(editor);
        if triggered {
            if current != previous {
                if let Some(previous) = previous {
                    self.prototype_history.borrow_mut().push(previous);
                }
                self.set_presentation(current);
            }
            *self.canvas.overlays.borrow_mut() = overlays;
            self.canvas.invalidate_visual();
        }
    }
}

fn number(value: f64) -> String {
    if value.fract().abs() < 0.0001 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}
fn css(n: &Node) -> String {
    let mut output = format!(
        "/* {} */\nposition: absolute;\nleft: {}px;\ntop: {}px;\nwidth: {}px;\nheight: {}px;\n",
        n.name.replace("*/", ""),
        number(n.x),
        number(n.y),
        number(n.width),
        number(n.height)
    );
    if n.rotation != 0.0 {
        output += &format!("transform: rotate({}deg);\n", number(n.rotation));
    }
    if n.opacity < 1.0 {
        output += &format!("opacity: {};\n", number(n.opacity));
    }
    if n.corner_radius > 0.0 {
        output += &format!("border-radius: {}px;\n", number(n.corner_radius));
    }
    if let Some(fill) = n.fills.first() {
        match &fill.paint {
            Paint::Solid(color) => output += &format!("background: {};\n", color.css()),
            Paint::LinearGradient { stops, .. } => {
                output += &format!(
                    "background: linear-gradient(135deg, {});\n",
                    stops
                        .iter()
                        .map(|s| format!("{} {}%", s.color.css(), number(s.offset * 100.0)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            _ => {}
        }
    }
    if let Some(stroke) = n.strokes.first() {
        output += &format!(
            "border: {}px solid {};\n",
            number(stroke.width),
            stroke.paint.representative_color().css()
        );
    }
    if n.kind == NodeKind::Text {
        output+=&format!("font-family: '{}';\nfont-size: {}px;\nfont-weight: {};\nline-height: {};\ncolor: {};\ntext-align: {};\n",
            n.font_family.replace('\'',""),number(n.font_size),n.font_weight,number(n.line_height),n.fill_color().css(),
            match n.text_align{TextAlign::Center=>"center",TextAlign::Right=>"right",TextAlign::Justify=>"justify",_=>"left"});
    }
    if let Some(layout) = &n.layout {
        output += &format!(
            "display: {};\n",
            if layout.axis == LayoutAxis::Grid {
                "grid"
            } else {
                "flex"
            }
        );
        if layout.axis == LayoutAxis::Grid {
            output += &format!("grid-template-columns: repeat({}, 1fr);\n", layout.columns);
        } else {
            output += &format!(
                "flex-direction: {};\n",
                if layout.axis == LayoutAxis::Row {
                    "row"
                } else {
                    "column"
                }
            );
        }
        output += &format!(
            "gap: {}px;\npadding: {}px {}px {}px {}px;\n",
            number(layout.gap),
            number(layout.padding.top),
            number(layout.padding.right),
            number(layout.padding.bottom),
            number(layout.padding.left)
        );
    }
    output
}
fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "&#10;")
}
fn xaml_color(color: Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color.a, color.r, color.g, color.b
    )
}
fn export_xaml(doc: &core::Document, roots: &[NodeId]) -> String {
    let mut result=String::from("<Canvas xmlns=\"https://github.com/ferroui\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">\n");
    for &id in roots {
        xaml_node(doc, id, 1, &mut result)
    }
    result += "</Canvas>\n";
    result
}
fn xaml_node(doc: &core::Document, id: NodeId, depth: usize, result: &mut String) {
    let Some(n) = doc.nodes.get(&id) else { return };
    let pad = "  ".repeat(depth);
    let position=format!("Canvas.Left=\"{}\" Canvas.Top=\"{}\" Width=\"{}\" Height=\"{}\" Opacity=\"{}\" IsVisible=\"{}\"",
        number(n.x),number(n.y),number(n.width),number(n.height),number(n.opacity),if n.visible{"True"}else{"False"});
    let fill = xaml_color(n.fill_color());
    let transform = if n.rotation != 0.0 {
        format!(
            " RenderTransform=\"rotate({}deg)\" RenderTransformOrigin=\"50%,50%\"",
            number(n.rotation)
        )
    } else {
        String::new()
    };
    if n.kind.is_container() {
        *result += &format!(
            "{pad}<Canvas {position}{transform} Background=\"{fill}\" ClipToBounds=\"{}\">\n",
            if n.clip_content { "True" } else { "False" }
        );
        for &child in &n.children {
            xaml_node(doc, child, depth + 1, result)
        }
        *result += &format!("{pad}</Canvas>\n");
        return;
    }
    let stroke = n
        .strokes
        .first()
        .map(|s| {
            format!(
                " Stroke=\"{}\" StrokeThickness=\"{}\"",
                xaml_color(s.paint.representative_color()),
                number(s.width)
            )
        })
        .unwrap_or_default();
    match n.kind {
        NodeKind::Text=>*result+=&format!("{pad}<TextBlock {position}{transform} Text=\"{}\" FontFamily=\"{}\" FontSize=\"{}\" FontWeight=\"{}\" Foreground=\"{fill}\" TextWrapping=\"Wrap\"/>\n",xml(&n.text),xml(&n.font_family),number(n.font_size),n.font_weight),
        NodeKind::Ellipse=>*result+=&format!("{pad}<Ellipse {position}{transform} Fill=\"{fill}\"{stroke}/>\n"),
        NodeKind::Vector=>{
            let mut data=String::new();
            for segment in &n.path.segments{data+=&match segment{
                core::PathSegment::MoveTo{point}=>format!("M{},{} ",number(point.x),number(point.y)),
                core::PathSegment::LineTo{point}=>format!("L{},{} ",number(point.x),number(point.y)),
                core::PathSegment::CubicTo{control1,control2,point}=>format!("C{},{} {},{} {},{} ",number(control1.x),number(control1.y),number(control2.x),number(control2.y),number(point.x),number(point.y)),
                core::PathSegment::QuadraticTo{control,point}=>format!("Q{},{} {},{} ",number(control.x),number(control.y),number(point.x),number(point.y)),
                core::PathSegment::Close=>"Z ".into(),
            };}
            *result+=&format!("{pad}<Path {position}{transform} Data=\"{data}\" Fill=\"{fill}\"{stroke}/>\n");
        }
        NodeKind::Line=>*result+=&format!("{pad}<Line {position}{transform} StartPoint=\"0,0\" EndPoint=\"{},{}\"{stroke}/>\n",number(n.width),number(n.height)),
        NodeKind::Polygon|NodeKind::Star=>{
            let points=n.polygon_points().iter().map(|p|format!("{},{}",number(p.x),number(p.y))).collect::<Vec<_>>().join(" ");
            *result+=&format!("{pad}<Polygon {position}{transform} Points=\"{points}\" Fill=\"{fill}\"{stroke}/>\n");
        }
        _=>*result+=&format!("{pad}<Rectangle {position}{transform} RadiusX=\"{}\" RadiusY=\"{}\" Fill=\"{fill}\"{stroke}/>\n",number(n.corner_radius),number(n.corner_radius)),
    }
}
const FILE_ACTIONS: &[(&str, &str)] = &[
    ("New design", "new"),
    ("Open design…  Ctrl+O", "open"),
    ("Import JSON…", "import-json"),
    ("Save editable design…  Ctrl+S", "save"),
    ("Export SVG", "export-svg"),
    ("Export PNG", "export-png"),
    ("Export FerroUI XAML", "export-xaml"),
    ("Local variables and styles", "variables"),
    ("Local components", "assets"),
    ("Comments", "comments"),
    ("Present prototype", "present"),
];
const LAYER_ACTIONS: &[(&str, &str)] = &[
    ("Undo  Ctrl+Z", "undo"),
    ("Redo  Ctrl+Shift+Z", "redo"),
    ("Copy  Ctrl+C", "copy"),
    ("Cut  Ctrl+X", "cut"),
    ("Paste  Ctrl+V", "paste"),
    ("Duplicate  Ctrl+D", "duplicate"),
    ("Delete", "delete"),
    ("Group  Ctrl+G", "group"),
    ("Ungroup  Ctrl+Shift+G", "ungroup"),
    ("Frame selection", "frame-selection"),
    ("Bring to front", "front"),
    ("Bring forward", "forward"),
    ("Send backward", "backward"),
    ("Send to back", "back"),
    ("Align left", "align-left"),
    ("Align horizontal centers", "align-center"),
    ("Align right", "align-right"),
    ("Align top", "align-top"),
    ("Align vertical centers", "align-middle"),
    ("Align bottom", "align-bottom"),
    ("Distribute horizontally", "distribute-horizontal"),
    ("Distribute vertically", "distribute-vertical"),
    ("Union selection", "union"),
    ("Subtract selection", "subtract"),
    ("Intersect selection", "intersect"),
    ("Exclude overlap", "exclude"),
    ("Flatten to vector", "flatten"),
    ("Add auto layout", "auto-layout"),
    ("Remove auto layout", "remove-layout"),
    ("Create component", "component"),
    ("Create instance", "instance"),
    ("Update instances", "sync-instances"),
    ("Detach instance", "detach"),
    ("Layer data…", "advanced"),
];
const ACTIONS: &[(&str, &str)] = &[
    ("Move / Select  V", "select"),
    ("Frame  F", "frame"),
    ("Rectangle  R", "rectangle"),
    ("Ellipse  O", "ellipse"),
    ("Line  L", "line"),
    ("Polygon", "polygon"),
    ("Star", "star"),
    ("Pen  P", "pen"),
    ("Pencil  Shift+P", "pencil"),
    ("Text  T", "text"),
    ("Hand  H", "hand"),
    ("Comment  C", "comment"),
    ("Section  Shift+F", "section"),
    ("Undo  Ctrl+Z", "undo"),
    ("Redo  Ctrl+Shift+Z", "redo"),
    ("Duplicate  Ctrl+D", "duplicate"),
    ("Group  Ctrl+G", "group"),
    ("Ungroup  Ctrl+Shift+G", "ungroup"),
    ("Frame selection", "frame-selection"),
    ("Add auto layout", "auto-layout"),
    ("Union selection", "union"),
    ("Subtract selection", "subtract"),
    ("Intersect selection", "intersect"),
    ("Exclude overlap", "exclude"),
    ("Flatten to vector", "flatten"),
    ("Create component", "component"),
    ("Create instance", "instance"),
    ("Update instances", "sync-instances"),
    ("Local variables and styles", "variables"),
    ("Toggle grid", "grid"),
    ("Toggle rulers", "rulers"),
    ("Toggle pixel snapping", "snapping"),
    ("Zoom to fit  1", "fit"),
    ("Zoom to selection  2", "fit-selection"),
    ("Layer data…", "advanced"),
];
