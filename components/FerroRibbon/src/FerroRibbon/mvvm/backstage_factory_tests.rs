use super::*;
use crate::{
    compat::{DataTemplateSelector, DataTemplateSelectorExt, DataTemplateSelectorImpl},
    controls::*,
};
use ferroribbon_core::{commands::*, model::*};
use ferroui_base::{BoxedValue, FerroObject, data::core::ValueTypes};
use ferroui_controls::{
    Control, TextBlock,
    templates::{FuncDataTemplate, IDataTemplate},
};
use serde_json::json;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};
fn check(name: &str, value: serde_json::Value) {
    super::element_factory_tests::check(name, value);
}
fn page(id: &str) -> Rc<RibbonBackstageItemModel> {
    let m = RibbonBackstageItemModel::new(Some(id.into()), Some(id.into()));
    m.set_content(Some(RibbonObject::value("page".to_owned())));
    m
}
fn snapshot(b: &RibbonBackstage, m: &RibbonBackstageModel) -> serde_json::Value {
    json!({"model":m.selected_item().and_then(|i|i.id()).map(|s|s.to_utf8().unwrap()),"element":b.selected_item().and_then(|i|i.id()),"count":b.items().count()})
}
fn same(
    a: &Option<Rc<dyn RibbonBackstageItemModelImpl>>,
    b: &Rc<RibbonBackstageItemModel>,
) -> bool {
    a.as_ref()
        .is_some_and(|a| Rc::ptr_eq(a, &(b.clone() as Rc<dyn RibbonBackstageItemModelImpl>)))
}
struct Payload {
    value: Cell<i32>,
}
impl PartialEq for Payload {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl std::fmt::Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "payload:{}", self.value.get())
    }
}
type Callback = Rc<dyn Fn()>;
struct Selector {
    template: Rc<dyn IDataTemplate>,
    calls: RefCell<Vec<String>>,
    callback: RefCell<Option<Callback>>,
}
impl Selector {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            template: FuncDataTemplate::new(
                |_| true,
                |_, _| Some(TextBlock::new().upcast()),
                false,
            ),
            calls: RefCell::new(vec![]),
            callback: RefCell::default(),
        })
    }
}
impl DataTemplateSelectorImpl for Selector {
    fn select_template_core(&self, item: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.calls.borrow_mut().push(
            item.as_ref()
                .map(|v| ValueTypes::to_display_string(Some(v)))
                .unwrap_or("null".into()),
        );
        let callback = self.callback.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        Some(self.template.clone())
    }
}
#[test]
fn original_factory_backstage_selection_mirror_duplicates_removal_and_disposal() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let m = RibbonBackstageModel::new();
        m.set_title(Some("File".into()));
        m.set_is_open(true);
        let a = page("a");
        a.set_is_enabled(false);
        let z = page("z");
        m.items().add(Some(a.clone()));
        m.items().add(Some(z.clone()));
        m.set_selected_item(Some(z.clone()));
        let b = f.create_backstage(Some(m.clone()), Some(scope.clone()));
        let ae = b.items().get(0).unwrap();
        let ze = b.items().get(1).unwrap();
        let mut trace = vec![snapshot(&b, &m)];
        b.set_selected_item(Some(ae.clone()));
        trace.push(snapshot(&b, &m));
        m.set_selected_item(None);
        trace.push(snapshot(&b, &m));
        b.set_selected_item(None);
        trace.push(snapshot(&b, &m));
        m.items().move_item(1, 0);
        trace.push(snapshot(&b, &m));
        let moved = b.items().get(0) == Some(ze.clone());
        m.set_selected_item(Some(z.clone()));
        m.items()
            .remove(&Some(z.clone() as Rc<dyn RibbonBackstageItemModelImpl>));
        trace.push(snapshot(&b, &m));
        z.set_label(Some("Removed".into()));
        let stopped = ze.header();
        m.items().add(Some(a.clone()));
        let distinct = b.items().get(0) != b.items().get(1);
        m.items().remove_at(0);
        let retained = b.items().get(0) == Some(ae);
        m.set_title(Some("Changed".into()));
        scope.dispose();
        m.set_title(Some("Disposed".into()));
        let current = m.selected_item();
        b.set_selected_item(None);
        let stopped_selection = match (current, m.selected_item()) {
            (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
            (None, None) => true,
            _ => false,
        };
        check(
            "backstage_selection",
            json!({"trace":trace,"moved":moved,"stopped":stopped,"distinct":distinct,"retained":retained,"title":b.title(),"selectionStopped":stopped_selection,"remaining":b.items().count(),"noRootModel":RibbonElementFactory::get_model(&b).is_none(),"isOpen":m.is_open()}),
        );
    });
}
#[test]
fn original_factory_backstage_duplicate_ids_and_suspension() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let m = RibbonBackstageModel::new();
        let a = page("same");
        let z = page("same");
        m.items().add(Some(a.clone()));
        m.items().add(Some(z.clone()));
        m.set_selected_item(Some(z));
        let b = f.create_backstage(Some(m.clone()), Some(scope.clone()));
        let second = b.selected_item() == b.items().get(1);
        m.set_selected_item(Some(a.clone()));
        let first = b.selected_item() == b.items().get(0);
        scope.suspend();
        b.set_selected_item(b.items().get(1));
        let ignored = same(&m.selected_item(), &a);
        m.items().move_item(1, 0);
        m.set_title(Some("Paused".into()));
        let during = b.title();
        scope.resume();
        let selected = b.selected_item() == b.items().get(1);
        let title = b.title();
        scope.dispose();
        check(
            "backstage_duplicate_ids",
            json!({"second":second,"first":first,"ignored":ignored,"during":during,"selected":selected,"title":title}),
        );
    });
}
#[test]
fn original_factory_backstage_payload_identity_and_content_template_assignment_order() {
    crate::group_tests::run(|| {
        ValueTypes::register_reference::<Payload>();
        ValueTypes::register_display::<Payload>();
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let selector = Selector::new();
        f.set_content_template_selector(Some(selector.clone()));
        let value = Rc::new(Payload {
            value: Cell::new(3),
        });
        let m = RibbonBackstageItemModel::new(Some("data".into()), Some("Data".into()));
        m.set_content(Some(RibbonObject::reference(value.clone())));
        m.set_command_parameter(Some(RibbonObject::reference(value.clone())));
        let e = f.create_backstage_item(Some(m.clone()), Some(scope.clone()));
        let original: Rc<dyn Any> = value.clone();
        let content: Rc<dyn Any> = e.content().unwrap();
        let parameter: Rc<dyn Any> = e.command_parameter().unwrap();
        let same = Rc::ptr_eq(&original, &content) && Rc::ptr_eq(&original, &parameter);
        let initial = e
            .content_template()
            .is_some_and(|t| Rc::ptr_eq(&t, &selector.template));
        let trace = Rc::new(RefCell::new(vec![]));
        let out = trace.clone();
        e.add_property_changed_handler(Rc::new(move |a| {
            if a.is_effective_value_change() {
                out.borrow_mut().push(a.property().name().to_owned());
            }
        }));
        let control = TextBlock::new();
        m.set_content(Some(RibbonObject::value(
            control.clone().upcast::<Control>(),
        )));
        let to_control = std::mem::take(&mut *trace.borrow_mut());
        let control_same = ValueTypes::as_object(&*e.content().unwrap())
            .unwrap()
            .ptr_eq(&control);
        m.set_content(Some(RibbonObject::reference(value)));
        let to_data = std::mem::take(&mut *trace.borrow_mut());
        m.set_content(None);
        let to_null = std::mem::take(&mut *trace.borrow_mut());
        scope.dispose();
        check(
            "backstage_payload",
            json!({"same":same,"initial":initial,"toControl":to_control,"controlSame":control_same,"toData":to_data,"toNull":to_null,"calls":*selector.calls.borrow(),"isPage":e.is_page(),"associated":Rc::ptr_eq(&RibbonElementFactory::get_model(&e).unwrap(),&(m as Rc<dyn RibbonNodeModelImpl>))}),
        );
    });
}
#[test]
fn original_factory_backstage_selector_replacement_waits_for_model_change() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let first = Selector::new();
        let second = Selector::new();
        let m = page("data");
        m.set_label(Some("Data".into()));
        m.set_content(Some(RibbonObject::value("body".to_owned())));
        f.set_content_template_selector(Some(first.clone()));
        let e = f.create_backstage_item(Some(m.clone()), Some(scope.clone()));
        f.set_content_template_selector(Some(second.clone()));
        let unchanged = e
            .content_template()
            .is_some_and(|t| Rc::ptr_eq(&t, &first.template));
        m.set_label(Some("Changed".into()));
        let changed = e
            .content_template()
            .is_some_and(|t| Rc::ptr_eq(&t, &second.template));
        scope.dispose();
        m.set_label(Some("Disposed".into()));
        check(
            "backstage_selector_replacement",
            json!({"unchanged":unchanged,"changed":changed,"first":*first.calls.borrow(),"second":*second.calls.borrow(),"header":e.header()}),
        );
    });
}
#[test]
fn original_factory_backstage_nested_screentip_replacement_and_disposal() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let tip = ferroribbon_core::model::RibbonScreenTip::new();
        tip.set_title(Some("old".into()));
        let m = page("tip");
        m.set_label(Some("Label".into()));
        m.set_description(Some("fallback".into()));
        m.set_screen_tip(Some(tip.clone()));
        let e = f.create_backstage_item(Some(m.clone()), Some(scope.clone()));
        let title = || {
            e.screen_tip().and_then(|v| {
                v.downcast_ref::<String>().cloned().or_else(|| {
                    ValueTypes::as_object(&*v)
                        .and_then(|o| o.cast::<crate::controls::RibbonScreenTip>())
                        .and_then(|t| t.title())
                })
            })
        };
        let initial = title();
        tip.set_title(Some("changed".into()));
        let inner = title();
        m.set_screen_tip(None);
        let cleared = title();
        tip.set_title(Some("stale".into()));
        scope.dispose();
        m.set_label(Some("Disposed".into()));
        check(
            "backstage_tip",
            json!({"initial":initial,"inner":inner,"cleared":cleared,"after":title(),"header":e.header()}),
        );
    });
}
#[test]
fn original_factory_backstage_catalog_resolution_direct_command_and_parameter_identity() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let calls = Rc::new(RefCell::new(vec![]));
        let make = |name: &'static str| {
            let calls = calls.clone();
            RibbonRelayCommand::new_with_parameter(
                move |p| {
                    calls.borrow_mut().push(format!(
                        "{name}:{}",
                        p.unwrap().downcast_ref::<String>().unwrap()
                    ))
                },
                None,
            )
        };
        let first = make("first");
        let replacement = make("replacement");
        let direct = make("direct");
        let catalog = RibbonCommandCatalog::new();
        let descriptor = RibbonCommandDescriptor::new(
            Some("save".into()),
            Some("Save".into()),
            Some(first.clone()),
        );
        catalog.register(Some(descriptor.clone()));
        let root = Ribbon::new();
        root.set_command_catalog(Some(catalog));
        f.set_ribbon(Some(root));
        let m = RibbonBackstageItemModel::new(Some("save".into()), Some("Save".into()));
        m.set_command_id(Some("SAVE".into()));
        m.set_command_parameter(Some(RibbonObject::value("p".to_owned())));
        let e = f.create_backstage_item(Some(m.clone()), Some(scope.clone()));
        let execute = || e.command().unwrap().execute(e.command_parameter().as_ref());
        execute();
        descriptor.set_command(Some(replacement.clone()));
        let still = Rc::ptr_eq(
            &crate::commands::source_core_command(&e.command().unwrap()).unwrap(),
            &(first as Rc<dyn ICommand>),
        );
        f.invalidate_commands();
        execute();
        m.set_command(Some(direct));
        f.invalidate_commands();
        execute();
        m.set_command_parameter(Some(RibbonObject::value("q".to_owned())));
        let parameter = e
            .command_parameter()
            .unwrap()
            .downcast_ref::<String>()
            .cloned();
        m.set_command(None);
        execute();
        scope.dispose();
        descriptor.set_command(Some(make("first")));
        f.invalidate_commands();
        let stopped = Rc::ptr_eq(
            &crate::commands::source_core_command(&e.command().unwrap()).unwrap(),
            &(replacement as Rc<dyn ICommand>),
        );
        check(
            "backstage_commands",
            json!({"calls":*calls.borrow(),"still":still,"parameter":parameter,"stopped":stopped}),
        );
    });
}
#[test]
fn generated_backstage_command_receives_core_only_model_parameter() {
    crate::group_tests::run(|| {
        struct SaveRequest(u32);

        let factory = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let model = RibbonBackstageItemModel::new(Some("save".into()), Some("Save".into()));
        let request = Rc::new(SaveRequest(7));
        let expected = request.clone();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        model.set_command_parameter(Some(RibbonObject::reference(request)));
        model.set_command(Some(RibbonRelayCommand::new_with_parameter(
            move |value| {
                let request = value.unwrap().downcast_ref::<SaveRequest>().unwrap();
                assert!(std::ptr::eq(request, expected.as_ref()));
                assert_eq!(request.0, 7);
                observed.set(observed.get() + 1);
            },
            None,
        )));
        let item = factory.create_backstage_item(Some(model), Some(scope.clone()));
        let backstage = RibbonBackstage::new();
        backstage.items().add(Some(item.clone()));
        backstage.invoke(Some(item));
        assert_eq!(calls.get(), 1);
        scope.dispose();
    });
}

#[test]
fn original_template_selector_default_overloads_dispatch_independently_and_guard_container() {
    crate::group_tests::run(|| {
        ValueTypes::register_reference::<Payload>();
        ValueTypes::register_display::<Payload>();
        let plain = DataTemplateSelector::new();
        let derived = Selector::new();
        let value = ValueTypes::box_erased(Rc::new(Payload {
            value: Cell::new(7),
        }))
        .unwrap();
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            plain.select_template_with_container(None, None)
        }));
        assert!(failure.is_err());
        let one = derived.select_template(Some(value.clone()));
        let two = derived.select_template_with_container(Some(value), Some(FerroObject::new()));
        check(
            "selector_dispatch",
            json!({"plain":plain.select_template(None).is_none(),"error":"container","one":one.is_some_and(|t|Rc::ptr_eq(&t,&derived.template)),"two":two.is_none(),"calls":*derived.calls.borrow()}),
        );
    });
}
#[test]
fn original_factory_backstage_initial_selector_reentry_preserves_captured_content() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let selector = Selector::new();
        let m = page("data");
        m.set_label(Some("Before".into()));
        m.set_content(Some(RibbonObject::value("initial".to_owned())));
        let model = m.clone();
        selector.callback.replace(Some(Rc::new(move || {
            model.set_content(Some(RibbonObject::value("from-selector".to_owned())))
        })));
        f.set_content_template_selector(Some(selector.clone()));
        let e = f.create_backstage_item(Some(m.clone()), Some(scope.clone()));
        let initial = e.content().unwrap().downcast_ref::<String>().cloned();
        let model = m.content().unwrap().downcast_ref::<String>().cloned();
        selector.callback.take();
        m.set_label(Some("After".into()));
        scope.dispose();
        check(
            "backstage_selector_reentry",
            json!({"initial":initial,"model":model,"after":e.content().unwrap().downcast_ref::<String>(),"calls":*selector.calls.borrow()}),
        );
    });
}

#[test]
fn original_factory_backstage_null_guards_empty_model_and_disposed_scope() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let mut errors = vec![];
        let mut attempt = |call: &dyn Fn()| {
            let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).unwrap_err();
            let s = error
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| error.downcast_ref::<String>().map(String::as_str))
                .unwrap();
            errors.push(if s.ends_with("model") {
                "model"
            } else if s.ends_with("bindings") {
                "bindings"
            } else {
                panic!("unexpected guard: {s}")
            });
        };
        attempt(&|| {
            f.create_backstage(None, Some(scope.clone()));
        });
        attempt(&|| {
            f.create_backstage(Some(RibbonBackstageModel::new()), None);
        });
        attempt(&|| {
            f.create_backstage_item(None, Some(scope.clone()));
        });
        attempt(&|| {
            f.create_backstage_item(Some(page("page")), None);
        });
        attempt(&|| {
            f.create_backstage(None, None);
        });
        let m = RibbonBackstageModel::new();
        m.set_title(Some("Empty".into()));
        m.set_selected_item(Some(page("external")));
        let b = f.create_backstage(Some(m.clone()), Some(scope.clone()));
        let empty = json!({"title":b.title(),"count":b.items().count(),"selected":m.selected_item().and_then(|i|i.id()).map(|s|s.to_utf8().unwrap())});
        scope.dispose();
        let disposed = RibbonModelBindings::new();
        disposed.dispose();
        let model = page("page");
        let item = f.create_backstage_item(Some(model.clone()), Some(disposed.clone()));
        let model_root = RibbonBackstageModel::new();
        model_root.set_title(Some("unused".into()));
        let root = f.create_backstage(Some(model_root), Some(disposed));
        check(
            "backstage_guards",
            json!({"errors":errors,"empty":empty,"disposed":{"header":item.header(),"isPage":item.is_page(),"associated":Rc::ptr_eq(&RibbonElementFactory::get_model(&item).unwrap(),&(model as Rc<dyn RibbonNodeModelImpl>)),"title":root.title(),"count":root.items().count()}}),
        );
    });
}

#[test]
fn native_factory_erased_payload_requires_explicit_registration_without_loss_or_proxy() {
    crate::group_tests::run(|| {
        struct Unregistered(Cell<i32>);
        impl PartialEq for Unregistered {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        }
        let f = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let value = Rc::new(Unregistered(Cell::new(2)));
        let model = page("custom");
        model.set_content(Some(RibbonObject::reference(value.clone())));
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f.create_backstage_item(Some(model.clone()), Some(scope.clone()))
        }));
        let error =
            failure.expect_err("unknown native capability must not silently fabricate a value");
        let error = error.downcast_ref::<String>().unwrap();
        assert!(error.contains("Unregistered native model payload"));
        ValueTypes::register_erased_value::<Unregistered>();
        let e = f.create_backstage_item(Some(model), Some(scope.clone()));
        let boxed = e.content().unwrap();
        boxed.downcast_ref::<Unregistered>().unwrap().0.set(7);
        assert_eq!(value.0.get(), 7);
        let original: Rc<dyn Any> = value;
        let recovered: Rc<dyn Any> = boxed;
        assert!(Rc::ptr_eq(&original, &recovered));
        scope.dispose();
    });
}
