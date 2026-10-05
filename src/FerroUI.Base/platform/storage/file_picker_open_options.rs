use super::{FilePickerFileType, PickerOptions};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// Options class for the
/// [`IStorageProvider::open_file_picker_async`](super::IStorageProvider::open_file_picker_async)
/// method.
#[derive(Clone, Default)]
pub struct FilePickerOpenOptions {
    base: PickerOptions,
    suggested_file_type: Option<Rc<FilePickerFileType>>,
    allow_multiple: bool,
    file_type_filter: Option<Vec<Rc<FilePickerFileType>>>,
}

impl Deref for FilePickerOpenOptions {
    type Target = PickerOptions;

    fn deref(&self) -> &PickerOptions {
        &self.base
    }
}

impl DerefMut for FilePickerOpenOptions {
    fn deref_mut(&mut self) -> &mut PickerOptions {
        &mut self.base
    }
}

impl From<PickerOptions> for FilePickerOpenOptions {
    fn from(base: PickerOptions) -> Self {
        Self { base, ..Self::default() }
    }
}

impl FilePickerOpenOptions {
    /// Creates options with nothing set.
    pub fn new() -> Self {
        Self::default()
    }

    /// The file type that should be preselected when the dialog is opened.
    ///
    /// This value should reference one of the items in
    /// [`file_type_filter`](Self::file_type_filter). If not set, the first
    /// file type may be selected by default.
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

    /// An option indicating whether the open picker allows users to select
    /// multiple files.
    pub fn allow_multiple(&self) -> bool {
        self.allow_multiple
    }

    pub fn set_allow_multiple(&mut self, value: bool) {
        self.allow_multiple = value;
    }

    /// Sets [`allow_multiple`](Self::allow_multiple) and returns the
    /// options.
    pub fn with_allow_multiple(mut self, value: bool) -> Self {
        self.allow_multiple = value;
        self
    }

    /// The collection of file types that the file open picker displays.
    pub fn file_type_filter(&self) -> Option<&[Rc<FilePickerFileType>]> {
        self.file_type_filter.as_deref()
    }

    pub fn set_file_type_filter(&mut self, value: Option<Vec<Rc<FilePickerFileType>>>) {
        self.file_type_filter = value;
    }

    /// Sets [`file_type_filter`](Self::file_type_filter) and returns the
    /// options.
    pub fn with_file_type_filter(mut self, value: Vec<Rc<FilePickerFileType>>) -> Self {
        self.file_type_filter = Some(value);
        self
    }
}
