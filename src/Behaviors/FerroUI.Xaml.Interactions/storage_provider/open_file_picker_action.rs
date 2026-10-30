//! Port of `StorageProvider/OpenFilePickerAction.cs`.

use super::utilities::FileFilterParser;
use super::{FilePickerEventArgs, PickerActionBase};
use crate::clipboard::visual_of;
use crate::StorageFileRef;
use ferroui_base::platform::storage::FilePickerOpenOptions;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::StyledElementActionImpl;
use std::rc::Rc;

/// An action that will open a file picker dialog.
///
/// The picked files (`Vec<StorageFileRef>`) are given to the handlers of [`pick`](Self::pick) and,
/// unless a handler handled them, to the command.
///
/// Deviation (DEVIATIONS.md, Behaviors): the `Pick` event is an event of the Rust API (a handler
/// list), not declared to markup.
#[repr(C)]
pub struct OpenFilePickerAction {
    base: PickerActionBase,
    pick: HandlerList<dyn Fn(&FilePickerEventArgs)>,
}

ferro_class! {
    OpenFilePickerAction: PickerActionBase, virtuals OpenFilePickerActionImpl: StyledElementActionImpl {
        /// Raises the [`pick`](OpenFilePickerAction::pick) event.
        fn on_pick(this, args: &FilePickerEventArgs);
    }
}
ferro_class_info!(OpenFilePickerAction { new: OpenFilePickerAction::new });
ferroui_base::ferro_impl_classes!(OpenFilePickerAction: StyledElementImpl);

ferro_properties! {
    impl OpenFilePickerAction {
        /// Identifies the `AllowMultiple` property.
        pub fn allow_multiple_property() -> StyledProperty<bool> {
            FerroProperty::register::<OpenFilePickerAction, _>("AllowMultiple", false)
        }

        /// Identifies the `FileTypeFilter` property.
        pub fn file_type_filter_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<OpenFilePickerAction, _>("FileTypeFilter", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for OpenFilePickerAction {
        /// The constructor body: the picked files are passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for OpenFilePickerAction {
    /// Executes the action.
    ///
    /// Returns the task of the picker operation; false when the sender is not a visual.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<OpenFilePickerAction> = this.to_ref();
        let task = this.track_dispatched_picker_operation(move || async move {
            action.open_file_picker_async(&visual).await;
        });
        Some(Rc::new(task))
    }
}

impl OpenFilePickerActionImpl for OpenFilePickerAction {
    fn on_pick(this: &Self, args: &FilePickerEventArgs) {
        for (_, handler) in this.pick.snapshot().iter() {
            handler(args);
        }
    }
}

impl OpenFilePickerAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PickerActionBase::construct(), pick: HandlerList::new() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs after the file picker successfully returns one or more files. Disposing the returned
    /// handle removes the handler.
    pub fn pick(&self, handler: impl Fn(&FilePickerEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.pick.add(Rc::new(handler));
        let this = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.pick.remove(token);
            }
        })
    }

    /// Gets or sets an option indicating whether the open picker allows users to select multiple
    /// files. This is a property of the object model.
    pub fn allow_multiple(&self) -> bool {
        self.get_value(Self::allow_multiple_property())
    }

    pub fn set_allow_multiple(&self, value: bool) {
        self.set_value(Self::allow_multiple_property(), value)
    }

    /// Gets or sets the file types that the file open picker displays, as a filter
    /// (`Description|*.ext;*.other|..`). This is a property of the object model.
    pub fn file_type_filter(&self) -> Option<String> {
        self.get_value(Self::file_type_filter_property())
    }

    pub fn set_file_type_filter(&self, value: Option<String>) {
        self.set_value(Self::file_type_filter_property(), value)
    }

    async fn open_file_picker_async(&self, visual: &Visual) {
        let command = self.command();
        if !self.is_enabled() || (command.is_none() && self.pick.is_empty()) {
            return;
        }

        let Some(storage_provider) = self.resolve_storage_provider(Some(visual)) else {
            return;
        };

        let suggested_start_location = self.resolve_suggested_start_location(Some(storage_provider.clone())).await;

        let mut options = FilePickerOpenOptions::new();
        options.set_title(self.title());
        options.set_suggested_start_location(suggested_start_location);
        options.set_suggested_file_name(self.suggested_file_name());
        options.set_allow_multiple(self.allow_multiple());
        options.set_file_type_filter(
            self.file_type_filter().and_then(|filter| FileFilterParser::convert_to_file_picker_file_type(&filter)),
        );

        // A failure of the picker fails the operation.
        let files = storage_provider.open_file_picker_async(options).await.unwrap_or_else(|error| panic!("{error}"));

        if files.is_empty() {
            return;
        }

        let files: Vec<StorageFileRef> = files.into_iter().map(StorageFileRef::new).collect();
        let event_args = FilePickerEventArgs::new(files.clone());
        self.on_pick(&event_args);
        let Some(command) = command.filter(|_| !event_args.handled()) else {
            return;
        };

        let resolved_parameter = self.resolve_parameter(Some(&(Rc::new(files) as BoxedValue)));

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }
}
