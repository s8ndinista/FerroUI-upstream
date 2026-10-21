//! WinUI Page's content and navigation lifecycle. See NOTICE.md for source attribution.

use super::{Frame, NavigatingCancelEventArgs, NavigationCacheMode, NavigationEventArgs};
use crate::primitives::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl, UserControl};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
    WeakRef,
};
use std::cell::{Cell, RefCell};

/// A WinUI-compatible navigable user control. This is separate from the framework's existing Page.
#[repr(C)]
pub struct Page {
    base: UserControl,
    frame: RefCell<Option<WeakRef<Frame>>>,
    cache_key: Cell<Option<&'static TypeInfo>>,
}

ferro_class! {
    Page: UserControl, virtuals PageImpl: ContentControlImpl {
        fn on_navigating_from(this, args: &NavigatingCancelEventArgs);
        fn on_navigated_from(this, args: &NavigationEventArgs);
        fn on_navigated_to(this, args: &NavigationEventArgs);
    }
}
ferro_class_info!(Page {
    new: Page::new,
    markup: { namespace: "FerroUI.Controls.WinUI", content: Content },
});
ferro_impl_classes!(Page: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);

impl StyledElementImpl for Page {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        UserControl::TYPE
    }
}

impl FerroObjectImpl for Page {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::navigation_cache_mode_property().as_property()
            && this.navigation_cache_mode() == NavigationCacheMode::Disabled
        {
            if let (Some(frame), Some(key)) = (this.frame(), this.cache_key.get()) {
                frame.remove_cached_page(key);
            }
        }
    }
}

impl PageImpl for Page {
    fn on_navigating_from(_this: &Self, _args: &NavigatingCancelEventArgs) {}
    fn on_navigated_from(_this: &Self, _args: &NavigationEventArgs) {}
    fn on_navigated_to(_this: &Self, _args: &NavigationEventArgs) {}
}

ferro_properties! {
    impl Page {
        pub fn navigation_cache_mode_property() -> StyledProperty<NavigationCacheMode> {
            FerroProperty::register::<Page, _>("NavigationCacheMode", NavigationCacheMode::Disabled)
        }
        pub fn frame_property() -> DirectProperty<Page, Option<Ref<Frame>>> {
            FerroProperty::register_direct::<Page, _>("Frame", Page::frame, None, None)
        }
    }
}

impl Page {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            frame: RefCell::new(None),
            cache_key: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn navigation_cache_mode(&self) -> NavigationCacheMode {
        self.get_value(Self::navigation_cache_mode_property())
    }
    pub fn set_navigation_cache_mode(&self, mode: NavigationCacheMode) {
        self.set_value(Self::navigation_cache_mode_property(), mode);
    }
    /// The containing frame, held weakly to avoid a cycle with frame content or cache entries.
    pub fn frame(&self) -> Option<Ref<Frame>> {
        self.frame.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn prepare(&self, frame: &Ref<Frame>, source_page_type: &'static TypeInfo) {
        self.cache_key.set(Some(source_page_type));
        let old = self.frame();
        if old.as_ref() != Some(frame) {
            *self.frame.borrow_mut() = Some(frame.downgrade());
            let new = Some(frame.clone());
            self.raise_property_changed(
                Self::frame_property().as_property(),
                Some(&old),
                &new,
                BindingPriority::LocalValue,
                true,
            );
        }
    }
}
