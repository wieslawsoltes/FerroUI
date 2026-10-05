use super::IStorageFolder;
use std::rc::Rc;

/// Common options for the picker dialogs.
#[derive(Clone, Default)]
pub struct PickerOptions {
    title: Option<String>,
    suggested_start_location: Option<Rc<dyn IStorageFolder>>,
    suggested_file_name: Option<String>,
}

impl PickerOptions {
    /// Creates options with nothing set.
    pub fn new() -> Self {
        Self::default()
    }

    /// The text that appears in the title bar of a picker.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn set_title(&mut self, value: Option<String>) {
        self.title = value;
    }

    /// Sets [`title`](Self::title) and returns the options.
    pub fn with_title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    /// The initial location where the file open picker looks for files to
    /// present to the user. Can be obtained from previously picked folder
    /// or using [`IStorageProvider::try_get_folder_from_path_async`](super::IStorageProvider::try_get_folder_from_path_async)
    /// or [`IStorageProvider::try_get_well_known_folder_async`](super::IStorageProvider::try_get_well_known_folder_async).
    pub fn suggested_start_location(&self) -> Option<&Rc<dyn IStorageFolder>> {
        self.suggested_start_location.as_ref()
    }

    pub fn set_suggested_start_location(&mut self, value: Option<Rc<dyn IStorageFolder>>) {
        self.suggested_start_location = value;
    }

    /// Sets [`suggested_start_location`](Self::suggested_start_location)
    /// and returns the options.
    pub fn with_suggested_start_location(mut self, value: Rc<dyn IStorageFolder>) -> Self {
        self.suggested_start_location = Some(value);
        self
    }

    /// The file name that the file picker suggests to the user.
    pub fn suggested_file_name(&self) -> Option<&str> {
        self.suggested_file_name.as_deref()
    }

    pub fn set_suggested_file_name(&mut self, value: Option<String>) {
        self.suggested_file_name = value;
    }

    /// Sets [`suggested_file_name`](Self::suggested_file_name) and returns
    /// the options.
    pub fn with_suggested_file_name(mut self, value: impl Into<String>) -> Self {
        self.suggested_file_name = Some(value.into());
        self
    }
}
