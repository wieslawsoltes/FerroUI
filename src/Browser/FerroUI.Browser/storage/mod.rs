//! The storage services of the page: the launcher, and the storage provider
//! with the streams of its files.

mod blob_readable_stream;
mod browser_launcher;
mod browser_storage_provider;
mod writeable_stream;

pub use blob_readable_stream::BlobReadableStream;
pub use browser_launcher::BrowserLauncher;
pub use browser_storage_provider::BrowserStorageProvider;
pub use writeable_stream::WriteableStream;
