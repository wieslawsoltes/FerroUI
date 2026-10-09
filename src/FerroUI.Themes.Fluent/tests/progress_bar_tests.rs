//! Not from upstream: the indeterminate progress bar of the theme. The control theme sizes
//! and moves its two indicators with bindings to the template settings of the progress bar
//! (`{Binding $parent[ProgressBar].TemplateSettings.ContainerWidth}`, and the key frames of
//! the animations of `:indeterminate`), which the progress bar computes from the size of its
//! track.

use super::support::*;
use ferroui_base::animation::TimeSpan;
use ferroui_base::controls::NameScopeRef;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{TransformGroup, TranslateTransform};
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::{Border, Control, ProgressBar, Window};
use std::cell::RefCell;
use std::rc::Rc;

/// The offset the animation of the theme gives the indicator: the `X` of the translation in
/// the render transform the animation creates for the member it animates.
fn translation(indicator: &Border) -> f64 {
    let transform = indicator.render_transform().expect("the animation sets a render transform");
    let group =
        transform.as_object().and_then(|object| object.to_ref().cast::<TransformGroup>()).expect("a transform group");
    let translation =
        group.children().iter().find_map(|child| child.cast::<TranslateTransform>()).expect("a translation in the group");
    translation.x()
}

struct Shown {
    progress_bar: Ref<ProgressBar>,
    first: Ref<Border>,
    second: Ref<Border>,
    // Keeps the progress bar in a shown window.
    _window: Ref<Window>,
}

/// An indeterminate progress bar of `width` in a shown window, with the two indicators of its
/// template.
fn indeterminate_progress_bar(width: f64) -> Shown {
    let progress_bar = ProgressBar::new();
    progress_bar.set_is_indeterminate(true);
    progress_bar.set_width(width);
    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    let _ = progress_bar.template_applied({
        let name_scope = name_scope.clone();
        move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone())
    });
    let window = Window::new();
    window.set_content(Some(Control::boxed(&progress_bar)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let name_scope = name_scope.borrow().clone().expect("the template of the progress bar was applied");
    Shown {
        first: name_scope.get_as::<Border>("IndeterminateProgressBarIndicator"),
        second: name_scope.get_as::<Border>("IndeterminateProgressBarIndicator2"),
        progress_bar,
        _window: window,
    }
}

/// The widths of the two indicators are the ones of the template settings, and follow them
/// when the size of the progress bar changes.
#[test]
fn the_indicators_of_an_indeterminate_progress_bar_have_the_widths_of_the_template_settings() {
    let _app = start_themed_application();
    let shown = indeterminate_progress_bar(300.0);
    let settings = shown.progress_bar.template_settings();

    assert_eq!(120.0, settings.container_width());
    assert_eq!(180.0, settings.container2_width());
    assert_eq!(settings.container_width(), shown.first.width());
    assert_eq!(settings.container2_width(), shown.second.width());

    shown.progress_bar.set_width(500.0);
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(200.0, settings.container_width());
    assert_eq!(300.0, settings.container2_width());
    assert_eq!(settings.container_width(), shown.first.width());
    assert_eq!(settings.container2_width(), shown.second.width());
}

/// The key frames of the animations of `:indeterminate` are bound to the positions of the
/// template settings: the indicators move between them, and between the new positions after
/// the size of the progress bar changed while the animations run.
#[test]
fn the_indeterminate_animations_follow_the_template_settings_after_a_size_change() {
    let clock = Rc::new(MockGlobalClock::new());
    let _app = start_themed_application_with_clock(clock.clone());
    let shown = indeterminate_progress_bar(300.0);
    let settings = shown.progress_bar.template_settings();
    let seconds = |seconds: f64| TimeSpan::from_milliseconds(seconds * 1000.0);

    // The animations take two seconds and repeat. The first indicator is at its start position
    // at the start and at its end position from 1.5 seconds on; the second one is at its start
    // position until 0.75 seconds and at its end position at the end.
    clock.pulse(seconds(0.0));
    assert_eq!(120.0 * -1.8, settings.container_animation_start_position());
    assert_eq!(settings.container_animation_start_position(), translation(&shown.first));
    assert_eq!(settings.container2_animation_start_position(), translation(&shown.second));
    clock.pulse(seconds(0.5));
    assert_eq!(settings.container2_animation_start_position(), translation(&shown.second));
    clock.pulse(seconds(1.75));
    assert_eq!(120.0 * 3.0, settings.container_animation_end_position());
    assert_eq!(settings.container_animation_end_position(), translation(&shown.first));

    shown.progress_bar.set_width(500.0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(200.0 * 3.0, settings.container_animation_end_position());
    assert_eq!(300.0 * -1.5, settings.container2_animation_start_position());

    // The same moments of the next iteration, with the positions of the new size.
    clock.pulse(seconds(2.5));
    assert_eq!(settings.container2_animation_start_position(), translation(&shown.second));
    clock.pulse(seconds(3.75));
    assert_eq!(settings.container_animation_end_position(), translation(&shown.first));
    // Between two key frames the indicator is between their positions.
    clock.pulse(seconds(4.75));
    let (start, end) = (settings.container_animation_start_position(), settings.container_animation_end_position());
    let offset = translation(&shown.first);
    assert!(start < offset && offset < end, "{start} < {offset} < {end}");
}
