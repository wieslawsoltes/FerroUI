use crate::windows_pointer_render::{rendered_frame, send_pointer};
use crate::windows_text_automation::{editor_client, run_uia_check};
use editor_demo::EditorDemo;
use ferroui_base::input::IScrollable;
use ferroui_base::{PixelPoint, Point, Ref, Vector};
use ferroui_controls::{TextBox, Window};
use ferroui_edit::{
    ITextRunConstructionContext, ITextViewConnect, InlineObjectElement, VisualLineElement,
    VisualLineElementGenerator, VisualLineElementGeneratorRef, VisualLineError,
};
use ferroui_win32::interop::unmanaged_methods::{
    get_focus, send_message, set_focus, VirtualKeyStates, WindowsMessage,
};
use std::error::Error;
use std::rc::Rc;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::SystemServices::MK_LBUTTON;

pub(super) async fn check(demo: &EditorDemo, window: &Window) -> Result<(), Box<dyn Error>> {
    let handle = window.try_get_platform_handle().ok_or("Missing HWND")?;
    if handle.handle_descriptor() != Some("HWND") {
        return Err("Expected a Win32 window".into());
    }
    let hwnd = handle.handle();
    set_focus(hwnd);
    if get_focus() != hwnd {
        return Err("The Direct2D test window did not acquire Win32 focus".into());
    }

    let editor = demo.editor();
    let area = editor.text_area();
    let document = format!("a!b\r\n{}", "line available for scrolling\r\n".repeat(100));
    editor.set_text(&document)?;
    editor.set_caret_offset(2)?;
    area.focus();
    let child = TextBox::new();
    child.set_width(128.0);
    child.set_height(36.0);
    child.set_text(Some("tail"));
    let generator: VisualLineElementGeneratorRef = Rc::new(InlineTextBox(child.clone())).into();
    area.element_generators().add(generator.clone());
    rendered_frame(window).await?;
    if child.visual_parent().is_none() || child.parent().is_none() || child.bounds().width <= 0.0 {
        return Err("The inline TextBox was not hosted and arranged".into());
    }

    let center = Point::new(child.bounds().width / 2.0, child.bounds().height / 2.0);
    let point = child
        .translate_point(center, window)
        .ok_or("The inline TextBox has no position in the Direct2D window")?;
    let scale = window.render_scaling();
    let pointer = |message, buttons| send_pointer(hwnd as HWND, scale, message, buttons, point);
    pointer(WindowsMessage::WM_MOUSEMOVE, 0);
    pointer(WindowsMessage::WM_LBUTTONDOWN, MK_LBUTTON as usize);
    pointer(WindowsMessage::WM_LBUTTONUP, 0);
    if !child.is_focused() {
        return Err("The native pointer did not focus the inline TextBox".into());
    }

    send_message(
        hwnd,
        WindowsMessage::WM_KEYDOWN,
        VirtualKeyStates::VK_HOME as usize,
        0x0147_0001,
    );
    send_message(
        hwnd,
        WindowsMessage::WM_KEYUP,
        VirtualKeyStates::VK_HOME as usize,
        0xc147_0001_u32 as isize,
    );
    send_message(hwnd, WindowsMessage::WM_CHAR, usize::from(b'x'), 1);
    if child.text().as_deref() != Some("xtail")
        || editor.text() != document
        || editor.caret_offset() != 2
    {
        return Err("Native Home and character input did not stay in the inline TextBox".into());
    }
    let screen = child.point_to_screen(center);
    run_uia_check(move || check_accessible_range(hwnd, Some(screen))).await?;

    area.set_offset(Vector::new(0.0, 1000.0));
    rendered_frame(window).await?;
    if area.offset().y <= 0.0
        || child.visual_parent().is_some()
        || child.parent().is_some()
        || !area.is_focused()
    {
        return Err("Scrolling did not release the inline TextBox and restore editor focus".into());
    }
    run_uia_check(move || check_accessible_range(hwnd, None)).await?;
    area.set_offset(Vector::default());
    rendered_frame(window).await?;
    if child.visual_parent().is_none()
        || child.parent().is_none()
        || child.text().as_deref() != Some("xtail")
    {
        return Err("Returning to the row did not reattach the edited TextBox".into());
    }
    area.element_generators().remove(&generator);
    rendered_frame(window).await?;
    if child.visual_parent().is_some() || child.parent().is_some() {
        return Err("The inline TextBox remained parented after removing its generator".into());
    }
    println!("Direct2D verified inline TextBox input, UI Automation text ranges and focus, eviction, and reattachment");
    Ok(())
}

fn check_accessible_range(hwnd: isize, screen: Option<PixelPoint>) -> Result<(), String> {
    let (client, editor) = editor_client(hwnd)?;
    let pattern = editor.text_pattern()?;
    let children = pattern.document_range()?.children()?;
    let Some(point) = screen else {
        if !children.is_empty() || !editor.has_keyboard_focus()? {
            return Err(
                "UI Automation did not remove the evicted TextBox and return focus to the editor"
                    .into(),
            );
        }
        return Ok(());
    };
    let [child] = children.as_slice() else {
        return Err(format!(
            "Expected one inline child in the UI Automation range, found {}",
            children.len()
        ));
    };
    if child.class_name()? != "TextBox"
        || !child.has_keyboard_focus()?
        || editor.has_keyboard_focus()?
    {
        return Err("UI Automation did not report focus on the inline TextBox".into());
    }
    let ranges = [
        pattern.range_from_child(child)?,
        pattern.range_from_point(point.x, point.y)?,
    ];
    for range in ranges {
        if range.text()? != "!" || !client.compare_elements(&range.enclosing_element()?, child)? {
            return Err(
                "UI Automation did not resolve the replacement text to the inline TextBox".into(),
            );
        }
    }
    Ok(())
}

struct InlineTextBox(Ref<TextBox>);

impl ITextViewConnect for InlineTextBox {}

impl VisualLineElementGenerator for InlineTextBox {
    fn get_first_interested_offset(
        &self,
        start: usize,
        _: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok((start <= 1).then_some(1))
    }

    fn construct_element(
        &self,
        _: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        Ok(Some(Box::new(InlineObjectElement::new(
            1,
            self.0.clone().upcast(),
            &**context.global_text_run_properties(),
        )?)))
    }
}
