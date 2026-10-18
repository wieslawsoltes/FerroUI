//! Port of `Internal/TreeDataGrid/TreeDataGridSourceExtensions.cs`: the index of the row a source
//! currently displays for a model.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::ITreeDataGridSource;
use std::rc::Rc;

/// The extensions of the demo on the sources of a tree data grid.
pub trait TreeDataGridSourceExtensions {
    /// The index of the row the source displays for `item`, after its filter and its sort, or -1
    /// when no displayed row shows it or `item` is null. Models are compared by identity
    /// (`ReferenceEquals`), never by value.
    fn find_displayed_row_index(&self, item: Option<&BoxedValue>) -> i32;
}

impl<S: ITreeDataGridSource + ?Sized> TreeDataGridSourceExtensions for S {
    fn find_displayed_row_index(&self, item: Option<&BoxedValue>) -> i32 {
        let Some(item) = item else {
            return -1;
        };

        let item = object_address(item);
        let rows = self.rows();
        for index in 0..rows.count() {
            if rows.get(index).model().is_some_and(|model| object_address(&model) == item) {
                return index as i32;
            }
        }

        -1
    }
}

/// The address of the object a value is: a model boxed as its handle (`Rc<M>` in the box) is the
/// object the handle refers to, the form in which the rows of a source hold their models.
fn object_address(value: &BoxedValue) -> *const () {
    let object = ValueTypes::reference_object(value).unwrap_or_else(|| value.clone());
    Rc::as_ptr(&object) as *const ()
}
