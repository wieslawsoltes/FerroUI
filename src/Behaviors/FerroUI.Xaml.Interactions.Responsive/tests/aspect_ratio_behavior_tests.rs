//! Not from the reference: `AspectRatioBehavior` adds the class of a setter while the ratio of the
//! width to the height of the bounds meets the ratio conditions of the setter, and removes it
//! otherwise.

use super::support::{object, start_app, Host};
use crate::{AspectRatioBehavior, AspectRatioClassSetter};
use ferroui_base::Ref;
use ferroui_xaml_interactivity::ComparisonConditionType;

/// A setter of the class `class_name`.
fn setter(class_name: &str) -> Ref<AspectRatioClassSetter> {
    let setter = AspectRatioClassSetter::new();
    setter.set_class_name(Some(class_name.to_string()));
    setter
}

/// A behavior with `setters`.
fn behavior(setters: &[&Ref<AspectRatioClassSetter>]) -> Ref<AspectRatioBehavior> {
    let behavior = AspectRatioBehavior::new();
    for setter in setters {
        behavior.setters().add((*setter).clone());
    }
    behavior
}

/// The landscape (ratio of at least 1) and portrait (ratio under 1) setters.
fn landscape_and_portrait() -> (Ref<AspectRatioClassSetter>, Ref<AspectRatioClassSetter>) {
    let landscape = setter("landscape");
    landscape.set_min_ratio(1.0);
    let portrait = setter("portrait");
    portrait.set_max_ratio(1.0);
    (landscape, portrait)
}

#[test]
fn setter_defaults_are_those_of_the_reference() {
    let _app = start_app();
    let setter = AspectRatioClassSetter::new();

    assert_eq!(0.0, setter.min_ratio());
    assert_eq!(ComparisonConditionType::GreaterThanOrEqual, setter.min_ratio_operator());
    assert_eq!(f64::INFINITY, setter.max_ratio());
    assert_eq!(ComparisonConditionType::LessThan, setter.max_ratio_operator());
    assert_eq!(None, setter.class_name());
    assert!(!setter.is_pseudo_class());
    assert!(setter.target_control().is_none());
}

#[test]
fn ratio_conditions_switch_the_classes() {
    let _app = start_app();
    let host = Host::new();
    let (landscape, portrait) = landscape_and_portrait();
    host.show_with(object(&behavior(&[&landscape, &portrait])));

    host.resize(200.0, 100.0);
    assert!(host.has("landscape"));
    assert!(!host.has("portrait"));

    host.resize(100.0, 200.0);
    assert!(!host.has("landscape"));
    assert!(host.has("portrait"));

    // A square: the minimum is inclusive and the maximum exclusive by default.
    host.resize(100.0, 100.0);
    assert!(host.has("landscape"));
    assert!(!host.has("portrait"));

    host.window.close();
}

#[test]
fn zero_height_is_an_infinite_ratio() {
    let _app = start_app();
    let host = Host::new();
    let (landscape, portrait) = landscape_and_portrait();
    host.show_with(object(&behavior(&[&landscape, &portrait])));

    host.resize(200.0, 100.0);
    assert!(host.has("landscape"));

    // The ratio is infinite, which is not under the default maximum (infinity, exclusive).
    host.resize(200.0, 0.0);
    assert!(!host.has("landscape"));
    assert!(!host.has("portrait"));

    host.window.close();
}

#[test]
fn operators_select_the_comparison() {
    let _app = start_app();
    let host = Host::new();
    let square = setter("square");
    square.set_min_ratio(1.0);
    square.set_max_ratio(1.0);
    square.set_max_ratio_operator(ComparisonConditionType::LessThanOrEqual);
    let wider = setter("wider");
    wider.set_min_ratio(1.0);
    wider.set_min_ratio_operator(ComparisonConditionType::GreaterThan);
    host.show_with(object(&behavior(&[&square, &wider])));

    host.resize(100.0, 100.0);
    assert!(host.has("square"));
    assert!(!host.has("wider"));

    host.resize(150.0, 100.0);
    assert!(!host.has("square"));
    assert!(host.has("wider"));

    host.window.close();
}

#[test]
fn pseudo_classes_and_target_controls() {
    let _app = start_app();
    let host = Host::new();
    let landscape = setter(":landscape");
    landscape.set_min_ratio(1.0);
    landscape.set_is_pseudo_class(true);
    let on_border = setter("on-border");
    on_border.set_min_ratio(1.0);
    on_border.set_target_control(&host.border);
    let behavior = behavior(&[&landscape, &on_border]);
    behavior.set_target_control(&host.other);
    host.show_with(object(&behavior));

    host.resize(200.0, 100.0);

    assert!(host.other_has(":landscape"));
    assert!(!host.has(":landscape"));
    assert!(host.has("on-border"));
    assert!(!host.other_has("on-border"));

    host.resize(100.0, 200.0);

    assert!(!host.other_has(":landscape"));
    assert!(!host.has("on-border"));

    host.window.close();
}

#[test]
fn bounds_of_the_source_control_are_observed() {
    let _app = start_app();
    let host = Host::new();
    let (landscape, _) = landscape_and_portrait();
    let behavior = behavior(&[&landscape]);
    behavior.set_source_control(&host.other);
    host.show_with(object(&behavior));

    host.resize_other(100.0, 200.0);
    assert!(!host.has("landscape"));

    host.resize_other(200.0, 100.0);
    assert!(host.has("landscape"));

    // The bounds of the associated object are not observed.
    host.resize(100.0, 200.0);
    assert!(host.has("landscape"));

    host.window.close();
}

#[test]
fn bounds_are_not_observed_once_detached_from_the_visual_tree() {
    let _app = start_app();
    let host = Host::new();
    let (landscape, _) = landscape_and_portrait();
    host.show_with(object(&behavior(&[&landscape])));

    host.resize(200.0, 100.0);
    assert!(host.has("landscape"));

    host.detach_border();
    host.resize(100.0, 200.0);

    assert!(host.has("landscape"));

    host.window.close();
}
