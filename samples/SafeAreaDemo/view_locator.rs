//! Port of `ViewLocator.cs`: the data template of the application that builds the view of a
//! view model.

use crate::view_models::is_view_model_base;
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::MarkupType;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, TypeInfo};
use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
use ferroui_controls::{Control, TextBlock};
use std::rc::{Rc, Weak};

/// Builds the view of a view model: the class whose full name is the full name of the class of
/// the view model with `ViewModel` replaced by `View`.
pub struct ViewLocator {
    this: Weak<ViewLocator>,
}

impl PartialEq for ViewLocator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ViewLocator {
    pub fn new() -> Rc<ViewLocator> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The locator as the data template contract.
    pub fn as_data_template(&self) -> Rc<dyn IDataTemplate> {
        self.this.upgrade().expect("the locator is alive while it is used")
    }

    /// `data.GetType().FullName`: the full name of the markup type of the value (the name of
    /// its Rust type when markup does not know it).
    fn full_name_of(data: &BoxedValue) -> String {
        let value_type = ValueType::of_value(&**data);
        match MarkupType::find_by_handle(value_type.id()) {
            Some(markup_type) => markup_type.full_name(),
            None => value_type.name().to_string(),
        }
    }

    /// `Type.GetType(name)`: the class of the full name, among the classes markup knows.
    fn find_type(name: &str) -> Option<&'static TypeInfo> {
        let (namespace, name) = name.rsplit_once('.').unwrap_or(("", name));
        TypeInfo::find(namespace, name)
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ViewLocator {
    fn build(&self, data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        let data = data.as_ref()?;

        let name = Self::full_name_of(data).replace("ViewModel", "View");
        let type_ = Self::find_type(&name);

        if let Some(type_) = type_ {
            // `(Control)Activator.CreateInstance(type)!`.
            let instance = type_.create_instance().unwrap_or_else(|| panic!("No parameterless constructor defined for type '{name}'."));
            return Some(instance.cast::<Control>().unwrap_or_else(|| panic!("Unable to cast an object of type '{name}' to type 'Control'.")));
        }

        let text_block = TextBlock::new();
        text_block.set_text(Some(&name));
        Some(text_block.upcast())
    }
}

impl IDataTemplate for ViewLocator {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        data.is_some_and(is_view_model_base)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

ferro_markup_type!(class ViewLocator {
    this: Rc<ViewLocator>,
    handles: [ViewLocator, Rc<ViewLocator>, Option<Rc<ViewLocator>>],
    interfaces: [Rc<dyn IDataTemplate>],
    constructors: [() => ViewLocator::new],
});
