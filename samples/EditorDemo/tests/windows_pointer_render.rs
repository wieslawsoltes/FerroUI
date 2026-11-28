use editor_demo::EditorDemo;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::IBrush;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::platform::PixelFormats;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{PixelRect, Point};
use ferroui_controls::Window;
use ferroui_edit::TextEditor;
use std::future::poll_fn;
use std::ptr::null_mut;
use std::rc::Rc;
use std::task::Poll;
use windows_sys::Win32::Foundation::{HWND, WPARAM};
use windows_sys::Win32::System::SystemServices::MK_LBUTTON;
use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GCLP_HCURSOR, GetClassLongPtrW, IDC_ARROW, IDC_IBEAM, LoadCursorW, SendMessageW,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};

// Diagnostic brushes are unique within the sample, so the check works with both system themes.
const NUMBERS: u32 = 0xff3157ac;
const FOLDING: u32 = 0xff1fdf53;
const SELECTION: u32 = 0xffe138ac;

#[derive(Debug, Default)]
struct FramePixels {
    line_numbers: usize,
    fold_markers: usize,
    selection: usize,
}

pub(super) async fn check(
    demo: &EditorDemo,
    window: &Window,
) -> Result<(), Box<dyn std::error::Error>> {
    let editor = demo.editor();
    let brush =
        |color| Some(Rc::new(ImmutableSolidColorBrush::from_uint32(color)) as Rc<dyn IBrush>);
    editor.set_line_numbers_foreground(brush(NUMBERS));
    editor.set_value(TextEditor::folding_marker_brush_property(), brush(FOLDING));
    editor.set_value(
        TextEditor::selected_folding_marker_brush_property(),
        brush(FOLDING),
    );
    editor.set_selection_brush(brush(SELECTION));

    let baseline = rendered_colors(window).await?;
    if baseline.line_numbers < 10 || baseline.fold_markers < 10 || baseline.selection != 0 {
        return Err(format!("Unexpected idle frame colors {baseline:?}").into());
    }

    let handle = window.try_get_platform_handle().ok_or("Missing HWND")?;
    if handle.handle_descriptor() != Some("HWND") {
        return Err("Expected a Win32 window".into());
    }
    let hwnd = handle.handle() as HWND;
    let scale = window.render_scaling();
    let pointer =
        |message, buttons, x, y| send_pointer(hwnd, scale, message, buttons, Point::new(x, y));
    // SAFETY: The resource IDs and HWND belong to this process's loaded Win32 window class.
    let (arrow, beam) = unsafe {
        (
            LoadCursorW(null_mut(), IDC_ARROW),
            LoadCursorW(null_mut(), IDC_IBEAM),
        )
    };
    let cursor = || unsafe { GetClassLongPtrW(hwnd, GCLP_HCURSOR) };
    pointer(WM_MOUSEMOVE, 0, 120.0, 31.0);
    if arrow.is_null() || cursor() != arrow as usize {
        return Err("Toolbar did not install the Arrow cursor".into());
    }
    pointer(WM_MOUSEMOVE, 0, 127.0, 145.0);
    if beam.is_null() || cursor() != beam as usize {
        return Err("Editor did not install the I-beam cursor".into());
    }

    let buttons = MK_LBUTTON as WPARAM;
    for (direction, anchor_y, first_y, step) in [
        ("forward", 145.0, 200.0, 6.0),
        ("backward", 400.0, 338.0, -6.0),
    ] {
        let mut min_pixels = usize::MAX;
        let mut max_pixels = 0;
        let mut last = Point::new(127.0, anchor_y);
        pointer(WM_LBUTTONDOWN, buttons, last.x, last.y);
        for position in 0..24 {
            let x = if position == 17 {
                pointer(WM_MOUSELEAVE, 0, 0.0, 0.0);
                window.bounds().width + 35.0
            } else {
                200.0 + f64::from(position % 7) * 17.0
            };
            last = Point::new(x, first_y + f64::from(position) * step);
            pointer(WM_MOUSEMOVE, buttons, last.x, last.y);
            let selection = editor.selection();
            if selection.is_empty() || selection.primary().is_reversed() != (step < 0.0) {
                return Err(
                    format!("{direction} drag lost its selection at position {position}").into(),
                );
            }
            if position == 18 && cursor() != beam as usize {
                return Err(format!("{direction} drag lost the I-beam on re-entry").into());
            }
            // A caret-line change can queue a second render. Read both batches with the button held.
            for frame in 0..2 {
                let colors = rendered_colors(window).await?;
                verify_colors(
                    &baseline,
                    &colors,
                    &format!("{direction} drag position {position}, batch {frame}"),
                )?;
                min_pixels = min_pixels.min(colors.selection);
                max_pixels = max_pixels.max(colors.selection);
            }
        }
        let expected = editor.selection();
        pointer(WM_LBUTTONUP, 0, last.x, last.y);
        if editor.selection() != expected {
            return Err(
                format!("{direction} selection changed on release at the same point").into(),
            );
        }
        verify_colors(
            &baseline,
            &rendered_colors(window).await?,
            &format!("{direction} release"),
        )?;
        if max_pixels - min_pixels < 100 {
            return Err(
                format!("The rendered {direction} selection did not change during drag").into(),
            );
        }
        println!(
            "Direct2D at {scale:.2}×: 48 {direction} drag frames and release retained both gutters and the selection ({min_pixels}–{max_pixels} selection pixels)"
        );
    }
    Ok(())
}

pub(super) fn send_pointer(hwnd: HWND, scale: f64, message: u32, buttons: WPARAM, point: Point) {
    let x = (point.x * scale).round() as i16 as u16;
    let y = (point.y * scale).round() as i16 as u16;
    let position = (u32::from(y) << 16) | u32::from(x);
    // SAFETY: The fixture runs on the window's UI thread. The message carries packed integer coordinates.
    unsafe { SendMessageW(hwnd, message, buttons, position as isize) };
}

pub(super) async fn rendered_frame(window: &Window) -> Result<Bitmap, Box<dyn std::error::Error>> {
    Dispatcher::yield_with_priority(DispatcherPriority::BACKGROUND).await;
    let visual = ElementComposition::get_element_visual(window)
        .ok_or("Missing window composition visual")?;
    let compositor = visual.compositor();
    let job = compositor.create_composition_visual_snapshot(&visual, window.render_scaling());
    let mut registered = false;
    compositor.commit();
    poll_fn(|cx| {
        if let Some(result) = job.take_result() {
            return Poll::Ready(result);
        }
        if !registered {
            let wake = cx.waker().clone();
            job.on_completed(move || wake.wake());
            registered = true;
        }
        Poll::Pending
    })
    .await
    .map_err(|error| error.to_string().into())
}

async fn rendered_colors(window: &Window) -> Result<FramePixels, Box<dyn std::error::Error>> {
    let bitmap = rendered_frame(window).await?;
    let size = bitmap.pixel_size();
    if bitmap.format() != Some(PixelFormats::BGRA8888) || size.width <= 0 || size.height <= 0 {
        return Err(format!(
            "Unexpected Direct2D snapshot {size:?} {:?}",
            bitmap.format()
        )
        .into());
    }
    let stride = size.width * 4;
    let mut pixels = vec![0; stride as usize * size.height as usize];
    bitmap.copy_pixels(
        PixelRect::new(0, 0, size.width, size.height),
        &mut pixels,
        stride,
    );
    let mut counts = FramePixels::default();
    for pixel in pixels.chunks_exact(4) {
        match u32::from_le_bytes(pixel.try_into().unwrap()) {
            NUMBERS => counts.line_numbers += 1,
            FOLDING => counts.fold_markers += 1,
            SELECTION => counts.selection += 1,
            _ => {}
        }
    }
    Ok(counts)
}

fn verify_colors(baseline: &FramePixels, current: &FramePixels, stage: &str) -> Result<(), String> {
    if current.line_numbers * 10 < baseline.line_numbers * 9
        || current.fold_markers * 10 < baseline.fold_markers * 9
        || current.selection < 100
    {
        return Err(format!(
            "{stage}: gutters or selection lost rendered pixels (idle {baseline:?}, current {current:?})"
        ));
    }
    Ok(())
}
