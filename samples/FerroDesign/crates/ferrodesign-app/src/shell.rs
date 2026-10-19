//! The XAML shell binds real FerroUI controls to the document command layer.
mod paints;
mod effects;
mod components;
mod layout;
mod layer_projection;
mod prototyping;
mod rich_text;
mod text;
mod variables;
use crate::{
    canvas::{CanvasEvent, DesignCanvas, Tool},
    icons, platform,
    prototype::{PrototypeRuntime, PrototypeRuntimeEvent},
};
use ferrodesign_core::{
    self as core, Align, Arrange, AutoLayout, BooleanOperation, Color, Distribution, Editor,
    Effect, Fill, GradientStop, LayoutAxis, Node, NodeId, NodeKind, Padding, Paint, Point, Stroke,
    TextAlign,
};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::time::{Duration, Instant};
use ferroui_base::{BoxedValue, ObjectType, Ref, Thickness, WeakRef};
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Button, CheckBox, ColumnDefinitions, ComboBox, Control, Grid, ItemsControl,
    ItemsSource, StackPanel, TextBlock, TextBox, UserControl,
};
use ferroui_markup_xaml::FerroXamlLoader;
use serde_json::json;
use layer_projection::{LayerProjection, LayerRow};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

pub struct EditorShell {
    pub root: Ref<UserControl>,
    pub canvas: Ref<DesignCanvas>,
    pub editor: Rc<RefCell<Editor>>,
    updating: Cell<bool>,
    inline_text: Cell<Option<NodeId>>,
    inline_updating: Cell<bool>,
    expanded: RefCell<BTreeSet<NodeId>>,
    layer_buttons: RefCell<BTreeMap<NodeId, WeakRef<Button>>>,
    layer_projection: RefCell<LayerProjection>,
    page_rows: RefCell<Vec<(NodeId, String)>>,
    assets: Cell<bool>,
    inspector_tab: Cell<u8>,
    prototype_targets: RefCell<Vec<NodeId>>,
    prototype: Rc<PrototypeRuntime>,
    pending_save: Cell<bool>,
    pending_save_timer: RefCell<Option<Rc<dyn IDisposable>>>,
    save_serializations: Cell<u64>,
    last_serialization_ms: Cell<f64>,
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
        let prototype = PrototypeRuntime::new(editor.clone(), canvas.clone());
        root.get_control::<Grid>("CanvasHost")
            .children()
            .insert(0, canvas.clone());
        let this = Rc::new(Self {
            root,
            canvas,
            editor,
            updating: Cell::new(false),
            inline_text: Cell::new(None),
            inline_updating: Cell::new(false),
            expanded: RefCell::new(expanded),
            layer_buttons: RefCell::new(BTreeMap::new()),
            layer_projection: RefCell::new(LayerProjection::default()),
            page_rows: RefCell::new(Vec::new()),
            assets: Cell::new(false),
            inspector_tab: Cell::new(0),
            prototype_targets: RefCell::new(Vec::new()),
            prototype,
            pending_save: Cell::new(false),
            pending_save_timer: RefCell::new(None),
            save_serializations: Cell::new(0),
            last_serialization_ms: Cell::new(0.0),
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
    fn choice(self: &Rc<Self>, name: &'static str, labels: &[&str], apply: fn(&mut Node, usize)) {
        let control = self.control::<ComboBox>(name);
        for label in labels {
            control.items().add(Some(Rc::new(label.to_string())));
        }
        let weak = Rc::downgrade(self);
        control.selection_changed(move |_, _| {
            if let Some(shell) = weak.upgrade() {
                if shell.updating.get() {
                    return;
                }
                let index = shell.control::<ComboBox>(name).selected_index();
                if index >= 0 {
                    shell.modify("Edit layer property", |node| apply(node, index as usize));
                    shell.changed();
                }
            }
        });
    }
    fn wire(self: &Rc<Self>) {
        self.wire_inline_text();
        self.hook("EditPaintsButton", "paints");
        self.hook("EditInteractionsButton", "interactions");
        self.hook("CropImageButton", "crop-image");
        self.hook("CropDoneButton", "crop-done");
        self.choice("BlendModeChoice", &BLEND_OPTIONS.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            |node, index| { node.blend_mode = BLEND_OPTIONS[index].1; node.overridden_properties.insert("blend_mode".into()); });
        let weak = Rc::downgrade(self);
        self.prototype.set_callback(move |event| {
            if let Some(shell) = weak.upgrade() {
                shell.prototype_event(event);
            }
        });
        let weak = Rc::downgrade(self);
        self.control::<ItemsControl>("LayersHost")
            .set_item_template(Some(FuncDataTemplate::for_type::<LayerRow>(
                move |row, _| weak.upgrade().map(|shell| shell.layer_row(row).upcast()),
                false,
            )));
        self.choice(
            "HorizontalConstraint",
            &["Left", "Right", "Center", "Left and right", "Scale"],
            |n, i| {
                n.constraints.horizontal = [
                    core::Constraint::Start,
                    core::Constraint::End,
                    core::Constraint::Center,
                    core::Constraint::Stretch,
                    core::Constraint::Scale,
                ][i];
            },
        );
        self.choice(
            "VerticalConstraint",
            &["Top", "Bottom", "Center", "Top and bottom", "Scale"],
            |n, i| {
                n.constraints.vertical = [
                    core::Constraint::Start,
                    core::Constraint::End,
                    core::Constraint::Center,
                    core::Constraint::Stretch,
                    core::Constraint::Scale,
                ][i];
            },
        );
        self.choice(
            "StrokeAlignChoice",
            &["Inside", "Center", "Outside"],
            |n, i| {
                if let Some(stroke) = n.strokes.first_mut() {
                    stroke.align = [
                        core::StrokeAlign::Inside,
                        core::StrokeAlign::Center,
                        core::StrokeAlign::Outside,
                    ][i];
                }
            },
        );
        self.choice(
            "StrokeCapChoice",
            &["Butt", "Round", "Square", "Arrow"],
            |n, i| {
                if let Some(stroke) = n.strokes.first_mut() {
                    stroke.cap = [
                        core::StrokeCap::Butt,
                        core::StrokeCap::Round,
                        core::StrokeCap::Square,
                        core::StrokeCap::Arrow,
                    ][i];
                }
            },
        );
        self.choice("StrokeJoinChoice", &["Miter", "Round", "Bevel"], |n, i| {
            if let Some(stroke) = n.strokes.first_mut() {
                stroke.join = [
                    core::StrokeJoin::Miter,
                    core::StrokeJoin::Round,
                    core::StrokeJoin::Bevel,
                ][i];
            }
        });
        self.choice(
            "TextVerticalChoice",
            &["Top", "Center", "Bottom"],
            |n, i| {
                n.text_vertical_align = [
                    core::TextVerticalAlign::Top,
                    core::TextVerticalAlign::Center,
                    core::TextVerticalAlign::Bottom,
                ][i];
            },
        );
        self.choice(
            "TextResizeChoice",
            &["Fixed size", "Auto height", "Auto width and height"],
            |n, i| {
                n.text_auto_resize = [
                    core::TextAutoResize::Fixed,
                    core::TextAutoResize::Height,
                    core::TextAutoResize::WidthAndHeight,
                ][i];
            },
        );
        self.choice(
            "TextDecorationChoice",
            &["None", "Underline", "Strikethrough"],
            |n, i| {
                n.text_decoration = [
                    core::TextDecoration::None,
                    core::TextDecoration::Underline,
                    core::TextDecoration::Strikethrough,
                ][i];
            },
        );
        self.choice(
            "LayoutMainChoice",
            &["Start", "Center", "End", "Space between", "Stretch"],
            |n, i| {
                if let Some(layout) = &mut n.layout {
                    layout.main_align = layout_alignment(i);
                }
            },
        );
        self.choice(
            "LayoutCrossChoice",
            &["Start", "Center", "End", "Space between", "Stretch"],
            |n, i| {
                if let Some(layout) = &mut n.layout {
                    layout.cross_align = layout_alignment(i);
                }
            },
        );
        self.choice(
            "WidthSizingChoice",
            &["Fixed", "Hug contents", "Fill container"],
            |n, i| {
                let mode = [
                    core::SizingMode::Fixed,
                    core::SizingMode::Hug,
                    core::SizingMode::Fill,
                ][i];
                n.layout_child.width = mode;
                if let Some(layout) = &mut n.layout {
                    layout.width = mode;
                }
            },
        );
        self.choice(
            "HeightSizingChoice",
            &["Fixed", "Hug contents", "Fill container"],
            |n, i| {
                let mode = [
                    core::SizingMode::Fixed,
                    core::SizingMode::Hug,
                    core::SizingMode::Fill,
                ][i];
                n.layout_child.height = mode;
                if let Some(layout) = &mut n.layout {
                    layout.height = mode;
                }
            },
        );
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
            ("GradientButton", "gradient"),
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
            ("EditLayoutButton", "layout-details"),
            ("PageStylesButton", "styles"),
            ("PageVariablesButton", "variables"),
            ("PageComponentsButton", "assets"),
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
            ("RichTextButton", "rich-text"),
            ("TextLeftButton", "text-left"),
            ("TextCenterButton", "text-center"),
            ("TextRightButton", "text-right"),
            ("CreateComponentButton", "component"),
            ("EditComponentsButton", "components"),
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
            ("EditVectorButton", "edit-vector"),
            ("DoneVectorButton", "done-vector"),
            ("ConvertNetworkButton", "vector-network"),
            ("NetworkPenButton", "network-pen"), ("NetworkJoinButton", "network-join"),
            ("NetworkFillButton", "network-fill"), ("NetworkWeldButton", "network-weld"),
            ("VectorSelectButton", "vector-points"),
            ("VectorBendButton", "vector-bend"),
            ("VectorEraseButton", "vector-erase"),
            ("CornerPointButton", "vector-corner"),
            ("SmoothPointButton", "vector-smooth"),
            ("SymmetricPointButton", "vector-symmetric"),
            ("ItalicButton", "italic"),
            ("TextJustifyButton", "text-justify"),
            ("IndependentCornersButton", "independent-corners"),
            ("LayoutWrapButton", "layout-wrap"),
            ("LayoutReverseButton", "layout-reverse"),
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
            "PageBackgroundField",
            "StrokeField",
            "StrokeWidthField",
            "TextField",
            "FontFamilyField",
            "FontSizeField",
            "FontWeightField",
            "LineHeightField",
            "LetterSpacingField",
            "DashField",
            "CornerTLField",
            "CornerTRField",
            "CornerBRField",
            "CornerBLField",
            "CrossGapField",
            "LayoutColumnsField",
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
            CanvasEvent::Selection => self.refresh_selection(),
            CanvasEvent::Camera => {
                self.sync_zoom();
                self.position_inline_text();
            }
            CanvasEvent::Tool => self.sync_tools(),
            CanvasEvent::Notice(message) => self.toast(&message),
            CanvasEvent::EditText(id) => {
                self.begin_inline_text(id);
            }
            CanvasEvent::Comment(position) => self.comment_dialog(position),
            CanvasEvent::ContextMenu => self.action_dialog(
                "Layer actions",
                "Edit, arrange, and organize the selected layers.",
                LAYER_ACTIONS,
            ),
            CanvasEvent::PrototypeTrigger(id, trigger) => {
                self.prototype.dispatch(id, trigger);
            }
            CanvasEvent::PrototypeScroll(id, delta) => { self.prototype.scroll(id, delta); }
            CanvasEvent::PrototypePointerMoved(point) => { self.prototype.pointer_moved(point); }
            CanvasEvent::PrototypePointerExited => { self.prototype.pointer_exited(); }
            CanvasEvent::PrototypeKey(key) => {
                if !self.prototype.key(&key) && key == "Escape" {
                    self.prototype.stop();
                }
            }
            CanvasEvent::ExitPresentation => self.set_presentation(None),
            CanvasEvent::Action(command) => {
                self.command(command);
            }
        }
    }
    pub fn document_json(&self) -> String {
        self.prototype.document_json().unwrap_or_default()
    }
    pub fn state_json(&self) -> String {
        let editor = self.editor.borrow();
        let v = self.canvas.viewport.get();
        let stats = self.canvas.render_stats();
        let (layer_row_count, layer_projection_stats) = {
            let projection = self.layer_projection.borrow();
            (projection.rows().len(), projection.stats())
        };
        let selection_bounds = editor
            .selection
            .iter()
            .filter_map(|&id| editor.document.world_bounds(id))
            .reduce(|a, b| a.union(b));
        let canvas = self.canvas.bounds();
        // Native control bounds make the browser validation bridge useful for
        // real pointer input and layout checks without a parallel DOM editor.
        let native_bounds = |control: &Control| {
            let bounds = control.bounds();
            let origin = control
                .translate_point(ferroui_base::Point::new(0.0, 0.0), &self.root)
                .unwrap_or_default();
            let mut state = json!({"x":origin.x,"y":origin.y,"width":bounds.width,"height":bounds.height,
                "visible":control.is_effectively_visible(),
                "enabled":control.is_effectively_enabled(),
                "measureValid":control.is_measure_valid(),"arrangeValid":control.is_arrange_valid(),
                "focused":control.is_focused(),"focusWithin":control.is_keyboard_focus_within()});
            if let Some(choice) = control.downcast_ref::<ComboBox>() {
                // Popups have a separate visual ancestry. Report the realized
                // item's actual keyboard focus instead of inferring it from
                // the ComboBox's focus-within flag or its selected value.
                let focused_index = choice.get_realized_containers().into_iter()
                    .find(|item| item.is_keyboard_focus_within())
                    .map(|item| choice.index_from_container(&item));
                state["choice"] = json!({"open":choice.is_drop_down_open(),
                    "selectedIndex":choice.selected_index(),"itemCount":choice.item_count(),
                    "focusedIndex":focused_index});
            }
            state
        };
        let mut chrome: serde_json::Map<String, serde_json::Value> = [
            "CanvasHost",
            "Toolbar",
            "SelectTool",
            "FrameTool",
            "RectTool",
            "EllipseTool",
            "PenTool",
            "TextTool",
            "HandTool",
            "CommentTool",
            "ActionsButton",
            "PagesViewport",
            "LayersViewport",
            "InspectorViewport",
            "PageInspector", "DesignInspector", "PageBackgroundField", "PageBackgroundSwatch",
            "PageStylesButton", "PageVariablesButton", "PageComponentsButton",
            "EditLayoutButton",
            "AddPageButton",
            "EditVectorButton",
            "DoneVectorButton",
            "VectorPointOptions",
            "VectorSelectButton", "VectorBendButton", "VectorEraseButton",
            "ConvertNetworkButton", "NetworkPenButton", "NetworkJoinButton", "NetworkFillButton", "NetworkWeldButton",
            "CornerPointButton", "SmoothPointButton", "SymmetricPointButton",
            "InlineTextBox",
            "EditPaintsButton",
            "EffectButton", "AddEffectButton",
            "CropImageButton", "CropDoneButton", "BlendModeChoice",
            "RichTextButton",
            "EditComponentsButton",
            "EditInteractionsButton",
            "ModalBackdrop",
            "ModalContent",
            "ModalViewport",
            "ModalActions",
        ]
        .into_iter()
        .map(|name| {
            (
                name.to_string(),
                native_bounds(&self.control::<Control>(name)),
            )
        })
        .collect();
        if self.control::<Control>("ModalBackdrop").is_visible() {
            for visual in self
                .control::<Control>("ModalBackdrop")
                .get_visual_descendants()
            {
                if let Some(control) = visual.cast::<Control>() {
                    if let Some(name) = control
                        .name()
                        .filter(|name| ["Paint", "Effect", "Interaction", "RichText", "Variable", "Component", "Crop", "Layout"]
                            .iter().any(|prefix| name.starts_with(prefix)))
                    {
                        chrome.insert(name, native_bounds(&control));
                    }
                }
            }
        }
        if let Some(index) = editor
            .document
            .pages
            .iter()
            .position(|p| p.id == editor.active_page)
        {
            let button = self
                .control::<StackPanel>("PagesHost")
                .children()
                .get(index);
            chrome.insert("ActivePage".into(), native_bounds(&button));
        }
        // Keep each macro expansion small while preserving the flat bridge
        // schema. These groups are moved into the result without cloning values.
        let mut state = serde_json::Map::new();
        for group in [json!({
            "revision":editor.revision,"documentRevision":editor.document.revision,"nodeCount":editor.document.nodes.len(),
            "selectionCount":editor.selection.len(),"selection":editor.selection,"selectionBounds":selection_bounds,
            "tool":self.canvas.tool.get().name(),"zoom":v.zoom,"pan":{"x":v.pan.x,"y":v.pan.y},
            "zoomLabel":self.button("ZoomValueButton").content().and_then(|value| value.downcast_ref::<String>().cloned()),
            "canvas":{"x":canvas.x,"y":canvas.y,"width":canvas.width,"height":canvas.height},
            "chrome":chrome,
            "layerRowCount":layer_row_count,
            "realizedLayerRows":self.layer_buttons.borrow().values().filter(|row| row.upgrade().is_some()).count(),
            "vectorNode":self.canvas.vector_node.get(),
            "vectorTool":format!("{:?}",self.canvas.vector_tool.get()),
            "networkPoint":self.canvas.network_point.get(),
            "networkVertices":*self.canvas.network_vertices.borrow(),
            "imageCrop":self.canvas.image_crop_state(),
            "layoutDrag":self.canvas.layout_drag_state(),
            "editingText":self.inline_text.get(),
            "editingComposition":self.inline_text.get().is_some() && self.field("InlineTextBox").is_composing_text(),
            "prototypeOverlays":*self.canvas.overlays.borrow(),
            "prototypeAnimation":self.prototype.animation_state(),
            "prototypeScrollOffsets":self.prototype.scroll_offsets(),
            "vectorPoint":self.canvas.vector_point.get().map(|point| json!({"segment":point.segment,"kind":format!("{:?}",point.kind)})),
        }), json!({
            "saveStatus":platform::document_save_status(),
            "serializationPending":self.pending_save.get(),
            "saveSerializations":self.save_serializations.get(),
            "lastSerializationMs":self.last_serialization_ms.get(),
            "renderer":"FerroUI Skia","pageCount":editor.document.pages.len(),"activePage":editor.active_page,
            "renderedFrames":self.canvas.frames.get(),"visibleNodes":stats.visible_nodes,"culledNodes":stats.culled_nodes,
            "cachedNodes":stats.cached_nodes,"missingImages":stats.missing_images,"unsupportedEffects":stats.unsupported_effects,
            "backdropCacheHits":stats.backdrop_cache_hits,"backdropRasterizations":stats.backdrop_rasterizations,
            "backdropSkipped":stats.backdrop_skipped,"backdropTooLarge":stats.backdrop_too_large,
            "fullPrepares":stats.full_prepares,"incrementalPrepares":stats.incremental_prepares,
            "preparedNodes":stats.prepared_nodes,"rebuiltVisuals":stats.rebuilt_visuals,
            "rootBoundsTests":stats.root_bounds_tests,"rootCandidates":stats.root_candidates,
            "rootQueryReused":stats.root_query_reused,
            "totalFullPrepares":stats.total_full_prepares,"totalIncrementalPrepares":stats.total_incremental_prepares,
            "totalPreparedNodes":stats.total_prepared_nodes,"totalRebuiltVisuals":stats.total_rebuilt_visuals,
            "invalidNodes":stats.invalid_nodes,
            "canUndo":editor.undo_label().is_some(),"canRedo":editor.redo_label().is_some(),
            "historyLength":editor.history_len(),"presentation":self.canvas.presentation.get(),
        }), json!({
            "layerProjectionBuilds":layer_projection_stats.builds,
            "layerProjectionReuses":layer_projection_stats.reuses,
        })] {
            let serde_json::Value::Object(fields) = group else {
                unreachable!("state groups are JSON objects");
            };
            state.extend(fields);
        }
        serde_json::Value::Object(state).to_string()
    }
    pub fn notify(&self, message: &str) {
        self.toast(message);
    }
    pub fn import_document(self: &Rc<Self>, text: &str, name: &str) -> bool {
        self.finish_inline_text(true);
        self.prototype.stop();
        self.canvas.cancel();
        let text = text.trim_start_matches('\u{feff}').trim_start();
        if text.starts_with("data:image/") {
            return self.import_image(text, name);
        }
        let is_svg = text.starts_with('<');
        let result = if is_svg {
            self.editor.borrow_mut().import_svg(text, name).map(|_| ())
        } else {
            self.editor.borrow_mut().load_json(text)
        };
        match result {
            Ok(_) => {
                self.expanded.borrow_mut().clear();
                if !is_svg {
                    self.canvas.reset_fit();
                }
                self.canvas.invalidate_scene();
                self.changed();
                self.close_modal();
                if is_svg {
                    self.canvas.zoom_to_selection();
                }
                true
            }
            Err(error) => {
                self.toast(&format!("Could not open design: {error}"));
                false
            }
        }
    }
    fn import_image(self: &Rc<Self>, href: &str, name: &str) -> bool {
        let mime = href.split_once(';').map(|(mime, _)| mime).unwrap_or("");
        if !matches!(
            mime,
            "data:image/png" | "data:image/jpeg" | "data:image/gif" | "data:image/bmp"
        ) {
            self.toast("Choose a PNG, JPEG, GIF, or BMP image.");
            return false;
        }
        let (width, height) = match self.canvas.register_image_data(href) {
            Ok(size) => size,
            Err(error) => {
                self.toast(&format!("Could not decode image: {error}"));
                return false;
            }
        };
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            self.toast("The image has invalid dimensions.");
            return false;
        }
        let view = self.canvas.viewport.get();
        let center = view.screen_to_world(Point::new(view.width / 2.0, view.height / 2.0));
        let mut node = Node::new(
            NodeKind::Rectangle,
            name,
            center.x - width / 2.0,
            center.y - height / 2.0,
            width,
            height,
        );
        let mut fill = Fill::solid(Color::WHITE);
        fill.paint = Paint::Image {
            href: href.to_owned(),
            scale: ferrodesign_core::ImageScale::Fill,
            crop: None,
        };
        node.fills = vec![fill];
        let result = {
            let mut editor = self.editor.borrow_mut();
            let parent = editor.active_page;
            editor.insert_node(parent, node)
        };
        match result {
            Ok(_) => {
                self.changed();
                self.close_modal();
                self.canvas.zoom_to_selection();
                true
            }
            Err(error) => {
                self.toast(&format!("Could not place image: {error}"));
                false
            }
        }
    }
    fn changed(self: &Rc<Self>) {
        // Coalesce before serializing: a held arrow key should not clone a
        // multi-megabyte JSON string on the input/render thread for each event.
        self.pending_save.set(true);
        platform::set_document_serialization_pending(true);
        if let Some(timer) = self.pending_save_timer.borrow_mut().take() { timer.dispose(); }
        let weak = Rc::downgrade(self);
        *self.pending_save_timer.borrow_mut() = Some(DispatcherTimer::run_once(
            move || { if let Some(shell) = weak.upgrade() { shell.flush_pending_save(); } },
            Duration::from_millis(220), DispatcherPriority::BACKGROUND));
        self.text("DocumentStatus", &platform::document_save_status());
        self.refresh();
        self.canvas.invalidate_visual();
    }
    /// Lifecycle/export boundaries synchronously capture confirmed authored
    /// state before the host awaits its durable IndexedDB transaction.
    fn flush_pending_save(&self) {
        if let Some(timer) = self.pending_save_timer.borrow_mut().take() { timer.dispose(); }
        if !self.pending_save.get() { return; }
        let started = Instant::now();
        let text = match self.prototype.autosave_json() {
            Ok(text) if !text.is_empty() => text,
            _ => { self.toast("The document could not be serialized. Export your design before closing."); return; }
        };
        self.last_serialization_ms.set(started.elapsed().as_secs_f64() * 1000.0);
        self.save_serializations.set(self.save_serializations.get().wrapping_add(1));
        if !platform::save_document(&text) {
            self.text("DocumentStatus", &platform::document_save_status());
            self.toast("Autosave could not accept this document. Export the design to keep your changes.");
            return;
        }
        self.pending_save.set(false);
        platform::set_document_serialization_pending(false);
        self.text("DocumentStatus", &platform::document_save_status());
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
        self.canvas.reconcile_image_crop();
        self.canvas.reconcile_vector_edit();
        self.sync_tools();
        self.refresh_pages();
        self.refresh_layers();
        self.refresh_inspector();
        self.updating.set(false);
    }
    fn refresh_selection(&self) {
        if self.updating.replace(true) {
            return;
        }
        // Selection does not change the layer tree. Reuse native control rows so
        // clicking artwork never recreates every page, icon and layer control.
        self.canvas.reconcile_image_crop();
        self.canvas.reconcile_vector_edit();
        self.refresh_layer_selection();
        self.refresh_inspector();
        self.updating.set(false);
    }
    fn refresh_layer_selection(&self) {
        let selection = self.editor.borrow().selection.clone();
        let buttons = {
            let mut buttons = Vec::new();
            self.layer_buttons.borrow_mut().retain(|id, weak| {
                if let Some(button) = weak.upgrade() {
                    buttons.push((button, selection.contains(id)));
                    true
                } else {
                    false
                }
            });
            buttons
        };
        // Native class changes can apply styles synchronously. Keep the weak
        // registry unborrowed while updating this bounded set of realized rows.
        for (button, selected) in buttons {
            button.classes().set("active", selected);
        }
    }
    fn refresh_pages(self: &Rc<Self>) {
        let host = self.control::<StackPanel>("PagesHost");
        let editor = self.editor.borrow();
        self.text("FileTitle", &editor.document.name);
        let rows: Vec<_> = editor
            .document
            .pages
            .iter()
            .map(|p| (p.id, p.name.clone()))
            .collect();
        if *self.page_rows.borrow() == rows {
            for (index, (id, name)) in rows.iter().enumerate() {
                let button = host
                    .children()
                    .get(index)
                    .cast::<Button>()
                    .expect("page button");
                button.classes().set("active", *id == editor.active_page);
                button.set_content(Some(Rc::new(format!(
                    "{}  {}",
                    if *id == editor.active_page {
                        "✓"
                    } else {
                        " "
                    },
                    name
                ))));
            }
            return;
        }
        *self.page_rows.borrow_mut() = rows;
        host.children().clear();
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
            host.children().add(button.clone());
            if id == editor.active_page {
                button.bring_into_view();
            }
        }
    }
    fn refresh_layers(self: &Rc<Self>) {
        let host = self.control::<ItemsControl>("LayersHost");
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
        let assets = self.assets.get();
        let (items, empty) = {
            let editor = self.editor.borrow();
            let expanded = self.expanded.borrow();
            let mut projection = self.layer_projection.borrow_mut();
            let changed = projection.refresh(&editor, assets, &filter, &expanded);
            let items = changed.then(|| ItemsSource::from_items(
                projection.rows().iter().cloned()
                    .map(|row| Some(Rc::new(row) as BoxedValue)),
            ));
            (items, projection.rows().is_empty())
        };
        // ItemsSource realization can synchronously call layer_row. No editor,
        // expansion or projection RefCell borrow crosses native control setters.
        self.visible("EmptyLayers", empty);
        self.text(
            "EmptyLayers",
            if !filter.is_empty() {
                "No layers match your search."
            } else if assets {
                "Create a component to reuse it here."
            } else {
                "Draw a frame or shape to get started."
            },
        );
        if let Some(items) = items {
            self.layer_buttons.borrow_mut().clear();
            host.set_items_source(Some(items));
        } else {
            self.refresh_layer_selection();
        }
    }
    fn layer_row(self: &Rc<Self>, row: &LayerRow) -> Ref<Grid> {
        let id = row.id;
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("20,*,24").unwrap());
        grid.set_height(30.0);
        grid.set_margin(Thickness::new(
            (row.depth.min(10) * 12) as f64,
            0.0,
            0.0,
            0.0,
        ));
        if row.has_children {
            let collapse = Button::new();
            collapse.set_min_width(20.0);
            collapse.set_padding(Thickness::uniform(0.0));
            collapse.set_content(Some(Control::boxed(icons::icon(
                if row.expanded {
                    "chevron-down"
                } else {
                    "chevron-right"
                },
                "#777777",
                8.0,
            ))));
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
            .set("active", self.editor.borrow().selection.contains(&id));
        if !row.visible {
            button.set_opacity(0.4);
        }
        let content = StackPanel::new();
        content.set_orientation(Orientation::Horizontal);
        content.set_spacing(7.0);
        let glyph = match row.kind {
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
            if matches!(row.kind, NodeKind::Component | NodeKind::Instance) {
                "#9747FF"
            } else {
                "#777777"
            },
            12.0,
        ));
        let name = TextBlock::new();
        name.set_text(Some(&row.name));
        name.set_vertical_alignment(VerticalAlignment::Center);
        name.set_font_size(11.0);
        content.children().add(name);
        button.set_content(Some(Control::boxed(content)));
        Grid::set_column(&button, 1);
        let weak = Rc::downgrade(self);
        let assets = row.assets;
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
                    s.refresh_selection();
                    s.canvas.invalidate_visual();
                }
                s.canvas.focus();
            }
        });
        // Weak handles must not keep scrolled-out item controls alive.
        self.layer_buttons
            .borrow_mut()
            .insert(id, button.downgrade());
        grid.children().add(button);
        let eye = Button::new();
        eye.set_padding(Thickness::uniform(5.0));
        eye.set_content(Some(Control::boxed(icons::icon(
            if row.locked { "lock" } else { "eye" },
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
        grid
    }
    fn refresh_inspector(&self) {
        let editor = self.editor.borrow();
        let node = editor
            .selection
            .first()
            .and_then(|id| editor.document.nodes.get(id));
        self.visible("DesignInspector", self.inspector_tab.get() == 0 && node.is_some());
        self.visible("PageInspector", self.inspector_tab.get() == 0 && node.is_none());
        self.visible("VisibleButton", node.is_some());
        self.visible("LockedButton", node.is_some());
        self.visible("PrototypeInspector", self.inspector_tab.get() == 1);
        self.visible("InspectInspector", self.inspector_tab.get() == 2);
        let vector_edit = self.canvas.vector_node.get().is_some();
        self.visible(
            "VectorOptions",
            self.inspector_tab.get() == 0 && node.is_some_and(|n| n.kind == NodeKind::Vector),
        );
        self.visible("VectorPointOptions", vector_edit);
        self.visible("EditVectorButton", !vector_edit);
        self.visible("DoneVectorButton", vector_edit);
        let image_crop = self.canvas.image_crop_state().is_some();
        self.visible("CropImageButton", !image_crop && node.is_some_and(|node|
            node.fills.iter().any(|fill| matches!(fill.paint, Paint::Image { .. }))));
        self.visible("CropDoneButton", image_crop);
        let network = node.is_some_and(|n| n.vector_network.is_some());
        self.visible("NetworkOptions", network && vector_edit);
        self.visible("ConvertNetworkButton", !network);
        self.button("NetworkPenButton").classes().set("active", self.canvas.vector_tool.get() == crate::canvas::VectorTool::Connect);
        for (name, mode) in [("VectorSelectButton", crate::canvas::VectorTool::Point),
            ("VectorBendButton", crate::canvas::VectorTool::Bend), ("VectorEraseButton", crate::canvas::VectorTool::Erase)] {
            self.button(name).classes().set("active", self.canvas.vector_tool.get() == mode);
        }
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
            ("FontWeightField", node.map(|n| n.font_weight as f64)),
            ("LineHeightField", node.map(|n| n.line_height * 100.0)),
            ("LetterSpacingField", node.map(|n| n.letter_spacing)),
            (
                "CornerTLField",
                node.map(|n| n.corner_radii.unwrap_or([n.corner_radius; 4])[0]),
            ),
            (
                "CornerTRField",
                node.map(|n| n.corner_radii.unwrap_or([n.corner_radius; 4])[1]),
            ),
            (
                "CornerBRField",
                node.map(|n| n.corner_radii.unwrap_or([n.corner_radius; 4])[2]),
            ),
            (
                "CornerBLField",
                node.map(|n| n.corner_radii.unwrap_or([n.corner_radius; 4])[3]),
            ),
            (
                "CrossGapField",
                node.and_then(|n| n.layout.as_ref().map(|l| l.cross_gap)),
            ),
            (
                "LayoutColumnsField",
                node.and_then(|n| n.layout.as_ref().map(|l| l.columns as f64)),
            ),
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
        let page_background = editor.document.page(editor.active_page)
            .map_or(Color::WHITE, |page| page.background);
        set("PageBackgroundField", page_background.hex().trim_start_matches('#').to_string());
        self.control::<Border>("PageBackgroundSwatch")
            .set_background(Some(icons::brush(&xaml_color(page_background))));
        self.text("PageResourcesSummary", &format!("{} styles · {} variable collections",
            editor.document.styles.len(), editor.document.variables.len()));
        self.button("FillSwatch")
            .set_background(Some(icons::brush(&xaml_color(fill))));
        set(
            "StrokeField",
            node.and_then(|n| n.strokes.first())
                .map(|s| s.paint.representative_color().hex())
                .unwrap_or_else(|| "000000".into()),
        );
        set(
            "DashField",
            node.and_then(|n| n.strokes.first())
                .map(|s| {
                    s.dash
                        .iter()
                        .map(|v| number(*v))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
        );
        self.visible("StrokeOptions", node.is_some_and(|n| !n.strokes.is_empty()));
        self.visible(
            "CornerOptions",
            node.is_some_and(|n| n.corner_radii.is_some()),
        );
        self.button("IndependentCornersButton")
            .classes()
            .set("active", node.is_some_and(|n| n.corner_radii.is_some()));
        self.button("ItalicButton")
            .classes()
            .set("active", node.is_some_and(|n| n.italic));
        self.button("LayoutWrapButton").classes().set(
            "active",
            node.is_some_and(|n| n.layout.as_ref().is_some_and(|l| l.wrap)),
        );
        self.button("LayoutReverseButton").classes().set(
            "active",
            node.is_some_and(|n| n.layout.as_ref().is_some_and(|l| l.reverse)),
        );
        for (name, index) in [
            ("BlendModeChoice", node.and_then(|node| BLEND_OPTIONS.iter()
                .position(|(_, mode)| *mode == node.blend_mode).map(|index| index as i32))),
            (
                "HorizontalConstraint",
                node.map(|n| constraint_index(n.constraints.horizontal)),
            ),
            (
                "VerticalConstraint",
                node.map(|n| constraint_index(n.constraints.vertical)),
            ),
            (
                "WidthSizingChoice",
                node.map(|n| {
                    sizing_index(n.layout.as_ref().map_or(n.layout_child.width, |l| l.width))
                }),
            ),
            (
                "HeightSizingChoice",
                node.map(|n| {
                    sizing_index(
                        n.layout
                            .as_ref()
                            .map_or(n.layout_child.height, |l| l.height),
                    )
                }),
            ),
            (
                "LayoutMainChoice",
                node.and_then(|n| n.layout.as_ref().map(|l| alignment_index(l.main_align))),
            ),
            (
                "LayoutCrossChoice",
                node.and_then(|n| n.layout.as_ref().map(|l| alignment_index(l.cross_align))),
            ),
            (
                "StrokeAlignChoice",
                node.and_then(|n| n.strokes.first()).map(|s| match s.align {
                    core::StrokeAlign::Inside => 0,
                    core::StrokeAlign::Center => 1,
                    core::StrokeAlign::Outside => 2,
                }),
            ),
            (
                "StrokeCapChoice",
                node.and_then(|n| n.strokes.first()).map(|s| match s.cap {
                    core::StrokeCap::Butt => 0,
                    core::StrokeCap::Round => 1,
                    core::StrokeCap::Square => 2,
                    core::StrokeCap::Arrow => 3,
                }),
            ),
            (
                "StrokeJoinChoice",
                node.and_then(|n| n.strokes.first()).map(|s| match s.join {
                    core::StrokeJoin::Miter => 0,
                    core::StrokeJoin::Round => 1,
                    core::StrokeJoin::Bevel => 2,
                }),
            ),
            (
                "TextVerticalChoice",
                node.map(|n| match n.text_vertical_align {
                    core::TextVerticalAlign::Top => 0,
                    core::TextVerticalAlign::Center => 1,
                    core::TextVerticalAlign::Bottom => 2,
                }),
            ),
            (
                "TextResizeChoice",
                node.map(|n| match n.text_auto_resize {
                    core::TextAutoResize::Fixed => 0,
                    core::TextAutoResize::Height => 1,
                    core::TextAutoResize::WidthAndHeight => 2,
                }),
            ),
            (
                "TextDecorationChoice",
                node.map(|n| match n.text_decoration {
                    core::TextDecoration::None => 0,
                    core::TextDecoration::Underline => 1,
                    core::TextDecoration::Strikethrough => 2,
                }),
            ),
        ] {
            self.control::<ComboBox>(name)
                .set_selected_index(index.unwrap_or(-1));
        }
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
        self.visible("EditLayoutButton", node.is_some_and(|node| node.layout.is_some()
            || node.parent.and_then(|parent| editor.document.nodes.get(&parent))
                .is_some_and(|parent| parent.layout.is_some())));
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
        self.canvas.finish_image_crop();
        let ids = self.editor.borrow().selection.clone();
        self.editor.borrow_mut().update_nodes_with_layout(&ids, label, |node| operation(node));
    }
    fn commit_field(self: &Rc<Self>, name: &str) {
        if self.updating.get() {
            return;
        }
        let text = self.field(name).text().unwrap_or_default();
        let id = self.editor.borrow().selection.first().copied();
        self.canvas.finish_image_crop();
        if id.is_none() {
            if name == "NameField" || name == "FillField" || name == "PageBackgroundField" {
                if name != "NameField" && Color::from_hex(&text).is_none() {
                    self.toast("Use a hex color, for example 7B61FF.");
                    self.refresh();
                    return;
                }
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
            "TextField" => self.modify("Edit text", |n| {
                n.replace_text_preserving_runs(text.clone());
            }),
            "FontFamilyField" => self.modify("Font family", |n| n.font_family = text.clone()),
            "DashField" => {
                let dash = text
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|s| !s.is_empty())
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>();
                let Ok(dash) = dash else {
                    self.toast("Enter nonnegative dash lengths, separated by commas.");
                    return;
                };
                if dash
                    .iter()
                    .any(|v| !v.is_finite() || *v < 0.0 || *v > 100000.0)
                    || (!dash.is_empty() && dash.iter().all(|v| *v == 0.0))
                {
                    self.toast(
                        "A dash pattern needs a positive length and finite nonnegative values.",
                    );
                    return;
                }
                self.modify("Stroke dash pattern", |n| {
                    if let Some(stroke) = n.strokes.first_mut() {
                        stroke.dash = dash.clone();
                    }
                });
            }
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
                        "RadiusField" => {
                            n.corner_radius = value.max(0.0);
                            n.corner_radii = None;
                        }
                        "FontSizeField" => n.font_size = value.clamp(1.0, 4096.0),
                        "FontWeightField" => {
                            n.font_weight = value.round().clamp(1.0, 1000.0) as u16
                        }
                        "LineHeightField" => n.line_height = (value / 100.0).clamp(0.1, 100.0),
                        "LetterSpacingField" => n.letter_spacing = value.clamp(-4096.0, 4096.0),
                        "CornerTLField" | "CornerTRField" | "CornerBRField" | "CornerBLField" => {
                            let index = match name {
                                "CornerTLField" => 0,
                                "CornerTRField" => 1,
                                "CornerBRField" => 2,
                                _ => 3,
                            };
                            n.corner_radii.get_or_insert([n.corner_radius; 4])[index] =
                                value.max(0.0);
                        }
                        "CrossGapField" => {
                            if let Some(layout) = &mut n.layout {
                                layout.cross_gap = value.max(0.0);
                            }
                        }
                        "LayoutColumnsField" => {
                            if let Some(layout) = &mut n.layout {
                                layout.columns = value.round().clamp(1.0, 1000.0) as usize;
                            }
                        }
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
        if command == "commit-edit" {
            self.finish_inline_text(true);
            self.canvas.finish_image_crop();
            self.flush_pending_save();
            return !self.pending_save.get();
        }
        if command == "storage-status" {
            self.text("DocumentStatus", &platform::document_save_status());
            return true;
        }
        self.finish_inline_text(true);
        if matches!(command, "undo" | "redo") && self.canvas.cancel_active_edit() {
            return true;
        }
        if !matches!(command, "crop-image" | "crop-done") { self.canvas.finish_image_crop(); }
        if self.prototype.is_active()
            && !matches!(
                command,
                "present" | "prototype-back" | "save" | "export-json"
            )
        {
            self.prototype.stop();
        }
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
            "crop-image" => {
                let opened = self.canvas.edit_image_crop(None);
                if !opened { self.toast("Select an unlocked shape with a Fill or Stretch image paint. Fit and Tile crops are available in the paint editor."); }
                return opened;
            }
            "crop-done" => return self.canvas.finish_image_crop(),
            "interactions" => {
                self.interactions_dialog();
                return true;
            }
            "paints" => {
                self.paints_dialog();
                return true;
            }
            "vector-network" => return self.canvas.convert_network(),
            "network-join" => return self.canvas.network_connect_selection(),
            "network-fill" => {
                let changed = self.canvas.network_fill_selection();
                if !changed { self.toast("Shift-select at least three connected vertices in boundary order."); }
                return changed;
            }
            "network-weld" => return self.canvas.network_weld_selection(),
            "network-pen" => {
                if self.canvas.vector_node.get().is_none() && !self.canvas.edit_vector() { return false; }
                self.canvas.convert_network();
                self.canvas.vector_tool.set(crate::canvas::VectorTool::Connect);
                self.refresh_selection(); self.canvas.focus(); return true;
            }
            "vector-points" | "vector-bend" | "vector-erase" => {
                if self.canvas.vector_node.get().is_none() && !self.canvas.edit_vector() { return false; }
                self.canvas.vector_tool.set(match command {
                    "vector-bend" => crate::canvas::VectorTool::Bend,
                    "vector-erase" => crate::canvas::VectorTool::Erase,
                    _ => crate::canvas::VectorTool::Point,
                });
                self.refresh_selection();
                self.canvas.focus();
                return true;
            }
            "edit-vector" => return self.canvas.edit_vector(),
            "done-vector" => {
                self.canvas.leave_vector_edit();
                self.refresh_selection();
                self.canvas.focus();
                return true;
            }
            "vector-corner" | "vector-smooth" | "vector-symmetric" => {
                let mode = match command {
                    "vector-corner" => core::AnchorMode::Corner,
                    "vector-smooth" => core::AnchorMode::Smooth,
                    _ => core::AnchorMode::Symmetric,
                };
                let changed = self.canvas.convert_vector_point(mode);
                self.canvas.focus();
                return changed;
            }
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
            "file" | "layers" | "assets" => {
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
                self.visual_variables_dialog();
                return true;
            }
            "components" => {
                self.components_dialog();
                return true;
            }
            "layout-details" => {
                self.layout_dialog();
                return true;
            }
            "rich-text" => {
                self.rich_text_dialog();
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
                return self.prototype.back();
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
            "place-image" => {
                #[cfg(target_os = "emscripten")]
                platform::pick_image();
                #[cfg(not(target_os = "emscripten"))]
                self.import_dialog();
                return true;
            }
            "styles" => {
                self.styles_dialog();
                return true;
            }
            "create-fill-style" | "create-text-style" | "create-effect-style" => {
                self.create_style(command);
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
                let flattened = self.editor.borrow_mut().flatten_selection().is_some();
                if !flattened {
                    self.toast("Select shapes or vectors to flatten. Text and boolean outlines are not supported yet.");
                    return false;
                }
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
            "layout-wrap" => self.modify("Toggle layout wrapping", |n| {
                if let Some(layout) = &mut n.layout {
                    layout.wrap = !layout.wrap;
                }
            }),
            "layout-reverse" => self.modify("Reverse layout order", |n| {
                if let Some(layout) = &mut n.layout {
                    layout.reverse = !layout.reverse;
                }
            }),
            "independent-corners" => self.modify("Toggle independent corners", |n| {
                n.corner_radii = if n.corner_radii.is_some() {
                    None
                } else {
                    Some([n.corner_radius; 4])
                };
            }),
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
            "add-effect" => {
                self.add_effect();
                return true;
            }
            "bold" => self.modify("Text weight", |n| {
                n.font_weight = if n.font_weight >= 600 { 400 } else { 700 }
            }),
            "italic" => self.modify("Text italic", |n| n.italic = !n.italic),
            "text-left" | "text-center" | "text-right" | "text-justify" => {
                self.modify("Text alignment", |n| {
                    n.text_align = match command {
                        "text-center" => TextAlign::Center,
                        "text-justify" => TextAlign::Justify,
                        "text-right" => TextAlign::Right,
                        _ => TextAlign::Left,
                    }
                })
            }
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
        search.set_placeholder_text(Some("Search actions…"));
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
        self.modal(
            "Import design",
            "Paste a FerroDesign JSON document, SVG markup, or an embedded raster image data URI.",
        );
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

    fn styles_dialog(self: &Rc<Self>) {
        self.modal(
            "Local styles",
            "Apply a reusable fill, text, or effect style to the current selection.",
        );
        let styles: Vec<_> = self
            .editor
            .borrow()
            .document
            .styles
            .values()
            .cloned()
            .collect();
        let host = self.control::<StackPanel>("ModalContent");
        for style in styles {
            let kind = match &style.value {
                core::StyleValue::Fill { .. } => "Fill",
                core::StyleValue::Text { .. } => "Text",
                core::StyleValue::Effect { .. } => "Effects",
                core::StyleValue::Grid { .. } => "Grid",
            };
            let button = Button::new();
            button.classes().add("layer");
            button.set_content(Some(Rc::new(format!("{}  ·  {}", style.name, kind))));
            let id = style.id;
            let weak = Rc::downgrade(self);
            button.click(move |_, _| {
                if let Some(s) = weak.upgrade() {
                    let mut editor = s.editor.borrow_mut();
                    let selection = editor.selection.clone();
                    if selection.is_empty() {
                        drop(editor);
                        s.toast("Select artwork before applying a style.");
                        return;
                    }
                    editor.begin_transaction("Apply local style");
                    let mut applied = false;
                    for node in selection {
                        applied |= editor.apply_style(node, id);
                    }
                    editor.commit_transaction();
                    drop(editor);
                    if applied {
                        s.changed();
                        s.close_modal();
                    } else {
                        s.toast("This style type has no visual application control yet.");
                    }
                }
            });
            host.children().add(button);
        }
        self.modal_button("Create fill style", "create-fill-style");
        self.modal_button("Create text style", "create-text-style");
        self.modal_button("Create effect style", "create-effect-style");
    }
    fn create_style(self: &Rc<Self>, command: &str) {
        let node = {
            let editor = self.editor.borrow();
            editor
                .selection
                .first()
                .and_then(|id| editor.document.nodes.get(id))
                .cloned()
        };
        let Some(node) = node else {
            self.toast("Select a layer to create a style from its properties.");
            return;
        };
        let value = match command {
            "create-text-style" => core::StyleValue::Text {
                family: node.font_family,
                size: node.font_size,
                weight: node.font_weight,
                line_height: node.line_height,
                letter_spacing: node.letter_spacing,
            },
            "create-effect-style" => core::StyleValue::Effect {
                effects: node.effects,
            },
            _ => core::StyleValue::Fill { fills: node.fills },
        };
        let mut editor = self.editor.borrow_mut();
        editor.begin_transaction("Create local style");
        let id = editor.transact("Create local style", |doc| {
            let id = doc.allocate_id();
            doc.styles.insert(
                id,
                core::Style {
                    id,
                    name: node.name,
                    description: String::new(),
                    value,
                },
            );
            id
        });
        for node in editor.selection.clone() {
            editor.apply_style(node, id);
        }
        editor.commit_transaction();
        drop(editor);
        self.changed();
        self.styles_dialog();
    }
    fn variables_dialog(self: &Rc<Self>) {
        let editor = self.editor.borrow();
        let value = serde_json::to_string_pretty(&editor.document.variables).unwrap_or_default();
        let mode = editor.document.variable_mode.clone();
        drop(editor);
        self.modal("Local variables","Edit the variable collection and switch its active mode. Bindings are applied to the document when saved.");
        let mode_field = TextBox::new();
        mode_field.set_text(Some(&mode));
        mode_field.set_placeholder_text(Some("Mode"));
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
                                doc.layout_all();
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
        self.modal_button("Local styles", "styles");
    }

    fn comment_dialog(self: &Rc<Self>, position: Point) {
        self.modal("Add a comment", "Comments are saved with this design.");
        let text = TextBox::new();
        text.set_accepts_return(true);
        text.set_min_height(120.0);
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_placeholder_text(Some("Leave a comment…"));
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
                author.set_font_weight(FontWeight::SemiBold);
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
        self.finish_inline_text(true);
        self.close_modal();
        if let Some(root) = root {
            self.prototype.start(root);
        } else {
            self.prototype.stop();
        }
    }
    fn prototype_event(self: &Rc<Self>, event: PrototypeRuntimeEvent) {
        match event {
            PrototypeRuntimeEvent::Started(_) | PrototypeRuntimeEvent::Stopped => {
                let present = self.prototype.is_active();
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
                    ColumnDefinitions::parse(if present { "0,0,*,0" } else { "48,248,*,288" })
                        .unwrap(),
                );
                if !present {
                    self.refresh();
                }
                self.canvas.focus();
            }
            PrototypeRuntimeEvent::Changed => self.canvas.invalidate_visual(),
            PrototypeRuntimeEvent::OpenUrl(url) => {
                if !platform::open_url(&url) {
                    self.toast("The link could not be opened by this host.");
                }
                self.prototype.take_pending_url();
            }
            PrototypeRuntimeEvent::ScrollTo(_) => {
                self.prototype.take_scroll_target();
            }
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
fn layout_alignment(index: usize) -> core::LayoutAlign {
    [
        core::LayoutAlign::Start,
        core::LayoutAlign::Center,
        core::LayoutAlign::End,
        core::LayoutAlign::SpaceBetween,
        core::LayoutAlign::Stretch,
    ][index]
}
fn sizing_index(mode: core::SizingMode) -> i32 {
    match mode {
        core::SizingMode::Fixed => 0,
        core::SizingMode::Hug => 1,
        core::SizingMode::Fill => 2,
    }
}
fn constraint_index(value: core::Constraint) -> i32 {
    match value {
        core::Constraint::Start => 0,
        core::Constraint::End => 1,
        core::Constraint::Center => 2,
        core::Constraint::Stretch => 3,
        core::Constraint::Scale => 4,
    }
}
fn alignment_index(value: core::LayoutAlign) -> i32 {
    match value {
        core::LayoutAlign::Start => 0,
        core::LayoutAlign::Center => 1,
        core::LayoutAlign::End => 2,
        core::LayoutAlign::SpaceBetween => 3,
        core::LayoutAlign::Stretch => 4,
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
const BLEND_OPTIONS: &[(&str, core::BlendMode)] = &[
    ("Normal", core::BlendMode::Normal), ("Multiply", core::BlendMode::Multiply),
    ("Screen", core::BlendMode::Screen), ("Overlay", core::BlendMode::Overlay),
    ("Darken", core::BlendMode::Darken), ("Lighten", core::BlendMode::Lighten),
    ("Color dodge", core::BlendMode::ColorDodge), ("Color burn", core::BlendMode::ColorBurn),
    ("Hard light", core::BlendMode::HardLight), ("Soft light", core::BlendMode::SoftLight),
    ("Difference", core::BlendMode::Difference), ("Exclusion", core::BlendMode::Exclusion),
    ("Hue", core::BlendMode::Hue), ("Saturation", core::BlendMode::Saturation),
    ("Color", core::BlendMode::Color), ("Luminosity", core::BlendMode::Luminosity),
];
const FILE_ACTIONS: &[(&str, &str)] = &[
    ("New design", "new"),
    ("Open design or SVG…  Ctrl+O", "open"),
    ("Place image…", "place-image"),
    ("Crop image", "crop-image"),
    ("Import JSON or SVG…", "import-json"),
    ("Save editable design…  Ctrl+S", "save"),
    ("Export SVG", "export-svg"),
    ("Export PNG", "export-png"),
    ("Export FerroUI XAML", "export-xaml"),
    ("Local variables", "variables"),
    ("Style text characters", "rich-text"),
    ("Local styles", "styles"),
    ("Local components", "assets"),
    ("Component properties and variants", "components"),
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
    ("Layout and grid placement…", "layout-details"),
    ("Remove auto layout", "remove-layout"),
    ("Create component", "component"),
    ("Create instance", "instance"),
    ("Update instances", "sync-instances"),
    ("Component properties and variants", "components"),
    ("Style text characters", "rich-text"),
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
    ("Place image…", "place-image"),
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
    ("Local variables", "variables"),
    ("Local styles", "styles"),
    ("Toggle grid", "grid"),
    ("Toggle rulers", "rulers"),
    ("Toggle pixel and object snapping", "snapping"),
    ("Zoom to fit  1", "fit"),
    ("Zoom to selection  2", "fit-selection"),
    ("Layer data…", "advanced"),
];
