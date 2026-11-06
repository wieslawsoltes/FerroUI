//! Ask Foundation for the application's user-domain support directory.
use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};
use std::{io, path::PathBuf};

/// iOS resolves this inside the current app sandbox. Discovery creates no files.
/// The existing file storage adapter creates the directory on its first write.
pub fn application_data_folder() -> io::Result<PathBuf> {
    let url = NSFileManager::defaultManager()
        .URLForDirectory_inDomain_appropriateForURL_create_error(
            NSSearchPathDirectory::ApplicationSupportDirectory,
            NSSearchPathDomainMask::UserDomainMask,
            None,
            false,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
    let path = url.to_file_path().ok_or_else(|| {
        io::Error::other("Foundation returned a non-file application support URL")
    })?;
    if !path.is_absolute() {
        return Err(io::Error::other(
            "Foundation returned a relative application support path",
        ));
    }
    Ok(path.join("FerroBoard"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_support_directory_is_stable_and_absolute_without_creating_it() {
        let first = application_data_folder().unwrap();
        assert!(first.is_absolute());
        assert_eq!(first.file_name().unwrap(), "FerroBoard");
        assert_eq!(first, application_data_folder().unwrap());
        assert!(
            first
                .parent()
                .unwrap()
                .ends_with("Library/Application Support")
        );
    }
}
