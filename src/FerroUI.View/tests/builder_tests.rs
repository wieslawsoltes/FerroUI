use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{Element, GridDefinitions, IntoElement};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::{Button, Control, Grid, StackPanel, TextBlock, TextBox};
use std::cell::Cell;
use std::rc::Rc;

fn mount(element: impl IntoElement) -> (Ref<Control>, Owner) {
    element.into_element().mount_root()
}

fn content_text(button: &Button) -> Option<String> {
    button.content().and_then(|content: BoxedValue| content.downcast_ref::<String>().cloned())
}

#[test]
fn setters_give_the_properties_their_values_with_the_conversions() {
    let (control, owner) = mount(
        text_block()
            .text("Hello")
            .width(120)
            .margin(4.0)
            .padding((2.0, 6.0))
            .horizontal_alignment(HorizontalAlignment::Center)
            .name("greeting")
            .class("title large"),
    );
    let text = control.cast::<TextBlock>().expect("a text block");
    assert_eq!(text.text().as_deref(), Some("Hello"));
    assert_eq!(text.width(), 120.0);
    assert_eq!(text.margin(), Thickness::uniform(4.0));
    assert_eq!(text.padding(), Thickness::symmetric(2.0, 6.0));
    assert_eq!(text.horizontal_alignment(), HorizontalAlignment::Center);
    assert_eq!(text.name().as_deref(), Some("greeting"));
    assert!(text.classes().contains("title") && text.classes().contains("large"));
    owner.dispose();
}

#[test]
fn set_takes_any_property_like_the_indexer_upstream() {
    let (control, owner) = mount(button().set(Grid::row_property(), 2).grid_column(1));
    assert_eq!(Grid::get_row(&control), 2);
    assert_eq!(Grid::get_column(&control), 1);
    owner.dispose();
}

#[test]
fn text_children_go_into_the_content_and_controls_into_a_panel() {
    let (control, owner) = mount(
        stack_panel()
            .spacing(8.0)
            .child(button().child("First"))
            .children((text_block().text("Second"), button().content("Third"))),
    );
    let panel = control.cast::<StackPanel>().expect("a panel");
    let children: Vec<Ref<Control>> = panel.children().to_vec().into_iter().collect();
    assert_eq!(children.len(), 3);
    assert_eq!(content_text(&children[0].cast::<Button>().unwrap()).as_deref(), Some("First"));
    assert_eq!(children[1].cast::<TextBlock>().unwrap().text().as_deref(), Some("Second"));
    assert_eq!(content_text(&children[2].cast::<Button>().unwrap()).as_deref(), Some("Third"));
    owner.dispose();
}

#[test]
fn an_element_given_to_an_object_property_becomes_its_control() {
    let (control, owner) = mount(button().content(text_block().text("Inside")));
    let button = control.cast::<Button>().unwrap();
    let content = button.content().and_then(|content| Control::from_boxed(&content)).expect("a control");
    assert_eq!(content.cast::<TextBlock>().unwrap().text().as_deref(), Some("Inside"));
    owner.dispose();
}

#[test]
fn a_property_follows_a_signal_and_a_closure() {
    let owner = Owner::new_root();
    let (count, control) = owner.with(|| {
        let count = signal(1);
        let control = stack_panel()
            .child(text_block().text(count))
            .child(text_block().text(move || format!("Count: {}", count.get())))
            .into_element()
            .mount();
        (count, control)
    });
    let texts = || -> Vec<String> {
        control.cast::<StackPanel>().unwrap().children().to_vec().into_iter().map(|c| c.cast::<TextBlock>().unwrap().text().unwrap_or_default()).collect()
    };
    assert_eq!(texts(), vec!["1", "Count: 1"]);
    count.set(7);
    assert_eq!(texts(), vec!["7", "Count: 7"]);
    owner.dispose();
}

#[test]
fn a_two_way_signal_follows_the_property_and_the_property_the_signal() {
    let owner = Owner::new_root();
    let (draft, control) = owner.with(|| {
        let draft = signal(String::from("start"));
        let control = text_box().text(draft.two_way()).into_element().mount();
        (draft, control)
    });
    let text_box = control.cast::<TextBox>().unwrap();
    assert_eq!(text_box.text().as_deref(), Some("start"));
    text_box.set_text(Some("typed"));
    assert_eq!(draft.get_untracked(), "typed");
    draft.set(String::from("from code"));
    assert_eq!(text_box.text().as_deref(), Some("from code"));
    owner.dispose();
}

#[test]
fn a_handler_of_a_routed_event_is_called() {
    let clicks = Rc::new(Cell::new(0));
    let counted = clicks.clone();
    let (control, owner) = mount(button().on_click(move |_| counted.set(counted.get() + 1)));
    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(clicks.get(), 2);
    owner.dispose();
}

#[test]
fn on_changed_reports_the_new_value_of_a_property() {
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let log = seen.clone();
    let (control, owner) = mount(slider().minimum(0.0).maximum(10.0).on_changed(
        ferroui_controls::primitives::RangeBase::value_property(),
        move |value| log.borrow_mut().push(*value),
    ));
    control.set_value(ferroui_controls::primitives::RangeBase::value_property(), 4.0);
    assert_eq!(*seen.borrow(), vec![4.0]);
    owner.dispose();
}

#[test]
fn grid_definitions_and_attached_properties() {
    let (control, owner) = mount(
        grid()
            .row_definitions("Auto,*")
            .column_definitions("*,Auto")
            .child(text_box().grid_row(0).grid_column(0))
            .child(button().grid_row(0).grid_column(1).child("Add")),
    );
    let grid = control.cast::<Grid>().unwrap();
    assert_eq!(grid.row_definitions().count(), 2);
    assert_eq!(grid.column_definitions().count(), 2);
    let add = grid.children().to_vec().into_iter().nth(1).unwrap();
    assert_eq!(Grid::get_column(&add), 1);
    owner.dispose();
}

#[test]
fn an_element_is_data_that_compares_by_value() {
    let a: Element = button().width(10.0).child("x").into_element();
    let b: Element = button().width(10.0).child("x").into_element();
    let (Element::Control(a), Element::Control(b)) = (a, b) else { panic!("control elements") };
    assert_eq!(a.class, b.class);
    assert_eq!(a.props, b.props);
    let c = button().width(11.0).into_element();
    let Element::Control(c) = c else { panic!() };
    assert_ne!(a.props, c.props);
}
