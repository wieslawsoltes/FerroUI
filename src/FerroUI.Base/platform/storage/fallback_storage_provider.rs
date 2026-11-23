use super::{
    FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageBookmarkFile,
    IStorageBookmarkFolder, IStorageFile, IStorageFolder, IStorageProvider, OpenFilePickerResult,
    SaveFilePickerResult, WellKnownFolder,
};
use crate::input::LocalBoxFuture;
use url::Url;
use std::cell::RefCell;
use std::rc::Rc;
use std::task::{Poll, Waker};

/// Creates a storage provider, or resolves to `None` when the provider is
/// not available.
pub type StorageProviderFactory = Rc<dyn Fn() -> LocalBoxFuture<Option<Rc<dyn IStorageProvider>>>>;

/// One factory: not started, running (with the tasks waiting for it), or finished.
enum Slot {
    Empty,
    Running(Vec<Waker>),
    Done(Option<Rc<dyn IStorageProvider>>),
}

type Slots = Rc<RefCell<Vec<Slot>>>;

/// Gives a slot back when its factory future is dropped before it finished, so that the next
/// caller can run the factory again and the callers waiting on it are not left behind.
struct ReleaseOnDrop<'a> {
    slots: &'a Slots,
    index: usize,
}

impl Drop for ReleaseOnDrop<'_> {
    fn drop(&mut self) {
        let mut slots = self.slots.borrow_mut();
        if let Some(slot @ Slot::Running(_)) = slots.get_mut(self.index) {
            if let Slot::Running(wakers) = std::mem::replace(slot, Slot::Empty) {
                wakers.into_iter().for_each(Waker::wake);
            }
        }
    }
}

struct State {
    factories: RefCell<Rc<[StorageProviderFactory]>>,
    slots: RefCell<Slots>,
}

fn empty_slots(count: usize) -> Slots {
    Rc::new(RefCell::new((0..count).map(|_| Slot::Empty).collect()))
}

impl State {
    /// The provider of factory `index`. Each factory runs once: concurrent callers wait for the
    /// same run instead of starting it again and skipping the providers after it.
    async fn provider_of(
        factory: &StorageProviderFactory,
        slots: &Slots,
        index: usize,
    ) -> Option<Rc<dyn IStorageProvider>> {
        loop {
            {
                let mut all = slots.borrow_mut();
                match &all[index] {
                    Slot::Done(provider) => return provider.clone(),
                    Slot::Empty => {
                        all[index] = Slot::Running(Vec::new());
                        break;
                    }
                    Slot::Running(_) => {}
                }
            }

            std::future::poll_fn(|cx| match &mut slots.borrow_mut()[index] {
                Slot::Running(wakers) => {
                    wakers.push(cx.waker().clone());
                    Poll::Pending
                }
                _ => Poll::Ready(()),
            })
            .await;
        }

        let release = ReleaseOnDrop { slots, index };
        let provider = factory().await;
        std::mem::forget(release);

        let previous = std::mem::replace(&mut slots.borrow_mut()[index], Slot::Done(provider.clone()));
        if let Slot::Running(wakers) = previous {
            wakers.into_iter().for_each(Waker::wake);
        }
        provider
    }

    /// The provider at `index` of the sequence of available providers: the factories are asked
    /// in order, and the ones that give no provider are skipped.
    async fn get_provider(&self, index: usize) -> Option<Rc<dyn IStorageProvider>> {
        // Both are captured, so that a reset during the search does not mix two factory lists.
        let factories = self.factories.borrow().clone();
        let slots = self.slots.borrow().clone();

        let mut available = 0;
        for (factory_index, factory) in factories.iter().enumerate() {
            if let Some(provider) = Self::provider_of(factory, &slots, factory_index).await {
                if available == index {
                    return Some(provider);
                }
                available += 1;
            }
        }
        None
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
                slots: RefCell::new(empty_slots(factories.len())),
                factories: RefCell::new(factories.into()),
            }),
        }
    }

    /// Replaces the factories and forgets the providers created so far.
    pub fn reset(&self, factories: Vec<StorageProviderFactory>) {
        *self.state.slots.borrow_mut() = empty_slots(factories.len());
        *self.state.factories.borrow_mut() = factories.into();
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

    fn try_get_file_from_path_async(&self, file_path: &Url) -> LocalBoxFuture<Option<Rc<dyn IStorageFile>>> {
        let state = self.state.clone();
        let file_path = file_path.clone();
        Box::pin(async move { state.first_not_null(|p| p.try_get_file_from_path_async(&file_path)).await })
    }

    fn try_get_folder_from_path_async(&self, folder_path: &Url) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
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
