use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{Element, GridDefinitions, IntoElement};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::Color;
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
fn elements_that_name_a_color_share_its_brush() {
    let (first, first_owner) = mount(text_block().text("a").foreground("#1E293B"));
    let (second, second_owner) = mount(text_block().text("b").foreground("#1e293b"));
    let (third, third_owner) = mount(text_block().text("c").foreground(Color::from_rgb(0x1E, 0x29, 0x3B)));
    let (other, other_owner) = mount(text_block().text("d").foreground("#1E293C"));
    let brush = |control: &Ref<Control>| {
        control.cast::<TextBlock>().expect("a text block").foreground().expect("a brush")
    };
    let color = |control: &Ref<Control>| brush(control).as_solid_color_brush().map(|solid| solid.color());

    // One color, written three ways, is one brush; another color is another.
    assert!(Rc::ptr_eq(&brush(&first), &brush(&second)));
    assert!(Rc::ptr_eq(&brush(&first), &brush(&third)));
    assert!(!Rc::ptr_eq(&brush(&first), &brush(&other)));
    assert_eq!(color(&first), Some(Color::from_rgb(0x1E, 0x29, 0x3B)));
    // A name is a color too.
    let (named, named_owner) = mount(text_block().text("e").foreground("Red"));
    assert_eq!(color(&named), Some(Color::from_rgb(0xFF, 0, 0)));

    for owner in [first_owner, second_owner, third_owner, other_owner, named_owner] {
        owner.dispose();
    }
}

/// Elements that give a property the same plain value hold the same box of it (section 9.100):
/// a width, an alignment and a margin are plain; a text has a box of its own. More values than
/// are kept change nothing but the boxes.
#[test]
fn elements_that_give_the_same_plain_value_share_its_box() {
    use crate::element::PropValue;
    let boxes = |element: Element| -> Vec<BoxedValue> {
        let Element::Control(control) = element else { panic!("an element of a control") };
        let values = control.props.iter().map(|entry| match &entry.value {
            PropValue::Value(value) => value.clone(),
            _ => panic!("a value"),
        });
        values.collect()
    };
    let tile = |text: &str| {
        text_block()
            .width(64.0)
            .horizontal_alignment(HorizontalAlignment::Center)
            .margin((4.0, 2.0, 0.0, 0.0))
            .text(text.to_string())
            .into_element()
    };
    let (first, second) = (boxes(tile("same")), boxes(tile("same")));
    assert_eq!(first.len(), 4);
    for index in 0..3 {
        assert!(Rc::ptr_eq(&first[index], &second[index]), "the plain value {index} has one box");
    }
    assert!(!Rc::ptr_eq(&first[3], &second[3]), "a text has a box of its own");
    assert_eq!(first[3].downcast_ref::<Option<String>>(), second[3].downcast_ref::<Option<String>>());
    assert_eq!(first[0].downcast_ref::<f64>(), Some(&64.0));
    assert_eq!(first[2].downcast_ref::<Thickness>(), Some(&Thickness::new(4.0, 2.0, 0.0, 0.0)));

    // A hundred other widths pass through; each element still has its own value.
    for width in 0..100 {
        let made = boxes(text_block().width(f64::from(width)).into_element());
        assert_eq!(made[0].downcast_ref::<f64>(), Some(&f64::from(width)));
    }
    let (control, owner) = mount(tile("mounted"));
    let text = control.cast::<TextBlock>().expect("a text block");
    assert_eq!((text.width(), text.margin()), (64.0, Thickness::new(4.0, 2.0, 0.0, 0.0)));
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
