//! Port of `FilesPreviewBehavior.cs`.

use crate::{DragAndDropEventsBehavior, DragAndDropEventsBehaviorImpl};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{DataFormat, DataTransferExtensions, DragEventArgs};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, DirectProperty, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl,
};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::cell::RefCell;
use std::rc::Rc;

/// Behavior that collects the files dragged over the associated control: the
/// [`preview_files`](Self::preview_files) hold them while the drag is over the control.
#[repr(C)]
pub struct FilesPreviewBehavior {
    base: DragAndDropEventsBehavior,
    preview_files: RefCell<Option<FerroList<Rc<dyn IStorageItem>>>>,
}

ferro_class!(FilesPreviewBehavior: DragAndDropEventsBehavior);
ferro_class_info!(FilesPreviewBehavior { new: FilesPreviewBehavior::new });
ferroui_base::ferro_impl_classes!(FilesPreviewBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl FilesPreviewBehavior {
        /// Identifies the `PreviewFiles` property.
        pub fn preview_files_property() -> DirectProperty<FilesPreviewBehavior, FerroList<Rc<dyn IStorageItem>>> {
            FerroProperty::register_direct::<FilesPreviewBehavior, _>(
                "PreviewFiles",
                |behavior| behavior.preview_files(),
                None,
                FerroList::new(),
            )
        }
    }
}

impl DragAndDropEventsBehaviorImpl for FilesPreviewBehavior {
    fn on_drag_enter(this: &Self, _sender: Option<&BoxedValue>, e: &DragEventArgs) {
        this.update_preview(e);
    }

    fn on_drag_leave(this: &Self, _sender: Option<&BoxedValue>, _e: &DragEventArgs) {
        this.clear_preview();
    }

    fn on_drag_over(this: &Self, _sender: Option<&BoxedValue>, e: &DragEventArgs) {
        this.update_preview(e);
    }

    fn on_drop(this: &Self, _sender: Option<&BoxedValue>, _e: &DragEventArgs) {
        this.clear_preview();
    }
}

impl FilesPreviewBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DragAndDropEventsBehavior::construct(), preview_files: RefCell::new(None) }
    }

    /// Creates the behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the collection of files currently previewed (created on first use).
    pub fn preview_files(&self) -> FerroList<Rc<dyn IStorageItem>> {
        self.preview_files.borrow_mut().get_or_insert_with(FerroList::new).clone()
    }

    fn update_preview(&self, e: &DragEventArgs) {
        if !e.data_transfer().contains(&DataFormat::file()) {
            return;
        }

        let Some(files) = e.data_transfer().try_get_files().filter(|files| !files.is_empty()) else {
            return;
        };

        let list = self.preview_files();
        list.clear();
        for file in files {
            list.add(file);
        }
    }

    fn clear_preview(&self) {
        let preview_files = self.preview_files.borrow().clone();
        if let Some(preview_files) = preview_files {
            preview_files.clear();
        }
    }
}
