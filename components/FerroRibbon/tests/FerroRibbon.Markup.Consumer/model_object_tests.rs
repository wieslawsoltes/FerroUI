//! Actual native object metadata versus the whole unchanged Core oracle.
use ferroribbon::metadata::models::*;
use ferroribbon_core::model::*;
use ferroui_base::{
    AnyValue, BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};
#[path = "model_value_transfer_tests.rs"]
mod transfer_tests;
type Trace = Rc<RefCell<Vec<String>>>;
type Hook = Rc<dyn Fn()>;

// Application subclass matching the unchanged model-metadata oracle's EqualButton.
// The native value facade can compare only this concrete type; the model bridge
// must still dispatch its virtual base operation against other model classes.
struct EqualButton {
    base: RibbonButtonModel,
}
impl EqualButton {
    fn new(label: &str) -> Rc<Self> {
        Rc::new(Self {
            base: RibbonButtonModel::construct(Some("id".into()), Some(label.into())),
        })
    }
}
impl PartialEq for EqualButton {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl INotifyPropertyChanged for EqualButton {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for EqualButton {}
impl RibbonNodeModelImpl for EqualButton {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
    fn equals_node(&self, other: &dyn RibbonNodeModelImpl) -> bool {
        self.label() == other.label()
    }
}
impl RibbonItemModelImpl for EqualButton {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for EqualButton {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
struct Payload {
    name: &'static str,
    mode: &'static str,
    key: &'static str,
    state: Cell<i32>,
    trace: Trace,
    hook: RefCell<Option<Hook>>,
}
impl Payload {
    fn new(name: &'static str, mode: &'static str, key: &'static str, trace: Trace) -> Rc<Self> {
        Rc::new(Self {
            name,
            mode,
            key,
            state: Cell::new(0),
            trace,
            hook: RefCell::new(None),
        })
    }
}
struct Other {
    name: &'static str,
    key: &'static str,
    trace: Trace,
}
fn name(value: &dyn Any) -> String {
    if let Some(p) = value.downcast_ref::<Payload>() {
        p.name.into()
    } else if let Some(p) = value.downcast_ref::<Other>() {
        p.name.into()
    } else {
        "value".into()
    }
}
impl AnyValue for Payload {
    fn any_value_as_any(&self) -> &dyn Any {
        self
    }
    fn any_value_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        self.trace
            .borrow_mut()
            .push(format!("equals:{}:{}", self.name, name(other.as_any())));
        let hook = self.hook.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
        match self.mode {
            "false" => false,
            "value" => other
                .downcast_ref::<Payload>()
                .is_some_and(|p| self.key == p.key),
            "cross" => other
                .downcast_ref::<Other>()
                .is_some_and(|p| self.key == p.key),
            _ => other
                .downcast_ref::<Payload>()
                .is_some_and(|p| std::ptr::eq(self, p)),
        }
    }
}
impl AnyValue for Other {
    fn any_value_as_any(&self) -> &dyn Any {
        self
    }
    fn any_value_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        self.trace
            .borrow_mut()
            .push(format!("equals:{}:{}", self.name, name(other.as_any())));
        other
            .downcast_ref::<Other>()
            .is_some_and(|p| std::ptr::eq(self, p))
    }
}
fn owner<T: Any>(value: Rc<T>) -> BoxedValue {
    ValueTypes::box_erased(value).expect("registered model")
}
fn property(m: &MarkupType, name: &str) -> &'static ferroui_base::metadata::MarkupProperty {
    let mut current = Some(m);
    while let Some(m) = current {
        if let Some(property) = m.find_property(name) {
            return property;
        }
        current = m.base_type();
    }
    panic!("missing inherited property {name}")
}
fn set(m: &MarkupType, name: &str, model: &BoxedValue, value: Option<BoxedValue>) {
    (property(m, name).set.unwrap())(&[Some(model.clone()), value]).unwrap();
}
fn get(m: &MarkupType, name: &str, model: &BoxedValue) -> Option<BoxedValue> {
    (property(m, name).get.unwrap())(&[Some(model.clone())]).unwrap()
}
fn expected() -> Value {
    serde_json::from_str(include_str!("../fixtures/model-object.json")).unwrap()
}

#[test]
fn native_object_fields_dispatch_source_virtual_equality_across_model_classes_and_views() {
    crate::tests::run(|| {
        register_ribbon_button_model::<EqualButton>().unwrap();
        ValueTypes::register_reference::<EqualButton>();
        ValueTypes::register_cast::<Rc<EqualButton>, Rc<dyn RibbonNodeModelImpl>>(|v| v.clone());
        let first = owner(EqualButton::new("same"));
        let same = owner(EqualButton::new("same"));
        let different = owner(EqualButton::new("other"));
        let ordinary = owner(RibbonButtonModel::new(
            Some("x".into()),
            Some("same".into()),
        ));
        let equals = |a: BoxedValue, b: BoxedValue| {
            let host = owner(RibbonButtonModel::new(None, None));
            let meta = RibbonNodeModelMetadata::MARKUP;
            set(meta, "Tag", &host, Some(a));
            let original = get(meta, "Tag", &host).unwrap();
            set(meta, "Tag", &host, Some(b));
            Rc::ptr_eq(&original, &get(meta, "Tag", &host).unwrap())
        };
        let node_view = |v: &BoxedValue| {
            ValueTypes::try_cast(v, ValueType::of::<Rc<dyn RibbonNodeModelImpl>>()).unwrap()
        };
        let actual = json!({
            "same":equals(first.clone(),same.clone()),
            "different":equals(first.clone(),different.clone()),
            "sameThroughNode":equals(node_view(&first),node_view(&same)),
            "ordinary":equals(first.clone(),ordinary.clone()),
            "reverse":equals(ordinary.clone(),first.clone()),
            "erased":equals(first.clone(),same),
            "erasedOther":equals(first.clone(),different),
            "erasedOrdinary":equals(first,ordinary),
        });
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        assert_eq!(actual, source["corpus"]["equality"]);
    });
}

#[test]
fn object_metadata_does_not_root_unregistered_payload_after_properties_and_readers_release_it() {
    crate::tests::run(|| {
        let node = RibbonButtonModel::new(None, None);
        let model = owner(node.clone());
        let meta = RibbonButtonModelMetadata::MARKUP;
        let payload = Payload::new("owned", "identity", "one", Rc::default());
        let weak = Rc::downgrade(&payload);
        set(meta, "Tag", &model, Some(payload.clone()));
        set(meta, "CommandParameter", &model, Some(payload.clone()));
        let reader = get(meta, "Tag", &model).unwrap();
        assert!(Rc::ptr_eq(&reader, &(payload.clone() as BoxedValue)));
        drop(payload);
        set(meta, "Tag", &model, None);
        drop(reader);
        assert!(weak.upgrade().is_some());
        set(meta, "CommandParameter", &model, None);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn roundtrip_to_same_property_retains_core_equality_operation_and_does_not_skip_false_self() {
    crate::tests::run(|| {
        ValueTypes::register_erased_value::<i32>();
        let node = RibbonButtonModel::new(None, None);
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let value = Rc::new(37_i32);
        let weak = Rc::downgrade(&value);
        node.set_tag(Some(RibbonObject::with_equality(
            value,
            move |old, other| {
                assert!(std::ptr::eq(old, other.downcast_ref::<i32>().unwrap()));
                observed.set(observed.get() + 1);
                false
            },
        )));
        let model = owner(node.clone());
        let native = get(RibbonButtonModelMetadata::MARKUP, "Tag", &model).unwrap();
        set(
            RibbonButtonModelMetadata::MARKUP,
            "Tag",
            &model,
            Some(native.clone()),
        );
        assert_eq!(calls.get(), 1);
        node.set_tag(None);
        assert!(weak.upgrade().is_some());
        drop(native);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn unknown_core_origin_payload_errors_explicitly_without_replacing_the_actual_field() {
    crate::tests::run(|| {
        struct CoreOnly(Cell<i32>);
        let payload = Rc::new(CoreOnly(Cell::new(9)));
        let weak = Rc::downgrade(&payload);
        let node = RibbonButtonModel::new(None, None);
        node.set_tag(Some(RibbonObject::reference(payload.clone())));
        let model = owner(node.clone());
        let result = (property(RibbonButtonModelMetadata::MARKUP, "Tag")
            .get
            .unwrap())(&[Some(model)]);
        let error = match result {
            Err(error) => error.to_string(),
            Ok(_) => panic!("unknown Core-origin boxer must not silently return null or a proxy"),
        };
        assert!(
            error.contains("Unregistered native model payload"),
            "{error}"
        );
        assert!(std::ptr::eq(
            node.tag().unwrap().downcast_ref::<CoreOnly>().unwrap(),
            payload.as_ref()
        ));
        assert_eq!(
            node.tag()
                .unwrap()
                .downcast_ref::<CoreOnly>()
                .unwrap()
                .0
                .get(),
            9
        );
        node.set_tag(None);
        drop(payload);
        assert!(weak.upgrade().is_none());
    });
}

/// Representation boundary: a per-reference Core callback is not
/// intrinsic to a raw native domain value. This diagnostic asserts source behavior and
/// must remain red until transferring the operation preserves identity/lifetime.
#[cfg(feature = "source-model-object-equality-transfer-reproducer")]
#[test]
fn source_object_equality_operation_survives_transfer_to_another_model_via_native_value() {
    crate::tests::run(|| {
        #[derive(PartialEq)]
        struct CoreDomain(&'static str);
        #[derive(PartialEq)]
        struct OtherDomain(&'static str);
        ValueTypes::register_erased_value::<CoreDomain>();
        ValueTypes::register_erased_value::<OtherDomain>();
        let source = RibbonButtonModel::new(None, None);
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        source.set_tag(Some(RibbonObject::with_equality(
            Rc::new(CoreDomain("same")),
            move |old, other| {
                observed.set(observed.get() + 1);
                other
                    .downcast_ref::<OtherDomain>()
                    .is_some_and(|other| old.0 == other.0)
            },
        )));
        let target = RibbonButtonModel::new(None, None);
        let source = owner(source);
        let target = owner(target);
        let meta = RibbonButtonModelMetadata::MARKUP;
        let original = get(meta, "Tag", &source).unwrap();
        set(meta, "Tag", &target, Some(original.clone()));
        set(meta, "Tag", &source, None);
        let other = ValueTypes::box_erased(Rc::new(OtherDomain("same"))).unwrap();
        set(meta, "Tag", &target, Some(other));
        assert_eq!(
            calls.get(),
            1,
            "source virtual equality must transfer with the object"
        );
        let source_cross = expected()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["scenario"] == "cross")
            .unwrap()["retained"]
            .as_bool()
            .unwrap();
        assert_eq!(
            Rc::ptr_eq(&get(meta, "Tag", &target).unwrap(), &original),
            source_cross
        );
    });
}
#[test]
fn original_object_property_identity_equality_reentry_and_constructor_cases_match_native_metadata()
{
    crate::tests::run(|| {
        let mut cases = Vec::new();
        for slot in [
            "Tag",
            "CommandParameter",
            "BackstageContent",
            "BackstageCommandParameter",
        ] {
            let node = RibbonButtonModel::new(Some("id".into()), Some("Label".into()));
            let backstage = RibbonBackstageItemModel::new(Some("info".into()), Some("Info".into()));
            let (m, property, model) = match slot {
                "Tag" => (
                    RibbonButtonModelMetadata::MARKUP,
                    "Tag",
                    owner(node.clone()),
                ),
                "CommandParameter" => (
                    RibbonButtonModelMetadata::MARKUP,
                    "CommandParameter",
                    owner(node.clone()),
                ),
                "BackstageContent" => (
                    RibbonBackstageItemModelMetadata::MARKUP,
                    "Content",
                    owner(backstage.clone()),
                ),
                _ => (
                    RibbonBackstageItemModelMetadata::MARKUP,
                    "CommandParameter",
                    owner(backstage.clone()),
                ),
            };
            let notifications = Rc::new(Cell::new(0));
            let n = notifications.clone();
            if slot.starts_with("Backstage") {
                backstage
                    .property_changed()
                    .add(Rc::new(move |_| n.set(n.get() + 1)));
            } else {
                node.property_changed()
                    .add(Rc::new(move |_| n.set(n.get() + 1)));
            }
            // No Clone, PartialEq or erased-boxer registration on this application payload.
            let payload = Payload::new("payload", "identity", "same", Rc::default());
            assert!(ValueTypes::box_erased(payload.clone()).is_err());
            let value: BoxedValue = payload.clone();
            set(m, property, &model, Some(value.clone()));
            let actual = get(m, property, &model).unwrap();
            let same = Rc::ptr_eq(&actual, &value);
            payload.state.set(9);
            let shared = actual.downcast_ref::<Payload>().unwrap().state.get();
            set(m, property, &model, Some(value));
            set(m, property, &model, None);
            cases.push(json!({"name":"property","slot":slot,"same":same,"shared":shared,"notifications":notifications.get(),"cleared":get(m,property,&model).is_none()}));
        }
        for scenario in ["false_self", "value", "reenter", "throw", "cross"] {
            let node = RibbonButtonModel::new(Some("id".into()), Some("Label".into()));
            let model = owner(node.clone());
            let trace: Trace = Rc::default();
            let output = trace.clone();
            let weak = Rc::downgrade(&node);
            node.property_changed().add(Rc::new(move |property| {
                let node = weak.upgrade().unwrap();
                output.borrow_mut().push(if property == "Tag" {
                    format!(
                        "Tag:{}",
                        node.tag()
                            .map(|v| name(v.payload().as_ref()))
                            .unwrap_or_else(|| "null".into())
                    )
                } else {
                    property.into()
                });
            }));
            let a = Payload::new(
                "a",
                if scenario == "false_self" {
                    "false"
                } else if scenario == "cross" {
                    "cross"
                } else {
                    "value"
                },
                "same",
                trace.clone(),
            );
            let b = Payload::new("b", "identity", "same", trace.clone());
            let m = RibbonButtonModelMetadata::MARKUP;
            set(m, "Tag", &model, Some(a.clone()));
            let mut retained = false;
            let mut threw = false;
            match scenario {
                "false_self" => {
                    set(m, "Tag", &model, Some(a.clone()));
                    set(m, "Tag", &model, Some(a.clone()));
                }
                "value" => {
                    set(m, "Tag", &model, Some(b));
                    retained =
                        Rc::ptr_eq(&get(m, "Tag", &model).unwrap(), &(a.clone() as BoxedValue));
                    set(
                        m,
                        "Tag",
                        &model,
                        Some(Payload::new("c", "identity", "different", trace.clone())),
                    );
                }
                "cross" => {
                    let other = Rc::new(Other {
                        name: "other",
                        key: "same",
                        trace: trace.clone(),
                    });
                    set(m, "Tag", &model, Some(other.clone()));
                    retained =
                        Rc::ptr_eq(&get(m, "Tag", &model).unwrap(), &(a.clone() as BoxedValue));
                    set(m, "Tag", &model, None);
                    set(m, "Tag", &model, Some(other));
                    set(m, "Tag", &model, Some(a));
                }
                _ => {
                    if scenario == "throw" {
                        a.hook.replace(Some(Rc::new(|| panic!("equals"))));
                    } else {
                        let weak = Rc::downgrade(&node);
                        a.hook.replace(Some(Rc::new(move || {
                            weak.upgrade().unwrap().set_label(Some("reentered".into()))
                        })));
                    }
                    threw = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        set(m, "Tag", &model, Some(b))
                    }))
                    .is_err();
                }
            }
            cases.push(json!({"name":"equality","scenario":scenario,"trace":*trace.borrow(),"retained":retained,"threw":threw,"final":node.tag().map(|v|name(v.payload().as_ref())).unwrap_or_else(||"null".into()),"label":node.label().map(|v|v.to_utf8().unwrap())}));
            node.set_tag(None);
        }
        {
            let node = RibbonButtonModel::new(Some("id".into()), Some("Label".into()));
            let model = owner(node.clone());
            let notifications = Rc::new(Cell::new(0));
            let count = notifications.clone();
            node.property_changed()
                .add(Rc::new(move |_| count.set(count.get() + 1)));
            for value in [
                Some(Rc::new(f64::NAN) as BoxedValue),
                Some(Rc::new(f64::NAN)),
                Some(Rc::new(0_f64)),
                Some(Rc::new(-0_f64)),
                Some(Rc::new("e\u{301} 😀".to_owned())),
                Some(Rc::new("e\u{301} 😀".to_owned())),
                Some(Rc::new("é 😀".to_owned())),
                None,
            ] {
                set(RibbonButtonModelMetadata::MARKUP, "Tag", &model, value);
            }
            cases.push(json!({"name":"scalars","notifications":notifications.get(),"cleared":node.tag().is_none()}));
        }
        {
            let payload = Payload::new("content", "identity", "same", Rc::default());
            let value: BoxedValue = payload.clone();
            let meta = RibbonBackstageItemModelMetadata::MARKUP;
            let ctor = meta
                .constructors
                .iter()
                .find(|c| c.parameters.len() == 4)
                .unwrap();
            let model = (ctor.invoke)(&[
                Some(Rc::new("id".to_owned())),
                Some(Rc::new("Info".to_owned())),
                None,
                Some(value.clone()),
            ])
            .unwrap()
            .unwrap();
            payload.state.set(12);
            let actual = get(meta, "Content", &model).unwrap();
            cases.push(json!({"name":"constructor","same":Rc::ptr_eq(&actual,&value),"shared":actual.downcast_ref::<Payload>().unwrap().state.get(),"Id":get(meta,"Id",&model).unwrap().downcast_ref::<String>().unwrap(),"Label":get(meta,"Label",&model).unwrap().downcast_ref::<String>().unwrap(),"commandParameterNull":get(meta,"CommandParameter",&model).is_none()}));
        }
        assert_eq!(Value::Array(cases), expected()["corpus"]["cases"]);
    });
}

#[test]
fn compiled_runtime_model_objects_reuse_native_child_and_command_parameter_through_factory_and_qat()
{
    crate::tests::run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        use ferroribbon_core::commands::RibbonRelayCommand;
        use ferroui_base::{Rect, Size, threading::Dispatcher};
        use ferroui_controls::{Border, Control, StackPanel, Window};
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ObjectModels.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ObjectModels.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |name: &str| {
                host.resources()
                    .try_get_resource(&name.into(), None)
                    .flatten()
                    .unwrap()
            };
            let body = ValueTypes::as_object(&*resource("Body"))
                .unwrap()
                .cast::<Border>()
                .unwrap();
            let graph = ValueTypes::try_cast(
                &resource("Graph"),
                ValueType::of::<Rc<dyn RibbonModelImpl>>(),
            )
            .unwrap();
            let graph = graph
                .downcast_ref::<Rc<dyn RibbonModelImpl>>()
                .unwrap()
                .clone();
            let command = ValueTypes::try_cast(
                &resource("Command"),
                ValueType::of::<Rc<dyn RibbonItemModelImpl>>(),
            )
            .unwrap();
            let command = command
                .downcast_ref::<Rc<dyn RibbonItemModelImpl>>()
                .unwrap()
                .clone();
            let info = ValueTypes::try_cast(
                &resource("Information"),
                ValueType::of::<Rc<dyn RibbonBackstageItemModelImpl>>(),
            )
            .unwrap();
            let info = info
                .downcast_ref::<Rc<dyn RibbonBackstageItemModelImpl>>()
                .unwrap()
                .clone();
            assert!(
                ValueTypes::as_object(
                    &*get(RibbonItemModelMetadata::MARKUP, "Tag", &resource("Command")).unwrap()
                )
                .unwrap()
                .ptr_eq(&body)
            );
            assert!(Rc::ptr_eq(
                &get(
                    RibbonBackstageItemModelMetadata::MARKUP,
                    "Tag",
                    &resource("Information")
                )
                .unwrap(),
                &resource("Command")
            ));
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            assert!(Rc::ptr_eq(&r.model().unwrap(), &graph));
            graph.backstage().items().add(Some(info.clone()));
            let native_backstage = r
                .backstage()
                .expect("model item activates generated backstage");
            assert_eq!(
                native_backstage.items().count(),
                1,
                "shared backstage model collection updates its native items; runtime={runtime}"
            );
            let calls = Rc::new(Cell::new(0));
            let count = calls.clone();
            let expected_parameter = command.command_parameter().unwrap().payload();
            command.set_command(Some(RibbonRelayCommand::new_with_parameter(
                move |parameter| {
                    assert!(std::ptr::addr_eq(
                        parameter.expect("same command object parameter") as *const dyn Any,
                        expected_parameter.as_ref() as *const dyn Any,
                    ));
                    count.set(count.get() + 1);
                },
                None,
            )));
            graph.quick_access_items().add(Some(command.clone()));
            let original = r
                .find_item(Some("run".into()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            let copy = r
                .quick_access_tool_bar()
                .unwrap()
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            assert!(
                ValueTypes::as_object(&*original.command_parameter().unwrap())
                    .unwrap()
                    .ptr_eq(&body)
            );
            // Source RibbonButton.OnRibbonClick forwards to SourceItem.PerformClick.
            // LinkCommon intentionally leaves the copy's command/parameter empty;
            // copying them would execute the original command twice.
            assert!(copy.command().is_none());
            assert!(copy.command_parameter().is_none());
            original.invoke_core();
            copy.invoke_core();
            assert_eq!(calls.get(), 2);
            let native_info = native_backstage
                .items()
                .get(0)
                .expect("generated navigation item");
            assert!(
                ValueTypes::as_object(&*native_info.content().unwrap())
                    .unwrap()
                    .ptr_eq(&body)
            );
            let w = Window::new();
            w.set_width(900.);
            w.set_height(500.);
            w.set_content(Some(Control::boxed(host.clone())));
            w.show();
            w.measure(Size::new(900., 500.));
            w.arrange(Rect::new(0., 0., 900., 500.));
            Dispatcher::ui_thread().run_jobs(None);
            let parameter = Payload::new("replacement", "identity", "live", Rc::default());
            let weak_parameter = Rc::downgrade(&parameter);
            let expected_parameter = parameter.clone();
            let count = calls.clone();
            let enabled = Rc::new(Cell::new(true));
            let can_execute = enabled.clone();
            let replacement_command = RibbonRelayCommand::new_with_parameter(
                move |value| {
                    assert!(std::ptr::eq(
                        value.unwrap().downcast_ref::<Payload>().unwrap(),
                        expected_parameter.as_ref()
                    ));
                    assert_eq!(expected_parameter.state.get(), 29);
                    count.set(count.get() + 1);
                },
                Some(Rc::new(move |_| can_execute.get())),
            );
            command.set_command_parameter(ferroribbon::mvvm::model_values::from_native(Some(
                parameter.clone(),
            )));
            command.set_command(Some(replacement_command.clone()));
            parameter.state.set(29);
            assert!(Rc::ptr_eq(
                &original.command_parameter().unwrap(),
                &(parameter.clone() as BoxedValue)
            ));
            enabled.set(false);
            replacement_command.notify_can_execute_changed();
            original.invoke_core();
            copy.invoke_core();
            assert_eq!(calls.get(), 2);
            enabled.set(true);
            replacement_command.notify_can_execute_changed();
            original.invoke_core();
            copy.invoke_core();
            assert_eq!(calls.get(), 4);
            graph.quick_access_items().clear();
            assert!(r.quick_access_tool_bar().unwrap().items().is_empty());
            let unlinked_label = copy.label();
            command.set_label(Some("Changed after removal".into()));
            assert_eq!(original.label().as_deref(), Some("Changed after removal"));
            assert_eq!(
                copy.label(),
                unlinked_label,
                "removed copy subscriptions are detached"
            );
            // Original Unlink/OnUnlinked do not clear SourceItem. An application
            // retaining a removed button can still invoke its original source.
            assert_eq!(
                ferroribbon::controls::RibbonItemHelper::get_source_item(&copy),
                Some(original.clone().upcast())
            );
            copy.invoke_core();
            assert_eq!(
                calls.get(),
                5,
                "retained removed copy preserves the original source invocation behavior"
            );
            command.set_command_parameter(None);
            command.set_command(None);
            drop(parameter);
            drop(replacement_command);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(weak_parameter.upgrade().is_none());
            r.set_is_backstage_open(true);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(r.is_backstage_open());
            assert!(body.visual_parent().is_some());
            body.set_width(137.);
            r.set_is_backstage_open(false);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                ValueTypes::as_object(&*native_info.content().unwrap())
                    .unwrap()
                    .ptr_eq(&body)
            );
            r.set_is_backstage_open(true);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(r.is_backstage_open());
            assert!(r.backstage().unwrap().ptr_eq(&native_backstage));
            assert!(
                ValueTypes::as_object(&*native_info.content().unwrap())
                    .unwrap()
                    .ptr_eq(&body)
            );
            assert_eq!(body.width(), 137.);
            assert!(body.visual_parent().is_some());
            r.set_is_backstage_open(false);
            let replacement = Border::new();
            let boxed = Control::boxed(replacement.clone());
            info.set_content(ferroribbon::mvvm::model_values::from_native(Some(boxed)));
            assert!(
                ValueTypes::as_object(&*native_info.content().unwrap())
                    .unwrap()
                    .ptr_eq(&replacement)
            );
            r.set_model(None);
            w.close();
            w.set_content(None);
            Dispatcher::ui_thread().run_jobs(None);
        }
    });
}
