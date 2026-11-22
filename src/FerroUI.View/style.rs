//! Styles in the form of the element builders ([`style`]) and their typed selectors.
//!
//! | text              | typed                                                   |
//! |-------------------|---------------------------------------------------------|
//! | `Button`          | `of::<Button>()`                                        |
//! | `:is(Control)`    | `is_type::<Control>()`                                  |
//! | `.accent`         | `any().class("accent")`                                 |
//! | `#title`          | `any().name("title")`                                   |
//! | `:pressed`        | `any().pressed()`                                       |
//! | `Button.a:focus`  | `of::<Button>().class("a").focus()`                     |
//! | `A > B`           | `of::<A>().child().of::<B>()`                           |
//! | `A B`             | `of::<A>().descendant().of::<B>()`                      |
//! | `A /template/ B`  | `of::<A>().template().of::<B>()`                        |
//! | `A:not(.x)`       | `of::<A>().not(any().class("x"))`                       |
//! | `:nth-child(2n+1)`| `any().nth_child(2, 1)`                                 |
//! | `A[P=v]`          | `of::<A>().property_equals(A::p_property(), v)`         |
//! | `A, B`            | `any_of([of::<A>(), of::<B>()])`                        |
//! | `^:pressed`       | `nesting().pressed()`                                   |
//!
//! A typed selector names its classes by identity, so it keeps no class in the executable that
//! the application does not make, and it links no selector parser.

use crate::builder::{IntoValue, TypedProperty};
use crate::generated::pseudo_classes::PseudoClassSelectors;
use ferroui_base::styling::{Selector, Selectors, Setter, Style};
use ferroui_base::{FerroProperty, ObjectType, Ref, StyledElement, TypeInfo, Upcast};
use std::rc::Rc;

/// A style being built: a selector, setters and nested styles. Turns into a
/// `Style` of the styles API of the port ([`build`](Self::build)), or is
/// given to an element (`styles: [..]` in `rsx!`, `.styles([..])`).
///
/// ```ignore
/// style(of::<Button>().class("accent"))
///     .set(Button::background_property(), "#0063B1")
///     .set(Button::foreground_property(), "White")
///     .nested(style(nesting().pointer_over()).set(Button::opacity_property(), 0.9))
/// ```
pub struct StyleBuilder {
    selector: Selector,
    setters: Vec<(&'static FerroProperty, ferroui_base::BoxedValue)>,
    nested: Vec<StyleBuilder>,
}

/// What [`style`] takes: a typed [`Selector`], or text in the syntax of XAML.
pub trait IntoSelector {
    /// The selector.
    fn into_selector(self) -> Selector;
}

impl IntoSelector for Selector {
    #[inline]
    fn into_selector(self) -> Selector {
        self
    }
}

impl IntoSelector for &str {
    #[track_caller]
    fn into_selector(self) -> Selector {
        parse_selector(self).unwrap_or_else(|error| panic!("the selector `{self}` does not parse: {error}"))
    }
}

impl IntoSelector for &String {
    #[track_caller]
    fn into_selector(self) -> Selector {
        self.as_str().into_selector()
    }
}

impl IntoSelector for String {
    #[track_caller]
    fn into_selector(self) -> Selector {
        self.as_str().into_selector()
    }
}

/// A style of `selector`: a typed selector (`of::<Button>().class("accent")`, see the
/// [module](self)), or text in the syntax of XAML (`Button`, `Button.accent`,
/// `StackPanel > TextBlock`, `:pointerover`, `^.selected` in a nested style).
///
/// A misspelled type or pseudo-class of a typed selector does not compile. In text, type names
/// are the classes of FerroUI.Base and FerroUI.Controls and of the crates that registered their
/// types, and a name that is not registered is matched by name, so a misspelled one never
/// applies. Only an application that passes text links the selector parser.
///
/// # Panics
/// Panics with the message of the parser when a text selector does not parse.
#[track_caller]
pub fn style(selector: impl IntoSelector) -> StyleBuilder {
    style_of(selector.into_selector())
}

/// A style of a selector built with the selectors API of the port
/// (`Selectors::of_type::<Button>().class("accent")`).
pub fn style_of(selector: Selector) -> StyleBuilder {
    StyleBuilder { selector, setters: Vec::new(), nested: Vec::new() }
}

/// The selector of the class `T` exactly: `Button` is `of::<Button>()`.
///
/// The class is named by its identity: the selector does not keep the class in an executable
/// that never makes one.
#[inline]
pub fn of<T: ObjectType + Upcast<StyledElement>>() -> Selector {
    Selectors::of_type_identity(None, T::IDENTITY)
}

/// The selector of the class `T` and the classes derived from it: `:is(Control)` is
/// `is_type::<Control>()`.
#[inline]
pub fn is_type<T: ObjectType + Upcast<StyledElement>>() -> Selector {
    Selectors::is_type_identity(None, T::IDENTITY)
}

/// The start of a nested style, the selector of the style it is in: `^:pressed` is
/// `nesting().pressed()`.
#[inline]
pub fn nesting() -> Selector {
    Selectors::nesting(None)
}

/// The start of a selector with no type: `.accent` is `any().class("accent")`, `:pressed` is
/// `any().pressed()`.
#[inline]
pub fn any() -> AnySelector {
    AnySelector
}

/// A selector that matches any of `selectors`: `A, B` is `any_of([of::<A>(), of::<B>()])`.
pub fn any_of(selectors: impl IntoIterator<Item = Selector>) -> Selector {
    Selectors::or(selectors)
}

/// The start of a selector with no type ([`any`]). It is not a selector until it has a condition.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnySelector;

impl AnySelector {
    /// The selector of a style class.
    pub fn class(self, name: &str) -> Selector {
        Selectors::class(None, name)
    }

    /// The selector of a control's name.
    pub fn name(self, name: &str) -> Selector {
        Selectors::name(None, name)
    }

    /// The selector of what `argument` does not match.
    pub fn not(self, argument: Selector) -> Selector {
        Selectors::not(None, argument)
    }

    /// The selector of a position among siblings (`:nth-child(2n+1)` is `nth_child(2, 1)`).
    pub fn nth_child(self, step: i32, offset: i32) -> Selector {
        Selectors::nth_child(None, step, offset)
    }

    /// The selector of a position among siblings, counted from the end.
    pub fn nth_last_child(self, step: i32, offset: i32) -> Selector {
        Selectors::nth_last_child(None, step, offset)
    }
}

impl PseudoClassSelectors for AnySelector {
    #[inline]
    fn with_pseudo_class(self, name: &'static str) -> Selector {
        Selectors::class(None, name)
    }
}

/// The classes of a selector after its start, by identity like [`of`] and [`is_type`].
pub trait TypedSelector: Sized {
    /// Additionally matches the class `T` exactly: `A > B` is `of::<A>().child().of::<B>()`.
    fn of<T: ObjectType + Upcast<StyledElement>>(self) -> Selector;

    /// Additionally matches the class `T` and the classes derived from it.
    fn is_type<T: ObjectType + Upcast<StyledElement>>(self) -> Selector;
}

impl TypedSelector for Selector {
    #[inline]
    fn of<T: ObjectType + Upcast<StyledElement>>(self) -> Selector {
        Selectors::of_type_identity(Some(self), T::IDENTITY)
    }

    #[inline]
    fn is_type<T: ObjectType + Upcast<StyledElement>>(self) -> Selector {
        Selectors::is_type_identity(Some(self), T::IDENTITY)
    }
}

/// Parses a selector in the syntax of XAML, nested selectors (`^.selected`,
/// `^:pointerover > Button`) included.
///
/// A type name is the class of that name: looked up among the classes that are known when the
/// selector is parsed, and matched by its name when it is not known yet (no object of it was
/// made). The tables of every type of the framework are not registered for it, so a style in
/// text keeps no control in the application that the application does not make
/// (docs/binary-size.md, section 4.23).
pub fn parse_selector(selector: &str) -> Result<Selector, String> {
    use ferroui_markup::markup::parsers::SelectorParser;
    let parser = SelectorParser::new(|_namespace, name| TypeInfo::find_by_name(name)).unknown_types_by_name();
    match parser.parse(selector) {
        Ok(Some(selector)) => Ok(selector),
        Ok(None) => Err(String::from("an empty selector")),
        Err(error) => Err(error.to_string()),
    }
}

impl StyleBuilder {
    /// Sets `property` to `value` (what converts to its value: `"Red"` for a
    /// brush, `8.0` for a thickness).
    pub fn set<P: TypedProperty + ?Sized>(mut self, property: &'static P, value: impl IntoValue<P::Value>) -> Self {
        self.setters.push((property.untyped(), Rc::new(value.into_value())));
        self
    }

    /// Adds a nested style, whose selector starts with the selector of this one:
    /// `style(nesting().pointer_over())`, or `style("^:pointerover")` in text.
    pub fn nested(mut self, style: StyleBuilder) -> Self {
        self.nested.push(style);
        self
    }

    /// The style.
    pub fn build(self) -> Ref<Style> {
        let setters = self.setters.into_iter().map(|(property, value)| Setter::new_untyped(property, value));
        let style = Style::with_setters(self.selector, setters);
        for nested in self.nested {
            style.children().add(nested.build());
        }
        style
    }
}
