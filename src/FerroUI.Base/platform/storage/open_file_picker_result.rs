use super::{FilePickerFileType, IStorageFile};
use std::rc::Rc;

/// Extended result of the
/// [`IStorageProvider::open_file_picker_with_result_async`](super::IStorageProvider::open_file_picker_with_result_async)
/// operation.
#[derive(Clone, Default)]
pub struct OpenFilePickerResult {
    /// The list of files picked by the user, empty if the user canceled
    /// the dialog.
    pub files: Vec<Rc<dyn IStorageFile>>,

    /// The file type selected by the user in the dialog.
    ///
    /// This value is `None` if the user canceled the dialog. It can also
    /// be `None` if the platform does not provide this information.
    pub selected_file_type: Option<Rc<FilePickerFileType>>,
}

/// Value equality over the members, each of which is a reference: two
/// results are equal when they hold the same files and the same file type.
impl PartialEq for OpenFilePickerResult {
    fn eq(&self, other: &Self) -> bool {
        self.files.len() == other.files.len()
            && self.files.iter().zip(&other.files).all(|(a, b)| Rc::ptr_eq(a, b))
            && self.selected_file_type == other.selected_file_type
    }
}
