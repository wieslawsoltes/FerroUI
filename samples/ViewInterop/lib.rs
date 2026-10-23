//! A directory of 10,000 people, written with `ferroui-view` and XAML
//! together (`docs/declarative-ui.md`, section 9):
//!
//! - the list is a virtualized `ListBox` whose item template is a component
//!   ([`template`]): only the visible items have controls, and scrolling
//!   gives their hosts other people instead of making new controls;
//! - the details are XAML (`Details.xaml`), compiled to Rust when the crate
//!   is built and loaded into the component by its URI ([`use_xaml`]); the
//!   component fills its named controls;
//! - inside that XAML, a `ComponentHost` shows the registered component
//!   `Badge`, which reads the person from its data context.
//!
//! `cargo run -p view-interop`.

use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::FontWeight;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Control, ItemsSource, TextBlock};
use ferroui_view::prelude::*;
use ferroui_view::{register_component, use_xaml};
use std::rc::Rc;

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as view_interop;

mod register_types;
pub use register_types::{register_types, ASSEMBLY};

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

/// A person of the directory.
#[derive(Clone, PartialEq, Debug)]
pub struct Person {
    pub id: u32,
    pub name: String,
    pub city: String,
}

const FIRST: [&str; 10] = ["Ada", "Grace", "Alan", "Edsger", "Barbara", "Donald", "Frances", "John", "Margaret", "Niklaus"];
const LAST: [&str; 10] = ["Lovelace", "Hopper", "Turing", "Dijkstra", "Liskov", "Knuth", "Allen", "Backus", "Hamilton", "Wirth"];
const CITIES: [&str; 8] = ["London", "New York", "Zurich", "Amsterdam", "Boston", "Stanford", "Austin", "Paris"];

/// The people of the directory: `count` of them, the same on every run.
pub fn people(count: u32) -> Vec<Person> {
    (0..count)
        .map(|id| Person {
            id,
            name: format!("{} {}", FIRST[(id % 10) as usize], LAST[((id / 10) % 10) as usize]),
            city: CITIES[(id % 8) as usize].to_string(),
        })
        .collect()
}

/// The details of a person: `Details.xaml`, compiled to Rust by the build of the crate
/// (`build.rs`), with a component inside.
pub const DETAILS: &str = "ferres://ViewInterop/Details.xaml";

/// Registers the components the XAML of the sample names.
pub fn register_components() {
    register_component("Badge", || {
        rerender(|| {
            let text = match use_data_context::<Person>() {
                Some(person) => format!("Component in XAML: person {} of 10,000", person.id + 1),
                None => String::from("Component in XAML: no person"),
            };
            rsx! { TextBlock { text, foreground: "#0063B1" } }
        })
    });
}

/// The directory: a component that runs again when the selection changes.
#[component(rerender)]
pub fn Directory(#[prop(default = 10_000)] count: u32) -> Element {
    let all = use_hook(move || Rc::new(people(count)));
    // The items and the template are made once: a new value on every run
    // would be set again, and the list would make its items again.
    let source = use_hook({
        let all = all.clone();
        move || ItemsSource::from_items(all.iter().map(|person| Some(Rc::new(person.clone()) as BoxedValue)))
    });
    let item_template = use_hook(|| {
        template(|person: Person| {
            rsx! {
                StackPanel { margin: (0.0, 2.0),
                    TextBlock { text: person.name, font_weight: FontWeight::SemiBold }
                    TextBlock { text: format!("{} - #{}", person.city, person.id + 1), foreground: "#666666" }
                }
            }
        })
    });
    let selected = use_signal(|| 41_i32);
    let person = usize::try_from(selected.get()).ok().and_then(|index| all.get(index).cloned());

    let details: Ref<Control> = use_xaml(DETAILS);
    use_effect_with(person.clone(), {
        let details = details.clone();
        move |person| {
            if let Some(person) = person {
                details.get_control::<TextBlock>("Title").set_text(Some(person.name.as_str()));
                details.get_control::<TextBlock>("City").set_text(Some(format!("Lives in {}", person.city).as_str()));
            }
            details.set_data_context(person.clone().map(|person| Rc::new(person) as BoxedValue));
        }
    });

    rsx! {
        Grid { column_definitions: "280,*", row_definitions: "Auto,*", margin: 16.0,
            TextBlock { Grid.column_span: 2, margin: (0.0, 0.0, 0.0, 8.0), vertical_alignment: VerticalAlignment::Center,
                text: format!("{} people; the list realizes only the visible ones", all.len()) }
            ListBox { name: "People", Grid.row: 1, items_source: source, item_template: item_template,
                selected_index: selected.two_way() }
            Border { Grid.row: 1, Grid.column: 1, margin: (16.0, 0.0, 0.0, 0.0), vertical_alignment: VerticalAlignment::Top,
                {Element::Existing(details)} }
        }
    }
}
