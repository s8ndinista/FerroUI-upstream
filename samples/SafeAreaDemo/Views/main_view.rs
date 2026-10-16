//! Port of `Views/MainView.xaml.cs`: the class of the document `Views/MainView.xaml`.

use crate::markup::xaml_class;
use crate::view_models::MainViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, ControlImplExt, TopLevel, UserControl};

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
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(MainView { new: MainView::new });
xaml_class!(MainView, "/Views/MainView.xaml");

impl ControlImpl for MainView {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        let insets_manager = TopLevel::get_top_level(Some(this)).and_then(|top_level| top_level.insets_manager());
        let input_pane = TopLevel::get_top_level(Some(this)).and_then(|top_level| top_level.input_pane());
        let view_model = MainViewModel::new();
        view_model.initialize(this, insets_manager, input_pane);
        this.set_data_context(Some(view_model as BoxedValue));
    }
}

impl MainView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
