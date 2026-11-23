//! Not ports: the kept copies of the size, margin, alignment and visibility properties
//! (docs/porting/desktop-performance.md, sections 9.9 and 9.28) are the values `get_value` reads
//! after every kind of change.

use crate::data::BindingPriority;
use crate::layout::Layoutable;
use crate::{FerroObjectImplExt, Thickness, Visual};

#[test]
fn a_kept_size_follows_local_style_animation_and_current_values() {
    let layoutable = Layoutable::new();
    assert!(layoutable.width().is_nan());

    layoutable.set_width(10.0);
    assert_eq!(10.0, layoutable.width());

    // A style value under the local one: the local value stays effective.
    let _style = layoutable.set_value_with_priority(Layoutable::width_property(), 20.0, BindingPriority::Style);
    assert_eq!(10.0, layoutable.width());

    // The local value cleared: the style value is effective.
    layoutable.clear_value(Layoutable::width_property());
    assert_eq!(20.0, layoutable.width());

    // A current value, then an animation over it, then the animation removed.
    layoutable.set_current_value(Layoutable::width_property(), 30.0);
    assert_eq!(30.0, layoutable.width());
    let animation =
        layoutable.set_value_with_priority(Layoutable::width_property(), 40.0, BindingPriority::Animation);
    assert_eq!(40.0, layoutable.width());
    if let Some(animation) = animation {
        animation.dispose();
    }
    assert_ne!(40.0, layoutable.width());
    assert_eq!(layoutable.get_value(Layoutable::width_property()), layoutable.width());
}

#[test]
fn a_kept_margin_and_visibility_follow_their_changes() {
    let layoutable = Layoutable::new();
    assert_eq!(Thickness::default(), layoutable.margin());
    layoutable.set_margin(Thickness::uniform(4.0));
    assert_eq!(Thickness::uniform(4.0), layoutable.margin());
    layoutable.clear_value(Layoutable::margin_property());
    assert_eq!(Thickness::default(), layoutable.margin());

    let visual: &Visual = &layoutable;
    assert!(visual.is_visible());
    visual.set_is_visible(false);
    assert!(!visual.is_visible());
    visual.clear_value(Visual::is_visible_property());
    assert!(visual.is_visible());
}

#[test]
fn the_kept_alignments_follow_their_changes() {
    use crate::layout::{HorizontalAlignment, VerticalAlignment};

    let layoutable = Layoutable::new();
    assert_eq!(HorizontalAlignment::Stretch, layoutable.horizontal_alignment());
    assert_eq!(VerticalAlignment::Stretch, layoutable.vertical_alignment());

    layoutable.set_horizontal_alignment(HorizontalAlignment::Center);
    layoutable.set_vertical_alignment(VerticalAlignment::Bottom);
    assert_eq!(HorizontalAlignment::Center, layoutable.horizontal_alignment());
    assert_eq!(VerticalAlignment::Bottom, layoutable.vertical_alignment());

    // A style value under the local one, then the local one cleared.
    let _style = layoutable.set_value_with_priority(
        Layoutable::horizontal_alignment_property(),
        HorizontalAlignment::Right,
        BindingPriority::Style,
    );
    assert_eq!(HorizontalAlignment::Center, layoutable.horizontal_alignment());
    layoutable.clear_value(Layoutable::horizontal_alignment_property());
    assert_eq!(HorizontalAlignment::Right, layoutable.horizontal_alignment());

    layoutable.clear_value(Layoutable::vertical_alignment_property());
    assert_eq!(VerticalAlignment::Stretch, layoutable.vertical_alignment());
}

#[test]
fn kept_layout_rounding_follows_a_value_of_the_element_and_of_an_ancestor() {
    use crate::styling::test_support::{remove_child, set_child, Class3};

    let parent = Class3::new();
    let child = Class3::new();
    set_child(&parent, &child);

    // No store holds a value: the default of the class, read twice so the second is the kept one.
    assert!(child.use_layout_rounding());
    assert!(child.use_layout_rounding());
    assert!(parent.use_layout_rounding());

    // A value of an ancestor is inherited by an element that kept the default.
    parent.set_use_layout_rounding(false);
    assert!(!parent.use_layout_rounding());
    assert!(!child.use_layout_rounding());
    assert_eq!(child.get_value(Layoutable::use_layout_rounding_property()), child.use_layout_rounding());

    // The value of the element over the inherited one, then cleared again.
    child.set_use_layout_rounding(true);
    assert!(child.use_layout_rounding());
    child.clear_value(Layoutable::use_layout_rounding_property());
    assert!(!child.use_layout_rounding());

    // Out of the tree of the ancestor the element has the default again.
    remove_child(&parent, &child);
    assert!(child.use_layout_rounding());
    set_child(&parent, &child);
    assert!(!child.use_layout_rounding());

    // No store holds a value any more: the kept default is the value again.
    parent.clear_value(Layoutable::use_layout_rounding_property());
    assert!(parent.use_layout_rounding());
    assert!(child.use_layout_rounding());

    // A style value is a value a store holds as well.
    let style = child.set_value_with_priority(
        Layoutable::use_layout_rounding_property(),
        false,
        BindingPriority::Style,
    );
    assert!(!child.use_layout_rounding());
    assert!(parent.use_layout_rounding());
    if let Some(style) = style {
        style.dispose();
    }
    assert!(child.use_layout_rounding());
}

#[test]
fn kept_container_sizing_follows_its_changes_between_two_measures() {
    use crate::styling::test_support::Class3;
    use crate::styling::{Container, ContainerSizing};
    use crate::Size;

    let container = Class3::new();
    container.set_width(40.0);
    container.measure(Size::new(120.0, 80.0));
    assert_eq!(Size::new(40.0, 0.0), container.desired_size());

    // The sizing set after a measure kept `Normal`: the next measure is one of a container.
    container.clear_value(Layoutable::width_property());
    Container::set_sizing(&container, ContainerSizing::Width);
    container.measure(Size::new(120.0, 80.0));
    assert_eq!(Size::new(120.0, 0.0), container.desired_size());

    // And back.
    Container::set_sizing(&container, ContainerSizing::Normal);
    container.invalidate_measure();
    container.measure(Size::new(120.0, 80.0));
    assert_eq!(Size::new(0.0, 0.0), container.desired_size());
}
