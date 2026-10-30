//! Ported from `tests/Xaml.Behaviors.Animations.UnitTests/SelectingItemsControlAnimationTests.cs`.
//!
//! Adapted: the windows are those of [`CompositionApp`], and the content controls of the
//! selection tests get [`content_control_template`] (the reference finds the indicators of its
//! content controls among their visual descendants). `Assert.Throws<ArgumentOutOfRangeException>`
//! checks for the panic that stands for the exception.

use super::support::{content_control_template, control, panics, start_app, CompositionApp};
use crate::{SelectingItemsControlBehavior, SelectionIndicatorAnimation};
use ferroui_base::animation::TimeSpan;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::{Ref, Visual};
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControl};
use ferroui_controls::{Border, Control, ContentControl, ListBox, StackPanel};

/// The control as a visual.
fn visual(element: &Control) -> Option<&Visual> {
    let visual: &Visual = element;
    Some(visual)
}

/// `new Border { Name = name, Width = 4, Height = 30 }`.
fn indicator(name: Option<&str>) -> Ref<Border> {
    let indicator = Border::new();
    indicator.set_name(name.map(str::to_string));
    indicator.set_width(4.0);
    indicator.set_height(30.0);
    indicator
}

/// `new ContentControl { Content = content, Height = 40 }`, with a template.
fn selection(content: &Ref<Border>) -> Ref<ContentControl> {
    let selection = ContentControl::new();
    selection.set_template(Some(content_control_template()));
    selection.set_content(Some(Control::boxed(content.clone())));
    selection.set_height(40.0);
    selection
}

#[test]
fn enable_selection_animation_attached_property_round_trips() {
    let _app = start_app();
    let target = ListBox::new();
    let target: &SelectingItemsControl = &target;

    SelectingItemsControlBehavior::set_enable_selection_animation(target, true);

    assert!(SelectingItemsControlBehavior::get_enable_selection_animation(target));
}

#[test]
fn selection_indicator_animation_installs_implicit_animation_for_templated_containers() {
    let app = CompositionApp::start();
    let old_indicator = indicator(Some("PART_SelectedPipe"));
    let new_indicator = indicator(Some("PART_SelectedPipe"));
    let old_selection = selection(&old_indicator);
    let new_selection = selection(&new_indicator);
    let panel = StackPanel::new();
    panel.children().add(old_selection.clone());
    panel.children().add(new_selection.clone());
    let window = app.window(&panel.clone().upcast());

    window.show();
    app.run_jobs();

    let new_templated: &TemplatedControl = &new_selection;
    let old_templated: &TemplatedControl = &old_selection;
    let started = SelectionIndicatorAnimation::try_start(
        Some(new_templated),
        Some(old_templated),
        SelectionIndicatorAnimation::default_duration(),
    );

    assert!(started);
    assert!(ElementComposition::get_element_visual(&new_indicator)
        .and_then(|visual| visual.implicit_animations())
        .is_some());
    window.close();
}

#[test]
fn selection_indicator_animation_rejects_negative_duration_and_missing_visuals() {
    let _app = start_app();
    assert_eq!(SelectionIndicatorAnimation::default_duration(), TimeSpan::from_milliseconds(250.0));
    let first = ContentControl::new();
    let second = ContentControl::new();
    let first: &TemplatedControl = &first;
    let second: &TemplatedControl = &second;
    assert!(!SelectionIndicatorAnimation::try_start(Some(first), Some(second), TimeSpan::ZERO));
    assert!(panics(|| {
        SelectionIndicatorAnimation::try_start(Some(first), Some(second), TimeSpan::from_milliseconds(-1.0));
    }));
}

#[test]
fn selection_indicator_animation_returns_false_for_zero_duration_with_available_visuals() {
    let app = CompositionApp::start();
    let old_indicator = indicator(None);
    let new_indicator = indicator(None);
    let old_selection = selection(&old_indicator);
    let new_selection = selection(&new_indicator);
    let panel = StackPanel::new();
    panel.children().add(old_selection.clone());
    panel.children().add(new_selection.clone());
    let window = app.window(&panel.clone().upcast());
    window.show();
    app.run_jobs();

    assert!(!SelectionIndicatorAnimation::try_start_with_visuals(
        visual(control(&new_indicator)),
        visual(control(&old_indicator)),
        visual(control(&new_selection)),
        visual(control(&old_selection)),
        TimeSpan::ZERO,
    ));
    window.close();
}
