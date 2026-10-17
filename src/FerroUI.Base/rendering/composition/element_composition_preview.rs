use super::CompositionVisual;
use crate::Visual;
use std::rc::Rc;

/// Access to the composition visuals of the visuals of the visual tree.
pub struct ElementComposition;

impl ElementComposition {
    /// Gets the composition visual of a visual; `None` while the visual is
    /// not attached to a compositor.
    pub fn get_element_visual(visual: &Visual) -> Option<Rc<CompositionVisual>> {
        visual.composition_visual().map(|v| (*v).clone())
    }

    /// Sets a composition visual that is shown as the last child of the
    /// composition visual of `visual`.
    pub fn set_element_child_visual(visual: &Visual, composition_visual: Option<Rc<CompositionVisual>>) {
        if let (Some(child), Some(own)) = (&composition_visual, visual.composition_visual()) {
            if !Rc::ptr_eq(child.compositor(), own.compositor()) {
                panic!("Composition visuals belong to different compositor instances");
            }
        }

        // Un-parent the previous child here rather than waiting for the renderer to
        // reconcile the children, because that sync may never run again for this element:
        // a child cleared from on_detached_from_visual_tree is cleared before
        // detach_from_compositor discards the element's composition visual, and a child
        // left parented there cannot be attached anywhere else.
        if let Some(old_child) = visual.child_composition_visual() {
            if !composition_visual.as_ref().is_some_and(|child| Rc::ptr_eq(child, &old_child)) {
                if let Some(own) = visual.composition_visual() {
                    own.children().remove(&old_child);
                }
            }
        }

        visual.set_child_composition_visual(composition_visual);
        if let Some(source) = visual.presentation_source() {
            source.renderer().recalculate_children(visual);
        }
    }

    /// Gets the composition visual set with
    /// [`set_element_child_visual`](Self::set_element_child_visual).
    pub fn get_element_child_visual(visual: &Visual) -> Option<Rc<CompositionVisual>> {
        visual.child_composition_visual()
    }
}
