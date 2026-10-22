//! Port of `Diagnostics/Views/TreePageView.xaml.cs`: the class of the
//! document `TreePageView.xaml`.

use crate::diagnostics::controls::ControlHighlightAdorner;
use crate::diagnostics::view_models::{TreeNode, TreePageViewModel};
use crate::diagnostics::DevToolsDataFormats;
use ferroui_base::input::{DataTransfer, DataTransferItem, IAsyncDataTransfer, InputElementImpl, PointerEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TopLevel, TreeView, TreeViewItem, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

/// The view of a tree page of the developer tools: the tree of the nodes,
/// with a highlight of the control of the node under the pointer, and the
/// details of the control of the selected node.
#[repr(C)]
pub struct TreePageView {
    base: UserControl,
    hovered: RefCell<Option<Ref<TreeViewItem>>>,
    tree: RefCell<Option<Ref<TreeView>>>,
    adorner: RefCell<Option<Rc<dyn IDisposable>>>,
    clipboard_copy_requested: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(TreePageView: UserControl);
ferro_impl_classes!(
    TreePageView: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(TreePageView {
    new: TreePageView::new,
    markup: {
        methods: [
            fn UpdateAdorner(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TreePageView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerEventArgs>() {
                        this.update_adorner(&sender, e)
                    }
                },
            fn RemoveAdorner(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TreePageView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerEventArgs>() {
                        this.remove_adorner(&sender, e)
                    }
                },
        ],
    },
});

impl FerroObjectImpl for TreePageView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == StyledElement::data_context_property().as_property() {
            // `oldViewModel.ClipboardCopyRequested -= OnClipboardCopyRequested`: the handler was
            // added to the old view model, through the subscription kept here.
            let old_view_model = change
                .get_old_value::<Option<BoxedValue>>()
                .flatten()
                .and_then(|old| from_markup_value::<Rc<TreePageViewModel>>(&Some(old)));
            if old_view_model.is_some() {
                let subscription = this.clipboard_copy_requested.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
            }

            let new_view_model = from_markup_value::<Rc<TreePageViewModel>>(&change.get_new_value::<Option<BoxedValue>>());
            if let Some(new_view_model) = new_view_model {
                let weak = this.to_ref().downgrade();
                let subscription = new_view_model.clipboard_copy_requested(move |selector: &str| {
                    if let Some(this) = weak.upgrade() {
                        this.on_clipboard_copy_requested(selector);
                    }
                });
                *this.clipboard_copy_requested.borrow_mut() = Some(subscription);
            }
        }
    }
}

impl TreePageView {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/TreePageView.xaml";

    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            hovered: RefCell::new(None),
            tree: RefCell::new(None),
            adorner: RefCell::new(None),
            clipboard_copy_requested: RefCell::new(None),
        }
    }

    /// Creates the view and populates it from its document
    /// (`InitializeComponent()`: the compiled markup of the document, which
    /// the build of the crate generates).
    ///
    /// # Panics
    /// Panics when the document fails to load (an exception of the
    /// constructor in the original).
    pub fn new() -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);
        *this.tree.borrow_mut() = Some(this.get_control::<TreeView>("tree"));
        this
    }

    /// `InitializeComponent()`.
    fn initialize_component(this: &Ref<Self>) {
        if let Err(error) = crate::compiled_diagnostics_views_tree_page_view::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }

    /// `UpdateAdorner`: highlights the control of the node under the
    /// pointer.
    pub fn update_adorner(&self, _sender: &Option<BoxedValue>, e: &PointerEventArgs) {
        let Some(source) = e.source().and_then(|source| source.cast::<StyledElement>()) else {
            return;
        };

        let item = source.find_logical_ancestor_of_type::<TreeViewItem>(false);
        if item == *self.hovered.borrow() {
            return;
        }

        let adorner = self.adorner.borrow().clone();
        if let Some(adorner) = adorner {
            adorner.dispose();
        }

        // Deviation (DEVIATIONS.md, Developer tools): `TreeViewItem::tree_view_owner` is internal
        // to the controls crate; the owner is found as it finds it, as the nearest logical ancestor
        // that is a tree view.
        let tree = self.tree.borrow().clone();
        let tree_view_owner = item.as_ref().and_then(|item| item.find_logical_ancestor_of_type::<TreeView>(false));
        let Some(item) = item.filter(|_| tree_view_owner == tree) else {
            *self.hovered.borrow_mut() = None;
            return;
        };

        *self.hovered.borrow_mut() = Some(item.clone());

        let visual = from_markup_value::<Rc<TreeNode>>(&item.data_context()).and_then(|node| node.visual().cast::<Visual>());
        let should_visualize_margin_padding = from_markup_value::<Rc<TreePageViewModel>>(&self.data_context())
            .and_then(|view_model| view_model.main_view())
            .map(|main_view| main_view.should_visualize_margin_padding());
        let (Some(visual), Some(should_visualize_margin_padding)) = (visual, should_visualize_margin_padding) else {
            return;
        };

        *self.adorner.borrow_mut() = ControlHighlightAdorner::add_visual(&visual, should_visualize_margin_padding);
    }

    /// `RemoveAdorner`: removes the highlight when the pointer leaves the
    /// tree.
    fn remove_adorner(&self, _sender: &Option<BoxedValue>, _e: &PointerEventArgs) {
        let adorner = self.adorner.borrow_mut().take();
        if let Some(adorner) = adorner {
            adorner.dispose();
        }
    }

    /// Copies a selector to the clipboard: as text, without the parts in
    /// braces, and in the format of selectors of the developer tools.
    fn on_clipboard_copy_requested(&self, selector: &str) {
        let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard()) else {
            return;
        };

        let data_transfer_item = DataTransferItem::new();
        let text = Self::to_text(selector);
        data_transfer_item.set_text(Some(text.as_str()));
        data_transfer_item.set(&DevToolsDataFormats::selector(), Some(selector.to_string()));

        let data_transfer = DataTransfer::new();
        data_transfer.add(data_transfer_item);
        let data_transfer: Rc<dyn IAsyncDataTransfer> = data_transfer;

        // The original does not await the task.
        let _ = Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let _ = clipboard.set_data_async(Some(data_transfer)).await;
        });
    }

    /// The text of a selector: the selector without the parts in braces
    /// (the assemblies of the types).
    pub(crate) fn to_text(text: &str) -> String {
        let mut sb = String::new();
        let mut buffer_start_index: Option<usize> = None;
        for c in text.chars() {
            match c {
                '{' => {
                    buffer_start_index = Some(sb.len());
                }
                '}' if buffer_start_index.is_some() => {
                    let start = buffer_start_index.unwrap_or_default();
                    sb.truncate(start);
                    buffer_start_index = Some(sb.len());
                }
                _ => sb.push(c),
            }
        }
        sb
    }
}
