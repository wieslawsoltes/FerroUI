use super::MountedVolumeInfo;
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// Defines a platform-specific mount volumes info provider implementation.
///
/// This API is unstable: it may change with the platform backends.
pub trait IMountedVolumeInfoProvider {
    /// Listens to any changes in volume mounts and forwards updates to the
    /// referenced [`MountedVolumeInfo`] collection, until the returned
    /// handle is disposed.
    fn listen(&self, mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable>;
}
