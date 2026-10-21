//! Pen palette and width editing built from the existing FerroUI controls.
use super::color_names::color_name;
use super::{InkToolbarFlyoutItemKind, InkToolbarPenButton};
use crate::automation::AutomationProperties;
use crate::primitives::{RangeBase, TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use crate::{
    Border, ButtonImpl, ContentControlImpl, Control, ControlImpl, PanelImpl, Slider, SliderImpl,
    StackPanel, StackPanelImpl, WrapPanel,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct InkToolbarStrokeWidthSlider {
    base: Slider,
}
ferro_class!(InkToolbarStrokeWidthSlider:Slider);
ferro_class_info!(InkToolbarStrokeWidthSlider {
    new: InkToolbarStrokeWidthSlider::new
});
ferro_impl_classes!(InkToolbarStrokeWidthSlider: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, SliderImpl);
impl ControlImpl for InkToolbarStrokeWidthSlider {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        super::InkToolbarStrokeWidthSliderAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarStrokeWidthSlider {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        Slider::TYPE
    }
}
impl FerroObjectImpl for InkToolbarStrokeWidthSlider {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_small_change(0.5);
        this.set_large_change(2.0);
        AutomationProperties::set_name(this, Some("Stroke width"));
    }
}
impl InkToolbarStrokeWidthSlider {
    pub fn construct() -> Self {
        Self {
            base: Slider::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[repr(C)]
pub struct InkToolbarPenConfigurationControl {
    base: StackPanel,
    pen: RefCell<Option<Ref<InkToolbarPenButton>>>,
    pen_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    palette_subscription: RefCell<Option<(FerroList<Rc<dyn IBrush>>, u64)>>,
    palette_panel: RefCell<Option<Ref<WrapPanel>>>,
    slider: RefCell<Option<Ref<InkToolbarStrokeWidthSlider>>>,
    syncing: Cell<bool>,
}
ferro_class!(InkToolbarPenConfigurationControl:StackPanel);
ferro_class_info!(InkToolbarPenConfigurationControl {
    new: InkToolbarPenConfigurationControl::new
});
ferro_impl_classes!(InkToolbarPenConfigurationControl:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl FerroObjectImpl for InkToolbarPenConfigurationControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_orientation(Orientation::Vertical);
        this.set_spacing(8.0);
        let palette = WrapPanel::new();
        palette.set_max_width(240.0);
        let slider = InkToolbarStrokeWidthSlider::new();
        slider.set_min_width(200.0);
        let weak = this.to_ref().downgrade();
        slider.property_changed(move |change| {
            if change.property() != RangeBase::value_property().as_property() {
                return;
            }
            if let Some(config) = weak.upgrade() {
                if !config.syncing.get() {
                    if let Some(pen) = config.pen_button() {
                        pen.set_selected_stroke_width(change.get_new_value::<f64>());
                    }
                }
            }
        });
        this.children().add(palette.clone());
        this.children().add(slider.clone());
        *this.palette_panel.borrow_mut() = Some(palette);
        *this.slider.borrow_mut() = Some(slider);
        AutomationProperties::set_name(this, Some("Pen configuration"));
        this.bind_pen();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::pen_button_property().as_property() {
            this.bind_pen();
        }
    }
}
ferro_properties! {impl InkToolbarPenConfigurationControl{pub fn pen_button_property()->DirectProperty<InkToolbarPenConfigurationControl,Option<Ref<InkToolbarPenButton>>>{FerroProperty::register_direct::<InkToolbarPenConfigurationControl,_>("PenButton",InkToolbarPenConfigurationControl::pen_button,None,None)}}}
impl InkToolbarPenConfigurationControl {
    pub fn construct() -> Self {
        Self {
            base: StackPanel::construct(),
            pen: RefCell::new(None),
            pen_subscription: RefCell::new(None),
            palette_subscription: RefCell::new(None),
            palette_panel: RefCell::new(None),
            slider: RefCell::new(None),
            syncing: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn for_pen(pen: &InkToolbarPenButton) -> Ref<Self> {
        let result = Self::new();
        result.set_and_raise(Self::pen_button_property(), &result.pen, Some(pen.to_ref()));
        result
    }
    pub fn pen_button(&self) -> Option<Ref<InkToolbarPenButton>> {
        self.pen.borrow().clone()
    }
    pub fn stroke_width_slider(&self) -> Option<Ref<InkToolbarStrokeWidthSlider>> {
        self.slider.borrow().clone()
    }
    fn bind_pen(&self) {
        if let Some(subscription) = self.pen_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(pen) = self.pen_button() {
            let weak = self.to_ref().downgrade();
            let subscription = pen.property_changed(move |change| {
                if let Some(config) = weak.upgrade() {
                    if change.property() == InkToolbarPenButton::palette_property().as_property() {
                        config.bind_palette();
                    }
                    config.synchronize();
                }
            });
            *self.pen_subscription.borrow_mut() = Some(subscription);
        }
        self.bind_palette();
        self.synchronize();
    }
    fn bind_palette(&self) {
        if let Some((palette, token)) = self.palette_subscription.borrow_mut().take() {
            palette.remove_collection_changed(token);
        }
        if let Some(pen) = self.pen_button() {
            let palette = pen.palette();
            let weak = self.to_ref().downgrade();
            let token = palette.add_collection_changed(Rc::new(move |_| {
                if let Some(config) = weak.upgrade() {
                    config.rebuild_palette();
                }
            }));
            *self.palette_subscription.borrow_mut() = Some((palette, token));
        }
        self.rebuild_palette();
    }
    fn rebuild_palette(&self) {
        let panel = self.palette_panel.borrow().clone();
        let Some(panel) = panel else {
            return;
        };
        panel.children().clear();
        let Some(pen) = self.pen_button() else {
            return;
        };
        for (index, brush) in pen.palette().snapshot().iter().enumerate() {
            let item = InkToolbarFlyoutItem::new();
            item.set_kind(InkToolbarFlyoutItemKind::RadioCheck);
            item.set_width(36.0);
            item.set_height(36.0);
            let swatch = Border::new();
            swatch.set_width(20.0);
            swatch.set_height(20.0);
            swatch.set_background(Some(brush.clone()));
            item.set_content(Some(Rc::new(swatch.upcast::<Control>())));
            let name = brush
                .as_solid_color_brush()
                .map(|s| color_name(s.color()))
                .unwrap_or_else(|| format!("Brush {}", index + 1));
            AutomationProperties::set_name(&item, Some(&name));
            let weak = pen.downgrade();
            item.checked(move |_| {
                if let Some(pen) = weak.upgrade() {
                    pen.set_selected_brush_index(index as i32);
                }
            });
            panel.children().add(item);
        }
        self.synchronize();
    }
    fn synchronize(&self) {
        if self.syncing.replace(true) {
            return;
        }
        let pen = self.pen_button();
        let slider = self.slider.borrow().clone();
        let panel = self.palette_panel.borrow().clone();
        if let (Some(pen), Some(slider)) = (pen.as_ref(), slider) {
            slider.set_minimum(pen.min_stroke_width());
            slider.set_maximum(pen.max_stroke_width().max(pen.min_stroke_width()));
            slider.set_range_value(pen.selected_stroke_width());
        }
        if let (Some(pen), Some(panel)) = (pen, panel) {
            for (index, item) in panel.children().snapshot().iter().enumerate() {
                if let Some(item) = item.cast::<InkToolbarFlyoutItem>() {
                    item.set_is_checked(Some(index as i32 == pen.selected_brush_index()));
                }
            }
        }
        self.syncing.set(false);
    }
}

type ItemHandlers = Rc<HandlerList<dyn Fn(&InkToolbarFlyoutItem)>>;
#[repr(C)]
pub struct InkToolbarFlyoutItem {
    base: ToggleButton,
    checked: ItemHandlers,
    unchecked: ItemHandlers,
}
ferro_class!(InkToolbarFlyoutItem:ToggleButton);
ferro_class_info!(InkToolbarFlyoutItem {
    new: InkToolbarFlyoutItem::new
});
ferro_impl_classes!(InkToolbarFlyoutItem: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl);
impl ControlImpl for InkToolbarFlyoutItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        super::InkToolbarFlyoutItemAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarFlyoutItem {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ToggleButton::TYPE
    }
}
impl FerroObjectImpl for InkToolbarFlyoutItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == ToggleButton::is_checked_property().as_property() {
            let handlers = if this.is_checked() == Some(true) {
                &this.checked
            } else {
                &this.unchecked
            };
            for (_, handler) in handlers.snapshot().iter() {
                handler(this);
            }
        }
    }
}
impl ToggleButtonImpl for InkToolbarFlyoutItem {
    fn toggle(this: &Self) {
        let value = match this.kind() {
            InkToolbarFlyoutItemKind::Simple => return,
            InkToolbarFlyoutItemKind::Radio | InkToolbarFlyoutItemKind::RadioCheck => true,
            InkToolbarFlyoutItemKind::Check => this.is_checked() != Some(true),
        };
        this.set_current_value(ToggleButton::is_checked_property(), Some(value));
    }
}
ferro_properties! {impl InkToolbarFlyoutItem{pub fn kind_property()->StyledProperty<InkToolbarFlyoutItemKind>{FerroProperty::register::<InkToolbarFlyoutItem,_>("Kind",InkToolbarFlyoutItemKind::Simple)}}}
impl InkToolbarFlyoutItem {
    pub fn construct() -> Self {
        Self {
            base: ToggleButton::construct(),
            checked: Rc::new(HandlerList::new()),
            unchecked: Rc::new(HandlerList::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn kind(&self) -> InkToolbarFlyoutItemKind {
        self.get_value(Self::kind_property())
    }
    pub fn set_kind(&self, value: InkToolbarFlyoutItemKind) {
        self.set_value(Self::kind_property(), value);
    }
    pub fn checked(
        &self,
        handler: impl Fn(&InkToolbarFlyoutItem) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.checked, handler)
    }
    pub fn unchecked(
        &self,
        handler: impl Fn(&InkToolbarFlyoutItem) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.unchecked, handler)
    }
}
fn subscribe(
    list: &ItemHandlers,
    handler: impl Fn(&InkToolbarFlyoutItem) + 'static,
) -> Rc<dyn IDisposable> {
    let token = list.add(Rc::new(handler));
    let weak = Rc::downgrade(list);
    Disposable::create(move || {
        if let Some(list) = weak.upgrade() {
            list.remove(token);
        }
    })
}
