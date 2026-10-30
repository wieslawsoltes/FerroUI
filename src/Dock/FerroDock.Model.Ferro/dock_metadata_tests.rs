//! Core-only consumers retain original lists, navigation and interface identity.
use crate::{DockableListMetadata, IDockMetadata, IDockableMetadata};
use ferrodock_model::{collections::ListHandle, core::*};
use ferrodock_model_reactiveui::{controls::Document, Factory};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupAssembly, MarkupType, MarkupTyped},
    BoxedValue,
};
use ferroui_reactiveui::ICommand;
use std::{any::TypeId, cell::RefCell, rc::Rc};

fn get(metadata: &MarkupType, value: &BoxedValue, name: &str) -> Option<BoxedValue> {
    (metadata.find_property(name).unwrap().get.unwrap())(&[Some(value.clone())]).unwrap()
}

#[test]
fn core_contract_ownership_and_source_downcasts_work_without_controls() {
    crate::register_types();
    for metadata in [
        <dyn IDockMetadata>::MARKUP,
        <dyn IDockableMetadata>::MARKUP,
        <dyn DockableListMetadata>::MARKUP,
    ] {
        assert_eq!(
            MarkupAssembly::of_module(metadata.module_path)
                .unwrap()
                .name,
            "FerroDock.Model"
        );
    }
    type Provider = Rc<dyn ferrodock_controls_recycling_model_ferro::IControlRecyclingIdProvider>;
    assert_eq!(
        (<dyn IDockableMetadata>::MARKUP.interfaces[0])(),
        ValueType::of::<Provider>()
    );
    let recycling = MarkupType::find_by_handle(TypeId::of::<Provider>()).unwrap();
    assert_eq!(recycling.name, "IControlRecyclingIdProvider");
    assert_eq!(
        MarkupAssembly::of_module(recycling.module_path)
            .unwrap()
            .name,
        "FerroDock.Controls.Recycling.Model"
    );
    let factory = Factory::new();
    let dock: Rc<dyn IDock> = factory.create_document_dock();
    let item: Rc<dyn IDockable> = dock.clone();
    let value: BoxedValue = Rc::new(item);
    let cast = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDock>>()).unwrap();
    assert!(Rc::ptr_eq(
        cast.downcast_ref::<Rc<dyn IDock>>().unwrap(),
        &dock
    ));
    let document: Rc<dyn IDockable> = Document::new();
    let document: BoxedValue = Rc::new(document);
    assert!(ValueTypes::try_cast(&document, ValueType::of::<Rc<dyn IDock>>()).is_none());
}

#[test]
fn core_list_metadata_mutates_the_original_observable_list_and_forwards_notifications() {
    crate::register_types();
    let factory = Factory::new();
    let dock: Rc<dyn IDock> = factory.create_document_dock();
    let list = factory.create_list::<Option<Rc<dyn IDockable>>>(Vec::new());
    let dock_value: BoxedValue = Rc::new(dock.clone());
    let property = <dyn IDockMetadata>::MARKUP
        .find_property("VisibleDockables")
        .unwrap();
    (property.set.unwrap())(&[Some(dock_value.clone()), Some(Rc::new(list.clone()))]).unwrap();
    let exposed = get(<dyn IDockMetadata>::MARKUP, &dock_value, "VisibleDockables");
    let handle = from_markup_value::<ListHandle<Option<Rc<dyn IDockable>>>>(&exposed).unwrap();
    assert!(list.ptr_eq(&handle));
    let metadata =
        MarkupType::find_by_handle(TypeId::of::<ListHandle<Option<Rc<dyn IDockable>>>>()).unwrap();
    let boxed: BoxedValue = Rc::new(handle.clone());
    let separately_boxed: BoxedValue = Rc::new(list.clone());
    let different_list: BoxedValue =
        Rc::new(factory.create_list::<Option<Rc<dyn IDockable>>>(Vec::new()));
    assert!(ValueTypes::reference_equals(&boxed, &separately_boxed));
    assert!(!ValueTypes::reference_equals(&boxed, &different_list));
    let notifier = (metadata.notify_property_changed.unwrap())(boxed.as_ref()).unwrap();
    assert!(std::ptr::eq(
        notifier,
        list.as_notify_property_changed().unwrap()
    ));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let subscription = notifier.property_changed().add(Rc::new(move |name| {
        observed.borrow_mut().push(name.to_owned());
    }));
    let item: Rc<dyn IDockable> = Document::new();
    let add = metadata.find_methods("Add").next().unwrap();
    (add.invoke)(&[Some(boxed.clone()), Some(Rc::new(item.clone()))]).unwrap();
    assert_eq!(list.count(), 1);
    assert!(Rc::ptr_eq(
        (dock.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &item
    ));
    assert_eq!(
        get(metadata, &boxed, "Count")
            .unwrap()
            .downcast_ref::<i32>(),
        Some(&1)
    );
    list.add(Some(Document::new()));
    assert_eq!(handle.count(), 2);
    let remove = metadata.find_methods("Remove").next().unwrap();
    let result = (remove.invoke)(&[Some(boxed.clone()), Some(Rc::new(item))])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<bool>(), Some(&true));
    assert_eq!(list.count(), 1);
    let clear = metadata.find_methods("Clear").next().unwrap();
    (clear.invoke)(&[Some(boxed.clone())]).unwrap();
    assert!(list.is_empty());
    assert_eq!(
        changes
            .borrow()
            .iter()
            .filter(|name| name.as_str() == "Count")
            .count(),
        4
    );
    notifier.property_changed().remove(subscription);
}

#[test]
fn core_command_getters_keep_source_command_identity_and_execute_original_navigation() {
    crate::register_types();
    let factory = Factory::new();
    let dock: Rc<dyn IDock> = factory.create_document_dock();
    let first: Rc<dyn IDockable> = Document::new();
    let second: Rc<dyn IDockable> = Document::new();
    dock.set_visible_dockables(Some(
        factory.create_list(vec![Some(first.clone()), Some(second.clone())]),
    ));
    dock.set_active_dockable(Some(first.clone()));
    let boxed: BoxedValue = Rc::new(dock.clone());
    let mut commands = Vec::new();
    for (name, expected) in [
        ("GoBack", dock.go_back()),
        ("GoForward", dock.go_forward()),
        ("Navigate", dock.navigate()),
        ("Close", dock.close()),
    ] {
        let command =
            from_markup_value::<Rc<dyn ICommand>>(&get(<dyn IDockMetadata>::MARKUP, &boxed, name))
                .unwrap();
        assert!(Rc::ptr_eq(&command, &expected));
        commands.push(command);
    }
    commands[2].execute(Some(&second));
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &second));
    assert!(dock.can_go_back());
    commands[0].execute(None);
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &first));
    assert!(dock.can_go_forward());
    commands[1].execute(None);
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &second));
}
