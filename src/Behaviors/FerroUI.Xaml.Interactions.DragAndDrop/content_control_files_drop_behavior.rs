//! Port of `ContentControlFilesDropBehavior.cs`.

use crate::values::object_of;
use crate::{DropBehaviorBase, DropHandlerBase};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{DataFormat, DataTransferExtensions, DragDropEffects, DragEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::ContentControl;
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::rc::Rc;

/// Behavior that handles file drop operations for [`ContentControl`]: while files are dragged
/// over the control it shows [`content_during_drag`](Self::content_during_drag) on
/// [`background_during_drag`](Self::background_during_drag), and the command of the behavior
/// executes with the dropped files (see [`FilesDropBehavior`](crate::FilesDropBehavior)).
#[repr(C)]
pub struct ContentControlFilesDropBehavior {
    base: DropBehaviorBase,
}

ferro_class!(ContentControlFilesDropBehavior: DropBehaviorBase);
ferro_class_info!(ContentControlFilesDropBehavior { new: ContentControlFilesDropBehavior::new });
ferroui_base::ferro_impl_classes!(ContentControlFilesDropBehavior: StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl ContentControlFilesDropBehavior {
        /// Identifies the `ContentDuringDrag` property.
        pub fn content_during_drag_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentControlFilesDropBehavior, _>("ContentDuringDrag", None)
        }

        /// Identifies the `BackgroundDuringDrag` property.
        pub fn background_during_drag_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<ContentControlFilesDropBehavior, _>("BackgroundDuringDrag", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ContentControlFilesDropBehavior {
        /// The constructor body: the event args are passed to the command (to pass the parameter
        /// on drop), and the handler reads the values to show during the drag from the behavior.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_pass_event_args_to_command(true);
            let execute_behavior = this.to_ref().downgrade();
            let content_behavior = this.to_ref().downgrade();
            let background_behavior = this.to_ref().downgrade();
            this.set_handler(Some(Rc::new(FilesDropHandler {
                execute: Rc::new(move |parameter: Option<BoxedValue>| {
                    if let Some(behavior) = execute_behavior.upgrade() {
                        behavior.execute_command(parameter.as_ref());
                    }
                }),
                get_content_during_drag: Rc::new(move || {
                    content_behavior.upgrade().and_then(|behavior| behavior.content_during_drag())
                }),
                get_background_during_drag: Rc::new(move || {
                    background_behavior.upgrade().and_then(|behavior| behavior.background_during_drag())
                }),
            })));
        }
    }
}

impl ContentControlFilesDropBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DropBehaviorBase::construct() }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// If the sender is a content control, this content is set during drag over.
    pub fn content_during_drag(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_during_drag_property())
    }

    pub fn set_content_during_drag(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_during_drag_property(), value)
    }

    /// If the sender is a content control, this background is set during drag over.
    pub fn background_during_drag(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_during_drag_property())
    }

    pub fn set_background_during_drag(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_during_drag_property(), value)
    }
}

struct FilesDropHandler {
    execute: Rc<dyn Fn(Option<BoxedValue>)>,
    get_content_during_drag: Rc<dyn Fn() -> Option<BoxedValue>>,
    get_background_during_drag: Rc<dyn Fn() -> Option<Rc<dyn IBrush>>>,
}

impl FilesDropHandler {
    fn clear_drag_values(sender: Option<&BoxedValue>) {
        let Some(content_control) = object_of(sender).and_then(|sender| sender.cast::<ContentControl>()) else {
            return;
        };

        content_control.clear_value(ContentControl::content_property());
        content_control.clear_value(TemplatedControl::background_property());
    }
}

impl DropHandlerBase for FilesDropHandler {
    fn enter(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::base_enter(self, sender, e, source_context, target_context);

        if e.drag_effects() == DragDropEffects::NONE {
            return;
        }
        let Some(content_control) = object_of(sender).and_then(|sender| sender.cast::<ContentControl>()) else {
            return;
        };

        if let Some(content_during_drag) = (self.get_content_during_drag)() {
            content_control.set_current_value(ContentControl::content_property(), Some(content_during_drag));
        }

        if let Some(background_during_drag) = (self.get_background_during_drag)() {
            content_control.set_current_value(TemplatedControl::background_property(), Some(background_during_drag));
        }
    }

    fn drop(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::base_drop(self, sender, e, source_context, target_context);

        Self::clear_drag_values(sender);
    }

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

    fn cancel(&self, sender: Option<&BoxedValue>, _e: &RoutedEventArgs) {
        Self::clear_drag_values(sender);
    }
}
