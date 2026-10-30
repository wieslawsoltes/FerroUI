//! Port of `StorageProvider/OpenFolderPickerAction.cs`.

use super::{FolderPickerEventArgs, PickerActionBase};
use crate::clipboard::visual_of;
use crate::StorageFolderRef;
use ferroui_base::platform::storage::FolderPickerOpenOptions;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::StyledElementActionImpl;
use std::rc::Rc;

/// An action that will open a folder picker dialog.
///
/// The picked folders (`Vec<StorageFolderRef>`) are given to the handlers of [`pick`](Self::pick) and,
/// unless a handler handled them, to the command.
///
/// Deviation (DEVIATIONS.md, Behaviors): the `Pick` event is an event of the Rust API (a handler
/// list), not declared to markup.
#[repr(C)]
pub struct OpenFolderPickerAction {
    base: PickerActionBase,
    pick: HandlerList<dyn Fn(&FolderPickerEventArgs)>,
}

ferro_class! {
    OpenFolderPickerAction: PickerActionBase, virtuals OpenFolderPickerActionImpl: StyledElementActionImpl {
        /// Raises the [`pick`](OpenFolderPickerAction::pick) event.
        fn on_pick(this, args: &FolderPickerEventArgs);
    }
}
ferro_class_info!(OpenFolderPickerAction { new: OpenFolderPickerAction::new });
ferroui_base::ferro_impl_classes!(OpenFolderPickerAction: StyledElementImpl);

ferro_properties! {
    impl OpenFolderPickerAction {
        /// Identifies the `AllowMultiple` property.
        pub fn allow_multiple_property() -> StyledProperty<bool> {
            FerroProperty::register::<OpenFolderPickerAction, _>("AllowMultiple", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for OpenFolderPickerAction {
        /// The constructor body: the picked folders are passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for OpenFolderPickerAction {
    /// Executes the action.
    ///
    /// Returns the task of the picker operation; false when the sender is not a visual.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<OpenFolderPickerAction> = this.to_ref();
        let task = this.track_dispatched_picker_operation(move || async move {
            action.open_folder_picker_async(&visual).await;
        });
        Some(Rc::new(task))
    }
}

impl OpenFolderPickerActionImpl for OpenFolderPickerAction {
    fn on_pick(this: &Self, args: &FolderPickerEventArgs) {
        for (_, handler) in this.pick.snapshot().iter() {
            handler(args);
        }
    }
}

impl OpenFolderPickerAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PickerActionBase::construct(), pick: HandlerList::new() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs after the folder picker successfully returns one or more folders. Disposing the returned
    /// handle removes the handler.
    pub fn pick(&self, handler: impl Fn(&FolderPickerEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.pick.add(Rc::new(handler));
        let this = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.pick.remove(token);
            }
        })
    }

    /// Gets or sets an option indicating whether the open picker allows users to select multiple
    /// folders. This is a property of the object model.
    pub fn allow_multiple(&self) -> bool {
        self.get_value(Self::allow_multiple_property())
    }

    pub fn set_allow_multiple(&self, value: bool) {
        self.set_value(Self::allow_multiple_property(), value)
    }

    async fn open_folder_picker_async(&self, visual: &Visual) {
        let command = self.command();
        if !self.is_enabled() || (command.is_none() && self.pick.is_empty()) {
            return;
        }

        let Some(storage_provider) = self.resolve_storage_provider(Some(visual)) else {
            return;
        };

        let suggested_start_location = self.resolve_suggested_start_location(Some(storage_provider.clone())).await;

        let mut options = FolderPickerOpenOptions::new();
        options.set_title(self.title());
        options.set_suggested_start_location(suggested_start_location);
        options.set_suggested_file_name(self.suggested_file_name());
        options.set_allow_multiple(self.allow_multiple());

        // A failure of the picker fails the operation.
        let folders = storage_provider.open_folder_picker_async(options).await.unwrap_or_else(|error| panic!("{error}"));

        if folders.is_empty() {
            return;
        }

        let folders: Vec<StorageFolderRef> = folders.into_iter().map(StorageFolderRef::new).collect();
        let event_args = FolderPickerEventArgs::new(folders.clone());
        self.on_pick(&event_args);
        let Some(command) = command.filter(|_| !event_args.handled()) else {
            return;
        };

        let resolved_parameter = self.resolve_parameter(Some(&(Rc::new(folders) as BoxedValue)));

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }
}
