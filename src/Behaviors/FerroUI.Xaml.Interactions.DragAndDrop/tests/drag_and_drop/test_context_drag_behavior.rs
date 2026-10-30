//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/DragAndDrop/TestContextDragBehavior.cs`
//! of Xaml.Behaviors.

use crate::{ContextDragBehaviorBase, ContextDragBehaviorBaseImpl};
use ferroui_base::input::PointerEventArgs;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::cell::Cell;

/// A context drag behavior that records its drag callbacks and counts the attachments of its
/// control to the visual tree.
#[repr(C)]
pub(crate) struct TestContextDragBehavior {
    base: ContextDragBehaviorBase,
    before_called: Cell<bool>,
    after_called: Cell<bool>,
    attached_to_visual_tree_count: Cell<i32>,
    detached_from_visual_tree_count: Cell<i32>,
}

ferro_class!(TestContextDragBehavior: ContextDragBehaviorBase);
ferroui_base::ferro_impl_classes!(TestContextDragBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TestContextDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);
            this.attached_to_visual_tree_count.set(this.attached_to_visual_tree_count.get() + 1);
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);
            this.detached_from_visual_tree_count.set(this.detached_from_visual_tree_count.get() + 1);
        }
    }
}

impl ContextDragBehaviorBaseImpl for TestContextDragBehavior {
    fn on_before_drag_drop(this: &Self, _sender: Option<&BoxedValue>, _e: &PointerEventArgs, _context: Option<&BoxedValue>) {
        this.before_called.set(true);
    }

    fn on_after_drag_drop(this: &Self, _sender: Option<&BoxedValue>, _e: &PointerEventArgs, _context: Option<&BoxedValue>) {
        this.after_called.set(true);
    }
}

#[allow(dead_code)]
impl TestContextDragBehavior {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: ContextDragBehaviorBase::construct(),
            before_called: Cell::new(false),
            after_called: Cell::new(false),
            attached_to_visual_tree_count: Cell::new(0),
            detached_from_visual_tree_count: Cell::new(0),
        })
    }

    pub(crate) fn before_called(&self) -> bool {
        self.before_called.get()
    }

    pub(crate) fn after_called(&self) -> bool {
        self.after_called.get()
    }

    pub(crate) fn attached_to_visual_tree_count(&self) -> i32 {
        self.attached_to_visual_tree_count.get()
    }

    pub(crate) fn detached_from_visual_tree_count(&self) -> i32 {
        self.detached_from_visual_tree_count.get()
    }
}
