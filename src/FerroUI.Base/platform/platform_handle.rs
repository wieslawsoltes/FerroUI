use super::IPlatformHandle;
use std::any::Any;
use std::fmt;

/// Represents a platform-specific handle.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlatformHandle {
    handle: isize,
    handle_descriptor: Option<String>,
}

impl PlatformHandle {
    /// Creates a handle.
    ///
    /// `descriptor` is an optional string that describes what `handle`
    /// represents.
    pub fn new(handle: isize, descriptor: Option<&str>) -> Self {
        Self { handle, handle_descriptor: descriptor.map(str::to_owned) }
    }
}

impl IPlatformHandle for PlatformHandle {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        self.handle_descriptor.as_deref()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn equals(&self, other: &dyn IPlatformHandle) -> bool {
        other.as_any().downcast_ref::<PlatformHandle>().is_some_and(|other| self == other)
    }
}

impl fmt::Display for PlatformHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PlatformHandle {{ {} = {} }}", self.handle_descriptor.as_deref().unwrap_or(""), self.handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    struct IdentityHandle(u8);

    impl IPlatformHandle for IdentityHandle {
        fn handle(&self) -> isize {
            1
        }

        fn handle_descriptor(&self) -> Option<&str> {
            Some("HWND")
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn hash_of(value: &PlatformHandle) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn display_writes_descriptor_and_handle() {
        assert_eq!(PlatformHandle::new(42, Some("HWND")).to_string(), "PlatformHandle { HWND = 42 }");
        assert_eq!(PlatformHandle::new(7, None).to_string(), "PlatformHandle {  = 7 }");
    }

    #[test]
    fn handles_compare_by_handle_and_descriptor() {
        let a = PlatformHandle::new(1, Some("HWND"));
        assert_eq!(a, PlatformHandle::new(1, Some("HWND")));
        assert_eq!(hash_of(&a), hash_of(&PlatformHandle::new(1, Some("HWND"))));
        assert_ne!(a, PlatformHandle::new(2, Some("HWND")));
        assert_ne!(a, PlatformHandle::new(1, Some("NSView")));
        assert_ne!(a, PlatformHandle::new(1, None));

        assert!(a.equals(&PlatformHandle::new(1, Some("HWND"))));
        assert!(!a.equals(&PlatformHandle::new(2, Some("HWND"))));
    }

    #[test]
    fn handles_of_other_types_are_not_equal() {
        let a = PlatformHandle::new(1, Some("HWND"));
        let b = IdentityHandle(0);
        assert!(!a.equals(&b));
        assert!(!b.equals(&a));
        // The default equality is identity.
        assert!(b.equals(&b));
        assert!(!b.equals(&IdentityHandle(b.0)));
    }
}
