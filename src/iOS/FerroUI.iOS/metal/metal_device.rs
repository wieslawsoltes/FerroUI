//! A Metal device and its command queue.

use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::DisposableLock;
use ferroui_metal::IMetalDevice;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLCommandQueue, MTLDevice};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::{Rc, Weak};

/// A Metal device and the command queue frames are presented with.
pub struct MetalDevice {
    weak_self: Weak<MetalDevice>,
    sync_root: DisposableLock,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: RefCell<Option<Retained<ProtocolObject<dyn MTLCommandQueue>>>>,
}

impl MetalDevice {
    /// Creates the device with a command queue of its own.
    ///
    /// # Panics
    /// Panics when the device gives no command queue.
    pub(crate) fn new(device: Retained<ProtocolObject<dyn MTLDevice>>) -> Rc<MetalDevice> {
        let Some(queue) = device.newCommandQueue() else {
            panic!("IMTLCommandQueue is not available");
        };
        Rc::new_cyclic(|weak_self| MetalDevice {
            weak_self: weak_self.clone(),
            sync_root: DisposableLock::new(),
            device,
            queue: RefCell::new(Some(queue)),
        })
    }

    /// The Metal device.
    pub fn mtl_device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.device
    }

    /// The command queue.
    ///
    /// # Panics
    /// Panics when the device is disposed.
    #[track_caller]
    pub fn queue(&self) -> Retained<ProtocolObject<dyn MTLCommandQueue>> {
        match self.queue.borrow().clone() {
            Some(queue) => queue,
            None => panic!("Cannot access a disposed object: MetalDevice"),
        }
    }
}

impl IOptionalFeatureProvider for MetalDevice {
    /// The reference's device has no optional features. The device
    /// announces itself as a Metal device here, which is how a render
    /// backend of the port finds the Metal contract of a graphics context
    /// (`ferroui-metal`).
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IMetalDevice>() {
            let this: Rc<dyn IMetalDevice> = self.weak_self.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for MetalDevice {
    fn is_lost(&self) -> bool {
        false
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.sync_root.lock()
    }

    fn dispose(&self) {
        let queue = self.queue.borrow_mut().take();
        drop(queue);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalDevice for MetalDevice {
    fn device(&self) -> *mut c_void {
        Retained::as_ptr(&self.device) as *mut c_void
    }

    fn command_queue(&self) -> *mut c_void {
        Retained::as_ptr(&self.queue()) as *mut c_void
    }
}
