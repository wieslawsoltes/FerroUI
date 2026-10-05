use super::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageProvider, OpenFilePickerResult,
    SaveFilePickerResult, WellKnownFolder,
};
use crate::input::LocalBoxFuture;
use crate::utilities::Uri;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Creates a storage provider, or resolves to `None` when the provider is
/// not available.
pub type StorageProviderFactory = Rc<dyn Fn() -> LocalBoxFuture<Option<Rc<dyn IStorageProvider>>>>;

struct State {
    factories: RefCell<Rc<[StorageProviderFactory]>>,
    providers: RefCell<Vec<Rc<dyn IStorageProvider>>>,
    next_provider_factory: Cell<usize>,
}

impl State {
    /// The provider at `index` of the sequence of available providers: the
    /// ones created so far, followed by the ones the remaining factories
    /// create. `None` once the factories are exhausted.
    async fn get_provider(&self, index: usize) -> Option<Rc<dyn IStorageProvider>> {
        loop {
            if let Some(p) = self.providers.borrow().get(index) {
                return Some(p.clone());
            }

            let factory = {
                let factories = self.factories.borrow();
                factories.get(self.next_provider_factory.get())?.clone()
            };
            let p = factory().await;
            self.next_provider_factory.set(self.next_provider_factory.get() + 1);
            if let Some(p) = p {
                self.providers.borrow_mut().push(p);
            }
        }
    }

    async fn get_for(&self, filter: fn(&dyn IStorageProvider) -> bool) -> std::io::Result<Rc<dyn IStorageProvider>> {
        let mut index = 0;
        while let Some(p) = self.get_provider(index).await {
            if filter(&*p) {
                return Ok(p);
            }
            index += 1;
        }
        Err(std::io::Error::other("Unable to select a suitable storage provider"))
    }

    async fn first_not_null<TResult, F>(&self, cb: F) -> Option<TResult>
    where
        F: Fn(&dyn IStorageProvider) -> LocalBoxFuture<Option<TResult>>,
    {
        let mut index = 0;
        while let Some(p) = self.get_provider(index).await {
            let res = cb(&*p).await;
            if res.is_some() {
                return res;
            }
            index += 1;
        }
        None
    }
}

/// A storage provider that forwards each request to the first of a list of
/// lazily created providers that can serve it.
///
/// This is an implementation detail of the platform backends.
pub struct FallbackStorageProvider {
    state: Rc<State>,
}

impl FallbackStorageProvider {
    /// Creates a provider over the given factories, which are tried in
    /// order and at most once each.
    pub fn new(factories: Vec<StorageProviderFactory>) -> Self {
        Self {
            state: Rc::new(State {
                factories: RefCell::new(factories.into()),
                providers: RefCell::new(Vec::new()),
                next_provider_factory: Cell::new(0),
            }),
        }
    }

    /// Replaces the factories and forgets the providers created so far.
    pub fn reset(&self, factories: Vec<StorageProviderFactory>) {
        *self.state.factories.borrow_mut() = factories.into();
        self.state.providers.borrow_mut().clear();
        self.state.next_provider_factory.set(0);
    }
}

impl IStorageProvider for FallbackStorageProvider {
    // Those should _really_ have been asynchronous,
    // but this class is expected to fall back to the managed implementation anyway
    fn can_open(&self) -> bool {
        true
    }

    fn can_save(&self) -> bool {
        true
    }

    fn can_pick_folder(&self) -> bool {
        true
    }

    fn open_file_picker_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFile>>>> {
        let state = self.state.clone();
        Box::pin(async move { state.get_for(|p| p.can_open()).await?.open_file_picker_async(options).await })
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        let state = self.state.clone();
        Box::pin(
            async move { state.get_for(|p| p.can_open()).await?.open_file_picker_with_result_async(options).await },
        )
    }

    fn save_file_picker_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<Option<Rc<dyn IStorageFile>>>> {
        let state = self.state.clone();
        Box::pin(async move { state.get_for(|p| p.can_save()).await?.save_file_picker_async(options).await })
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        let state = self.state.clone();
        Box::pin(
            async move { state.get_for(|p| p.can_save()).await?.save_file_picker_with_result_async(options).await },
        )
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        let state = self.state.clone();
        Box::pin(async move { state.get_for(|p| p.can_pick_folder()).await?.open_folder_picker_async(options).await })
    }

    fn open_file_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFile>>> {
        let state = self.state.clone();
        let bookmark = bookmark.to_owned();
        Box::pin(async move { state.first_not_null(|p| p.open_file_bookmark_async(&bookmark)).await })
    }

    fn open_folder_bookmark_async(&self, bookmark: &str) -> LocalBoxFuture<Option<Rc<dyn IStorageBookmarkFolder>>> {
        let state = self.state.clone();
        let bookmark = bookmark.to_owned();
        Box::pin(async move { state.first_not_null(|p| p.open_folder_bookmark_async(&bookmark)).await })
    }

    fn try_get_file_from_path_async(&self, file_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        let state = self.state.clone();
        let file_path = file_path.clone();
        Box::pin(async move { state.first_not_null(|p| p.try_get_file_from_path_async(&file_path)).await })
    }

    fn try_get_folder_from_path_async(&self, folder_path: &Uri) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        let state = self.state.clone();
        let folder_path = folder_path.clone();
        Box::pin(async move { state.first_not_null(|p| p.try_get_folder_from_path_async(&folder_path)).await })
    }

    fn try_get_well_known_folder_async(
        &self,
        well_known_folder: WellKnownFolder,
    ) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
        let state = self.state.clone();
        Box::pin(async move { state.first_not_null(|p| p.try_get_well_known_folder_async(well_known_folder)).await })
    }
}
