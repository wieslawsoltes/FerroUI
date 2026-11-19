//! Styles in the form of the element builders ([`style`]).

use crate::{AnimationBuilder, builder::{IntoValue, TypedProperty}};
use ferroui_base::styling::{Selector, Setter, Style};
use ferroui_base::{FerroProperty, Ref, TypeInfo};
use std::rc::Rc;

/// A style being built: a selector, setters, animations and nested styles. Turns into a
/// `Style` of the styles API of the port ([`build`](Self::build)), or is
/// given to an element (`styles: [..]` in `rsx!`, `.styles([..])`).
///
/// ```ignore
/// style("Button.accent")
///     .set(Button::background_property(), "#0063B1")
///     .set(Button::foreground_property(), "White")
///     .nested(style("^:pointerover").set(Button::opacity_property(), 0.9))
/// ```
pub struct StyleBuilder {
    selector: Selector,
    setters: Vec<(&'static FerroProperty, ferroui_base::BoxedValue)>,
    animations: Vec<AnimationBuilder>,
    nested: Vec<StyleBuilder>,
}

/// A style of the selector `selector`, in the syntax of XAML (`Button`,
/// `Button.accent`, `StackPanel > TextBlock`, `:pointerover`, `^.selected`
/// in a nested style). Type names are the classes of FerroUI.Base and
/// FerroUI.Controls and of the crates that registered their types.
///
/// # Panics
/// Panics with the message of the parser when the selector does not parse.
pub fn style(selector: &str) -> StyleBuilder {
    style_of(parse_selector(selector).unwrap_or_else(|error| panic!("the selector `{selector}` does not parse: {error}")))
}

/// A style of a selector built with the selectors API of the port
/// (`Selectors::of_type::<Button>().class("accent")`).
pub fn style_of(selector: Selector) -> StyleBuilder {
    StyleBuilder { selector, setters: Vec::new(), animations: Vec::new(), nested: Vec::new() }
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

    /// Runs the native animation on each control that matches the style.
    pub fn animation(mut self, animation: AnimationBuilder) -> Self {
        self.animations.push(animation);
        self
    }

    /// Adds a nested style, whose selector starts with `^` (the selector of
    /// this one): `style("^:pointerover")`.
    pub fn nested(mut self, style: StyleBuilder) -> Self {
        self.nested.push(style);
        self
    }

    /// The style.
    pub fn build(self) -> Ref<Style> {
        let setters = self.setters.into_iter().map(|(property, value)| Setter::new_untyped(property, value));
        let style = Style::with_setters(self.selector, setters);
        for animation in self.animations {
            style.add_animation(animation.build());
        }
        for nested in self.nested {
            style.children().add(nested.build());
        }
        style
    }
}
