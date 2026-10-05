use ferroui_base::collections::FerroList;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_controls::platform::{IMountedVolumeInfoProvider, MountedVolumeInfo};
use std::rc::Rc;

/// A volume of the file system, as the drive list of the .NET base library
/// describes it.
struct DriveInfo {
    /// The path the volume is mounted at (`RootDirectory.FullName`).
    root_directory: String,
    /// The label of the volume: the name of the volume where volumes have
    /// one (Windows), the mount path elsewhere.
    volume_label: String,
    total_size: u64,
}

impl DriveInfo {
    /// The mounted volumes (`DriveInfo.GetDrives()`).
    ///
    /// The list comes from the disk list of `sysinfo`: on Linux it reads
    /// the mount table as the .NET base library does, but leaves out the
    /// mounts of virtual file systems (`proc`, `sysfs`, ...), which the
    /// managed list contains.
    #[cfg(not(target_arch = "wasm32"))]
    fn get_drives() -> Vec<DriveInfo> {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        disks
            .list()
            .iter()
            .map(|disk| {
                let root_directory = disk.mount_point().to_string_lossy().into_owned();
                let volume_label = if cfg!(windows) {
                    disk.name().to_string_lossy().into_owned()
                } else {
                    // The label of a drive outside Windows is its name: the mount path.
                    root_directory.clone()
                };
                DriveInfo { root_directory, volume_label, total_size: disk.total_space() }
            })
            .collect()
    }

    /// The mounted volumes: none in the browser, which has no file system
    /// of the operating system.
    #[cfg(target_arch = "wasm32")]
    fn get_drives() -> Vec<DriveInfo> {
        Vec::new()
    }

    /// Whether the volume can be read (`IsReady`).
    fn is_ready(&self) -> bool {
        std::path::Path::new(&self.root_directory).is_dir()
    }
}

/// The mounted volume info provider of the file system, used when the
/// platform registers none.
///
/// Internal upstream; public here for the platform backends.
pub struct BclMountedVolumeInfoProvider;

impl IMountedVolumeInfoProvider for BclMountedVolumeInfoProvider {
    fn listen(&self, mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable> {
        for drive in DriveInfo::get_drives() {
            if !drive.is_ready() {
                continue;
            }
            let total_size = drive.total_size;
            let directory = drive.root_directory.clone();

            // Volumes that cannot be listed are left out.
            if std::fs::read_dir(&directory).is_err() {
                continue;
            }

            mounted_drives.add(MountedVolumeInfo {
                volume_label: Some(if drive.volume_label.trim().is_empty() { directory.clone() } else { drive.volume_label }),
                volume_path: Some(directory),
                volume_size_bytes: total_size,
            });
        }
        Disposable::empty()
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;

    #[test]
    fn adds_readable_volumes_with_a_label_and_a_path() {
        let volumes = FerroList::new();
        BclMountedVolumeInfoProvider.listen(volumes.clone()).dispose();

        for volume in volumes.to_vec() {
            let path = volume.volume_path.expect("a path");
            assert!(std::path::Path::new(&path).is_dir(), "{path}");
            assert!(!volume.volume_label.expect("a label").trim().is_empty());
        }
    }
}
