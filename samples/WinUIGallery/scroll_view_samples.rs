//! The code behind of the pinned ScrollView page in Rust: `ScrollViewPage.xaml.cs`.
//!
//! The NumberBox of the zoom factor keeps FerroUI's formatting (the source sets a DecimalFormatter
//! with one fraction digit); the C# snippets of the third example (`Example3.CSharp`, read from
//! `Samples/SampleCode/ScrollView`) are not shown, as the gallery shows no code.

use ferroui_base::media::IImage;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    Button, ComboBox, Control, Image, NumberBox, ScrollView, ScrollingAnimationMode, ScrollingScrollBarVisibility,
    ScrollingScrollMode, ScrollingScrollOptions, ScrollingSnapPointsMode, ScrollingZoomMode, ScrollingZoomOptions,
    StackPanel,
};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The images of the second example, from the top.
pub(crate) const EXAMPLE2_IMAGES: &[(&str, &str)] = &[
    ("grapes", "grapes.jpg"),
    ("rainier", "rainier.jpg"),
    ("sunset", "sunset.jpg"),
    ("treetops", "treetops.jpg"),
    ("valley", "valley.jpg"),
    ("cliff", "cliff.jpg"),
];

/// The images of the third example, from the top.
pub(crate) const EXAMPLE3_IMAGES: &[(&str, &str)] = &[
    ("leaves", "LandscapeImage1.jpg"),
    ("carousel", "LandscapeImage2.jpg"),
    ("bicycles", "LandscapeImage3.jpg"),
    ("pond", "LandscapeImage4.jpg"),
    ("marina", "LandscapeImage5.jpg"),
    ("beach", "LandscapeImage6.jpg"),
    ("rampart", "LandscapeImage7.jpg"),
    ("mountain", "LandscapeImage8.jpg"),
];

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "ScrollView" {
        wire_scroll_view(root);
    }
}

fn sample_media(file: &str) -> Option<Rc<dyn IImage>> {
    crate::assets::load_bitmap(&format!("ferres://WinUIGallery/Assets/SampleMedia/{file}"))
        .ok()
        .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>)
}

fn fill_images(panel: &Ref<StackPanel>, images: &[(&str, &str)]) {
    for (name, file) in images {
        let image = Image::new();
        AutomationProperties::set_name(&image, Some(name));
        image.set_stretch(ferroui_base::media::Stretch::Uniform);
        image.set_source(sample_media(file));
        panel.children().add(image.upcast::<Control>());
    }
}

/// Calls `handler` with the selected index of the combo box `name` when it changes.
fn on_selection(root: &Ref<Control>, name: &str, handler: impl Fn(&Ref<Control>, i32) + 'static) {
    let weak_root = root.downgrade();
    let combo = root.get_control::<ComboBox>(name);
    let weak_combo = combo.downgrade();
    combo.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(root), Some(combo)) = (weak_root.upgrade(), weak_combo.upgrade()) {
            handler(&root, combo.selected_index());
        }
    });
}

/// `(ScrollingScrollMode)cmb.SelectedIndex`.
pub(crate) fn scroll_mode(index: i32) -> ScrollingScrollMode {
    match index {
        0 => ScrollingScrollMode::Enabled,
        1 => ScrollingScrollMode::Disabled,
        _ => ScrollingScrollMode::Auto,
    }
}

/// `(ScrollingScrollBarVisibility)cmb.SelectedIndex`.
pub(crate) fn scroll_bar_visibility(index: i32) -> ScrollingScrollBarVisibility {
    match index {
        1 => ScrollingScrollBarVisibility::Visible,
        2 => ScrollingScrollBarVisibility::Hidden,
        _ => ScrollingScrollBarVisibility::Auto,
    }
}

/// `NbVerticalVelocity_ValueChanged`: the constant velocity that a new value of the box asks for,
/// 30 or -30 when the value moves into the dead range between them (toward the end that the view
/// is not at).
pub(crate) fn constant_velocity(new_value: f64, old_value: f64, offset: f64, scrollable_height: f64) -> f32 {
    let mut velocity = new_value as f32;
    if (-30.0..=30.0).contains(&new_value) {
        velocity = if new_value < old_value {
            if offset == 0.0 { 30.0 } else { -30.0 }
        } else if offset == scrollable_height {
            -30.0
        } else {
            30.0
        };
    } else if new_value < 30.0 && offset == 0.0 {
        velocity = 30.0;
    } else if new_value > 30.0 && offset == scrollable_height {
        velocity = -30.0;
    }
    velocity
}

/// `GetTargetVerticalOffset`: a fifth of the scrollable height from the end the view is closer to.
pub(crate) fn target_vertical_offset(offset: f64, scrollable_height: f64) -> f64 {
    if offset > scrollable_height / 2.0 {
        scrollable_height / 5.0
    } else {
        4.0 * scrollable_height / 5.0
    }
}

fn wire_scroll_view(root: &Ref<Control>) {
    root.get_control::<Image>("cliffImage").set_source(sample_media("cliff.jpg"));
    fill_images(&root.get_control::<StackPanel>("scrollView2Images"), EXAMPLE2_IMAGES);
    fill_images(&root.get_control::<StackPanel>("scrollView3Images"), EXAMPLE3_IMAGES);

    // ScrollViewPage_Loaded: an animated zoom to 4, once.
    let first = root.get_control::<ScrollView>("scrollView1");
    let zoomed = Cell::new(false);
    let weak = first.downgrade();
    first.loaded(move |_, _| {
        if let Some(view) = weak.upgrade() {
            if !zoomed.replace(true) {
                view.zoom_to_with(
                    4.0,
                    None,
                    ScrollingZoomOptions::with_snap_points_mode(ScrollingAnimationMode::Enabled, ScrollingSnapPointsMode::Ignore),
                );
            }
        }
    });
    on_selection(root, "cmbZoomMode", |root, index| {
        let mode = if index == 0 { ScrollingZoomMode::Enabled } else { ScrollingZoomMode::Disabled };
        root.get_control::<ScrollView>("scrollView1").set_zoom_mode(mode);
    });
    let weak = first.downgrade();
    root.get_control::<NumberBox>("nbZoomFactor")
        .value_changed(move |args| {
            if let Some(view) = weak.upgrade() {
                view.zoom_to(args.new_value() as f32, None);
            }
        })
        .detach();
    on_selection(root, "cmbHorizontalScrollMode", |root, index| {
        root.get_control::<ScrollView>("scrollView1").set_horizontal_scroll_mode(scroll_mode(index));
    });
    on_selection(root, "cmbVerticalScrollMode", |root, index| {
        root.get_control::<ScrollView>("scrollView1").set_vertical_scroll_mode(scroll_mode(index));
    });
    on_selection(root, "cmbHorizontalScrollBarVisibility", |root, index| {
        root.get_control::<ScrollView>("scrollView1").set_horizontal_scroll_bar_visibility(scroll_bar_visibility(index));
    });
    on_selection(root, "cmbVerticalScrollBarVisibility", |root, index| {
        root.get_control::<ScrollView>("scrollView1").set_vertical_scroll_bar_visibility(scroll_bar_visibility(index));
    });

    // NbVerticalVelocity_ValueChanged: stop, then scroll at the constant velocity of the box.
    let second = root.get_control::<ScrollView>("scrollView2");
    let velocity_box = root.get_control::<NumberBox>("nbVerticalVelocity");
    let weak_view = second.downgrade();
    let weak_box = velocity_box.downgrade();
    velocity_box
        .value_changed(move |args| {
            if args.old_value().is_nan() {
                return;
            }
            let (Some(view), Some(velocity_box)) = (weak_view.upgrade(), weak_box.upgrade()) else { return };
            view.scroll_by_with(
                0.0,
                0.0,
                ScrollingScrollOptions::with_snap_points_mode(ScrollingAnimationMode::Disabled, ScrollingSnapPointsMode::Ignore),
            );
            let velocity =
                constant_velocity(args.new_value(), args.old_value(), view.vertical_offset(), view.scrollable_height());
            velocity_box.set_numeric_value(f64::from(velocity));
            // An empty inertia decay rate for a constant velocity.
            view.add_scroll_velocity(Vector2::new(0.0, velocity), Some(Vector2::new(0.0, 0.0)));
        })
        .detach();

    // BtnScrollWithAnimation_Click and ScrollView_ScrollAnimationStarting.
    let third = root.get_control::<ScrollView>("scrollView3");
    let weak = third.downgrade();
    root.get_control::<Button>("btnScrollWithAnimation").click(move |_, _| {
        if let Some(view) = weak.upgrade() {
            view.scroll_to_with(
                view.horizontal_offset(),
                target_vertical_offset(view.vertical_offset(), view.scrollable_height()),
                ScrollingScrollOptions::with_snap_points_mode(ScrollingAnimationMode::Enabled, ScrollingSnapPointsMode::Ignore),
            );
        }
    });
    let weak_root = root.downgrade();
    third
        .scroll_animation_starting(move |view, args| {
            let (Some(root), Some(stock)) = (weak_root.upgrade(), args.animation()) else { return };
            let duration = Duration::from_secs_f64(root.get_control::<NumberBox>("nbAnimationDuration").value() / 1000.0);
            let kind = root.get_control::<ComboBox>("cmbVerticalAnimation").selected_index();
            if kind == 0 {
                stock.set_duration(duration);
                return;
            }
            let target = target_vertical_offset(view.vertical_offset(), view.scrollable_height());
            let target_position = target as f32;
            let horizontal = view.horizontal_offset() as f32;
            let compositor = stock.compositor();
            let custom = compositor.create_vector3_key_frame_animation();
            if kind == 1 {
                // Accordion: three overshoots of a tenth of the distance, each half the one before.
                let mut delta = 0.1 * (target - view.vertical_offset()) as f32;
                for step in 0..3 {
                    custom.insert_key_frame(
                        1.0 - (0.4 / 2f32.powi(step)),
                        Vector3::new(horizontal, target_position + delta, 0.0),
                    );
                    delta /= -2.0;
                }
                custom.insert_key_frame(1.0, Vector3::new(horizontal, target_position, 0.0));
            } else {
                // Teleportation: ease in over 90% of the distance, jump, ease out over the rest.
                let delta = (target - view.vertical_offset()) as f32;
                let start = compositor.create_cubic_bezier_easing_function(Vector2::new(1.0, 0.0), Vector2::new(1.0, 0.0));
                let step = compositor.create_step_easing_function_with_step_count(1);
                let end = compositor.create_cubic_bezier_easing_function(Vector2::new(0.0, 1.0), Vector2::new(0.0, 1.0));
                custom.insert_key_frame_with_easing(
                    0.499999,
                    Vector3::new(horizontal, target_position - 0.9 * delta, 0.0),
                    start,
                );
                custom.insert_key_frame_with_easing(0.5, Vector3::new(horizontal, target_position - 0.1 * delta, 0.0), step);
                custom.insert_key_frame_with_easing(1.0, Vector3::new(horizontal, target_position, 0.0), end);
            }
            custom.set_duration(duration);
            args.set_animation(Some(custom));
        })
        .detach();
}
