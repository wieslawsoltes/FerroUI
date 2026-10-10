//! The mounted volumes of a Linux system, polled once a second (the port
//! of `LinuxMountedVolumeInfoListener.cs`).

use crate::native_methods;
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::platform::MountedVolumeInfo;
use std::cell::{Cell, RefCell};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const DEV_BY_LABEL_DIR: &str = "/dev/disk/by-label/";
const PROC_PARTITIONS_DIR: &str = "/proc/partitions";
const PROC_MOUNTS_DIR: &str = "/proc/mounts";

pub struct LinuxMountedVolumeInfoListener {
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    target_obs: FerroList<MountedVolumeInfo>,
    been_disposed: Cell<bool>,
}

/// Replaces every escape of `prefix` followed by `digits` digits of
/// `radix` by the character with that code (`UnescapeString`). An escape
/// whose value does not fit a byte is left as it is.
fn unescape_string(input: &str, prefix: &str, digits: usize, radix: u32, is_digit: fn(char) -> bool) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find(prefix) {
        let after = &rest[index + prefix.len()..];
        let code: String = after.chars().take(digits).collect();
        let value = (code.chars().count() == digits && code.chars().all(is_digit))
            .then(|| u8::from_str_radix(&code, radix).ok())
            .flatten();
        match value {
            Some(value) => {
                output.push_str(&rest[..index]);
                output.push(char::from(value));
                rest = &after[code.len()..];
            }
            None => {
                output.push_str(&rest[..index + prefix.len()]);
                rest = after;
            }
        }
    }
    output.push_str(rest);
    output
}

/// A path as `/proc/mounts` writes it: a space, a tab, a newline and a
/// backslash are a backslash and three octal digits.
pub(crate) fn unescape_path_from_proc_mounts(input: &str) -> String {
    unescape_string(input, "\\", 3, 8, |c| c.is_ascii_digit())
}

/// A label as the names in `/dev/disk/by-label` write it: other
/// characters than the plain ones are `\x` and two hexadecimal digits.
pub(crate) fn unescape_device_label(input: &str) -> String {
    unescape_string(input, "\\x", 2, 16, |c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// The full path of the target of a link of the label directory
/// (`GetSymlinkTarget`): the target relative to the directory, without
/// `.` and `..`.
pub(crate) fn full_path(directory: &str, target: &str) -> String {
    let mut path = PathBuf::new();
    for component in Path::new(directory).join(target).components() {
        match component {
            Component::ParentDir => {
                path.pop();
            }
            Component::CurDir => {}
            other => path.push(other.as_os_str()),
        }
    }
    path.to_string_lossy().into_owned()
}

/// The sizes in bytes and the device paths of the lines of
/// `/proc/partitions` (major, minor, blocks of 1024 bytes, name), without
/// its header.
pub(crate) fn parse_partitions(text: &str) -> Vec<(u64, String)> {
    text.lines()
        .skip(1)
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            let p: Vec<&str> = p.split_whitespace().collect();
            let blocks: u64 = p.get(2)?.parse().ok()?;
            Some((blocks * 1024, format!("/dev/{}", p.get(3)?)))
        })
        .collect()
}

/// The devices and the mount points of the lines of `/proc/mounts`,
/// without the mounts of snap packages.
pub(crate) fn parse_mounts(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|x| {
            let mut x = x.split(' ');
            Some((x.next()?.to_string(), unescape_path_from_proc_mounts(x.next()?)))
        })
        .filter(|x| !x.1.to_ascii_lowercase().starts_with("/snap/"))
        .collect()
}

/// The volumes: every mount of a device that is a partition, with the
/// size of the partition and its label when it has one (a row per label
/// when it has several).
pub(crate) fn mounted_volumes(
    proc_partitions: &[(u64, String)],
    proc_mounts: &[(String, String)],
    label_dev_path_pairs: &[(String, String)],
) -> Vec<MountedVolumeInfo> {
    let mut volumes = Vec::new();
    for mount in proc_mounts {
        for device in proc_partitions.iter().filter(|device| device.1 == mount.0) {
            let mut labels = label_dev_path_pairs.iter().filter(|label| label.0 == device.1).peekable();
            if labels.peek().is_none() {
                volumes.push(MountedVolumeInfo {
                    volume_path: Some(mount.1.clone()),
                    volume_size_bytes: device.0,
                    volume_label: None,
                });
            }
            for label in labels {
                volumes.push(MountedVolumeInfo {
                    volume_path: Some(mount.1.clone()),
                    volume_size_bytes: device.0,
                    volume_label: Some(label.1.clone()),
                });
            }
        }
    }
    volumes
}

/// The links of the label directory: the device each names, and its label.
fn read_labels() -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(DEV_BY_LABEL_DIR) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let target = native_methods::read_link(&entry.path().to_string_lossy())?;
            Some((full_path(DEV_BY_LABEL_DIR, &target), unescape_device_label(&entry.file_name().to_string_lossy())))
        })
        .collect()
}

impl LinuxMountedVolumeInfoListener {
    /// Fills `target` now and keeps it current until the listener is
    /// disposed.
    ///
    /// # Panics
    /// Panics when called from a thread other than the UI thread.
    pub fn new(target: FerroList<MountedVolumeInfo>) -> Rc<Self> {
        let this = Rc::new(Self { disposable: RefCell::new(None), target_obs: target, been_disposed: Cell::new(false) });

        let weak = Rc::downgrade(&this);
        let timer = DispatcherTimer::run(
            move || weak.upgrade().is_some_and(|this| this.poll()),
            Duration::from_secs(1),
            DispatcherPriority::DEFAULT,
        );
        *this.disposable.borrow_mut() = Some(timer);
        this.poll();
        this
    }

    fn poll(&self) -> bool {
        // A file that cannot be read is an empty one: the reference fails
        // in the timer on a system without them, which a Linux system is
        // not.
        let f_proc_partitions = parse_partitions(&std::fs::read_to_string(PROC_PARTITIONS_DIR).unwrap_or_default());
        let f_proc_mounts = parse_mounts(&std::fs::read_to_string(PROC_MOUNTS_DIR).unwrap_or_default());
        let label_dev_path_pairs = read_labels();

        let mount_vol_infos = mounted_volumes(&f_proc_partitions, &f_proc_mounts, &label_dev_path_pairs);

        if self.target_obs.to_vec() == mount_vol_infos {
            true
        } else {
            self.target_obs.clear();

            for i in mount_vol_infos {
                self.target_obs.add(i);
            }
            true
        }
    }
}

impl IDisposable for LinuxMountedVolumeInfoListener {
    fn dispose(&self) {
        if !self.been_disposed.replace(true) {
            if let Some(disposable) = self.disposable.borrow_mut().take() {
                disposable.dispose();
            }
            self.target_obs.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;

    const PARTITIONS: &str = "major minor  #blocks  name\n\n 259        0  500107608 nvme0n1\n 259        1     523264 nvme0n1p1\n 259        2  499582976 nvme0n1p2\n   8        0   15633408 sda\n   8        1   15632384 sda1\n";
    const MOUNTS: &str = "sysfs /sys sysfs rw,nosuid 0 0\n/dev/nvme0n1p2 / ext4 rw,relatime 0 0\n/dev/loop3 /snap/core22/1122 squashfs ro 0 0\n/dev/nvme0n1p1 /boot/efi vfat rw 0 0\n/dev/sda1 /media/user/My\\040Stick vfat rw 0 0\n/dev/sda1 /mnt/again vfat rw 0 0\n";

    fn volume(path: &str, size: u64, label: Option<&str>) -> MountedVolumeInfo {
        MountedVolumeInfo {
            volume_path: Some(path.to_string()),
            volume_size_bytes: size,
            volume_label: label.map(str::to_string),
        }
    }

    #[test]
    fn escapes_of_mount_points_and_labels() {
        assert_eq!(unescape_path_from_proc_mounts("/media/My\\040Stick"), "/media/My Stick");
        assert_eq!(unescape_path_from_proc_mounts("/a\\011b\\134c"), "/a\tb\\c");
        assert_eq!(unescape_path_from_proc_mounts("/plain"), "/plain");
        // Not three digits, and a value that is no octal number: left alone.
        assert_eq!(unescape_path_from_proc_mounts("/a\\04"), "/a\\04");
        assert_eq!(unescape_path_from_proc_mounts("/a\\089b"), "/a\\089b");

        assert_eq!(unescape_device_label("My\\x20Stick"), "My Stick");
        assert_eq!(unescape_device_label("a\\x2fb"), "a/b");
        // Upper case digits are not an escape of that directory.
        assert_eq!(unescape_device_label("a\\x2Fb"), "a\\x2Fb");
        assert_eq!(unescape_device_label("EFI"), "EFI");
    }

    #[test]
    fn the_target_of_a_label_link_is_a_device_path() {
        assert_eq!(full_path(DEV_BY_LABEL_DIR, "../../sda1"), "/dev/sda1");
        assert_eq!(full_path(DEV_BY_LABEL_DIR, "../../mapper/./root"), "/dev/mapper/root");
        assert_eq!(full_path(DEV_BY_LABEL_DIR, "/dev/nvme0n1p1"), "/dev/nvme0n1p1");
    }

    #[test]
    fn partitions_have_a_size_in_bytes_and_a_device_path() {
        let partitions = parse_partitions(PARTITIONS);
        assert_eq!(partitions.len(), 5);
        assert_eq!(partitions[1], (523264 * 1024, "/dev/nvme0n1p1".to_string()));
        assert_eq!(partitions[4], (15632384 * 1024, "/dev/sda1".to_string()));
        assert!(parse_partitions("").is_empty());
        assert!(parse_partitions("major minor  #blocks  name\n").is_empty());
    }

    #[test]
    fn mounts_have_a_device_and_a_path_and_snap_mounts_are_left_out() {
        let mounts = parse_mounts(MOUNTS);
        assert_eq!(
            mounts,
            [
                ("sysfs".to_string(), "/sys".to_string()),
                ("/dev/nvme0n1p2".to_string(), "/".to_string()),
                ("/dev/nvme0n1p1".to_string(), "/boot/efi".to_string()),
                ("/dev/sda1".to_string(), "/media/user/My Stick".to_string()),
                ("/dev/sda1".to_string(), "/mnt/again".to_string()),
            ]
        );
    }

    #[test]
    fn a_volume_is_a_mounted_partition_with_its_label() {
        let labels =
            vec![("/dev/sda1".to_string(), "My Stick".to_string()), ("/dev/sdb1".to_string(), "Elsewhere".to_string())];
        let volumes = mounted_volumes(&parse_partitions(PARTITIONS), &parse_mounts(MOUNTS), &labels);
        assert_eq!(
            volumes,
            [
                // In the order of the mounts; what is not a partition (sysfs) is no volume.
                volume("/", 499582976 * 1024, None),
                volume("/boot/efi", 523264 * 1024, None),
                volume("/media/user/My Stick", 15632384 * 1024, Some("My Stick")),
                volume("/mnt/again", 15632384 * 1024, Some("My Stick")),
            ]
        );

        // A partition with two labels is listed once per label.
        let labels = vec![("/dev/sda1".to_string(), "A".to_string()), ("/dev/sda1".to_string(), "B".to_string())];
        let volumes = mounted_volumes(&parse_partitions(PARTITIONS), &[("/dev/sda1".to_string(), "/m".to_string())], &labels);
        assert_eq!(volumes, [volume("/m", 15632384 * 1024, Some("A")), volume("/m", 15632384 * 1024, Some("B"))]);
        assert!(mounted_volumes(&[], &parse_mounts(MOUNTS), &[]).is_empty());
    }

    #[test]
    fn a_listener_fills_its_list_and_empties_it_when_disposed() {
        let _scope = crate::test_support::scope();
        let list = FerroList::new();
        list.add(volume("/stale", 1, None));
        let listener = LinuxMountedVolumeInfoListener::new(list.clone());
        // What the system has now (nothing, where the files of a Linux system are not there).
        assert!(list.to_vec().iter().all(|volume| volume.volume_path.as_deref() != Some("/stale")));
        listener.dispose();
        assert!(list.is_empty());
        listener.dispose();
    }
}
