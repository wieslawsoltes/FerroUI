//! Original state corpus plus actual native menu, ownership and command regressions.
use super::*;
use crate::{compat::*, controls::*};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{collections::RibbonReadOnlyList, commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    FerroObjectImpl, Ref, data::core::ValueTypes, ferro_class, ferro_class_info, instantiate,
};
use ferroui_controls::{
    Border, Control, Flyout, MenuFlyout, StackPanel, TextBlock, primitives::FlyoutBase,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn check(name: &str, mut actual: Value) {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/menu-factory.json")).unwrap();
    actual["name"] = json!(name);
    assert_eq!(
        actual,
        *fixture["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap()
    );
}
struct Token(usize);
#[repr(C)]
struct Factory {
    base: RibbonElementFactory,
    created: Cell<usize>,
    disposed: Rc<RefCell<Vec<usize>>>,
}
ferro_class!(Factory:RibbonElementFactory);
ferro_class_info!(Factory {});
impl FerroObjectImpl for Factory {}
impl RibbonElementFactoryImpl for Factory {
    fn create_flyout(
        this: &Self,
        _entries: Option<RibbonReadOnlyList<RibbonNodeReference>>,
        _content: Option<RibbonObject>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<FlyoutBase>> {
        let token = this.created.get() + 1;
        this.created.set(token);
        let d = this.disposed.clone();
        bindings
            .unwrap()
            .add(Some(Rc::new(move || d.borrow_mut().push(token))));
        let flyout = MenuFlyout::new();
        flyout.set_instance_data(Some(Rc::new(Token(token))));
        Some(flyout.upcast())
    }
}
fn factory() -> Ref<Factory> {
    instantiate(Factory {
        base: RibbonElementFactory::construct(),
        created: Cell::new(0),
        disposed: Rc::new(RefCell::new(vec![])),
    })
}
fn token(flyout: Option<Ref<FlyoutBase>>) -> usize {
    flyout.unwrap().instance_data::<Token>().unwrap().0
}
#[test]
fn original_rebuild_signature_disposes_children_and_suspension_refreshes_current_structure() {
    crate::group_tests::run(|| {
        let f = factory();
        let b = RibbonModelBindings::new();
        let m = RibbonDropDownButtonModel::new(Some("d".into()), Some("Drop".into()));
        let item = RibbonMenuItemModel::new(Some("a".into()), Some("A".into()));
        m.menu_items().add(Some(item.clone()));
        let e = f
            .create_drop_down_button(m.clone(), b.clone())
            .cast::<RibbonDropDownButton>()
            .unwrap();
        let mut counts = vec![f.created.get()];
        item.set_label(Some("Changed".into()));
        item.set_is_checked(true);
        counts.push(f.created.get());
        item.set_is_checkable(true);
        counts.push(f.created.get());
        item.set_group_name(Some("".into()));
        counts.push(f.created.get());
        let child = RibbonMenuItemModel::new(Some("c".into()), Some("Child".into()));
        item.items().add(Some(child.clone()));
        counts.push(f.created.get());
        child.set_is_checkable(true);
        counts.push(f.created.get());
        item.items().remove_at(0);
        counts.push(f.created.get());
        child.set_group_name(Some("removed".into()));
        counts.push(f.created.get());
        b.suspend();
        item.set_group_name(Some("suspended".into()));
        counts.push(f.created.get());
        b.resume();
        counts.push(f.created.get());
        let before = token(e.flyout());
        b.dispose();
        item.set_is_checkable(false);
        counts.push(f.created.get());
        check(
            "rebuild",
            json!({"counts":counts,"disposed":f.disposed.borrow().clone(),"token":before,"after":token(e.flyout())}),
        );
    });
}
#[test]
fn original_split_two_way_defaults_null_projection_and_disposal() {
    crate::group_tests::run(|| {
        let f = factory();
        let b = RibbonModelBindings::new();
        let m = RibbonSplitButtonModel::new(Some("s".into()), Some("Split".into()));
        m.set_is_checkable(true);
        m.set_follow_last_choice(true);
        m.set_is_checked(true);
        let e = f
            .create_split_button(m.clone(), b.clone())
            .cast::<RibbonSplitButton>()
            .unwrap();
        let initial = [
            e.is_checkable(),
            e.follow_last_choice(),
            e.is_checked() == Some(true),
        ];
        e.set_is_checked(None);
        let nil = m.is_checked();
        e.set_is_checked(Some(true));
        let yes = m.is_checked();
        m.set_follow_last_choice(false);
        m.set_is_checkable(false);
        let final_state = [
            e.is_checkable(),
            e.follow_last_choice(),
            e.is_checked() == Some(true),
        ];
        b.dispose();
        m.set_is_checked(false);
        check(
            "split",
            json!({"initial":initial,"nil":nil,"yes":yes,"final":final_state,"disposedElement":e.is_checked()}),
        );
    });
}
#[test]
fn original_reentrant_assignment_uses_live_child_scope() {
    crate::group_tests::run(|| {
        let f = factory();
        let b = RibbonModelBindings::new();
        let m = RibbonDropDownButtonModel::new(Some("r".into()), Some("Reentrant".into()));
        let assigned = Rc::new(RefCell::new(vec![]));
        let a = assigned.clone();
        let first = Cell::new(true);
        f.bind_flyout(
            m.clone(),
            b.clone(),
            Rc::new(move |flyout| {
                a.borrow_mut().push(token(flyout));
                if first.replace(false) {
                    m.menu_items().add(Some(RibbonMenuItemModel::new(
                        Some("nested".into()),
                        Some("Nested".into()),
                    )));
                }
            }),
        );
        let created = f.created.get();
        let before = f.disposed.borrow().clone();
        b.dispose();
        check(
            "reentrant",
            json!({"assigned":assigned.borrow().clone(),"created":created,"disposedBefore":before,"disposed":f.disposed.borrow().clone()}),
        );
    });
}
fn menu_at(flyout: &FlyoutBase, index: usize) -> Ref<MenuFlyoutItem> {
    Control::from_boxed(
        &flyout
            .downcast_ref::<MenuFlyout>()
            .unwrap()
            .items()
            .get_at(index)
            .unwrap(),
    )
    .unwrap()
    .cast()
    .unwrap()
}
#[test]
fn actual_simple_menu_updates_in_place_recreates_kinds_and_releases_old_bindings() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonDropDownButtonModel::new(None, None);
        let item = RibbonMenuItemModel::new(Some("entry".into()), Some("Old".into()));
        m.menu_items().add(Some(item.clone()));
        let e = f
            .create_item(Some(m.clone()), b.clone())
            .unwrap()
            .cast::<RibbonDropDownButton>()
            .unwrap();
        let old_flyout = e.flyout().unwrap();
        let old = menu_at(&old_flyout, 0);
        item.set_label(Some("Updated".into()));
        assert_eq!(old.text().as_deref(), Some("Updated"));
        assert!(old_flyout.ptr_eq(&e.flyout().unwrap()));
        item.set_is_checkable(true);
        let new_flyout = e.flyout().unwrap();
        assert!(!old_flyout.ptr_eq(&new_flyout));
        let current = menu_at(&new_flyout, 0);
        assert!(current.is::<ToggleMenuFlyoutItem>());
        item.set_label(Some("Current".into()));
        assert_eq!(current.text().as_deref(), Some("Current"));
        assert_eq!(old.text().as_deref(), Some("Updated"));
        item.set_group_name(Some("".into()));
        assert!(menu_at(&e.flyout().unwrap(), 0).is::<RadioMenuFlyoutItem>());
        m.menu_items().clear();
        assert!(e.flyout().is_none());
        b.dispose();
        let weak = Rc::downgrade(&m);
        drop(m);
        drop(e);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn actual_menu_invocation_click_and_alternate_have_single_command_and_source_order() {
    crate::group_tests::run(|| {
        struct MenuArgument(i32);

        let f = RibbonElementFactory::new();
        f.set_ribbon(Some(Ribbon::new()));
        let b = RibbonModelBindings::new();
        let m = RibbonMenuItemModel::new(Some("check".into()), Some("Check".into()));
        m.set_is_checkable(true);
        let calls = Rc::new(RefCell::new(vec![]));
        let c = calls.clone();
        let weak = Rc::downgrade(&m);
        let argument = Rc::new(MenuArgument(42));
        let expected = argument.clone();
        m.set_command(Some(RibbonRelayCommand::new_with_parameter(
            move |p| {
                let value = p.unwrap().downcast_ref::<MenuArgument>().unwrap();
                assert!(std::ptr::eq(value, expected.as_ref()));
                c.borrow_mut()
                    .push((weak.upgrade().unwrap().is_checked(), value.0));
            },
            None,
        )));
        m.set_command_parameter(Some(RibbonObject::reference(argument)));
        let e = f.create_menu_item(Some(m.clone()), Some(b.clone()));
        e.invoke_menu_core();
        assert_eq!(&*calls.borrow(), &[(true, 42)]);
        assert!(m.is_checked());
        RibbonMenu::get_invoke(&e).unwrap().try_invoke(&[]).unwrap();
        assert_eq!(&*calls.borrow(), &[(true, 42), (true, 42)]);
        assert!(!m.is_checked());
        m.set_is_enabled(false);
        assert!(!e.is_enabled());
        b.dispose();
    });
}
#[test]
fn rich_flyout_reparents_same_content_and_detaches_on_rebuild_and_disposal() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonDropDownButtonModel::new(None, None);
        let content = TextBlock::new();
        content.set_text(Some("Identity"));
        let old_parent = Border::new();
        old_parent.set_child(content.clone());
        m.set_drop_down_content(Some(
            crate::metadata::model_objects::from_native(Some(Control::boxed(content.clone())))
                .unwrap(),
        ));
        let e = f
            .create_drop_down_button(m.clone(), b.clone())
            .cast::<RibbonDropDownButton>()
            .unwrap();
        assert!(old_parent.child().is_none());
        let first = e.flyout().unwrap();
        let panel =
            ValueTypes::as_object(&*first.downcast_ref::<Flyout>().unwrap().content().unwrap())
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
        assert!(panel.children().get(0).ptr_eq(&content));
        m.menu_items()
            .add(Some(RibbonMenuHeaderModel::new(Some("Header".into()))));
        assert_eq!(panel.children().count(), 0);
        let second = e.flyout().unwrap();
        let new_panel =
            ValueTypes::as_object(&*second.downcast_ref::<Flyout>().unwrap().content().unwrap())
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
        assert!(new_panel.children().get(0).ptr_eq(&content));
        assert_eq!(new_panel.children().count(), 2);
        b.dispose();
        assert_eq!(new_panel.children().count(), 1);
        assert!(content.get_logical_parent().is_none());
    });
}
#[test]
fn menu_items_defer_validation_and_creation_and_reject_live_collection_version_changes() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let items = ferroui_system::collections::object_model::ObservableCollection::new_managed();
        let m = RibbonMenuItemModel::new(None, Some("First".into()));
        items.add(Some(m.clone() as Rc<dyn RibbonNodeModelImpl>));
        let query = f.create_menu_items(
            Some(RibbonItemsSource::from_collection(items.clone())),
            Some(b.clone()),
        );
        let mut cursor = query.iter();
        m.set_label(Some("Before next".into()));
        assert_eq!(
            cursor
                .next()
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Before next")
        );
        items.add(None);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next())).is_err());
        // A panic ends this native Rust iteration operation.
        drop(cursor);
        assert_eq!(
            query.iter().count(),
            1,
            "fresh enumeration sees the current live source"
        );
        let invalid = f.create_menu_items(None, None);
        let mut cursor = invalid.iter();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next())).is_err());
        // A panic ends this native Rust iteration operation.
        drop(cursor);
        b.dispose();
    });
}
#[test]
fn public_flyout_accepts_custom_read_only_sequences_and_empty_null_contract() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let empty = RibbonFixedItems::new(vec![]) as RibbonReadOnlyList<_>;
        assert!(
            f.create_flyout(Some(empty), None, Some(b.clone()))
                .is_none()
        );
        let nodes = RibbonFixedItems::new(vec![
            Some(RibbonMenuHeaderModel::new(Some("Title".into())) as Rc<dyn RibbonNodeModelImpl>),
            Some(RibbonMenuSeparatorModel::new()),
            None,
        ]) as RibbonReadOnlyList<_>;
        let flyout = f.create_flyout(Some(nodes), None, Some(b.clone())).unwrap();
        assert!(flyout.is::<Flyout>());
        b.dispose();
    });
}
#[test]
fn source_signature_dispatches_new_content_equals_old_without_identity_shortcut() {
    crate::group_tests::run(|| {
        let f = factory();
        let b = RibbonModelBindings::new();
        let m = RibbonDropDownButtonModel::new(Some("eq".into()), Some("Equality".into()));
        struct Payload(&'static str);
        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let value = |id| {
            let log = log.clone();
            RibbonObject::with_equality(Rc::new(Payload(id)), move |old, other| {
                log.borrow_mut().push(format!(
                    "{}>{}",
                    old.0,
                    other
                        .downcast_ref::<Payload>()
                        .map(|p| p.0)
                        .unwrap_or("null")
                ));
                false
            })
        };
        m.set_drop_down_content(Some(value("old")));
        f.create_drop_down_button(m.clone(), b.clone());
        m.set_drop_down_content(Some(value("new")));
        check(
            "content_equality",
            json!({"log":log.borrow().clone(),"created":f.created.get()}),
        );
        b.dispose();
    });
}
#[repr(C)]
struct MenuFactoryOverride {
    base: RibbonElementFactory,
    created: Cell<usize>,
}
ferro_class!(MenuFactoryOverride:RibbonElementFactory);
ferro_class_info!(MenuFactoryOverride {});
impl FerroObjectImpl for MenuFactoryOverride {}
impl RibbonElementFactoryImpl for MenuFactoryOverride {
    fn create_menu_item(
        this: &Self,
        item: Option<Rc<dyn RibbonMenuItemModelImpl>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<MenuFlyoutItem> {
        this.created.set(this.created.get() + 1);
        let element = Self::parent_create_menu_item(this, item, bindings);
        element.set_name(Some("downstream".into()));
        element
    }
}
#[test]
fn lazy_nested_menu_creation_dispatches_the_native_virtual_override() {
    crate::group_tests::run(|| {
        let f = instantiate(MenuFactoryOverride {
            base: RibbonElementFactory::construct(),
            created: Cell::new(0),
        });
        let b = RibbonModelBindings::new();
        let root = RibbonMenuItemModel::new(None, Some("Parent".into()));
        root.items()
            .add(Some(RibbonMenuItemModel::new(None, Some("Child".into()))));
        let nodes = RibbonItemsSource::from_values(vec![Some(root as Rc<dyn RibbonNodeModelImpl>)]);
        let query = f.create_menu_items(Some(nodes), Some(b.clone()));
        assert_eq!(f.created.get(), 0);
        let parent = query
            .iter()
            .next()
            .unwrap()
            .cast::<MenuFlyoutSubItem>()
            .unwrap();
        assert_eq!(f.created.get(), 1);
        assert_eq!(
            parent.items().get(0).unwrap().name().as_deref(),
            Some("downstream")
        );
        b.dispose();
    });
}
