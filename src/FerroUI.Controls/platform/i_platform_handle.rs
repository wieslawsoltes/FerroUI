use std::any::Any;

/// Represents a platform-specific handle.
pub trait IPlatformHandle {
    /// Gets the handle (a native pointer or identifier).
    fn handle(&self) -> isize;

    /// Gets an optional string that describes what `handle` represents.
    fn handle_descriptor(&self) -> Option<&str>;

    /// Lets backends recover the concrete handle type.
    fn as_any(&self) -> &dyn Any;

    /// Whether this handle is equal to `other`.
    ///
    /// The default is identity: a handle is only equal to itself. Value-like
    /// handles (see [`PlatformHandle`](super::PlatformHandle)) compare their
    /// contents.
    fn equals(&self, other: &dyn IPlatformHandle) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn IPlatformHandle)
    }

    /// The handle as the handle of a native control that can be destroyed,
    /// if it is one (C# `handle as INativeControlHostDestroyableControlHandle`).
    /// An implementation of that contract returns itself.
    fn as_native_control_host_destroyable_control_handle(
        &self,
    ) -> Option<&dyn super::INativeControlHostDestroyableControlHandle> {
        None
    }
}
