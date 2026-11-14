//! The properties a class names for its `on_property_changed` (`PropertyChangeHandler`,
//! desktop-performance.md section 9.59).

use super::*;
use crate::animation::Animatable;
use crate::data::BindingPriority;
use crate::input::InputElement;
use crate::layout::Layoutable;
use crate::styling::ThemeVariant;
use crate::PropertyChangeHandler as Handler;
use crate::*;

const HANDLERS: [Handler; 6] = [
    Handler::Animatable,
    Handler::StyledElement,
    Handler::Visual,
    Handler::Layoutable,
    Handler::InputElement,
    Handler::Control,
];

fn handlers_of(property: &'static FerroProperty) -> Vec<Handler> {
    HANDLERS.into_iter().filter(|handler| property.has_change_handler(*handler)).collect()
}

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Bar", s("bardefault"))
    });
}

test_class!(Class2: FerroObject);

impl Class2 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        Class1::foo_property().add_owner::<Class2>()
    });
}

fn change_of<'a>(
    sender: &'a FerroObject,
    property: &'static FerroProperty,
    old_value: &'a String,
    new_value: &'a String,
) -> FerroPropertyChangedEventArgs<'a> {
    FerroPropertyChangedEventArgs::new(sender, property, Some(old_value), new_value, BindingPriority::LocalValue, true)
}

#[test]
fn a_class_names_the_properties_its_handler_reacts_to() {
    InputElement::TYPE.ensure_class_init();

    assert_eq!(handlers_of(Animatable::transitions_property().as_property()), [Handler::Animatable]);
    assert_eq!(handlers_of(StyledElement::theme_property().as_property()), [Handler::StyledElement]);
    assert_eq!(
        handlers_of(ThemeVariant::actual_theme_variant_property().as_property()),
        [Handler::StyledElement, Handler::Visual]
    );
    assert_eq!(
        handlers_of(Visual::is_visible_property().as_property()),
        [Handler::Visual, Handler::Layoutable, Handler::InputElement]
    );
    assert_eq!(handlers_of(Visual::shadow_property().as_property()), [Handler::Visual]);
    assert_eq!(handlers_of(Visual::flow_direction_property().as_property()), [Handler::Visual]);
    assert_eq!(handlers_of(InputElement::is_focused_property().as_property()), [Handler::InputElement]);
    assert_eq!(handlers_of(InputElement::is_pointer_over_property().as_property()), [Handler::InputElement]);
    assert_eq!(
        handlers_of(InputElement::is_keyboard_focus_within_property().as_property()),
        [Handler::InputElement]
    );

    // What changes with every arrange and every frame of an animation is named by none of
    // the classes of this crate: their handlers return before they compare.
    for property in [
        Visual::bounds_property().as_property(),
        Visual::opacity_property().as_property(),
        Layoutable::width_property().as_property(),
        Layoutable::margin_property().as_property(),
        StyledElement::data_context_property().as_property(),
    ] {
        assert!(handlers_of(property).is_empty(), "{}", property.name());
    }
}

#[test]
fn a_property_added_to_another_owner_is_named_with_the_property_it_was_added_from() {
    let source = Class1::foo_property().as_property();
    let added = Class2::foo_property().as_property();
    assert!(handlers_of(source).is_empty());
    assert!(handlers_of(added).is_empty());

    Handler::Control.reacts_to(&[added]);
    assert_eq!(handlers_of(source), [Handler::Control]);
    assert_eq!(handlers_of(added), [Handler::Control]);

    // Named twice, and from the other side.
    Handler::Control.reacts_to(&[source, added]);
    Handler::Layoutable.reacts_to(&[source]);
    assert_eq!(handlers_of(source), [Handler::Layoutable, Handler::Control]);
    assert_eq!(handlers_of(added), [Handler::Layoutable, Handler::Control]);

    assert!(handlers_of(Class1::bar_property().as_property()).is_empty());
}

#[test]
fn a_change_reaches_the_handlers_its_property_was_named_for() {
    let target = Class1::new();
    let sender: &FerroObject = &target;
    let (old_value, new_value) = (s("old"), s("new"));
    let named = Class1::foo_property().as_property();
    let other = Class1::bar_property().as_property();
    Handler::Control.reacts_to(&[named]);

    let change = change_of(sender, named, &old_value, &new_value);
    assert!(change.reaches(Handler::Control));
    assert!(change.is_for(Handler::Control, named));
    assert!(!change.is_for(Handler::Control, other));
    // A handler the property was not named for: a release build leaves it out, a debug build
    // runs its comparisons to check them against what its class named.
    assert_eq!(change.reaches(Handler::Visual), cfg!(debug_assertions));
    assert!(!change.is_for(Handler::Visual, other));

    let change = change_of(sender, other, &old_value, &new_value);
    assert_eq!(change.reaches(Handler::Control), cfg!(debug_assertions));
    assert!(!change.is_for(Handler::Control, named));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "which its class did not name")]
fn a_handler_that_reacts_to_a_property_its_class_did_not_name_fails_in_a_debug_build() {
    let target = Class1::new();
    let sender: &FerroObject = &target;
    let (old_value, new_value) = (s("old"), s("new"));
    let other = Class1::bar_property().as_property();

    let change = change_of(sender, other, &old_value, &new_value);
    let _ = change.is_for(Handler::Control, other);
}
