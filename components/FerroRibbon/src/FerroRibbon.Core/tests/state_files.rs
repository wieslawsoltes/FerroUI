//! Call-time persistence contracts from unchanged Core, plus Rust ownership paths.
use ferroribbon_core::state::*;
use ferroui_system::threading::CancellationToken;
use futures_executor::block_on;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

struct Directory(PathBuf);
impl Directory {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "state-futures-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/state-file-calls.json"
    ))
    .unwrap()
}
fn reference_error(error: &RibbonStateError) -> &'static str {
    match error.kind {
        RibbonStateErrorKind::ArgumentNull => "ArgumentNullException",
        RibbonStateErrorKind::Argument => "ArgumentException",
        RibbonStateErrorKind::Canceled => "TaskCanceledException",
        RibbonStateErrorKind::Io => "IOException",
        RibbonStateErrorKind::Unauthorized => "UnauthorizedAccessException",
    }
}

#[test]
fn source_call_time_snapshot_precedence_and_abandoned_save_match_reference() {
    for row in fixture()["cases"].as_array().unwrap() {
        let mode = row["mode"].as_str().unwrap();
        let directory = Directory::new(mode);
        let mut path = directory.0.join("created/state.json");
        if mode == "trailing-separator" {
            path.as_mut_os_string().push("/");
        }
        let mut temp = path.as_os_str().to_os_string();
        temp.push(".tmp");
        let temp = PathBuf::from(temp);
        if mode == "temp-directory" {
            std::fs::create_dir_all(&temp).unwrap();
        }
        let cancel = CancellationToken::new();
        if mode.starts_with("canceled") {
            cancel.cancel();
        }
        let state = RibbonState::new();
        state.set_selected_tab_id(Some("before".into()));
        let input = if mode.contains("null-state") {
            None
        } else {
            Some(&*state)
        };
        let future = RibbonStateSerializer::save_async(input, Some(&path), cancel);
        assert_eq!(
            json!(path.parent().unwrap().is_dir()),
            row["directoryAtReturn"],
            "{mode}"
        );
        // The individual temp/target observation can race a valid completed rename.
        assert_eq!(
            json!(temp.is_file() || path.is_file()),
            row["fileStartedAtReturn"],
            "{mode}"
        );
        state.set_selected_tab_id(Some("after".into()));
        let error = if mode == "abandoned-save" {
            drop(future);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !path.is_file() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                path.is_file(),
                "unobserved save must still replace the target"
            );
            None
        } else {
            block_on(future).err().map(|e| reference_error(&e))
        };
        let loaded = block_on(RibbonStateSerializer::load_async_default(Some(&path))).unwrap();
        assert_eq!(json!(error), row["error"], "{mode}");
        assert_eq!(json!(path.is_file()), row["target"], "{mode}");
        assert_eq!(json!(temp.is_file()), row["temp"], "{mode}");
        assert_eq!(json!(path.is_dir()), row["directoryTarget"], "{mode}");
        assert_eq!(
            json!(
                loaded
                    .and_then(|s| s.selected_tab_id())
                    .map(|s| s.to_utf8().unwrap())
            ),
            row["loaded"],
            "{mode}"
        );
    }
}

#[test]
fn a_started_read_keeps_its_open_file_after_pathname_replacement() {
    let directory = Directory::new("opened-read");
    let path = directory.0.join("state.json");
    std::fs::write(&path, "{\"selectedTabId\":\"before\"}").unwrap();
    let future = RibbonStateSerializer::load_async_default(Some(&path));
    #[cfg(unix)]
    {
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "{\"selectedTabId\":\"after\"}").unwrap();
    }
    let value = block_on(future)
        .unwrap()
        .unwrap()
        .selected_tab_id()
        .unwrap();
    assert_eq!(
        json!(value.to_utf8().unwrap()),
        fixture()["readOpenedAtCall"]
    );
}

#[test]
#[cfg(unix)]
fn read_sharing_rejects_exclusive_locks_and_never_truncates_a_locked_temp() {
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
    };
    let directory = Directory::new("sharing");
    let path = directory.0.join("state.json");
    std::fs::write(&path, "{}").unwrap();
    let file = File::options().read(true).write(true).open(&path).unwrap();
    file.lock().unwrap();
    let loaded = block_on(RibbonStateSerializer::load_async_default(Some(&path))).unwrap();
    assert_eq!(json!(loaded.is_none()), fixture()["lockedReadReturnsNull"]);
    drop(file);
    let temp = directory.0.join("state.json.tmp");
    std::fs::write(&temp, "preserve").unwrap();
    let mut file = File::options().read(true).write(true).open(&temp).unwrap();
    file.lock().unwrap();
    let error = block_on(RibbonStateSerializer::save_async_default(
        Some(&RibbonState::new()),
        Some(&path),
    ))
    .unwrap_err();
    assert_eq!(json!(reference_error(&error)), fixture()["lockedSaveError"]);
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    assert_eq!(json!(text), fixture()["lockedTempContents"]);
}

#[test]
fn pending_save_does_not_retain_the_model_or_caller_path() {
    let directory = Directory::new("owned-bytes");
    let path = directory.0.join("state.json");
    let state = RibbonState::new();
    let weak = Rc::downgrade(&state);
    state.set_selected_tab_id(Some("owned".into()));
    let future = RibbonStateSerializer::save_async_default(Some(&state), Some(&path));
    fn assert_send<T: Send>(_: &T) {}
    assert_send(&future);
    assert_send(&RibbonStateSerializer::load_async_default(Some(&path)));
    drop(state);
    assert!(weak.upgrade().is_none());
    drop(path);
    block_on(future).unwrap();
    let loaded = block_on(RibbonStateSerializer::load_async_default(Some(
        &directory.0.join("state.json"),
    )))
    .unwrap()
    .unwrap();
    assert_eq!(
        loaded.selected_tab_id().unwrap().to_utf8().unwrap(),
        "owned"
    );
}

#[test]
#[cfg(unix)]
fn native_non_utf8_paths_preserve_platform_errors_or_round_trip_without_replacement() {
    use std::os::unix::ffi::OsStringExt;
    let directory = Directory::new("native-path");
    let path = directory
        .0
        .join(std::ffi::OsString::from_vec(b"state-\xff.json".to_vec()));
    let state = RibbonState::new();
    state.set_selected_tab_id(Some("native".into()));
    if let Err(native_error) = std::fs::write(&path, "{}").map_err(|error| error.to_string()) {
        let error = block_on(RibbonStateSerializer::save_async_default(
            Some(&state),
            Some(&path),
        ))
        .unwrap_err();
        assert_eq!(error.message, native_error);
        assert!(!directory.0.join("state-�.json").exists());
        return;
    }
    block_on(RibbonStateSerializer::save_async_default(
        Some(&state),
        Some(&path),
    ))
    .unwrap();
    let loaded = block_on(RibbonStateSerializer::load_async_default(Some(&path)))
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.selected_tab_id().unwrap().to_utf8().unwrap(),
        "native"
    );
}
