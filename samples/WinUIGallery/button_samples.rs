//! The code behind of the pinned Button, RepeatButton and ToggleButton pages in Rust: the click,
//! checked and unchecked handlers of `ButtonPage.xaml.cs`, `RepeatButtonPage.xaml.cs` and
//! `ToggleButtonPage.xaml.cs`. The HyperlinkButton page's handler navigates the gallery
//! (`examples::GALLERY_LINKS`). The controls own input, repetition and checked state; the sample
//! owns only its output.

use ferroui_base::Ref;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Button, Control, Image, RepeatButton, TextBlock};
use std::cell::Cell;
use std::rc::Rc;

/// The image of the source's `Button2`: `/Assets/SampleMedia/Slices.png`.
const SLICES: &str = "ferres://WinUIGallery/Assets/SampleMedia/Slices.png";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "Button" => wire_button(root),
        "RepeatButton" => wire_repeat_button(root),
        "ToggleButton" => wire_toggle_button(root),
        _ => {}
    }
}

/// `Button_Click` of `ButtonPage.xaml.cs`: the clicked button's name picks its output.
fn wire_button(root: &Ref<Control>) {
    let outputs = Rc::new([
        root.get_control::<TextBlock>("Control1Output").downgrade(),
        root.get_control::<TextBlock>("Control2Output").downgrade(),
    ]);
    for name in ["Button1", "Button2"] {
        let outputs = outputs.clone();
        root.get_control::<Button>(name).click(move |sender, _| {
            let Some(name) = sender.name() else { return };
            let output = match name.as_str() {
                "Button1" => &outputs[0],
                "Button2" => &outputs[1],
                _ => return,
            };
            if let Some(output) = output.upgrade() {
                output.set_text(Some(&format!("You clicked: {name}")));
            }
        });
    }
    let image = root.get_control::<Image>("Button2Image");
    match crate::assets::load_bitmap(SLICES) {
        Ok(bitmap) => image.set_source(Some(Rc::new(bitmap))),
        Err(error) => {
            let diagnostic = root.get_control::<TextBlock>("Button2ImageError");
            diagnostic.set_text(Some(&format!("Slices.png: {error}")));
            diagnostic.set_is_visible(true);
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The source's `private static int _clicks`: the count is kept when the page is opened again.
    static REPEAT_CLICKS: Cell<u64> = const { Cell::new(0) };
}

/// `RepeatButton_Click` of `RepeatButtonPage.xaml.cs`. The output is a polite live region, which
/// announces the changed text as the source's `LiveRegionChanged` event does.
fn wire_repeat_button(root: &Ref<Control>) {
    let output = root.get_control::<TextBlock>("RepeatButton1Output").downgrade();
    root.get_control::<RepeatButton>("RepeatButton1").click(move |_, _| {
        let clicks = REPEAT_CLICKS.with(|clicks| {
            clicks.set(clicks.get().saturating_add(1));
            clicks.get()
        });
        if let Some(output) = output.upgrade() {
            output.set_text(Some(&format!("Number of clicks: {clicks}")));
        }
    });
}

/// The constructor and `ToggleButton_Checked`/`ToggleButton_Unchecked` of
/// `ToggleButtonPage.xaml.cs`: the output is "On" while the button is checked, "Off" otherwise.
fn wire_toggle_button(root: &Ref<Control>) {
    let toggle = root.get_control::<ToggleButton>("Toggle1");
    let output = root.get_control::<TextBlock>("Toggle1Output");
    output.set_text(Some(if toggle.is_checked() == Some(true) { "On" } else { "Off" }));
    let weak_toggle = toggle.downgrade();
    let weak_output = output.downgrade();
    toggle.is_checked_changed(move |_, _| {
        let (Some(toggle), Some(output)) = (weak_toggle.upgrade(), weak_output.upgrade()) else {
            return;
        };
        match toggle.is_checked() {
            Some(true) => output.set_text(Some("On")),
            Some(false) => output.set_text(Some("Off")),
            // Neither Checked nor Unchecked is raised for the indeterminate state.
            None => {}
        }
    });
}
