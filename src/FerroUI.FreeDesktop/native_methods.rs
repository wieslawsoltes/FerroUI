//! Reading a symbolic link (the port of `NativeMethods.cs`).
//!
//! The reference calls `readlink` of the C library with a buffer of a
//! fixed size; the standard library makes the same call and grows the
//! buffer.

/// The target of a symbolic link as it is stored (not resolved); `None`
/// when `path` is not a link that can be read.
pub fn read_link(path: &str) -> Option<String> {
    std::fs::read_link(path).ok().map(|target| target.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;

    #[test]
    fn a_link_gives_its_target_as_it_is_stored() {
        let directory = std::env::temp_dir().join(format!("ferroui-freedesktop-link-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let link = directory.join("label");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("../../sda1", &link).unwrap();

        assert_eq!(read_link(&link.to_string_lossy()).as_deref(), Some("../../sda1"));
        // Not a link, and not there.
        assert_eq!(read_link(&directory.to_string_lossy()), None);
        assert_eq!(read_link(&directory.join("missing").to_string_lossy()), None);

        std::fs::remove_dir_all(&directory).unwrap();
    }
}
