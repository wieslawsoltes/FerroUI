//! Standalone Core consumers invoke application host contracts with original values.
use crate::*;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::Factory;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupAssembly, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug, PartialEq)]
enum Call {
    Present(bool),
    Exit,
    Position(f64, f64),
    Size(f64, f64),
    Title(Option<String>),
    Active,
}
struct State;
impl IDockManagerState for State {}
impl IHostWindowState for State {}
#[derive(Default)]
struct Host {
    tracked: Cell<bool>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    state: RefCell<Option<Rc<dyn IHostWindowState>>>,
    window_state: Cell<DockWindowState>,
    layout: RefCell<Option<Rc<dyn IDock>>>,
    calls: RefCell<Vec<Call>>,
}
impl IHostWindow for Host {
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        self.state.borrow().clone()
    }
    fn is_tracked(&self) -> bool {
        self.tracked.get()
    }
    fn set_is_tracked(&self, value: bool) {
        self.tracked.set(value);
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    fn set_window(&self, value: Option<Rc<dyn IDockWindow>>) {
        *self.window.borrow_mut() = value;
    }
    fn present(&self, value: bool) {
        self.calls.borrow_mut().push(Call::Present(value));
    }
    fn exit(&self) {
        self.calls.borrow_mut().push(Call::Exit);
    }
    fn set_position(&self, x: f64, y: f64) {
        self.calls.borrow_mut().push(Call::Position(x, y));
    }
    fn get_position(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_size(&self, w: f64, h: f64) {
        self.calls.borrow_mut().push(Call::Size(w, h));
    }
    fn get_size(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_window_state(&self, value: DockWindowState) {
        self.window_state.set(value);
    }
    fn get_window_state(&self) -> DockWindowState {
        self.window_state.get()
    }
    fn set_title(&self, value: Option<String>) {
        self.calls.borrow_mut().push(Call::Title(value));
    }
    fn set_layout(&self, value: Rc<dyn IDock>) {
        *self.layout.borrow_mut() = Some(value);
    }
    fn set_active(&self) {
        self.calls.borrow_mut().push(Call::Active);
    }
}
fn get(metadata: &MarkupType, this: &BoxedValue, name: &str) -> Option<BoxedValue> {
    (metadata.find_property(name).unwrap().get.unwrap())(&[Some(this.clone())]).unwrap()
}
fn set(metadata: &MarkupType, this: &BoxedValue, name: &str, value: Option<BoxedValue>) {
    (metadata.find_property(name).unwrap().set.unwrap())(&[Some(this.clone()), value]).unwrap();
}
fn call(this: &BoxedValue, name: &str, args: Vec<Option<BoxedValue>>) -> Option<BoxedValue> {
    let mut values = vec![Some(this.clone())];
    values.extend(args);
    (<dyn IHostWindowMetadata>::MARKUP
        .find_methods(name)
        .next()
        .unwrap()
        .invoke)(&values)
    .unwrap()
}

#[test]
fn core_host_contract_metadata_has_original_owner_properties_and_supported_method_signatures() {
    register_types();
    for metadata in [
        <dyn IHostWindowMetadata>::MARKUP,
        <dyn IHostWindowStateMetadata>::MARKUP,
        <dyn IDockManagerStateMetadata>::MARKUP,
    ] {
        assert_eq!(
            MarkupAssembly::of_module(metadata.module_path)
                .unwrap()
                .name,
            "FerroDock.Model"
        );
        assert_eq!(metadata.namespace(), "FerroDock.Model.Core");
    }
    let host = <dyn IHostWindowMetadata>::MARKUP;
    assert_eq!(
        host.properties.iter().map(|p| p.name).collect::<Vec<_>>(),
        vec!["HostWindowState", "IsTracked", "Window"]
    );
    assert!(host.find_property("HostWindowState").unwrap().set.is_none());
    assert_eq!(
        host.methods.iter().map(|m| m.name).collect::<Vec<_>>(),
        vec![
            "Present",
            "Exit",
            "SetPosition",
            "SetSize",
            "SetWindowState",
            "GetWindowState",
            "SetTitle",
            "SetLayout",
            "SetActive"
        ]
    );
    assert!(host.find_methods("GetPosition").next().is_none());
    assert!(host.find_methods("GetSize").next().is_none());
    let state: Rc<dyn IHostWindowState> = Rc::new(State);
    let source: BoxedValue = Rc::new(state.clone());
    let target =
        ValueTypes::try_cast(&source, ValueType::of::<Rc<dyn IDockManagerState>>()).unwrap();
    assert!(ValueTypes::reference_equals(&source, &target));
    assert_eq!(
        <dyn IHostWindowStateMetadata>::MARKUP
            .base_type()
            .unwrap()
            .name,
        "IDockManagerState"
    );
}

#[test]
fn core_host_properties_keep_original_nullable_state_window_and_host_identity() {
    register_types();
    let host = Rc::new(Host::default());
    let contract: Rc<dyn IHostWindow> = host.clone();
    let boxed: BoxedValue = Rc::new(contract.clone());
    let metadata = <dyn IHostWindowMetadata>::MARKUP;
    assert!(get(metadata, &boxed, "HostWindowState").is_none());
    assert!(get(metadata, &boxed, "Window").is_none());
    set(metadata, &boxed, "IsTracked", Some(Rc::new(true)));
    assert!(host.tracked.get());
    set(metadata, &boxed, "IsTracked", Some(Rc::new(false)));
    assert!(!host.tracked.get());
    let state: Rc<dyn IHostWindowState> = Rc::new(State);
    *host.state.borrow_mut() = Some(state.clone());
    let actual =
        from_markup_value::<Rc<dyn IHostWindowState>>(&get(metadata, &boxed, "HostWindowState"))
            .unwrap();
    assert!(Rc::ptr_eq(&actual, &state));
    let window: Rc<dyn IDockWindow> = Factory::new().create_dock_window();
    set(metadata, &boxed, "Window", Some(Rc::new(window.clone())));
    assert!(Rc::ptr_eq(&host.window().unwrap(), &window));
    let actual =
        from_markup_value::<Rc<dyn IDockWindow>>(&get(metadata, &boxed, "Window")).unwrap();
    assert!(Rc::ptr_eq(&actual, &window));
    set(metadata, &boxed, "Window", None);
    assert!(host.window().is_none());
    assert!(ValueTypes::reference_equals(
        &boxed,
        &(Rc::new(contract) as BoxedValue)
    ));
}

#[test]
fn core_host_methods_dispatch_source_arguments_order_and_results_without_a_window_engine() {
    register_types();
    let host = Rc::new(Host::default());
    let contract: Rc<dyn IHostWindow> = host.clone();
    let boxed: BoxedValue = Rc::new(contract);
    call(&boxed, "Present", vec![Some(Rc::new(false))]);
    call(&boxed, "Present", vec![Some(Rc::new(true))]);
    call(
        &boxed,
        "SetPosition",
        vec![Some(Rc::new(-12.5)), Some(Rc::new(20.0))],
    );
    call(
        &boxed,
        "SetSize",
        vec![Some(Rc::new(640.0)), Some(Rc::new(480.0))],
    );
    for title in [None, Some(String::new()), Some("Source title".to_owned())] {
        call(
            &boxed,
            "SetTitle",
            vec![title.map(|t| Rc::new(t) as BoxedValue)],
        );
    }
    call(&boxed, "SetActive", vec![]);
    call(&boxed, "Exit", vec![]);
    assert_eq!(
        *host.calls.borrow(),
        vec![
            Call::Present(false),
            Call::Present(true),
            Call::Position(-12.5, 20.0),
            Call::Size(640.0, 480.0),
            Call::Title(None),
            Call::Title(Some(String::new())),
            Call::Title(Some("Source title".into())),
            Call::Active,
            Call::Exit
        ]
    );
    for expected in [
        DockWindowState::Normal,
        DockWindowState::Minimized,
        DockWindowState::Maximized,
        DockWindowState::FullScreen,
    ] {
        call(&boxed, "SetWindowState", vec![Some(Rc::new(expected))]);
        let result = call(&boxed, "GetWindowState", vec![]);
        assert_eq!(
            from_markup_value::<DockWindowState>(&result),
            Some(expected)
        );
    }
    let layout: Rc<dyn IDock> = Factory::new().create_root_dock();
    call(&boxed, "SetLayout", vec![Some(Rc::new(layout.clone()))]);
    assert!(Rc::ptr_eq(host.layout.borrow().as_ref().unwrap(), &layout));
}
