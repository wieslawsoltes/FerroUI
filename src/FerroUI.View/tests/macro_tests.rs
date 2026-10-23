use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{component, rsx, Element, IntoElement};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, Control, Grid, StackPanel, TextBlock};

#[test]
fn rsx_expands_to_the_builders() {
    let built: Element = rsx! {
        StackPanel { spacing: 8.0, name: "root",
            TextBlock { text: "Title", class: "h1" }
            Button { Grid.row: 1, width: 80.0, "Go" }
        }
    };
    let by_hand: Element = stack_panel()
        .spacing(8.0)
        .name("root")
        .child(text_block().text("Title").class("h1"))
        .child(button().grid_row(1).width(80.0).child("Go"))
        .into_element();
    assert_eq!(format!("{built:?}"), format!("{by_hand:?}"));
    let (Element::Control(a), Element::Control(b)) = (built, by_hand) else { panic!("control elements") };
    assert_eq!(a.props, b.props);
}

#[test]
fn rsx_children_from_expressions_conditions_and_loops() {
    let names = ["a", "b", "c"];
    let with_footer = false;
    let element = rsx! {
        StackPanel {
            for name in names {
                TextBlock { text: name }
            }
            if with_footer {
                TextBlock { text: "footer" }
            } else {
                TextBlock { text: "no footer" }
            }
            { text_block().text("expression") }
        }
    };
    let (control, owner) = element.mount_root();
    let texts: Vec<String> = control
        .cast::<StackPanel>()
        .unwrap()
        .children()
        .to_vec()
        .into_iter()
        .map(|c| c.cast::<TextBlock>().unwrap().text().unwrap_or_default())
        .collect();
    assert_eq!(texts, vec!["a", "b", "c", "no footer", "expression"]);
    owner.dispose();
}

/// A labelled value.
#[component]
fn Labelled(label: String, #[prop(default = 2)] digits: usize, value: f64) -> Element {
    rsx! { TextBlock { text: format!("{label}: {value:.digits$}") } }
}

/// A frame around its children.
#[component]
fn Frame(children: Element, title: Option<String>) -> Element {
    rsx! {
        StackPanel {
            TextBlock { text: title.unwrap_or_else(|| String::from("untitled")) }
            { children }
        }
    }
}

#[test]
fn a_component_has_a_builder_with_defaults_and_children() {
    let element = rsx! {
        Frame { title: "Totals",
            Labelled { label: "Sum", value: 3.14159 }
            Labelled { label: "Mean", value: 1.5, digits: 0usize }
        }
    };
    let (control, owner) = element.mount_root();
    let texts: Vec<String> = control
        .cast::<StackPanel>()
        .unwrap()
        .children()
        .to_vec()
        .into_iter()
        .map(|c| c.cast::<TextBlock>().unwrap().text().unwrap_or_default())
        .collect();
    assert_eq!(texts, vec!["Totals", "Sum: 3.14", "Mean: 2"]);
    owner.dispose();
}

#[test]
#[should_panic(expected = "`value` is a required property of `Labelled`")]
fn a_missing_required_property_of_a_component_is_named() {
    let _ = labelled().label("x").into_element();
}

#[test]
fn rsx_with_reactive_values_and_handlers() {
    let owner = Owner::new_root();
    let (count, control) = owner.with(|| {
        let count = signal(0);
        let control = rsx! {
            StackPanel {
                TextBlock { text: move || format!("Count: {}", count.get()) }
                Button { on_click: move |_| count.update(|c| *c += 1), "Increment" }
            }
        }
        .mount();
        (count, control)
    });
    let panel = control.cast::<StackPanel>().unwrap();
    let button: Ref<Control> = panel.children().to_vec().into_iter().nth(1).unwrap();
    button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(count.get_untracked(), 1);
    let text = panel.children().to_vec().into_iter().next().unwrap().cast::<TextBlock>().unwrap().text();
    assert_eq!(text.as_deref(), Some("Count: 1"));
    let content: Option<BoxedValue> = button.cast::<Button>().unwrap().content();
    assert_eq!(content.and_then(|c| c.downcast_ref::<String>().cloned()).as_deref(), Some("Increment"));
    let _ = Grid::row_property();
    owner.dispose();
}
