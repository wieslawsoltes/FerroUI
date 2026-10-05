/// Describes a mounted volume.
///
/// This API is unstable: it may change with the platform backends.
#[derive(Clone, Debug, Default)]
pub struct MountedVolumeInfo {
    /// The label of the volume.
    pub volume_label: Option<String>,
    /// The path the volume is mounted at.
    pub volume_path: Option<String>,
    /// The size of the volume in bytes.
    pub volume_size_bytes: u64,
}

impl PartialEq for MountedVolumeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.volume_size_bytes == other.volume_size_bytes
            && self.volume_path == other.volume_path
            && self.volume_label.as_deref().unwrap_or_default() == other.volume_label.as_deref().unwrap_or_default()
    }
}
