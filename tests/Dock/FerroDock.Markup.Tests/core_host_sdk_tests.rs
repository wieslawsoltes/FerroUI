//! Original native hosts and application container contracts through a separate consumer.
use crate::markup_extension_tests::run;
use ferrodock_ferro::controls::HostWindow;
use ferrodock_model::core::*;
use ferrodock_model_ferro::*;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{from_markup_value, MarkupType, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{Button, CheckBox, ContentControl, Control, StackPanel, TextBlock, TextBox};
use std::{cell::RefCell, rc::Rc};

fn load(name: &str) -> Ref<StackPanel> {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
    .downcast_ref::<Ref<StackPanel>>()
    .unwrap()
    .clone()
}
fn call(host: &BoxedValue, name: &str, args: Vec<Option<BoxedValue>>) -> Option<BoxedValue> {
    let mut values = vec![Some(host.clone())];
    values.extend(args);
    (<dyn IHostWindowMetadata>::MARKUP
        .find_methods(name)
        .next()
        .unwrap()
        .invoke)(&values)
    .unwrap()
}
fn get(metadata: &MarkupType, this: &BoxedValue, name: &str) -> Option<BoxedValue> {
    (metadata.find_property(name).unwrap().get.unwrap())(&[Some(this.clone())]).unwrap()
}

fn assert_host_bindings(panel: &Ref<StackPanel>) {
    let native = HostWindow::new();
    let host = HostWindow::as_host_window(native.clone());
    let window: Rc<dyn IDockWindow> =
        ferrodock_model_reactiveui::Factory::new().create_dock_window();
    window.set_title("Source window".into());
    host.set_window(Some(window.clone()));
    let boxed: BoxedValue = Rc::new(host.clone());
    panel.set_data_context(Some(boxed.clone()));
    Dispatcher::ui_thread().run_jobs(None);
    let text = panel.children().get(1).cast::<TextBlock>().unwrap();
    assert_eq!(text.text().as_deref(), Some("Source window"));
    window.set_title("Updated source".into());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(text.text().as_deref(), Some("Updated source"));
    panel
        .children()
        .get(0)
        .cast::<CheckBox>()
        .unwrap()
        .set_is_checked(Some(true));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(host.is_tracked());
    let content = panel
        .children()
        .get(2)
        .cast::<ContentControl>()
        .unwrap()
        .content();
    let state = from_markup_value::<Rc<dyn IHostWindowState>>(&content).unwrap();
    assert!(Rc::ptr_eq(&state, &host.host_window_state().unwrap()));
    let button = panel.children().get(3).cast::<Button>().unwrap();
    button
        .command()
        .unwrap()
        .execute(button.command_parameter().as_ref());
    assert_eq!(native.title().as_deref(), Some("Core command title"));
    assert!(ValueTypes::reference_equals(
        &boxed,
        &(Rc::new(native.clone()) as BoxedValue)
    ));
    panel.set_data_context(None);
    host.set_window(None);
    native.close();
}

#[test]
fn core_host_compiled_bindings_keep_original_model_state_and_execute_original_native_methods() {
    run(|| assert_host_bindings(&load("CoreHostBindings")));
}

#[test]
fn shared_weak_source_restores_declared_host_and_nullable_interfaces() {
    run(|| {
        use ferroui_base::{data::core::WeakValue, FerroObject};
        use std::any::TypeId;
        let native = HostWindow::new();
        let host = HostWindow::as_host_window(native.clone());
        let boxed: BoxedValue = Rc::new(host.clone());
        let weak = WeakValue::new(&boxed);
        assert!(matches!(weak, WeakValue::Object(_, _)));
        let restored = from_markup_value::<Rc<dyn IHostWindow>>(&weak.upgrade()).unwrap();
        assert!(Rc::ptr_eq(&restored, &host));
        let nullable = WeakValue::Object(
            native.clone().upcast::<FerroObject>().downgrade(),
            TypeId::of::<Option<Rc<dyn IHostWindow>>>(),
        );
        let restored_nullable =
            from_markup_value::<Option<Rc<dyn IHostWindow>>>(&nullable.upgrade())
                .unwrap()
                .unwrap();
        assert!(Rc::ptr_eq(&restored_nullable, &host));
        let unrelated = WeakValue::Object(
            native.clone().upcast::<FerroObject>().downgrade(),
            TypeId::of::<Rc<dyn ILocalTarget>>(),
        );
        assert!(unrelated.upgrade().is_none());
        native.close();
        drop(restored);
        drop(restored_nullable);
        drop(boxed);
        drop(host);
        drop(native);
        // HostWindowState retains its original strong readonly host reference.
        // General source GC-cycle collection remains a separate port requirement.
    });
}

#[test]
fn shared_weak_source_restores_inherited_brush_interface_without_retaining_the_native_object() {
    run(|| {
        use ferroui_base::{
            data::core::WeakValue,
            media::{IBrush, SolidColorBrush},
            FerroObject,
        };
        ValueTypes::register_interface_object::<Rc<dyn IBrush>>(|brush| {
            brush.as_object().map(FerroObject::to_ref)
        });
        let native = SolidColorBrush::new();
        let weak_native = native.downgrade();
        let brush: Rc<dyn IBrush> = native.clone().into();
        let boxed: BoxedValue = Rc::new(brush.clone());
        let weak = WeakValue::new(&boxed);
        assert!(matches!(weak, WeakValue::Object(_, _)));
        let restored = from_markup_value::<Rc<dyn IBrush>>(&weak.upgrade()).unwrap();
        assert_eq!(
            restored.as_object().unwrap().to_ref(),
            native.clone().upcast::<FerroObject>()
        );
        let boxed_restored: BoxedValue = Rc::new(restored.clone());
        assert!(ValueTypes::reference_equals(&boxed, &boxed_restored));
        drop(boxed_restored);
        drop(restored);
        drop(boxed);
        drop(brush);
        drop(native);
        assert!(weak_native.upgrade().is_none());
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn core_host_metadata_drives_existing_native_geometry_layout_state_and_lifecycle() {
    run(|| {
        let native = HostWindow::new();
        let host = HostWindow::as_host_window(native.clone());
        let boxed: BoxedValue = Rc::new(host.clone());
        let factory = ferrodock_model_reactiveui::Factory::new();
        let layout: Rc<dyn IDock> = factory.create_tool_dock();
        call(&boxed, "SetLayout", vec![Some(Rc::new(layout.clone()))]);
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<dyn IDockable>>(&native.data_context()).unwrap(),
            &(layout.clone() as Rc<dyn IDockable>)
        ));
        assert!(native.is_tool_window());
        call(
            &boxed,
            "SetPosition",
            vec![Some(Rc::new(-12.0)), Some(Rc::new(20.0))],
        );
        assert_eq!(host.get_position(), (-12.0, 20.0));
        call(
            &boxed,
            "SetSize",
            vec![Some(Rc::new(640.0)), Some(Rc::new(480.0))],
        );
        assert_eq!(host.get_size(), (640.0, 480.0));
        call(
            &boxed,
            "SetSize",
            vec![Some(Rc::new(f64::NAN)), Some(Rc::new(500.0))],
        );
        assert_eq!(host.get_size(), (640.0, 500.0));
        for state in [
            DockWindowState::Normal,
            DockWindowState::Minimized,
            DockWindowState::Maximized,
            DockWindowState::FullScreen,
        ] {
            call(&boxed, "SetWindowState", vec![Some(Rc::new(state))]);
            assert_eq!(
                from_markup_value::<DockWindowState>(&call(&boxed, "GetWindowState", vec![])),
                Some(state)
            );
        }
        call(
            &boxed,
            "SetWindowState",
            vec![Some(Rc::new(DockWindowState::Normal))],
        );
        call(
            &boxed,
            "SetTitle",
            vec![Some(Rc::new("Original host".to_owned()))],
        );
        call(&boxed, "SetTitle", vec![None]);
        call(&boxed, "SetTitle", vec![Some(Rc::new(String::new()))]);
        assert_eq!(native.title().as_deref(), Some("Original host"));
        call(&boxed, "Present", vec![Some(Rc::new(false))]);
        assert!(native.is_visible());
        call(&boxed, "SetActive", vec![]);
        call(&boxed, "Exit", vec![]);
        assert!(!native.is_visible());
    });
}

#[derive(Default)]
struct Container {
    theme: RefCell<Option<ferrodock_model::BoxedValue>>,
    selector: RefCell<Option<ferrodock_model::BoxedValue>>,
}
impl IDockItemContainerMetadata for Container {
    fn item_container_theme(&self) -> Option<ferrodock_model::BoxedValue> {
        self.theme.borrow().clone()
    }
    fn set_item_container_theme(&self, value: Option<ferrodock_model::BoxedValue>) {
        *self.theme.borrow_mut() = value;
    }
    fn item_template_selector(&self) -> Option<ferrodock_model::BoxedValue> {
        self.selector.borrow().clone()
    }
    fn set_item_template_selector(&self, value: Option<ferrodock_model::BoxedValue>) {
        *self.selector.borrow_mut() = value;
    }
}
fn assert_container_bindings(panel: &Ref<StackPanel>) {
    let container = Rc::new(Container::default());
    let contract: Rc<dyn IDockItemContainerMetadata> = container.clone();
    let boxed: BoxedValue = Rc::new(contract);
    let control: Ref<Control> = TextBlock::new().upcast();
    let raw = Rc::new(control.clone());
    container.set_item_container_theme(Some(raw.clone()));
    container.set_item_template_selector(Some(Rc::new("selector key".to_owned())));
    panel.set_data_context(Some(boxed.clone()));
    Dispatcher::ui_thread().run_jobs(None);
    let direct = get(
        <dyn IDockItemContainerMetadataDescriptor>::MARKUP,
        &boxed,
        "ItemContainerTheme",
    )
    .unwrap();
    assert!(Rc::ptr_eq(&direct, &(raw.clone() as BoxedValue)));
    let content = panel
        .children()
        .get(0)
        .cast::<ContentControl>()
        .unwrap()
        .content()
        .unwrap();
    assert!(ValueTypes::reference_equals(&content, &(raw as BoxedValue)));
    assert_eq!(Control::from_boxed(&content), Some(control));
    let text = panel.children().get(1).cast::<TextBox>().unwrap();
    assert_eq!(text.text().as_deref(), Some("selector key"));
    text.set_text(Some("edited selector"));
    Dispatcher::ui_thread().run_jobs(None);
    let selector = get(
        <dyn IDockItemContainerMetadataDescriptor>::MARKUP,
        &boxed,
        "ItemTemplateSelector",
    )
    .unwrap();
    assert_eq!(
        selector.downcast_ref::<String>().map(String::as_str),
        Some("edited selector"),
        "actual selector type: {}; nullable string: {:?}",
        selector.as_ref().type_name(),
        selector.downcast_ref::<Option<String>>()
    );
    text.set_text(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(get(
        <dyn IDockItemContainerMetadataDescriptor>::MARKUP,
        &boxed,
        "ItemTemplateSelector"
    )
    .is_none());
    panel.set_data_context(None);
}
#[test]
fn core_container_compiled_bindings_preserve_raw_control_metadata_and_original_object_setters() {
    run(|| {
        for (core, alias) in [
            (
                <dyn ILocalTargetMetadata>::MARKUP,
                <dyn ferrodock_ferro::ILocalTargetMetadata>::MARKUP,
            ),
            (
                <dyn IGlobalTargetMetadata>::MARKUP,
                <dyn ferrodock_ferro::IGlobalTargetMetadata>::MARKUP,
            ),
            (
                <dyn IDockItemContainerMetadataDescriptor>::MARKUP,
                <dyn ferrodock_ferro::IDockItemContainerMetadataDescriptor>::MARKUP,
            ),
        ] {
            assert!(std::ptr::eq(core, alias));
        }
        assert_container_bindings(&load("CoreContainerBindings"));
    });
}

#[test]
fn shared_object_setter_normalizes_nullable_transport_boxes_and_retains_enclosing_controls() {
    run(|| {
        use ferroui_markup_xaml::xaml_il::runtime::compiled::object_assignment_value;
        let text: BoxedValue = Rc::new("assigned string".to_owned());
        let nested: BoxedValue = Rc::new(Some(Rc::new(Some(text.clone())) as BoxedValue));
        assert!(Rc::ptr_eq(
            &object_assignment_value(Some(nested)).unwrap(),
            &text
        ));
        for value in [
            None,
            Some(Rc::new(None::<BoxedValue>) as BoxedValue),
            Some(Rc::new(None::<String>) as BoxedValue),
        ] {
            assert!(object_assignment_value(value).is_none());
        }
        let nullable: BoxedValue = Rc::new(Some("nullable string".to_owned()));
        assert_eq!(
            object_assignment_value(Some(nullable))
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("nullable string")
        );
        let number: BoxedValue = Rc::new(42_i32);
        assert!(Rc::ptr_eq(
            &object_assignment_value(Some(number.clone())).unwrap(),
            &number
        ));
        let control = TextBlock::new();
        let original: BoxedValue = Rc::new(control.clone());
        let assigned = object_assignment_value(Some(original.clone())).unwrap();
        assert!(ValueTypes::reference_equals(&original, &assigned));
        assert_eq!(Control::from_boxed(&assigned), Some(control.upcast()));
    });
}

#[test]
fn both_runtime_compiler_and_interpreter_use_original_core_host_and_container_contracts() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let documents = [
            ("Host.xaml", include_str!("CoreHostBindings.xaml")),
            ("Container.xaml", include_str!("CoreContainerBindings.xaml")),
        ];
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(emitted.len(), 2);
        for document in emitted {
            assert!(document.source.is_ok(), "{:?}", document.source);
        }
        for (index, (_, source)) in documents.iter().enumerate() {
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
                Some(config.clone()),
            )
            .unwrap();
            let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
            if index == 0 {
                assert_host_bindings(panel);
            } else {
                assert_container_bindings(panel);
            }
        }
    });
}
