//! What the receiver of data on the clipboard is asked to do with it (WinUI's
//! `DataPackage.RequestedOperation` and `DataPackageView.RequestedOperation`).
//!
//! The platforms keep it in formats of their own: Windows in `Preferred DropEffect`, a DWORD of
//! the effects (the values of `DROPEFFECT_*`, which are those of `DataPackageOperation`), and the
//! file managers of Linux in `x-special/gnome-copied-files`: `copy` or `cut`, then the URIs of the
//! files, one a line. macOS, iOS, Android and the browser have no such format: there the
//! operation is `NONE` when read and is not written (DEVIATIONS.md, Input).

use super::platform::ClipboardError;
use super::{AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransferItem, IAsyncDataTransfer, LocalBoxFuture};
use crate::platform::storage::IStorageItem;
use bitflags::bitflags;
use std::rc::Rc;

bitflags! {
    /// The operations a receiver of data is asked to do (WinUI's `DataPackageOperation`).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DataPackageOperation: u32 {
        /// No operation is asked for.
        const NONE = 0;
        /// The data is copied.
        const COPY = 1;
        /// The data is moved: the source removes it when it is pasted (a cut).
        const MOVE = 2;
        /// The data is linked.
        const LINK = 4;
    }
}

/// The format of Windows.
fn preferred_drop_effect() -> DataFormatOf<Rc<[u8]>> {
    DataFormat::create_bytes_platform_format("Preferred DropEffect")
}

/// The format of the file managers of Linux.
fn gnome_copied_files() -> DataFormatOf<Rc<[u8]>> {
    DataFormat::create_bytes_platform_format("x-special/gnome-copied-files")
}

/// The operation of `Preferred DropEffect`.
pub(crate) fn from_drop_effect(bytes: &[u8]) -> DataPackageOperation {
    let value = bytes.get(..4).map_or(0, |bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    DataPackageOperation::from_bits_truncate(value)
}

/// The operation of `x-special/gnome-copied-files`: its first line.
pub(crate) fn from_gnome_copied_files(bytes: &[u8]) -> DataPackageOperation {
    match bytes.split(|byte| *byte == b'\n').next().map(|line| line.trim_ascii()) {
        Some(b"cut") => DataPackageOperation::MOVE,
        Some(b"copy") => DataPackageOperation::COPY,
        _ => DataPackageOperation::NONE,
    }
}

/// The content of `x-special/gnome-copied-files` for the files at `uris`: `None` for an operation
/// the file managers do not have (only a copy and a cut) or no files.
pub(crate) fn to_gnome_copied_files(operation: DataPackageOperation, uris: &[String]) -> Option<Vec<u8>> {
    let verb = if operation.contains(DataPackageOperation::MOVE) {
        "cut"
    } else if operation.contains(DataPackageOperation::COPY) {
        "copy"
    } else {
        return None;
    };
    if uris.is_empty() {
        return None;
    }
    Some(std::iter::once(verb).chain(uris.iter().map(String::as_str)).collect::<Vec<_>>().join("\n").into_bytes())
}

impl DataPackageOperation {
    /// The item that asks for the operation on this platform for the copied `files`, `None` where
    /// the platform has no format for it. Added to a data transfer with its other items.
    pub fn to_data_transfer_item(self, files: &[Rc<dyn IStorageItem>]) -> Option<Rc<DataTransferItem>> {
        if self.is_empty() {
            return None;
        }
        if cfg!(windows) {
            let bytes: Rc<[u8]> = Rc::from(self.bits().to_le_bytes().as_slice());
            return Some(DataTransferItem::create(&preferred_drop_effect(), Some(bytes)));
        }
        if cfg!(target_os = "linux") {
            let uris: Vec<String> = files.iter().map(|file| file.path().to_string()).collect();
            let bytes: Rc<[u8]> = Rc::from(to_gnome_copied_files(self, &uris)?);
            return Some(DataTransferItem::create(&gnome_copied_files(), Some(bytes)));
        }
        None
    }

    /// The operation the data on the clipboard asks for (WinUI's
    /// `DataPackageView.RequestedOperation`); `NONE` when the source asked for none or the
    /// platform has no format for it.
    pub fn of_async(data: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<DataPackageOperation, ClipboardError>> {
        Box::pin(async move {
            if let Some(bytes) = data.try_get_value_async(&preferred_drop_effect()).await? {
                return Ok(from_drop_effect(&bytes));
            }
            if let Some(bytes) = data.try_get_value_async(&gnome_copied_files()).await? {
                return Ok(from_gnome_copied_files(&bytes));
            }
            Ok(DataPackageOperation::NONE)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_formats_of_the_platforms_give_the_operation() {
        assert_eq!(DataPackageOperation::MOVE, from_drop_effect(&2u32.to_le_bytes()));
        assert_eq!(DataPackageOperation::COPY | DataPackageOperation::LINK, from_drop_effect(&5u32.to_le_bytes()));
        assert_eq!(DataPackageOperation::NONE, from_drop_effect(&[]));
        assert_eq!(DataPackageOperation::MOVE, from_gnome_copied_files(b"cut\nfile:///data/a.txt"));
        assert_eq!(DataPackageOperation::COPY, from_gnome_copied_files(b"copy\nfile:///data/a.txt"));
        assert_eq!(DataPackageOperation::NONE, from_gnome_copied_files(b"file:///data/a.txt"));
    }

    #[test]
    fn the_file_managers_of_linux_read_back_what_is_written() {
        let uris = ["file:///data/a.txt".to_owned(), "file:///data/b%20c.txt".to_owned()];
        let cut = to_gnome_copied_files(DataPackageOperation::MOVE, &uris).unwrap();
        assert_eq!(b"cut\nfile:///data/a.txt\nfile:///data/b%20c.txt".as_slice(), cut.as_slice());
        assert_eq!(DataPackageOperation::MOVE, from_gnome_copied_files(&cut));
        let copy = to_gnome_copied_files(DataPackageOperation::COPY, &uris).unwrap();
        assert_eq!(DataPackageOperation::COPY, from_gnome_copied_files(&copy));
        // A link and no files are not written.
        assert!(to_gnome_copied_files(DataPackageOperation::LINK, &uris).is_none());
        assert!(to_gnome_copied_files(DataPackageOperation::COPY, &[]).is_none());
    }

    #[test]
    fn the_item_of_the_operation_is_the_one_of_the_platform() {
        let item = DataPackageOperation::MOVE.to_data_transfer_item(&[]);
        // Windows writes the effect alone; Linux needs the files.
        assert_eq!(cfg!(windows), item.is_some());
        assert!(DataPackageOperation::NONE.to_data_transfer_item(&[]).is_none());
    }
}
