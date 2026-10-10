//! The Metal platform graphics: the default device of the system.

use super::MetalDevice;
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
use std::rc::Rc;
use std::sync::Arc;

/// The Metal platform graphics of the iOS backend.
///
/// The graphics are shared with the thread that renders
/// (`IPlatformGraphics: Send + Sync`): a Metal device may be used from any
/// thread, which the bindings state (`MTLDevice: Send + Sync`).
pub struct MetalPlatformGraphics {
    default_device: Retained<ProtocolObject<dyn MTLDevice>>,
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MetalPlatformGraphics>();
};

impl MetalPlatformGraphics {
    /// The graphics over the default device of the system; none where the
    /// system has no Metal device.
    pub fn try_create() -> Option<Arc<MetalPlatformGraphics>> {
        // Can be null on unsupported OS versions.
        let device = MTLCreateSystemDefaultDevice()?;
        Some(Arc::new(MetalPlatformGraphics { default_device: device }))
    }
}

impl IPlatformGraphics for MetalPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        MetalDevice::new(self.default_device.clone())
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.");
    }
}
