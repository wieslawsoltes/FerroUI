//! The code behind of the pinned Border, Canvas and StackPanel pages in Rust: the handlers of
//! `BorderPage.xaml.cs`, `CanvasPage.xaml.cs` and `StackPanelPage.xaml.cs`. The Grid and
//! RelativePanel pages have no code-behind upstream; their options are x:Bind of the markup. The
//! panels own layout; the sample owns only its options.

use ferroui_base::layout::Orientation;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{RangeBase, ToggleButton};
use ferroui_controls::{Border, Control, RadioButton, Slider, StackPanel};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "Border" => wire_border(root),
        "Canvas" => wire_canvas(root),
        "StackPanel" => wire_stack_panel(root),
        _ => {}
    }
}

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

/// The color names of the Border page's options.
pub(crate) const BORDER_COLORS: [&str; 4] = ["Green", "Yellow", "Blue", "White"];

/// `ThicknessSlider_ValueChanged`, `BGRadioButton_Checked` and `RadioButton_Checked` of
/// `BorderPage.xaml.cs`: the slider sets the border's thickness, the Background options its
/// background (Green, Yellow, Blue, White) and the BorderBrush options its border (DarkGreen,
/// Gold, DarkBlue, White).
fn wire_border(root: &Ref<Control>) {
    let border = root.get_control::<Border>("Control1");
    let slider = root.get_control::<Slider>("ThicknessSlider");
    let weak_border = border.downgrade();
    let weak_slider = slider.downgrade();
    slider.property_changed(move |change| {
        if change.property() != RangeBase::value_property().as_property() {
            return;
        }
        if let (Some(border), Some(slider)) = (weak_border.upgrade(), weak_slider.upgrade()) {
            let value = slider.value();
            border.set_border_thickness(Thickness::new(value, value, value, value));
        }
    });
    for (group, background) in [("Background", true), ("BorderBrush", false)] {
        for name in BORDER_COLORS {
            let radio = root.get_control::<RadioButton>(&format!("{group}{name}"));
            let weak_radio = radio.downgrade();
            let weak_border = border.downgrade();
            radio.property_changed(move |change| {
                if change.property() != ToggleButton::is_checked_property().as_property() {
                    return;
                }
                let (Some(radio), Some(border)) = (weak_radio.upgrade(), weak_border.upgrade())
                else {
                    return;
                };
                if radio.is_checked() != Some(true) {
                    return;
                }
                let color = match (background, name) {
                    (true, "Yellow") => Colors::YELLOW,
                    (true, "Green") => Colors::GREEN,
                    (true, "Blue") => Colors::BLUE,
                    (false, "Yellow") => Colors::GOLD,
                    (false, "Green") => Colors::DARK_GREEN,
                    (false, "Blue") => Colors::DARK_BLUE,
                    _ => Colors::WHITE,
                };
                if background {
                    border.set_background(Some(solid(color)));
                } else {
                    border.set_border_brush(Some(solid(color)));
                }
            });
        }
    }
}

/// The automation name `UpdateZSliderAutomationName` of `CanvasPage.xaml.cs` gives the z-index
/// slider.
pub(crate) fn z_slider_automation_name(slider: &Slider) -> String {
    format!(
        "Canvas.ZIndex value {} of range {} to {}",
        slider.value() as i32,
        slider.minimum() as i32,
        slider.maximum() as i32
    )
}

/// The constructor and `ZSlider_ValueChanged` of `CanvasPage.xaml.cs`: the z-index slider's
/// automation name states its value and range.
fn wire_canvas(root: &Ref<Control>) {
    let slider = root.get_control::<Slider>("ZSlider");
    AutomationProperties::set_name(&slider, Some(z_slider_automation_name(&slider).as_str()));
    let weak_slider = slider.downgrade();
    slider.property_changed(move |change| {
        if change.property() != RangeBase::value_property().as_property() {
            return;
        }
        if let Some(slider) = weak_slider.upgrade() {
            AutomationProperties::set_name(
                &slider,
                Some(z_slider_automation_name(&slider).as_str()),
            );
        }
    });
}

/// `OrientationGroup_SelectionChanged` of `StackPanelPage.xaml.cs`: the Tag of the checked option
/// is the orientation of the panel.
fn wire_stack_panel(root: &Ref<Control>) {
    let panel = root.get_control::<StackPanel>("Control1");
    for name in ["HorizontalOrientation", "VerticalOrientation"] {
        let radio = root.get_control::<RadioButton>(name);
        let weak_radio = radio.downgrade();
        let weak_panel = panel.downgrade();
        radio.property_changed(move |change| {
            if change.property() != ToggleButton::is_checked_property().as_property() {
                return;
            }
            let (Some(radio), Some(panel)) = (weak_radio.upgrade(), weak_panel.upgrade()) else {
                return;
            };
            if radio.is_checked() != Some(true) {
                return;
            }
            let tag = radio.tag();
            match tag
                .as_ref()
                .and_then(|tag| tag.downcast_ref::<String>())
                .map(String::as_str)
            {
                Some("Horizontal") => panel.set_orientation(Orientation::Horizontal),
                Some("Vertical") => panel.set_orientation(Orientation::Vertical),
                _ => {}
            }
        });
    }
}
