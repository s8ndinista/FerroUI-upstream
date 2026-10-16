//! Attaching simple styles to a new control in one styling pass: styles
//! whose selector matches the control and styles whose selector does not,
//! in every combination of 1, 5 and 50 of each.
//!
//! The styling pass of the value store of the control
//! (`GetValueStore().BeginStyling()`, `EndStyling()`) is reached through the
//! `testing` feature of the base crate (`ferroui_base::styling::testing`), as
//! upstream's benchmark project reaches it through its access to the
//! internals of the framework.

use crate::harness::Registry;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
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

pub struct StyleApply {
    styles: Vec<Ref<Style>>,
}

impl StyleApply {
    pub fn new(matching_styles: usize, non_matching_styles: usize) -> Self {
        StyleApplyTestClass::TYPE.ensure_class_init();

        let mut styles = Vec::new();

        for _ in 0..matching_styles {
            styles.push(Style::with_setters(
                Selectors::of_type::<StyleApplyTestClass>(),
                [Setter::new(StyleApplyTestClass::string_property(), Some("foo".to_string()))],
            ));
        }

        for _ in 0..non_matching_styles {
            styles.push(Style::with_setters(
                Selectors::of_type::<StyleApplyTestClass2>().class("missing"),
                [Setter::new(StyleApplyTestClass::string_property(), Some("foo".to_string()))],
            ));
        }

        Self { styles }
    }

    pub fn apply_simple_styles(&self) {
        let target = StyleApplyTestClass::new();

        begin_styling(&target);

        for style in &self.styles {
            try_attach(style, &target, None);
        }

        end_styling(&target);
    }
}

/// The control of the benchmark (`TestClass` upstream): a control with a
/// text property and eight properties of value types of growing size.
#[repr(C)]
struct StyleApplyTestClass {
    base: Control,
}

ferro_class!(StyleApplyTestClass: Control);
ferro_impl_classes!(
    StyleApplyTestClass: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl StyleApplyTestClass {
        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<StyleApplyTestClass, _>("String", None)
        }

        pub fn struct1_property() -> StyledProperty<Struct1> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct1", Struct1::default())
        }

        pub fn struct2_property() -> StyledProperty<Struct2> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct2", Struct2::default())
        }

        pub fn struct3_property() -> StyledProperty<Struct3> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct3", Struct3::default())
        }

        pub fn struct4_property() -> StyledProperty<Struct4> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct4", Struct4::default())
        }

        pub fn struct5_property() -> StyledProperty<Struct5> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct5", Struct5::default())
        }

        pub fn struct6_property() -> StyledProperty<Struct6> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct6", Struct6::default())
        }

        pub fn struct7_property() -> StyledProperty<Struct7> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct7", Struct7::default())
        }

        pub fn struct8_property() -> StyledProperty<Struct8> {
            FerroProperty::register::<StyleApplyTestClass, _>("Struct8", Struct8::default())
        }
    }
}

impl StyleApplyTestClass {
    fn construct() -> Self {
        Self { base: Control::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The type the selector of the styles that do not match names
/// (`TestClass2` upstream).
#[repr(C)]
struct StyleApplyTestClass2 {
    base: Control,
}

ferro_class!(StyleApplyTestClass2: Control);
ferro_impl_classes!(
    StyleApplyTestClass2: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "StyleApply");
    for matching_styles in [1, 5, 50] {
        for non_matching_styles in [1, 5, 50] {
            class.benchmark(
                "apply_simple_styles",
                format!("MatchingStyles={matching_styles}, NonMatchingStyles={non_matching_styles}"),
                move || StyleApply::new(matching_styles, non_matching_styles),
                |b| b.apply_simple_styles(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn style_apply() {
        crate::harness::smoke_class(super::register, "StyleApply");
    }
}
