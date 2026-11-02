//! Rebuild a mounted native theme when its binary resource operands change.
use crate::{catch_reload, styles::StyleSlot, FileBatch, FileWatcher, PreparedReload, ReloadError};
use ferroui_base::{
    styling::{IStyle, Styles},
    Ref,
};
use ferroui_markup_xaml::xaml_il::runtime::compiled::binary_resources::{
    fingerprint, reload::Snapshot, Kind, MAX_BLOCK_BYTES,
};
use std::{
    collections::BTreeMap, fs::File, io::Read, path::Path, rc::Rc, sync::Arc, time::Duration,
};

// An FXB identity always has the form module/filename. The sentinel sorts before
// every real leaf, so an obsolete call graph wins over dangling-file diagnostics.
const MANIFEST: &str = "";
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

/// Watches the specified FXB files with the normal two-read debounce. Pump on the UI
/// thread, where style owners and the constructed theme live. All files in a stable
/// revision must validate before the existing contribution is replaced.
pub struct BinaryThemeWatcher {
    files: FileWatcher<Arc<[u8]>>,
    baseline: Manifest,
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
        let mut paths: BTreeMap<_, _> = baseline
            .blocks
            .keys()
            .map(|name| (name.clone(), directory.join(name)))
            .collect();
        paths.insert(
            MANIFEST.into(),
            directory.join(
                path.file_name()
                    .ok_or_else(|| failure("Missing FXB manifest filename"))?,
            ),
        );
        let files = FileWatcher::poll_files(
            paths,
            interval,
            Some(BTreeMap::new()),
            Vec::new(),
            move |path| {
                let canonical = path
                    .canonicalize()
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                if canonical.parent() != Some(directory.as_path()) || !canonical.is_file() {
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
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                if bytes.len() > MAX_BLOCK_BYTES {
                    return Err(format!("{} exceeds FXB size limit", path.display()));
                }
                Ok(Arc::from(bytes))
            },
        );
        Ok(Self {
            files,
            baseline,
            slot,
        })
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
        batch: FileBatch<Arc<[u8]>>,
        build: impl FnOnce() -> Result<Rc<dyn IStyle>, ReloadError>,
    ) -> Result<bool, ReloadError> {
        let stale = || batch.revision.as_ref().is_some_and(|rev| !rev.is_current());
        if stale() {
            return Ok(false);
        }
        let mut files = batch.files.into_iter();
        let head = files
            .next()
            .filter(|f| f.uri == MANIFEST)
            .ok_or_else(|| failure("Missing FXB generation manifest"))?;
        let generation = Manifest::parse(&head.contents.map_err(ReloadError::Failed)?)?;
        if !self.baseline.compatible(&generation) {
            return Err(ReloadError::RestartRequired(
                "FXB caller or block table changed; rebuild and restart the native caller".into(),
            ));
        }
        let mut snapshot = Snapshot::default();
        for file in files {
            let data = file.contents.map_err(ReloadError::Failed)?;
            let (kind, hash) = generation.blocks[&file.uri];
            if fingerprint(&data) != hash {
                return Err(failure(format!(
                    "{} does not match the published FXB generation",
                    file.uri
                )));
            }
            snapshot
                .replace(&format!("{}/{}", generation.module, file.uri), kind, &data)
                .map_err(|e| failure(e.to_string()))?;
        }
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
        std::fs::write(&other, red).unwrap();
        pump(&mut watcher).unwrap();
        assert_eq!(color(&container), Color::from_uint32(0xff00ff00));
        let accepted = container.get(0);

        std::fs::write(&path, &green[..5]).unwrap();
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
