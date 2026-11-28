//! Property values through the scalar setters and getters.

use super::support::*;
use crate::*;

#[test]
fn text_is_written_and_read() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(set_str(text, "Text", "Hello, World!"), FERRO_OK, "{}", last_error());
        assert_eq!(get_str(text, "Text").as_deref(), Ok("Hello, World!"));
        // A NUL-terminated string.
        // SAFETY: a NUL-terminated literal.
        let code = unsafe { ferro_set_str(text, property_id("Text"), c"Grüße".as_ptr(), -1) };
        assert_eq!(code, FERRO_OK);
        assert_eq!(get_str(text, "Text").as_deref(), Ok("Grüße"));
        // The empty string, as NULL with a length of 0.
        // SAFETY: NULL with a length of 0.
        assert_eq!(unsafe { ferro_set_str(text, property_id("Text"), std::ptr::null(), 0) }, FERRO_OK);
        assert_eq!(get_str(text, "Text").as_deref(), Ok(""));
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn a_buffer_that_is_too_small_is_reported_with_the_length_it_needs() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(set_str(text, "Text", "abcdef"), FERRO_OK);
        let mut buffer = [0x55u8; 6];
        let mut length = -1;
        // SAFETY: the buffer has 6 bytes.
        let code = unsafe { ferro_get_str(text, property_id("Text"), buffer.as_mut_ptr().cast(), 6, &mut length) };
        assert_eq!(code, FERRO_BUFFER_TOO_SMALL);
        assert_eq!(length, 6);
        assert_eq!(buffer, [0x55; 6], "nothing is written to a buffer that is too small");
        let mut buffer = [0x55u8; 7];
        // SAFETY: the buffer has 7 bytes; the length pointer may be NULL.
        let code = unsafe { ferro_get_str(text, property_id("Text"), buffer.as_mut_ptr().cast(), 7, std::ptr::null_mut()) };
        assert_eq!(code, FERRO_OK);
        assert_eq!(&buffer, b"abcdef\0");
        // SAFETY: a capacity without a buffer, and a negative capacity, are errors.
        unsafe {
            assert_eq!(ferro_get_str(text, property_id("Text"), std::ptr::null_mut(), 4, &mut length), FERRO_NULL_POINTER);
            assert_eq!(ferro_get_str(text, property_id("Text"), buffer.as_mut_ptr().cast(), -1, &mut length), FERRO_INVALID_ARGUMENT);
        }
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn a_c_string_of_the_library_is_returned_for_languages_without_buffers() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(set_str(text, "Text", "borrowed"), FERRO_OK);
        let pointer = ferro_get_cstr(text, property_id("Text"));
        assert!(!pointer.is_null());
        // SAFETY: valid until the next `ferro_get_cstr` of the thread.
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(pointer) }.to_str(), Ok("borrowed"));
        assert!(ferro_get_cstr(0, property_id("Text")).is_null());
        assert_eq!(ferro_last_error_code(), FERRO_BAD_HANDLE);
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn numbers_booleans_and_integers_are_written_and_read() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(ferro_set_f64(text, property_id("FontSize"), 28.0), FERRO_OK, "{}", last_error());
        assert_eq!(get_f64(text, "FontSize"), Ok(28.0));
        // An integer to a property of type double.
        assert_eq!(ferro_set_i64(text, property_id("Width"), 120), FERRO_OK, "{}", last_error());
        assert_eq!(get_f64(text, "Width"), Ok(120.0));
        assert_eq!(ferro_set_bool(text, property_id("IsEnabled"), false), FERRO_OK, "{}", last_error());
        assert_eq!(get_bool(text, "IsEnabled"), Ok(false));
        assert_eq!(ferro_set_bool(text, property_id("IsEnabled"), true), FERRO_OK);
        assert_eq!(get_bool(text, "IsEnabled"), Ok(true));
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn enumerations_are_integers_or_member_names() {
    ui(|| {
        let panel = create("StackPanel");
        assert_eq!(set_str(panel, "Orientation", "horizontal"), FERRO_OK, "{}", last_error());
        let horizontal = get_i64(panel, "Orientation").expect("an integer");
        assert_eq!(get_str(panel, "Orientation").as_deref(), Ok("Horizontal"));
        assert_eq!(set_str(panel, "Orientation", "Vertical"), FERRO_OK);
        let vertical = get_i64(panel, "Orientation").expect("an integer");
        assert_ne!(horizontal, vertical);
        assert_eq!(ferro_set_i64(panel, property_id("Orientation"), horizontal), FERRO_OK, "{}", last_error());
        assert_eq!(get_str(panel, "Orientation").as_deref(), Ok("Horizontal"));
        assert_eq!(set_str(panel, "HorizontalAlignment", "Center"), FERRO_OK, "{}", last_error());
        assert_eq!(get_str(panel, "HorizontalAlignment").as_deref(), Ok("Center"));
        assert_eq!(ferro_set_f64(panel, property_id("Spacing"), 12.0), FERRO_OK);
        assert_eq!(get_f64(panel, "Spacing"), Ok(12.0));
        assert_eq!(ferro_release(panel), FERRO_OK);
    });
}

#[test]
fn text_is_parsed_into_values_of_other_types() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(set_str(text, "Margin", "4,2"), FERRO_OK, "{}", last_error());
        assert_eq!(set_str(text, "Width", "64"), FERRO_OK, "{}", last_error());
        assert_eq!(get_f64(text, "Width"), Ok(64.0));
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn attached_properties_are_named_with_their_owner() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(ferro_set_i64(text, property_id("Grid.Row"), 2), FERRO_OK, "{}", last_error());
        assert_eq!(get_i64(text, "Grid.Row"), Ok(2));
        // The validation of the property: a row is not negative.
        assert_eq!(ferro_set_i64(text, property_id("Grid.Row"), -1), FERRO_INVALID_VALUE);
        assert_eq!(get_i64(text, "Grid.Row"), Ok(2));
        assert_eq!(ferro_clear(text, property_id("Grid.Row")), FERRO_OK);
        assert_eq!(get_i64(text, "Grid.Row"), Ok(0));
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}

#[test]
fn objects_are_written_and_read_by_handle() {
    ui(|| {
        let button = create("Button");
        let label = create("TextBlock");
        assert_eq!(ferro_set_handle(button, property_id("Content"), label), FERRO_OK, "{}", last_error());
        let content = get_handle(button, "Content").expect("a handle");
        assert_eq!(content, label, "an object has one handle");
        assert_eq!(ferro_release(content), FERRO_OK, "the getter counted a reference");
        assert_eq!(ferro_set_handle(button, property_id("Content"), 0), FERRO_OK, "{}", last_error());
        assert_eq!(get_handle(button, "Content"), Ok(0));
        // Text as content.
        assert_eq!(set_str(button, "Content", "Click me"), FERRO_OK, "{}", last_error());
        assert_eq!(get_str(button, "Content").as_deref(), Ok("Click me"));
        assert_eq!(ferro_release(label), FERRO_OK);
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}

#[test]
fn values_that_do_not_fit_are_refused() {
    ui(|| {
        let text = create("TextBlock");
        let button = create("Button");
        // No such property on the class.
        assert_eq!(ferro_set_f64(text, property_id("Orientation"), 1.0), FERRO_NOT_FOUND);
        assert!(last_error().contains("TextBlock"), "{}", last_error());
        assert_eq!(ferro_set_f64(text, 0, 1.0), FERRO_NOT_FOUND);
        assert_eq!(ferro_set_f64(text, 1 << 40, 1.0), FERRO_NOT_FOUND);
        // An object where a number goes.
        assert_eq!(ferro_set_handle(text, property_id("FontSize"), button), FERRO_TYPE_MISMATCH);
        // A read-only property.
        assert_eq!(ferro_set_f64(text, property_id("Bounds"), 1.0), FERRO_READ_ONLY);
        // A number is not a boolean.
        assert_eq!(get_bool(text, "FontSize"), Err(FERRO_TYPE_MISMATCH));
        // NULL for a result.
        // SAFETY: NULL is checked.
        assert_eq!(unsafe { ferro_get_f64(text, property_id("FontSize"), std::ptr::null_mut()) }, FERRO_NULL_POINTER);
        assert_eq!(ferro_release(button), FERRO_OK);
        assert_eq!(ferro_release(text), FERRO_OK);
    });
}
