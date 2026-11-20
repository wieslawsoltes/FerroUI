//! `Controls/CameraCaptureUI.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a style: CameraCaptureUI of Windows is a system dialog, and microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0)
//! has no style for it. Here it is a control in the window (docs/media/ISSUE-561.md): a
//! CameraPreview over the Start, Take photo, Record, Finish and Stop actions and a status text,
//! which are Button and TextBlock controls with the control themes of the theme.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::Brush;
use ferroui_base::media::TextWrapping;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::Border;
use ferroui_controls::Button;
use ferroui_controls::ContentControl;
use ferroui_controls::Control;
use ferroui_controls::Decorator;
use ferroui_controls::StackPanel;
use ferroui_controls::TextBlock;
use ferroui_controls::WrapPanel;
use ferroui_markup_xaml::rust_theme::type_key;
use ferroui_media_camera_controls::CameraCaptureUI;
use ferroui_media_camera_controls::CameraPreview;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    resources.control_theme(type_key::<CameraCaptureUI>(), camera_capture_ui_theme);
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

/// The template of `CameraCaptureUI` of `camera_capture_ui_theme`.
fn camera_capture_ui_theme_template(t: &Template<CameraCaptureUI>) -> Result<Ref<Control>> {
    Ok(t.make(StackPanel::new())
        .set(StackPanel::spacing_property(), 8.0)
        .children([
            t.make(Border::new())
                .set(Border::background_property(), Some(literal(Brush::parse("Black"))?))
                .set(Layoutable::height_property(), 300.0)
                .child(
                    Decorator::child_property(),
                    t.make(CameraPreview::new()).name("CapturePreview")?,
                )
                .control(),
            t.make(WrapPanel::new())
                .children([
                    t.make(Button::new())
                        .name("CaptureStart")?
                        .set(
                            ContentControl::content_property(),
                            Some(value("Start camera".to_string()).expect("a value")),
                        )
                        .set(
                            AutomationProperties::name_property(),
                            Some(String::from("Start camera")),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("CapturePhoto")?
                        .set(
                            ContentControl::content_property(),
                            Some(value("Take photo".to_string()).expect("a value")),
                        )
                        .set(AutomationProperties::name_property(), Some(String::from("Take photo")))
                        .control(),
                    t.make(Button::new())
                        .name("CaptureRecord")?
                        .set(
                            ContentControl::content_property(),
                            Some(value("Record video".to_string()).expect("a value")),
                        )
                        .set(
                            AutomationProperties::name_property(),
                            Some(String::from("Record silent video")),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("CaptureFinish")?
                        .set(
                            ContentControl::content_property(),
                            Some(value("Finish video".to_string()).expect("a value")),
                        )
                        .set(
                            AutomationProperties::name_property(),
                            Some(String::from("Finish video")),
                        )
                        .control(),
                    t.make(Button::new())
                        .name("CaptureCancel")?
                        .set(
                            ContentControl::content_property(),
                            Some(value("Stop / Cancel".to_string()).expect("a value")),
                        )
                        .set(
                            AutomationProperties::name_property(),
                            Some(String::from("Stop camera and cancel capture")),
                        )
                        .control(),
                ])
                .control(),
            t.make(TextBlock::new())
                .name("CaptureStatus")?
                .set(TextBlock::text_wrapping_property(), TextWrapping::Wrap)
                .set(
                    AutomationProperties::name_property(),
                    Some(String::from("Capture status")),
                )
                .control(),
        ])
        .control())
}

/// The default control theme of `CameraCaptureUI`.
fn camera_capture_ui_theme(theme: &StyleBuilder) -> Result {
    theme.target::<CameraCaptureUI>();
    theme.template(camera_capture_ui_theme_template);
    Ok(())
}
