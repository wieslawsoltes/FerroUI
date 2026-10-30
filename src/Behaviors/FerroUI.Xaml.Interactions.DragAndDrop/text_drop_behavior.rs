//! Port of `TextDropBehavior.cs`.

use crate::{DropBehaviorBase, DropHandlerBase};
use ferroui_base::input::{DataFormat, DataTransferExtensions, DragEventArgs};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that handles text drop operations: the command of the behavior executes with the
/// dropped text.
#[repr(C)]
pub struct TextDropBehavior {
    base: DropBehaviorBase,
}

ferro_class!(TextDropBehavior: DropBehaviorBase);
ferro_class_info!(TextDropBehavior { new: TextDropBehavior::new });
ferroui_base::ferro_impl_classes!(TextDropBehavior: StyledElementImpl, StyledElementBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for TextDropBehavior {
        /// The constructor body: the event args are passed to the command, and the handler
        /// executes it with the text.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_pass_event_args_to_command(true);
            let behavior = this.to_ref().downgrade();
            this.set_handler(Some(Rc::new(TextDropHandler {
                execute: Rc::new(move |parameter: Option<BoxedValue>| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.execute_command(parameter.as_ref());
                    }
                }),
            })));
        }
    }
}

impl TextDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DropBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Internal handler used to validate and execute text drop operations.
struct TextDropHandler {
    /// Callback invoked when text is successfully dropped.
    execute: Rc<dyn Fn(Option<BoxedValue>)>,
}

impl DropHandlerBase for TextDropHandler {
    fn validate(
        &self,
        _sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        e.data_transfer().contains(&DataFormat::text())
    }

    fn execute(
        &self,
        _sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        if !e.data_transfer().contains(&DataFormat::text()) {
            return false;
        }

        let Some(text) = e.data_transfer().try_get_text() else {
            return false;
        };

        (self.execute)(Some(Rc::new(text)));

        true
    }
}
