//! Port of `Behaviors/FilesDropHandler.cs`.

use crate::view_models::items::as_item;
use crate::view_models::MainWindowViewModel;
use ferroui_base::input::{DataFormat, DataTransferExtensions, DragEventArgs};
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_xaml_interactions_draganddrop::{DropHandlerBase, IDropHandler};
use std::rc::Rc;

/// The drop handler of the page of the file drop handler: the dropped files are added to the file
/// items of the view model of the window.
#[derive(Default)]
pub struct FilesDropHandler;

impl PartialEq for FilesDropHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DropHandlerBase for FilesDropHandler {
    fn validate(
        &self,
        _sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        e.data_transfer().contains(&DataFormat::file())
            && target_context
                .and_then(as_item::<MainWindowViewModel>)
                .is_some()
    }

    fn execute(
        &self,
        _sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        let vm = target_context.and_then(as_item::<MainWindowViewModel>);
        let (true, Some(vm)) = (e.data_transfer().contains(&DataFormat::file()), vm) else {
            return false;
        };

        let Some(files) = e.data_transfer().try_get_files() else {
            return false;
        };
        if files.is_empty() {
            return false;
        }

        for file in files {
            if let Some(file_items) = vm.file_items() {
                file_items.add(Rc::new(file.path()) as BoxedValue);
            }
        }

        true
    }
}

ferro_markup_type!(class FilesDropHandler {
    this: Rc<FilesDropHandler>,
    handles: [FilesDropHandler, Rc<FilesDropHandler>, Option<Rc<FilesDropHandler>>],
    interfaces: [Rc<dyn IDropHandler>],
    constructors: [() => || Rc::new(FilesDropHandler)],
});
