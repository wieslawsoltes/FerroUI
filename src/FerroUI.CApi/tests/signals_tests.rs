//! Signals shared through memory and the event ring (docs/c-abi.md, section 14): slots written
//! with a store and applied to bound properties, watched properties written into slots, the
//! checks of a slot index, `bind` records of frames, and the ring.

use super::support::*;
use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// The table of the process (the tests share it): the same capacity for every test.
fn table() -> &'static FerroSignalTable {
    let table = ferro_signal_table(256);
    assert!(!table.is_null(), "{}", last_error());
    // SAFETY: the table lives for the process.
    unsafe { &*table }
}

/// Stores a number into a slot as a language that writes memory does: the value, then the bit.
fn store_f64(table: &FerroSignalTable, slot: i64, value: f64) {
    let index = slot as usize;
    // SAFETY: the slot is inside the table; the arrays are atomics of the library.
    unsafe {
        (*(table.values.add(index) as *const AtomicU64)).store(value.to_bits(), Ordering::Relaxed);
        (*(table.dirty.add(index / 64) as *const AtomicU64)).fetch_or(1 << (index % 64), Ordering::Release);
    }
}

fn load(words: *mut u64, index: usize) -> u64 {
    // SAFETY: the index is inside the array; the arrays are atomics of the library.
    unsafe { (*(words.add(index) as *const AtomicU64)).load(Ordering::Acquire) }
}

#[test]
fn the_table_is_made_once_with_a_capacity_of_whole_words() {
    ui(|| {
        let first = table();
        assert_eq!(first.layout, FERRO_SIGNAL_LAYOUT);
        assert_eq!(first.capacity % 64, 0);
        assert!(first.capacity >= 256);
        assert!(std::ptr::eq(ferro_signal_table(1), first), "the same table, whatever the capacity");
    });
}

#[test]
fn a_store_and_its_dirty_bit_update_a_bound_property_at_the_apply() {
    ui(|| {
        let table = table();
        let slot = ferro_signal_new(FERRO_SIGNAL_F64);
        assert!(slot >= 0, "{}", last_error());
        let text = create("TextBlock");
        // A bind writes the value the slot has (0 is no font size: a value first).
        store_f64(table, slot, 12.0);
        let id = ferro_signal_bind(slot, text, property_id("FontSize"));
        assert!(id > 0, "{}", last_error());
        assert_eq!(get_f64(text, "FontSize"), Ok(12.0));
        assert_eq!(ferro_signals_apply(), 1, "{}", last_error());
        // A store and its bit, applied.
        store_f64(table, slot, 21.0);
        let version = ferro_signal_version(slot);
        assert_eq!(get_f64(text, "FontSize"), Ok(12.0), "nothing changes before the apply");
        assert_eq!(ferro_signals_apply(), 1, "{}", last_error());
        assert_eq!(get_f64(text, "FontSize"), Ok(21.0));
        assert_eq!(ferro_signal_version(slot), version + 1);
        // The call form of a write, applied at the next defined point.
        assert_eq!(ferro_signal_set_f64(slot, 30.0), FERRO_OK);
        assert_eq!(ferro_signal_get_f64(slot), 30.0);
        assert_eq!(ferro_signals_apply(), 1);
        assert_eq!(get_f64(text, "FontSize"), Ok(30.0));
        assert_eq!(ferro_signals_apply(), 0, "nothing is dirty any more");
        assert_eq!(ferro_signal_unbind(id), FERRO_OK);
        assert_eq!(ferro_signal_set_f64(slot, 40.0), FERRO_OK);
        assert_eq!(ferro_signals_apply(), 1);
        assert_eq!(get_f64(text, "FontSize"), Ok(30.0), "an unbound property keeps its value");
        assert_eq!(ferro_signal_free(slot), FERRO_OK);
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn a_watched_property_is_written_into_its_slot() {
    ui(|| {
        let table = table();
        let slot = ferro_signal_new(FERRO_SIGNAL_TEXT);
        let field = create("TextBox");
        assert_eq!(set_str(field, "Text", "start"), FERRO_OK);
        let id = ferro_signal_watch(slot, field, property_id("Text"));
        assert!(id > 0, "{}", last_error());
        let read = || {
            // SAFETY: a NUL-terminated string of the library.
            unsafe { std::ffi::CStr::from_ptr(ferro_signal_get_cstr(slot)) }.to_string_lossy().into_owned()
        };
        assert_eq!(read(), "start");
        let version = ferro_signal_version(slot);
        assert!(ferro_signal_take_changed(slot));
        // Typing: the text changes, the slot follows with no call of the language.
        assert_eq!(set_str(field, "Text", "typed"), FERRO_OK);
        assert_eq!(ferro_signal_version(slot), version + 1);
        assert_eq!(load(table.values, slot as usize), 5, "the value of a text slot is its length");
        assert_ne!(load(table.changed, slot as usize / 64) & (1 << (slot % 64)), 0);
        assert_eq!(read(), "typed");
        // A number slot watched on a number property.
        let size = ferro_signal_new(FERRO_SIGNAL_F64);
        let watch = ferro_signal_watch(size, field, property_id("FontSize"));
        assert!(watch > 0, "{}", last_error());
        assert_eq!(ferro_set_f64(field, property_id("FontSize"), 17.0), FERRO_OK);
        assert_eq!(f64::from_bits(load(table.values, size as usize)), 17.0);
        assert_eq!(ferro_signal_unbind(watch), FERRO_OK);
        assert_eq!(ferro_set_f64(field, property_id("FontSize"), 18.0), FERRO_OK);
        assert_eq!(f64::from_bits(load(table.values, size as usize)), 17.0, "an ended watch writes nothing");
        assert_eq!(ferro_signal_free(size), FERRO_OK);
        assert_eq!(ferro_signal_free(slot), FERRO_OK);
        assert_eq!(ferro_release(field), FERRO_OK);
    });
}

#[test]
fn a_two_way_slot_does_not_echo() {
    ui(|| {
        table();
        let slot = ferro_signal_new(FERRO_SIGNAL_TEXT);
        let field = create("TextBox");
        assert!(ferro_signal_watch(slot, field, property_id("Text")) > 0, "{}", last_error());
        assert!(ferro_signal_bind(slot, field, property_id("Text")) > 0, "{}", last_error());
        let version = ferro_signal_version(slot);
        // SAFETY: a NUL-terminated literal.
        assert_eq!(unsafe { ferro_signal_set_str(slot, c"from the language".as_ptr(), -1) }, FERRO_OK);
        assert_eq!(ferro_signals_apply(), 1);
        assert_eq!(get_str(field, "Text").as_deref(), Ok("from the language"));
        assert_eq!(ferro_signal_version(slot), version + 1, "the apply raised it once; the watch did not again");
        assert_eq!(ferro_signal_free(slot), FERRO_OK);
        assert_eq!(ferro_release(field), FERRO_OK);
    });
}

#[test]
fn slots_are_checked() {
    ui(|| {
        let table = table();
        assert_eq!(ferro_signal_set_f64(-1, 1.0), FERRO_INVALID_ARGUMENT);
        assert_eq!(ferro_signal_set_f64(i64::from(table.capacity), 1.0), FERRO_INVALID_ARGUMENT);
        assert_eq!(ferro_signal_version(1 << 40), -1);
        assert_eq!(ferro_signal_new(9), i64::from(FERRO_INVALID_ARGUMENT));
        let slot = ferro_signal_new(FERRO_SIGNAL_BOOL);
        assert_eq!(ferro_signal_set_f64(slot, 1.0), FERRO_TYPE_MISMATCH);
        assert_eq!(ferro_signal_bind(slot, 0, property_id("IsEnabled")), i64::from(FERRO_BAD_HANDLE));
        assert_eq!(ferro_signal_free(slot), FERRO_OK);
        assert_eq!(ferro_signal_free(slot), FERRO_INVALID_STATE);
        assert_eq!(ferro_signal_set_bool(slot, true), FERRO_TYPE_MISMATCH, "a free slot takes no value");
        // Dirty bits of free slots are ignored.
        store_f64(table, slot, 1.0);
        assert_eq!(ferro_signals_apply(), 0);
        // Off the UI thread: the calls that need it refuse.
        std::thread::spawn(|| assert_eq!(ferro_signal_new(FERRO_SIGNAL_I64), i64::from(FERRO_WRONG_THREAD)))
            .join()
            .expect("the thread runs");
    });
}

#[test]
fn a_bind_record_keeps_a_property_on_a_slot_between_frames() {
    ui(|| {
        table();
        let slot = ferro_signal_new(FERRO_SIGNAL_F64);
        assert_eq!(ferro_signal_set_f64(slot, 15.0), FERRO_OK);
        let root = create("Border");
        let frame = |bound: bool| {
            assert_eq!(ferro_frame_begin(), FERRO_OK);
            assert_eq!(ferro_frame_open(type_id("TextBlock"), 1), FERRO_OK);
            if bound {
                assert_eq!(ferro_frame_bind(property_id("FontSize"), slot), FERRO_OK);
            }
            assert_eq!(ferro_frame_close(), FERRO_OK);
            assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        };
        frame(true);
        let text = ferro_frame_find(root, 1);
        assert_eq!(ferro_release(text), FERRO_OK);
        assert_eq!(get_f64(text, "FontSize"), Ok(15.0));
        // A write of the slot, no frame: the property follows at the apply.
        assert_eq!(ferro_signal_set_f64(slot, 25.0), FERRO_OK);
        assert_eq!(ferro_signals_apply(), 1);
        assert_eq!(get_f64(text, "FontSize"), Ok(25.0));
        // The same frame again keeps the binding; a write is applied at the start of a commit.
        assert_eq!(ferro_signal_set_f64(slot, 26.0), FERRO_OK);
        frame(true);
        assert_eq!(get_f64(text, "FontSize"), Ok(26.0));
        // A frame without the record ends the binding.
        frame(false);
        assert_eq!(ferro_signal_set_f64(slot, 27.0), FERRO_OK);
        assert_eq!(ferro_signals_apply(), 1);
        assert_eq!(get_f64(text, "FontSize"), Ok(26.0));
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        assert_eq!(ferro_release(root), FERRO_OK);
        assert_eq!(ferro_signal_free(slot), FERRO_OK);
    });
}

#[test]
fn events_go_into_the_ring_and_a_full_ring_drops() {
    ui(|| {
        let ring = ferro_event_ring(4);
        assert!(!ring.is_null(), "{}", last_error());
        // SAFETY: the ring lives for the process.
        let ring = unsafe { &*ring };
        assert_eq!(ring.capacity, 4);
        let button = create("Button");
        let click = event_id("Click");
        let subscription = ferro_subscribe_ring(button, click, 7);
        assert!(subscription > 0, "{}", last_error());
        while ferro_event_ring_pop() >= 0 {}
        let head = load(ring.head, 0);
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(load(ring.head, 0), head + 1);
        let at = ((head % 4) * 4) as usize;
        // SAFETY: a record inside the ring.
        let record = unsafe { std::slice::from_raw_parts(ring.records.add(at), 4) };
        assert_eq!((record[0], record[1] as FerroHandle, record[3]), (7, button, head as i64));
        assert_eq!(ferro_event_ring_pop(), 7);
        assert_eq!(ferro_event_ring_sender(), button);
        assert_eq!(ferro_event_ring_pop(), i64::from(FERRO_NOT_FOUND));
        let dropped = load(ring.dropped, 0);
        for _ in 0..6 {
            assert_eq!(ferro_raise(button, click), FERRO_OK);
        }
        assert_eq!(load(ring.dropped, 0), dropped + 2, "four fit, two are dropped");
        while ferro_event_ring_pop() >= 0 {}
        assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        assert_eq!(ferro_subscribe_ring(button, click, -1), i64::from(FERRO_INVALID_ARGUMENT));
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}
