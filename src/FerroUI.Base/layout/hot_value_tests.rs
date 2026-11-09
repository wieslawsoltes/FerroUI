//! Not ports: the kept copies of the size, margin and visibility properties
//! (docs/porting/desktop-performance.md, section 9.9) are the values `get_value` reads after every
//! kind of change.

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
