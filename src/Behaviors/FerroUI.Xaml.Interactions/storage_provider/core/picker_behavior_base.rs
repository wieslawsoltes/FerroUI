//! Port of `StorageProvider/Core/PickerBehaviorBase.cs`.

use super::picker_action_base::resolve_storage_provider_from_object;
use super::PickerSuggestedStartLocationHelper;
use crate::{StorageFolderRef, StorageProviderRef};
use ferroui_base::platform::storage::{IStorageFolder, IStorageProvider};
use ferroui_base::{
    ferro_class, ferro_properties, FerroObject, FerroObjectImpl, FerroProperty, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::{InvokeCommandBehaviorBase, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Base class for picker behaviors: the storage provider, the title and the suggested start
/// location of the picker.
///
/// The class is abstract.
#[repr(C)]
pub struct PickerBehaviorBase {
    base: InvokeCommandBehaviorBase,
}

ferro_class!(PickerBehaviorBase: InvokeCommandBehaviorBase);
ferroui_base::ferro_impl_classes!(PickerBehaviorBase: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl PickerBehaviorBase {
        /// Identifies the `StorageProvider` property.
        pub fn storage_provider_property() -> StyledProperty<Option<StorageProviderRef>> {
            FerroProperty::register::<PickerBehaviorBase, _>("StorageProvider", None)
        }

        /// Identifies the `Title` property.
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PickerBehaviorBase, _>("Title", None)
        }

        /// Identifies the `SuggestedStartLocation` property.
        pub fn suggested_start_location_property() -> StyledProperty<Option<StorageFolderRef>> {
            FerroProperty::register::<PickerBehaviorBase, _>("SuggestedStartLocation", None)
        }

        /// Identifies the `SuggestedStartLocationPath` property.
        pub fn suggested_start_location_path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PickerBehaviorBase, _>("SuggestedStartLocationPath", None)
        }

        /// Identifies the `SuggestedFileName` property.
        pub fn suggested_file_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<PickerBehaviorBase, _>("SuggestedFileName", None)
        }

        /// Identifies the `CreateSuggestedStartLocationDirectory` property.
        pub fn create_suggested_start_location_directory_property() -> StyledProperty<bool> {
            FerroProperty::register::<PickerBehaviorBase, _>("CreateSuggestedStartLocationDirectory", false)
        }
    }
}

impl PickerBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandBehaviorBase::construct() }
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
    /// (then the associated object, then the behavior itself).
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

        let associated_object = self.associated_object();
        let this: &FerroObject = self;
        resolve_storage_provider_from_object(associated_object.as_deref())
            .or_else(|| resolve_storage_provider_from_object(Some(this)))
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
}
