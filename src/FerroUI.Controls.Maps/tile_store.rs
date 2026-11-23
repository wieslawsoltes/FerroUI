//! The tiles of the view: decoded in memory, encoded on disk, and loaded a few at a time, nearest
//! the middle of the view first.

use crate::projection::TileId;
use crate::tile_cache::LruBudget;
use crate::tile_source::{RasterTileSource, SharedTileLoader, TileError};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};

/// How many tiles load at once.
pub(crate) const MAX_LOADS: usize = 6;
/// How often a tile whose load failed is tried again, until the source changes.
pub(crate) const MAX_ATTEMPTS: u8 = 3;
/// How many levels up a tile that is not there yet is drawn from.
const ANCESTOR_LEVELS: u8 = 6;
/// The decoded tiles kept in memory: about 370 tiles of 256 by 256 pixels.
pub(crate) const MEMORY_BUDGET: u64 = 96 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod disk {
    use crate::projection::TileId;
    use crate::tile_cache::DiskTileCache;
    use ferroui_system::task::oneshot;
    use std::path::Path;
    use std::sync::mpsc;

    enum Job {
        Get(TileId, oneshot::Sender<Option<Vec<u8>>>),
        Put(TileId, Vec<u8>),
    }

    /// The disk cache on a thread of its own: the UI thread does no file I/O.
    pub(crate) struct DiskWorker {
        jobs: mpsc::Sender<Job>,
    }

    impl DiskWorker {
        pub(crate) fn open(folder: &Path, source: &str, budget: u64) -> std::io::Result<Self> {
            let mut cache = DiskTileCache::open(folder, source, budget)?;
            let (jobs, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("ferroui-map-tiles".to_owned())
                .spawn(move || {
                    for job in receiver {
                        match job {
                            Job::Get(tile, answer) => {
                                let _ = answer.send(cache.get(tile));
                            }
                            // A tile that cannot be stored is loaded again next time.
                            Job::Put(tile, bytes) => {
                                let _ = cache.put(tile, &bytes);
                            }
                        }
                    }
                })?;
            Ok(Self { jobs })
        }

        pub(crate) fn get(&self, tile: TileId) -> Option<oneshot::Receiver<Option<Vec<u8>>>> {
            let (answer, receiver) = oneshot::channel();
            self.jobs.send(Job::Get(tile, answer)).ok()?;
            Some(receiver)
        }

        pub(crate) fn put(&self, tile: TileId, bytes: Vec<u8>) {
            let _ = self.jobs.send(Job::Put(tile, bytes));
        }
    }
}

type Decode<T> = Box<dyn Fn(&[u8]) -> Option<(T, u64)>>;

struct Inner<T: 'static> {
    source: RefCell<Option<Rc<RasterTileSource>>>,
    loader: RefCell<Option<SharedTileLoader>>,
    decode: Decode<T>,
    on_loaded: RefCell<Option<Rc<dyn Fn()>>>,
    memory: RefCell<LruBudget<TileId, Rc<T>>>,
    wanted: RefCell<VecDeque<TileId>>,
    loading: RefCell<HashSet<TileId>>,
    missing: RefCell<HashSet<TileId>>,
    attempts: RefCell<HashMap<TileId, u8>>,
    /// Raised when the source changes: what is still loading for the old one is dropped.
    generation: Cell<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    disk: RefCell<Option<Rc<disk::DiskWorker>>>,
}

/// The tiles of one map.
pub(crate) struct TileStore<T: 'static> {
    inner: Rc<Inner<T>>,
}

impl<T: 'static> TileStore<T> {
    /// `decode` makes a tile and tells its size in bytes; `None` for data that is not an image.
    pub(crate) fn new(budget: u64, decode: impl Fn(&[u8]) -> Option<(T, u64)> + 'static) -> Self {
        Self {
            inner: Rc::new(Inner {
                source: RefCell::new(None),
                loader: RefCell::new(None),
                decode: Box::new(decode),
                on_loaded: RefCell::new(None),
                memory: RefCell::new(LruBudget::new(budget)),
                wanted: RefCell::new(VecDeque::new()),
                loading: RefCell::new(HashSet::new()),
                missing: RefCell::new(HashSet::new()),
                attempts: RefCell::new(HashMap::new()),
                generation: Cell::new(0),
                #[cfg(not(target_arch = "wasm32"))]
                disk: RefCell::new(None),
            }),
        }
    }

    /// Called after a tile arrived.
    pub(crate) fn set_on_loaded(&self, action: impl Fn() + 'static) {
        *self.inner.on_loaded.borrow_mut() = Some(Rc::new(action));
    }

    /// Changes where tiles come from; what the store holds belongs to the old source.
    pub(crate) fn set_source(
        &self,
        source: Option<Rc<RasterTileSource>>,
        loader: Option<SharedTileLoader>,
    ) {
        let inner = &self.inner;
        inner.generation.set(inner.generation.get() + 1);
        *inner.source.borrow_mut() = source;
        *inner.loader.borrow_mut() = loader;
        inner.memory.borrow_mut().clear();
        inner.wanted.borrow_mut().clear();
        inner.loading.borrow_mut().clear();
        inner.missing.borrow_mut().clear();
        inner.attempts.borrow_mut().clear();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_disk(&self, worker: Option<disk::DiskWorker>) {
        *self.inner.disk.borrow_mut() = worker.map(Rc::new);
    }

    /// The tile, or the nearest ancestor that is there, with the part of it the tile covers:
    /// left, top and side, from 0 to 1.
    pub(crate) fn best(&self, tile: TileId) -> Option<(Rc<T>, (f64, f64, f64))> {
        let mut memory = self.inner.memory.borrow_mut();
        let deepest = tile.zoom.saturating_sub(ANCESTOR_LEVELS);
        for zoom in (deepest..=tile.zoom).rev() {
            let ancestor = tile.ancestor(zoom)?;
            if let Some(value) = memory.get(&ancestor) {
                return Some((value.clone(), tile.part_of_ancestor(zoom)?));
            }
        }
        None
    }

    /// Replaces what is wanted, most important first, and starts loading.
    pub(crate) fn request(&self, tiles: impl IntoIterator<Item = TileId>) {
        let inner = &self.inner;
        {
            let memory = inner.memory.borrow();
            let loading = inner.loading.borrow();
            let missing = inner.missing.borrow();
            let mut wanted = inner.wanted.borrow_mut();
            wanted.clear();
            wanted.extend(tiles.into_iter().filter(|tile| {
                !memory.contains(tile) && !loading.contains(tile) && !missing.contains(tile)
            }));
        }
        Inner::pump(inner);
    }

    #[cfg(test)]
    pub(crate) fn loading_count(&self) -> usize {
        self.inner.loading.borrow().len()
    }
}

impl<T: 'static> Inner<T> {
    fn pump(this: &Rc<Self>) {
        while this.loading.borrow().len() < MAX_LOADS {
            let tile = this.wanted.borrow_mut().pop_front();
            let Some(tile) = tile else { break };
            Self::start(this, tile);
        }
    }

    fn start(this: &Rc<Self>, tile: TileId) {
        let source = this.source.borrow().clone();
        let loader = this.loader.borrow().clone();
        let (Some(source), Some(loader)) = (source, loader) else {
            return;
        };
        this.loading.borrow_mut().insert(tile);
        let address = source.address(tile);
        let generation = this.generation.get();
        let weak: Weak<Self> = Rc::downgrade(this);
        #[cfg(not(target_arch = "wasm32"))]
        let stored = this.disk.borrow().as_ref().and_then(|disk| disk.get(tile));
        let task = async move {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(stored) = stored {
                if let Ok(Some(bytes)) = stored.await {
                    if let Some(this) = weak.upgrade() {
                        Self::finish(&this, generation, tile, Ok(bytes), false);
                    }
                    return;
                }
            }
            let loaded = loader.load(&address, tile).await;
            if let Some(this) = weak.upgrade() {
                Self::finish(&this, generation, tile, loaded, true);
            }
        };
        Dispatcher::ui_thread().spawn_local_with_priority(task, DispatcherPriority::DEFAULT);
    }

    fn finish(
        this: &Rc<Self>,
        generation: u64,
        tile: TileId,
        loaded: Result<Vec<u8>, TileError>,
        is_new: bool,
    ) {
        if generation != this.generation.get() {
            return;
        }
        this.loading.borrow_mut().remove(&tile);
        let mut arrived = false;
        match loaded {
            Ok(bytes) => match (this.decode)(&bytes) {
                Some((value, size)) => {
                    let removed = this.memory.borrow_mut().insert(tile, Rc::new(value), size);
                    drop(removed);
                    this.attempts.borrow_mut().remove(&tile);
                    arrived = true;
                    #[cfg(not(target_arch = "wasm32"))]
                    if is_new {
                        if let Some(disk) = this.disk.borrow().as_ref() {
                            disk.put(tile, bytes);
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    let _ = is_new;
                }
                None => {
                    this.missing.borrow_mut().insert(tile);
                }
            },
            Err(TileError::NotFound) => {
                this.missing.borrow_mut().insert(tile);
            }
            Err(TileError::Failed(_)) => {
                let attempts = {
                    let mut attempts = this.attempts.borrow_mut();
                    let count = attempts.entry(tile).or_insert(0);
                    *count += 1;
                    *count
                };
                if attempts >= MAX_ATTEMPTS {
                    this.missing.borrow_mut().insert(tile);
                }
            }
        }
        Self::pump(this);
        if arrived {
            let action = this.on_loaded.borrow().clone();
            if let Some(action) = action {
                action();
            }
        }
    }
}
