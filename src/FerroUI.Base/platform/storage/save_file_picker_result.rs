use super::{FilePickerFileType, IStorageFile};
use std::rc::Rc;

/// Extended result of the
/// [`IStorageProvider::save_file_picker_with_result_async`](super::IStorageProvider::save_file_picker_with_result_async)
/// operation.
#[derive(Clone, Default)]
pub struct SaveFilePickerResult {
    /// The file picked by the user, or `None` if the user canceled the
    /// dialog.
    pub file: Option<Rc<dyn IStorageFile>>,

    /// The file type selected by the user in the dialog.
    ///
    /// This value is `None` if the user canceled the dialog. It can also
    /// be `None` if the platform does not provide this information.
    pub selected_file_type: Option<Rc<FilePickerFileType>>,
}

/// Value equality over the members, each of which is a reference: two
/// results are equal when they hold the same file and the same file type.
impl PartialEq for SaveFilePickerResult {
    fn eq(&self, other: &Self) -> bool {
        let same_file = match (&self.file, &other.file) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        same_file && self.selected_file_type == other.selected_file_type
    }
}
