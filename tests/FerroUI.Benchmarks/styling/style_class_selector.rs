//! Attaching a style whose selector names a type and a class to a new
//! control fifty times in one styling pass; then, after the pass, adding and
//! removing the class (`apply_toggle`) or detaching the styles
//! (`apply_detach`).
//!
//! The styling pass of the value store of the control
//! (`GetValueStore().BeginStyling()`, `EndStyling()`) is reached through the
//! `testing` feature of the base crate (`ferroui_base::styling::testing`), as
//! upstream's benchmark project reaches it through its access to the
//! internals of the framework.

use crate::harness::Registry;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::testing::{begin_styling, end_styling, try_attach};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};

pub struct StyleClassSelector {
    style: Ref<Style>,
}

impl StyleClassSelector {
    pub fn new() -> Self {
        StyleClassSelectorTestClass::TYPE.ensure_class_init();

        let style = Style::with_setters(
            Selectors::of_type::<StyleClassSelectorTestClass>().class("foo"),
            [Setter::new(StyleClassSelectorTestClass::string_property(), Some("foo".to_string()))],
        );

        Self { style }
    }

    pub fn apply(&self) {
        let target = StyleClassSelectorTestClass::new();

        begin_styling(&target);

        for _ in 0..50 {
            try_attach(&self.style, &target, None);
        }

        end_styling(&target);
    }

    pub fn apply_toggle(&self) {
        let target = StyleClassSelectorTestClass::new();

        begin_styling(&target);

        for _ in 0..50 {
            try_attach(&self.style, &target, None);
        }

        end_styling(&target);

        target.classes().add("foo");
        target.classes().remove("foo");
    }

    pub fn apply_detach(&self) {
        let target = StyleClassSelectorTestClass::new();

        begin_styling(&target);

        for _ in 0..50 {
            try_attach(&self.style, &target, None);
        }

        end_styling(&target);

        target.detach_styles();
    }
}

/// The control of the benchmark (`TestClass` upstream): a control with a
/// text property.
#[repr(C)]
struct StyleClassSelectorTestClass {
    base: Control,
}

ferro_class!(StyleClassSelectorTestClass: Control);
ferro_impl_classes!(
    StyleClassSelectorTestClass: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl StyleClassSelectorTestClass {
        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<StyleClassSelectorTestClass, _>("String", None)
        }
    }
}

impl StyleClassSelectorTestClass {
    fn construct() -> Self {
        Self { base: Control::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn detach_styles(&self) {
        self.invalidate_styles(true);
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "StyleClassSelector");
    class.benchmark("apply", "", StyleClassSelector::new, |b| b.apply()).operations_per_invoke(50);
    class.benchmark("apply_toggle", "", StyleClassSelector::new, |b| b.apply_toggle()).operations_per_invoke(50);
    class.benchmark("apply_detach", "", StyleClassSelector::new, |b| b.apply_detach()).operations_per_invoke(50);
}

#[cfg(test)]
mod tests {
    #[test]
    fn style_class_selector() {
        crate::harness::smoke_class(super::register, "StyleClassSelector");
    }
}
