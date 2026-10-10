//! Changing an inherited property (the data context) at the root of a tree
//! of controls.

use crate::control_hierarchy_creator::ControlHierarchyCreator;
use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::testing::NullRenderer;
use ferroui_controls::{Control, StackPanel};
use std::rc::Rc;

pub struct InheritedProperties {
    root: Ref<TestRoot>,
    _controls: Vec<Ref<Control>>,
    /// The dispatcher of the thread of the benchmark: the layout pass of the root reads it, and
    /// a dispatcher belongs to the thread that first asks for it. Declared last: released after
    /// the tree.
    _dispatcher: UnitTestDispatcherScope,
}

impl InheritedProperties {
    pub fn new() -> Self {
        let dispatcher = Dispatcher::unit_test_scope();
        let mut controls: Vec<Ref<Control>> = Vec::new();

        let panel = StackPanel::new();

        let root = TestRoot::new();
        root.set_child(&panel);
        root.set_renderer(NullRenderer::new());

        controls.push(panel.clone().upcast());
        let controls = ControlHierarchyCreator::create_children(controls, &panel, 3, 5, 5);

        root.layout_manager().execute_initial_layout_pass();

        Self { root, _controls: controls, _dispatcher: dispatcher }
    }

    #[inline(never)]
    pub fn change_data_context(&self) {
        let data_contexts: [BoxedValue; 3] =
            [Rc::new(TestDataContext), Rc::new(TestDataContext), Rc::new(TestDataContext)];

        for _ in 0..100 {
            for data_context in &data_contexts {
                self.root.set_data_context(Some(data_context.clone()));
            }
        }
    }
}

pub struct TestDataContext;

/// A reference type: two objects are equal when they are the same object.
impl PartialEq for TestDataContext {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("data", "InheritedProperties");
    class.benchmark("change_data_context", "", InheritedProperties::new, |b| b.change_data_context());
}

#[cfg(test)]
mod tests {
    #[test]
    fn inherited_properties() {
        crate::harness::smoke_class(super::register, "InheritedProperties");
    }
}
