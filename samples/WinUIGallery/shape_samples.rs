//! The Line and Geometry pages of the pinned WinUI Gallery in Rust: the coordinates of the Line
//! example (the Line page has no code-behind upstream) and the code behind of
//! `GeometryPage.xaml.cs`. The Shape page has no code-behind upstream and keeps its x:Bind.
//!
//! WinUI's `X1`, `Y1`, `X2` and `Y2` of a Line are FerroUI's `StartPoint` and `EndPoint`: the four
//! point sliders set them, for the source's TwoWay x:Bind of each coordinate. FerroUI has no
//! TeachingTip: the tips of the Geometry page are the Flyouts of their buttons, which the buttons'
//! clicks open and close as `ShowGeometryButtonClick` toggles `IsOpen`.

use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::media::IImage;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref};
use ferroui_controls::shapes::Line;
use ferroui_controls::{Button, Control, Image, Slider, TextBlock, TopLevel};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "Line" => wire_line(root),
        "Geometry" => wire_geometry(root),
        _ => {}
    }
}

/// The start and end points of the Line example: `X1`, `Y1`, `X2` and `Y2` of the sliders.
fn update_line(root: &Ref<Control>) {
    let value = |name: &str| root.get_control::<Slider>(name).value();
    let line = root.get_control::<Line>("Line1");
    line.set_start_point(Point::new(value("lineSlider1"), value("lineSlider2")));
    line.set_end_point(Point::new(value("lineSlider3"), value("lineSlider4")));
}

fn wire_line(root: &Ref<Control>) {
    update_line(root);
    for name in ["lineSlider1", "lineSlider2", "lineSlider3", "lineSlider4"] {
        let weak_root = root.downgrade();
        root.get_control::<Slider>(name).value_changed(move |_, _| {
            if let Some(root) = weak_root.upgrade() {
                update_line(&root);
            }
        });
    }
}

/// The source's `HeaderImage` of the theme dictionaries: Geometry.dark.png in the Default (dark)
/// theme, Geometry.light.png in the Light theme.
pub(crate) fn header_image_uri(theme: Option<&ThemeVariant>) -> &'static str {
    let light = theme
        .is_some_and(|theme| *theme == ThemeVariant::light() || theme.inherit_variant() == Some(ThemeVariant::light()));
    if light {
        "ferres://WinUIGallery/Assets/Design/Geometry.light.png"
    } else {
        "ferres://WinUIGallery/Assets/Design/Geometry.dark.png"
    }
}

fn update_header_image(root: &Ref<Control>) {
    let uri = header_image_uri(root.actual_theme_variant().as_ref());
    let image = crate::assets::load_bitmap(uri)
        .ok()
        .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>);
    root.get_control::<Image>("HeaderImage").set_source(image);
}

/// The texts of `CopyOverlayResourceToClipboardButton_Click` and
/// `CopyControlResourceToClipboardButton_Click`.
pub(crate) const COPIED_RESOURCES: [(&str, &str); 2] = [
    ("CopyOverlayResourceToClipboardButton", "OverlayCornerRadius"),
    ("CopyControlResourceToClipboardButton", "ControlCornerRadius"),
];

fn wire_geometry(root: &Ref<Control>) {
    update_header_image(root);
    let weak_root = root.downgrade();
    root.actual_theme_variant_changed(move || {
        if let Some(root) = weak_root.upgrade() {
            update_header_image(&root);
        }
    })
    .detach();
    // The copy buttons put the resource's name on the clipboard of the window, and announce it.
    for (name, text) in COPIED_RESOURCES {
        let weak_root = root.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            let Some(root) = weak_root.upgrade() else {
                return;
            };
            let output = root.get_control::<TextBlock>("CopyOutput");
            let Some(clipboard) = TopLevel::get_top_level(Some(&root)).and_then(|host| host.clipboard()) else {
                output.set_text(Some("This window has no clipboard service."));
                return;
            };
            let weak_output = output.downgrade();
            drop(Dispatcher::ui_thread().spawn_local(async move {
                let message = match clipboard.set_text_async(Some(text)).await {
                    Ok(()) => format!("Copied {text} to the clipboard."),
                    Err(error) => format!("The clipboard failed: {error}"),
                };
                if let Some(output) = weak_output.upgrade() {
                    output.set_text(Some(&message));
                }
            }));
        });
    }
}
