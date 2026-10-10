//! Port of `WindowsMountedVolumeInfoListener.cs`: keeps a collection of
//! mounted volumes equal to the drives of the system that are ready, which
//! it reads once a second.
//!
//! The reference asks the class library of its runtime for the drives
//! (`DriveInfo`); here the same is asked of the system: the letters of
//! `GetLogicalDrives`, a drive is ready when its root directory exists,
//! its size is the one of `GetDiskFreeSpaceEx` and its label the one of
//! `GetVolumeInformation`.

/// The names of the drives of a mask of `GetLogicalDrives`, which are
/// their root directories: `C:\`.
pub(crate) fn drive_names(mask: u32) -> Vec<String> {
    (0..26u8).filter(|bit| mask & (1 << bit) != 0).map(|bit| format!("{}:\\", char::from(b'A' + bit))).collect()
}

/// The label of a volume as it is shown: the root directory of the drive
/// when the volume has no label, and the label with the name of the drive
/// otherwise.
pub(crate) fn volume_label(label: &str, name: &str, root_directory: &str) -> String {
    if label.trim().is_empty() {
        root_directory.to_string()
    } else {
        format!("{label} ({name})")
    }
}

#[cfg(windows)]
pub(crate) use imp::WindowsMountedVolumeInfoListener;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{get_disk_total_size, get_logical_drives, get_volume_label};
    use ferroui_base::collections::FerroList;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::platform::MountedVolumeInfo;
    use std::cell::Cell;
    use std::path::Path;
    use std::rc::Rc;
    use std::time::Duration;

    pub(crate) struct WindowsMountedVolumeInfoListener {
        disposable: Rc<dyn IDisposable>,
        been_disposed: Cell<bool>,
    }

    impl WindowsMountedVolumeInfoListener {
        pub(crate) fn new(mounted_drives: FerroList<MountedVolumeInfo>) -> Self {
            let polled = mounted_drives.clone();
            let disposable =
                DispatcherTimer::run(move || Self::poll(&polled), Duration::from_secs(1), DispatcherPriority::default());

            Self::poll(&mounted_drives);

            Self { disposable, been_disposed: Cell::new(false) }
        }

        /// The drives that are ready, as mounted volumes.
        pub(crate) fn mounted_volumes() -> Vec<MountedVolumeInfo> {
            drive_names(get_logical_drives())
                .into_iter()
                .filter_map(|name| {
                    if !Path::new(&name).is_dir() {
                        return None;
                    }
                    // try to read size as a proof of read access.
                    let Some(total_size) = get_disk_total_size(&name) else {
                        if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                            log.log(None, &format!("Error in Windows drive enumeration: the size of {name} could not be read"));
                        }
                        return None;
                    };
                    let label = get_volume_label(&name).unwrap_or_default();
                    Some(MountedVolumeInfo {
                        volume_label: Some(volume_label(&label, &name, &name)),
                        volume_path: Some(name),
                        volume_size_bytes: total_size,
                    })
                })
                .collect()
        }

        fn poll(mounted_drives: &FerroList<MountedVolumeInfo>) -> bool {
            let mount_vol_infos = Self::mounted_volumes();

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

    impl IDisposable for WindowsMountedVolumeInfoListener {
        fn dispose(&self) {
            if !self.been_disposed.get() {
                self.disposable.dispose();
                self.been_disposed.set(true);
            }
        }
    }

    /// Against the system: the drive of the temporary directory is one of
    /// the mounted volumes, with a size and its root as its path.
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_drive_of_the_temporary_directory_is_a_mounted_volume() {
            let volumes = WindowsMountedVolumeInfoListener::mounted_volumes();
            println!("{volumes:?}");
            let temp = std::env::temp_dir();
            let temp = temp.to_string_lossy().to_uppercase();
            let volume = volumes
                .iter()
                .find(|volume| volume.volume_path.as_deref().is_some_and(|path| temp.starts_with(&path.to_uppercase())))
                .unwrap_or_else(|| panic!("no volume for {temp}: {volumes:?}"));
            assert!(volume.volume_size_bytes > 0);
            assert!(volume.volume_label.as_deref().is_some_and(|label| !label.is_empty()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drives_of_a_mask_are_their_root_directories() {
        assert_eq!(drive_names(0), Vec::<String>::new());
        assert_eq!(drive_names(0b100), vec!["C:\\".to_string()]);
        assert_eq!(drive_names(0b1101), vec!["A:\\".to_string(), "C:\\".to_string(), "D:\\".to_string()]);
        assert_eq!(drive_names(1 << 25), vec!["Z:\\".to_string()]);
        // The bits above the letters name nothing.
        assert_eq!(drive_names(1 << 26), Vec::<String>::new());
    }

    #[test]
    fn a_volume_without_a_label_is_named_after_its_root() {
        assert_eq!(volume_label("", "C:\\", "C:\\"), "C:\\");
        assert_eq!(volume_label("  ", "C:\\", "C:\\"), "C:\\");
        assert_eq!(volume_label("Windows", "C:\\", "C:\\"), "Windows (C:\\)");
    }
}
