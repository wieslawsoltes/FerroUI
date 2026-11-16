//! The pipelines Graphite makes, written down, and pipelines made ahead from what was written
//! down (docs/porting/gpu-rendering.md, "Pipelines made ahead of their first draw").
//!
//! Graphite reports every pipeline it adds to its cache with a key that can be stored, and makes
//! a pipeline from such a key before a draw needs it. A *record* is the keys of the pipelines an
//! application made: recorded on one run, shipped with the application, and made ahead on the
//! next start, on the threads of the pipeline pool, while the application is still starting. A
//! key belongs to the version of Skia and to the kind of device it was recorded on; one that does
//! not fit is refused by Skia and costs nothing.
//!
//! The format of a record: the magic `FPR1`, then for each key its length as a little-endian
//! `u32` and its bytes.
//!
//! - `FERROUI_PIPELINE_RECORD=<file>`: the keys of the pipelines the process makes are appended
//!   to the file as they are made (a record), and a line a pipeline goes to `<file>.txt`: the
//!   milliseconds since the first, whether it came from a record, and Skia's label of it.

use skia_safe::gpu::graphite;
use skia_safe::Data;
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

/// The magic of a record.
const MAGIC: &[u8; 4] = b"FPR1";

/// Parts of Skia's label of a pipeline that keep its key out of a record: pipelines the pinned
/// Skia cannot make from a key. A pipeline that samples an image through the snippet that
/// normalises its coordinates (`CoordNormalize`, every draw of a texture) has a shader node with
/// no function of its own, which the path from a key calls (`invoke_node`, a null name): the
/// process ends there. Such pipelines are made when they are first drawn, on the pipeline pool.
const NOT_FROM_A_KEY: [&str; 1] = ["CoordNormalize"];

/// Whether the key of the pipeline Skia labels `label` may go into a record.
fn may_be_recorded(label: &str) -> bool {
    !NOT_FROM_A_KEY.iter().any(|part| label.contains(part))
}

unsafe extern "C" {
    fn ferroui_skia_pipeline_callback();
    fn ferroui_skia_precompile_context_new(context: *mut c_void) -> *mut c_void;
    fn ferroui_skia_precompile(precompile_context: *mut c_void, key: *mut c_void) -> bool;
    fn ferroui_skia_precompile_context_delete(precompile_context: *mut c_void);
}

/// The function Graphite is given as `ContextOptions::fPipelineCachingCallback`, or null when
/// nothing listens: the pipelines are reported only while something records them.
pub(crate) fn caching_callback() -> *const c_void {
    if recorder().is_some() {
        ferroui_skia_pipeline_callback as *const c_void
    } else {
        std::ptr::null()
    }
}

struct Recorder {
    keys: std::fs::File,
    labels: std::fs::File,
    started: Instant,
    seen: std::collections::HashSet<Vec<u8>>,
}

fn recorder() -> Option<&'static Mutex<Recorder>> {
    static RECORDER: OnceLock<Option<Mutex<Recorder>>> = OnceLock::new();
    RECORDER
        .get_or_init(|| {
            let path = std::env::var_os("FERROUI_PIPELINE_RECORD").filter(|path| !path.is_empty())?;
            let mut keys = std::fs::File::create(&path).ok()?;
            keys.write_all(MAGIC).ok()?;
            let mut labels = path.clone();
            labels.push(".txt");
            let labels = std::fs::File::create(labels).ok()?;
            Some(Mutex::new(Recorder { keys, labels, started: Instant::now(), seen: Default::default() }))
        })
        .as_ref()
}

/// Gives one reference to a `SkData` back (`pipeline_record.cpp`: the smart pointer of the file
/// that declares Skia's has no access to Skia's own release).
#[unsafe(no_mangle)]
extern "C" fn ferroui_skia_data_unref(data: *mut c_void) {
    if !data.is_null() {
        // SAFETY: a handle of the binding is a non-null pointer to the object and owns one
        // reference, which dropping it gives back.
        drop(unsafe { std::mem::transmute::<*mut c_void, Data>(data) });
    }
}

/// Called by Graphite, on whatever thread makes or finds a pipeline (`pipeline_record.cpp`).
/// `data` is the key, Skia's for the time of the call, or null.
#[unsafe(no_mangle)]
extern "C" fn ferroui_skia_pipeline_seen(
    op: c_int,
    label: *const c_char,
    label_length: usize,
    _hash: u32,
    from_precompile: bool,
    data: *mut c_void,
) {
    // SAFETY: a handle of the binding is a non-null pointer to the object. The reference is
    // Skia's, so the handle is never dropped.
    let key: Option<std::mem::ManuallyDrop<Data>> = (!data.is_null())
        .then(|| std::mem::ManuallyDrop::new(unsafe { std::mem::transmute::<*mut c_void, Data>(data) }));
    // 0 is `PipelineCacheOp::kAddingPipeline`: the pipeline was made now.
    let (Some(recorder), 0) = (recorder(), op) else { return };
    let mut recorder = recorder.lock().unwrap_or_else(PoisonError::into_inner);
    // SAFETY: the label is a string of Skia that lives for the call.
    let label = unsafe { std::slice::from_raw_parts(label.cast::<u8>(), label_length) };
    let label = String::from_utf8_lossy(label);
    let ms = recorder.started.elapsed().as_secs_f64() * 1000.0;
    let origin = if from_precompile { "record" } else { "draw" };
    let stored = match &key {
        Some(_) if !may_be_recorded(&label) => "left out",
        Some(key) if recorder.seen.insert(key.as_bytes().to_vec()) => {
            let bytes = key.as_bytes();
            let _ = recorder.keys.write_all(&(bytes.len() as u32).to_le_bytes());
            let _ = recorder.keys.write_all(bytes);
            "key"
        }
        Some(_) => "key seen",
        None => "no key",
    };
    let _ = writeln!(recorder.labels, "{ms:9.1} ms  {origin:6}  {stored:8}  {label}");
}

/// The keys of a record, or `None` when the bytes are not one.
pub(crate) fn keys(record: &[u8]) -> Option<Vec<&[u8]>> {
    let mut rest = record.strip_prefix(MAGIC)?;
    let mut keys = Vec::new();
    while !rest.is_empty() {
        let (length, after) = rest.split_first_chunk::<4>()?;
        let length = u32::from_le_bytes(*length) as usize;
        if length > after.len() {
            return None;
        }
        let (key, after) = after.split_at(length);
        keys.push(key);
        rest = after;
    }
    Some(keys)
}

/// A precompile context on its way to the thread that uses it.
struct Precompile(*mut c_void);

// SAFETY: a precompile context of Graphite is made to be used on another thread than the context
// it came from; this one is used by one thread and deleted there.
unsafe impl Send for Precompile {}

/// Asks Graphite for the pipelines of `records` on a thread of its own, which hands each key to
/// the pipeline pool of the context and ends. Returns the thread, which answers how many keys
/// Skia took; `None` when there is nothing to do or the thread could not be started.
pub(crate) fn precompile(
    context: &mut graphite::Context,
    records: Vec<std::borrow::Cow<'static, [u8]>>,
) -> Option<std::thread::JoinHandle<usize>> {
    if records.iter().all(|record| keys(record).is_none_or(|keys| keys.is_empty())) {
        return None;
    }
    // SAFETY: a handle of the binding is a non-null pointer to the native context.
    let native = unsafe { std::mem::transmute_copy::<graphite::Context, *mut c_void>(context) };
    // SAFETY: `native` is a live Graphite context, used by this thread now.
    let precompile = Precompile(unsafe { ferroui_skia_precompile_context_new(native) });
    if precompile.0.is_null() {
        return None;
    }
    std::thread::Builder::new()
        .name("ferroui-graphite-precompile".to_owned())
        .spawn(move || {
            let precompile = precompile;
            let mut taken = 0;
            for record in &records {
                for key in keys(record).unwrap_or_default() {
                    let key = Data::new_copy(key);
                    // SAFETY: the handle is a pointer that owns one reference, which the call
                    // takes; the precompile context is this thread's.
                    let key = unsafe { std::mem::transmute::<Data, *mut c_void>(key) };
                    if unsafe { ferroui_skia_precompile(precompile.0, key) } {
                        taken += 1;
                    }
                }
            }
            // SAFETY: made above, not used after this.
            unsafe { ferroui_skia_precompile_context_delete(precompile.0) };
            taken
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_is_its_magic_and_its_keys_with_their_lengths() {
        let mut record = MAGIC.to_vec();
        for key in [&b"one"[..], &b""[..], &b"three"[..]] {
            record.extend((key.len() as u32).to_le_bytes());
            record.extend(key);
        }
        assert_eq!(keys(&record), Some(vec![&b"one"[..], &b""[..], &b"three"[..]]));
        assert_eq!(keys(MAGIC), Some(Vec::new()));
        assert_eq!(keys(b"FPR2"), None, "another magic");
        assert_eq!(keys(&record[..record.len() - 1]), None, "a key cut short");
        assert_eq!(keys(&record[..6]), None, "a length cut short");
    }

    #[test]
    fn a_pipeline_skia_cannot_make_from_its_key_stays_out_of_a_record() {
        let pass = "RP((RGBA8+D32F_S8 x4->1).rgba) + ";
        assert!(may_be_recorded(&format!("{pass}AnalyticRRectRenderStep + SolidColor SrcOver")));
        assert!(may_be_recorded(&format!("{pass}CoverBoundsRenderStep[NonAAFill] + $1DBlur4[ImageShaderClamp(0)] Src")));
        let image = "LocalMatrix[CoordNormalize[HardwareImage(0)]+PreAlpha] SrcOver";
        assert!(!may_be_recorded(&format!("{pass}CoverBoundsRenderStep[NonAAFill] + {image}")));
    }
}
