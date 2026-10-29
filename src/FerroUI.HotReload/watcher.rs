//! Portable content watcher. Reading by path each poll survives editor atomic-save renames.
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
#[derive(Debug)]
pub struct FileChange {
    pub uri: String,
    pub contents: Result<String, String>,
}
/// One stable read of the entire watched set. Includes unchanged documents so a
/// failed/missing dependency can recover without losing edits observed beside it.
#[derive(Debug)]
pub struct FileBatch {
    pub files: Vec<FileChange>,
    pub diagnostics: Vec<String>,
    /// The watcher invalidates this as soon as a different read is observed,
    /// including an unstable/partial save which has not passed debounce yet.
    /// Editor-owned snapshots without a watcher can use `None`.
    pub revision: Option<WatchRevision>,
}
#[derive(Clone, Debug)]
pub struct WatchRevision {
    current: Arc<AtomicU64>,
    observed: u64,
}
impl WatchRevision {
    pub fn is_current(&self) -> bool {
        self.current.load(Ordering::Acquire) == self.observed
    }
}
/// A document directory and its canonical assembly URI prefix (ending in `/`).
#[derive(Clone, Debug)]
pub struct WatchDirectory {
    pub path: PathBuf,
    pub uri_prefix: String,
}
pub struct FileWatcher {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    pub changes: Receiver<FileBatch>,
}
impl FileWatcher {
    /// Files map canonical document URI to source path. Two identical reads debounce partial saves.
    pub fn new(files: BTreeMap<String, PathBuf>, interval: Duration) -> Self {
        Self::start(files, interval, None, Vec::new())
    }
    /// Start from the exact source snapshot loaded by the caller, avoiding a startup
    /// race when an editor saves before the worker takes its first sample.
    pub fn with_baseline(
        files: BTreeMap<String, PathBuf>,
        interval: Duration,
        baseline: BTreeMap<String, String>,
    ) -> Self {
        Self::start(files, interval, Some(baseline), Vec::new())
    }
    /// Discover XAML files recursively as well as watching the original manifest
    /// paths. New documents still require session admission; discovery never
    /// silently establishes a new compiled compatibility baseline.
    pub fn with_directories(
        files: BTreeMap<String, PathBuf>,
        directories: Vec<WatchDirectory>,
        interval: Duration,
        baseline: BTreeMap<String, String>,
    ) -> Self {
        Self::start(files, interval, Some(baseline), directories)
    }
    fn start(
        mut files: BTreeMap<String, PathBuf>,
        interval: Duration,
        baseline: Option<BTreeMap<String, String>>,
        directories: Vec<WatchDirectory>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let (send, changes) = mpsc::channel();
        let worker = thread::spawn(move || {
            let generation = Arc::new(AtomicU64::new(0));
            let mut delivered_generation = 0;
            let read =
                |path: &PathBuf| std::fs::read_to_string(path).map_err(|error| error.to_string());
            let mut accepted: BTreeMap<_, _> = match baseline {
                Some(baseline) => baseline
                    .into_iter()
                    .map(|(uri, text)| (uri, Ok(text)))
                    .collect(),
                None => files
                    .iter()
                    .map(|(uri, path)| (uri.clone(), read(path)))
                    .collect(),
            };
            let mut previous = accepted.clone();
            let mut accepted_diagnostics = Vec::new();
            let mut previous_diagnostics = Vec::new();
            while !worker_stop.load(Ordering::Acquire) {
                thread::park_timeout(interval.max(Duration::from_millis(10)));
                let mut diagnostics = Vec::new();
                for directory in &directories {
                    if let Err(error) = discover(directory, &mut files) {
                        diagnostics.push(format!("{}: {error}", directory.path.display()));
                    }
                }
                diagnostics.sort();
                let next: BTreeMap<_, _> = files
                    .iter()
                    .map(|(uri, path)| (uri.clone(), read(path)))
                    .collect();
                if next != previous || diagnostics != previous_diagnostics {
                    generation.fetch_add(1, Ordering::AcqRel);
                }
                let observed = generation.load(Ordering::Acquire);
                if next == previous
                    && diagnostics == previous_diagnostics
                    && (next != accepted
                        || diagnostics != accepted_diagnostics
                        || observed != delivered_generation)
                {
                    let batch = FileBatch {
                        files: next
                            .iter()
                            .map(|(uri, contents)| FileChange {
                                uri: uri.clone(),
                                contents: contents.clone(),
                            })
                            .collect(),
                        diagnostics: diagnostics.clone(),
                        revision: Some(WatchRevision {
                            current: generation.clone(),
                            observed,
                        }),
                    };
                    if send.send(batch).is_err() {
                        return;
                    }
                    accepted = next.clone();
                    accepted_diagnostics = diagnostics.clone();
                    delivered_generation = observed;
                }
                previous = next;
                previous_diagnostics = diagnostics;
            }
        });
        Self {
            stop,
            worker: Some(worker),
            changes,
        }
    }
}

fn discover(
    directory: &WatchDirectory,
    files: &mut BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    use ferroui_base::utilities::{Uri, UriKind};
    let base = Uri::try_create(&directory.uri_prefix, UriKind::Absolute)
        .filter(|_| directory.uri_prefix.ends_with('/'))
        .ok_or_else(|| "Directory URI must be absolute and end in '/'".to_owned())?;
    let mut pending = vec![directory.path.clone()];
    while let Some(path) = pending.pop() {
        if std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Symlink directories require explicit document paths".into());
        }
        for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err("Symlink entries require explicit document paths".into());
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("xaml"))
            {
                if !kind.is_file() {
                    return Err("Discovered XAML must be a regular file".into());
                }
                let path = entry.path();
                let relative = path
                    .strip_prefix(&directory.path)
                    .map_err(|e| e.to_string())?;
                let name = relative
                    .iter()
                    .map(|part| part.to_str().ok_or("Non-UTF-8 document path"))
                    .collect::<Result<Vec<_>, _>>()?
                    .join("/");
                // Escape URI delimiters in physical filenames, preserving directory separators.
                let escaped: String = name
                    .bytes()
                    .map(|byte| {
                        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                            (byte as char).to_string()
                        } else {
                            format!("%{byte:02X}")
                        }
                    })
                    .collect();
                let relative =
                    Uri::try_create(&escaped, UriKind::Relative).ok_or("Invalid document path")?;
                let uri = Uri::combine(&base, &relative).absolute_uri().to_string();
                if let Some(existing) = files.get(&uri) {
                    if existing != &path && existing.canonicalize().ok() != path.canonicalize().ok()
                    {
                        return Err(format!("Conflicting source paths for {uri}"));
                    }
                } else {
                    files.insert(uri, path);
                }
            }
        }
    }
    Ok(())
}
impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}
