//! Shader libraries made when the application was built (a FerroUI addition,
//! docs/porting/gpu-rendering.md, "Pipelines made ahead of their first draw").
//!
//! Skia writes the Metal text of every shader of a pipeline and Metal compiles it when the
//! pipeline is first made: three quarters of what a pipeline costs on a first start. A table
//! holds libraries compiled from the same texts when the application was built
//! (`scripts/perf/metal-libraries.py`), by the text they were made from; `shader_libraries.m`
//! asks it for each text Skia hands the device, and the device compiles what the table does not
//! have, as before. The pictures are the same either way: a library of the table is what Metal
//! makes of that very text.
//!
//! The crate carries a table of ordinary drawing for each kind of device it has one for
//! (`ORDINARY`: macOS on Apple silicon, iOS on a device and in its simulator; `build.rs`,
//! `tables_of_the_target`): an application gets it without doing anything, and adds the
//! libraries of its own screens with [`register_metal_libraries`]. A table is read when the
//! device is first given a text, on the thread that gives it, not when it is registered.
//!
//! - `FERROUI_METAL_LIBRARY_RECORD=<folder>` writes every text the device is given to the
//!   folder (`<key>.metal`), which is what the script compiles.
//! - `FERROUI_METAL_LIBRARIES=<file>` reads a table from a file when the first context is made;
//!   `off` asks no table (to compare, to diagnose).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError, RwLock};

/// The first bytes of a table.
const MAGIC: &[u8; 4] = b"FML1";

/// The first bytes of a packed table: then the length of the table (u32, little endian) and the
/// table as an LZFSE stream, which the system unpacks (Apple platforms).
const PACKED: &[u8; 4] = b"FMZ1";

/// The longest table a packed table may unpack to.
const LONGEST: usize = 64 << 20;

/// `MTLLanguageVersion2_3`: what Skia asks for on every system this crate supports, and what
/// the libraries of a table are made for.
const LANGUAGE_VERSION: u64 = (2 << 16) + 3;

/// A library of a table: the length of the text it was made from, and the library.
type Library = (u32, &'static [u8]);

/// The libraries of ordinary drawing for the kind of device of the target
/// (`gpu/metal/libraries/ordinary-<platform>.fmz`, which `build.rs` names when the crate has
/// one): compiled for that platform from the texts Skia writes for the GPUs of the Apple family.
/// Another kind of device writes other texts, finds none of these and compiles its own.
/// `scripts/perf/metal-libraries.py` says how a table is made again.
#[cfg(ferro_skia_ordinary_metal_libraries)]
const ORDINARY: Option<&[u8]> = Some(include_bytes!(env!("FERROUI_SKIA_ORDINARY_METAL_LIBRARIES")));
#[cfg(not(ferro_skia_ordinary_metal_libraries))]
const ORDINARY: Option<&[u8]> = None;

/// The table this crate carries for this kind of device, unpacked, if it has one (tests).
#[cfg(test)]
pub(crate) fn ordinary_table() -> Option<&'static [u8]> {
    ORDINARY.and_then(unpacked)
}

/// The tables given and not read yet: the one of the crate first.
static GIVEN: Mutex<Vec<&'static [u8]>> = Mutex::new(Vec::new());
/// Whether `GIVEN` holds a table.
static UNREAD: AtomicBool = AtomicBool::new(ORDINARY.is_some());
/// `FERROUI_METAL_LIBRARIES=off`: the table is not asked.
static OFF: AtomicBool = AtomicBool::new(false);
/// How many texts were given a library of the table, and how many the device compiled, since
/// the process started.
static FOUND: AtomicU64 = AtomicU64::new(0);
static COMPILED: AtomicU64 = AtomicU64::new(0);

/// The libraries that were read, by the key of their text.
fn libraries() -> &'static RwLock<HashMap<u64, Library>> {
    static LIBRARIES: OnceLock<RwLock<HashMap<u64, Library>>> = OnceLock::new();
    LIBRARIES.get_or_init(Default::default)
}

/// Reads the tables that were given since the last look.
fn read_given() {
    if !UNREAD.swap(false, Ordering::AcqRel) {
        return;
    }
    static ORDINARY_READ: AtomicBool = AtomicBool::new(false);
    let mut tables = Vec::new();
    if !ORDINARY_READ.swap(true, Ordering::AcqRel) {
        tables.extend(ORDINARY);
    }
    tables.append(&mut GIVEN.lock().unwrap_or_else(PoisonError::into_inner));
    let mut libraries = libraries().write().unwrap_or_else(PoisonError::into_inner);
    for table in tables {
        libraries.extend(unpacked(table).map(parse).unwrap_or_default());
    }
}

fn record_folder() -> Option<&'static PathBuf> {
    static FOLDER: OnceLock<Option<PathBuf>> = OnceLock::new();
    FOLDER
        .get_or_init(|| {
            let folder = PathBuf::from(std::env::var_os("FERROUI_METAL_LIBRARY_RECORD").filter(|v| !v.is_empty())?);
            std::fs::create_dir_all(&folder).ok()?;
            Some(folder)
        })
        .as_ref()
}

/// The key of a shader text: FNV-1a over its bytes, which the script computes the same way.
pub(crate) fn key(text: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The libraries of a table: `FML1`, then for each library the key of its text (u64), the
/// length of the text (u32), the length of the library (u32), little endian, and the library.
/// A table that is cut short gives the libraries before the cut.
pub(crate) fn parse(table: &'static [u8]) -> Vec<(u64, Library)> {
    let mut found = Vec::new();
    let Some(mut rest) = table.strip_prefix(MAGIC) else { return found };
    while rest.len() >= 16 {
        let key = u64::from_le_bytes(rest[..8].try_into().expect("eight bytes"));
        let text_length = u32::from_le_bytes(rest[8..12].try_into().expect("four bytes"));
        let length = u32::from_le_bytes(rest[12..16].try_into().expect("four bytes")) as usize;
        rest = &rest[16..];
        if rest.len() < length {
            break;
        }
        found.push((key, (text_length, &rest[..length])));
        rest = &rest[length..];
    }
    found
}

/// A table as [`parse`] reads it: itself, or what a packed table unpacks to, which lives as
/// long as the process as the libraries of a table do. None for anything else, and for a packed
/// table where the system has nothing to unpack it with.
pub(crate) fn unpacked(table: &'static [u8]) -> Option<&'static [u8]> {
    if table.starts_with(MAGIC) {
        return Some(table);
    }
    let rest = table.strip_prefix(PACKED)?;
    let length = u32::from_le_bytes(rest.get(..4)?.try_into().ok()?) as usize;
    if length > LONGEST {
        return None;
    }
    let table = unpack(&rest[4..], length)?;
    Some(Box::leak(table.into_boxed_slice()))
}

/// The `length` bytes an LZFSE stream unpacks to, with the system's own decoder.
#[cfg(target_vendor = "apple")]
fn unpack(packed: &[u8], length: usize) -> Option<Vec<u8>> {
    let mut table = vec![0_u8; length];
    // SAFETY: both buffers are live and as long as said; the decoder asks for no scratch buffer
    // when given none.
    let written = unsafe {
        compression::compression_decode_buffer(
            table.as_mut_ptr(),
            table.len(),
            packed.as_ptr(),
            packed.len(),
            std::ptr::null_mut(),
            compression::LZFSE,
        )
    };
    (written == length).then_some(table)
}

#[cfg(not(target_vendor = "apple"))]
fn unpack(_packed: &[u8], _length: usize) -> Option<Vec<u8>> {
    None
}

/// The buffer functions of the system's `libcompression`.
#[cfg(target_vendor = "apple")]
mod compression {
    use std::ffi::c_void;

    /// `COMPRESSION_LZFSE`.
    pub const LZFSE: i32 = 0x801;

    #[link(name = "compression")]
    unsafe extern "C" {
        pub fn compression_decode_buffer(
            destination: *mut u8,
            destination_size: usize,
            source: *const u8,
            source_size: usize,
            scratch: *mut c_void,
            algorithm: i32,
        ) -> usize;

        #[cfg(test)]
        pub fn compression_encode_buffer(
            destination: *mut u8,
            destination_size: usize,
            source: *const u8,
            source_size: usize,
            scratch: *mut c_void,
            algorithm: i32,
        ) -> usize;
    }
}

/// Adds the libraries of `table` to the ones the Metal contexts of this process look in. A
/// program gives the table it was built with before its first window:
/// `register_metal_libraries(include_bytes!("my-app.fmz"))`.
///
/// A table is what `scripts/perf/metal-libraries.py` makes of the shader texts a run of the
/// program wrote with `FERROUI_METAL_LIBRARY_RECORD=<folder>` in its environment: libraries
/// the Metal compiler of Xcode made of them ahead, which the device then loads where it would
/// compile. With it a first start after an install does not wait for the shader compiler. A
/// table belongs to the version of the framework and to the kind of device its texts were
/// written on; a text it does not have is compiled as before, and a platform that has no use
/// for a table (any but Metal) keeps nothing of it.
pub fn register_metal_libraries(table: &'static [u8]) {
    GIVEN.lock().unwrap_or_else(PoisonError::into_inner).push(table);
    UNREAD.store(true, Ordering::Release);
}

/// Whether there is anything to ask or to write: the device keeps its own method otherwise.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
pub(crate) fn wanted() -> bool {
    static FROM_FILE: OnceLock<()> = OnceLock::new();
    FROM_FILE.get_or_init(|| {
        let Some(file) = std::env::var_os("FERROUI_METAL_LIBRARIES").filter(|v| !v.is_empty()) else { return };
        if file == "off" {
            OFF.store(true, Ordering::Relaxed);
            return;
        }
        match std::fs::read(&file) {
            // The table of a file lives as long as the process, as one of the program does.
            Ok(bytes) => register_metal_libraries(Box::leak(bytes.into_boxed_slice())),
            Err(error) => eprintln!("FERROUI_METAL_LIBRARIES: {}: {error}", file.to_string_lossy()),
        }
    });
    if record_folder().is_some() {
        return true;
    }
    !OFF.load(Ordering::Relaxed)
        && (UNREAD.load(Ordering::Acquire) || !libraries().read().unwrap_or_else(PoisonError::into_inner).is_empty())
}

/// The texts that were given a library of the table and the texts the device compiled.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
pub(crate) fn counts() -> (u64, u64) {
    (FOUND.load(Ordering::Relaxed), COMPILED.load(Ordering::Relaxed))
}

/// The keys of the texts that were given a library of the table (tests).
#[cfg(test)]
pub(crate) fn found_keys() -> &'static Mutex<std::collections::HashSet<u64>> {
    static KEYS: OnceLock<Mutex<std::collections::HashSet<u64>>> = OnceLock::new();
    KEYS.get_or_init(Default::default)
}

/// The library of a shader text that is to be compiled with `language_version` and, when
/// `plain_options`, with nothing else asked of the compiler: none when the table has none, or
/// when the text is to be compiled another way than the libraries of a table are. A text is
/// written to the folder of `FERROUI_METAL_LIBRARY_RECORD` on the way.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
pub(crate) fn library_for(text: &[u8], language_version: u64, plain_options: bool) -> Option<&'static [u8]> {
    let key = key(text);
    if let Some(folder) = record_folder() {
        let file = folder.join(format!("{key:016x}.metal"));
        if !file.exists() {
            let _ = std::fs::write(file, text);
        }
    }
    let found = if language_version == LANGUAGE_VERSION && plain_options && !OFF.load(Ordering::Relaxed) {
        read_given();
        let libraries = libraries().read().unwrap_or_else(PoisonError::into_inner);
        match libraries.get(&key) {
            Some((text_length, library)) if *text_length as usize == text.len() => Some(*library),
            _ => None,
        }
    } else {
        None
    };
    match found {
        Some(_) => {
            FOUND.fetch_add(1, Ordering::Relaxed);
            #[cfg(test)]
            found_keys().lock().unwrap_or_else(PoisonError::into_inner).insert(key);
        }
        None => {
            COMPILED.fetch_add(1, Ordering::Relaxed);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(libraries: &[(&[u8], &[u8])]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        for (text, library) in libraries {
            bytes.extend(key(text).to_le_bytes());
            bytes.extend((text.len() as u32).to_le_bytes());
            bytes.extend((library.len() as u32).to_le_bytes());
            bytes.extend(*library);
        }
        bytes
    }

    fn kept(bytes: Vec<u8>) -> &'static [u8] {
        Box::leak(bytes.into_boxed_slice())
    }

    #[test]
    fn the_key_of_a_text_is_fnv_1a() {
        assert_eq!(key(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(key(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(key(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn a_table_gives_its_libraries_and_stops_where_it_is_cut() {
        let whole = kept(table(&[(b"first text", b"FIRST"), (b"second", b"SECOND LIBRARY")]));
        let found = parse(whole);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], (key(b"first text"), (10, &b"FIRST"[..])));
        assert_eq!(found[1], (key(b"second"), (6, &b"SECOND LIBRARY"[..])));

        assert_eq!(parse(&whole[..whole.len() - 1]).len(), 1);
        assert!(parse(b"FPR1").is_empty());
        assert!(parse(b"").is_empty());
        assert_eq!(unpacked(whole), Some(whole));
        assert_eq!(unpacked(b"FPR1"), None);
        assert_eq!(unpacked(b"FMZ1"), None);
    }

    #[test]
    fn a_library_is_given_for_its_text_compiled_the_way_the_table_was() {
        let text = b"a text of this test alone: 7f3a";
        register_metal_libraries(kept(table(&[(text, b"LIBRARY")])));
        let (found, _) = counts();
        assert_eq!(library_for(text, LANGUAGE_VERSION, true), Some(&b"LIBRARY"[..]));
        assert!(counts().0 > found);
        assert!(found_keys().lock().expect("the keys").contains(&key(text)));
        // Another text, another language version, other options: the device compiles.
        assert_eq!(library_for(b"another text of this test: 7f3a", LANGUAGE_VERSION, true), None);
        assert_eq!(library_for(text, LANGUAGE_VERSION + 1, true), None);
        assert_eq!(library_for(text, LANGUAGE_VERSION, false), None);
    }

    /// A packed table is the table: the system unpacks what it packed.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn a_packed_table_gives_the_libraries_of_the_table() {
        let text = b"a text of the packed table of this test: 91c4";
        let plain = table(&[(text, &[7_u8; 4096][..]), (b"its second text", b"SECOND")]);
        let mut stream = vec![0_u8; plain.len() + 4096];
        // SAFETY: both buffers are live and as long as said.
        let written = unsafe {
            compression::compression_encode_buffer(
                stream.as_mut_ptr(),
                stream.len(),
                plain.as_ptr(),
                plain.len(),
                std::ptr::null_mut(),
                compression::LZFSE,
            )
        };
        assert!(written > 0 && written < plain.len());
        let mut packed = PACKED.to_vec();
        packed.extend((plain.len() as u32).to_le_bytes());
        packed.extend(&stream[..written]);
        let packed = kept(packed);

        assert_eq!(unpacked(packed), Some(&plain[..]));
        register_metal_libraries(packed);
        assert_eq!(library_for(text, LANGUAGE_VERSION, true), Some(&[7_u8; 4096][..]));
        // A stream that is cut, and a length that is not the stream's, give nothing.
        assert_eq!(unpacked(&packed[..packed.len() - 8]), None);
        let mut longer = packed.to_vec();
        longer[4..8].copy_from_slice(&(plain.len() as u32 + 1).to_le_bytes());
        assert_eq!(unpacked(kept(longer)), None);
    }

    /// The table the crate carries is a table with libraries in it.
    #[test]
    fn the_table_of_ordinary_drawing_is_a_table() {
        let Some(table) = ordinary_table() else { return };
        let libraries = parse(table);
        assert!(libraries.len() >= 30, "{} libraries", libraries.len());
        // Every library is a Metal library (`MTLB`), made from a text of its own.
        assert!(libraries.iter().all(|(_, (text_length, library))| *text_length > 0 && library.starts_with(b"MTLB")));
        let keys: std::collections::HashSet<u64> = libraries.iter().map(|(key, _)| *key).collect();
        assert_eq!(keys.len(), libraries.len());
    }

    /// The device of this process loads every library of the table the crate carries for its
    /// kind of device: the libraries were made for this platform (the simulator refuses a library
    /// made for a device, and a device one made for the simulator).
    #[cfg(target_vendor = "apple")]
    #[test]
    fn the_device_loads_every_library_of_the_table_of_ordinary_drawing() {
        let Some(table) = ordinary_table() else { return };
        let Some(device) = device::Device::system_default() else {
            eprintln!("skipped: no Metal device");
            return;
        };
        let refused: Vec<String> = parse(table)
            .into_iter()
            .filter(|(_, (_, library))| !device.loads(library))
            .map(|(key, _)| format!("{key:016x}"))
            .collect();
        assert!(refused.is_empty(), "the device refused the libraries of {refused:?}");
    }

    /// The little of Metal the test above needs, through the Objective-C runtime.
    #[cfg(target_vendor = "apple")]
    mod device {
        use std::ffi::{c_char, c_void};

        type Id = *mut c_void;
        type Sel = *mut c_void;

        #[link(name = "Metal", kind = "framework")]
        unsafe extern "C" {
            fn MTLCreateSystemDefaultDevice() -> Id;
        }

        #[link(name = "objc")]
        unsafe extern "C" {
            fn sel_registerName(name: *const c_char) -> Sel;
            fn objc_msgSend();
        }

        // libdispatch, part of the system library.
        unsafe extern "C" {
            fn dispatch_data_create(buffer: *const c_void, size: usize, queue: Id, destructor: Id) -> Id;
            fn dispatch_release(object: Id);
        }

        fn sel(name: &'static [u8]) -> Sel {
            assert!(name.ends_with(b"\0"));
            // SAFETY: `name` is a NUL-terminated string.
            unsafe { sel_registerName(name.as_ptr().cast()) }
        }

        fn release(object: Id) {
            // SAFETY: `object` is an object this module holds one reference to.
            let send: unsafe extern "C" fn(Id, Sel) =
                unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
            unsafe { send(object, sel(b"release\0")) };
        }

        /// The system's default `id<MTLDevice>`, one reference held.
        pub struct Device(Id);

        impl Device {
            pub fn system_default() -> Option<Self> {
                // SAFETY: a plain framework call; nil when there is no device.
                let device = unsafe { MTLCreateSystemDefaultDevice() };
                (!device.is_null()).then_some(Self(device))
            }

            /// Whether `newLibraryWithData:error:` gives a library for `library`.
            pub fn loads(&self, library: &[u8]) -> bool {
                // SAFETY: the buffer is live for the call; with no destructor the data is a copy.
                let (queue, destructor) = (std::ptr::null_mut(), std::ptr::null_mut());
                let data = unsafe { dispatch_data_create(library.as_ptr().cast(), library.len(), queue, destructor) };
                if data.is_null() {
                    return false;
                }
                let mut error: Id = std::ptr::null_mut();
                // SAFETY: `objc_msgSend` through the C signature of the method; the device and the
                // data are live objects.
                let new_library: unsafe extern "C" fn(Id, Sel, Id, *mut Id) -> Id =
                    unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
                let loaded = unsafe { new_library(self.0, sel(b"newLibraryWithData:error:\0"), data, &mut error) };
                // SAFETY: the data was made above; the library is returned with one reference.
                unsafe { dispatch_release(data) };
                if loaded.is_null() {
                    return false;
                }
                release(loaded);
                true
            }
        }

        impl Drop for Device {
            fn drop(&mut self) {
                release(self.0);
            }
        }
    }
}
