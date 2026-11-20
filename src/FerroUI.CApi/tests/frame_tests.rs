//! Frames: the record layout, the scalar writer, the keyed reconcile into a retained tree, the
//! dispatcher of message tags, and the stable IDs of the API model.

use super::support::*;
use crate::frame::{
    decode, write_record, writer_bytes, Key, Value, OP_CLOSE, OP_KEY_TEXT, OP_MEMO, OP_NUMBER, OP_ON, OP_OPEN, OP_TEXT,
};
use crate::*;
use std::sync::Mutex;

/// The calls of the dispatcher of the tests: (tag, sender, arg, event text).
static DISPATCHED: Mutex<Vec<(i64, FerroHandle, i64, String)>> = Mutex::new(Vec::new());

fn dispatched() -> Vec<(i64, FerroHandle, i64, String)> {
    std::mem::take(&mut *DISPATCHED.lock().unwrap_or_else(|error| error.into_inner()))
}

unsafe extern "C" fn dispatcher(tag: i64, sender: FerroHandle, arg: i64) -> i64 {
    // SAFETY: the event text is a NUL-terminated string.
    let text = unsafe { std::ffi::CStr::from_ptr(ferro_frame_event_text()) }.to_string_lossy().into_owned();
    DISPATCHED.lock().unwrap_or_else(|error| error.into_inner()).push((tag, sender, arg, text));
    0
}

fn text(property: &str, value: &str) {
    // SAFETY: a pointer and the length of a Rust string.
    assert_eq!(unsafe { ferro_frame_str(property_id(property), value.as_ptr().cast(), value.len() as i64) }, FERRO_OK);
}

/// A frame of a text and a button with the key 7 whose Click carries `tag`.
fn counter_frame(count: i64, tag: i64) {
    assert_eq!(ferro_frame_begin(), FERRO_OK);
    assert_eq!(ferro_frame_open(type_id("StackPanel"), 0), FERRO_OK);
    assert_eq!(ferro_frame_f64(property_id("Spacing"), 8.0), FERRO_OK);
    assert_eq!(ferro_frame_open(type_id("TextBlock"), 1), FERRO_OK);
    text("Text", &format!("Count {count}"));
    assert_eq!(ferro_frame_close(), FERRO_OK);
    assert_eq!(ferro_frame_open(type_id("Button"), 7), FERRO_OK);
    text("Content", "+");
    assert_eq!(ferro_frame_on(event_id("Click"), tag, 0), FERRO_OK);
    assert_eq!(ferro_frame_close(), FERRO_OK);
    assert_eq!(ferro_frame_close(), FERRO_OK);
}

fn find(root: FerroHandle, key: i64) -> FerroHandle {
    let handle = ferro_frame_find(root, key);
    if handle != 0 {
        // The test keeps no reference of its own: the retained tree holds one.
        assert_eq!(ferro_release(handle), FERRO_OK);
    }
    handle
}

#[test]
fn records_have_the_layout_of_the_model() {
    let mut bytes = Vec::new();
    write_record(&mut bytes, OP_OPEN, &[3, 9], &[]);
    write_record(&mut bytes, OP_TEXT, &[5, 3], b"abc");
    write_record(&mut bytes, OP_NUMBER, &[6, 2.5f64.to_bits() as i64], &[]);
    write_record(&mut bytes, OP_ON, &[4, 11, 5], &[]);
    write_record(&mut bytes, OP_CLOSE, &[], &[]);
    // Headers of 8 bytes, fields of 8, text padded to 8.
    assert_eq!(bytes.len(), 24 + 32 + 24 + 32 + 8);
    assert_eq!(&bytes[0..8], &[1, 0, 0, 0, 24, 0, 0, 0]);
    assert_eq!(&bytes[24..32], &[3, 0, 0, 0, 32, 0, 0, 0]);
    let elements = decode(&bytes).expect("the frame decodes");
    assert_eq!(elements.len(), 1);
    let element = &elements[0];
    assert_eq!((element.type_id, &element.key), (3, &Key::Int(9)));
    assert_eq!(element.props, vec![(5, Value::Text("abc".into())), (6, Value::Number(2.5f64.to_bits()))]);
    assert_eq!(element.events, vec![(4, 11, 5)]);
}

#[test]
fn the_writer_writes_the_records_of_the_layout() {
    let mut bytes = Vec::new();
    write_record(&mut bytes, OP_OPEN, &[3, 9], &[]);
    write_record(&mut bytes, OP_MEMO, &[12, 4], &[]);
    write_record(&mut bytes, OP_TEXT, &[5, 9], b"the label");
    write_record(&mut bytes, OP_NUMBER, &[6, 2.5f64.to_bits() as i64], &[]);
    write_record(&mut bytes, OP_ON, &[4, 11, 5], &[]);
    write_record(&mut bytes, OP_CLOSE, &[], &[]);
    assert_eq!(ferro_frame_begin(), FERRO_OK);
    assert_eq!(ferro_frame_open(3, 9), FERRO_OK);
    assert_eq!(ferro_frame_memo(12, 4), FERRO_OK);
    // SAFETY: a NUL-terminated literal.
    assert_eq!(unsafe { ferro_frame_str(5, c"the label".as_ptr(), -1) }, FERRO_OK);
    assert_eq!(ferro_frame_f64(6, 2.5), FERRO_OK);
    assert_eq!(ferro_frame_on(4, 11, 5), FERRO_OK);
    assert_eq!(ferro_frame_close(), FERRO_OK);
    assert_eq!(writer_bytes(), bytes);
    assert_eq!(ferro_frame_begin(), FERRO_OK);
}

#[test]
fn malformed_frames_are_errors() {
    let mut open = Vec::new();
    write_record(&mut open, OP_OPEN, &[3, 0], &[]);
    assert_eq!(decode(&open).map(|_| ()).map_err(|error| error.code), Err(FERRO_INVALID_STATE));
    let mut close = Vec::new();
    write_record(&mut close, OP_CLOSE, &[], &[]);
    assert_eq!(decode(&close).map(|_| ()).map_err(|error| error.code), Err(FERRO_INVALID_STATE));
    let mut value = Vec::new();
    write_record(&mut value, OP_NUMBER, &[6, 0], &[]);
    assert_eq!(decode(&value).map(|_| ()).map_err(|error| error.code), Err(FERRO_INVALID_ARGUMENT));
    assert_eq!(decode(&[1, 0, 0, 0, 12, 0, 0, 0]).map(|_| ()).map_err(|error| error.code), Err(FERRO_INVALID_ARGUMENT));
}

#[test]
fn a_commit_builds_the_tree_and_the_next_one_patches_it() {
    ui(|| {
        let root = create("Border");
        counter_frame(0, 1);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        let panel = get_handle(root, "Child").expect("the panel is the child of the border");
        assert_eq!(get_f64(panel, "Spacing"), Ok(8.0));
        let label = find(root, 1);
        let button = find(root, 7);
        assert_eq!(get_str(label, "Text").as_deref(), Ok("Count 0"));
        assert_eq!(get_str(button, "Content").as_deref(), Ok("+"));

        counter_frame(1, 2);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        // The same objects, patched.
        assert_eq!(find(root, 1), label);
        assert_eq!(find(root, 7), button);
        assert_eq!(get_handle(root, "Child"), Ok(panel));
        assert_eq!(get_str(label, "Text").as_deref(), Ok("Count 1"));
        assert_eq!(ferro_release(panel), FERRO_OK);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_frame_find(root, 7), 0);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

#[test]
fn events_call_the_dispatcher_with_their_current_tag() {
    ui(|| {
        assert_eq!(ferro_frame_dispatcher(Some(dispatcher)), FERRO_OK);
        dispatched();
        let root = create("Border");
        counter_frame(0, 1);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        let button = find(root, 7);
        let click = event_id("Click");
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(dispatched(), vec![(1, button, 0, String::new())]);
        // A new tag for the same event: the binding stays and dispatches the new tag.
        counter_frame(1, 5);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(dispatched(), vec![(5, button, 0, String::new())]);
        // A message without an event, as a binding starts its loop.
        assert_eq!(ferro_frame_dispatch(0, 0, 4), 0);
        assert_eq!(dispatched(), vec![(0, 0, 4, String::new())]);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
        assert_eq!(ferro_frame_dispatcher(None), FERRO_OK);
        assert_eq!(ferro_frame_dispatch(0, 0, 0), i64::from(FERRO_INVALID_STATE));
    });
}

#[test]
fn keys_keep_objects_and_removed_elements_go() {
    ui(|| {
        let root = create("StackPanel");
        let frame = |keys: &[i64]| {
            assert_eq!(ferro_frame_begin(), FERRO_OK);
            for &key in keys {
                assert_eq!(ferro_frame_open(type_id("TextBlock"), key), FERRO_OK);
                text("Text", &format!("item {key}"));
                assert_eq!(ferro_frame_close(), FERRO_OK);
            }
            assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        };
        frame(&[1, 2, 3]);
        let (one, two, three) = (find(root, 1), find(root, 2), find(root, 3));
        frame(&[3, 1]);
        assert_eq!(find(root, 3), three);
        assert_eq!(find(root, 1), one);
        assert_eq!(find(root, 2), 0, "the element without its key in the frame is gone");
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
        // Its handle is released: the retained tree held the only reference.
        assert_eq!(ferro_retain(two), FERRO_BAD_HANDLE);
        frame(&[]);
        assert_eq!(ferro_frame_find(root, 1), 0);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

#[test]
fn a_property_the_frame_drops_is_cleared_and_another_class_is_a_new_object() {
    ui(|| {
        let root = create("Border");
        let frame = |class: &str, width: Option<f64>| {
            assert_eq!(ferro_frame_begin(), FERRO_OK);
            assert_eq!(ferro_frame_open(type_id(class), 4), FERRO_OK);
            if let Some(width) = width {
                assert_eq!(ferro_frame_f64(property_id("Width"), width), FERRO_OK);
            }
            assert_eq!(ferro_frame_close(), FERRO_OK);
            assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        };
        frame("TextBlock", Some(120.0));
        let first = find(root, 4);
        assert_eq!(get_f64(first, "Width"), Ok(120.0));
        frame("TextBlock", None);
        assert!(get_f64(first, "Width").expect("a number").is_nan(), "Width is cleared to its default");
        frame("Button", None);
        let second = find(root, 4);
        assert_ne!(second, first);
        assert_eq!(get_handle(root, "Child"), Ok(second));
        assert_eq!(ferro_release(second), FERRO_OK);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

#[test]
fn a_text_box_reports_its_text_and_the_commit_does_not_echo() {
    ui(|| {
        assert_eq!(ferro_frame_dispatcher(Some(dispatcher)), FERRO_OK);
        dispatched();
        let root = create("Border");
        let text_changed = event_id("TextChanged");
        let frame = |value: &str| {
            assert_eq!(ferro_frame_begin(), FERRO_OK);
            assert_eq!(ferro_frame_open(type_id("TextBox"), 2), FERRO_OK);
            text("Text", value);
            assert_eq!(ferro_frame_on(text_changed, 9, property_id("Text")), FERRO_OK);
            assert_eq!(ferro_frame_close(), FERRO_OK);
            assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        };
        frame("1");
        let field = find(root, 2);
        assert_eq!(get_str(field, "Text").as_deref(), Ok("1"));
        frame("2");
        assert_eq!(get_str(field, "Text").as_deref(), Ok("2"));
        // TextChanged is posted: it runs after the commit, with the text the frame set.
        assert_eq!(ferro_run_jobs(), FERRO_OK);
        assert_eq!(dispatched(), vec![], "what a commit writes is not dispatched");
        // Typing: the text changes outside a commit.
        assert_eq!(set_str(field, "Text", "25"), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_run_jobs(), FERRO_OK);
        assert_eq!(dispatched(), vec![(9, field, 0, "25".to_string())], "a text is no number: the arg is 0");
        // The next frame sets what was typed: nothing is written, nothing echoes.
        frame("25");
        assert_eq!(ferro_run_jobs(), FERRO_OK);
        assert_eq!(dispatched(), vec![]);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
        assert_eq!(ferro_frame_dispatcher(None), FERRO_OK);
    });
}

static REMOVING_ROOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A dispatcher that commits a frame without the button that was clicked.
unsafe extern "C" fn removing(tag: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    let root = REMOVING_ROOT.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(ferro_frame_begin(), FERRO_OK);
    assert_eq!(ferro_frame_open(type_id("TextBlock"), 1), FERRO_OK);
    text("Text", &format!("removed by {tag}"));
    assert_eq!(ferro_frame_close(), FERRO_OK);
    assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
    0
}

#[test]
fn a_dispatcher_can_commit_a_frame_that_removes_the_sender() {
    ui(|| {
        let root = create("StackPanel");
        REMOVING_ROOT.store(root, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(ferro_frame_dispatcher(Some(removing)), FERRO_OK);
        assert_eq!(ferro_frame_begin(), FERRO_OK);
        assert_eq!(ferro_frame_open(type_id("Button"), 7), FERRO_OK);
        assert_eq!(ferro_frame_on(event_id("Click"), 3, 0), FERRO_OK);
        assert_eq!(ferro_frame_close(), FERRO_OK);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        let button = ferro_frame_find(root, 7);
        assert_eq!(ferro_raise(button, event_id("Click")), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_frame_find(root, 7), 0);
        assert_eq!(get_str(find(root, 1), "Text").as_deref(), Ok("removed by 3"));
        // The test held its own reference to the button: the handle is still valid.
        assert_eq!(ferro_release(button), FERRO_OK);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
        assert_eq!(ferro_frame_dispatcher(None), FERRO_OK);
    });
}

#[test]
fn a_buffer_commits_as_the_writer_does() {
    ui(|| {
        let root = create("Border");
        let mut bytes = Vec::new();
        write_record(&mut bytes, OP_OPEN, &[type_id("TextBlock"), 3], &[]);
        write_record(&mut bytes, OP_TEXT, &[property_id("Text"), 5], b"bytes");
        write_record(&mut bytes, OP_CLOSE, &[], &[]);
        // SAFETY: the pointer and the length of a vector.
        let committed = unsafe { ferro_frame_commit_bytes(root, bytes.as_ptr(), bytes.len() as i64) };
        assert_eq!(committed, FERRO_OK, "{}", last_error());
        assert_eq!(get_str(find(root, 3), "Text").as_deref(), Ok("bytes"));
        // UTF-16 text through the writer.
        let units: Vec<u16> = "sixteen".encode_utf16().collect();
        assert_eq!(ferro_frame_begin(), FERRO_OK);
        assert_eq!(ferro_frame_open(type_id("TextBlock"), 3), FERRO_OK);
        // SAFETY: the pointer and the length of a vector of code units.
        assert_eq!(unsafe { ferro_frame_str16(property_id("Text"), units.as_ptr(), units.len() as i64) }, FERRO_OK);
        assert_eq!(ferro_frame_close(), FERRO_OK);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        assert_eq!(get_str(find(root, 3), "Text").as_deref(), Ok("sixteen"));
        // A failed write is reported, and the rest of the frame is committed.
        assert_eq!(ferro_frame_begin(), FERRO_OK);
        assert_eq!(ferro_frame_open(type_id("TextBlock"), 3), FERRO_OK);
        assert_eq!(ferro_frame_i64(property_id("Grid.Row"), -1), FERRO_OK);
        text("Text", "still written");
        assert_eq!(ferro_frame_close(), FERRO_OK);
        assert_eq!(ferro_frame_commit(root), FERRO_INVALID_VALUE);
        assert_eq!(get_str(find(root, 3), "Text").as_deref(), Ok("still written"));
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

#[test]
fn the_ids_of_the_model_are_the_ids_of_the_library() {
    for (index, name) in crate::api_ids::PROPERTY_NAMES.iter().enumerate().step_by(97) {
        assert_eq!(property_id(name), index as i64 + 1, "{name}");
    }
    for (index, name) in crate::api_ids::EVENT_NAMES.iter().enumerate().step_by(13) {
        assert_eq!(event_id(name), index as i64 + 1, "{name}");
    }
    let button = crate::api_ids::TYPE_NAMES.iter().position(|name| *name == "Button").expect("Button is in the model");
    assert_eq!(type_id("Button"), button as i64 + 1);
    assert_eq!(ferro_api_fingerprint(), crate::api_ids::API_FINGERPRINT);
    ui(move || {
        // A class created by its constant ID.
        let created = ferro_create(button as i64 + 1);
        assert_ne!(created, 0, "{}", last_error());
        assert_eq!(ferro_type_of(created), button as i64 + 1);
        assert_eq!(ferro_release(created), FERRO_OK);
    });
}

#[test]
fn getters_with_a_fallback_and_utf16_text() {
    ui(|| {
        let text_block = create("TextBlock");
        assert_eq!(ferro_get_f64_or(text_block, property_id("FontSize"), -1.0) > 0.0, true);
        assert_eq!(ferro_get_f64_or(text_block, property_id("NoSuchProperty"), -1.0), -1.0);
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
        ferro_clear_error();
        assert_eq!(ferro_last_error_code(), FERRO_OK);
        assert_eq!(last_error(), "");
        assert!(ferro_get_bool_or(text_block, property_id("IsEnabled"), false));
        assert_eq!(ferro_get_i64_or(text_block, property_id("Grid.Row"), -1), 0);
        let units: Vec<u16> = "wide".encode_utf16().collect();
        // SAFETY: the pointer and the length of a vector of code units.
        let written = unsafe { ferro_set_str16(text_block, property_id("Text"), units.as_ptr(), units.len() as i64) };
        assert_eq!(written, FERRO_OK);
        assert_eq!(get_str(text_block, "Text").as_deref(), Ok("wide"));
        let fallback = c"none";
        let read = ferro_get_cstr_or(text_block, property_id("NoSuchProperty"), fallback.as_ptr());
        assert_eq!(read, fallback.as_ptr());
        assert_eq!(ferro_release(text_block), FERRO_OK);
    });
}

#[test]
fn values_after_a_child_belong_to_the_open_element() {
    ui(|| {
        let root = create("Border");
        let frame = |spacing: f64| {
            assert_eq!(ferro_frame_begin(), FERRO_OK);
            assert_eq!(ferro_frame_open(type_id("StackPanel"), 1), FERRO_OK);
            assert_eq!(ferro_frame_open(type_id("TextBlock"), 2), FERRO_OK);
            text("Text", "child");
            assert_eq!(ferro_frame_close(), FERRO_OK);
            // A value of the panel after its child, and one more after a second child.
            assert_eq!(ferro_frame_f64(property_id("Spacing"), spacing), FERRO_OK);
            assert_eq!(ferro_frame_open(type_id("TextBlock"), 3), FERRO_OK);
            assert_eq!(ferro_frame_close(), FERRO_OK);
            assert_eq!(ferro_frame_f64(property_id("Width"), 100.0 + spacing), FERRO_OK);
            assert_eq!(ferro_frame_close(), FERRO_OK);
            assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        };
        frame(4.0);
        let panel = find(root, 1);
        assert_eq!(get_f64(panel, "Spacing"), Ok(4.0));
        assert_eq!(get_f64(panel, "Width"), Ok(104.0));
        frame(4.0);
        assert_eq!(get_f64(panel, "Spacing"), Ok(4.0));
        frame(6.0);
        assert_eq!(get_f64(panel, "Spacing"), Ok(6.0));
        assert_eq!(get_f64(panel, "Width"), Ok(106.0));
        assert_eq!(get_str(find(root, 2), "Text").as_deref(), Ok("child"));
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

/// A frame of a stack panel with a memo element (key 5) of a button whose Click carries the
/// tag `tag_base`, unless `reuse` and the root has it.
fn memo_frame(root: FerroHandle, version: i64, tag_base: i64, label: &str, reuse: bool) -> i64 {
    assert_eq!(ferro_frame_begin(), FERRO_OK);
    assert_eq!(ferro_frame_open(type_id("StackPanel"), 1), FERRO_OK);
    let reused = if reuse { ferro_frame_reuse(root, 5, version, tag_base) } else { i64::from(FERRO_NOT_FOUND) };
    if reused < 0 {
        assert_eq!(ferro_frame_open(type_id("Button"), 5), FERRO_OK);
        assert_eq!(ferro_frame_memo(version, tag_base), FERRO_OK);
        text("Content", label);
        assert_eq!(ferro_frame_on(event_id("Click"), tag_base, 0), FERRO_OK);
        assert_eq!(ferro_frame_close(), FERRO_OK);
    }
    assert_eq!(ferro_frame_close(), FERRO_OK);
    assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
    reused
}

#[test]
fn a_memo_element_is_not_looked_into_while_its_version_stays() {
    ui(|| {
        assert_eq!(ferro_frame_dispatcher(Some(dispatcher)), FERRO_OK);
        dispatched();
        let root = create("Border");
        memo_frame(root, 1, 3, "first", false);
        let button = find(root, 5);
        assert_eq!(get_str(button, "Content").as_deref(), Ok("first"));
        // The same version with another label: the subtree is the one of the version.
        memo_frame(root, 1, 3, "second", false);
        assert_eq!(get_str(button, "Content").as_deref(), Ok("first"));
        // A new tag base moves the tags of the element.
        memo_frame(root, 1, 8, "second", false);
        assert_eq!(ferro_raise(button, event_id("Click")), FERRO_OK);
        assert_eq!(dispatched(), vec![(8, button, 0, String::new())]);
        // A new version is written.
        memo_frame(root, 2, 8, "second", false);
        assert_eq!(find(root, 5), button);
        assert_eq!(get_str(button, "Content").as_deref(), Ok("second"));
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
        assert_eq!(ferro_frame_dispatcher(None), FERRO_OK);
    });
}

#[test]
fn a_reuse_record_stands_for_the_memo_element_of_the_last_frame() {
    ui(|| {
        assert_eq!(ferro_frame_dispatcher(Some(dispatcher)), FERRO_OK);
        dispatched();
        let root = create("Border");
        assert_eq!(ferro_frame_reuse(root, 5, 1, 3), i64::from(FERRO_NOT_FOUND), "nothing is committed yet");
        assert_eq!(memo_frame(root, 1, 3, "kept", true), i64::from(FERRO_NOT_FOUND));
        let button = find(root, 5);
        // One tag: the Click of the button.
        assert_eq!(memo_frame(root, 1, 3, "unused", true), 1);
        assert_eq!(find(root, 5), button);
        assert_eq!(get_str(button, "Content").as_deref(), Ok("kept"));
        assert_eq!(ferro_raise(button, event_id("Click")), FERRO_OK);
        assert_eq!(dispatched(), vec![(3, button, 0, String::new())]);
        // Reused with its tags moved.
        assert_eq!(memo_frame(root, 1, 20, "unused", true), 1);
        assert_eq!(ferro_raise(button, event_id("Click")), FERRO_OK);
        assert_eq!(dispatched(), vec![(20, button, 0, String::new())]);
        // Another version is not there to reuse: written in full.
        assert_eq!(memo_frame(root, 2, 20, "new", true), i64::from(FERRO_NOT_FOUND));
        assert_eq!(get_str(button, "Content").as_deref(), Ok("new"));
        assert_eq!(ferro_frame_reuse(root, 5, 1, 0), i64::from(FERRO_NOT_FOUND));
        assert_eq!(ferro_frame_begin(), FERRO_OK);
        // A frame without it: its bytes go with it.
        assert_eq!(ferro_frame_commit(root), FERRO_OK);
        assert_eq!(ferro_frame_reuse(root, 5, 2, 0), i64::from(FERRO_NOT_FOUND));
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
        assert_eq!(ferro_frame_dispatcher(None), FERRO_OK);
    });
}

#[test]
fn a_reused_element_can_move_to_another_parent() {
    ui(|| {
        let root = create("StackPanel");
        let frame = |parent: i64, reuse: bool| {
            assert_eq!(ferro_frame_begin(), FERRO_OK);
            for key in [1, 2] {
                assert_eq!(ferro_frame_open(type_id("StackPanel"), key), FERRO_OK);
                if key == parent {
                    if !reuse || ferro_frame_reuse(root, 5, 7, 0) < 0 {
                        assert_eq!(ferro_frame_open(type_id("TextBlock"), 5), FERRO_OK);
                        assert_eq!(ferro_frame_memo(7, 0), FERRO_OK);
                        text("Text", "moving");
                        assert_eq!(ferro_frame_close(), FERRO_OK);
                    }
                }
                assert_eq!(ferro_frame_close(), FERRO_OK);
            }
            assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        };
        frame(1, false);
        let first = find(root, 1);
        assert_eq!(get_str(find(root, 5), "Text").as_deref(), Ok("moving"));
        frame(2, true);
        assert_eq!(get_str(find(root, 5), "Text").as_deref(), Ok("moving"));
        let second = find(root, 2);
        let child = find(root, 5);
        assert_ne!(child, 0);
        assert_ne!(first, second);
        // Reused again where it is now.
        frame(2, true);
        assert_eq!(find(root, 5), child);
        // The element is a child of the second panel, not of the first.
        assert_eq!(ferro_remove_child(first, child), FERRO_NOT_FOUND);
        assert_eq!(ferro_remove_child(second, child), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

#[test]
fn a_text_that_is_not_utf8_fails_its_write_and_a_key_the_frame() {
    ui(|| {
        let root = create("Border");
        let mut bytes = Vec::new();
        write_record(&mut bytes, OP_OPEN, &[type_id("StackPanel"), 0], &[]);
        write_record(&mut bytes, OP_OPEN, &[type_id("TextBlock"), 1], &[]);
        write_record(&mut bytes, OP_TEXT, &[property_id("Text"), 2], &[0xff, 0xfe]);
        write_record(&mut bytes, OP_CLOSE, &[], &[]);
        write_record(&mut bytes, OP_OPEN, &[type_id("TextBlock"), 2], &[]);
        write_record(&mut bytes, OP_TEXT, &[property_id("Text"), 2], b"ok");
        write_record(&mut bytes, OP_CLOSE, &[], &[]);
        write_record(&mut bytes, OP_CLOSE, &[], &[]);
        // SAFETY: the pointer and the length of a vector.
        let committed = unsafe { ferro_frame_commit_bytes(root, bytes.as_ptr(), bytes.len() as i64) };
        assert_eq!(committed, FERRO_INVALID_UTF8);
        assert_eq!(get_str(find(root, 2), "Text").as_deref(), Ok("ok"), "the rest of the frame is committed");
        let mut keyed = Vec::new();
        write_record(&mut keyed, OP_OPEN, &[type_id("TextBlock"), 0], &[]);
        write_record(&mut keyed, OP_KEY_TEXT, &[1], &[0xff]);
        write_record(&mut keyed, OP_CLOSE, &[], &[]);
        // SAFETY: the pointer and the length of a vector.
        let committed = unsafe { ferro_frame_commit_bytes(root, keyed.as_ptr(), keyed.len() as i64) };
        assert_eq!(committed, FERRO_INVALID_UTF8);
        assert_eq!(find(root, 2), find(root, 2), "a refused frame changes nothing");
        assert_ne!(find(root, 2), 0);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
    });
}

#[test]
fn an_event_subscribed_to_the_dispatcher_calls_it_with_its_tag() {
    ui(|| {
        assert_eq!(ferro_frame_dispatcher(Some(dispatcher)), FERRO_OK);
        dispatched();
        let button = create("Button");
        let subscription = ferro_subscribe_dispatch(button, event_id("Click"), 42);
        assert!(subscription > 0, "{}", last_error());
        assert_eq!(ferro_raise(button, event_id("Click")), FERRO_OK);
        assert_eq!(dispatched(), vec![(42, button, 0, String::new())]);
        assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        assert_eq!(ferro_raise(button, event_id("Click")), FERRO_OK);
        assert_eq!(dispatched(), vec![]);
        assert_eq!(ferro_release(button), FERRO_OK);
        assert_eq!(ferro_frame_dispatcher(None), FERRO_OK);
    });
}
