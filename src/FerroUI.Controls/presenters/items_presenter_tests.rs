use super::{ContentPresenter, ItemsPresenter};
use crate::primitives::TemplatedControl;
use crate::templates::FuncControlTemplate;
use crate::test_support::{boxed_str, test_scope, TestRoot, TestScope};
use crate::{items_equal, ItemsControl, ItemsSource, StackPanel};
use ferroui_base::collections::FerroList;
use ferroui_base::{FerroObject, Ref};
use std::rc::Rc;

struct Target {
    _scope: TestScope,
    target: Ref<ItemsPresenter>,
    items_control: Ref<ItemsControl>,
    root: Ref<TestRoot>,
}

fn create_target(items: Option<ItemsSource>) -> Target {
    let scope = test_scope();
    let result = ItemsPresenter::new();

    let items_control = ItemsControl::new();
    items_control.set_items_source(items);
    let presenter = result.clone();
    TemplatedControl::set_template(
        &items_control,
        Some(FuncControlTemplate::for_type::<ItemsControl>(move |_, _| presenter.clone().upcast())),
    );

    let root = TestRoot::with_child(items_control.clone());
    root.execute_initial_layout_pass();
    Target { _scope: scope, target: result, items_control, root }
}

fn list(values: &[&str]) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(values.iter().map(|v| v.to_string())))
}

fn stack_panel(target: &ItemsPresenter) -> Ref<StackPanel> {
    let panel = target.panel().unwrap();
    assert_eq!(panel.get_type(), <StackPanel as ferroui_base::StaticType>::TYPE);
    panel.cast::<StackPanel>().unwrap()
}

fn assert_containers(panel: &StackPanel, items: &[String]) {
    assert_eq!(items.len(), panel.children().count());

    for (i, item) in items.iter().enumerate() {
        let container = panel.children().get(i).cast::<ContentPresenter>().unwrap();
        assert!(items_equal(&boxed_str(item), &container.data_context()));
        assert!(items_equal(&boxed_str(item), &container.content()));
    }
}

#[test]
fn should_register_with_host_when_templated_parent_set() {
    let host = ItemsControl::new();
    let target = ItemsPresenter::new();

    assert!(host.presenter().is_none());

    target.set_templated_parent(host.clone().upcast::<FerroObject>());

    assert_eq!(Some(target), host.presenter());
}

#[test]
fn panel_should_be_visual_child() {
    let t = create_target(None);
    let children = t.target.visual_children().to_vec();

    assert_eq!(children.len(), 1);
    assert_eq!(t.target.panel().map(Ref::upcast), Some(children[0].clone()));
}

#[test]
fn creates_containers_for_initial_items() {
    let items = ["foo", "bar", "baz"];
    let t = create_target(Some(ItemsSource::from_strs(items)));
    let panel = stack_panel(&t.target);

    assert_containers(&panel, &items.map(str::to_string));
}

#[test]
fn creates_containers_for_inserted_items() {
    let items = list(&["foo", "bar", "baz"]);
    let t = create_target(Some(items.clone().into()));
    let panel = stack_panel(&t.target);

    items.insert(1, "foo2".to_string());
    t.root.layout_manager().execute_layout_pass();

    assert_containers(&panel, &items.to_vec());
}

#[test]
fn removes_containers_for_removed_items() {
    let items = list(&["foo", "bar", "baz"]);
    let t = create_target(Some(items.clone().into()));
    let panel = stack_panel(&t.target);

    items.remove_at(1);
    t.root.layout_manager().execute_layout_pass();

    assert_containers(&panel, &items.to_vec());
}

#[test]
fn updates_containers_for_moved_items() {
    let items = list(&["foo", "bar", "baz"]);
    let t = create_target(Some(items.clone().into()));
    let panel = stack_panel(&t.target);

    items.move_item(0, 2);
    t.root.layout_manager().execute_layout_pass();

    assert_containers(&panel, &items.to_vec());
}

#[test]
fn updates_container_for_moved_range_of_items() {
    let items = list(&["foo", "bar", "baz"]);
    let t = create_target(Some(items.clone().into()));
    let panel = stack_panel(&t.target);

    items.move_range(0, 2, 2);
    t.root.layout_manager().execute_layout_pass();

    assert_containers(&panel, &items.to_vec());
}

#[test]
fn updates_containers_for_replaced_items() {
    let items = list(&["foo", "bar", "baz"]);
    let t = create_target(Some(items.clone().into()));
    let panel = stack_panel(&t.target);

    items.set(1, "bar2".to_string());
    t.root.layout_manager().execute_layout_pass();

    assert_containers(&panel, &items.to_vec());
}

#[test]
fn updates_containers_on_items_changed() {
    let items = list(&["foo", "bar", "baz"]);
    let t = create_target(Some(items.into()));
    let panel = stack_panel(&t.target);

    let new_items = ["qux", "quux", "corge"];
    t.items_control.set_items_source(Some(ItemsSource::from_strs(new_items)));
    t.root.layout_manager().execute_layout_pass();

    assert_containers(&panel, &new_items.map(str::to_string));
}
