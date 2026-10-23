//! Styles in the form of the element builders ([`style`]).

use crate::builder::{IntoValue, TypedProperty};
use ferroui_base::styling::{Selector, Setter, Style};
use ferroui_base::{FerroProperty, Ref, TypeInfo};
use std::rc::Rc;

/// A style being built: a selector, setters and nested styles. Turns into a
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
    StyleBuilder { selector, setters: Vec::new(), nested: Vec::new() }
}

/// Parses a selector in the syntax of XAML, nested selectors (`^.selected`,
/// `^:pointerover > Button`) included.
pub fn parse_selector(selector: &str) -> Result<Selector, String> {
    ferroui_controls::register_types();
    let parser = ferroui_markup::markup::parsers::SelectorParser::new(|_namespace, name| TypeInfo::find_by_name(name));
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
        for nested in self.nested {
            style.children().add(nested.build());
        }
        style
    }
}
