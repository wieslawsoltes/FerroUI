//! Phase 5: the fewest moves of keyed children, handlers and values that
//! skip work.

use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{rerender, view_stats, Element, IntoElement};

fn host_operations() -> u64 {
    view_stats().host_operations
}
use ferroui_base::Ref;
use ferroui_controls::{Control, StackPanel, TextBlock};

fn mount_list(order: crate::Signal<Vec<u32>>) -> Ref<StackPanel> {
    let panel = rerender(move || {
        stack_panel().children(order.get().into_iter().map(|id| text_block().key(id).text(id.to_string())).collect::<Vec<_>>())
    })
    .into_element()
    .mount();
    panel.cast::<StackPanel>().unwrap()
}

fn shown(panel: &Ref<StackPanel>) -> Vec<String> {
    panel.children().to_vec().into_iter().map(|c| c.cast::<TextBlock>().unwrap().text().unwrap_or_default()).collect()
}

fn controls(panel: &Ref<StackPanel>) -> Vec<Ref<Control>> {
    panel.children().to_vec().into_iter().collect()
}

#[test]
fn keyed_children_are_reordered_with_the_fewest_moves() {
    let owner = Owner::new_root();
    let order = owner.with(|| signal(vec![1, 2, 3, 4, 5]));
    let panel = owner.with(|| mount_list(order));
    let before = controls(&panel);

    let operations = host_operations();
    order.set(vec![5, 1, 2, 3, 4]);
    assert_eq!(shown(&panel), ["5", "1", "2", "3", "4"]);
    assert_eq!(host_operations() - operations, 1, "the last child moved to the front is one move");

    let operations = host_operations();
    order.set(vec![1, 2, 3, 4, 5]);
    assert_eq!(shown(&panel), ["1", "2", "3", "4", "5"]);
    assert_eq!(host_operations() - operations, 1, "back in order is one move");

    let operations = host_operations();
    order.set(vec![5, 4, 3, 2, 1]);
    assert_eq!(shown(&panel), ["5", "4", "3", "2", "1"]);
    assert_eq!(host_operations() - operations, 4, "five children reversed are four moves (the prefix and suffix rule took five removals and five insertions)");
    let after = controls(&panel);
    for control in &before {
        assert!(after.iter().any(|c| c.ptr_eq(control)), "every control is kept");
    }
    owner.dispose();
}

#[test]
fn random_edits_of_a_keyed_list_give_the_list_and_keep_the_controls() {
    // A small generator of the sequences, the same on every run.
    let mut seed: u64 = 0x5eed_1234;
    let mut next = move |bound: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % bound
    };
    let owner = Owner::new_root();
    let order = owner.with(|| signal((0..20_u32).collect::<Vec<_>>()));
    let panel = owner.with(|| mount_list(order));
    let mut fresh = 100_u32;
    for _ in 0..300 {
        let mut list = order.get_untracked();
        for _ in 0..(1 + next(4)) {
            match next(4) {
                0 if !list.is_empty() => {
                    let at = next(list.len() as u64) as usize;
                    list.remove(at);
                }
                1 => {
                    let at = next(list.len() as u64 + 1) as usize;
                    list.insert(at, fresh);
                    fresh += 1;
                }
                _ if list.len() > 1 => {
                    let from = next(list.len() as u64) as usize;
                    let item = list.remove(from);
                    let to = next(list.len() as u64 + 1) as usize;
                    list.insert(to, item);
                }
                _ => {}
            }
        }
        let kept: Vec<(u32, Ref<Control>)> = order
            .get_untracked()
            .into_iter()
            .zip(controls(&panel))
            .filter(|(id, _)| list.contains(id))
            .collect();
        order.set(list.clone());
        let expected: Vec<String> = list.iter().map(u32::to_string).collect();
        assert_eq!(shown(&panel), expected);
        let now = controls(&panel);
        for (id, control) in kept {
            let at = list.iter().position(|x| *x == id).unwrap();
            assert!(now[at].ptr_eq(&control), "the control of {id} is kept");
        }
    }
    owner.dispose();
}

#[test]
fn the_longest_increasing_subsequence_is_found() {
    use crate::mount::longest_increasing_subsequence as lis;
    assert_eq!(lis(&[]).len(), 0);
    assert_eq!(lis(&[3, 1, 2]).len(), 2);
    assert_eq!(lis(&[0, 8, 4, 12, 2, 10, 6, 14, 1, 9]).len(), 4);
    let reversed: Vec<usize> = (0..10).rev().collect();
    assert_eq!(lis(&reversed).len(), 1);
    let _ = Element::Empty;
}

#[test]
fn a_component_that_runs_again_swaps_its_handlers_and_keeps_values_that_follow_the_same_signal() {
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::Button;
    use std::cell::Cell;
    use std::rc::Rc;
    let owner = Owner::new_root();
    let clicks = Rc::new(Cell::new(0));
    let (tick, label, control) = owner.with(|| {
        let tick = signal(0);
        let label = signal(String::from("a"));
        let counted = clicks.clone();
        let control = rerender(move || {
            let run = tick.get();
            let counted = counted.clone();
            button().content(label).on_click(move |_| counted.set(counted.get() + run + 1)).into_element()
        })
        .into_element()
        .mount();
        (tick, label, control)
    });
    let before = view_stats();
    tick.set(10);
    let after = view_stats();
    assert_eq!(after.handler_attachments, before.handler_attachments, "the handler's closure is swapped, not attached again");
    assert_eq!(after.value_starts, before.value_starts, "a value that follows the same signal is not started again");
    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(clicks.get(), 11, "the closure of the latest run handles the click");
    label.set(String::from("b"));
    let content = control.cast::<Button>().unwrap().content().and_then(|c| c.downcast_ref::<String>().cloned());
    assert_eq!(content.as_deref(), Some("b"), "the value still follows its signal");
    owner.dispose();
}

#[test]
fn components_with_comparable_parameters_are_memoized_by_default() {
    use crate::component;
    use std::cell::Cell;
    use std::rc::Rc;
    ferroui_base::ferro_thread_local! {
        static DEFAULT_RUNS: Cell<u32> = Cell::new(0);
        static ALWAYS_RUNS: Cell<u32> = Cell::new(0);
        static CLOSURE_RUNS: Cell<u32> = Cell::new(0);
    }
    #[component(rerender)]
    fn Plain(text: String) -> Element {
        DEFAULT_RUNS.with(|runs| runs.set(runs.get() + 1));
        text_block().text(text).into_element()
    }
    #[component(rerender, always)]
    fn Always(text: String) -> Element {
        ALWAYS_RUNS.with(|runs| runs.set(runs.get() + 1));
        text_block().text(text).into_element()
    }
    #[component(rerender)]
    fn WithClosure(text: String, on_click: Rc<dyn Fn()>) -> Element {
        CLOSURE_RUNS.with(|runs| runs.set(runs.get() + 1));
        let _ = on_click;
        text_block().text(text).into_element()
    }
    let owner = Owner::new_root();
    let tick = owner.with(|| {
        let tick = signal(0);
        let _ = rerender(move || {
            let _ = tick.get();
            let on_click: Rc<dyn Fn()> = Rc::new(|| {});
            stack_panel()
                .child(plain().text("a"))
                .child(always().text("a"))
                .child(with_closure().text("a").on_click(on_click))
        })
        .into_element()
        .mount();
        tick
    });
    tick.set(1);
    tick.set(2);
    assert_eq!(DEFAULT_RUNS.with(Cell::get), 1, "equal comparable parameters: not run again");
    assert_eq!(ALWAYS_RUNS.with(Cell::get), 3, "`always`: runs with its parent");
    assert_eq!(CLOSURE_RUNS.with(Cell::get), 3, "parameters that cannot be compared: runs with its parent");
    owner.dispose();
}
