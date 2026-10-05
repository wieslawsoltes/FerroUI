use crate::presenters::ItemsPresenter;
use crate::primitives::{TabStrip, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::test_scope;
use crate::{Control, TabItem};
use ferroui_base::{BoxedValue, Ref};
use std::rc::Rc;

fn create_tab_strip_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::for_type::<TabStrip>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("itemsPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    }))
}

fn tab_item(name: &str) -> Option<BoxedValue> {
    let item = TabItem::new();
    item.set_name(Some(name.to_string()));
    Some(Control::boxed(item))
}

fn create_target(names: &[&str]) -> Ref<TabStrip> {
    let target = TabStrip::new();
    target.set_value(TemplatedControl::template_property(), create_tab_strip_template());
    for name in names {
        target.items().add(tab_item(name));
    }
    target
}

fn control(item: Option<BoxedValue>) -> Option<Ref<Control>> {
    item.as_ref().and_then(Control::from_boxed)
}

#[test]
fn first_tab_should_be_selected_by_default() {
    let _scope = test_scope();
    let target = create_target(&["first", "second"]);

    target.apply_template();

    assert_eq!(0, target.selected_index());
    assert!(control(target.items().get_at(0)).is_some());
    assert_eq!(control(target.items().get_at(0)), control(target.selected_item()));
}

#[test]
fn setting_selected_item_should_set_selection() {
    let _scope = test_scope();
    let target = create_target(&["first", "second"]);

    target.set_selected_item(target.items().get_at(1));
    target.apply_template();

    assert_eq!(1, target.selected_index());
    assert!(control(target.items().get_at(1)).is_some());
    assert_eq!(control(target.items().get_at(1)), control(target.selected_item()));
}

#[test]
fn removing_selected_should_select_first() {
    let _scope = test_scope();
    let target = create_target(&["first", "second", "3rd"]);

    target.apply_template();
    target.set_selected_item(target.items().get_at(1));
    assert!(control(target.items().get_at(1)).is_some());
    assert_eq!(control(target.items().get_at(1)), control(target.selected_item()));
    target.items().remove_at(1);

    assert_eq!(0, target.selected_index());
    assert_eq!(control(target.items().get_at(0)), control(target.selected_item()));
    assert_eq!(
        Some("first".to_string()),
        control(target.selected_item()).unwrap().cast::<TabItem>().unwrap().name()
    );
}
