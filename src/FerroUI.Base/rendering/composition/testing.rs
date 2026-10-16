//! The members of the compositor the benchmarks of the framework reach:
//! upstream's composition target benchmark builds draw list visuals without
//! a visual of the visual tree through the internal constructor of
//! `CompositionDrawListVisual`, with its access to the internals of the
//! framework.

use super::{CompositionDrawListVisual, Compositor};
use std::rc::Rc;

/// `new CompositionDrawListVisual(compositor, new
/// ServerCompositionDrawListVisual(compositor.Server, null), null)`: a draw
/// list visual of no visual of the tree. It draws the draw list it is given
/// and has no custom hit test.
pub fn create_draw_list_visual(compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
    CompositionDrawListVisual::without_visual(compositor)
}
