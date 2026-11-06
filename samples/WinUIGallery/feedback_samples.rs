//! Original ProgressRing, RatingControl, InfoBadge and PersonPicture event handlers in Rust.
//! Controls own input, rendering and value semantics; the sample owns only its UI behavior.

use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Brushes, IBrush, SolidColorBrush};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{
    ComboBox, Control, InfoBadge, NavigationView, NavigationViewPaneDisplayMode, NumberBox,
    PersonPicture, ProgressRing, RadioButton, RatingControl, TextBlock, ToggleSwitch,
};
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    wire_progress_rings(root);
    wire_ratings(root);
    wire_badges(root);
    wire_navigation_badge(root);
    wire_person_picture(root);
}

/// `RadioButtons_SelectionChanged` of `PersonPicturePage.xaml.cs`: the checked radio button picks
/// the profile image, the display name or the initials, and clears the other two.
fn wire_person_picture(root: &Ref<Control>) {
    let Some(picture) = find::<PersonPicture>(root, "personPicture") else {
        return;
    };
    let radios: Vec<_> = ["ProfileImageRadio", "DisplayNameRadio", "InitialsRadio"]
        .into_iter()
        .map(|name| root.get_control::<RadioButton>(name).downgrade())
        .collect();
    let update = {
        let picture = picture.downgrade();
        let radios = radios.clone();
        move || {
            let Some(picture) = picture.upgrade() else { return };
            let checked = |index: usize| radios[index].upgrade().is_some_and(|radio| radio.is_checked() == Some(true));
            if checked(0) {
                let image = crate::assets::load_bitmap("ferres://WinUIGallery/Assets/ControlImages/PersonPicture.png")
                    .ok()
                    .map(|bitmap| Rc::new(bitmap) as Rc<dyn ferroui_base::media::IImageBrushSource>);
                picture.set_profile_picture(image);
                picture.set_display_name("");
                picture.set_initials("");
            } else if checked(1) {
                picture.set_profile_picture(None);
                picture.set_display_name("Jane Doe");
                picture.set_initials("");
            } else if checked(2) {
                picture.set_profile_picture(None);
                picture.set_display_name("");
                picture.set_initials("SB");
            }
        }
    };
    update();
    let update = Rc::new(update);
    for radio in radios.iter().filter_map(|radio| radio.upgrade()) {
        let update = update.clone();
        radio.property_changed(move |change| {
            if change.property() == ToggleButton::is_checked_property().as_property() {
                update();
            }
        });
    }
}

fn wire_navigation_badge(root: &Ref<Control>) {
    let Some(nav) = find::<NavigationView>(root, "nvSample1") else {
        return;
    };
    let badge = root.get_control::<InfoBadge>("infoBadge1");
    // In the source, IsOn=True raises Toggled while loading and changes the
    // view model's initial zero opacity to one. Wiring follows native loading.
    badge.set_opacity(1.0);
    let toggle = root.get_control::<ToggleSwitch>("ToggleInfoBadgeOpacity");
    let weak_badge = badge.downgrade();
    toggle.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let Some(badge) = weak_badge.upgrade() {
            badge.set_opacity(if badge.opacity() == 0.0 { 1.0 } else { 0.0 });
        }
    });
    let modes = root.get_control::<ComboBox>("NavigationViewDisplayMode");
    let weak_modes = modes.downgrade();
    let weak_nav = nav.downgrade();
    modes.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(modes), Some(nav)) = (weak_modes.upgrade(), weak_nav.upgrade()) else {
            return;
        };
        let (mode, open) = match modes.selected_index() {
            0 => (NavigationViewPaneDisplayMode::Left, true),
            1 => (NavigationViewPaneDisplayMode::LeftCompact, false),
            2 => (NavigationViewPaneDisplayMode::Top, true),
            _ => return,
        };
        nav.set_pane_display_mode(mode);
        nav.set_is_pane_open(open);
    });
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

fn wire_progress_rings(root: &Ref<Control>) {
    let Some(value) = find::<NumberBox>(root, "ProgressValue") else {
        return;
    };
    let ring = root.get_control::<ProgressRing>("ProgressRing2");
    let weak_ring = ring.downgrade();
    let weak_value = value.downgrade();
    value.value_changed(move |_| {
        if let (Some(value), Some(ring)) = (weak_value.upgrade(), weak_ring.upgrade()) {
            if value.value().is_nan() {
                // NumberBox coalesces a reentrant change: explicitly synchronize the
                // ring after restoring zero instead of relying on a nested event.
                value.set_numeric_value(0.0);
            }
            ring.set_range_value(value.value());
        }
    }).detach();
    for (ring_name, option_name) in [
        ("ProgressRing1", "BackgroundComboBox1"),
        ("ProgressRing2", "BackgroundComboBox2"),
    ] {
        let ring = root.get_control::<ProgressRing>(ring_name);
        ring.set_background(Some(Brushes::transparent()));
        let option = root.get_control::<ComboBox>(option_name);
        let weak_ring = ring.downgrade();
        let weak_option = option.downgrade();
        option.property_changed(move |change| {
            if change.property() != SelectingItemsControl::selected_index_property().as_property() {
                return;
            }
            if let (Some(ring), Some(option)) = (weak_ring.upgrade(), weak_option.upgrade()) {
                let brush: Rc<dyn IBrush> = match option.selected_index() {
                    0 => Brushes::transparent(),
                    1 => SolidColorBrush::from_uint32(0xffd3d3d3).into(),
                    _ => return,
                };
                ring.set_background(Some(brush));
            }
        });
    }
}

fn wire_ratings(root: &Ref<Control>) {
    let Some(rating) = find::<RatingControl>(root, "RatingControl1") else {
        return;
    };
    rating.value_changed(|sender| sender.set_caption("Your rating")).detach();
    let weak_output = root.get_control::<TextBlock>("RatingOutput").downgrade();
    let weak_rating = rating.downgrade();
    rating.property_changed(move |change| {
        if change.property() != RatingControl::value_property().as_property() {
            return;
        }
        if let (Some(rating), Some(output)) = (weak_rating.upgrade(), weak_output.upgrade()) {
            output.set_text(Some(&rating.value().to_string()));
        }
    });
}

fn wire_badges(root: &Ref<Control>) {
    let Some(value) = find::<NumberBox>(root, "ValueNumberBox") else {
        return;
    };
    let weak_badge = root
        .get_control::<InfoBadge>("DynamicInfoBadge")
        .downgrade();
    let weak_value = value.downgrade();
    value.value_changed(move |_| {
        if let (Some(value), Some(badge)) = (weak_value.upgrade(), weak_badge.upgrade()) {
            // Preserve the C# handler's truncation and guard without inventing a
            // notification count for NaN, infinity or an out-of-range conversion.
            let number = value.value();
            if number.is_finite() && number.trunc() >= -1.0 && number <= i32::MAX as f64 {
                badge.set_numeric_value(number as i32);
            }
        }
    }).detach();
    let options = root.get_control::<ComboBox>("InfoBadgeStyleComboBox");
    let weak_root = root.downgrade();
    let weak_options = options.downgrade();
    options.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(root), Some(options)) = (weak_root.upgrade(), weak_options.upgrade()) else {
            return;
        };
        let severity = match options.selected_index() {
            0 => "Attention",
            1 => "Informational",
            2 => "Success",
            3 => "Critical",
            _ => return,
        };
        for (name, kind) in [
            ("StyleIconBadge", "Icon"),
            ("StyleValueBadge", "Value"),
            ("StyleDotBadge", "Dot"),
        ] {
            let key = ResourceKey::from(format!("{severity}{kind}InfoBadgeStyle"));
            if let Some(theme) = root
                .try_find_resource(&key, root.actual_theme_variant().as_ref())
                .flatten()
                .and_then(|value| value.downcast_ref::<Ref<ControlTheme>>().cloned())
            {
                root.get_control::<InfoBadge>(name).set_theme(Some(theme));
            }
        }
    });
}
