use editor_demo::EditorDemo;
use ferroui_controls::Window;
use ferroui_system::task::oneshot;
use ferroui_win32::automation::testing::{
    Element, TextRange, UiaClient, VariantValue, TREE_SCOPE_DESCENDANTS,
};
use ferroui_win32::interop::unmanaged_methods::{
    client_to_screen, get_focus, gui_thread_caret, set_focus, POINT,
};
use std::thread;
use std::time::Duration;

const TEXT: &str = "alpha 😀\r\nomega gamma";
const CLASS_NAME_PROPERTY: i32 = 30012;

pub(super) async fn check(
    demo: &EditorDemo,
    window: &Window,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = window.try_get_platform_handle().ok_or("Missing HWND")?;
    if handle.handle_descriptor() != Some("HWND") {
        return Err("Expected a Win32 window".into());
    }
    let hwnd = handle.handle();
    // The private station has no foreground application to give the test
    // window keyboard focus automatically.
    set_focus(hwnd);
    if get_focus() != hwnd {
        return Err("The Direct2D test window did not acquire Win32 focus".into());
    }
    demo.editor().text_area().focus();
    run_uia_check(move || check_text_pattern(hwnd)).await
}

pub(super) async fn run_uia_check(
    check: impl FnOnce() -> Result<(), String> + Send + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = oneshot::channel();
    // UI Automation makes synchronous system calls into the window. Drive the
    // client on a separate COM thread while the editor pumps the dispatcher.
    thread::Builder::new()
        .name("Editor UI Automation client".into())
        .spawn(move || {
            let _ = sender.send(check());
        })?;
    receiver.await?.map_err(Into::into)
}

pub(super) fn editor_client(hwnd: isize) -> Result<(UiaClient, Element), String> {
    let client = UiaClient::new_v8()?;
    client.set_timeouts(1000, 3000)?;
    let root = client.element_from_handle(hwnd)?;
    let editor_condition = client.property_condition(
        CLASS_NAME_PROPERTY,
        &VariantValue::Bstr("TextEditor".into()),
    )?;
    let editors = root.find_all(TREE_SCOPE_DESCENDANTS, &editor_condition)?;
    let [editor] = editors.try_into().map_err(|editors: Vec<Element>| {
        format!(
            "Expected one TextEditor automation element, found {}",
            editors.len()
        )
    })?;
    Ok((client, editor))
}

fn check_text_pattern(hwnd: isize) -> Result<(), String> {
    let (_client, editor) = editor_client(hwnd)?;
    editor.set_focus()?;
    editor.set_value("")?;
    let pattern = editor.text_pattern()?;
    let document = pattern.document_range()?;
    editor.set_value(TEXT)?;
    if document.text()? != TEXT {
        return Err("A retained DocumentRange lost text inserted into an empty document".into());
    }

    let mut previous_caret = None;
    for expected in ["😀", "omega"] {
        let target = document
            .find_text(expected)?
            .ok_or_else(|| format!("Missing {expected:?} in UIA text"))?;
        target.select()?;
        let selection = pattern.selection()?;
        if !matches!(selection.as_slice(), [range] if range.text()? == expected) {
            return Err(format!(
                "UI Automation selection did not report {expected:?}"
            ));
        }
        let (active, caret) = pattern.caret_range()?;
        if !active || !caret.text()?.is_empty() || !caret.starts_at_end_of(&target)? {
            return Err(format!(
                "TextPattern2 did not report an active caret at the end of {expected:?}"
            ));
        }
        let caret_y = system_caret_y(hwnd, &target)?;
        if previous_caret.is_some_and(|previous| caret_y <= previous) {
            return Err(
                "The Windows system caret did not move down after selecting the next line".into(),
            );
        }
        previous_caret = Some(caret_y);
    }
    println!("Windows UI Automation verified live CRLF and emoji ranges, selection, glyph bounds and the TextPattern2 and system carets");
    Ok(())
}

fn system_caret_y(hwnd: isize, target: &TextRange) -> Result<i32, String> {
    let bounds = target.bounding_rectangles()?;
    let [[_, top, width, height]] = bounds.as_slice() else {
        return Err(format!(
            "Expected one rectangle for visible text, got {bounds:?}"
        ));
    };
    if *width <= 0.0 || *height <= 0.0 {
        return Err(format!(
            "Expected positive glyph dimensions, got {bounds:?}"
        ));
    }
    // The provider updates the system caret through the UI dispatcher after selection.
    let mut observed = None;
    for _ in 0..100 {
        if let Some((owner, rect, _)) = gui_thread_caret(hwnd) {
            let screen = client_to_screen(
                hwnd,
                POINT {
                    x: rect.left,
                    y: rect.top,
                },
            );
            if owner == hwnd
                && rect.bottom > rect.top
                && (f64::from(screen.y) - top).abs() <= height / 2.0
            {
                return Ok(screen.y);
            }
            observed = Some((owner == hwnd, screen.y, rect.bottom - rect.top));
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "No Windows system caret on the line at {top} (text height {height}). Last caret (owned, screen y, height): {observed:?}"
    ))
}
