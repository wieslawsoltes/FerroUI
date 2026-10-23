//! Phase 4: the diagnostics and hot reload of `rsx!`.

use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{rerender, rsx, set_diagnostics_handler, clear_diagnostics_handler, Diagnostic, Element, IntoElement};
use ferroui_controls::{StackPanel, TextBlock};
use std::cell::RefCell;
use std::rc::Rc;

fn collect() -> Rc<RefCell<Vec<Diagnostic>>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    set_diagnostics_handler(move |diagnostic| sink.borrow_mut().push(diagnostic.clone()));
    seen
}

#[cfg(debug_assertions)]
#[test]
fn a_list_without_keys_and_duplicate_keys_are_reported() {
    let seen = collect();
    let owner = Owner::new_root();
    owner.with(|| {
        let names = ["a", "b", "c"];
        let _ = rsx! {
            StackPanel {
                for name in names { TextBlock { text: name } }
            }
        }
        .mount();
        let _ = stack_panel()
            .child(text_block().key(1).text("one"))
            .child(text_block().key(1).text("again"))
            .into_element()
            .mount();
    });
    let seen = seen.borrow();
    assert!(seen.iter().any(|d| matches!(d, Diagnostic::MissingKeys { items: 3, first } if first == "TextBlock")), "{seen:?}");
    assert!(seen.iter().any(|d| matches!(d, Diagnostic::DuplicateKey { .. })), "{seen:?}");
    drop(seen);
    clear_diagnostics_handler();
    owner.dispose();
}

#[cfg(debug_assertions)]
#[test]
fn a_component_that_writes_what_it_reads_is_reported() {
    let seen = collect();
    let owner = Owner::new_root();
    owner.with(|| {
        let count = signal(0);
        let _ = rerender(move || {
            let value = count.get();
            if value < 150 {
                count.set(value + 1);
            }
            text_block().text(value.to_string())
        })
        .into_element()
        .mount();
    });
    assert!(seen.borrow().iter().any(|d| matches!(d, Diagnostic::RunawayComponent { runs: 101, .. })), "{:?}", seen.borrow());
    clear_diagnostics_handler();
    owner.dispose();
}

#[cfg(feature = "hot-reload")]
#[test]
fn hot_reload_changes_the_literals_of_a_component_and_keeps_its_controls() {
    let seen = collect();
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        rerender(|| {
            let element: Element = rsx! {
                StackPanel { spacing: 4.0,
                    TextBlock { name: "Label", text: "HOT_A", width: 80.0 }
                    "HOT_TEXT_A"
                }
            };
            element
        })
        .into_element()
        .mount()
    });
    let panel = panel.cast::<StackPanel>().unwrap();
    let label = || panel.children().to_vec()[0].cast::<TextBlock>().unwrap();
    let first = label();
    assert_eq!(first.text().as_deref(), Some("HOT_A"));
    assert_eq!(first.width(), 80.0);

    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/tools_tests.rs")).expect("the source");
    let edited = source
        .replacen(&format!("\"HOT_{}\"", "A"), "\"HOT_B\"", 1)
        .replacen("width: 80.0 }", "width: 120.0 }", 1)
        .replacen(&format!("\"HOT_TEXT_{}\"", "A"), "\"HOT_TEXT_B\"", 1);
    crate::hot::apply_source(file!(), &edited);
    assert_eq!(label().text().as_deref(), Some("HOT_B"));
    assert_eq!(label().width(), 120.0);
    assert!(first.ptr_eq(&label()), "the controls are patched, not made again");
    let texts: Vec<String> = panel.children().to_vec().into_iter().filter_map(|c| c.cast::<TextBlock>()).filter_map(|t| t.text()).collect();
    assert!(texts.contains(&String::from("HOT_TEXT_B")), "{texts:?}");
    assert!(seen.borrow().iter().any(|d| matches!(d, Diagnostic::HotReloaded { literals: 3, .. })), "{:?}", seen.borrow());

    let structural = source.replacen("spacing: 4.0,", "spacing: 4.0, margin: 2.0,", 1);
    crate::hot::apply_source(file!(), &structural);
    assert!(seen.borrow().iter().any(|d| matches!(d, Diagnostic::NeedsRebuild { .. })), "{:?}", seen.borrow());
    crate::hot::apply_source(file!(), &source);
    assert_eq!(label().text().as_deref(), Some("HOT_A"), "the source as built gives the literals back");
    clear_diagnostics_handler();
    owner.dispose();
}

#[test]
fn the_inspector_describes_the_mounted_trees() {
    use crate::{for_each, mounted_trees, InstanceKind, InstanceNode};
    let owner = Owner::new_root();
    let items = owner.with(|| {
        let items = signal(vec![1, 2]);
        let count = signal(0);
        let _ = rerender(move || {
            stack_panel()
                .name("Root")
                .class("panel")
                .child(text_block().text(move || format!("{}", count.get())))
                .child(for_each(move || items.get(), |n| *n, |n| text_block().text(format!("item {n}"))))
        })
        .into_element()
        .mount();
        items
    });
    fn find<'a>(node: &'a InstanceNode, test: &dyn Fn(&InstanceNode) -> bool) -> Option<&'a InstanceNode> {
        if test(node) {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, test))
    }
    let trees = mounted_trees();
    let component = trees
        .iter()
        .flat_map(|tree| tree.roots.iter())
        .find_map(|root| find(root, &|node| matches!(&node.kind, InstanceKind::Component { runs: 1, .. })))
        .expect("the component");
    let panel = find(component, &|node| matches!(&node.kind, InstanceKind::Control { name: Some(name), .. } if name == "Root")).expect("the panel");
    let InstanceKind::Control { class, classes, .. } = &panel.kind else { unreachable!() };
    assert_eq!(class, "StackPanel");
    assert_eq!(classes, &vec![String::from("panel")]);
    let text = find(panel, &|node| matches!(&node.kind, InstanceKind::Control { class, .. } if class == "TextBlock")).expect("a text");
    let InstanceKind::Control { properties, .. } = &text.kind else { unreachable!() };
    assert!(properties.iter().any(|p| p.name == "Text" && p.value == "\"0\"" && p.source == "follows state"), "{properties:?}");
    let list = find(panel, &|node| matches!(node.kind, InstanceKind::Dynamic { .. })).expect("the list");
    assert_eq!(list.children.len(), 2);
    assert_eq!(list.children[1].key, Some(crate::Key::Int(2)));
    assert!(component.label().contains("runs 1"));

    items.set(vec![3]);
    let trees = mounted_trees();
    let list = trees
        .iter()
        .flat_map(|tree| tree.roots.iter())
        .find_map(|root| find(root, &|node| matches!(node.kind, InstanceKind::Dynamic { .. })))
        .expect("the list");
    assert_eq!(list.children.len(), 1, "the snapshot follows the list");

    let view = owner.with(|| crate::inspector().mount());
    let list = view.get_logical_descendants().find_map(|d| d.cast::<ferroui_controls::ListBox>()).expect("the list of the inspector");
    assert!(list.items().count() > 4, "the inspector lists the instances of the trees ({})", list.items().count());
    owner.dispose();
}
