//! Port of `StorageProvider/SaveFilePickerAction.cs`.

use super::utilities::FileFilterParser;
use super::{SaveFilePickerEventArgs, PickerActionBase};
use crate::clipboard::visual_of;
use crate::StorageFileRef;
use ferroui_base::platform::storage::FilePickerSaveOptions;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::StyledElementActionImpl;
use std::rc::Rc;

/// An action that will open a save file picker dialog.
///
/// The picked file (`StorageFileRef`) is given to the handlers of [`pick`](Self::pick) and,
/// unless a handler handled it, to the command.
///
/// Deviation (DEVIATIONS.md, Behaviors): the `Pick` event is an event of the Rust API (a handler
/// list), not declared to markup.
#[repr(C)]
pub struct SaveFilePickerAction {
    base: PickerActionBase,
    pick: HandlerList<dyn Fn(&SaveFilePickerEventArgs)>,
}

ferro_class! {
    SaveFilePickerAction: PickerActionBase, virtuals SaveFilePickerActionImpl: StyledElementActionImpl {
        /// Raises the [`pick`](SaveFilePickerAction::pick) event.
        fn on_pick(this, args: &SaveFilePickerEventArgs);
    }
}
ferro_class_info!(SaveFilePickerAction { new: SaveFilePickerAction::new });
ferroui_base::ferro_impl_classes!(SaveFilePickerAction: StyledElementImpl);

ferro_properties! {
    impl SaveFilePickerAction {
        /// Identifies the `DefaultExtension` property.
        pub fn default_extension_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SaveFilePickerAction, _>("DefaultExtension", None)
        }

        /// Identifies the `FileTypeChoices` property.
        pub fn file_type_choices_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SaveFilePickerAction, _>("FileTypeChoices", None)
        }

        /// Identifies the `ShowOverwritePrompt` property.
        pub fn show_overwrite_prompt_property() -> StyledProperty<Option<bool>> {
            FerroProperty::register::<SaveFilePickerAction, _>("ShowOverwritePrompt", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for SaveFilePickerAction {
        /// The constructor body: the picked file is passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for SaveFilePickerAction {
    /// Executes the action.
    ///
    /// Returns the task of the picker operation; false when the sender is not a visual.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = visual_of(sender) else {
            return Some(Rc::new(false));
        };

        let action: Ref<SaveFilePickerAction> = this.to_ref();
        let task = this.track_dispatched_picker_operation(move || async move {
            action.save_file_picker_async(&visual).await;
        });
        Some(Rc::new(task))
    }
}

impl SaveFilePickerActionImpl for SaveFilePickerAction {
    fn on_pick(this: &Self, args: &SaveFilePickerEventArgs) {
        for (_, handler) in this.pick.snapshot().iter() {
            handler(args);
        }
    }
}

impl SaveFilePickerAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PickerActionBase::construct(), pick: HandlerList::new() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs after the save file picker successfully returns a file. Disposing the returned
    /// handle removes the handler.
    pub fn pick(&self, handler: impl Fn(&SaveFilePickerEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.pick.add(Rc::new(handler));
        let this = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.pick.remove(token);
            }
        })
    }

    /// Gets or sets the default extension to be used to save the file. This is a property of the
    /// object model.
    pub fn default_extension(&self) -> Option<String> {
        self.get_value(Self::default_extension_property())
    }

    pub fn set_default_extension(&self, value: Option<String>) {
        self.set_value(Self::default_extension_property(), value)
    }

    /// Gets or sets the file types that the user can choose to assign to a file, as a filter
    /// (`Description|*.ext;*.other|..`). This is a property of the object model.
    pub fn file_type_choices(&self) -> Option<String> {
        self.get_value(Self::file_type_choices_property())
    }

    pub fn set_file_type_choices(&self, value: Option<String>) {
        self.set_value(Self::file_type_choices_property(), value)
    }

    /// Gets or sets a value indicating whether the picker displays a warning if the user specifies
    /// the name of a file that already exists. This is a property of the object model.
    pub fn show_overwrite_prompt(&self) -> Option<bool> {
        self.get_value(Self::show_overwrite_prompt_property())
    }

    pub fn set_show_overwrite_prompt(&self, value: Option<bool>) {
        self.set_value(Self::show_overwrite_prompt_property(), value)
    }

    async fn save_file_picker_async(&self, visual: &Visual) {
        let command = self.command();
        if !self.is_enabled() || (command.is_none() && self.pick.is_empty()) {
            return;
        }

        let Some(storage_provider) = self.resolve_storage_provider(Some(visual)) else {
            return;
        };

        let suggested_start_location = self.resolve_suggested_start_location(Some(storage_provider.clone())).await;

        let mut options = FilePickerSaveOptions::new();
        options.set_title(self.title());
        options.set_suggested_start_location(suggested_start_location);
        options.set_suggested_file_name(self.suggested_file_name());
        options.set_default_extension(self.default_extension());
        options.set_file_type_choices(
            self.file_type_choices().and_then(|filter| FileFilterParser::convert_to_file_picker_file_type(&filter)),
        );
        options.set_show_overwrite_prompt(self.show_overwrite_prompt());

        // A failure of the picker fails the operation.
        let file = storage_provider.save_file_picker_async(options).await.unwrap_or_else(|error| panic!("{error}"));

        let Some(file) = file else {
            return;
        };

        let file = StorageFileRef::new(file);
        let event_args = SaveFilePickerEventArgs::new(file.clone());
        self.on_pick(&event_args);
        let Some(command) = command.filter(|_| !event_args.handled()) else {
            return;
        };

        let resolved_parameter = self.resolve_parameter(Some(&(Rc::new(file) as BoxedValue)));

        if !command.can_execute(resolved_parameter.as_ref()) {
            return;
        }

        command.execute(resolved_parameter.as_ref());
    }
}
