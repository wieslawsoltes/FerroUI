//! Port of `StorageProvider/Core/PickerSuggestedStartLocationHelper.cs`.

#[cfg(not(target_arch = "wasm32"))]
use ferroui_base::platform::storage::file_io::StorageProviderHelpers;
use ferroui_base::platform::storage::{IStorageFolder, IStorageProvider};
use ferroui_base::utilities::{Uri, UriKind};
use std::rc::Rc;

/// Resolves the suggested start locations of the pickers (an internal class of the reference).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference waits synchronously for the folder of the
/// path (`GetAwaiter().GetResult()`); here the resolution is asynchronous and the picker
/// operation awaits it.
pub(crate) struct PickerSuggestedStartLocationHelper;

impl PickerSuggestedStartLocationHelper {
    /// The start location of a picker: `suggested_start_location` when it is set (or there is no
    /// provider), else the folder of `suggested_start_location_path`, which is created first when
    /// it does not exist and `create_suggested_start_location_directory` is set.
    pub(crate) async fn resolve(
        suggested_start_location: Option<Rc<dyn IStorageFolder>>,
        suggested_start_location_path: Option<String>,
        create_suggested_start_location_directory: bool,
        provider: Option<Rc<dyn IStorageProvider>>,
    ) -> Option<Rc<dyn IStorageFolder>> {
        let Some(provider) = provider.filter(|_| suggested_start_location.is_none()) else {
            return suggested_start_location;
        };

        let suggested_start_location_path = suggested_start_location_path.filter(|path| !path.trim().is_empty())?;

        let uri = Self::try_create_uri(&suggested_start_location_path)?;

        let mut folder = provider.try_get_folder_from_path_async(&uri).await;

        if folder.is_none() && create_suggested_start_location_directory {
            // Failures to create the directory are ignored.
            if Self::try_create_directory(&uri, &suggested_start_location_path) {
                folder = provider.try_get_folder_from_path_async(&uri).await;
            }
        }

        folder
    }

    /// Creates the directory of the location; true when it was created.
    #[cfg(not(target_arch = "wasm32"))]
    fn try_create_directory(uri: &Uri, suggested_start_location_path: &str) -> bool {
        let directory_path = if uri.scheme() == "file" {
            uri.local_path()
        } else {
            match std::path::absolute(suggested_start_location_path) {
                Ok(path) => path.to_string_lossy().into_owned(),
                Err(_) => return false,
            }
        };

        if directory_path.trim().is_empty() || std::path::Path::new(&directory_path).is_dir() {
            return false;
        }

        std::fs::create_dir_all(&directory_path).is_ok()
    }

    /// The browser has no local directories to create.
    #[cfg(target_arch = "wasm32")]
    fn try_create_directory(_uri: &Uri, _suggested_start_location_path: &str) -> bool {
        false
    }

    fn try_create_uri(path: &str) -> Option<Uri> {
        if path.trim().is_empty() {
            return None;
        }

        if let Some(uri) = Uri::try_create(path, UriKind::Absolute) {
            return Some(uri);
        }

        Self::try_create_file_uri(path)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn try_create_file_uri(path: &str) -> Option<Uri> {
        let full_path = std::path::absolute(path).ok()?;
        if !full_path.is_absolute() {
            return None;
        }

        StorageProviderHelpers::try_get_uri_from_file_path(&full_path.to_string_lossy(), false)
    }

    /// The browser has no local paths.
    #[cfg(target_arch = "wasm32")]
    fn try_create_file_uri(_path: &str) -> Option<Uri> {
        None
    }
}
