//! The small prefix of WebScene.NativeEngine ABI 3 used in the worker. The
//! `struct_size` negotiation leaves all later native options disabled.
use libloading::Library;
use std::{
    ffi::{c_char, c_void},
    path::Path,
    ptr,
    sync::mpsc,
};
type Raw = *mut c_void;
pub type Loader = unsafe extern "C" fn(
    Raw,
    u32,
    *const c_char,
    usize,
    *const c_char,
    usize,
    i64,
    *mut c_char,
    usize,
) -> usize;
#[repr(C)]
struct Options {
    size: u32,
    chart: u32,
    cache: *const c_char,
    cache_len: usize,
    load: Loader,
    user: Raw,
}
#[repr(C)]
struct Request {
    size: u32,
    version: u32,
    source: *const c_char,
    length: usize,
    name: *const c_char,
    name_len: usize,
    flags: u32,
    reserved: u32,
}
#[repr(C)]
struct Scalar {
    kind: u32,
    flags: u32,
    offset: u32,
    length: u32,
    payload: u64,
}
#[repr(C)]
struct ResultView {
    size: u32,
    version: u32,
    status: u32,
    flags: u32,
    id: u64,
    values: *const Scalar,
    edges: *const c_void,
    utf8: *const u8,
    error: *const u8,
    lease: u64,
    value_count: u32,
    edge_count: u32,
    utf8_count: u32,
    error_count: u32,
    root: u32,
    reserved: [u32; 3],
}

type Begin = unsafe extern "C" fn(Raw, *const Request, extern "C" fn(Raw, u64), Raw) -> u64;
type Take = unsafe extern "C" fn(Raw, u64) -> *const ResultView;
type Release = unsafe extern "C" fn(*const ResultView, u64);
pub struct Engine {
    raw: Raw,
    begin: Begin,
    take: Take,
    release: Release,
    destroy: unsafe extern "C" fn(Raw),
    _library: Library,
}

impl Engine {
    /// The caller keeps `user` alive until after this engine is dropped. The
    /// engine worker only invokes `load` while the library is resident.
    pub unsafe fn open(path: &Path, load: Loader, user: Raw) -> Result<Self, String> {
        let library = unsafe { Library::new(path) }.map_err(|e| e.to_string())?;
        unsafe fn symbol<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, String> {
            unsafe { lib.get::<T>(name) }
                .map(|s| *s)
                .map_err(|e| e.to_string())
        }
        unsafe {
            let version: unsafe extern "C" fn() -> u32 =
                symbol(&library, b"webscene_engine_get_abi_version")?;
            if version() != 3 {
                return Err("expected WebScene native engine ABI 3".into());
            }
            let create: unsafe extern "C" fn(*const Options) -> Raw =
                symbol(&library, b"webscene_engine_create_with_options")?;
            let navigate: unsafe extern "C" fn(Raw, *const c_char, usize) -> u8 =
                symbol(&library, b"webscene_engine_load_url")?;
            let begin = symbol(&library, b"webscene_engine_begin_evaluate_v3")?;
            let take = symbol(&library, b"webscene_engine_take_invoke_result_v3")?;
            let release = symbol(&library, b"webscene_interop_result_release_v3")?;
            let destroy = symbol(&library, b"webscene_engine_destroy")?;
            let raw = create(&Options {
                size: size_of::<Options>() as u32,
                chart: 0,
                cache: ptr::null(),
                cache_len: 0,
                load,
                user,
            });
            if raw.is_null() {
                return Err("WebScene engine initialization failed".into());
            }
            let engine = Self {
                raw,
                begin,
                take,
                release,
                destroy,
                _library: library,
            };
            let document = b"https://qml.invalid/index.html";
            if navigate(raw, document.as_ptr().cast(), document.len()) == 0 {
                return Err("WebScene navigation failed".into());
            }
            Ok(engine)
        }
    }

    /// Wait only inside the disposable child process. The parent independently
    /// enforces a monotonic deadline, including JS running an infinite loop.
    pub fn eval(&self, source: &str) -> Result<String, String> {
        extern "C" fn complete(user: Raw, id: u64) {
            // The receiver stays in this stack frame until completion fires.
            // Copy the sender before signaling: the receiver can wake immediately.
            let sender = unsafe { &*user.cast::<mpsc::Sender<u64>>() }.clone();
            let _ = sender.send(id);
        }
        let (sender, receiver) = mpsc::channel::<u64>();
        let name = b"https://qml.invalid/runtime.js";
        let request = Request {
            size: size_of::<Request>() as u32,
            version: 3,
            source: source.as_ptr().cast(),
            length: source.len(),
            name: name.as_ptr().cast(),
            name_len: name.len(),
            flags: 0,
            reserved: 0,
        };
        let id = unsafe {
            (self.begin)(
                self.raw,
                &request,
                complete,
                ptr::from_ref(&sender).cast_mut().cast(),
            )
        };
        if id == 0 {
            return Err("WebScene refused evaluation".into());
        }
        if receiver.recv().map_err(|e| e.to_string())? != id {
            return Err("WebScene completion id mismatch".into());
        }
        let view = unsafe { (self.take)(self.raw, id) };
        if view.is_null() {
            return Err("WebScene returned no result".into());
        }
        // All pointers belong to this lease until release, including error paths.
        let result = unsafe { Self::copy_result(&*view) };
        unsafe { (self.release)(view, (*view).lease) };
        result
    }

    unsafe fn copy_result(view: &ResultView) -> Result<String, String> {
        unsafe fn bytes(_lease: &ResultView, ptr: *const u8, len: usize) -> Result<&[u8], String> {
            if len == 0 {
                return Ok(&[]);
            }
            if ptr.is_null() || len > crate::framing::LIMIT {
                return Err("invalid WebScene byte range".into());
            }
            Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
        }
        if view.version != 3 || view.size < (size_of::<ResultView>() as u32) {
            return Err("incompatible WebScene result layout".into());
        }
        if view.status != 0 {
            return Err(String::from_utf8_lossy(unsafe {
                bytes(view, view.error, view.error_count as usize)
            }?)
            .into_owned());
        }
        if view.root >= view.value_count || view.values.is_null() {
            return Err("invalid root value".into());
        }
        let value = unsafe { &*view.values.add(view.root as usize) };
        if value.kind != 4 {
            return Err("WebScene bridge must return a string".into());
        }
        let all = unsafe { bytes(view, view.utf8, view.utf8_count as usize) }?;
        let start = value.offset as usize;
        let end = start
            .checked_add(value.length as usize)
            .ok_or("WebScene result overflow")?;
        String::from_utf8(
            all.get(start..end)
                .ok_or("invalid WebScene string range")?
                .to_vec(),
        )
        .map_err(|e| e.to_string())
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.raw) }
    }
}
