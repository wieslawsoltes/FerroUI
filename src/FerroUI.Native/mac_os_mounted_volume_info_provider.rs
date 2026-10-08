use ferroui_base::collections::FerroList;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::platform::{IMountedVolumeInfoProvider, MountedVolumeInfo};
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

/// Keeps a collection of mounted volumes equal to the directories of
/// `/Volumes/`, which it reads once a second.
pub(crate) struct MacOSMountedVolumeInfoListener {
    // The timer is not stopped when the listener is disposed, as in the
    // reference: the field is held and never disposed.
    #[allow(dead_code)]
    disposable: Rc<dyn IDisposable>,
    been_disposed: Cell<bool>,
}

impl MacOSMountedVolumeInfoListener {
    pub(crate) fn new(mounted_drives: FerroList<MountedVolumeInfo>) -> Self {
        let polled = mounted_drives.clone();
        let disposable = DispatcherTimer::run(move || Self::poll(&polled), Duration::from_secs(1), DispatcherPriority::default());

        Self::poll(&mounted_drives);

        Self { disposable, been_disposed: Cell::new(false) }
    }

    fn poll(mounted_drives: &FerroList<MountedVolumeInfo>) -> bool {
        let mount_vol_infos: Vec<MountedVolumeInfo> = directories("/Volumes/")
            .into_iter()
            .map(|p| MountedVolumeInfo {
                volume_label: Path::new(&p).file_name().map(|name| name.to_string_lossy().into_owned()),
                volume_path: Some(p),
                volume_size_bytes: 0,
            })
            .collect();

        if mounted_drives.snapshot().iter().eq(mount_vol_infos.iter()) {
            true
        } else {
            mounted_drives.clear();

            for i in mount_vol_infos {
                mounted_drives.add(i);
            }
            true
        }
    }
}

/// The directories directly inside `path`, as full paths. A directory that
/// cannot be read has none.
fn directories(path: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

impl IDisposable for MacOSMountedVolumeInfoListener {
    fn dispose(&self) {
        if !self.been_disposed.get() {
            self.been_disposed.set(true);
        }
    }
}

/// Lists the mounted volumes of macOS.
#[derive(Default)]
pub(crate) struct MacOSMountedVolumeInfoProvider;

impl IMountedVolumeInfoProvider for MacOSMountedVolumeInfoProvider {
    fn listen(&self, mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable> {
        Rc::new(MacOSMountedVolumeInfoListener::new(mounted_drives))
    }
}
