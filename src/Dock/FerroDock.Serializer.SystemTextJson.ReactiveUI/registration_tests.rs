// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::{
    core::{DockBase, DockableBaseImpl},
    impl_dock,
};
use ferrodock_serializer_system_text_json::JsonPropertyInfo;
use std::cell::{Cell, RefCell};

struct ApplicationDock {
    base: DockBase,
    note: RefCell<String>,
}
ferroui_reactiveui::reactive_object!(ApplicationDock, base);
impl DockableBaseImpl for ApplicationDock {}
impl_dock!(@base ApplicationDock, DockBase);
impl ApplicationDock {
    fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|runtime| Self {
            base: DockBase::construct(runtime.clone()),
            note: RefCell::new("default".into()),
        });
        model.base.initialize(&model);
        model
    }
}
fn application(name: &str) -> DockJsonModelRegistration {
    DockJsonModelRegistration::dock(
        name,
        ApplicationDock::new,
        vec![JsonPropertyInfo::new(
            "Note",
            |v: &Rc<ApplicationDock>| v.note.borrow().clone(),
            |v, note| {
                *v.note.borrow_mut() = note;
            },
        )],
    )
}

#[test]
fn explicit_application_properties_and_polymorphic_views_share_the_actual_model() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let model = DockJsonModelRegistration::dock(
        "Application.Dock",
        move || {
            count.set(count.get() + 1);
            ApplicationDock::new()
        },
        vec![JsonPropertyInfo::new(
            "Note",
            |v: &Rc<ApplicationDock>| v.note.borrow().clone(),
            |v, note| {
                *v.note.borrow_mut() = note;
            },
        )],
    );
    let serializer = create_serializer_with_models(vec![model]).unwrap();
    assert_eq!(calls.get(), 0);
    let loaded = serializer.deserialize::<Rc<dyn IDockable>>(Some(r#"{"$id":"1","$type":"Application.Dock","Note":"restored","Id":"custom","Owner":{"$ref":"1"}}"#)).unwrap().unwrap();
    assert_eq!(calls.get(), 1);
    let actual = loaded
        .clone()
        .boxed_reference()
        .downcast_ref::<Rc<ApplicationDock>>()
        .unwrap()
        .clone();
    assert_eq!(&*actual.note.borrow(), "restored");
    assert_eq!(actual.id(), "custom");
    assert!(Rc::ptr_eq(&actual.owner().unwrap(), &loaded));
    let wire = serializer.serialize(loaded).unwrap();
    assert!(
        wire.find("\"Note\"").unwrap()
            < wire
                .find("\"VisibleDockables\"")
                .unwrap_or_else(|| wire.find("\"IsActive\"").unwrap())
    );
    let dock = actual.clone() as Rc<dyn IDock>;
    let wire = serializer.serialize(dock).unwrap();
    assert!(wire.contains("\"$type\": \"Application.Dock\""));
    assert!(wire.contains("\"Note\": \"restored\""));
}

#[test]
fn invalid_application_registrations_fail_before_constructing_models() {
    for registrations in [
        vec![application("")],
        vec![application("  ")],
        vec![application("One"), application("Two")],
        vec![application("Dock.Model.ReactiveUI.Controls.RootDock")],
        vec![DockJsonModelRegistration::root(
            "Application.Root",
            ferrodock_model_reactiveui::controls::RootDock::new,
            Vec::new(),
        )],
        vec![
            application("Same"),
            DockJsonModelRegistration::document(
                "Same",
                ferrodock_model_reactiveui::controls::Document::new,
                Vec::new(),
            ),
        ],
    ] {
        assert!(model_type_metadata_with_models(registrations).is_err());
    }
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let registration = DockJsonModelRegistration::dock(
        "",
        move || {
            count.set(count.get() + 1);
            ApplicationDock::new()
        },
        Vec::new(),
    );
    assert!(create_serializer_with_models(vec![registration]).is_err());
    assert_eq!(calls.get(), 0);
}
