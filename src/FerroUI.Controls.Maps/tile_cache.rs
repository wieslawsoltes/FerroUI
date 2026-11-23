//! The tile caches: least recently used entries go first, within a budget of bytes.

use crate::projection::TileId;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::Hash;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A map of least recently used entries with a budget of bytes.
pub struct LruBudget<K, V> {
    entries: HashMap<K, Entry<V>>,
    order: BTreeMap<u64, K>,
    clock: u64,
    used: u64,
    budget: u64,
}

struct Entry<V> {
    value: V,
    size: u64,
    stamp: u64,
}

impl<K: Clone + Eq + Hash, V> LruBudget<K, V> {
    pub fn new(budget: u64) -> Self {
        Self {
            entries: HashMap::new(),
            order: BTreeMap::new(),
            clock: 0,
            used: 0,
            budget,
        }
    }

    pub fn used(&self) -> u64 {
        self.used
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// The value, now the most recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let stamp = self.tick();
        let entry = self.entries.get_mut(key)?;
        self.order.remove(&entry.stamp);
        entry.stamp = stamp;
        self.order.insert(stamp, key.clone());
        Some(&entry.value)
    }

    /// Adds a value and returns what left to make room. A value over the whole budget is not
    /// kept and comes back with them.
    pub fn insert(&mut self, key: K, value: V, size: u64) -> Vec<(K, V)> {
        let mut removed = Vec::new();
        if let Some(old) = self.remove(&key) {
            removed.push((key.clone(), old));
        }
        if size > self.budget {
            removed.push((key, value));
            return removed;
        }
        while self.used + size > self.budget {
            let Some((_, oldest)) = self.order.pop_first() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.used -= entry.size;
                removed.push((oldest, entry.value));
            }
        }
        let stamp = self.tick();
        self.order.insert(stamp, key.clone());
        self.entries.insert(key, Entry { value, size, stamp });
        self.used += size;
        removed
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let entry = self.entries.remove(key)?;
        self.order.remove(&entry.stamp);
        self.used -= entry.size;
        Some(entry.value)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.used = 0;
    }
}

/// A stable name for a source, for its folder on disk: FNV-1a of its template.
pub fn source_key(template: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in template.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Encoded tiles in a folder. Its calls do file I/O: run them off the UI thread.
pub struct DiskTileCache {
    root: PathBuf,
    index: LruBudget<TileId, ()>,
}

impl DiskTileCache {
    /// Opens the cache of one source; `source` comes from [`source_key`]. What is already there
    /// is indexed, oldest first, and what is over the budget is removed.
    pub fn open(folder: &Path, source: &str, budget: u64) -> io::Result<Self> {
        let root = folder.join(source);
        fs::create_dir_all(&root)?;
        let mut found = Vec::new();
        scan(&root, &mut found)?;
        found.sort_by_key(|file: &Found| file.modified);
        let mut cache = Self {
            root,
            index: LruBudget::new(budget),
        };
        for file in found {
            for (old, ()) in cache.index.insert(file.tile, (), file.size) {
                // A file that cannot be removed now is only space; it is overwritten or retried.
                let _ = fs::remove_file(cache.path(old));
            }
        }
        Ok(cache)
    }

    fn path(&self, tile: TileId) -> PathBuf {
        self.root
            .join(tile.zoom.to_string())
            .join(tile.x.to_string())
            .join(format!("{}.tile", tile.y))
    }

    pub fn used(&self) -> u64 {
        self.index.used()
    }

    /// The encoded tile. A file that went away or cannot be read is a miss.
    pub fn get(&mut self, tile: TileId) -> Option<Vec<u8>> {
        self.index.get(&tile)?;
        match fs::read(self.path(tile)) {
            Ok(bytes) => Some(bytes),
            Err(_) => {
                self.index.remove(&tile);
                None
            }
        }
    }

    /// Stores a tile. The file appears whole or not at all, also if the process stops halfway.
    pub fn put(&mut self, tile: TileId, bytes: &[u8]) -> io::Result<()> {
        let path = self.path(tile);
        let folder = path.parent().expect("a tile path has a folder");
        fs::create_dir_all(folder)?;
        let number = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let temporary = folder.join(format!("{}.{}.{number}.tmp", tile.y, std::process::id()));
        let written = (|| {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(bytes)?;
            file.sync_data()?;
            fs::rename(&temporary, &path)
        })();
        if let Err(error) = written {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        for (old, ()) in self.index.insert(tile, (), bytes.len() as u64) {
            if old != tile || bytes.len() as u64 > self.index.budget {
                let _ = fs::remove_file(self.path(old));
            }
        }
        Ok(())
    }
}

struct Found {
    tile: TileId,
    size: u64,
    modified: std::time::SystemTime,
}

/// Finds `<zoom>/<x>/<y>.tile` files; anything else in the folder is left alone.
fn scan(root: &Path, found: &mut Vec<Found>) -> io::Result<()> {
    for zoom_entry in fs::read_dir(root)? {
        let zoom_entry = zoom_entry?;
        let Some(zoom) = number::<u8>(&zoom_entry.file_name()) else {
            continue;
        };
        if !zoom_entry.file_type()?.is_dir() {
            continue;
        }
        for x_entry in fs::read_dir(zoom_entry.path())? {
            let x_entry = x_entry?;
            let Some(x) = number::<u32>(&x_entry.file_name()) else {
                continue;
            };
            if !x_entry.file_type()?.is_dir() {
                continue;
            }
            for file in fs::read_dir(x_entry.path())? {
                let file = file?;
                let name = file.file_name();
                let Some(y) = name
                    .to_str()
                    .and_then(|name| name.strip_suffix(".tile"))
                    .and_then(|y| y.parse().ok())
                else {
                    continue;
                };
                let metadata = file.metadata()?;
                if metadata.is_file() {
                    found.push(Found {
                        tile: TileId { zoom, x, y },
                        size: metadata.len(),
                        modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
                    });
                }
            }
        }
    }
    Ok(())
}

fn number<T: std::str::FromStr>(name: &std::ffi::OsStr) -> Option<T> {
    name.to_str()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn tile(y: u32) -> TileId {
        TileId { zoom: 4, x: 3, y }
    }

    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("ferroui-map-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn the_least_recently_used_entry_goes_first() {
        let mut cache = LruBudget::new(30);
        assert!(cache.insert(1, "a", 10).is_empty());
        assert!(cache.insert(2, "b", 10).is_empty());
        assert!(cache.insert(3, "c", 10).is_empty());
        assert_eq!(cache.get(&1), Some(&"a"));
        assert_eq!(cache.insert(4, "d", 10), [(2, "b")]);
        assert!(cache.contains(&1));
        assert_eq!(cache.used(), 30);
    }

    #[test]
    fn replacing_an_entry_counts_its_size_once() {
        let mut cache = LruBudget::new(30);
        cache.insert(1, "a", 10);
        assert_eq!(cache.insert(1, "b", 20), [(1, "a")]);
        assert_eq!(cache.used(), 20);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn a_value_over_the_budget_is_not_kept() {
        let mut cache = LruBudget::new(10);
        cache.insert(1, "a", 5);
        assert_eq!(cache.insert(2, "b", 11), [(2, "b")]);
        assert!(cache.contains(&1));
        assert_eq!(cache.used(), 5);
    }

    #[test]
    fn the_source_key_is_stable() {
        assert_eq!(source_key(""), "cbf29ce484222325");
        assert_ne!(
            source_key("https://a.test/{z}"),
            source_key("https://b.test/{z}")
        );
    }

    #[test]
    fn tiles_survive_reopening_the_cache() {
        let root = folder("reopen");
        let mut cache = DiskTileCache::open(&root, "source", 100).unwrap();
        cache.put(tile(1), b"first").unwrap();
        assert_eq!(cache.get(tile(1)).as_deref(), Some(&b"first"[..]));
        drop(cache);
        let mut cache = DiskTileCache::open(&root, "source", 100).unwrap();
        assert_eq!(cache.used(), 5);
        assert_eq!(cache.get(tile(1)).as_deref(), Some(&b"first"[..]));
        assert_eq!(cache.get(tile(2)), None);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_disk_budget_removes_old_files() {
        let root = folder("budget");
        let mut cache = DiskTileCache::open(&root, "source", 10).unwrap();
        cache.put(tile(1), b"12345").unwrap();
        cache.put(tile(2), b"12345").unwrap();
        cache.put(tile(3), b"12345").unwrap();
        assert_eq!(cache.get(tile(1)), None);
        assert!(!cache.path(tile(1)).exists());
        assert!(cache.path(tile(3)).exists());
        assert_eq!(cache.used(), 10);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rewriting_a_tile_keeps_its_file() {
        let root = folder("rewrite");
        let mut cache = DiskTileCache::open(&root, "source", 100).unwrap();
        cache.put(tile(1), b"old").unwrap();
        cache.put(tile(1), b"newer").unwrap();
        assert_eq!(cache.get(tile(1)).as_deref(), Some(&b"newer"[..]));
        assert_eq!(cache.used(), 5);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_file_that_went_away_is_a_miss() {
        let root = folder("missing");
        let mut cache = DiskTileCache::open(&root, "source", 100).unwrap();
        cache.put(tile(1), b"data").unwrap();
        fs::remove_file(cache.path(tile(1))).unwrap();
        assert_eq!(cache.get(tile(1)), None);
        assert_eq!(cache.used(), 0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn other_files_in_the_folder_are_left_alone() {
        let root = folder("foreign");
        let source = root.join("source").join("4").join("3");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("notes.txt"), b"keep").unwrap();
        fs::write(source.join("9.tile"), b"tile").unwrap();
        let mut cache = DiskTileCache::open(&root, "source", 100).unwrap();
        assert_eq!(cache.used(), 4);
        assert!(cache.get(tile(9)).is_some());
        assert!(source.join("notes.txt").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
