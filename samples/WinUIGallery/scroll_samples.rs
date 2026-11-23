//! The code behind of the pinned ScrollViewer page in Rust: `ScrollViewerPage.xaml.cs`.
//!
//! FerroUI's ScrollViewer has no `HorizontalScrollMode` or `VerticalScrollMode`: a Disabled scroll
//! mode is the bar's Disabled visibility, which turns the scrolling of the axis off, and Enabled and
//! Auto keep the visibility of the bar's box. WinUI's `ViewChanged` (a view that is not
//! intermediate) is a change of `ZoomFactor`.

use ferroui_base::media::IImage;
use ferroui_base::Ref;
use ferroui_controls::primitives::{ScrollBarVisibility, SelectingItemsControl};
use ferroui_controls::{ComboBox, Control, Image, ScrollViewer, Slider, ZoomMode};
use std::cell::Cell;
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "ScrollViewer" {
        wire_scroll_viewer(root);
    }
}

/// `ScrollBarVisibility` in the order of the boxes (WinUI's values: Disabled, Auto, Hidden,
/// Visible).
pub(crate) const BAR_VISIBILITIES: [ScrollBarVisibility; 4] = [
    ScrollBarVisibility::Disabled,
    ScrollBarVisibility::Auto,
    ScrollBarVisibility::Hidden,
    ScrollBarVisibility::Visible,
];

/// The visibility of an axis' bar for its scroll mode (Disabled, Enabled, Auto) and the index of its
/// visibility box.
pub(crate) fn bar_visibility(scroll_mode: i32, visibility: i32) -> ScrollBarVisibility {
    if scroll_mode == 0 {
        return ScrollBarVisibility::Disabled;
    }
    usize::try_from(visibility)
        .ok()
        .and_then(|index| BAR_VISIBILITIES.get(index).copied())
        .unwrap_or(ScrollBarVisibility::Auto)
}

/// `hsmCombo_SelectionChanged`, `hsbvCombo_SelectionChanged`, `vsmCombo_SelectionChanged` and
/// `vsbvCombo_SelectionChanged`: the visibilities of the two bars.
fn update_bars(root: &Ref<Control>) {
    let index = |name: &str| root.get_control::<ComboBox>(name).selected_index();
    let viewer = root.get_control::<ScrollViewer>("ScrollViewerControl");
    viewer.set_horizontal_scroll_bar_visibility(bar_visibility(index("hsmCombo"), index("hsbvCombo")));
    viewer.set_vertical_scroll_bar_visibility(bar_visibility(index("vsmCombo"), index("vsbvCombo")));
}

/// `ZoomModeComboBox_SelectionChanged`: the zoom mode, the slider enabled only while zooming is, and
/// the factor 2 when it is not.
fn zoom_mode_changed(root: &Ref<Control>) {
    let index = root.get_control::<ComboBox>("zoomCombo").selected_index();
    let viewer = root.get_control::<ScrollViewer>("ScrollViewerControl");
    let slider = root.get_control::<Slider>("ZoomSlider");
    viewer.set_zoom_mode(if index == 1 {
        ZoomMode::Enabled
    } else {
        ZoomMode::Disabled
    });
    slider.set_is_enabled(index == 1);
    if !slider.is_enabled() {
        viewer.zoom_to_factor(2.0);
    }
}

fn on_selection(root: &Ref<Control>, name: &str, handler: fn(&Ref<Control>)) {
    let weak_root = root.downgrade();
    root.get_control::<ComboBox>(name).property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let Some(root) = weak_root.upgrade() {
            handler(&root);
        }
    });
}

fn wire_scroll_viewer(root: &Ref<Control>) {
    let image = crate::assets::load_bitmap("ferres://WinUIGallery/Assets/SampleMedia/cliff.jpg")
        .ok()
        .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>);
    root.get_control::<Image>("CliffImage").set_source(image);
    let viewer = root.get_control::<ScrollViewer>("ScrollViewerControl");
    // The constructor's ZoomToFactor(4), once the scroll viewer has its presenter (when it is first
    // loaded).
    let zoomed = Cell::new(false);
    let weak_viewer = viewer.downgrade();
    viewer.loaded(move |_, _| {
        if let Some(viewer) = weak_viewer.upgrade() {
            if !zoomed.replace(true) {
                viewer.zoom_to_factor(4.0);
            }
        }
    });
    update_bars(root);
    on_selection(root, "zoomCombo", zoom_mode_changed);
    for name in ["hsmCombo", "hsbvCombo", "vsmCombo", "vsbvCombo"] {
        on_selection(root, name, update_bars);
    }
    // `ZoomSlider_ValueChanged`.
    let weak_viewer = viewer.downgrade();
    root.get_control::<Slider>("ZoomSlider").value_changed(move |_, args| {
        if let Some(viewer) = weak_viewer.upgrade() {
            viewer.change_view(None, None, Some(args.new_value() as f32));
        }
    });
    // `ScrollViewerControl_ViewChanged`: the slider follows the zoom factor.
    let weak_viewer = viewer.downgrade();
    let weak_slider = root.get_control::<Slider>("ZoomSlider").downgrade();
    viewer.property_changed(move |change| {
        if change.property() != ScrollViewer::zoom_factor_property().as_property() {
            return;
        }
        if let (Some(viewer), Some(slider)) = (weak_viewer.upgrade(), weak_slider.upgrade()) {
            slider.set_range_value(f64::from(viewer.zoom_factor()));
        }
    });
}
