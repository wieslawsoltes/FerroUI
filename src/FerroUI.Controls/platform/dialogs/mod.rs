//! The contracts of the file dialogs of a platform.

mod i_mounted_volume_info_provider;
mod i_storage_provider_factory;
mod mounted_drive_info;

pub use i_mounted_volume_info_provider::IMountedVolumeInfoProvider;
pub use i_storage_provider_factory::IStorageProviderFactory;
pub use mounted_drive_info::MountedVolumeInfo;

#[cfg(test)]
mod tests {
    use super::*;

    // --- not from upstream ---

    #[test]
    fn mounted_volume_info_compares_by_value_and_treats_a_missing_label_as_empty() {
        let volume = |label: Option<&str>, path: Option<&str>, size| MountedVolumeInfo {
            volume_label: label.map(str::to_owned),
            volume_path: path.map(str::to_owned),
            volume_size_bytes: size,
        };

        assert_eq!(volume(Some("Data"), Some("/mnt/data"), 10), volume(Some("Data"), Some("/mnt/data"), 10));
        assert_eq!(volume(None, Some("/mnt/data"), 10), volume(Some(""), Some("/mnt/data"), 10));
        assert_ne!(volume(Some("Data"), Some("/mnt/data"), 10), volume(Some("Data"), Some("/mnt/data"), 11));
        assert_ne!(volume(Some("Data"), Some("/mnt/data"), 10), volume(Some("Data"), None, 10));
        assert_ne!(volume(Some("Data"), Some("/mnt/data"), 10), volume(Some("Other"), Some("/mnt/data"), 10));
        assert_eq!(MountedVolumeInfo::default(), volume(None, None, 0));
    }
}
