//! Port of `WindowsMountedVolumeInfoProvider.cs`.

use crate::windows_mounted_volume_info_listener::WindowsMountedVolumeInfoListener;
use ferroui_base::collections::FerroList;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::platform::{IMountedVolumeInfoProvider, MountedVolumeInfo};
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct WindowsMountedVolumeInfoProvider;

impl IMountedVolumeInfoProvider for WindowsMountedVolumeInfoProvider {
    fn listen(&self, mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable> {
        Rc::new(WindowsMountedVolumeInfoListener::new(mounted_drives))
    }
}
