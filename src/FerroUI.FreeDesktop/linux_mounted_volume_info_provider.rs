//! The provider of the mounted volumes for the managed file dialogs (the
//! port of `LinuxMountedVolumeInfoProvider.cs`).

use crate::linux_mounted_volume_info_listener::LinuxMountedVolumeInfoListener;
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::platform::{IMountedVolumeInfoProvider, MountedVolumeInfo};
use std::rc::Rc;

#[derive(Default)]
pub struct LinuxMountedVolumeInfoProvider;

impl LinuxMountedVolumeInfoProvider {
    pub fn new() -> Self {
        Self
    }
}

impl IMountedVolumeInfoProvider for LinuxMountedVolumeInfoProvider {
    fn listen(&self, mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable> {
        LinuxMountedVolumeInfoListener::new(mounted_drives)
    }
}
