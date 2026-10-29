//! Port of `StorageProvider/Core/PickerActionBase.cs`.

use super::PickerSuggestedStartLocationHelper;
use crate::action_task::{ActionTask, CatchUnwind};
use crate::{StorageFolderRef, StorageProviderRef};
use ferroui_base::platform::storage::{IStorageFolder, IStorageProvider};
use ferroui_base::{
    ferro_class, ferro_properties, FerroObject, FerroObjectImpl, FerroProperty, StyledElement, StyledElementImpl,
    StyledProperty, Visual,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{InvokeCommandActionBase, StyledElementActionImpl};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/// Base class for picker actions: the storage provider, the title and the suggested start location
/// of the picker, and the tracking of the picker operations that run.
///
/// The class is abstract: a deriving class implements [`StyledElementActionImpl::execute`].
#[repr(C)]
pub struct PickerActionBase {
    base: InvokeCommandActionBase,
    active_picker_operations: Rc<RefCell<Vec<ActionTask>>>,
}

ferro_class!(PickerActionBase: InvokeCommandActionBase);
ferroui_base::ferro_impl_classes!(PickerActionBase: FerroObjectImpl, StyledElementImpl, StyledElementActionImpl);

ferro_properties! {
    impl PickerActionBase {
        /// Identifies the `StorageProvider` property.
        pub fn storage_provider_property() -> StyledProperty<Option<StorageProviderRef>> {
            FerroProperty::register::<PickerActionBase, _>("StorageProvider", None)
        }

        /// Identifies the `Title` property.
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PickerActionBase, _>("Title", None)
        }

        /// Identifies the `SuggestedStartLocation` property.
        pub fn suggested_start_location_property() -> StyledProperty<Option<StorageFolderRef>> {
            FerroProperty::register::<PickerActionBase, _>("SuggestedStartLocation", None)
        }

        /// Identifies the `SuggestedStartLocationPath` property.
        pub fn suggested_start_location_path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PickerActionBase, _>("SuggestedStartLocationPath", None)
        }

        /// Identifies the `SuggestedFileName` property.
        pub fn suggested_file_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PickerActionBase, _>("SuggestedFileName", None)
        }

        /// Identifies the `CreateSuggestedStartLocationDirectory` property.
        pub fn create_suggested_start_location_directory_property() -> StyledProperty<bool> {
            FerroProperty::register::<PickerActionBase, _>("CreateSuggestedStartLocationDirectory", false)
        }
    }
}

/// The storage provider of the top level of an object (`ResolveFromObject`): the provider of a top
/// level, else of the top level among the logical ancestors of a styled element, else of the top
/// level of the visual tree of a visual.
pub(crate) fn resolve_storage_provider_from_object(target: Option<&FerroObject>) -> Option<Rc<dyn IStorageProvider>> {
    let target = target?;

    if let Some(top_level) = target.downcast_ref::<TopLevel>() {
        return Some(top_level.storage_provider());
    }

    if let Some(visual) = target.downcast_ref::<Visual>() {
        return visual
            .get_self_and_logical_ancestors()
            .find_map(|logical| logical.cast::<TopLevel>())
            .or_else(|| TopLevel::get_top_level(Some(visual)))
            .map(|top_level| top_level.storage_provider());
    }

    if let Some(logical) = target.downcast_ref::<StyledElement>() {
        return logical
            .get_self_and_logical_ancestors()
            .find_map(|logical| logical.cast::<TopLevel>())
            .map(|top_level| top_level.storage_provider());
    }

    None
}

impl PickerActionBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandActionBase::construct(), active_picker_operations: Rc::new(RefCell::new(Vec::new())) }
    }

    /// Gets or sets the storage provider that the picker uses to access the file system. This is a
    /// property of the object model.
    pub fn storage_provider(&self) -> Option<StorageProviderRef> {
        self.get_value(Self::storage_provider_property())
    }

    pub fn set_storage_provider(&self, value: Option<StorageProviderRef>) {
        self.set_value(Self::storage_provider_property(), value)
    }

    /// Gets or sets the text that appears in the title bar of a picker. This is a property of the
    /// object model.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_property(), value)
    }

    /// Gets or sets the initial location where the picker looks for files to present to the user.
    /// This is a property of the object model.
    pub fn suggested_start_location(&self) -> Option<StorageFolderRef> {
        self.get_value(Self::suggested_start_location_property())
    }

    pub fn set_suggested_start_location(&self, value: Option<StorageFolderRef>) {
        self.set_value(Self::suggested_start_location_property(), value)
    }

    /// Gets or sets a fallback path that is used to resolve the suggested start location when no
    /// folder is provided.
    pub fn suggested_start_location_path(&self) -> Option<String> {
        self.get_value(Self::suggested_start_location_path_property())
    }

    pub fn set_suggested_start_location_path(&self, value: Option<String>) {
        self.set_value(Self::suggested_start_location_path_property(), value)
    }

    /// Gets or sets the file name that the file picker suggests to the user. This is a property of
    /// the object model.
    pub fn suggested_file_name(&self) -> Option<String> {
        self.get_value(Self::suggested_file_name_property())
    }

    pub fn set_suggested_file_name(&self, value: Option<String>) {
        self.set_value(Self::suggested_file_name_property(), value)
    }

    /// Gets or sets a value indicating whether to create the suggested start location directory if
    /// it doesn't exist. This is a property of the object model.
    pub fn create_suggested_start_location_directory(&self) -> bool {
        self.get_value(Self::create_suggested_start_location_directory_property())
    }

    pub fn set_create_suggested_start_location_directory(&self, value: bool) {
        self.set_value(Self::create_suggested_start_location_directory_property(), value)
    }

    /// Resolves the storage provider using the configured value or the provided fallback visual
    /// (then the action itself).
    pub fn resolve_storage_provider(&self, fallback_visual: Option<&Visual>) -> Option<Rc<dyn IStorageProvider>> {
        if let Some(provider) = self.storage_provider() {
            return Some(provider.0);
        }

        if let Some(fallback_visual) = fallback_visual {
            let fallback: &FerroObject = fallback_visual;
            if let Some(provider) = resolve_storage_provider_from_object(Some(fallback)) {
                return Some(provider);
            }
        }

        let this: &FerroObject = self;
        resolve_storage_provider_from_object(Some(this))
    }

    /// Resolves the suggested start folder using the configured value or the configured path,
    /// translated by `provider`.
    pub async fn resolve_suggested_start_location(
        &self,
        provider: Option<Rc<dyn IStorageProvider>>,
    ) -> Option<Rc<dyn IStorageFolder>> {
        PickerSuggestedStartLocationHelper::resolve(
            self.suggested_start_location().map(|folder| folder.0),
            self.suggested_start_location_path(),
            self.create_suggested_start_location_directory(),
            provider,
        )
        .await
    }

    /// Keeps an asynchronous picker operation rooted until it completes, and returns it.
    pub fn track_picker_operation(&self, operation: ActionTask) -> ActionTask {
        self.active_picker_operations.borrow_mut().push(operation.clone());

        let operations = self.active_picker_operations.clone();
        let tracked = operation.clone();
        drop(ActionTask::spawn(async move {
            // The caller receives the original task; this continuation only owns lifetime
            // clean-up, whatever the outcome of the operation.
            let _ = CatchUnwind::new(tracked.clone()).await;
            operations.borrow_mut().retain(|active| *active != tracked);
        }));

        operation
    }

    /// Runs and tracks an asynchronous picker operation on the dispatcher of the user interface.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the reference accepts the call from any thread; the
    /// operation of the port uses the objects of the user interface, which belong to its thread,
    /// so the call is made on that thread (the operation still runs in a job of its dispatcher).
    pub fn track_dispatched_picker_operation<F, Fut>(&self, operation: F) -> ActionTask
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        self.track_picker_operation(ActionTask::spawn(async move { operation().await }))
    }

    /// The number of the picker operations that run (an internal member of the reference).
    #[doc(hidden)]
    pub fn active_picker_operation_count(&self) -> usize {
        self.active_picker_operations.borrow().len()
    }
}
