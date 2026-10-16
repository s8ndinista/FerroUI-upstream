//! Port of `MainView.xaml.cs`: the class of the document `MainView.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, UserControl};

#[repr(C)]
pub struct MainView {
    base: UserControl,
}

ferro_class!(MainView: UserControl);
ferro_impl_classes!(
    MainView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(MainView {
    new: MainView::new,
    markup: {
        methods: [
            fn ButtonCommand() => |this: &Ref<MainView>| this.button_command(),
        ],
    },
});
xaml_class!(MainView, "/MainView.xaml");

impl MainView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn button_command(&self) {
        println!("Button pressed");
    }
}
