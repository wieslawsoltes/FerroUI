//! What the windows of the DirectComposition mode share: the device and
//! the lock of its batch.

use super::IDCompositionDesktopDevice;
use crate::sync_root::{SharedCom, SyncRoot};
use ferroui_microcom::ComPtr;
use std::sync::Arc;

pub(crate) struct DirectCompositionShared {
    sync_root: Arc<SyncRoot>,
    device: SharedCom<IDCompositionDesktopDevice>,
}

impl DirectCompositionShared {
    /// Holds a reference of its own to the device.
    pub fn new(device: &ComPtr<IDCompositionDesktopDevice>) -> Arc<DirectCompositionShared> {
        Arc::new(DirectCompositionShared {
            sync_root: SyncRoot::new(),
            // SAFETY: an object of DirectComposition, which is
            // free-threaded; the changes of a batch are serialized by the
            // lock beside it.
            device: unsafe { SharedCom::new(device.clone()) },
        })
    }

    /// The lock around the changes of a frame and their commit.
    pub fn sync_root(&self) -> &Arc<SyncRoot> {
        &self.sync_root
    }

    pub fn device(&self) -> &ComPtr<IDCompositionDesktopDevice> {
        &self.device
    }
}
