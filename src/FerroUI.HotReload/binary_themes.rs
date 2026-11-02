//! Rebuild a mounted native theme when its binary resource operands change.
use crate::{catch_reload, styles::StyleSlot, FileBatch, FileWatcher, PreparedReload, ReloadError};
use ferroui_base::{
    styling::{IStyle, Styles},
    Ref,
};
use ferroui_markup_xaml::xaml_il::runtime::compiled::binary_resources::{
    fingerprint, reload::Snapshot, Batch, Kind, MAX_BLOCK_BYTES,
};
use std::{
    collections::BTreeMap, fs::File, io::Read, path::Path, rc::Rc, sync::Arc, time::Duration,
};

fn failure(message: impl ToString) -> ReloadError {
    ReloadError::Failed(message.to_string())
}

struct Manifest {
    module: String,
    caller: u64,
    blocks: BTreeMap<String, (Kind, u64)>,
}

impl Manifest {
    fn parse(bytes: &[u8]) -> Result<Self, ReloadError> {
        let digest = |v: &serde_json::Value| {
            v.as_str()
                .filter(|s| s.len() == 16)
                .and_then(|s| u64::from_str_radix(s, 16).ok())
                .ok_or_else(|| failure("Invalid FXB fingerprint"))
        };
        let manifest: serde_json::Value = serde_json::from_slice(bytes).map_err(failure)?;
        let module = manifest["module"]
            .as_str()
            .filter(|s| !s.is_empty() && !s.contains('/'))
            .ok_or_else(|| failure("Missing or invalid FXB module name"))?
            .to_owned();
        if manifest["version"].as_u64() != Some(1) {
            return Err(failure("Unsupported FXB manifest version"));
        }
        let caller = digest(&manifest["caller"])?;
        let leaves = manifest["blocks"]
            .as_object()
            .ok_or_else(|| failure("Missing FXB block table"))?;
        let mut blocks = BTreeMap::new();
        for (name, fields) in leaves {
            if !name.ends_with(".fxb")
                || name.contains(['/', '\\', ':', '\0'])
                || name.starts_with('.')
                || Path::new(name).file_name() != Some(name.as_ref())
            {
                return Err(failure("FXB manifest paths must be plain .fxb filenames"));
            }
            let fields = fields
                .as_array()
                .filter(|f| f.len() == 2)
                .ok_or_else(|| failure(format!("Invalid FXB entry for {name}")))?;
            let kind = fields[0]
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .and_then(Kind::from_wire)
                .ok_or_else(|| failure(format!("Unknown FXB factory kind for {name}")))?;
            blocks.insert(name.clone(), (kind, digest(&fields[1])?));
        }
        if blocks.is_empty() {
            return Err(failure("The compiled module has no binary resource blocks"));
        }
        Ok(Self {
            module,
            caller,
            blocks,
        })
    }

    fn compatible(&self, other: &Self) -> bool {
        self.module == other.module
            && self.caller == other.caller
            && self
                .blocks
                .iter()
                .map(|(name, (kind, _))| (name, kind))
                .eq(other.blocks.iter().map(|(name, (kind, _))| (name, kind)))
    }
}

struct Generation {
    manifest: Vec<u8>,
    blocks: BTreeMap<String, (Kind, u64)>,
    snapshot: Result<Snapshot, ReloadError>,
}

// Verified content fingerprints bind the complete decoded snapshot to these bytes.
impl PartialEq for Generation {
    fn eq(&self, other: &Self) -> bool {
        self.manifest == other.manifest
    }
}
impl Eq for Generation {}

struct GenerationReader {
    baseline: Manifest,
    previous: Option<Arc<Generation>>,
}

impl GenerationReader {
    fn read(
        &mut self,
        manifest: Vec<u8>,
        mut leaf: impl FnMut(&str) -> Result<Vec<u8>, String>,
    ) -> Result<Arc<Generation>, String> {
        if let Some(previous) = &self.previous {
            if previous.manifest == manifest {
                return Ok(previous.clone());
            }
        }
        let generation = Manifest::parse(&manifest).map_err(|e| format!("{e:?}"))?;
        let snapshot = if self.baseline.compatible(&generation) {
            let previous = self
                .previous
                .as_deref()
                .and_then(|p| p.snapshot.as_ref().ok().map(|s| (s, &p.blocks)));
            let mut snapshot = previous.map(|(s, _)| s.clone()).unwrap_or_default();
            for (name, &(kind, hash)) in &generation.blocks {
                if previous.is_some_and(|(_, blocks)| blocks.get(name) == Some(&(kind, hash))) {
                    continue;
                }
                let bytes = leaf(name)?;
                if fingerprint(&bytes) != hash {
                    return Err(format!(
                        "{name} does not match the published FXB generation"
                    ));
                }
                let batch = Batch::decode(&bytes).map_err(|e| format!("{name}: {e}"))?;
                snapshot
                    .replace_decoded(&format!("{}/{}", generation.module, name), kind, batch)
                    .map_err(|e| e.to_string())?;
            }
            Ok(snapshot)
        } else {
            Err(ReloadError::RestartRequired(
                "FXB caller or block table changed; rebuild and restart the native caller".into(),
            ))
        };
        let candidate = Arc::new(Generation {
            manifest,
            blocks: generation.blocks,
            snapshot,
        });
        self.previous = Some(candidate.clone());
        Ok(candidate)
    }
}

fn read_in_directory(directory: &Path, path: &Path) -> Result<Vec<u8>, String> {
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if canonical.parent() != Some(directory) || !canonical.is_file() {
        return Err(format!(
            "FXB file escapes manifest directory or is not regular: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    File::open(&canonical)
        .and_then(|file| {
            file.take(MAX_BLOCK_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > MAX_BLOCK_BYTES {
        return Err(format!("{} exceeds FXB size limit", path.display()));
    }
    Ok(bytes)
}

/// Polls only the manifest when idle. A new generation is read, checked and decoded
/// on the watcher thread; the UI thread reconstructs and replaces the owned style.
pub struct BinaryThemeWatcher {
    files: FileWatcher<Arc<Generation>>,
    slot: StyleSlot,
}

impl BinaryThemeWatcher {
    /// Watch the exact files named by a compiled module's debug-only `FXB_MANIFEST`
    /// constant. To watch a deployed copy, supply its manifest path and the original
    /// embedded text. A changed native caller or file table requires a rebuild/restart.
    pub fn from_manifest(
        container: &Ref<Styles>,
        index: usize,
        (path, embedded): (&str, &str),
        interval: Duration,
    ) -> Result<Self, ReloadError> {
        let slot = StyleSlot::new(container, index)?;
        let path = Path::new(path);
        let directory = path
            .parent()
            .ok_or_else(|| failure("Missing FXB manifest directory"))?
            .canonicalize()
            .map_err(failure)?;
        let baseline = Manifest::parse(embedded.as_bytes())?;
        let manifest = directory.join(
            path.file_name()
                .ok_or_else(|| failure("Missing FXB manifest filename"))?,
        );
        let mut reader = GenerationReader {
            baseline,
            previous: None,
        };
        let files = FileWatcher::poll_files(
            BTreeMap::from([(String::new(), manifest)]),
            interval,
            Some(BTreeMap::new()),
            Vec::new(),
            move |path| {
                let image = read_in_directory(&directory, path)?;
                reader.read(image, |name| {
                    read_in_directory(&directory, &directory.join(name))
                })
            },
        );
        Ok(Self { files, slot })
    }

    /// Apply the newest stable revision. The callback constructs a fresh native theme.
    /// False means no revision was ready or the pending revision was superseded.
    pub fn poll(
        &mut self,
        build: impl FnOnce() -> Result<Rc<dyn IStyle>, ReloadError>,
    ) -> Result<bool, ReloadError> {
        let Some(batch) = self.files.changes.try_iter().last() else {
            return Ok(false);
        };
        self.apply(batch, build)
    }

    fn apply(
        &self,
        batch: FileBatch<Arc<Generation>>,
        build: impl FnOnce() -> Result<Rc<dyn IStyle>, ReloadError>,
    ) -> Result<bool, ReloadError> {
        let stale = || batch.revision.as_ref().is_some_and(|rev| !rev.is_current());
        if stale() {
            return Ok(false);
        }
        let candidate = batch
            .files
            .into_iter()
            .next()
            .ok_or_else(|| failure("Missing FXB generation"))?
            .contents
            .map_err(ReloadError::Failed)?;
        let snapshot = candidate.snapshot.clone()?;
        let mut change = catch_reload(|| self.slot.prepare(|| Rc::new(snapshot).run(build)))?;
        if stale() {
            return Ok(false);
        }
        if let Err(err) = catch_reload(|| change.commit()) {
            catch_reload(|| {
                change.rollback();
                Ok(())
            })?;
            return Err(err);
        }
        if stale() {
            catch_reload(|| {
                change.rollback();
                Ok(())
            })?;
            return Ok(false);
        }
        change.finish();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{
        media::{Color, SolidColorBrush},
        styling::style_ptr_eq,
        threading::Dispatcher,
    };
    use ferroui_markup_xaml::xaml_il::runtime::compiled::{
        self as rt,
        binary_resources::{literal_brushes, Block},
    };

    const ID: &str = "fixture::compiled_xaml/brush.fxb";
    const BLUE: &[u8] = b"FXB\0\x01\x02\x01\x01\x00\x01A\x00\xff\x00\x00\xff\x01\x01\x00\x00";
    const SECOND: &[u8] = b"FXB\0\x01\x02\x01\x01\x00\x01B\x00\xff\x00\x00\xff\x01\x01\x00\x00";
    static DOC: rt::DocumentInfo = rt::DocumentInfo {
        base_uri: None,
        namespaces: &[],
    };

    fn build() -> Result<Rc<dyn IStyle>, ReloadError> {
        let theme = Styles::new();
        let ctx = rt::populate_context(None, &DOC, rt::to_value(theme.clone()));
        literal_brushes(&theme.resources(), &ctx, &DOC, Block::new(ID, BLUE))
            .map_err(|error| ReloadError::Failed(error.to_string()))?;
        literal_brushes(
            &theme.resources(),
            &ctx,
            &DOC,
            Block::new("fixture::compiled_xaml/other.fxb", SECOND),
        )
        .map_err(|error| ReloadError::Failed(error.to_string()))?;
        Ok(theme.into())
    }

    fn color(container: &Styles) -> Color {
        let brush = container
            .try_get_resource(&"A".into(), None)
            .unwrap()
            .unwrap();
        brush
            .downcast_ref::<Ref<SolidColorBrush>>()
            .unwrap()
            .color()
    }

    fn pump(watcher: &mut BinaryThemeWatcher) -> Result<(), ReloadError> {
        let timeout = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < timeout {
            if watcher.poll(build)? {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the file watcher did not deliver a stable revision");
    }

    #[test]
    fn idle_polls_read_no_leaves_and_a_one_block_edit_reads_only_that_block() {
        let blocks: BTreeMap<_, _> = (0..456)
            .map(|i| {
                (
                    format!("block-{i}.fxb"),
                    (2, format!("{:016x}", fingerprint(BLUE))),
                )
            })
            .collect();
        let manifest = serde_json::json!({"version": 1, "module":"fixture", "caller":"1234567890abcdef", "blocks":blocks}).to_string().into_bytes();
        let mut reader = GenerationReader {
            baseline: Manifest::parse(&manifest).unwrap(),
            previous: None,
        };
        let mut reads = 0;
        for _ in 0..10 {
            reader
                .read(manifest.clone(), |_| {
                    reads += 1;
                    Ok(BLUE.to_vec())
                })
                .unwrap();
        }
        assert_eq!(reads, 456);
        let mut edited = BLUE.to_vec();
        edited[12..16].copy_from_slice(&0xff00ff00u32.to_le_bytes());
        let mut next: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
        next["blocks"]["block-17.fxb"][1] = format!("{:016x}", fingerprint(&edited)).into();
        reader
            .read(next.to_string().into_bytes(), |name| {
                assert_eq!(name, "block-17.fxb");
                reads += 1;
                Ok(edited.clone())
            })
            .unwrap();
        assert_eq!(reads, 457);
    }

    #[test]
    fn file_edit_replaces_the_mounted_contribution_and_rejects_an_invalid_save() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_markup_xaml::register_types();
        let container = Styles::new();
        let sibling: Rc<dyn IStyle> = Styles::new().into();
        container.add(build().unwrap());
        container.add(sibling.clone());
        assert_eq!(color(&container), Color::from_uint32(0xff0000ff));

        let folder = std::env::temp_dir().join(format!("ferroui-fxb-watch-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("brush.fxb");
        let manifest = folder.join("fixture.fxb.json");
        let other = folder.join("other.fxb");
        let image = |a: &[u8], b: &[u8]| {
            serde_json::json!({
            "version":1, "module":"fixture::compiled_xaml", "caller":"0123456789abcdef",
            "blocks":{"brush.fxb":[2, format!("{:016x}", fingerprint(a))], "other.fxb":[2, format!("{:016x}", fingerprint(b))]}
        }).to_string()
        };
        let embedded = image(BLUE, SECOND);
        assert!(Manifest::parse(embedded.replace("brush.fxb", "../brush.fxb").as_bytes()).is_err());
        let mut green = BLUE.to_vec();
        green[12..16].copy_from_slice(&0xff00ff00u32.to_le_bytes());
        let mut red = SECOND.to_vec();
        red[12..16].copy_from_slice(&0xffff0000u32.to_le_bytes());
        std::fs::write(&path, &green).unwrap();
        std::fs::write(&other, SECOND).unwrap();
        std::fs::write(&manifest, image(&green, &red)).unwrap();
        let mut watcher = BinaryThemeWatcher::from_manifest(
            &container,
            0,
            (manifest.to_str().unwrap(), &embedded),
            Duration::from_millis(10),
        )
        .unwrap();
        let original = container.get(0);
        // Two individually valid files from different generations must never reach the owner.
        assert!(matches!(pump(&mut watcher), Err(ReloadError::Failed(_))));
        assert!(style_ptr_eq(&container.get(0), &original));
        std::fs::write(&other, &red).unwrap();
        pump(&mut watcher).unwrap();
        assert_eq!(color(&container), Color::from_uint32(0xff00ff00));
        let accepted = container.get(0);

        std::fs::write(&path, &green[..5]).unwrap();
        // Publishing only an incomplete leaf is staging, not a new generation.
        // Commit its fingerprint here to exercise rejection of the corrupt generation.
        std::fs::write(&manifest, image(&green[..5], &red)).unwrap();
        assert!(matches!(pump(&mut watcher), Err(ReloadError::Failed(_))));
        assert!(style_ptr_eq(&container.get(0), &accepted));

        std::fs::write(&path, BLUE).unwrap();
        std::fs::write(&other, SECOND).unwrap();
        std::fs::write(&manifest, &embedded).unwrap();
        pump(&mut watcher).unwrap();
        assert_eq!(color(&container), Color::from_uint32(0xff0000ff));
        assert!(style_ptr_eq(&container.get(1), &sibling));
        let last = container.get(0);
        std::fs::write(&manifest, embedded.replace("brush.fxb", "new.fxb")).unwrap();
        assert!(matches!(
            pump(&mut watcher),
            Err(ReloadError::RestartRequired(_))
        ));
        assert!(style_ptr_eq(&container.get(0), &last));
        drop(watcher);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(other).unwrap();
        std::fs::remove_file(manifest).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
