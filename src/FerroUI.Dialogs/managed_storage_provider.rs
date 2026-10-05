use crate::internal::{ManagedFileChooserFilterViewModel, ManagedFileChooserViewModel};
use crate::task_completion_source::TaskCompletionSource;
use crate::{ManagedFileChooser, ManagedFileChooserOverwritePrompt, ManagedFileDialogOptions};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::layout::Layoutable;
use ferroui_base::platform::storage::file_io::{path, BclStorageFile, BclStorageFolder, BclStorageProvider, FileSystemInfo};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFile,
    IStorageFolder, OpenFilePickerResult, SaveFilePickerResult,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::primitives::Popup;
use ferroui_controls::{ContentControl, Control, Flyout, Panel, PlacementMode, TopLevel, Window, WindowBase};
use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;

/// The storage provider that shows the managed file chooser: in a window,
/// or in a popup over the content of a top level that has no windows.
///
/// Internal upstream; public here for the platform backends.
///
/// The top level is held weakly: the provider is usually the storage
/// provider of that top level, which keeps it.
#[derive(Clone)]
pub struct ManagedStorageProvider {
    parent: Option<WeakRef<TopLevel>>,
    managed_options: ManagedFileDialogOptions,
}

impl ManagedStorageProvider {
    pub fn new(parent: Option<&Ref<TopLevel>>, managed_options: Option<ManagedFileDialogOptions>) -> Self {
        Self { parent: parent.map(Ref::downgrade), managed_options: managed_options.unwrap_or_default() }
    }

    fn parent(&self) -> Option<Ref<TopLevel>> {
        self.parent.as_ref().and_then(WeakRef::upgrade)
    }

    fn prepare_root(&self, model: &Rc<ManagedFileChooserViewModel>) -> Ref<ContentControl> {
        let root = match self.managed_options.content_root_factory() {
            Some(factory) => factory(),
            None => {
                if self.parent().is_some_and(|parent| !parent.is::<Window>()) {
                    ContentControl::new()
                } else {
                    Window::new().upcast()
                }
            }
        };

        root.set_content(Some(Control::boxed(ManagedFileChooser::new())));
        root.set_data_context(Some(model.clone() as BoxedValue));

        root
    }

    async fn show(&self, model: Rc<ManagedFileChooserViewModel>) -> io::Result<Vec<String>> {
        let root = self.prepare_root(&model);

        if let Some(window) = root.cast::<Window>() {
            Ok(self.show_as_window(window, model).await)
        } else if let Some(parent) = self.parent() {
            self.show_as_popup(&parent, root, model).await
        } else {
            Err(io::Error::other("Managed File Chooser requires existing parent or compatible windowing system."))
        }
    }

    async fn show_as_window(&self, window: Ref<Window>, model: Rc<ManagedFileChooserViewModel>) -> Vec<String> {
        let tcs = TaskCompletionSource::<bool>::new();
        window.set_title(model.title());
        {
            let model = model.clone();
            let tcs = tcs.clone();
            window.closed(move || {
                model.cancel();
                tcs.try_set_result(true);
            });
        }

        let result: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        // The handlers of the model hold the window weakly: the window holds the model as its
        // data context.
        {
            let result = result.clone();
            let weak = window.downgrade();
            model.complete_requested(move |items| {
                *result.borrow_mut() = items.to_vec();
                if let Some(window) = weak.upgrade() {
                    window.close();
                }
            });
        }

        {
            let result = result.clone();
            let weak = window.downgrade();
            model.overwrite_prompt(move |filename| {
                let filename = filename.to_string();
                let result = result.clone();
                let weak = weak.clone();
                Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                    let Some(window) = weak.upgrade() else { return };
                    if Self::show_overwrite_prompt(&filename, &window.clone().upcast()).await {
                        *result.borrow_mut() = vec![filename];
                        window.close();
                    }
                });
            });
        }

        {
            let weak = window.downgrade();
            model.cancel_requested(move || {
                if let Some(window) = weak.upgrade() {
                    window.close();
                }
            });
        }

        match self.parent().and_then(|parent| parent.cast::<Window>()) {
            Some(parent) => {
                // The dialog result is not used: the outcome comes from the model.
                let _ = window.show_dialog(&parent).await;
            }
            None => WindowBase::show(&window),
        }

        tcs.task().await;

        let result = result.borrow().clone();
        result
    }

    async fn show_as_popup(
        &self,
        parent: &Ref<TopLevel>,
        root: Ref<ContentControl>,
        model: Rc<ManagedFileChooserViewModel>,
    ) -> io::Result<Vec<String>> {
        let tcs = TaskCompletionSource::<bool>::new();
        let root_panel = parent
            .find_descendant_of_type::<Panel>(false)
            .ok_or_else(|| io::Error::other("The top level has no panel to host the managed file chooser."))?;

        let popup = Popup::new();
        popup.set_placement(PlacementMode::Center);
        popup.set_is_light_dismiss_enabled(false);
        popup.set_child(root.clone());
        popup.set_width(Layoutable::width(parent));
        popup.set_height(Layoutable::height(parent));

        {
            let model = model.clone();
            let tcs = tcs.clone();
            popup.closed(move || {
                model.cancel();
                tcs.try_set_result(true);
            });
        }

        let result: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        {
            let result = result.clone();
            let weak = popup.downgrade();
            model.complete_requested(move |items| {
                *result.borrow_mut() = items.to_vec();
                if let Some(popup) = weak.upgrade() {
                    popup.close();
                }
            });
        }

        {
            let result = result.clone();
            let weak_popup = popup.downgrade();
            let weak_root = root.downgrade();
            model.overwrite_prompt(move |filename| {
                let filename = filename.to_string();
                let result = result.clone();
                let weak_popup = weak_popup.clone();
                let weak_root = weak_root.clone();
                Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                    let Some(root) = weak_root.upgrade() else { return };
                    if Self::show_overwrite_prompt(&filename, &root).await {
                        *result.borrow_mut() = vec![filename];
                        if let Some(popup) = weak_popup.upgrade() {
                            popup.close();
                        }
                    }
                });
            });
        }

        {
            let weak = popup.downgrade();
            model.cancel_requested(move || {
                if let Some(popup) = weak.upgrade() {
                    popup.close();
                }
            });
        }

        root_panel.children().add(popup.clone());

        // `ParentOnSizeChanged`: unsubscribes itself once the popup is closed, and keeps the
        // popup the size of the top level.
        let token: Rc<Cell<Option<RoutedEventHandlerToken>>> = Rc::new(Cell::new(None));
        {
            let weak_popup = popup.downgrade();
            let weak_parent = parent.downgrade();
            let handler_token = token.clone();
            token.set(Some(parent.size_changed(move |_, _| {
                let (Some(popup), Some(parent)) = (weak_popup.upgrade(), weak_parent.upgrade()) else { return };
                if !popup.is_open() {
                    if let Some(token) = handler_token.take() {
                        parent.remove_handler(Control::size_changed_event(), token);
                    }
                }

                popup.set_width(Layoutable::width(&parent));
                popup.set_height(Layoutable::height(&parent));
            })));
        }

        popup.open();
        tcs.task().await;

        root_panel.children().remove(popup.clone());
        if let Some(token) = token.take() {
            parent.remove_handler(Control::size_changed_event(), token);
        }

        let result = result.borrow().clone();
        Ok(result)
    }

    async fn show_overwrite_prompt(filename: &str, root: &Ref<ContentControl>) -> bool {
        let tcs = TaskCompletionSource::<bool>::new();
        let prompt = ManagedFileChooserOverwritePrompt::new();
        prompt.set_file_name(path::get_file_name(filename).to_string());
        {
            let tcs = tcs.clone();
            prompt.result(move |r| {
                tcs.try_set_result(r);
            });
        }

        let flyout = Flyout::new();
        {
            let tcs = tcs.clone();
            flyout.closed(move || {
                tcs.try_set_result(false);
            });
        }
        flyout.set_content(Some(Control::boxed(prompt)));
        flyout.set_placement(PlacementMode::Center);
        FlyoutBase::show_at(&flyout, root);

        let prompt_result = tcs.task().await;
        FlyoutBase::hide(&flyout);

        prompt_result
    }

    fn try_get_selected_file_type(
        file_types: Option<&[Rc<FilePickerFileType>]>,
        selected_file_type: Option<&Rc<ManagedFileChooserFilterViewModel>>,
    ) -> Option<Rc<FilePickerFileType>> {
        let file_types = file_types?;
        let index = selected_file_type?.index();
        if index >= 0 && (index as usize) < file_types.len() {
            Some(file_types[index as usize].clone())
        } else {
            None
        }
    }
}

impl BclStorageProvider for ManagedStorageProvider {
    fn can_open(&self) -> bool {
        true
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<OpenFilePickerResult>> {
        let this = self.clone();
        Box::pin(async move {
            let model = ManagedFileChooserViewModel::for_open_file(&options, this.managed_options.clone());
            let results = this.show(model.clone()).await?;

            let files = results
                .iter()
                .map(|f| BclStorageFile::new(FileSystemInfo::file(f)) as Rc<dyn IStorageFile>)
                .collect();
            let file_type = Self::try_get_selected_file_type(options.file_type_filter(), model.selected_filter().as_ref());

            Ok(OpenFilePickerResult { files, selected_file_type: file_type })
        })
    }

    fn can_save(&self) -> bool {
        true
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<io::Result<SaveFilePickerResult>> {
        let this = self.clone();
        Box::pin(async move {
            let model = ManagedFileChooserViewModel::for_save_file(&options, this.managed_options.clone());
            let results = this.show(model.clone()).await?;

            let file = results.first().map(|result| BclStorageFile::new(FileSystemInfo::file(result)) as Rc<dyn IStorageFile>);
            let file_type = Self::try_get_selected_file_type(options.file_type_choices(), model.selected_filter().as_ref());

            Ok(SaveFilePickerResult { file, selected_file_type: file_type })
        })
    }

    fn can_pick_folder(&self) -> bool {
        true
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        let this = self.clone();
        Box::pin(async move {
            let model = ManagedFileChooserViewModel::for_open_folder(&options, this.managed_options.clone());
            let results = this.show(model).await?;

            Ok(results
                .iter()
                .map(|f| BclStorageFolder::new(FileSystemInfo::directory(f)) as Rc<dyn IStorageFolder>)
                .collect())
        })
    }
}
