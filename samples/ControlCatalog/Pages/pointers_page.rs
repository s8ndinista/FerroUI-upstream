//! Port of `Pages/PointersPage.xaml.cs`: the class of the document
//! `Pages/PointersPage.xaml`.

use super::PointerCanvas;
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct PointersPage {
    base: ContentPage,
    // The field of the named element: assigned once the document is loaded.
    canvas: RefCell<Option<Ref<PointerCanvas>>>,
}

content_page_class!(PointersPage);
ferro_class_info!(PointersPage {
    new: PointersPage::new,
    markup: {
        methods: [
            fn Clear_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PointersPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.clear_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(PointersPage, "/Pages/PointersPage.xaml");

impl PointersPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), canvas: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        *this.canvas.borrow_mut() = this.find_control::<PointerCanvas>("Canvas");
        this
    }

    /// # Panics
    /// Panics if the canvas does not exist (a null reference in the managed original).
    fn clear_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let canvas = self.canvas.borrow().clone().expect("the pointer canvas");
        canvas.clear();
    }
}
