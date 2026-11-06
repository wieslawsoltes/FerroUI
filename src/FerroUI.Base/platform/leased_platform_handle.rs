use super::{IPlatformHandle, PlatformHandle};
use crate::rendering::composition::frames::FrameLease;
use std::any::Any;

/// A native import handle with explicit immutable-frame ownership. Composition
/// carries this lease across the queued import, even when it must copy a
/// thread-local handle's raw value. GPU consumers retain their own frame leases.
#[derive(Clone)]
pub struct LeasedPlatformHandle {
    handle: PlatformHandle,
    frame: FrameLease,
}
impl LeasedPlatformHandle {
    pub fn new(handle: isize, descriptor: Option<&str>, frame: FrameLease) -> Self {
        Self {
            handle: PlatformHandle::new(handle, descriptor),
            frame,
        }
    }
    pub fn frame(&self) -> &FrameLease {
        &self.frame
    }
}
impl IPlatformHandle for LeasedPlatformHandle {
    fn handle(&self) -> isize {
        self.handle.handle()
    }
    fn handle_descriptor(&self) -> Option<&str> {
        self.handle.handle_descriptor()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
