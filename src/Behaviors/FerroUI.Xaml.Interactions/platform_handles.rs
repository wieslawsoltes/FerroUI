//! Not from the reference: handles of the platform services the actions of the crate hold in
//! properties and pass to commands.
//!
//! A property value and an untyped value compare with `==`; the contracts of the platform
//! services (the clipboard, the storage provider, the storage items, the data transfers) do not,
//! so the actions hold them in handles that compare by identity, as the reference compares its
//! interface references.
//!
//! Deviation (DEVIATIONS.md, Behaviors): the properties typed `IClipboard`, `IStorageProvider`,
//! `IStorageFolder` and `IAsyncDataTransfer` in the reference hold the handles of this module,
//! and the picked files and folders are passed to commands as lists of handles.

use ferroui_base::input::platform::IClipboard;
use ferroui_base::input::IAsyncDataTransfer;
use ferroui_base::platform::storage::{IStorageFile, IStorageFolder, IStorageItem, IStorageProvider};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

macro_rules! handle {
    ($(#[$meta:meta])* $name:ident => $contract:ident) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name(pub Rc<dyn $contract>);

        impl $name {
            /// The handle of `value`.
            pub fn new(value: Rc<dyn $contract>) -> Self {
                Self(value)
            }

            /// The shared object.
            pub fn get(&self) -> &Rc<dyn $contract> {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = dyn $contract;

            fn deref(&self) -> &(dyn $contract + 'static) {
                &*self.0
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
            }
        }

        impl From<Rc<dyn $contract>> for $name {
            fn from(value: Rc<dyn $contract>) -> Self {
                Self(value)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:p})", stringify!($name), Rc::as_ptr(&self.0))
            }
        }
    };
}

handle!(
    /// A clipboard (`IClipboard`).
    ClipboardRef => IClipboard
);

handle!(
    /// A storage provider (`IStorageProvider`).
    StorageProviderRef => IStorageProvider
);

handle!(
    /// A folder of a storage provider (`IStorageFolder`).
    StorageFolderRef => IStorageFolder
);

handle!(
    /// A file of a storage provider (`IStorageFile`).
    StorageFileRef => IStorageFile
);

handle!(
    /// A file or folder of a storage provider (`IStorageItem`).
    StorageItemRef => IStorageItem
);

handle!(
    /// A data transfer that is read asynchronously (`IAsyncDataTransfer`).
    AsyncDataTransferRef => IAsyncDataTransfer
);
