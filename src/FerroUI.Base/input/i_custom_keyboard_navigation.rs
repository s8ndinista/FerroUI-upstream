use super::{InputElement, NavigationDirection};
use crate::Ref;

/// Designates a control as handling its own keyboard navigation.
///
/// An input element that implements it returns itself from the
/// `as_custom_keyboard_navigation` virtual member of `InputElement`.
pub trait ICustomKeyboardNavigation {
    /// Supplies the direct children that participate in sequential focus
    /// navigation, in their logical item order. Returning `None` keeps the
    /// default visual-child enumeration.
    ///
    /// Virtualizing controls can keep recycled visuals attached for reuse.
    /// This hook excludes those visuals and preserves source order when the
    /// native focus manager enters, leaves, or wraps through the control.
    fn get_children_in_tab_focus_order(&self) -> Option<Vec<Ref<InputElement>>> {
        None
    }

    /// Gets the next element in the specified navigation direction.
    ///
    /// `element` is the element being navigated from. Returns a tuple
    /// consisting of:
    /// - a boolean indicating whether the request was handled. If false is
    ///   returned then custom navigation will be bypassed for the request.
    /// - if handled is true: the next element in the navigation direction,
    ///   or `None` if default navigation should continue outside the
    ///   element.
    fn get_next(
        &self,
        element: &Ref<InputElement>,
        direction: NavigationDirection,
    ) -> (bool, Option<Ref<InputElement>>);
}
