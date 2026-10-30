//! Port of `FilesDropBehavior.cs`.

use crate::{DropBehaviorBase, DropHandlerBase};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{DataFormat, DataTransferExtensions, DragEventArgs};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that handles file drop operations: the command of the behavior executes with the
/// dropped files.
///
/// Deviation (DEVIATIONS.md, Behaviors): the array of storage items of the reference is passed
/// as a `FerroList<Rc<dyn IStorageItem>>` (a storage item has no equality, which an untyped value
/// needs; the list compares by identity).
#[repr(C)]
pub struct FilesDropBehavior {
    base: DropBehaviorBase,
}

ferro_class!(FilesDropBehavior: DropBehaviorBase);
ferro_class_info!(FilesDropBehavior { new: FilesDropBehavior::new });
ferroui_base::ferro_impl_classes!(FilesDropBehavior: StyledElementImpl, StyledElementBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for FilesDropBehavior {
        /// The constructor body: the event args are passed to the command, and the handler
        /// executes it with the files.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_pass_event_args_to_command(true);
            let behavior = this.to_ref().downgrade();
            this.set_handler(Some(Rc::new(FilesDropHandler {
                execute: Rc::new(move |parameter: Option<BoxedValue>| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.execute_command(parameter.as_ref());
                    }
                }),
            })));
        }
    }
}

impl FilesDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DropBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Internal handler used to validate and execute file drop operations.
struct FilesDropHandler {
    /// Callback invoked when files are successfully dropped.
    execute: Rc<dyn Fn(Option<BoxedValue>)>,
}

impl DropHandlerBase for FilesDropHandler {
    fn validate(
        &self,
        _sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        e.data_transfer().contains(&DataFormat::file())
    }

    fn execute(
        &self,
        _sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        if !e.data_transfer().contains(&DataFormat::file()) {
            return false;
        }

        let Some(files) = e.data_transfer().try_get_files().filter(|files| !files.is_empty()) else {
            return false;
        };

        (self.execute)(Some(Rc::new(FerroList::from_items(files))));

        true
    }
}
