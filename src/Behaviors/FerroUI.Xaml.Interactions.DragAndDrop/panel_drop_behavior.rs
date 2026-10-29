//! Port of `PanelDropBehavior.cs`.

use crate::drag_drop_context_store::DragDropContextStore;
use crate::values::{boxed_event_args, object_of};
use crate::{ContextDropBehaviorBase, DropBehaviorBase, DropHandlerBase};
use ferroui_base::input::{DataTransferExtensions, DragEventArgs};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
};
use ferroui_controls::{Canvas, Control, Panel};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that allows dropping controls dragged with [`PanelDragBehavior`](crate::PanelDragBehavior):
/// the dragged control moves to the panel of the behavior (to the drop position in a canvas).
#[repr(C)]
pub struct PanelDropBehavior {
    base: DropBehaviorBase,
}

ferro_class!(PanelDropBehavior: DropBehaviorBase);
ferro_class_info!(PanelDropBehavior { new: PanelDropBehavior::new });
ferroui_base::ferro_impl_classes!(PanelDropBehavior: StyledElementImpl, StyledElementBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for PanelDropBehavior {
        /// The constructor body: the event args are passed to the command, and the handler
        /// executes it after the move.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_pass_event_args_to_command(true);
            let behavior = this.to_ref().downgrade();
            this.set_handler(Some(Rc::new(PanelDropHandler {
                execute: Rc::new(move |parameter: Option<BoxedValue>| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.execute_command(parameter.as_ref());
                    }
                }),
            })));
        }
    }
}

impl PanelDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DropBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The control the drag data of `e` carries (by its key in the context store).
fn dragged_control(e: &DragEventArgs) -> Option<Ref<Control>> {
    let key = e.data_transfer().try_get_value(&ContextDropBehaviorBase::context_data_transfer_format());
    let context = DragDropContextStore::get(key.as_deref());
    object_of(context.as_ref()).and_then(|object| object.cast::<Control>())
}

struct PanelDropHandler {
    execute: Rc<dyn Fn(Option<BoxedValue>)>,
}

impl DropHandlerBase for PanelDropHandler {
    fn validate(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        object_of(sender).is_some_and(|sender| sender.is::<Panel>()) && dragged_control(e).is_some()
    }

    fn execute(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        let Some(target) = object_of(sender).and_then(|sender| sender.cast::<Panel>()) else {
            return false;
        };

        let Some(control) = dragged_control(e) else {
            return false;
        };

        if let Some(source) = control.parent().and_then(|parent| parent.cast::<Panel>()) {
            source.children().remove(control.clone());
        }

        target.children().add(control.clone());

        if let Some(canvas) = target.cast::<Canvas>() {
            let p = e.get_position(&canvas);
            Canvas::set_left(&control, p.x - control.bounds().width / 2.0);
            Canvas::set_top(&control, p.y - control.bounds().height / 2.0);
        }

        (self.execute)(Some(boxed_event_args(e)));
        true
    }
}
