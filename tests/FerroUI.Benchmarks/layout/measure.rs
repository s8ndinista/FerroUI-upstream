//! Measuring and arranging a tree again: every control of the tree is
//! marked as not measured and not arranged, and the layout manager runs a
//! layout pass.
//!
//! The flags are set through their private setters, as upstream does with
//! accessors of them (`UnsafeAccessor`) instead of `InvalidateMeasure`,
//! otherwise a lot of time is spent invalidating controls, which this
//! benchmark is not about: it is focused on measure and arrange. The
//! `testing` feature of the base crate exposes the setters
//! (`ferroui_base::layout::testing`).

use crate::control_hierarchy_creator::ControlHierarchyCreator;
use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::layout::testing::{set_is_arrange_valid, set_is_measure_valid};
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::Ref;
use ferroui_controls::testing::NullRenderer;
use ferroui_controls::{Control, StackPanel};

pub struct Measure {
    root: Ref<TestRoot>,
    controls: Vec<Ref<Control>>,
    /// The dispatcher of the thread of the benchmark: the layout pass of the root reads it, and
    /// a dispatcher belongs to the thread that first asks for it. Declared last: released after
    /// the tree.
    _dispatcher: UnitTestDispatcherScope,
}

impl Measure {
    pub fn new() -> Self {
        let dispatcher = Dispatcher::unit_test_scope();
        let panel = StackPanel::new();

        let root = TestRoot::new();
        root.set_child(&panel);
        root.set_renderer(NullRenderer::new());

        let mut controls: Vec<Ref<Control>> = Vec::new();
        controls.push(panel.clone().upcast());
        let controls = ControlHierarchyCreator::create_children(controls, &panel, 3, 5, 5);

        root.layout_manager().execute_initial_layout_pass();

        Self { root, controls, _dispatcher: dispatcher }
    }

    pub fn remeasure(&self) {
        self.root.invalidate_measure();

        for control in &self.controls {
            // Not `invalidate_measure`: see the module documentation.
            set_is_measure_valid(control, false);
            set_is_arrange_valid(control, false);
        }

        self.root.layout_manager().execute_layout_pass();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout", "Measure");
    class.benchmark("remeasure", "", Measure::new, |b| b.remeasure());
}

#[cfg(test)]
mod tests {
    #[test]
    fn measure() {
        crate::harness::smoke_class(super::register, "Measure");
    }
}
