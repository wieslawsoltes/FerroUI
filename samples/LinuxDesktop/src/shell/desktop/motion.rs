//! Only the overlay visual moves. Layout, the input seat and client surfaces
//! remain in place while the render thread plays these finite key frames.
use super::{look, shell_panel, Appearance, ColorMode, Layout};
use ferroui_base::controls::ResourceKey;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::numerics::Vector2;
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::rendering::composition::animations::CompositionEasingFunction;
use ferroui_base::rendering::composition::{ElementComposition, ICompositionObjectAnimations};
use ferroui_base::{FerroLocator, LocatorExtensions, Visual};
use ferroui_controls::Border;
use ferroui_view::prelude::*;
use std::time::Duration;

pub(super) fn sheet() -> El<Border> {
    let profile = look();
    let host = NodeRef::<Border>::new();
    let enter = move || {
        if let Some(control) = host.get_untracked().filter(|c| c.is_visible()) {
            animate(&control, profile);
        }
    };
    shell_panel()
        .node_ref(host)
        .on_loaded(move |_| enter())
        .on_changed(Visual::is_visible_property(), move |visible| {
            // Start retains its controls between visits; Loaded only covers
            // the first visit. The other sheets are mounted on demand.
            if *visible {
                enter();
            }
        })
}

fn animate(host: &Border, profile: Appearance) {
    let Some(visual) = ElementComposition::get_element_visual(host) else {
        return;
    };
    if profile
        .winui()
        .is_some_and(|s| s.mode == ColorMode::HighContrast)
        || FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .is_some_and(|settings| !settings.animations_enabled())
    {
        return;
    }
    let (offset, initial_opacity, ms, preference) = match profile.layout() {
        Layout::Taskbar => (14., 0.75, 155, None),
        Layout::MenuBarDock => (0., 0.72, 105, Some("MacOSReduceMotion")),
        Layout::BottomNavigation => (28., 0.86, 200, Some("MaterialReducedMotion")),
    };
    if let Some(key) = preference {
        let value = host
            .try_find_resource(
                &ResourceKey::from(key),
                host.actual_theme_variant().as_ref(),
            )
            .flatten();
        if from_markup_value::<bool>(&value) == Some(true) {
            return;
        }
    }

    let compositor = visual.compositor();
    let easing = CompositionEasingFunction::create_cubic_bezier_easing_function(
        compositor,
        Vector2::new(0.16, 1.0),
        Vector2::new(0.3, 1.0),
    );
    let play = |property, start, end| {
        let animation = compositor.create_scalar_key_frame_animation();
        animation.insert_key_frame(0.0, start);
        animation.insert_key_frame_with_easing(1.0, end, easing.as_rc().clone());
        animation.set_duration(Duration::from_millis(ms));
        visual.start_animation(property, &*animation);
    };
    play("Opacity", initial_opacity, 1.);
    if offset != 0. {
        // Leave the Z translation that drives WinUI theme shadows intact.
        play("Translation.Y", offset, 0.);
    }
}
