//! Exercises the editor through the accessibility selectors of a live AppKit window.

use editor_demo::EditorDemo;
use ferroui_base::layout::ILayoutManager;
use ferroui_controls::Window;
use ferroui_controls::automation::AutomationProperties;
use ferroui_edit::core::editing::{Selection, SelectionSegment};
use ferroui_edit::core::highlighting::RichTextModel;
use ferroui_edit::{RichTextColorizer, VisualLineTransformerRef};
use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_void};
use std::rc::Rc;

type Object = *mut c_void;
type Selector = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    location: usize,
    length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
struct Frame {
    origin: Point,
    width: f64,
    height: f64,
}

#[link(name = "objc")]
unsafe extern "C" {
    fn sel_registerName(name: *const c_char) -> Selector;
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSAccessibilityForegroundColorTextAttribute: Object;
    static NSAccessibilityFontTextAttribute: Object;
    static NSAccessibilityFontNameKey: Object;
    static NSAccessibilityFontSizeKey: Object;
    static NSAccessibilityUnderlineTextAttribute: Object;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorGetNumberOfComponents(color: Object) -> usize;
    fn CGColorGetComponents(color: Object) -> *const f64;
}

fn selector(name: &CStr) -> Selector {
    // SAFETY: The Objective-C runtime interns the NUL-terminated name.
    unsafe { sel_registerName(name.as_ptr()) }
}

/// The caller supplies the selector's actual return type and a live receiver.
unsafe fn get<T>(object: Object, name: &CStr) -> T {
    let call: unsafe extern "C" fn(Object, Selector) -> T =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { call(object, selector(name)) }
}

/// The caller supplies the selector's actual argument and return types.
unsafe fn call<A, T>(object: Object, name: &CStr, argument: A) -> T {
    let call: unsafe extern "C" fn(Object, Selector, A) -> T =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { call(object, selector(name), argument) }
}

unsafe fn attribute(text: Object, key: Object, index: usize, range: &mut Range) -> Object {
    let call: unsafe extern "C" fn(Object, Selector, Object, usize, *mut Range) -> Object =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe {
        call(
            text,
            selector(c"attribute:atIndex:effectiveRange:"),
            key,
            index,
            range,
        )
    }
}

unsafe fn frame_for_range(object: Object, range: Range) -> Frame {
    #[cfg(target_arch = "aarch64")]
    {
        unsafe { call(object, c"accessibilityFrameForRange:", range) }
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut frame = std::mem::MaybeUninit::<Frame>::uninit();
        // The Intel Objective-C ABI returns NSRect through a hidden pointer.
        let call: unsafe extern "C" fn(*mut Frame, Object, Selector, Range) =
            unsafe { std::mem::transmute(objc_msgSend_stret as unsafe extern "C" fn()) };
        unsafe {
            call(
                frame.as_mut_ptr(),
                object,
                selector(c"accessibilityFrameForRange:"),
                range,
            );
            frame.assume_init()
        }
    }
}

unsafe fn string(object: Object) -> Option<String> {
    if object.is_null() {
        return None;
    }
    let bytes: *const c_char = unsafe { get(object, c"UTF8String") };
    (!bytes.is_null()).then(|| {
        unsafe { CStr::from_ptr(bytes) }
            .to_string_lossy()
            .into_owned()
    })
}

fn find_editor(object: Object, depth: usize) -> Option<Object> {
    if object.is_null() || depth == 0 {
        return None;
    }
    // SAFETY: AppKit accessibility elements expose an optional NSString id
    // and an NSArray of child elements. Each reference is borrowed on the UI thread.
    unsafe {
        if call::<_, bool>(
            object,
            c"respondsToSelector:",
            selector(c"accessibilityIdentifier"),
        ) {
            let id: Object = get(object, c"accessibilityIdentifier");
            if string(id).as_deref() == Some("ferroedit-appkit-text-fixture") {
                return Some(object);
            }
        }
        if !call::<_, bool>(
            object,
            c"respondsToSelector:",
            selector(c"accessibilityChildren"),
        ) {
            return None;
        }
        let children: Object = get(object, c"accessibilityChildren");
        if children.is_null() {
            return None;
        }
        let count: usize = get(children, c"count");
        (0..count)
            .find_map(|index| find_editor(call(children, c"objectAtIndex:", index), depth - 1))
    }
}

pub(super) fn check(demo: &EditorDemo, window: &Window) -> Result<(), Box<dyn std::error::Error>> {
    let editor = demo.editor();
    let story = "A😀D\r\nxyz\0";
    let emoji = Range {
        location: 1,
        length: 2,
    };
    AutomationProperties::set_automation_id(&editor, Some("ferroedit-appkit-text-fixture"));
    editor.set_text(story)?;
    let mut model = RichTextModel::new();
    model.set_foreground(1, 2, Some("Red"))?;
    model.set_font_family(1, 2, Some("Menlo"))?;
    model.set_font_size(1, 2, Some(24.0))?;
    model.set_underline(1, 2, Some(true))?;
    let transformer: VisualLineTransformerRef =
        Rc::new(RichTextColorizer::new(Rc::new(RefCell::new(model)))).into();
    editor.text_area().line_transformers().add(transformer);
    editor.set_selection(Selection::simple(3, 1))?;
    window.layout_manager().execute_layout_pass();

    let handle = window
        .try_get_platform_handle()
        .ok_or("Missing AppKit window")?;
    if handle.handle_descriptor() != Some("NSWindow") || handle.handle() == 0 {
        return Err("Expected an NSWindow handle".into());
    }

    // SAFETY: The live AppKit window, its content view, and its accessibility
    // elements stay on the UI thread. The calls below use their declared ABI.
    unsafe {
        // FrnWindow exposes the accessibility root on its view inside the content container.
        let view: Object = get(handle.handle() as Object, c"view");
        let element =
            find_editor(view, 16).ok_or("Editor missing from the AppKit accessibility tree")?;
        assert_eq!(
            string(get(element, c"accessibilityRole")).as_deref(),
            Some("AXTextArea")
        );
        assert_eq!(
            get::<isize>(element, c"accessibilityNumberOfCharacters"),
            10
        );
        let value: Object = get(element, c"accessibilityValue");
        assert_eq!(get::<usize>(value, c"length"), 10);
        assert_eq!(
            get::<Range>(element, c"accessibilitySelectedTextRange"),
            emoji
        );
        assert_eq!(
            string(get(element, c"accessibilitySelectedText")).as_deref(),
            Some("😀")
        );

        for name in [
            c"accessibilityLineForIndex:",
            c"accessibilityRangeForLine:",
            c"accessibilityInsertionPointLineNumber",
            c"accessibilityStyleRangeForIndex:",
            c"accessibilityAttributedStringForRange:",
        ] {
            assert!(call::<_, bool>(
                element,
                c"isAccessibilitySelectorAllowed:",
                selector(name)
            ));
        }
        for (offset, line) in [(2, 0), (4, 0), (5, 0), (6, 1), (10, 1), (11, isize::MAX)] {
            assert_eq!(
                call::<isize, isize>(element, c"accessibilityLineForIndex:", offset),
                line
            );
        }
        assert_eq!(
            get::<isize>(element, c"accessibilityInsertionPointLineNumber"),
            0
        );
        for (line, location, length) in [(0, 0, 6), (1, 6, 4), (2, isize::MAX as usize, 0)] {
            assert_eq!(
                call::<isize, Range>(element, c"accessibilityRangeForLine:", line),
                Range { location, length }
            );
        }

        let visible: Range = get(element, c"accessibilityVisibleCharacterRange");
        assert_eq!(
            visible,
            Range {
                location: 0,
                length: 10
            }
        );
        let nul: Object = call(
            element,
            c"accessibilityStringForRange:",
            Range {
                location: 9,
                length: 1,
            },
        );
        assert!(!nul.is_null());
        assert_eq!(get::<usize>(nul, c"length"), 1);
        assert_eq!(call::<usize, u16>(nul, c"characterAtIndex:", 0), 0);
        let invalid: Object = call(
            element,
            c"accessibilityStringForRange:",
            Range {
                location: usize::MAX,
                length: 1,
            },
        );
        assert!(invalid.is_null());

        assert_eq!(
            call::<isize, Range>(element, c"accessibilityStyleRangeForIndex:", 2),
            emoji
        );
        let styled: Object = call(
            element,
            c"accessibilityAttributedStringForRange:",
            Range {
                location: 1,
                length: 9,
            },
        );
        assert!(!styled.is_null());
        let content: Object = get(styled, c"string");
        assert_eq!(get::<usize>(content, c"length"), 9);
        assert_eq!(call::<usize, u16>(content, c"characterAtIndex:", 8), 0);
        let mut style_range = Range {
            location: 0,
            length: 0,
        };
        let color = attribute(
            styled,
            NSAccessibilityForegroundColorTextAttribute,
            1,
            &mut style_range,
        );
        assert!(!color.is_null());
        assert_eq!(
            style_range,
            Range {
                location: 0,
                length: 2
            }
        );
        assert_eq!(CGColorGetNumberOfComponents(color), 4);
        let components = CGColorGetComponents(color);
        assert!(!components.is_null());
        assert_eq!(
            std::slice::from_raw_parts(components, 4),
            [1.0, 0.0, 0.0, 1.0]
        );
        let font = attribute(
            styled,
            NSAccessibilityFontTextAttribute,
            0,
            &mut style_range,
        );
        assert!(!font.is_null());
        let name: Object = call(font, c"objectForKey:", NSAccessibilityFontNameKey);
        assert_eq!(string(name).as_deref(), Some("Menlo-Regular"));
        let size: Object = call(font, c"objectForKey:", NSAccessibilityFontSizeKey);
        assert_eq!(get::<f64>(size, c"doubleValue"), 18.0);
        let underline = attribute(
            styled,
            NSAccessibilityUnderlineTextAttribute,
            1,
            &mut style_range,
        );
        assert_eq!(get::<isize>(underline, c"integerValue"), 1);
        assert!(
            attribute(
                styled,
                NSAccessibilityUnderlineTextAttribute,
                2,
                &mut style_range
            )
            .is_null()
        );

        let frame = frame_for_range(element, emoji);
        assert_eq!(
            call::<isize, Range>(element, c"accessibilityRangeForIndex:", 2),
            emoji
        );
        assert!(frame.width > 0.0 && frame.height > 0.0);
        for horizontal in [0.25, 0.75] {
            let point = Point {
                x: frame.origin.x + horizontal * frame.width,
                y: frame.origin.y + frame.height / 2.0,
            };
            assert_eq!(
                call::<_, Range>(element, c"accessibilityRangeForPosition:", point),
                emoji
            );
        }

        editor.set_selection(Selection::from_segments(
            [SelectionSegment::new(0, 1), SelectionSegment::new(6, 7)],
            1,
        ))?;
        let ranges: Object = get(element, c"accessibilitySelectedTextRanges");
        assert_eq!(get::<usize>(ranges, c"count"), 2);
        assert_eq!(
            get::<isize>(element, c"accessibilityInsertionPointLineNumber"),
            1
        );
        assert_eq!(
            get::<Range>(element, c"accessibilitySelectedTextRange"),
            Range {
                location: 6,
                length: 1
            }
        );

        call::<_, ()>(element, c"setAccessibilitySelectedTextRange:", emoji);
        assert_eq!(editor.selected_text()?, "😀");
        assert_eq!(
            get::<Range>(element, c"accessibilitySelectedTextRange"),
            emoji
        );

        editor.set_caret_offset(3)?;
        assert_eq!(
            get::<Range>(element, c"accessibilitySelectedTextRange"),
            Range {
                location: 3,
                length: 0
            }
        );
        assert_eq!(
            string(get(element, c"accessibilitySelectedText")).as_deref(),
            Some("")
        );

        let replacement = "a\0β😀z";
        editor.set_text(replacement)?;
        let native_value: Object = get(element, c"accessibilityValue");
        assert!(!native_value.is_null());
        assert_eq!(get::<usize>(native_value, c"length"), 6);
        editor.set_text("before")?;
        call::<_, ()>(element, c"setAccessibilityValue:", native_value);
        assert_eq!(editor.text(), replacement);

        editor.set_is_read_only(true);
        call::<_, ()>(
            element,
            c"setAccessibilityValue:",
            std::ptr::null_mut::<c_void>(),
        );
        assert_eq!(editor.text(), replacement);
    }

    println!(
        "AppKit verified UTF-16 text and selections, document lines, formatting, visible text, range geometry, and glyph hits"
    );
    Ok(())
}
