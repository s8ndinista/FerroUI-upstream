use crate::{ControlImpl, RichTextBlock, RichTextBlockImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};

/// A continuation of another rich-text view, with its own width and height.
/// Link it with `RichTextBlock::set_overflow_content_target`; the document and
/// its shaped paragraph continuation are shared through weak view references.
#[repr(C)]
pub struct RichTextBlockOverflow {
    base: RichTextBlock,
}
ferro_class!(RichTextBlockOverflow: RichTextBlock);
ferroui_base::ferro_class_info!(RichTextBlockOverflow {
    new: RichTextBlockOverflow::new
});
ferroui_base::ferro_impl_classes!(RichTextBlockOverflow:
    FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, RichTextBlockImpl);

impl RichTextBlockOverflow {
    pub fn construct() -> Self {
        Self {
            base: RichTextBlock::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
