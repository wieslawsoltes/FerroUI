//! Port of `PanelDragBehavior.cs`.

use crate::{ContextDragBehaviorBase, ContextDragBehaviorBaseImpl};
use ferroui_base::input::PointerEventArgs;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, ElementRef, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use std::rc::Rc;

/// Behavior that starts a drag operation for a control so it can be moved between panels. The
/// control itself is used as the drag context.
#[repr(C)]
pub struct PanelDragBehavior {
    base: ContextDragBehaviorBase,
}

ferro_class!(PanelDragBehavior: ContextDragBehaviorBase);
ferro_class_info!(PanelDragBehavior { new: PanelDragBehavior::new });
ferroui_base::ferro_impl_classes!(PanelDragBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PanelDragBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            // Deviation (DEVIATIONS.md, Behaviors): the context is a weak reference to the control
            // (an element reference, which untyped code views as the control): the control holds
            // the behavior, and the behavior holding the control would keep both alive.
            let context: Option<BoxedValue> = this
                .associated_object_as::<Control>()
                .map(|associated_object| Rc::new(ElementRef::of(&associated_object)) as BoxedValue);
            this.set_context(context);
        }
    }
}

impl ContextDragBehaviorBaseImpl for PanelDragBehavior {
    fn on_before_drag_drop(_this: &Self, _sender: Option<&BoxedValue>, _e: &PointerEventArgs, _context: Option<&BoxedValue>) {}

    fn on_after_drag_drop(_this: &Self, _sender: Option<&BoxedValue>, _e: &PointerEventArgs, _context: Option<&BoxedValue>) {}
}

impl PanelDragBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContextDragBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
