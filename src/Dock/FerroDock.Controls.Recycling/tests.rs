// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::*;
use ferrodock_controls_recycling_model::IControlRecyclingIdProvider;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{ClrPropertyInfo, Untyped};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::{BindingOperations, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IDataTemplate,
    IRecyclingDataTemplate, ITemplateWithParam,
};
use ferroui_controls::{
    AppBuilder, Application, Border, ContentControl, ContentControlImpl, Control, ControlImpl,
    StackPanel, TextBlock, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-recycling-headless",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct ReferenceObject;
impl PartialEq for ReferenceObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
fn object() -> BoxedValue {
    Rc::new(ReferenceObject)
}
fn control(value: BoxedValue) -> Ref<Control> {
    Control::from_boxed(&value).unwrap()
}
fn update(window: &Window) {
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
}
fn host(child: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(child)));
    window.show();
    update(&window);
    window
}
fn parent() -> Ref<Control> {
    let p = Control::new();
    p.data_templates().add(FuncDataTemplate::new(
        |_| true,
        |_, _| Some(Border::new().upcast()),
        true,
    ));
    p
}

#[repr(C)]
struct IdData {
    base: FerroObject,
    id: Option<String>,
    value: String,
}
ferro_class!(IdData:FerroObject);
ferro_impl_classes!(IdData:FerroObjectImpl);
ferro_class_info!(IdData {interfaces:[Rc<dyn IControlRecyclingIdProvider> => IdData::as_id_provider]});
impl IdData {
    fn new(id: Option<&str>, value: &str) -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            id: id.map(str::to_owned),
            value: value.into(),
        })
    }
    fn as_id_provider(this: Ref<Self>) -> Rc<dyn IControlRecyclingIdProvider> {
        Rc::new(IdHandle(this))
    }
}
struct IdHandle(Ref<IdData>);
impl IControlRecyclingIdProvider for IdHandle {
    fn get_control_recycling_id(&self) -> Option<String> {
        self.0.id.clone()
    }
}
struct TrackingTemplate {
    calls: Cell<usize>,
}
impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for TrackingTemplate {
    fn build(&self, data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(data.as_ref(), None)
    }
}
impl IDataTemplate for TrackingTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        data.is_some_and(|d| d.is::<Ref<IdData>>())
    }
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}
impl IRecyclingDataTemplate for TrackingTemplate {
    fn build_with_existing(
        &self,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        self.calls.set(self.calls.get() + 1);
        let control = existing.unwrap_or_else(|| TextBlock::new().upcast());
        control.set_tag(
            data.and_then(|d| d.downcast_ref::<Ref<IdData>>())
                .map(|d| Rc::new(d.value.clone()) as BoxedValue),
        );
        Some(control)
    }
}
fn tag(control: &Control) -> String {
    control
        .tag()
        .unwrap()
        .downcast_ref::<String>()
        .unwrap()
        .clone()
}

#[repr(C)]
struct ManualPresenterHost {
    base: ContentControl,
}
ferro_class!(ManualPresenterHost:ContentControl);
ferro_impl_classes!(ManualPresenterHost:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,
    InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl ManualPresenterHost {
    fn static_constructor() {
        TemplatedControl::template_property().override_default_value::<Self>(Some(
            FuncControlTemplate::new(|_, scope| {
                let presenter = ContentPresenter::new();
                presenter.set_name(Some("PART_ContentPresenter".into()));
                presenter.register_in_name_scope(&**scope).upcast()
            }),
        ));
    }
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: ContentControl::construct(),
        })
    }
}

#[test]
fn constructor_defaults() {
    run(|| {
        assert!(!ControlRecycling::new().try_to_use_id_as_key());
        assert!(ControlRecyclingDataTemplate::new().parent().is_none());
    });
}
#[test]
fn add_and_try_get_value_work() {
    run(|| {
        let cache = ControlRecycling::new();
        let data = object();
        let c = TextBlock::new();
        cache.add(data.clone(), Control::boxed(c.clone()));
        assert_eq!(
            control(cache.try_get_value(Some(&data)).unwrap()),
            c.upcast::<Control>()
        );
    });
}
#[test]
fn null_lookup_build_and_clear_follow_source_guards() {
    run(|| {
        let cache = ControlRecycling::new();
        assert!(cache.try_get_value(None).is_none());
        assert!(cache.build(None, None, None).is_none());
        let data = object();
        cache.add(data.clone(), Control::boxed(TextBlock::new()));
        cache.clear();
        assert!(cache.try_get_value(Some(&data)).is_none());
    });
}
#[test]
fn build_uses_data_template_and_caches_result() {
    run(|| {
        let cache = ControlRecycling::new();
        let parent = Control::boxed(parent());
        let data: BoxedValue = Rc::new(10_i32);
        let a = control(cache.build(Some(&data), None, Some(&parent)).unwrap());
        let b = control(
            cache
                .build(Some(&(Rc::new(10_i32) as BoxedValue)), None, Some(&parent))
                .unwrap(),
        );
        assert_eq!(a, b);
    });
}
#[test]
fn build_updates_cached_control_for_recycling_template_and_separates_different_keys() {
    run(|| {
        let cache = ControlRecycling::new();
        cache.set_try_to_use_id_as_key(true);
        let parent = Control::new();
        let template = Rc::new(TrackingTemplate {
            calls: Cell::new(0),
        });
        parent.data_templates().add(template.clone());
        let parent = Control::boxed(parent);
        let a: BoxedValue = Rc::new(IdData::new(Some("a"), "first"));
        let b: BoxedValue = Rc::new(IdData::new(Some("a"), "second"));
        let first = control(cache.build(Some(&a), None, Some(&parent)).unwrap());
        let second = control(cache.build(Some(&b), None, Some(&parent)).unwrap());
        assert_eq!(first, second);
        assert_eq!(tag(&first), "second");
        assert_eq!(template.calls.get(), 2);
        let c: BoxedValue = Rc::new(IdData::new(Some("b"), "third"));
        let first_box = Control::boxed(first.clone());
        let third = control(
            cache
                .build(Some(&c), Some(&first_box), Some(&parent))
                .unwrap(),
        );
        assert_ne!(first, third);
        assert_eq!(tag(&first), "second");
        assert_eq!(tag(&third), "third");
        assert_eq!(
            control(
                cache
                    .try_get_value(Some(&(Rc::new("a".to_owned()) as BoxedValue)))
                    .unwrap()
            ),
            first
        );
        assert_eq!(
            control(
                cache
                    .try_get_value(Some(&(Rc::new("b".to_owned()) as BoxedValue)))
                    .unwrap()
            ),
            third
        );
    });
}
#[test]
fn identifier_is_used_only_when_enabled_and_nonblank() {
    run(|| {
        for (enabled, id, same) in [
            (true, Some("a"), true),
            (false, Some("a"), false),
            (true, None, false),
            (true, Some(""), false),
            (true, Some(" \t\n\u{0085}\u{2003}"), false),
            (true, Some(" a "), true),
        ] {
            let cache = ControlRecycling::new();
            cache.set_try_to_use_id_as_key(enabled);
            let parent = Control::boxed(parent());
            let a: BoxedValue = Rc::new(IdData::new(id, "first"));
            let b: BoxedValue = Rc::new(IdData::new(id, "second"));
            let first = control(cache.build(Some(&a), None, Some(&parent)).unwrap());
            let second = control(cache.build(Some(&b), None, Some(&parent)).unwrap());
            assert_eq!(first == second, same);
        }
    });
}
#[test]
fn cached_control_is_removed_from_panel_and_reused_when_unparented() {
    run(|| {
        let cache = ControlRecycling::new();
        let data = object();
        let c = TextBlock::new();
        let panel = StackPanel::new();
        panel.children().add(c.clone());
        cache.add(data.clone(), Control::boxed(c.clone()));
        let a = control(cache.build(Some(&data), None, None).unwrap());
        assert_eq!(a, c.clone().upcast::<Control>());
        assert_eq!(panel.children().count(), 0);
        let b = control(cache.build(Some(&data), None, None).unwrap());
        assert_eq!(a, b);
        assert!(c.visual_parent().is_none());
    });
}
#[test]
fn cache_and_template_detach_from_real_content_presenter() {
    run(|| {
        for use_cache in [false, true] {
            let c = TextBlock::new();
            let presenter = ContentPresenter::new();
            presenter.set_content(Some(Control::boxed(c.clone())));
            let window = host(presenter.clone().upcast());
            let result = if use_cache {
                let cache = ControlRecycling::new();
                let data = object();
                cache.add(data.clone(), Control::boxed(c.clone()));
                control(cache.build(Some(&data), None, None).unwrap())
            } else {
                ControlRecyclingDataTemplate::new()
                    .build(Some(&Control::boxed(c.clone())))
                    .unwrap()
            };
            assert_eq!(result, c.clone().upcast::<Control>());
            assert!(presenter.content().is_none());
            assert!(c.visual_parent().is_none());
            window.close();
        }
    });
}
#[test]
fn cache_and_template_prefer_visual_presenter_over_different_host_content() {
    run(|| {
        for use_cache in [false, true] {
            let c = TextBlock::new();
            let host_control = ManualPresenterHost::new();
            let host_content = object();
            host_control.set_content(Some(host_content.clone()));
            let window = host(host_control.clone().upcast());
            host_control.apply_template();
            update(&window);
            let presenter = host_control.presenter().unwrap();
            presenter.set_content(Some(Control::boxed(c.clone())));
            update(&window);
            assert_eq!(presenter.child(), Some(c.clone().upcast::<Control>()));
            let result = if use_cache {
                let cache = ControlRecycling::new();
                let data = object();
                cache.add(data.clone(), Control::boxed(c.clone()));
                control(cache.build(Some(&data), None, None).unwrap())
            } else {
                ControlRecyclingDataTemplate::new()
                    .build(Some(&Control::boxed(c.clone())))
                    .unwrap()
            };
            assert_eq!(result, c.clone().upcast::<Control>());
            assert!(c.visual_parent().is_none());
            assert!(Rc::ptr_eq(&host_control.content().unwrap(), &host_content));
            window.close();
        }
    });
}
#[test]
fn template_no_parent_guard_parent_cache_and_template_fallback() {
    run(|| {
        let template = ControlRecyclingDataTemplate::new();
        assert!(template.build(Some(&object())).is_none());
        assert!(template.match_(None));
        assert!(template.match_(Some(&object())));
        let parent = parent();
        let cache = ControlRecycling::new();
        ControlRecyclingDataTemplate::set_control_recycling(
            &parent,
            Some(ControlRecycling::as_recycling(cache.clone())),
        );
        template.set_parent(Some(parent.clone()));
        let data: BoxedValue = Rc::new("test".to_owned());
        let c = template.build_with_existing(Some(&data), None).unwrap();
        assert_eq!(control(cache.try_get_value(Some(&data)).unwrap()), c);
        ControlRecyclingDataTemplate::set_control_recycling(&parent, None);
        assert!(template
            .build(Some(&(Rc::new(5_i32) as BoxedValue)))
            .is_some());
    });
}

#[derive(Default)]
struct TestViewModel {
    item: RefCell<Option<BoxedValue>>,
    changed: Event<str>,
}
impl PartialEq for TestViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl TestViewModel {
    fn set_item(&self, value: Option<BoxedValue>) {
        if ferroui_controls::reference_equals(&self.item.borrow(), &value) {
            return;
        }
        self.item.replace(value);
        self.changed.raise("Item");
    }
}
impl INotifyPropertyChanged for TestViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
#[test]
fn moving_cached_content_preserves_native_compiled_binding_on_original_presenter() {
    run(|| {
        let data = object();
        let updated = object();
        let vm = Rc::new(TestViewModel::default());
        vm.set_item(Some(data.clone()));
        let cache = ControlRecycling::new();
        let data_template = FuncDataTemplate::new(
            |_| true,
            |data, _| {
                let b = Border::new();
                b.set_tag(data.clone());
                Some(b.upcast())
            },
            true,
        );
        let a = ContentPresenter::new();
        a.data_templates().add(data_template.clone());
        let binding = CompiledBinding::new(
            CompiledBindingPathBuilder::new()
                .property(
                    Rc::new(ClrPropertyInfo::read_write::<TestViewModel, Untyped>(
                        "Item",
                        |vm| vm.item.borrow().clone(),
                        TestViewModel::set_item,
                    )),
                    PropertyInfoAccessorFactory::create_inpc_property_accessor::<TestViewModel>(),
                )
                .build(),
        )
        .with_source(Some(vm.clone()));
        a.bind_binding(
            ContentPresenter::content_property().as_property(),
            &*binding,
        );
        let ta = ControlRecyclingDataTemplate::new();
        ta.set_parent(Some(a.clone().upcast()));
        a.set_content_template(Some(ControlRecyclingDataTemplate::as_data_template(ta)));
        ControlRecyclingDataTemplate::set_control_recycling(
            &a,
            Some(ControlRecycling::as_recycling(cache.clone())),
        );
        let b = ContentPresenter::new();
        b.data_templates().add(data_template);
        let tb = ControlRecyclingDataTemplate::new();
        tb.set_parent(Some(b.clone().upcast()));
        b.set_content_template(Some(ControlRecyclingDataTemplate::as_data_template(tb)));
        ControlRecyclingDataTemplate::set_control_recycling(
            &b,
            Some(ControlRecycling::as_recycling(cache)),
        );
        let panel = StackPanel::new();
        panel.children().add(a.clone());
        panel.children().add(b.clone());
        let window = host(panel.upcast());
        let cached = a.child().unwrap();
        b.set_content(Some(data));
        update(&window);
        assert_eq!(b.child(), Some(cached));
        assert!(a.child().is_none());
        assert!(BindingOperations::get_binding_expression_base(
            &a,
            ContentPresenter::content_property().as_property()
        )
        .is_some());
        vm.set_item(Some(updated.clone()));
        update(&window);
        assert!(Rc::ptr_eq(&a.content().unwrap(), &updated));
        assert!(a.child().is_some());
        window.close();
    });
}

struct ScriptedTemplate {
    recycling: bool,
    results: RefCell<std::collections::VecDeque<Option<Ref<Control>>>>,
    calls: RefCell<Vec<Option<Ref<Control>>>>,
}
impl ScriptedTemplate {
    fn new(recycling: bool, results: impl IntoIterator<Item = Option<Ref<Control>>>) -> Rc<Self> {
        Rc::new(Self {
            recycling,
            results: RefCell::new(results.into_iter().collect()),
            calls: RefCell::new(Vec::new()),
        })
    }
}
impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ScriptedTemplate {
    fn build(&self, _: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(None, None)
    }
}
impl IDataTemplate for ScriptedTemplate {
    fn match_(&self, _: Option<&BoxedValue>) -> bool {
        true
    }
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        if self.recycling {
            Some(self)
        } else {
            None
        }
    }
}
impl IRecyclingDataTemplate for ScriptedTemplate {
    fn build_with_existing(
        &self,
        _: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        self.calls.borrow_mut().push(existing);
        self.results
            .borrow_mut()
            .pop_front()
            .expect("script contains every expected callback")
    }
}
fn unremovable(child: Ref<Control>) -> Ref<Control> {
    let host = Control::new();
    host.visual_children().add(child.upcast());
    host
}
#[test]
fn failed_detach_uses_source_fallback_order_and_caches_successful_rebuild() {
    run(|| {
        for recycling in [false, true] {
            for cached in [false, true] {
                let cache = ControlRecycling::new();
                let data = object();
                let stuck = Border::new().upcast::<Control>();
                let _host = unremovable(stuck.clone());
                let replacement = Border::new().upcast::<Control>();
                let mut results = vec![Some(stuck.clone()), Some(replacement.clone())];
                if !cached || recycling {
                    results.insert(0, Some(stuck.clone()));
                }
                let template = ScriptedTemplate::new(recycling, results);
                let parent = Control::new();
                parent.data_templates().add(template.clone());
                let parent = Control::boxed(parent);
                if cached {
                    cache.add(data.clone(), Control::boxed(stuck.clone()));
                }
                let actual = control(cache.build(Some(&data), None, Some(&parent)).unwrap());
                assert_eq!(actual, replacement);
                assert_eq!(
                    control(cache.try_get_value(Some(&data)).unwrap()),
                    replacement
                );
                // A non-recycling cache hit calls no initial template before fallback.
                let expected = if cached && !recycling { 2 } else { 3 };
                assert_eq!(template.calls.borrow().len(), expected);
                if recycling && cached {
                    assert_eq!(template.calls.borrow()[0], Some(stuck));
                }
                assert!(template.calls.borrow().last().unwrap().is_none());
            }
        }
    });
}
#[test]
fn failed_fallback_retains_prior_cache_and_never_reparents_unsupported_host() {
    run(|| {
        let cache = ControlRecycling::new();
        let data = object();
        let stuck = Border::new().upcast::<Control>();
        let host = unremovable(stuck.clone());
        cache.add(data.clone(), Control::boxed(stuck.clone()));
        let template = ScriptedTemplate::new(true, [Some(stuck.clone()), None, None]);
        let parent = Control::new();
        parent.data_templates().add(template.clone());
        assert!(cache
            .build(Some(&data), None, Some(&Control::boxed(parent)))
            .is_none());
        assert_eq!(control(cache.try_get_value(Some(&data)).unwrap()), stuck);
        assert_eq!(host.visual_children().count(), 1);
        assert_eq!(template.calls.borrow().len(), 3);
    });
}
#[test]
fn cached_template_replacement_is_stored_before_detachment_and_null_update_keeps_old_control() {
    run(|| {
        for replaces in [false, true] {
            let cache = ControlRecycling::new();
            let data = object();
            let old = Border::new().upcast::<Control>();
            let new = Border::new().upcast::<Control>();
            let template =
                ScriptedTemplate::new(true, [if replaces { Some(new.clone()) } else { None }]);
            let parent = Control::new();
            parent.data_templates().add(template.clone());
            cache.add(data.clone(), Control::boxed(old.clone()));
            let actual = control(
                cache
                    .build(Some(&data), None, Some(&Control::boxed(parent)))
                    .unwrap(),
            );
            let expected = if replaces { new } else { old.clone() };
            assert_eq!(actual, expected);
            assert_eq!(control(cache.try_get_value(Some(&data)).unwrap()), expected);
            assert_eq!(template.calls.borrow()[0], Some(old));
        }
    });
}
#[test]
fn recycling_fallback_tries_existing_then_null_and_accepts_existing_without_detaching() {
    run(|| {
        let cache = ControlRecycling::new();
        let data = object();
        let stuck = Border::new().upcast::<Control>();
        let _host = unremovable(stuck.clone());
        let existing = Border::new().upcast::<Control>();
        let existing_host = unremovable(existing.clone());
        let template = ScriptedTemplate::new(true, [Some(stuck), None, Some(existing.clone())]);
        let parent = Control::new();
        parent.data_templates().add(template.clone());
        let actual = control(
            cache
                .build(
                    Some(&data),
                    Some(&Control::boxed(existing.clone())),
                    Some(&Control::boxed(parent)),
                )
                .unwrap(),
        );
        assert_eq!(actual, existing);
        assert_eq!(existing_host.visual_children().count(), 1);
        assert_eq!(&*template.calls.borrow(), &[None, Some(existing), None]);
    });
}
#[test]
fn data_template_data_dispatch_rebuild_and_self_reference_guards_use_native_contracts() {
    run(|| {
        for recycling in [false, true] {
            let stuck = Border::new().upcast::<Control>();
            let _host = unremovable(stuck.clone());
            let existing = Border::new().upcast::<Control>();
            let _existing_host = unremovable(existing.clone());
            let scripted = ScriptedTemplate::new(recycling, [Some(stuck), Some(existing.clone())]);
            let parent = Control::new();
            parent.data_templates().add(scripted.clone());
            let wrapper = ControlRecyclingDataTemplate::new();
            wrapper.set_parent(Some(parent));
            let data = if recycling {
                Rc::new(scripted.clone() as Rc<dyn IRecyclingDataTemplate>) as BoxedValue
            } else {
                object()
            };
            assert_eq!(
                wrapper.build_with_existing(Some(&data), Some(existing.clone())),
                Some(existing.clone())
            );
            assert_eq!(
                &*scripted.calls.borrow(),
                &[if recycling { Some(existing) } else { None }, None]
            );
        }
        let wrapper = ControlRecyclingDataTemplate::new();
        let own: BoxedValue = Rc::new(wrapper.clone());
        assert!(wrapper.build(Some(&own)).is_none());
    });
}
#[test]
fn native_metadata_inheritance_direct_notifications_contract_identity_and_release() {
    run(|| {
        use ferroui_base::data::core::{ValueType, ValueTypes};
        let cache = ControlRecycling::new();
        let changes = Rc::new(Cell::new(0));
        let contract = ferroui_base::metadata::MarkupType::find_by_handle(std::any::TypeId::of::<
            Rc<dyn crate::IControlRecycling>,
        >())
        .unwrap();
        assert_eq!(contract.name, "IControlRecycling");
        assert_eq!(
            contract.kind,
            ferroui_base::metadata::MarkupTypeKind::Interface
        );
        let seen = changes.clone();
        let sub = cache.property_changed(move |e| {
            if e.property() == ControlRecycling::try_to_use_id_as_key_property().as_property() {
                seen.set(seen.get() + 1);
            }
        });
        cache.set_value_untyped(
            ControlRecycling::try_to_use_id_as_key_property().as_property(),
            &true,
            ferroui_base::data::BindingPriority::LocalValue,
        );
        cache.set_try_to_use_id_as_key(true);
        cache.set_try_to_use_id_as_key(false);
        assert_eq!(changes.get(), 2);
        sub.dispose();
        let a = ControlRecycling::as_recycling(cache.clone());
        let b = ControlRecycling::as_recycling(cache.clone());
        assert!(Rc::ptr_eq(&a, &b));
        let value: BoxedValue = Rc::new(a.clone());
        assert!(ValueTypes::as_object(&*value).unwrap().ptr_eq(&cache));
        assert!(ValueTypes::try_cast(
            &(Rc::new(cache.clone()) as BoxedValue),
            ValueType::of::<Rc<dyn ferrodock_controls_recycling_model::IControlRecycling>>()
        )
        .is_some());
        let parent = StackPanel::new();
        let child = Border::new();
        parent.children().add(child.clone());
        ControlRecyclingDataTemplate::set_control_recycling(&parent, Some(a));
        assert!(Rc::ptr_eq(
            &ControlRecyclingDataTemplate::get_control_recycling(&child).unwrap(),
            &b
        ));
        let data = object();
        let c = Border::new();
        let weak = c.downgrade();
        cache.add(data.clone(), Control::boxed(c));
        assert!(weak.upgrade().is_some());
        cache.clear();
        assert!(weak.upgrade().is_none());
        assert!(!cache.remove(None));
        let raw: Rc<dyn std::any::Any> = Rc::new(ReferenceObject);
        let raw_control: Rc<dyn std::any::Any> = Rc::new(ReferenceObject);
        b.add(raw.clone(), raw_control.clone());
        let got = b.try_get_value(Some(raw.clone())).unwrap();
        assert!(Rc::ptr_eq(&got, &raw_control));
        assert!(b.build(None, None, None).is_none());
        assert!(cache.remove(Some(&from_recycling_value(&raw))));
    });
}
#[test]
fn scalar_and_custom_key_equality_uses_shared_value_comparison_and_remove_uses_id() {
    run(|| {
        #[derive(PartialEq)]
        struct CustomValue(i32);
        let cache = ControlRecycling::new();
        let c = Control::boxed(Border::new());
        cache.add(Rc::new(CustomValue(1)), c.clone());
        assert!(cache
            .try_get_value(Some(&(Rc::new(CustomValue(1)) as BoxedValue)))
            .is_some());
        for (a, b) in [(f64::NAN, f64::NAN), (-0.0, 0.0)] {
            cache.add(Rc::new(a), c.clone());
            assert!(cache
                .try_get_value(Some(&(Rc::new(b) as BoxedValue)))
                .is_some());
        }
        let panel = Border::new();
        cache.add(Control::boxed(panel.clone()), c);
        assert!(cache
            .try_get_value(Some(&(Rc::new(panel) as BoxedValue)))
            .is_some());
        cache.set_try_to_use_id_as_key(true);
        let data: BoxedValue = Rc::new(IdData::new(Some("key"), "value"));
        let parent = Control::boxed(parent());
        cache.build(Some(&data), None, Some(&parent)).unwrap();
        assert!(cache.try_get_value(Some(&data)).is_none());
        assert!(cache.remove(Some(&data)));
        assert!(cache
            .try_get_value(Some(&(Rc::new("key".to_owned()) as BoxedValue)))
            .is_none());
    });
}

#[test]
fn domain_contract_projects_raw_derived_native_handles_through_shared_object_metadata() {
    run(|| {
        let cache = ControlRecycling::new();
        let handle = ControlRecycling::as_recycling(cache.clone());
        let parent = Control::new();
        parent
            .data_templates()
            .add(FuncDataTemplate::for_type::<Ref<IdData>>(
                |_, _| Some(Border::new().upcast()),
                true,
            ));
        let parent: Rc<dyn std::any::Any> = Rc::new(parent);
        let first = IdData::new(Some("a"), "first");
        let second = IdData::new(Some("a"), "second");
        handle.set_try_to_use_id_as_key(true);
        let a = handle
            .build(Some(Rc::new(first)), None, Some(parent.clone()))
            .unwrap();
        let b = handle
            .build(Some(Rc::new(second)), None, Some(parent))
            .unwrap();
        assert_eq!(
            control(from_recycling_value(&a)),
            control(from_recycling_value(&b))
        );
        let data: Rc<dyn std::any::Any> = Rc::new(ReferenceObject);
        let border = Border::new();
        let raw: Rc<dyn std::any::Any> = Rc::new(border.clone());
        handle.add(data.clone(), raw);
        let built = handle.build(Some(data), None, None).unwrap();
        assert_eq!(
            control(from_recycling_value(&built)),
            border.upcast::<Control>()
        );
    });
}

#[test]
fn registered_raw_values_preserve_actual_type_identity_and_do_not_root_instances() {
    use crate::{from_recycling_value, register_value_type, to_recycling_value};
    use std::{any::Any, cell::Cell, rc::Rc};
    struct Context(Cell<i32>);
    impl PartialEq for Context {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    register_value_type::<Context>();
    let value = Rc::new(Context(Cell::new(7)));
    let weak = Rc::downgrade(&value);
    let source: Rc<dyn Any> = value.clone();
    let native = from_recycling_value(&source);
    assert_eq!(native.downcast_ref::<Context>().unwrap().0.get(), 7);
    let recovered = to_recycling_value(native.clone());
    assert!(Rc::ptr_eq(&source, &recovered));
    let again = from_recycling_value(&recovered);
    assert!(Rc::ptr_eq(&native, &again));
    drop((source, native, recovered, again, value));
    assert!(weak.upgrade().is_none());
}
