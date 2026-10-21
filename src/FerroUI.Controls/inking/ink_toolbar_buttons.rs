//! Toolbar controls adapted from the WinUI InkToolbar control family.
use super::color_names::{color_name, default_palette};
use super::{
    InkDrawingAttributes, InkError, InkToolbarMenuKind, InkToolbarStencilKind, InkToolbarToggle,
    InkToolbarTool, PenTipShape,
};
use crate::automation::AutomationProperties;
use crate::primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use crate::{
    ButtonImpl, ButtonImplExt, CheckBox, ContentControlImpl, Control, ControlImpl, RadioButton,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Palette = FerroList<Rc<dyn IBrush>>;

/// A mutually exclusive toolbar tool; inherits keyboard and selection automation from RadioButton.
#[repr(C)]
pub struct InkToolbarToolButton {
    base: RadioButton,
    tool_kind: Cell<InkToolbarTool>,
    permit_flyout: Cell<bool>,
}
ferro_class!(InkToolbarToolButton: RadioButton);
ferro_class_info!(InkToolbarToolButton { new: InkToolbarToolButton::new, markup: {
    properties: [ToolKind: InkToolbarTool { get: InkToolbarToolButton::tool_kind }],
} });
ferro_impl_classes!(InkToolbarToolButton: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, ContentControlImpl, ToggleButtonImpl);
impl ButtonImpl for InkToolbarToolButton {
    fn on_click(this: &Self) {
        let was_checked = this.is_checked() == Some(true);
        let previous = this.permit_flyout.replace(was_checked);
        Self::parent_on_click(this);
        this.permit_flyout.set(previous);
    }
    fn open_flyout(this: &Self) {
        if this.permit_flyout.get() && this.is_extension_glyph_shown() {
            Self::parent_open_flyout(this);
        }
    }
}
impl ControlImpl for InkToolbarToolButton {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        super::InkToolbarToolButtonAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarToolButton {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        RadioButton::TYPE
    }
}
ferro_properties! { impl InkToolbarToolButton {
    pub fn is_extension_glyph_shown_property() -> StyledProperty<bool> { FerroProperty::register::<InkToolbarToolButton,_>("IsExtensionGlyphShown", true) }
} }
impl InkToolbarToolButton {
    pub fn construct() -> Self {
        Self {
            base: RadioButton::construct(),
            tool_kind: Cell::new(InkToolbarTool::CustomTool),
            permit_flyout: Cell::new(true),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn tool_kind(&self) -> InkToolbarTool {
        self.tool_kind.get()
    }
    pub(crate) fn set_tool_kind(&self, value: InkToolbarTool) {
        self.tool_kind.set(value);
    }
    pub fn is_extension_glyph_shown(&self) -> bool {
        self.get_value(Self::is_extension_glyph_shown_property())
    }
    pub fn set_is_extension_glyph_shown(&self, value: bool) {
        self.set_value(Self::is_extension_glyph_shown_property(), value);
    }
}

/// Base pen tool. Palettes are observable handles owned independently by each pen.
#[repr(C)]
pub struct InkToolbarPenButton {
    base: InkToolbarToolButton,
    palette_subscription: RefCell<Option<(Palette, u64)>>,
    selected_brush: RefCell<Option<Rc<dyn IBrush>>>,
}
ferro_class! { InkToolbarPenButton: InkToolbarToolButton, virtuals InkToolbarPenButtonImpl: ToggleButtonImpl {
    fn create_ink_drawing_attributes(this) -> Result<InkDrawingAttributes, InkError>;
} }
ferro_class_info!(InkToolbarPenButton {
    new: InkToolbarPenButton::new
});
ferro_impl_classes!(InkToolbarPenButton: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl, ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarPenButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_palette(default_palette(false));
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::palette_property().as_property() {
            if let Some((old, token)) = this.palette_subscription.borrow_mut().take() {
                old.remove_collection_changed(token);
            }
            let palette = this.palette();
            let weak = this.to_ref().downgrade();
            let token = palette.add_collection_changed(Rc::new(move |_| {
                if let Some(pen) = weak.upgrade() {
                    pen.update_selected_brush();
                }
            }));
            *this.palette_subscription.borrow_mut() = Some((palette, token));
            this.update_selected_brush();
        } else if change.property() == Self::selected_brush_index_property().as_property() {
            this.update_selected_brush();
        } else if change.property() == Self::min_stroke_width_property().as_property()
            || change.property() == Self::max_stroke_width_property().as_property()
        {
            this.coerce_value(Self::selected_stroke_width_property().as_property());
        }
    }
}
impl InkToolbarPenButtonImpl for InkToolbarPenButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        let attrs = InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width, width),
            ..InkDrawingAttributes::default()
        };
        attrs.validate()?;
        Ok(attrs)
    }
}
ferro_properties! { impl InkToolbarPenButton {
    pub fn palette_property() -> StyledProperty<FerroList<Rc<dyn IBrush>>> { FerroProperty::register::<InkToolbarPenButton,_>("Palette", FerroList::new()) }
    pub fn min_stroke_width_property() -> StyledProperty<f64> { FerroProperty::register_with::<InkToolbarPenButton,_>("MinStrokeWidth", StyledPropertyOptions::new(0.5_f64).coerce(|_,v| if v.is_finite() && v > 0.0 {v} else {0.5})) }
    pub fn max_stroke_width_property() -> StyledProperty<f64> { FerroProperty::register_with::<InkToolbarPenButton,_>("MaxStrokeWidth", StyledPropertyOptions::new(32.0_f64).coerce(|_,v| if v.is_finite() && v > 0.0 {v} else {32.0})) }
    pub fn selected_brush_property() -> DirectProperty<InkToolbarPenButton,Option<Rc<dyn IBrush>>> { FerroProperty::register_direct::<InkToolbarPenButton,_>("SelectedBrush",InkToolbarPenButton::selected_brush,None,None) }
    pub fn selected_brush_index_property() -> StyledProperty<i32> { FerroProperty::register::<InkToolbarPenButton,_>("SelectedBrushIndex", 0) }
    pub fn selected_stroke_width_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<InkToolbarPenButton,_>("SelectedStrokeWidth", StyledPropertyOptions::new(2.0_f64).coerce(|sender,value| {
            let min = sender.get_value(Self::min_stroke_width_property());
            let max = sender.get_value(Self::max_stroke_width_property()).max(min);
            if value.is_finite() { value.clamp(min,max) } else { min }
        }))
    }
} }
impl InkToolbarPenButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToolButton::construct(),
            palette_subscription: RefCell::new(None),
            selected_brush: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn palette(&self) -> Palette {
        self.get_value(Self::palette_property())
    }
    pub fn set_palette(&self, value: Palette) {
        self.set_value(Self::palette_property(), value);
    }
    pub fn min_stroke_width(&self) -> f64 {
        self.get_value(Self::min_stroke_width_property())
    }
    pub fn set_min_stroke_width(&self, value: f64) {
        self.set_value(Self::min_stroke_width_property(), value);
    }
    pub fn max_stroke_width(&self) -> f64 {
        self.get_value(Self::max_stroke_width_property())
    }
    pub fn set_max_stroke_width(&self, value: f64) {
        self.set_value(Self::max_stroke_width_property(), value);
    }
    pub fn selected_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.selected_brush.borrow().clone()
    }
    pub fn selected_brush_index(&self) -> i32 {
        self.get_value(Self::selected_brush_index_property())
    }
    pub fn set_selected_brush_index(&self, value: i32) {
        self.set_value(Self::selected_brush_index_property(), value);
    }
    pub fn selected_stroke_width(&self) -> f64 {
        self.get_value(Self::selected_stroke_width_property())
    }
    pub fn set_selected_stroke_width(&self, value: f64) {
        self.set_value(Self::selected_stroke_width_property(), value);
    }
    pub fn determine_stroke_width(&self) -> f64 {
        self.selected_stroke_width().clamp(
            self.min_stroke_width(),
            self.max_stroke_width().max(self.min_stroke_width()),
        )
    }
    pub fn selected_color(&self) -> Result<Color, InkError> {
        self.selected_brush()
            .and_then(|b| b.as_solid_color_brush().map(|s| s.color()))
            .ok_or(InkError::InvalidInput(
                "Ink pens require a selected solid-color brush",
            ))
    }
    fn update_selected_brush(&self) {
        let index = self.selected_brush_index();
        let items = self.palette().snapshot();
        // The source retains the previous brush for an invalid palette index.
        let Some(selected) = (index >= 0)
            .then(|| items.get(index as usize).cloned())
            .flatten()
        else {
            return;
        };
        let selected = Some(selected);
        self.set_and_raise(
            Self::selected_brush_property(),
            &self.selected_brush,
            selected.clone(),
        );
        let help = selected.and_then(|b| b.as_solid_color_brush().map(|s| color_name(s.color())));
        AutomationProperties::set_help_text(self, help.as_deref());
    }
}

#[repr(C)]
pub struct InkToolbarBallpointPenButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarBallpointPenButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarBallpointPenButton {
    new: InkToolbarBallpointPenButton::new
});
ferro_impl_classes!(InkToolbarBallpointPenButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarBallpointPenButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::BallpointPen);
        this.set_content(Some(Rc::new("Ballpoint pen".to_owned())));
        AutomationProperties::set_name(this, Some("Ballpoint pen"));
    }
}
impl InkToolbarPenButtonImpl for InkToolbarBallpointPenButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        Ok(InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width, width),
            ..InkDrawingAttributes::default()
        })
    }
}
impl InkToolbarBallpointPenButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarPencilButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarPencilButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarPencilButton {
    new: InkToolbarPencilButton::new
});
ferro_impl_classes!(InkToolbarPencilButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarPencilButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::Pencil);
        this.set_content(Some(Rc::new("Pencil".to_owned())));
        AutomationProperties::set_name(this, Some("Pencil"));
    }
}
impl InkToolbarPenButtonImpl for InkToolbarPencilButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        Ok(InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width, width),
            ..InkDrawingAttributes::create_for_pencil()
        })
    }
}
impl InkToolbarPencilButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarHighlighterButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarHighlighterButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarHighlighterButton {
    new: InkToolbarHighlighterButton::new
});
ferro_impl_classes!(InkToolbarHighlighterButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarHighlighterButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::Highlighter);
        this.set_content(Some(Rc::new("Highlighter".to_owned())));
        AutomationProperties::set_name(this, Some("Highlighter"));
        this.set_palette(default_palette(true));
        this.set_selected_stroke_width(16.0);
    }
}
impl InkToolbarPenButtonImpl for InkToolbarHighlighterButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        Ok(InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width / 3.0, width),
            pen_tip: PenTipShape::Rectangle,
            draw_as_highlighter: true,
            ..InkDrawingAttributes::default()
        })
    }
}
impl InkToolbarHighlighterButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Custom pen construction uses a real virtual dispatch, so subclasses can define the nib.
#[repr(C)]
pub struct InkToolbarCustomPen {
    base: FerroObject,
}
ferro_class! { InkToolbarCustomPen: FerroObject, virtuals InkToolbarCustomPenImpl: FerroObjectImpl {
    fn create_ink_drawing_attributes_core(this, brush: Rc<dyn IBrush>, stroke_width: f64) -> Result<InkDrawingAttributes,InkError>;
} }
ferro_class_info!(InkToolbarCustomPen {
    new: InkToolbarCustomPen::new
});
ferro_impl_classes!(InkToolbarCustomPen: FerroObjectImpl);
impl InkToolbarCustomPenImpl for InkToolbarCustomPen {
    fn create_ink_drawing_attributes_core(
        _this: &Self,
        brush: Rc<dyn IBrush>,
        stroke_width: f64,
    ) -> Result<InkDrawingAttributes, InkError> {
        let color = brush
            .as_solid_color_brush()
            .ok_or(InkError::InvalidInput(
                "Custom ink pen requires a solid color or a custom factory",
            ))?
            .color();
        let attrs = InkDrawingAttributes {
            color,
            size: Size::new(stroke_width, stroke_width),
            ..InkDrawingAttributes::default()
        };
        attrs.validate()?;
        Ok(attrs)
    }
}
impl InkToolbarCustomPen {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn create_ink_drawing_attributes(
        &self,
        brush: Rc<dyn IBrush>,
        stroke_width: f64,
    ) -> Result<InkDrawingAttributes, InkError> {
        self.create_ink_drawing_attributes_core(brush, stroke_width)
    }
}
#[repr(C)]
pub struct InkToolbarCustomPenButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarCustomPenButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarCustomPenButton {
    new: InkToolbarCustomPenButton::new
});
ferro_impl_classes!(InkToolbarCustomPenButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarCustomPenButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::CustomPen);
        this.set_content(Some(Rc::new("Custom pen".to_owned())));
        AutomationProperties::set_name(this, Some("Custom pen"));
    }
}
impl InkToolbarPenButtonImpl for InkToolbarCustomPenButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        match this.custom_pen() {
            Some(pen) => pen.create_ink_drawing_attributes(
                this.selected_brush()
                    .ok_or(InkError::InvalidInput("No selected brush"))?,
                this.determine_stroke_width(),
            ),
            None => {
                let width = this.determine_stroke_width();
                Ok(InkDrawingAttributes {
                    color: this.selected_color()?,
                    size: Size::new(width, width),
                    ..InkDrawingAttributes::default()
                })
            }
        }
    }
}
ferro_properties! {impl InkToolbarCustomPenButton{
    pub fn custom_pen_property()->StyledProperty<Option<Ref<InkToolbarCustomPen>>>{FerroProperty::register::<InkToolbarCustomPenButton,_>("CustomPen",None)}
    pub fn configuration_content_property()->StyledProperty<Option<Ref<Control>>>{FerroProperty::register::<InkToolbarCustomPenButton,_>("ConfigurationContent",None)}
}}
impl InkToolbarCustomPenButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn custom_pen(&self) -> Option<Ref<InkToolbarCustomPen>> {
        self.get_value(Self::custom_pen_property())
    }
    pub fn set_custom_pen(&self, value: Option<Ref<InkToolbarCustomPen>>) {
        self.set_value(Self::custom_pen_property(), value);
    }
    pub fn configuration_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::configuration_content_property())
    }
    pub fn set_configuration_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::configuration_content_property(), value);
    }
}

#[repr(C)]
pub struct InkToolbarEraserButton {
    base: InkToolbarToolButton,
}
ferro_class!(InkToolbarEraserButton:InkToolbarToolButton);
ferro_class_info!(InkToolbarEraserButton {
    new: InkToolbarEraserButton::new
});
ferro_impl_classes!(InkToolbarEraserButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarEraserButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::Eraser);
        this.set_content(Some(Rc::new("Eraser".to_owned())));
        AutomationProperties::set_name(this, Some("Eraser"));
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_clear_all_visible_property().as_property() {
            this.set_is_extension_glyph_shown(this.is_clear_all_visible());
        }
    }
}
impl InkToolbarEraserButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToolButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarCustomToolButton {
    base: InkToolbarToolButton,
}
ferro_class!(InkToolbarCustomToolButton:InkToolbarToolButton);
ferro_class_info!(InkToolbarCustomToolButton {
    new: InkToolbarCustomToolButton::new
});
ferro_impl_classes!(InkToolbarCustomToolButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarCustomToolButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::CustomTool);
        this.set_content(Some(Rc::new("Custom tool".to_owned())));
        AutomationProperties::set_name(this, Some("Custom tool"));
    }
}
impl InkToolbarCustomToolButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToolButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
ferro_properties! {impl InkToolbarEraserButton{pub fn is_clear_all_visible_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarEraserButton,_>("IsClearAllVisible",true)}}}
impl InkToolbarEraserButton {
    pub fn is_clear_all_visible(&self) -> bool {
        self.get_value(Self::is_clear_all_visible_property())
    }
    pub fn set_is_clear_all_visible(&self, value: bool) {
        self.set_value(Self::is_clear_all_visible_property(), value);
    }
}
ferro_properties! {impl InkToolbarCustomToolButton{pub fn configuration_content_property()->StyledProperty<Option<Ref<Control>>>{FerroProperty::register::<InkToolbarCustomToolButton,_>("ConfigurationContent",None)}}}
impl InkToolbarCustomToolButton {
    pub fn configuration_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::configuration_content_property())
    }
    pub fn set_configuration_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::configuration_content_property(), value);
    }
}

#[repr(C)]
pub struct InkToolbarToggleButton {
    base: CheckBox,
    toggle_kind: Cell<InkToolbarToggle>,
}
ferro_class!(InkToolbarToggleButton:CheckBox);
ferro_class_info!(InkToolbarToggleButton{new:InkToolbarToggleButton::new,markup:{properties:[ToggleKind:InkToolbarToggle{get:InkToolbarToggleButton::toggle_kind}],}});
ferro_impl_classes!(InkToolbarToggleButton:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl StyledElementImpl for InkToolbarToggleButton {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        CheckBox::TYPE
    }
}
impl InkToolbarToggleButton {
    pub fn construct() -> Self {
        Self {
            base: CheckBox::construct(),
            toggle_kind: Cell::new(InkToolbarToggle::Custom),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn toggle_kind(&self) -> InkToolbarToggle {
        self.toggle_kind.get()
    }
}
#[repr(C)]
pub struct InkToolbarCustomToggleButton {
    base: InkToolbarToggleButton,
}
ferro_class!(InkToolbarCustomToggleButton:InkToolbarToggleButton);
ferro_class_info!(InkToolbarCustomToggleButton {
    new: InkToolbarCustomToggleButton::new
});
ferro_impl_classes!(InkToolbarCustomToggleButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarCustomToggleButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.toggle_kind.set(InkToolbarToggle::Custom);
        this.set_content(Some(Rc::new("Custom toggle".to_owned())));
        AutomationProperties::set_name(this, Some("Custom toggle"));
    }
}
impl InkToolbarCustomToggleButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToggleButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarRulerButton {
    base: InkToolbarToggleButton,
}
ferro_class!(InkToolbarRulerButton:InkToolbarToggleButton);
ferro_class_info!(InkToolbarRulerButton {
    new: InkToolbarRulerButton::new
});
ferro_impl_classes!(InkToolbarRulerButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarRulerButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.toggle_kind.set(InkToolbarToggle::Ruler);
        this.set_content(Some(Rc::new("Ruler".to_owned())));
        AutomationProperties::set_name(this, Some("Ruler"));
    }
}
impl InkToolbarRulerButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToggleButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarMenuButton {
    base: ToggleButton,
}
ferro_class!(InkToolbarMenuButton:ToggleButton);
ferro_class_info!(InkToolbarMenuButton{new:InkToolbarMenuButton::new,markup:{properties:[MenuKind:InkToolbarMenuKind{get:InkToolbarMenuButton::menu_kind}],}});
ferro_impl_classes!(InkToolbarMenuButton: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl, ToggleButtonImpl);
impl ControlImpl for InkToolbarMenuButton {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        super::InkToolbarMenuButtonAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarMenuButton {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ToggleButton::TYPE
    }
}
ferro_properties! {impl InkToolbarMenuButton{pub fn is_extension_glyph_shown_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarMenuButton,_>("IsExtensionGlyphShown",true)}}}
impl InkToolbarMenuButton {
    pub fn construct() -> Self {
        Self {
            base: ToggleButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn menu_kind(&self) -> InkToolbarMenuKind {
        InkToolbarMenuKind::Stencil
    }
    pub fn is_extension_glyph_shown(&self) -> bool {
        self.get_value(Self::is_extension_glyph_shown_property())
    }
    pub fn set_is_extension_glyph_shown(&self, value: bool) {
        self.set_value(Self::is_extension_glyph_shown_property(), value);
    }
}

#[repr(C)]
pub struct InkToolbarStencilButton {
    base: InkToolbarMenuButton,
    presenter: RefCell<Option<Rc<super::InkPresenter>>>,
    ruler: RefCell<Option<Rc<super::InkPresenterRuler>>>,
    protractor: RefCell<Option<Rc<super::InkPresenterProtractor>>>,
}
ferro_class!(InkToolbarStencilButton:InkToolbarMenuButton);
ferro_class_info!(InkToolbarStencilButton {
    new: InkToolbarStencilButton::new
});
ferro_impl_classes!(InkToolbarStencilButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarStencilButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_content(Some(Rc::new("Stencil".to_owned())));
        AutomationProperties::set_name(this, Some("Stencil"));
    }
}
ferro_properties! {impl InkToolbarStencilButton{
    pub fn ruler_property()->DirectProperty<InkToolbarStencilButton,Option<Rc<super::InkPresenterRuler>>>{FerroProperty::register_direct::<InkToolbarStencilButton,_>("Ruler",InkToolbarStencilButton::ruler,None,None)}
    pub fn protractor_property()->DirectProperty<InkToolbarStencilButton,Option<Rc<super::InkPresenterProtractor>>>{FerroProperty::register_direct::<InkToolbarStencilButton,_>("Protractor",InkToolbarStencilButton::protractor,None,None)}
    pub fn selected_stencil_property()->StyledProperty<InkToolbarStencilKind>{FerroProperty::register::<InkToolbarStencilButton,_>("SelectedStencil",InkToolbarStencilKind::Ruler)}
    pub fn is_ruler_item_visible_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarStencilButton,_>("IsRulerItemVisible",true)}
    pub fn is_protractor_item_visible_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarStencilButton,_>("IsProtractorItemVisible",true)}
}}
impl InkToolbarStencilButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarMenuButton::construct(),
            presenter: RefCell::new(None),
            ruler: RefCell::new(None),
            protractor: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_presenter(&self, value: Option<Rc<super::InkPresenter>>) {
        let ruler = value.as_ref().map(|p| p.ruler());
        let protractor = value.as_ref().map(|p| p.protractor());
        *self.presenter.borrow_mut() = value;
        self.set_and_raise(Self::ruler_property(), &self.ruler, ruler);
        self.set_and_raise(Self::protractor_property(), &self.protractor, protractor);
    }
    pub fn ruler(&self) -> Option<Rc<super::InkPresenterRuler>> {
        self.ruler.borrow().clone()
    }
    pub fn protractor(&self) -> Option<Rc<super::InkPresenterProtractor>> {
        self.protractor.borrow().clone()
    }
    pub fn selected_stencil(&self) -> InkToolbarStencilKind {
        self.get_value(Self::selected_stencil_property())
    }
    pub fn set_selected_stencil(&self, value: InkToolbarStencilKind) {
        self.set_value(Self::selected_stencil_property(), value);
    }
    pub fn is_ruler_item_visible(&self) -> bool {
        self.get_value(Self::is_ruler_item_visible_property())
    }
    pub fn set_is_ruler_item_visible(&self, value: bool) {
        self.set_value(Self::is_ruler_item_visible_property(), value);
    }
    pub fn is_protractor_item_visible(&self) -> bool {
        self.get_value(Self::is_protractor_item_visible_property())
    }
    pub fn set_is_protractor_item_visible(&self, value: bool) {
        self.set_value(Self::is_protractor_item_visible_property(), value);
    }
}
