use super::{
    ChildSelector, DescendantSelector, NestingSelector, NotSelector, NthChildSelector, NthLastChildSelector,
    OrSelector, PropertyEqualsSelector, Selector, TemplateSelector, TypeNameAndClassSelector,
};
use crate::{BoxedValue, FerroProperty, ObjectType, PropertyValue, StyledElement, StyledProperty, TypeInfo, Upcast};
use std::rc::Rc;

/// Functions for building [`Selector`]s.
///
/// Every function takes the selector that precedes the new one (`None` to
/// start a selector). The same functions are available as chaining methods on
/// [`Selector`]:
///
/// ```ignore
/// // Button.big > :is(Control)
/// let selector = Selectors::of_type::<Button>().class("big").child().is::<Control>();
/// ```
pub struct Selectors;

impl Selectors {
    /// Returns a selector which matches a previous selector's child.
    pub fn child(previous: Option<Selector>) -> Selector {
        Selector::new(ChildSelector::new(previous))
    }

    /// Returns a selector which matches a control's style class.
    pub fn class(previous: Option<Selector>, name: &str) -> Selector {
        if name.trim().is_empty() {
            panic!("Name may not be empty");
        }

        if let Some(previous) = &previous {
            if let Some(tac) = previous.node::<TypeNameAndClassSelector>() {
                tac.add_class(name);
                return previous.clone();
            }
        }

        Selector::new(TypeNameAndClassSelector::for_class(previous, name))
    }

    /// Returns a selector which matches a descendant of a previous selector.
    pub fn descendant(previous: Option<Selector>) -> Selector {
        Selector::new(DescendantSelector::new(previous))
    }

    /// Returns a selector which matches a type or a derived type.
    pub fn is_type_info(previous: Option<Selector>, type_: &'static TypeInfo) -> Selector {
        Selector::new(TypeNameAndClassSelector::is(previous, type_))
    }

    /// Returns a selector which matches class `T` or a class derived from it.
    pub fn is<T: ObjectType + Upcast<StyledElement>>() -> Selector {
        Self::is_type_info(None, T::TYPE)
    }

    /// Returns a selector which matches a control's name.
    pub fn name(previous: Option<Selector>, name: &str) -> Selector {
        if name.trim().is_empty() {
            panic!("Name may not be empty");
        }

        if let Some(previous) = &previous {
            if let Some(tac) = previous.node::<TypeNameAndClassSelector>() {
                tac.set_name(name);
                return previous.clone();
            }
        }

        Selector::new(TypeNameAndClassSelector::for_name(previous, name))
    }

    /// Returns the nesting selector, which stands for the selector of the
    /// parent style.
    pub fn nesting(_previous: Option<Selector>) -> Selector {
        Selector::new(NestingSelector)
    }

    /// Returns a selector which inverts the results of selector argument.
    pub fn not(previous: Option<Selector>, argument: Selector) -> Selector {
        Selector::new(NotSelector::new(previous, argument))
    }

    /// Returns a selector which inverts the results of the selector built by
    /// `argument`.
    pub fn not_with(previous: Option<Selector>, argument: impl FnOnce(Option<Selector>) -> Selector) -> Selector {
        Selector::new(NotSelector::new(previous, argument(None)))
    }

    /// Returns a selector which matches elements based on their position
    /// among a group of siblings.
    pub fn nth_child(previous: Option<Selector>, step: i32, offset: i32) -> Selector {
        Selector::new(NthChildSelector::new(previous, step, offset))
    }

    /// Returns a selector which matches elements based on their position
    /// among a group of siblings, counting from the end.
    pub fn nth_last_child(previous: Option<Selector>, step: i32, offset: i32) -> Selector {
        Selector::new(NthLastChildSelector::new(previous, step, offset))
    }

    /// Returns a selector which matches a type.
    pub fn of_type_info(previous: Option<Selector>, type_: &'static TypeInfo) -> Selector {
        Selector::new(TypeNameAndClassSelector::of_type(previous, type_))
    }

    /// Returns a selector which matches class `T` exactly.
    pub fn of_type<T: ObjectType + Upcast<StyledElement>>() -> Selector {
        Self::of_type_info(None, T::TYPE)
    }

    /// Returns a selector which ORs selectors.
    pub fn or(selectors: impl IntoIterator<Item = Selector>) -> Selector {
        Selector::new(OrSelector::new(selectors.into_iter().collect()))
    }

    /// Returns a selector which matches a control with the specified property
    /// value.
    pub fn property_equals<T: PropertyValue>(
        previous: Option<Selector>,
        property: &'static StyledProperty<T>,
        value: T,
    ) -> Selector {
        Selector::new(PropertyEqualsSelector::new(previous, property, Rc::new(value)))
    }

    /// Returns a selector which matches a control with the specified property
    /// value. `value` must hold exactly the property's value type.
    pub fn property_equals_untyped(
        previous: Option<Selector>,
        property: &'static FerroProperty,
        value: BoxedValue,
    ) -> Selector {
        Selector::new(PropertyEqualsSelector::new(previous, property, value))
    }

    /// Returns a selector which enters a lookless control's template.
    pub fn template(previous: Option<Selector>) -> Selector {
        Selector::new(TemplateSelector::new(previous))
    }
}

impl Selector {
    /// Returns a selector which matches this selector's child.
    pub fn child(self) -> Selector {
        Selectors::child(Some(self))
    }

    /// Returns a selector which additionally matches a control's style class.
    pub fn class(self, name: &str) -> Selector {
        Selectors::class(Some(self), name)
    }

    /// Returns a selector which matches a descendant of this selector.
    pub fn descendant(self) -> Selector {
        Selectors::descendant(Some(self))
    }

    /// Returns a selector which additionally matches class `T` or a class
    /// derived from it.
    pub fn is<T: ObjectType + Upcast<StyledElement>>(self) -> Selector {
        Selectors::is_type_info(Some(self), T::TYPE)
    }

    /// Returns a selector which additionally matches a type or a derived
    /// type.
    pub fn is_type_info(self, type_: &'static TypeInfo) -> Selector {
        Selectors::is_type_info(Some(self), type_)
    }

    /// Returns a selector which additionally matches a control's name.
    pub fn name(self, name: &str) -> Selector {
        Selectors::name(Some(self), name)
    }

    /// Returns the nesting selector.
    pub fn nesting(self) -> Selector {
        Selectors::nesting(Some(self))
    }

    /// Returns a selector which additionally inverts the results of
    /// `argument`.
    pub fn not(self, argument: Selector) -> Selector {
        Selectors::not(Some(self), argument)
    }

    /// Returns a selector which additionally inverts the results of the
    /// selector built by `argument`.
    pub fn not_with(self, argument: impl FnOnce(Option<Selector>) -> Selector) -> Selector {
        Selectors::not_with(Some(self), argument)
    }

    /// Returns a selector which additionally matches elements based on their
    /// position among a group of siblings.
    pub fn nth_child(self, step: i32, offset: i32) -> Selector {
        Selectors::nth_child(Some(self), step, offset)
    }

    /// Returns a selector which additionally matches elements based on their
    /// position among a group of siblings, counting from the end.
    pub fn nth_last_child(self, step: i32, offset: i32) -> Selector {
        Selectors::nth_last_child(Some(self), step, offset)
    }

    /// Returns a selector which additionally matches class `T` exactly.
    pub fn of_type<T: ObjectType + Upcast<StyledElement>>(self) -> Selector {
        Selectors::of_type_info(Some(self), T::TYPE)
    }

    /// Returns a selector which additionally matches a type.
    pub fn of_type_info(self, type_: &'static TypeInfo) -> Selector {
        Selectors::of_type_info(Some(self), type_)
    }

    /// Returns a selector which additionally matches a control with the
    /// specified property value.
    pub fn property_equals<T: PropertyValue>(self, property: &'static StyledProperty<T>, value: T) -> Selector {
        Selectors::property_equals(Some(self), property, value)
    }

    /// Returns a selector which additionally matches a control with the
    /// specified property value. `value` must hold exactly the property's
    /// value type.
    pub fn property_equals_untyped(self, property: &'static FerroProperty, value: BoxedValue) -> Selector {
        Selectors::property_equals_untyped(Some(self), property, value)
    }

    /// Returns a selector which enters a lookless control's template.
    pub fn template(self) -> Selector {
        Selectors::template(Some(self))
    }
}
