//! Not from the reference: `AdaptiveBehavior` adds the class of a setter while the bounds meet the
//! width and height conditions of the setter, and removes it otherwise.

use super::support::{object, start_app, Host};
use crate::{AdaptiveBehavior, AdaptiveClassSetter};
use ferroui_base::Ref;
use ferroui_xaml_interactivity::ComparisonConditionType;

/// A setter of the class `class_name`.
fn setter(class_name: &str) -> Ref<AdaptiveClassSetter> {
    let setter = AdaptiveClassSetter::new();
    setter.set_class_name(Some(class_name.to_string()));
    setter
}

/// A behavior with `setters`.
fn behavior(setters: &[&Ref<AdaptiveClassSetter>]) -> Ref<AdaptiveBehavior> {
    let behavior = AdaptiveBehavior::new();
    for setter in setters {
        behavior.setters().add((*setter).clone());
    }
    behavior
}

#[test]
fn setter_defaults_are_those_of_the_reference() {
    let _app = start_app();
    let setter = AdaptiveClassSetter::new();

    assert_eq!(0.0, setter.min_width());
    assert_eq!(ComparisonConditionType::GreaterThanOrEqual, setter.min_width_operator());
    assert_eq!(f64::INFINITY, setter.max_width());
    assert_eq!(ComparisonConditionType::LessThan, setter.max_width_operator());
    assert_eq!(0.0, setter.min_height());
    assert_eq!(ComparisonConditionType::GreaterThanOrEqual, setter.min_height_operator());
    assert_eq!(f64::INFINITY, setter.max_height());
    assert_eq!(ComparisonConditionType::LessThan, setter.max_height_operator());
    assert_eq!(None, setter.class_name());
    assert!(!setter.is_pseudo_class());
    assert!(setter.target_control().is_none());
}

#[test]
fn width_conditions_switch_the_classes() {
    let _app = start_app();
    let host = Host::new();
    let narrow = setter("narrow");
    narrow.set_max_width(500.0);
    let wide = setter("wide");
    wide.set_min_width(500.0);
    host.show_with(object(&behavior(&[&narrow, &wide])));

    host.resize(300.0, 100.0);
    assert!(host.has("narrow"));
    assert!(!host.has("wide"));

    host.resize(800.0, 100.0);
    assert!(!host.has("narrow"));
    assert!(host.has("wide"));

    // The maximum is exclusive and the minimum inclusive by default.
    host.resize(500.0, 100.0);
    assert!(!host.has("narrow"));
    assert!(host.has("wide"));

    host.window.close();
}

#[test]
fn height_conditions_switch_the_classes() {
    let _app = start_app();
    let host = Host::new();
    let tall = setter("tall");
    tall.set_min_height(200.0);
    host.show_with(object(&behavior(&[&tall])));

    host.resize(100.0, 300.0);
    assert!(host.has("tall"));

    host.resize(100.0, 150.0);
    assert!(!host.has("tall"));

    host.window.close();
}

#[test]
fn width_and_height_conditions_must_both_be_met() {
    let _app = start_app();
    let host = Host::new();
    let large = setter("large");
    large.set_min_width(400.0);
    large.set_min_height(200.0);
    host.show_with(object(&behavior(&[&large])));

    host.resize(500.0, 100.0);
    assert!(!host.has("large"));

    host.resize(300.0, 300.0);
    assert!(!host.has("large"));

    host.resize(500.0, 300.0);
    assert!(host.has("large"));

    host.window.close();
}

#[test]
fn setter_without_conditions_removes_its_class() {
    let _app = start_app();
    let host = Host::new();
    host.border.classes().add("plain");
    let plain = setter("plain");
    host.show_with(object(&behavior(&[&plain])));

    host.resize(300.0, 300.0);

    assert!(!host.has("plain"));

    host.window.close();
}

#[test]
fn operators_select_the_comparison() {
    let _app = start_app();
    let host = Host::new();
    let wider = setter("wider");
    wider.set_min_width(500.0);
    wider.set_min_width_operator(ComparisonConditionType::GreaterThan);
    let exact = setter("exact");
    exact.set_min_width(500.0);
    exact.set_min_width_operator(ComparisonConditionType::Equal);
    let not_exact = setter("not-exact");
    not_exact.set_max_width(500.0);
    not_exact.set_max_width_operator(ComparisonConditionType::NotEqual);
    let up_to = setter("up-to");
    up_to.set_max_width(500.0);
    up_to.set_max_width_operator(ComparisonConditionType::LessThanOrEqual);
    host.show_with(object(&behavior(&[&wider, &exact, &not_exact, &up_to])));

    host.resize(500.0, 100.0);
    assert!(!host.has("wider"));
    assert!(host.has("exact"));
    assert!(!host.has("not-exact"));
    assert!(host.has("up-to"));

    host.resize(501.0, 100.0);
    assert!(host.has("wider"));
    assert!(!host.has("exact"));
    assert!(host.has("not-exact"));
    assert!(!host.has("up-to"));

    host.window.close();
}

#[test]
fn pseudo_classes_are_set_as_pseudo_classes() {
    let _app = start_app();
    let host = Host::new();
    let narrow = setter(":narrow");
    narrow.set_is_pseudo_class(true);
    narrow.set_max_width(400.0);
    host.show_with(object(&behavior(&[&narrow])));

    host.resize(300.0, 100.0);
    assert!(host.has(":narrow"));

    host.resize(600.0, 100.0);
    assert!(!host.has(":narrow"));

    host.window.close();
}

#[test]
fn classes_go_to_the_target_control_of_the_behavior() {
    let _app = start_app();
    let host = Host::new();
    let narrow = setter("narrow");
    narrow.set_max_width(400.0);
    let behavior = behavior(&[&narrow]);
    behavior.set_target_control(&host.other);
    host.show_with(object(&behavior));

    host.resize(300.0, 100.0);

    assert!(host.other_has("narrow"));
    assert!(!host.has("narrow"));

    host.window.close();
}

#[test]
fn the_target_control_of_the_setter_wins_over_the_one_of_the_behavior() {
    let _app = start_app();
    let host = Host::new();
    let on_border = setter("on-border");
    on_border.set_max_width(400.0);
    on_border.set_target_control(&host.border);
    let on_other = setter("on-other");
    on_other.set_max_width(400.0);
    let behavior = behavior(&[&on_border, &on_other]);
    behavior.set_target_control(&host.other);
    host.show_with(object(&behavior));

    host.resize(300.0, 100.0);

    assert!(host.has("on-border"));
    assert!(!host.other_has("on-border"));
    assert!(host.other_has("on-other"));
    assert!(!host.has("on-other"));

    host.window.close();
}

#[test]
fn bounds_of_the_source_control_are_observed() {
    let _app = start_app();
    let host = Host::new();
    let narrow = setter("narrow");
    narrow.set_max_width(400.0);
    let behavior = behavior(&[&narrow]);
    behavior.set_source_control(&host.other);
    host.show_with(object(&behavior));

    host.resize(300.0, 100.0);
    host.resize_other(600.0, 100.0);
    assert!(!host.has("narrow"));

    host.resize_other(300.0, 100.0);
    assert!(host.has("narrow"));

    // The bounds of the associated object are not observed.
    host.resize(800.0, 100.0);
    assert!(host.has("narrow"));

    host.window.close();
}

#[test]
fn bounds_are_not_observed_once_detached_from_the_visual_tree() {
    let _app = start_app();
    let host = Host::new();
    let narrow = setter("narrow");
    narrow.set_max_width(400.0);
    host.show_with(object(&behavior(&[&narrow])));

    host.resize(300.0, 100.0);
    assert!(host.has("narrow"));

    host.detach_border();
    host.resize(800.0, 100.0);

    assert!(host.has("narrow"));

    host.window.close();
}

#[test]
fn setters_added_after_attach_apply_on_the_next_bounds_change() {
    let _app = start_app();
    let host = Host::new();
    let behavior = AdaptiveBehavior::new();
    host.show_with(object(&behavior));

    let narrow = setter("narrow");
    narrow.set_max_width(400.0);
    behavior.setters().add(narrow);
    host.resize(300.0, 100.0);

    assert!(host.has("narrow"));

    host.window.close();
}
