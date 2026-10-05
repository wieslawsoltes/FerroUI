use super::{FilePickerFileType, PickerOptions};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// Options class for the
/// [`IStorageProvider::save_file_picker_async`](super::IStorageProvider::save_file_picker_async)
/// method.
#[derive(Clone, Default)]
pub struct FilePickerSaveOptions {
    base: PickerOptions,
    suggested_file_type: Option<Rc<FilePickerFileType>>,
    default_extension: Option<String>,
    file_type_choices: Option<Vec<Rc<FilePickerFileType>>>,
    show_overwrite_prompt: Option<bool>,
}

impl Deref for FilePickerSaveOptions {
    type Target = PickerOptions;

    fn deref(&self) -> &PickerOptions {
        &self.base
    }
}

impl DerefMut for FilePickerSaveOptions {
    fn deref_mut(&mut self) -> &mut PickerOptions {
        &mut self.base
    }
}

impl From<PickerOptions> for FilePickerSaveOptions {
    fn from(base: PickerOptions) -> Self {
        Self { base, ..Self::default() }
    }
}

impl FilePickerSaveOptions {
    /// Creates options with nothing set.
    pub fn new() -> Self {
        Self::default()
    }

    /// The file type that should be preselected when the dialog is opened.
    ///
    /// This value should reference one of the items in
    /// [`file_type_choices`](Self::file_type_choices). If not set, the
    /// first file type may be selected by default.
    pub fn suggested_file_type(&self) -> Option<&Rc<FilePickerFileType>> {
        self.suggested_file_type.as_ref()
    }

    pub fn set_suggested_file_type(&mut self, value: Option<Rc<FilePickerFileType>>) {
        self.suggested_file_type = value;
    }

    /// Sets [`suggested_file_type`](Self::suggested_file_type) and returns
    /// the options.
    pub fn with_suggested_file_type(mut self, value: Rc<FilePickerFileType>) -> Self {
        self.suggested_file_type = Some(value);
        self
    }

    /// The default extension to be used to save the file.
    pub fn default_extension(&self) -> Option<&str> {
        self.default_extension.as_deref()
    }

    pub fn set_default_extension(&mut self, value: Option<String>) {
        self.default_extension = value;
    }

    /// Sets [`default_extension`](Self::default_extension) and returns the
    /// options.
    pub fn with_default_extension(mut self, value: impl Into<String>) -> Self {
        self.default_extension = Some(value.into());
        self
    }

    /// The collection of valid file types that the user can choose to
    /// assign to a file.
    pub fn file_type_choices(&self) -> Option<&[Rc<FilePickerFileType>]> {
        self.file_type_choices.as_deref()
    }

    pub fn set_file_type_choices(&mut self, value: Option<Vec<Rc<FilePickerFileType>>>) {
        self.file_type_choices = value;
    }

    /// Sets [`file_type_choices`](Self::file_type_choices) and returns the
    /// options.
    pub fn with_file_type_choices(mut self, value: Vec<Rc<FilePickerFileType>>) -> Self {
        self.file_type_choices = Some(value);
        self
    }

    /// A value indicating whether the file dialog displays a warning if the
    /// user specifies the name of a file that already exists.
    pub fn show_overwrite_prompt(&self) -> Option<bool> {
        self.show_overwrite_prompt
    }

    pub fn set_show_overwrite_prompt(&mut self, value: Option<bool>) {
        self.show_overwrite_prompt = value;
    }

    /// Sets [`show_overwrite_prompt`](Self::show_overwrite_prompt) and
    /// returns the options.
    pub fn with_show_overwrite_prompt(mut self, value: bool) -> Self {
        self.show_overwrite_prompt = Some(value);
        self
    }
}
