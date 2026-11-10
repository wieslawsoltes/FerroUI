//! Styles in scope that do not apply: a styling pass of the texts of an attached wall whose root
//! holds one style that matches them and many that do not. The wall stands inside four nested
//! borders in every benchmark, so the walk of the style hosts is the same and the difference is
//! what the styles that do not match cost.
//!
//! - `rejected_by_type`: 10, 100 and 1,000 styles for another type, each handed to the matcher,
//!   which rules it out by the type of the text;
//! - `rejected_by_type_indexed`: the same styles with the selector index enabled, as the styles of
//!   the generated themes are: the collection keeps the candidates of a style key and offers a
//!   text the one style that can match (section 9.26); the cost no longer grows with the styles;
//! - `inactive_by_class`: styles for the type of the texts and a class no text carries: the type
//!   matches, so every text gets an instance of every style, switched off, with an activator
//!   that follows its classes;
//! - `rejected_on_ancestors`: the 100 styles for another type spread over the root and the four
//!   borders, twenty each, in place of all on the root (`rejected_by_type` with 100 styles): five
//!   collections evaluated for every text.

use super::wall::{tag_setter, Wall, CONTROLS};
use crate::harness::Registry;
use ferroui_base::styling::{Selectors, Style};
use ferroui_base::Ref;
use ferroui_controls::{Button, TextBlock};

/// The borders around the wall: with the root, five style hosts above it.
const WRAPPERS: usize = 4;

/// Why a style does not apply to a text.
#[derive(Clone, Copy)]
enum Rejection {
    /// The style selects another type; the matcher sees it.
    Type,
    /// The style selects another type and is index-enabled; the collection leaves it out.
    TypeIndexed,
    /// The style selects the type of the texts and a class none of them carries.
    Class,
}

fn rejected_style(rejection: Rejection, index: usize) -> Ref<Style> {
    match rejection {
        Rejection::Type => Style::with_setters(Selectors::of_type::<Button>(), [tag_setter()]),
        Rejection::TypeIndexed => {
            let style = Style::with_setters(Selectors::of_type::<Button>(), [tag_setter()]);
            style.enable_selector_index();
            style
        }
        Rejection::Class => {
            Style::with_setters(Selectors::of_type::<TextBlock>().class(&format!("absent-{index}")), [tag_setter()])
        }
    }
}

/// An attached wall with the matching style on the root and `styles` styles that do not apply:
/// all on the root, or in turns on the root and the borders when `spread`.
fn wall(styles: usize, rejection: Rejection, spread: bool) -> Wall {
    let mut wall = Wall::with_wrappers(CONTROLS, WRAPPERS);
    let root = wall.fixture.root.styles();
    let matching = Style::with_setters(Selectors::of_type::<TextBlock>(), [tag_setter()]);
    if matches!(rejection, Rejection::TypeIndexed) {
        matching.enable_selector_index();
    }
    root.add(&matching);
    for index in 0..styles {
        let style = rejected_style(rejection, index);
        match index % (WRAPPERS + 1) {
            host if spread && host > 0 => wall.wrappers[host - 1].styles().add(&style),
            _ => root.add(&style),
        }
    }
    wall.attach();
    wall.check_restyle(true);
    wall
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "StylesInScope").min_iteration_time(200);
    let cases: [(&'static str, Rejection, &[usize]); 3] = [
        ("rejected_by_type", Rejection::Type, &[10, 100, 1_000]),
        ("rejected_by_type_indexed", Rejection::TypeIndexed, &[100, 1_000]),
        ("inactive_by_class", Rejection::Class, &[10, 100]),
    ];
    for (name, rejection, sizes) in cases {
        for &styles in sizes {
            class.benchmark_with_iteration_setup(
                name,
                format!("Styles={styles}"),
                move || wall(styles, rejection, false),
                Wall::prepare_restyle,
                |wall| wall.style_texts(),
            );
        }
    }
    class.benchmark_with_iteration_setup(
        "rejected_on_ancestors",
        format!("Styles=100, Hosts={}", WRAPPERS + 1),
        || wall(100, Rejection::Type, true),
        Wall::prepare_restyle,
        |wall| wall.style_texts(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "StylesInScope");
    }
}
