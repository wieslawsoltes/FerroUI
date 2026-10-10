//! Adapts shell virtual-file clipboard formats to storage files.
//!
//! Windows Explorer uses FileGroupDescriptorW plus indexed FileContents
//! entries when a file has no filesystem path yet, for example while
//! dragging an entry from a ZIP folder.
//!
//! Reading the descriptors of a block is a function of its bytes, tested
//! on every host; the rest (`imp`) asks a data object of the system.
//!
//! The reference hands out files that a consumer may read and dispose on a
//! worker thread, and marshals the stream of a file and the asynchronous
//! capability of the data object for that thread. A storage item of the
//! port is a value of the UI thread (it is shared with `Rc`), so a file is
//! read and disposed there; the interface pointers still travel through
//! the marshaling streams, as in the reference, which on one thread gives
//! back the pointer that went in.

use crate::interop::unmanaged_methods::FILEDESCRIPTORW;

/// A file of a group descriptor: its name and, when the source says it,
/// its size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Descriptor {
    pub name: String,
    pub size: Option<u64>,
}

/// The descriptors of the bytes of a `FileGroupDescriptorW` block: a count
/// and that many `FILEDESCRIPTORW` structures. A block that is too small
/// for the descriptors it announces has none.
pub(crate) fn read_descriptors_from_bytes(bytes: &[u8]) -> Vec<Descriptor> {
    let available_size = bytes.len() as u64;
    if available_size < 4 {
        return Vec::new();
    }

    let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if count > i32::MAX as u32 || 4 + u64::from(count) * FILEDESCRIPTORW::SIZE as u64 > available_size {
        return Vec::new();
    }

    let u32_at = |offset: usize| u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
    let mut descriptors = Vec::with_capacity(count as usize);

    for index in 0..count as usize {
        let descriptor = 4 + index * FILEDESCRIPTORW::SIZE;
        let file_name = descriptor + FILEDESCRIPTORW::OFFSET_OF_FILE_NAME;
        let name: Vec<u16> = bytes[file_name..file_name + FILEDESCRIPTORW::FILE_NAME_LENGTH * 2]
            .chunks_exact(2)
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .take_while(|&unit| unit != 0)
            .collect();

        let name = String::from_utf16_lossy(&name);
        let mut size = None;

        if (u32_at(descriptor + FILEDESCRIPTORW::OFFSET_OF_FLAGS) & FILEDESCRIPTORW::FD_FILESIZE) != 0 {
            size = Some(
                (u64::from(u32_at(descriptor + FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_HIGH)) << 32)
                    | u64::from(u32_at(descriptor + FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_LOW)),
            );
        }

        descriptors.push(Descriptor { name, size });
    }

    descriptors
}

/// The address a virtual file has: its name, escaped as the data of a
/// URI, under a scheme of its own.
pub(crate) fn virtual_file_uri(name: &str) -> String {
    let mut uri = String::from("virtual-file:///");
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

#[cfg(windows)]
pub(crate) use imp::*;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::clipboard_format_registry::ClipboardFormatRegistry;
    use crate::interop::unmanaged_methods::{self, DVASPECT, FORMATETC, HRESULT, STATFLAG_NONAME, STATSTG, STGMEDIUM, TYMED};
    use crate::ole_data_object_helper;
    use crate::win32_com::{DropEffect, IDataObject, IDataObjectAsyncCapability, IStream};
    use ferroui_base::input::{DataFormat, DataFormatOf, LocalBoxFuture};
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::storage::{IStorageFile, IStorageFolder, IStorageItem, StorageItemProperties};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::utilities::Uri;
    use ferroui_microcom::{ComPtr, Interface};
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::io::{self, Read, Seek, SeekFrom, Write};
    use std::rc::Rc;

    thread_local! {
        static FILE_GROUP_DESCRIPTOR_FORMAT: DataFormatOf<Rc<[u8]>> =
            DataFormat::create_bytes_platform_format("FileGroupDescriptorW");
        static FILE_CONTENTS_FORMAT: DataFormatOf<Rc<[u8]>> = DataFormat::create_bytes_platform_format("FileContents");
    }

    pub(crate) fn file_group_descriptor_format() -> DataFormatOf<Rc<[u8]>> {
        FILE_GROUP_DESCRIPTOR_FORMAT.with(Clone::clone)
    }

    pub(crate) fn file_contents_format() -> DataFormatOf<Rc<[u8]>> {
        FILE_CONTENTS_FORMAT.with(Clone::clone)
    }

    fn log_warning(message: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
            logger.log(None, message);
        }
    }

    /// Releases a storage medium a data object filled when dropped.
    struct MediumGuard(STGMEDIUM);

    impl Drop for MediumGuard {
        fn drop(&mut self) {
            // SAFETY: the medium was filled by a successful `GetData` and
            // is not used after this.
            unsafe { unmanaged_methods::release_stg_medium(&mut self.0) };
        }
    }

    /// The virtual files of a data object; `None` when it has none.
    pub(crate) fn try_create_files(data_object: &IDataObject) -> Option<Vec<Rc<dyn IStorageFile>>> {
        let mut descriptor_format =
            ole_data_object_helper::to_format_etc(ClipboardFormatRegistry::get_or_add_format(&file_group_descriptor_format()));
        let mut descriptor_medium = STGMEDIUM::default();

        // SAFETY: two structures of this frame, for the duration of the
        // call.
        if unsafe { data_object.get_data(&mut descriptor_format, &mut descriptor_medium) } != HRESULT::S_OK {
            return None;
        }

        let descriptors = {
            let descriptor_medium = MediumGuard(descriptor_medium);
            if descriptor_medium.0.tymed != TYMED::TYMED_HGLOBAL || descriptor_medium.0.unionmember == 0 {
                return None;
            }

            // SAFETY: the block of a medium the data object just filled,
            // alive until the guard releases it.
            unsafe { read_descriptors(descriptor_medium.0.unionmember) }
        };

        if descriptors.is_empty() {
            return None;
        }

        // Keep the source-side data object alive after IDropTarget.Drop returns. Consumers that retain
        // returned virtual files must dispose them once they finish reading; the operation ends then.
        let operation = Rc::new(Operation::new(begin_async_operation(data_object), descriptors.len()));
        let mut files: Vec<Rc<dyn IStorageFile>> = Vec::with_capacity(descriptors.len());
        let file_contents_format = ClipboardFormatRegistry::get_or_add_format(&file_contents_format());

        for (index, descriptor) in descriptors.iter().enumerate() {
            match try_create_file(data_object, file_contents_format, descriptor, index as i32, &operation) {
                Some(file) => files.push(file),
                None => operation.complete_file(),
            }
        }

        if files.is_empty() {
            None
        } else {
            Some(files)
        }
    }

    fn try_create_file(
        data_object: &IDataObject,
        file_contents_format: u16,
        descriptor: &Descriptor,
        index: i32,
        operation: &Rc<Operation>,
    ) -> Option<Rc<dyn IStorageFile>> {
        let create = || -> io::Result<Rc<dyn IStorageFile>> {
            let mut content_format = FORMATETC {
                cf_format: file_contents_format,
                dw_aspect: DVASPECT::DVASPECT_CONTENT,
                lindex: index,
                ptd: 0,
                tymed: TYMED::TYMED_ISTREAM,
            };
            let mut content_medium = STGMEDIUM::default();
            // SAFETY (of both calls): two structures of this frame, for
            // the duration of the call.
            let mut result = unsafe { data_object.get_data(&mut content_format, &mut content_medium) };
            if result != HRESULT::S_OK {
                // Some IDataObject implementations expose FileContents only as HGLOBAL, so retry
                // with that medium explicitly when the IStream request fails.
                content_format.tymed = TYMED::TYMED_HGLOBAL;
                content_medium = STGMEDIUM::default();
                result = unsafe { data_object.get_data(&mut content_format, &mut content_medium) };
            }

            if result != HRESULT::S_OK {
                return Err(io::Error::other(format!(
                    "The virtual file stream is unavailable (GetData HRESULT 0x{result:08X}, index {index})."
                )));
            }

            let content_medium = MediumGuard(content_medium);
            if content_medium.0.tymed == TYMED::TYMED_ISTREAM && content_medium.0.unionmember != 0 {
                // FileContents is obtained on the OLE/UI thread, while consumers commonly copy it
                // on a worker thread. Marshal IStream explicitly to preserve COM apartment affinity.
                // SAFETY: the stream of a medium the data object just
                // filled, alive until the guard releases it.
                let marshaled_stream = unsafe {
                    unmanaged_methods::co_marshal_inter_thread_interface_in_stream(
                        &IStream::IID,
                        content_medium.0.unionmember as *mut c_void,
                    )
                }
                .map_err(|code| io::Error::other(format!("The stream of the virtual file could not be marshaled (0x{code:08X}).")))?;

                // SAFETY: the function returned a stream, of which this
                // code owns the one reference.
                let marshaled_stream = unsafe { ComPtr::from_raw(marshaled_stream as *mut IStream) };
                return Ok(Rc::new(VirtualStorageFile::new(descriptor, marshaled_stream, None, operation.clone())));
            }

            if content_medium.0.tymed == TYMED::TYMED_HGLOBAL && content_medium.0.unionmember != 0 {
                // SAFETY: the block of a medium the data object just
                // filled, alive until the guard releases it.
                let contents =
                    unsafe { ole_data_object_helper::read_bytes_from_hglobal(content_medium.0.unionmember) }.unwrap_or_default();
                return Ok(Rc::new(VirtualStorageFile::new(descriptor, None, Some(Rc::from(contents)), operation.clone())));
            }

            Err(io::Error::other(format!(
                "The virtual file stream is unavailable (unexpected TYMED {:?}, index {index}).",
                content_medium.0.tymed
            )))
        };

        match create() {
            Ok(file) => Some(file),
            Err(exception) => {
                log_warning(&format!("Failed to create virtual file at index {index}: {exception}"));
                None
            }
        }
    }

    /// The descriptors of a `FileGroupDescriptorW` block.
    ///
    /// # Safety
    /// `h_global` is a live block of global memory.
    pub(crate) unsafe fn read_descriptors(h_global: isize) -> Vec<Descriptor> {
        // SAFETY: the contract of this function.
        match unsafe { unmanaged_methods::read_global(h_global) } {
            Some(bytes) => read_descriptors_from_bytes(&bytes),
            None => Vec::new(),
        }
    }

    /// Starts the asynchronous operation of a data object that has the
    /// capability, and returns the capability marshaled into a stream;
    /// `None` for a data object without it.
    fn begin_async_operation(data_object: &IDataObject) -> Option<ComPtr<IStream>> {
        let capability = ComPtr::from_ref(data_object).cast::<IDataObjectAsyncCapability>().ok()?;
        capability.set_async_mode(1).ok()?;
        // SAFETY: the reserved argument is null.
        unsafe { capability.start_operation(std::ptr::null_mut()) }.ok()?;

        // EndOperation can run when a consumer disposes the last file on a worker thread. Marshal
        // the capability now so that call uses a proxy for the completing thread's COM apartment.
        // SAFETY: a live object that implements the interface.
        let result = unsafe {
            unmanaged_methods::co_marshal_inter_thread_interface_in_stream(
                &IDataObjectAsyncCapability::IID,
                capability.as_ptr().cast(),
            )
        };
        match result {
            // SAFETY: the function returned a stream, of which this code
            // owns the one reference.
            Ok(marshaled_capability) => unsafe { ComPtr::from_raw(marshaled_capability as *mut IStream) },
            Err(_) => {
                // The operation was started.
                // SAFETY: the reserved argument is null.
                let _ = unsafe { capability.end_operation(0, std::ptr::null_mut(), DropEffect::Copy.0) };
                None
            }
        }
    }

    /// The asynchronous operation of a data object, which ends when the
    /// last of its files is completed.
    struct Operation {
        marshaled_capability: RefCell<Option<ComPtr<IStream>>>,
        remaining_count: Cell<usize>,
    }

    impl Operation {
        fn new(marshaled_capability: Option<ComPtr<IStream>>, remaining_count: usize) -> Operation {
            Operation { marshaled_capability: RefCell::new(marshaled_capability), remaining_count: Cell::new(remaining_count) }
        }

        fn complete_file(&self) {
            let remaining = self.remaining_count.get().saturating_sub(1);
            self.remaining_count.set(remaining);
            if remaining != 0 {
                return;
            }

            let Some(marshaled) = self.marshaled_capability.borrow_mut().take() else {
                return;
            };

            // SAFETY: a stream made by the marshaling function; its one
            // reference is given to the call.
            let result = unsafe {
                unmanaged_methods::co_get_interface_and_release_stream(
                    marshaled.into_raw().cast(),
                    &IDataObjectAsyncCapability::IID,
                )
            };
            let ended = result.and_then(|capability_pointer| {
                // SAFETY: the call returned an interface pointer of the
                // identifier asked for, with a reference for this code.
                let capability = unsafe { ComPtr::from_raw(capability_pointer as *mut IDataObjectAsyncCapability) };
                match capability {
                    // SAFETY: the reserved argument is null.
                    Some(capability) => unsafe { capability.end_operation(0, std::ptr::null_mut(), DropEffect::Copy.0) }
                        .map_err(|error| error.0),
                    None => Ok(()),
                }
            });
            if let Err(code) = ended {
                log_warning(&format!("Failed to end an asynchronous virtual-file operation: 0x{code:08X}"));
            }
        }
    }

    struct VirtualStorageFile {
        name: String,
        path: Uri,
        size: Option<u64>,
        contents: Option<Rc<[u8]>>,
        operation: Rc<Operation>,
        marshaled_stream: RefCell<Option<ComPtr<IStream>>>,
        disposed: Cell<bool>,
    }

    impl VirtualStorageFile {
        fn new(
            descriptor: &Descriptor,
            marshaled_stream: Option<ComPtr<IStream>>,
            contents: Option<Rc<[u8]>>,
            operation: Rc<Operation>,
        ) -> VirtualStorageFile {
            VirtualStorageFile {
                name: descriptor.name.clone(),
                size: descriptor.size,
                path: Uri::absolute(&virtual_file_uri(&descriptor.name)).expect("the address of a virtual file is an absolute URI"),
                contents,
                operation,
                marshaled_stream: RefCell::new(marshaled_stream),
                disposed: Cell::new(false),
            }
        }

        fn open_read(&self) -> io::Result<Box<dyn Read>> {
            if let Some(contents) = &self.contents {
                return Ok(Box::new(io::Cursor::new(contents.clone())));
            }

            let Some(marshaled_stream) = self.marshaled_stream.borrow_mut().take() else {
                return Err(io::Error::other("The virtual file stream has already been opened or disposed."));
            };

            // SAFETY: a stream made by the marshaling function; its one
            // reference is given to the call.
            let stream_pointer =
                unsafe { unmanaged_methods::co_get_interface_and_release_stream(marshaled_stream.into_raw().cast(), &IStream::IID) }
                    .map_err(|code| io::Error::other(format!("The stream of the virtual file could not be opened (0x{code:08X}).")))?;

            // SAFETY: the call returned an interface pointer of the
            // identifier asked for, with a reference for this code.
            let stream = unsafe { ComPtr::from_raw(stream_pointer as *mut IStream) }
                .ok_or_else(|| io::Error::other("The stream of the virtual file is null."))?;
            Ok(Box::new(ComReadStream { stream }))
        }
    }

    impl IDisposable for VirtualStorageFile {
        fn dispose(&self) {
            if self.disposed.replace(true) {
                return;
            }

            self.marshaled_stream.borrow_mut().take();

            self.operation.complete_file();
        }
    }

    impl Drop for VirtualStorageFile {
        /// A file that is dropped without being disposed completes its
        /// part of the operation too: the reference relies on its
        /// consumers, and on the collector of its runtime for the COM
        /// references of the ones that forget.
        fn drop(&mut self) {
            self.dispose();
        }
    }

    impl IStorageItem for VirtualStorageFile {
        fn name(&self) -> String {
            self.name.clone()
        }

        fn path(&self) -> Uri {
            self.path.clone()
        }

        fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
            Box::pin(std::future::ready(StorageItemProperties::new(self.size, None, None)))
        }

        fn can_bookmark(&self) -> bool {
            false
        }

        fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> {
            Box::pin(std::future::ready(None))
        }

        fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> {
            Box::pin(std::future::ready(None))
        }

        fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> {
            Box::pin(std::future::ready(Err(io::Error::from(io::ErrorKind::Unsupported))))
        }

        fn move_async(&self, _destination: Rc<dyn IStorageFolder>) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> {
            Box::pin(std::future::ready(Err(io::Error::from(io::ErrorKind::Unsupported))))
        }

        fn as_storage_file(self: Rc<Self>) -> Option<Rc<dyn IStorageFile>> {
            Some(self)
        }
    }

    impl IStorageFile for VirtualStorageFile {
        fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
            Box::pin(std::future::ready(self.open_read()))
        }

        fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
            Box::pin(std::future::ready(Err(io::Error::from(io::ErrorKind::Unsupported))))
        }
    }

    /// Reads a stream of the system.
    pub(crate) struct ComReadStream {
        stream: ComPtr<IStream>,
    }

    impl ComReadStream {
        /// The length of the stream. (A member of the stream of the
        /// reference; a reader behind the storage contract is not asked
        /// for it.)
        #[allow(dead_code)]
        pub(crate) fn length(&self) -> io::Result<u64> {
            let mut stat = STATSTG::default();
            // SAFETY: a structure of this frame the stream writes to; no
            // name is asked for, so nothing is allocated for the caller.
            let result = unsafe { self.stream.stat((&mut stat as *mut STATSTG).cast(), STATFLAG_NONAME) };
            if result as u32 != HRESULT::S_OK {
                return Err(io::Error::other(format!("The length of the stream could not be read (0x{:08X}).", result as u32)));
            }

            Ok(stat.cb_size)
        }
    }

    impl Read for ComReadStream {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if buffer.is_empty() {
                return Ok(0);
            }

            let mut bytes_read = 0u32;
            let count = buffer.len().min(u32::MAX as usize) as u32;
            // SAFETY: a buffer of at least `count` bytes and a count of
            // this frame.
            let result = unsafe { self.stream.read(buffer.as_mut_ptr().cast(), count, &mut bytes_read) };
            if result as u32 != HRESULT::S_OK && result as u32 != HRESULT::S_FALSE {
                return Err(io::Error::other(format!("The stream could not be read (0x{:08X}).", result as u32)));
            }

            Ok(bytes_read as usize)
        }
    }

    impl Seek for ComReadStream {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            let (offset, dw_origin) = match position {
                SeekFrom::Start(offset) => (i64::try_from(offset).map_err(io::Error::other)?, 0),
                SeekFrom::Current(offset) => (offset, 1),
                SeekFrom::End(offset) => (offset, 2),
            };

            let mut position = 0u64;
            // SAFETY: a position of this frame the stream writes to.
            let result = unsafe { self.stream.seek(offset, dw_origin, &mut position) };
            if result as u32 != HRESULT::S_OK {
                return Err(io::Error::other(format!("The position of the stream could not be set (0x{:08X}).", result as u32)));
            }

            Ok(position)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The bytes of a group descriptor, as `CreateFileGroupDescriptor` of
    /// the tests of the reference lays them out.
    pub(crate) fn create_file_group_descriptor_bytes(files: &[(&str, u64)]) -> Vec<u8> {
        let mut bytes = vec![0u8; 4 + files.len() * FILEDESCRIPTORW::SIZE];
        bytes[0..4].copy_from_slice(&(files.len() as u32).to_le_bytes());
        for (index, (name, size)) in files.iter().enumerate() {
            let descriptor = 4 + index * FILEDESCRIPTORW::SIZE;
            let mut put = |offset: usize, value: u32| {
                bytes[descriptor + offset..descriptor + offset + 4].copy_from_slice(&value.to_le_bytes())
            };
            put(FILEDESCRIPTORW::OFFSET_OF_FLAGS, FILEDESCRIPTORW::FD_FILESIZE);
            put(FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_HIGH, (size >> 32) as u32);
            put(FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_LOW, *size as u32);
            let name_offset = descriptor + FILEDESCRIPTORW::OFFSET_OF_FILE_NAME;
            for (i, unit) in name.encode_utf16().take(FILEDESCRIPTORW::FILE_NAME_LENGTH - 1).enumerate() {
                bytes[name_offset + i * 2..name_offset + i * 2 + 2].copy_from_slice(&unit.to_le_bytes());
            }
        }
        bytes
    }

    #[test]
    fn the_structure_of_a_descriptor_has_the_layout_of_the_system() {
        // dwFlags, clsid, sizel, pointl, dwFileAttributes, three times,
        // the two halves of the size, the name.
        assert_eq!(FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_HIGH, 4 + 16 + 8 + 8 + 4 + 3 * 8);
        assert_eq!(FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_LOW, FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_HIGH + 4);
        assert_eq!(FILEDESCRIPTORW::OFFSET_OF_FILE_NAME, FILEDESCRIPTORW::OFFSET_OF_FILE_SIZE_LOW + 4);
        assert_eq!(FILEDESCRIPTORW::SIZE, FILEDESCRIPTORW::OFFSET_OF_FILE_NAME + 260 * 2);
    }

    #[test]
    fn reads_the_names_and_the_sizes_of_the_descriptors() {
        let bytes = create_file_group_descriptor_bytes(&[("archive-entry.txt", 4), ("żółć.bin", (7 << 32) | 9)]);
        assert_eq!(
            read_descriptors_from_bytes(&bytes),
            [
                Descriptor { name: "archive-entry.txt".to_string(), size: Some(4) },
                Descriptor { name: "żółć.bin".to_string(), size: Some((7 << 32) | 9) },
            ]
        );
    }

    #[test]
    fn a_descriptor_without_the_size_flag_has_no_size() {
        let mut bytes = create_file_group_descriptor_bytes(&[("a", 5)]);
        bytes[4..8].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(read_descriptors_from_bytes(&bytes), [Descriptor { name: "a".to_string(), size: None }]);
    }

    #[test]
    fn a_name_that_fills_its_member_has_no_terminator() {
        let name = "n".repeat(300);
        let mut bytes = create_file_group_descriptor_bytes(&[(&name, 0)]);
        // The helper leaves room for a terminator; fill the last unit.
        let last = 4 + FILEDESCRIPTORW::OFFSET_OF_FILE_NAME + 259 * 2;
        bytes[last..last + 2].copy_from_slice(&u16::from(b'n').to_le_bytes());
        assert_eq!(read_descriptors_from_bytes(&bytes)[0].name.len(), 260);
    }

    /// `Rejects_Truncated_FileGroupDescriptorW` of the tests of the
    /// reference, on the bytes of the block (the test against a block of
    /// global memory runs on Windows, in `ole_tests.rs`).
    #[test]
    fn rejects_truncated_file_group_descriptor_w_bytes() {
        assert!(read_descriptors_from_bytes(&1u32.to_le_bytes()).is_empty());
        assert!(read_descriptors_from_bytes(&[]).is_empty());
        assert!(read_descriptors_from_bytes(&[1, 0]).is_empty());
        let mut bytes = create_file_group_descriptor_bytes(&[("a", 1), ("b", 2)]);
        bytes.pop();
        assert!(read_descriptors_from_bytes(&bytes).is_empty());
        // A count the block cannot hold, however large.
        let mut huge = vec![0u8; 4 + FILEDESCRIPTORW::SIZE];
        huge[0..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read_descriptors_from_bytes(&huge).is_empty());
        assert!(read_descriptors_from_bytes(&0u32.to_le_bytes()).is_empty());
    }

    #[test]
    fn the_address_of_a_virtual_file_escapes_its_name() {
        assert_eq!(virtual_file_uri("archive-entry.txt"), "virtual-file:///archive-entry.txt");
        assert_eq!(virtual_file_uri("a b/c#d.txt"), "virtual-file:///a%20b%2Fc%23d.txt");
        assert_eq!(virtual_file_uri("ż"), "virtual-file:///%C5%BC");
    }
}
