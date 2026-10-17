use super::{
    Block, BlockImpl, IInlineHost, Inline, InlineCollection, TextElementImpl, TextElementImplExt,
};
use crate::Control;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::metadata::IAddChild;
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, IntoRef,
    Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, ferro_class, instantiate,
};
use std::rc::Rc;

/// A paragraph of structured inline content, including styled spans and embedded controls.
#[repr(C)]
pub struct Paragraph {
    base: Block,
    inlines: InlineCollection,
}

ferro_class!(Paragraph: Block);
ferroui_base::ferro_class_info!(Paragraph {
    new: Paragraph::new,
    interfaces: [
        Rc<dyn IAddChild<Ref<Inline>>> => Paragraph::as_add_child_of_inline,
        Rc<dyn IAddChild<Ref<Control>>> => Paragraph::as_add_child_of_control,
        Rc<dyn IAddChild<String>> => Paragraph::as_add_child_of_string,
    ],
    markup: {
        content: Inlines,
        properties: [Inlines: InlineCollection { get: Paragraph::inlines }],
    },
});

ferroui_base::ferro_impl_classes!(Paragraph: StyledElementImpl);

impl FerroObjectImpl for Paragraph {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.inlines.set_logical_children(Some(this));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::text_indent_property().as_property() {
            if let Some(host) = this.inline_host() {
                host.invalidate();
            }
        }
    }
}

impl TextElementImpl for Paragraph {
    fn on_inline_host_changed(
        this: &Self,
        old_value: Option<&Rc<dyn IInlineHost>>,
        new_value: Option<&Rc<dyn IInlineHost>>,
    ) {
        Self::parent_on_inline_host_changed(this, old_value, new_value);
        this.inlines.set_inline_host(new_value.cloned());
    }
}

impl BlockImpl for Paragraph {
    fn text(this: &Self) -> String {
        this.inlines.text().unwrap_or_default()
    }

    fn build_text_runs(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        for inline in this.inlines.snapshot().iter() {
            inline.build_text_run(text_runs);
        }
    }

    fn measure_embedded_controls(this: &Self, available_size: Size) -> bool {
        let mut changed = false;
        for inline in this.inlines.snapshot().iter() {
            changed |= inline.measure_embedded_controls(available_size);
        }
        changed
    }
}

ferroui_base::ferro_properties! {
    impl Paragraph {
        pub fn text_indent_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("TextIndent",
                StyledPropertyOptions::new(0.0).validate(|value: &f64| value.is_finite()))
        }
    }
}

impl Paragraph {
    pub fn construct() -> Self {
        Self {
            base: Block::construct(),
            inlines: InlineCollection::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_text(text: &str) -> Ref<Self> {
        let paragraph = Self::new();
        paragraph.inlines.add_text(text);
        paragraph
    }

    pub fn inlines(&self) -> InlineCollection {
        self.inlines.clone()
    }

    pub fn add_child(&self, child: impl IntoRef<Inline>) {
        self.inlines.add(child);
    }

    pub fn add_child_text(&self, text: &str) {
        self.inlines.add_text(text);
    }

    pub fn add_child_control(&self, child: impl IntoRef<Control>) {
        self.inlines.add_control(child);
    }

    pub fn text_indent(&self) -> f64 {
        self.get_value(Self::text_indent_property())
    }

    pub fn set_text_indent(&self, value: f64) {
        self.set_value(Self::text_indent_property(), value);
    }

    pub fn as_add_child_of_inline(this: Ref<Self>) -> Rc<dyn IAddChild<Ref<Inline>>> {
        Rc::new(ParagraphAddChild(this))
    }

    pub fn as_add_child_of_control(this: Ref<Self>) -> Rc<dyn IAddChild<Ref<Control>>> {
        Rc::new(ParagraphAddChild(this))
    }

    pub fn as_add_child_of_string(this: Ref<Self>) -> Rc<dyn IAddChild<String>> {
        Rc::new(ParagraphAddChild(this))
    }
}

struct ParagraphAddChild(Ref<Paragraph>);

impl IAddChild<Ref<Inline>> for ParagraphAddChild {
    fn add_child(&self, value: Ref<Inline>) {
        self.0.add_child(value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const Paragraph as usize
    }
}

impl IAddChild<Ref<Control>> for ParagraphAddChild {
    fn add_child(&self, value: Ref<Control>) {
        self.0.add_child_control(value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const Paragraph as usize
    }
}

impl IAddChild<String> for ParagraphAddChild {
    fn add_child(&self, value: String) {
        self.0.add_child_text(&value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const Paragraph as usize
    }
}
