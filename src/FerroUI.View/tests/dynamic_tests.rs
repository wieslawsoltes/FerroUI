use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{dynamic, for_each, show, show_else, IntoElement};
use ferroui_base::Ref;
use ferroui_controls::{Control, ListBox, StackPanel, TextBlock};

fn texts(panel: &Ref<Control>) -> Vec<String> {
    panel
        .cast::<StackPanel>()
        .unwrap()
        .children()
        .to_vec()
        .into_iter()
        .map(|child| child.cast::<TextBlock>().and_then(|t| t.text()).unwrap_or_else(|| String::from("?")))
        .collect()
}

#[derive(Clone, PartialEq)]
struct Item {
    id: u32,
    title: String,
}

#[test]
fn a_keyed_list_keeps_the_controls_of_items_that_stay() {
    let owner = Owner::new_root();
    let (items, panel) = owner.with(|| {
        let items = signal(vec![Item { id: 1, title: "a".into() }, Item { id: 2, title: "b".into() }]);
        let panel = stack_panel()
            .child(text_block().text("header"))
            .child(for_each(move || items.get(), |item| item.id, |item| text_block().text(item.title)))
            .child(text_block().text("footer"))
            .into_element()
            .mount();
        (items, panel)
    });
    assert_eq!(texts(&panel), vec!["header", "a", "b", "footer"]);
    let first = panel.cast::<StackPanel>().unwrap().children().to_vec().into_iter().nth(1).unwrap();

    items.update(|list| list.push(Item { id: 3, title: "c".into() }));
    assert_eq!(texts(&panel), vec!["header", "a", "b", "c", "footer"]);
    let still_first = panel.cast::<StackPanel>().unwrap().children().to_vec().into_iter().nth(1).unwrap();
    assert!(Ref::ptr_eq(&first, &still_first), "the control of an item that stays is kept");

    items.update(|list| {
        list.remove(0);
        list.reverse();
    });
    assert_eq!(texts(&panel), vec!["header", "c", "b", "footer"]);

    items.update(|list| list[0].title = "C".into());
    assert_eq!(texts(&panel), vec!["header", "C", "b", "footer"], "an item whose value changed is made again");
    owner.dispose();
}

#[test]
fn show_adds_and_removes_its_children_with_the_condition() {
    let owner = Owner::new_root();
    let (visible, panel) = owner.with(|| {
        let visible = signal(false);
        let panel = stack_panel()
            .child(show(move || visible.get(), || text_block().text("shown")))
            .child(show_else(move || visible.get(), || text_block().text("yes"), || text_block().text("no")))
            .into_element()
            .mount();
        (visible, panel)
    });
    assert_eq!(texts(&panel), vec!["no"]);
    visible.set(true);
    assert_eq!(texts(&panel), vec!["shown", "yes"]);
    visible.set(false);
    assert_eq!(texts(&panel), vec!["no"]);
    owner.dispose();
}

#[test]
fn dynamic_children_of_an_items_control_are_its_items() {
    let owner = Owner::new_root();
    let (names, list) = owner.with(|| {
        let names = signal(vec!["Emil", "Max"]);
        let list = list_box()
            .child(dynamic(move || names.get().into_iter().map(|name| list_box_item().content(name).key(name)).collect::<Vec<_>>()))
            .into_element()
            .mount();
        (names, list)
    });
    let list = list.cast::<ListBox>().unwrap();
    assert_eq!(list.items().count(), 2);
    names.set(vec!["Emil", "Max", "Tisch"]);
    assert_eq!(list.items().count(), 3);
    names.set(vec!["Tisch"]);
    assert_eq!(list.items().count(), 1);
    owner.dispose();
}

#[test]
fn the_signals_of_an_item_end_with_it() {
    let owner = Owner::new_root();
    let (items, inner) = owner.with(|| {
        let items = signal(vec![1]);
        let inner = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let made = inner.clone();
        let _panel = stack_panel()
            .child(for_each(move || items.get(), |item| *item, move |item| {
                made.borrow_mut().push(signal(item));
                text_block().text(item.to_string())
            }))
            .into_element()
            .mount();
        (items, inner)
    });
    items.set(vec![]);
    assert!(!inner.borrow()[0].is_alive(), "the owner of a removed item is disposed");
    owner.dispose();
}
