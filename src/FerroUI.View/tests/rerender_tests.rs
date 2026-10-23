//! Components that run again: the reconciler and the hooks.

use crate::generated::builders::*;
use crate::reactive::{batch, on_cleanup, signal, Owner};
use crate::{
    component, for_each, provide_context, rerender, rsx, use_context, use_effect, use_effect_with, use_hook, use_memo,
    use_signal, Element, IntoElement,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, Control, ListBox, StackPanel, TextBlock};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn mount(element: impl IntoElement) -> (Ref<Control>, Owner) {
    element.into_element().mount_root()
}

fn children(panel: &Ref<Control>) -> Vec<Ref<Control>> {
    panel.cast::<StackPanel>().expect("a panel").children().to_vec().into_iter().collect()
}

fn texts(panel: &Ref<Control>) -> Vec<String> {
    children(panel)
        .into_iter()
        .map(|child| {
            if let Some(text) = child.cast::<TextBlock>() {
                return text.text().unwrap_or_default();
            }
            if let Some(button) = child.cast::<Button>() {
                return button.content().and_then(|c: BoxedValue| c.downcast_ref::<String>().cloned()).unwrap_or_default();
            }
            String::from("?")
        })
        .collect()
}

fn click(control: &Ref<Control>) {
    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn a_component_runs_again_when_a_signal_it_read_changes_and_keeps_its_controls() {
    let owner = Owner::new_root();
    let (count, runs, panel) = owner.with(|| {
        let count = signal(0);
        let runs = Rc::new(Cell::new(0));
        let counted = runs.clone();
        let panel = stack_panel()
            .child(rerender(move || {
                counted.set(counted.get() + 1);
                text_block().text(format!("Count: {}", count.get()))
            }))
            .into_element()
            .mount();
        (count, runs, panel)
    });
    assert_eq!(texts(&panel), vec!["Count: 0"]);
    let first = children(&panel)[0].clone();
    count.set(1);
    assert_eq!(texts(&panel), vec!["Count: 1"]);
    assert_eq!(runs.get(), 2);
    assert!(first.ptr_eq(&children(&panel)[0]), "an element of the same class keeps its control");
    owner.dispose();
}

#[test]
fn equal_values_are_left_alone_and_values_that_are_gone_are_cleared() {
    let owner = Owner::new_root();
    let (step, control) = owner.with(|| {
        let step = signal(0);
        let control = rerender(move || {
            let element = text_block().width(100.0).text(format!("{}", step.get()));
            if step.get() == 0 {
                element.height(40.0).class("first")
            } else {
                element.class("second")
            }
        })
        .into_element()
        .mount();
        (step, control)
    });
    let text = control.cast::<TextBlock>().unwrap();
    let width_changes = Rc::new(Cell::new(0));
    let counted = width_changes.clone();
    let _subscription = control.property_changed(move |e| {
        if std::ptr::eq(e.property(), ferroui_base::layout::Layoutable::width_property().as_property()) {
            counted.set(counted.get() + 1);
        }
    });
    assert_eq!(text.height(), 40.0);
    assert!(text.classes().contains("first"));
    step.set(1);
    assert_eq!(text.text().as_deref(), Some("1"));
    assert_eq!(width_changes.get(), 0, "an equal value is not set again");
    assert!(text.height().is_nan(), "a value that is gone is cleared");
    assert!(!text.classes().contains("first") && text.classes().contains("second"));
    owner.dispose();
}

#[test]
fn children_are_matched_by_key_and_by_place_and_another_class_replaces_a_control() {
    let owner = Owner::new_root();
    let (order, panel) = owner.with(|| {
        let order = signal(vec![1, 2, 3]);
        let panel = stack_panel()
            .child(rerender(move || {
                let mut items: Vec<Element> =
                    order.get().into_iter().map(|id| text_block().key(id).text(format!("item {id}")).into()).collect();
                if order.with(|order| order.len()) == 2 {
                    items.push(button().content("button").into());
                } else {
                    items.push(text_block().text("text").into());
                }
                items
            }))
            .into_element()
            .mount();
        (order, panel)
    });
    assert_eq!(texts(&panel), vec!["item 1", "item 2", "item 3", "text"]);
    let before = children(&panel);
    order.set(vec![3, 1]);
    assert_eq!(texts(&panel), vec!["item 3", "item 1", "button"]);
    let after = children(&panel);
    assert!(after[0].ptr_eq(&before[2]) && after[1].ptr_eq(&before[0]), "keyed children keep their controls");
    assert!(after[2].cast::<Button>().is_some(), "an element of another class replaces the control");
    order.set(vec![3, 1, 4]);
    assert_eq!(texts(&panel), vec!["item 3", "item 1", "item 4", "text"]);
    owner.dispose();
}

#[test]
fn handlers_are_attached_again_and_batched_writes_run_the_component_once() {
    let owner = Owner::new_root();
    let (control, runs, seen) = owner.with(|| {
        let runs = Rc::new(Cell::new(0));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (counted, log) = (runs.clone(), seen.clone());
        let control = rerender(move || {
            counted.set(counted.get() + 1);
            let a = use_signal(|| 0);
            let b = use_signal(|| 0);
            let log = log.clone();
            let value = a.get() + b.get();
            button().content(format!("{value}")).on_click(move |_| {
                log.borrow_mut().push(value);
                batch(|| {
                    a.update(|a| *a += 1);
                    b.update(|b| *b += 1);
                });
            })
        })
        .into_element()
        .mount();
        (control, runs, seen)
    });
    click(&control);
    click(&control);
    assert_eq!(*seen.borrow(), vec![0, 2], "the handler of the latest run is the only one attached");
    assert_eq!(runs.get(), 3, "two writes in a batch run the component once");
    owner.dispose();
}

#[test]
fn hooks_keep_state_across_runs_and_a_child_keeps_its_state_when_its_parent_runs_again() {
    #[component(rerender)]
    fn Counter(label: String) -> Element {
        let count = use_signal(|| 0);
        rsx! {
            Button { on_click: move |_| count.update(|c| *c += 1), {format!("{label}: {}", count.get())} }
        }
    }

    let owner = Owner::new_root();
    let (prefix, panel) = owner.with(|| {
        let prefix = signal(String::from("A"));
        let panel = stack_panel()
            .child(rerender(move || rsx! { Counter { label: prefix.get() } }))
            .into_element()
            .mount();
        (prefix, panel)
    });
    assert_eq!(texts(&panel), vec!["A: 0"]);
    let button = children(&panel)[0].clone();
    click(&button);
    click(&button);
    assert_eq!(texts(&panel), vec!["A: 2"]);
    prefix.set(String::from("B"));
    assert_eq!(texts(&panel), vec!["B: 2"], "the child runs again with the new value and keeps its state");
    assert!(button.ptr_eq(&children(&panel)[0]));
    owner.dispose();
}

#[test]
fn a_memoized_component_does_not_run_again_for_equal_values() {
    ferroui_base::ferro_thread_local! {
        static RUNS: Cell<u32> = Cell::new(0);
    }

    #[component(rerender, memo)]
    fn Label(text: String) -> Element {
        RUNS.with(|runs| runs.set(runs.get() + 1));
        rsx! { TextBlock { text: text } }
    }

    let owner = Owner::new_root();
    let (tick, label, panel) = owner.with(|| {
        let tick = signal(0);
        let label = signal(String::from("x"));
        let panel = stack_panel()
            .child(rerender(move || {
                let _ = tick.get();
                rsx! { Label { text: label.get() } }
            }))
            .into_element()
            .mount();
        (tick, label, panel)
    });
    assert_eq!(RUNS.with(Cell::get), 1);
    tick.set(1);
    assert_eq!(RUNS.with(Cell::get), 1, "equal values: the child does not run again");
    label.set(String::from("y"));
    assert_eq!(RUNS.with(Cell::get), 2);
    assert_eq!(texts(&panel), vec!["y"]);
    owner.dispose();
}

#[test]
fn effects_run_once_the_controls_are_in_place_and_clean_up() {
    let owner = Owner::new_root();
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let (deps, show_child) = owner.with(|| {
        let deps = signal(1);
        let show_child = signal(true);
        let log = log.clone();
        let _panel = stack_panel()
            .child(rerender(move || {
                if !show_child.get() {
                    return Element::Empty;
                }
                let log = log.clone();
                rerender(move || {
                    let value = deps.get();
                    let (effect_log, deps_log) = (log.clone(), log.clone());
                    use_effect(move || effect_log.borrow_mut().push(String::from("effect")));
                    use_effect_with(value, move |value| {
                        let value = *value;
                        deps_log.borrow_mut().push(format!("run {value}"));
                        let cleanup_log = deps_log.clone();
                        on_cleanup(move || cleanup_log.borrow_mut().push(format!("cleanup {value}")));
                    });
                    text_block().text(format!("{value}")).into_element()
                })
            }))
            .into_element()
            .mount();
        (deps, show_child)
    });
    assert_eq!(*log.borrow(), vec!["effect", "run 1"]);
    deps.set(2);
    assert_eq!(*log.borrow(), vec!["effect", "run 1", "cleanup 1", "run 2"]);
    show_child.set(false);
    assert_eq!(log.borrow().last().map(String::as_str), Some("cleanup 2"), "taking the component out cleans up");
    owner.dispose();
}

#[test]
fn context_reaches_the_components_below() {
    #[derive(Clone, PartialEq)]
    struct Theme(&'static str);

    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(rerender(|| {
                provide_context(Theme("dark"));
                rerender(|| {
                    let theme = use_context::<Theme>().map_or("none", |theme| theme.0);
                    text_block().text(theme)
                })
            }))
            .into_element()
            .mount()
    });
    assert_eq!(texts(&panel), vec!["dark"]);
    owner.dispose();
}

#[test]
fn use_hook_and_use_memo_are_made_once() {
    let owner = Owner::new_root();
    let made = Rc::new(Cell::new(0));
    let (count, control) = owner.with(|| {
        let count = signal(1);
        let made = made.clone();
        let control = rerender(move || {
            let made = made.clone();
            let id = use_hook(move || {
                made.set(made.get() + 1);
                42
            });
            let doubled = use_memo(move || count.get() * 2);
            text_block().text(format!("{id} {}", doubled.get()))
        })
        .into_element()
        .mount();
        (count, control)
    });
    count.set(5);
    assert_eq!(control.cast::<TextBlock>().unwrap().text().as_deref(), Some("42 10"));
    assert_eq!(made.get(), 1);
    owner.dispose();
}

#[test]
#[should_panic(expected = "hooks are called in the same order")]
fn hooks_in_a_changing_order_panic() {
    let owner = Owner::new_root();
    let flag = owner.with(|| {
        let flag = signal(true);
        let _control = rerender(move || {
            if flag.get() {
                let _ = use_signal(|| 0);
            } else {
                let _ = use_hook(|| String::new());
            }
            text_block()
        })
        .into_element()
        .mount();
        flag
    });
    flag.set(false);
}

#[test]
fn a_list_in_a_component_that_runs_again_renders_its_kept_items_again() {
    let owner = Owner::new_root();
    let (suffix, panel) = owner.with(|| {
        let suffix = signal("!");
        let items = signal(vec![1, 2]);
        let panel = stack_panel()
            .child(rerender(move || {
                let suffix = suffix.get();
                for_each(move || items.get(), |id| *id, move |id| text_block().text(format!("{id}{suffix}")))
            }))
            .into_element()
            .mount();
        (suffix, panel)
    });
    assert_eq!(texts(&panel), vec!["1!", "2!"]);
    let first = children(&panel)[0].clone();
    suffix.set("?");
    assert_eq!(texts(&panel), vec!["1?", "2?"]);
    assert!(first.ptr_eq(&children(&panel)[0]), "an item that stays keeps its control and is patched");
    owner.dispose();
}

#[test]
fn rsx_conditions_and_loops_in_a_component_that_runs_again() {
    #[component(rerender)]
    fn TodoList(items: Vec<&'static str>) -> Element {
        let done = use_signal(|| false);
        rsx! {
            ListBox {
                for item in items.iter().copied() {
                    ListBoxItem { key: item, content: item }
                }
                if done.get() {
                    ListBoxItem { content: "all done" }
                }
            }
        }
    }

    let owner = Owner::new_root();
    let (items, control) = owner.with(|| {
        let items = signal(vec!["a", "b"]);
        let control = rerender(move || rsx! { TodoList { items: items.get() } }).into_element().mount();
        (items, control)
    });
    let list = control.cast::<ListBox>().unwrap();
    assert_eq!(list.items().count(), 2);
    batch(|| items.set(vec!["b", "c", "d"]));
    assert_eq!(list.items().count(), 3);
    owner.dispose();
}

#[test]
fn a_component_follows_the_data_context_of_the_control_it_is_in() {
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(rerender(|| {
                let name = crate::hooks::use_data_context::<String>().unwrap_or_else(|| String::from("none"));
                text_block().text(name)
            }))
            .into_element()
            .mount()
    });
    assert_eq!(texts(&panel), vec!["none"]);
    let first = children(&panel)[0].clone();
    panel.set_data_context(Some(Rc::new(String::from("Ada")) as BoxedValue));
    assert_eq!(texts(&panel), vec!["Ada"], "the data context is inherited and followed");
    assert!(first.ptr_eq(&children(&panel)[0]));
    owner.dispose();
}

#[test]
fn an_element_of_another_name_is_another_control() {
    let owner = Owner::new_root();
    let (named, panel) = owner.with(|| {
        let named = signal("Pending");
        let panel = stack_panel()
            .child(rerender(move || text_block().name(named.get()).text(named.get())))
            .into_element()
            .mount();
        (named, panel)
    });
    let first = children(&panel)[0].clone();
    named.set("Ready");
    let second = children(&panel)[0].clone();
    // A styled control keeps its name, so the reconciler makes another one instead of renaming it.
    assert!(!first.ptr_eq(&second));
    assert_eq!(second.name().as_deref(), Some("Ready"));
    assert_eq!(texts(&panel), vec!["Ready"]);
    named.set("Ready");
    assert!(second.ptr_eq(&children(&panel)[0]), "the same name keeps the control");
    owner.dispose();
}
