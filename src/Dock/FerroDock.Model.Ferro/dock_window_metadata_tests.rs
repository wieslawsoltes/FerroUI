//! Native dock-window metadata forwards to the original ReactiveUI model.
use ferrodock_model::core::{DockWindowState, IDockWindow};
use ferrodock_model_reactiveui::core::DockWindow;
use ferroui_base::{
    data::core::ValueType,
    metadata::{from_markup_value, MarkupType},
    BoxedValue,
};
use std::rc::Rc;
#[test]
fn window_metadata_preserves_source_properties_methods_optional_handle_and_notifier() {
    crate::register_types();
    let window = DockWindow::new();
    let model: Rc<dyn IDockWindow> = window.clone();
    let optional: BoxedValue = Rc::new(Some(model.clone()));
    let receiver = ferroui_base::data::core::ValueTypes::normalize(optional.clone()).unwrap();
    assert!(Rc::ptr_eq(
        receiver.downcast_ref::<Rc<dyn IDockWindow>>().unwrap(),
        &model
    ));
    let value: Option<BoxedValue> = Some(receiver.clone());
    let metadata =
        MarkupType::find_by_handle(ValueType::of::<Option<Rc<dyn IDockWindow>>>().id()).unwrap();
    for name in [
        "Id",
        "X",
        "Y",
        "Width",
        "Height",
        "WindowState",
        "Topmost",
        "Title",
        "OwnerMode",
        "ParentWindow",
        "IsModal",
        "ShowInTaskbar",
        "Owner",
        "Factory",
        "Layout",
        "Host",
    ] {
        let property = metadata.find_property(name).unwrap();
        assert!(property.get.is_some());
        assert!(property.set.is_some());
    }
    for name in [
        "OnClose",
        "OnMoveDragBegin",
        "OnMoveDrag",
        "OnMoveDragEnd",
        "Save",
        "Present",
        "Exit",
        "SetActive",
    ] {
        assert!(metadata.methods.iter().any(|method| method.name == name));
    }
    let property = metadata.find_property("Topmost").unwrap();
    property.set.unwrap()(&[value.clone(), Some(Rc::new(true))]).unwrap();
    assert!(model.topmost());
    assert_eq!(
        from_markup_value::<bool>(&property.get.unwrap()(std::slice::from_ref(&value)).unwrap()),
        Some(true)
    );
    let property = metadata.find_property("WindowState").unwrap();
    property.set.unwrap()(&[value.clone(), Some(Rc::new(DockWindowState::Maximized))]).unwrap();
    assert_eq!(model.window_state(), DockWindowState::Maximized);
    assert!(metadata.notify_property_changed.unwrap()(&*receiver).is_some());
    assert!(metadata.notify_property_changed.unwrap()(&*optional).is_some());
}
