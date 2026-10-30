// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, Visual,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::ContentControl;
use ferroui_controls::{AppBuilder, Application, Border, Control, ControlImpl, Panel, Window};
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
    name: "ferrodock-overlay-headless",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
pub(super) fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                OverlayLayerRegistry::use_service_provider(None);
                test();
                OverlayLayerRegistry::use_service_provider(None);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
pub(super) fn update(window: &Window) {
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
}
pub(super) fn window(child: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(child)));
    window.show();
    update(&window);
    window
}
fn layer(overlay: Ref<Control>, z: i32) -> Ref<OverlayLayer> {
    let layer = OverlayLayer::new();
    layer.set_overlay(Some(overlay));
    layer.set_z_index(z);
    layer
}
fn layers(values: impl IntoIterator<Item = Ref<OverlayLayer>>) -> OverlayLayerCollection {
    let collection = OverlayLayerCollection::new();
    for layer in values {
        collection.add(Some(OverlayLayer::as_layer(layer)));
    }
    collection
}
fn contains(root: &Visual, target: &Visual) -> bool {
    root.visual_children()
        .snapshot()
        .iter()
        .any(|child| child.ptr_eq(&target.to_ref()) || contains(child, target))
}
fn host(content: Ref<Control>, layers: OverlayLayerCollection) -> Ref<OverlayHost> {
    let host = OverlayHost::new();
    host.set_content(Some(Control::boxed(content)));
    host.set_use_service_layers(false);
    host.set_overlay_layers(Some(layers));
    host
}
#[test]
fn rebuild_pipeline_detaches_content_from_visual_parent() {
    run(|| {
        use ferroui_controls::presenters::ContentPresenter;
        let content = Border::new();
        let presenter = ContentPresenter::new();
        presenter.set_content(Some(Control::boxed(content.clone())));
        let panel = Panel::new();
        panel.children().add(presenter.clone());
        let w = window(panel.clone().upcast());
        assert!(content.visual_parent().unwrap().ptr_eq(&presenter));
        content.set_parent(None::<Ref<ferroui_base::StyledElement>>);
        let host = OverlayHost::new();
        host.set_content(Some(Control::boxed(content.clone())));
        panel.children().add(host);
        update(&w);
        assert!(content.visual_parent().is_some());
        assert!(!content.visual_parent().unwrap().ptr_eq(&presenter));
        w.close();
    });
}
#[test]
fn structurally_equivalent_replacement_keeps_current_attached_layers_and_pipeline() {
    run(|| {
        let content = Border::new();
        let overlay = Border::new();
        let old = layers([layer(overlay.clone().upcast(), 5)]);
        let host = host(content.clone().upcast(), old.clone());
        let w = window(host.clone().upcast());
        let before = host.child();
        assert!(before.is_some());
        assert!(overlay.visual_parent().is_some());
        let new = Border::new();
        let replacement = layers([layer(new.clone().upcast(), 5)]);
        host.set_overlay_layers(Some(replacement.clone()));
        update(&w);
        assert_eq!(host.child(), before);
        assert!(overlay.visual_parent().is_some());
        assert!(new.visual_parent().is_none());
        assert!(contains(&host, &content));
        // Source retains the old private list while the public property holds new.
        assert!(host.overlay_layers() == replacement);
        let extra = Border::new();
        old.add(Some(OverlayLayer::as_layer(layer(
            extra.clone().upcast(),
            10,
        ))));
        update(&w);
        assert!(contains(&host, &extra));
        w.close();
    });
}
#[test]
fn genuinely_different_layer_replacement_rebuilds() {
    run(|| {
        let content = Border::new();
        let first = layer(Border::new().upcast(), 5);
        let host = host(content.clone().upcast(), layers([first.clone()]));
        let w = window(host.clone().upcast());
        let overlay = Border::new();
        host.set_overlay_layers(Some(layers([first, layer(overlay.clone().upcast(), 10)])));
        update(&w);
        assert!(overlay.visual_parent().is_some());
        assert!(contains(&host, &content));
        w.close();
    });
}
#[derive(Default)]
struct Probe {
    depth: Cell<i32>,
    max: Cell<i32>,
}
impl Probe {
    fn enter(&self) {
        self.depth.set(self.depth.get() + 1);
        self.max.set(self.max.get().max(self.depth.get()));
    }
    fn exit(&self) {
        self.depth.set(self.depth.get() - 1);
    }
}
#[repr(C)]
struct ProbedControl {
    base: Control,
    probe: Rc<Probe>,
    attached: Cell<i32>,
    detached: Cell<i32>,
}
ferro_class!(ProbedControl:Control);
ferro_impl_classes!(ProbedControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ProbedControl {
    fn new(probe: Rc<Probe>) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            probe,
            attached: Cell::new(0),
            detached: Cell::new(0),
        })
    }
}
impl VisualImpl for ProbedControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attached.set(this.attached.get() + 1);
        this.probe.enter();
        this.probe.exit();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.detached.set(this.detached.get() + 1);
        this.probe.enter();
        this.probe.exit();
    }
}
#[repr(C)]
struct ProbedBorder {
    base: Border,
    probe: Rc<Probe>,
    attached: Cell<i32>,
    detached: Cell<i32>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
ferro_class!(ProbedBorder:Border);
ferro_impl_classes!(ProbedBorder:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ProbedBorder {
    fn new(probe: Rc<Probe>) -> Ref<Self> {
        instantiate(Self {
            base: Border::construct(),
            probe,
            attached: Cell::new(0),
            detached: Cell::new(0),
            callback: RefCell::new(None),
        })
    }
}
impl VisualImpl for ProbedBorder {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attached.set(this.attached.get() + 1);
        this.probe.enter();
        this.probe.exit();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.detached.set(this.detached.get() + 1);
        this.probe.enter();
        struct Exit<'a>(&'a Probe);
        impl Drop for Exit<'_> {
            fn drop(&mut self) {
                self.0.exit();
            }
        }
        let _exit = Exit(&this.probe);
        let callback = this.callback.take();
        if let Some(callback) = callback {
            callback();
        }
    }
}
#[test]
fn rebuild_reentrant_mutation_during_detach_is_coalesced_not_recursive() {
    run(|| {
        let content = Border::new();
        let probe = Rc::new(Probe::default());
        let inner = ProbedControl::new(probe.clone());
        let outer = ProbedBorder::new(probe.clone());
        let layers = layers([
            layer(inner.clone().upcast(), 0),
            layer(outer.clone().upcast(), 10),
        ]);
        let panel = Panel::new();
        let w = window(panel.clone().upcast());
        let host = host(content.upcast(), layers.clone());
        panel.children().add(host.clone());
        update(&w);
        assert_eq!(inner.attached.get(), 1);
        assert_eq!(outer.attached.get(), 1);
        let extra = Border::new();
        let new_layer = layer(extra.clone().upcast(), 20);
        let captured = layers.clone();
        outer.callback.replace(Some(Box::new(move || {
            captured.add(Some(OverlayLayer::as_layer(new_layer)))
        })));
        let new_content = Border::new();
        host.set_content(Some(Control::boxed(new_content.clone())));
        update(&w);
        assert_eq!(probe.max.get(), 1);
        assert!(outer.detached.get() >= 1);
        assert_eq!(layers.count(), 3);
        for control in [
            new_content.clone().upcast::<Control>(),
            inner.clone().upcast(),
            outer.clone().upcast(),
            extra.clone().upcast(),
        ] {
            assert!(contains(&host, &control));
            assert!(control.visual_parent().is_some());
        }
        w.close();
    });
}

#[test]
fn layer_defaults_properties_and_canonical_interface_preserve_native_identity() {
    run(|| {
        let a = OverlayLayer::new();
        assert_eq!(a.z_index(), 0);
        assert!(a.is_visible());
        assert!(a.blocks_input());
        assert!(a.style_key().is_none());
        assert!(a.overlay().is_none());
        a.set_z_index(12);
        a.set_is_visible(false);
        a.set_blocks_input(false);
        a.set_style_key(Some(Rc::new("theme".to_owned())));
        let overlay = Border::new();
        a.set_overlay(Some(overlay.clone().upcast()));
        let x = OverlayLayer::as_layer(a.clone());
        let y = OverlayLayer::as_layer(a.clone());
        assert!(Rc::ptr_eq(&x, &y));
        assert!(x.as_object().unwrap().ptr_eq(&a));
        assert_eq!(x.z_index(), 12);
        assert!(!x.is_visible());
        assert!(!x.blocks_input());
        assert_eq!(
            x.style_key().unwrap().downcast_ref::<String>().unwrap(),
            "theme"
        );
        assert_eq!(x.overlay(), Some(overlay.upcast::<Control>()));
    });
}
struct PlainLayer {
    z: Cell<i32>,
    overlay: Ref<Control>,
    on_z: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl IOverlayLayer for PlainLayer {
    fn z_index(&self) -> i32 {
        let action = self.on_z.take();
        if let Some(action) = action {
            action();
        }
        self.z.get()
    }
    fn is_visible(&self) -> bool {
        true
    }
    fn blocks_input(&self) -> bool {
        true
    }
    fn style_key(&self) -> Option<ferroui_base::BoxedValue> {
        None
    }
    fn overlay(&self) -> Option<Ref<Control>> {
        Some(self.overlay.clone())
    }
}
#[test]
fn ordered_layers_are_lazy_repeatable_stable_and_buffer_before_reading_keys() {
    run(|| {
        let collection = OverlayLayerCollection::new();
        let a = Rc::new(PlainLayer {
            z: Cell::new(5),
            overlay: Border::new().upcast(),
            on_z: RefCell::new(None),
        });
        let b = OverlayLayer::as_layer(layer(Border::new().upcast(), 5));
        let c = OverlayLayer::as_layer(layer(Border::new().upcast(), -1));
        collection.add(Some(a.clone()));
        collection.add(Some(b.clone()));
        let query = collection.get_ordered_layers();
        let mut iter = query.iter();
        collection.add(Some(c.clone()));
        let d = OverlayLayer::as_layer(layer(Border::new().upcast(), -10));
        let shared = collection.clone();
        a.on_z.replace(Some(Box::new(move || shared.add(Some(d)))));
        assert!(Rc::ptr_eq(&iter.next().unwrap(), &c));
        assert!(Rc::ptr_eq(
            &iter.next().unwrap(),
            &(a.clone() as Rc<dyn IOverlayLayer>)
        ));
        assert!(Rc::ptr_eq(&iter.next().unwrap(), &b));
        assert!(iter.next().is_none());
        assert_eq!(query.iter().count(), 4);
        a.z.set(-20);
        assert!(Rc::ptr_eq(
            &query.iter().next().unwrap(),
            &(a as Rc<dyn IOverlayLayer>)
        ));
        let mut raw = OverlayLayerSource::Collection(collection.clone()).iter();
        raw.next();
        collection.add(None);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| raw.next())).is_err());
    });
}
#[test]
fn registry_changes_are_atomic_identity_sensitive_reentrant_and_release_subscribers() {
    run(|| {
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        let event = OverlayLayerRegistry::provider_changed();
        let token = event.add(Rc::new(move |_| {
            seen.set(seen.get() + 1);
        }));
        let captured = Rc::new(Cell::new(0));
        let weak = Rc::downgrade(&captured);
        let retained = captured.clone();
        let provider: Rc<OverlayLayerProvider> = Rc::new(move || {
            retained.set(retained.get() + 1);
            Some(OverlayLayerSource::from_items(Vec::new()))
        });
        OverlayLayerRegistry::set_provider(Some(provider.clone()));
        OverlayLayerRegistry::set_provider(Some(provider.clone()));
        assert_eq!(calls.get(), 1);
        let factories: Rc<OverlayFactoryProvider> =
            Rc::new(|| Some(OverlayFactorySource::from_items(Vec::new())));
        OverlayLayerRegistry::set_factory_provider(Some(factories));
        assert_eq!(calls.get(), 2);
        let state = Rc::new(Cell::new(false));
        let observe = state.clone();
        let token2 = event.add(Rc::new(move |_| {
            observe.set(
                OverlayLayerRegistry::provider().is_none()
                    && OverlayLayerRegistry::factory_provider().is_none(),
            );
        }));
        drop(captured);
        drop(provider);
        assert!(weak.upgrade().is_some());
        OverlayLayerRegistry::use_service_provider(None);
        assert!(state.get());
        assert!(weak.upgrade().is_none());
        assert_eq!(calls.get(), 3);
        event.remove(token);
        event.remove(token2);
        let entered = Rc::new(Cell::new(false));
        let entered_callback = entered.clone();
        let nested = Rc::new(Cell::new(0));
        let count = nested.clone();
        let token = event.add(Rc::new(move |_| {
            count.set(count.get() + 1);
            if !entered_callback.replace(true) {
                OverlayLayerRegistry::set_provider(None);
            }
        }));
        OverlayLayerRegistry::set_provider(Some(Rc::new(|| None)));
        assert_eq!(nested.get(), 2);
        assert!(OverlayLayerRegistry::provider().is_none());
        event.remove(token);
    });
}
#[test]
fn factory_layers_take_priority_and_create_distinct_controls_for_each_host() {
    run(|| {
        let created = Rc::new(RefCell::new(Vec::new()));
        let output = created.clone();
        let factory = OverlayLayerFactory::new(Some(Rc::new(move || {
            let control = Border::new();
            output.borrow_mut().push(control.clone());
            Some(OverlayLayer::as_layer(layer(control.upcast(), 0)))
        })));
        let null = OverlayLayerFactory::new(Some(Rc::new(|| None)));
        let factories = OverlayFactorySource::from_items(vec![None, Some(null), Some(factory)]);
        OverlayLayerRegistry::set_factory_provider(Some(Rc::new(move || Some(factories.clone()))));
        OverlayLayerRegistry::set_provider(Some(Rc::new(|| {
            panic!("nonempty created factories must suppress provider")
        })));
        let first = OverlayHost::new();
        let second = OverlayHost::new();
        let panel = Panel::new();
        panel.children().add(first.clone());
        panel.children().add(second.clone());
        let w = window(panel.upcast());
        let values = created.borrow();
        assert_eq!(values.len(), 2);
        assert_ne!(values[0], values[1]);
        assert!(contains(&first, &values[0]));
        assert!(contains(&second, &values[1]));
        drop(values);
        w.close();
    });
}
#[test]
fn service_collection_identity_empty_fallback_and_invisible_layer_priority_are_retained() {
    run(|| {
        let explicit = Border::new();
        let fallback = Border::new();
        let service = Border::new();
        let item = layer(service.clone().upcast(), 0);
        item.set_is_visible(false);
        let shared = layers([item.clone()]);
        let provider_list = shared.clone();
        let provider_calls = Rc::new(Cell::new(0));
        let calls = provider_calls.clone();
        OverlayLayerRegistry::set_provider(Some(Rc::new(move || {
            calls.set(calls.get() + 1);
            Some(OverlayLayerSource::Collection(provider_list.clone()))
        })));
        let host = OverlayHost::new();
        host.set_content(Some(Control::boxed(Border::new())));
        host.set_overlay_layers(Some(layers([layer(explicit.clone().upcast(), 0)])));
        host.overlays()
            .unwrap()
            .add(Some(fallback.clone().upcast()));
        let w = window(host.clone().upcast());
        assert!(!contains(&host, &service));
        assert!(!contains(&host, &explicit));
        assert!(!contains(&host, &fallback));
        assert_eq!(provider_calls.get(), 1);
        item.set_is_visible(true);
        update(&w);
        assert!(contains(&host, &service));
        shared.clear();
        update(&w);
        assert!(contains(&host, &explicit));
        assert_eq!(provider_calls.get(), 1);
        shared.add(Some(OverlayLayer::as_layer(item)));
        update(&w);
        assert!(contains(&host, &service));
        host.set_use_service_layers(false);
        update(&w);
        assert!(contains(&host, &explicit));
        w.close();
    });
}
#[test]
fn native_and_plain_control_lists_keep_their_notification_capabilities() {
    run(|| {
        for native in [false, true] {
            let host = OverlayHost::new();
            host.set_use_service_layers(false);
            host.set_content(Some(Control::boxed(Border::new())));
            let list = if native {
                OverlayControlList::new()
            } else {
                OverlayControlList::plain(Vec::new())
            };
            host.set_overlays(Some(list.clone()));
            let w = window(host.clone().upcast());
            let old = host.child();
            let overlay = Border::new();
            list.add(Some(overlay.clone().upcast()));
            update(&w);
            assert_eq!(contains(&host, &overlay), native);
            assert_eq!(host.child() != old, native);
            if !native {
                host.set_content(Some(Control::boxed(Border::new())));
                update(&w);
                assert!(contains(&host, &overlay));
            }
            host.set_overlays(None);
            update(&w);
            assert!(host.overlays().is_none());
            assert!(!contains(&host, &overlay));
            w.close();
        }
    });
}
#[test]
fn styled_default_layers_apply_theme_hit_testing_and_nonzero_z_without_resetting_zero() {
    run(|| {
        use ferroui_base::styling::ControlTheme;
        let control = Control::new();
        let item = layer(control.clone(), 5);
        item.set_blocks_input(false);
        item.set_style_key(Some(Rc::new("theme".to_owned())));
        let host = host(Border::new().upcast(), layers([item.clone()]));
        let theme = ControlTheme::with_setters(Control::TYPE, []);
        host.resources().add("theme", Some(Rc::new(theme.clone())));
        let w = window(host.clone().upcast());
        assert_eq!(control.theme(), Some(theme));
        assert!(!control.is_hit_test_visible());
        assert_eq!(control.z_index(), 5);
        assert!(host
            .child()
            .unwrap()
            .cast::<ferroui_controls::Grid>()
            .is_some());
        item.set_z_index(0);
        item.set_style_key(None);
        item.set_blocks_input(true);
        update(&w);
        assert_eq!(control.z_index(), 5);
        assert!(control.is_hit_test_visible());
        assert!(control.theme().is_some());
        let type_theme = ControlTheme::with_setters(Control::TYPE, []);
        host.resources()
            .add(Control::TYPE, Some(Rc::new(type_theme.clone())));
        item.set_style_key(Some(Rc::new(ferroui_base::data::core::ValueType::of::<
            Ref<Control>,
        >())));
        update(&w);
        assert_eq!(control.theme(), Some(type_theme));
        w.close();
    });
}
#[test]
fn detached_host_unsubscribes_from_live_layer_lists_registry_and_rebuilds_latest_on_reattach() {
    run(|| {
        let overlay = Border::new();
        let item = layer(overlay.clone().upcast(), 0);
        let list = layers([item.clone()]);
        let host = host(Border::new().upcast(), list.clone());
        let w = window(host.clone().upcast());
        let old = host.child();
        w.close();
        w.set_content(None);
        item.set_is_visible(false);
        list.add(Some(OverlayLayer::as_layer(layer(
            Border::new().upcast(),
            10,
        ))));
        host.set_content(Some(Control::boxed(Border::new())));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(host.child(), old);
        let other = window(host.clone().upcast());
        assert_ne!(host.child(), old);
        assert!(!contains(&host, &overlay));
        let weak = host.downgrade();
        other.close();
        other.set_content(None);
        drop(host);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}

#[repr(C)]
struct ContentHostProbe {
    base: ferroui_controls::ContentControl,
    blocks: Cell<bool>,
}
ferro_class!(ContentHostProbe:ContentControl);
ferro_impl_classes!(ContentHostProbe:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ferroui_controls::primitives::TemplatedControlImpl,ferroui_controls::ContentControlImpl);
ferroui_base::ferro_class_info!(ContentHostProbe {interfaces:[Rc<dyn super::i_overlay_content_host::IOverlayContentHost> => ContentHostProbe::as_content_host]});
impl ContentHostProbe {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: ferroui_controls::ContentControl::construct(),
            blocks: Cell::new(true),
        })
    }
    fn as_content_host(
        this: Ref<Self>,
    ) -> Rc<dyn super::i_overlay_content_host::IOverlayContentHost> {
        Rc::new(ContentHostHandle(this))
    }
}
struct ContentHostHandle(Ref<ContentHostProbe>);
impl super::i_overlay_content_host::IOverlayContentHost for ContentHostHandle {
    fn content(&self) -> Option<ferroui_base::BoxedValue> {
        self.0.content()
    }
    fn set_content(&self, v: Option<ferroui_base::BoxedValue>) {
        self.0.set_content(v);
    }
    fn content_template(&self) -> Option<Rc<dyn ferroui_controls::templates::IDataTemplate>> {
        self.0.content_template()
    }
    fn set_content_template(&self, v: Option<Rc<dyn ferroui_controls::templates::IDataTemplate>>) {
        self.0.set_content_template(v);
    }
    fn blocks_input(&self) -> bool {
        self.0.blocks.get()
    }
    fn set_blocks_input(&self, v: bool) {
        self.0.blocks.set(v);
    }
}
#[test]
fn overlay_content_host_dispatch_precedes_content_control_and_plain_control_branches() {
    run(|| {
        let overlay = ContentHostProbe::new();
        let item = layer(overlay.clone().upcast(), 0);
        item.set_blocks_input(false);
        let content = Border::new();
        let host = host(content.clone().upcast(), layers([item]));
        let w = window(host.clone().upcast());
        assert!(!overlay.blocks.get());
        assert!(overlay.content_template().is_none());
        assert!(contains(&host, &content));
        assert!(host.child().unwrap().ptr_eq(&overlay));
        w.close();
    });
}
#[test]
fn service_provider_uses_shared_contract_and_factory_empty_falls_back_to_layers() {
    run(|| {
        struct Provider {
            layers: OverlayLayerCollection,
            calls: Rc<RefCell<Vec<std::any::TypeId>>>,
        }
        impl ferroui_base::metadata::IServiceProvider for Provider {
            fn get_service(&self, kind: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
                self.calls.borrow_mut().push(kind);
                ferroui_base::metadata::service(kind, || {
                    OverlayLayerSource::Collection(self.layers.clone())
                })
            }
        }
        let control = Border::new();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let provider = Rc::new(Provider {
            layers: layers([layer(control.clone().upcast(), 0)]),
            calls: calls.clone(),
        });
        let weak = Rc::downgrade(&provider);
        OverlayLayerRegistry::use_service_provider(Some(provider.clone()));
        drop(provider);
        let host = OverlayHost::new();
        let w = window(host.clone().upcast());
        assert!(contains(&host, &control));
        assert_eq!(
            &*calls.borrow(),
            &[
                std::any::TypeId::of::<OverlayFactorySource>(),
                std::any::TypeId::of::<OverlayLayerSource>()
            ]
        );
        assert!(weak.upgrade().is_some());
        OverlayLayerRegistry::use_service_provider(None);
        assert!(weak.upgrade().is_none());
        update(&w);
        w.close();
    });
}
#[test]
fn source_factory_rejects_null_delegate_and_style_keys_retain_original_resource_names() {
    run(|| {
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            OverlayLayerFactory::new(None)
        }))
        .is_err());
        let factory = OverlayLayerFactory::new(Some(Rc::new(|| None)));
        assert!(factory.create().is_none());
        assert_eq!(
            OverlayLayerStyleKeys::BUSY_LAYER_THEME_KEY,
            "DockOverlayBusyLayerTheme"
        );
        assert_eq!(
            OverlayLayerStyleKeys::DIALOG_LAYER_THEME_KEY,
            "DockOverlayDialogLayerTheme"
        );
        assert_eq!(
            OverlayLayerStyleKeys::CONFIRMATION_LAYER_THEME_KEY,
            "DockOverlayConfirmationLayerTheme"
        );
    });
}

#[test]
fn complete_control_list_mutations_identity_and_copy_delegate_to_each_backend() {
    run(|| {
        for list in [
            OverlayControlList::new(),
            OverlayControlList::plain(Vec::new()),
        ] {
            assert!(!list.is_read_only());
            let a = Border::new().upcast::<Control>();
            let b = Border::new().upcast::<Control>();
            list.add(Some(a.clone()));
            list.insert(0, None);
            assert_eq!(list.index_of(&Some(a.clone())), Some(1));
            assert!(list.contains(&None));
            list.set(0, Some(b.clone()));
            let mut output = [None, None, None];
            list.copy_to(&mut output, 1);
            assert_eq!(output[1], Some(b.clone()));
            assert_eq!(output[2], Some(a.clone()));
            assert_eq!(list.iter().count(), 2);
            assert!(list.clone() == list);
            assert!(list.remove(&Some(a)));
            list.remove_at(0);
            assert_eq!(list.count(), 0);
            list.add(Some(b));
            list.clear();
            assert_eq!(list.count(), 0);
        }
    });
}

#[test]
fn entry_projection_preserves_repeated_source_getters_and_left_to_right_order() {
    run(|| {
        struct Traced {
            log: Rc<RefCell<Vec<&'static str>>>,
            control: Ref<Control>,
        }
        impl IOverlayLayer for Traced {
            fn z_index(&self) -> i32 {
                self.log.borrow_mut().push("z");
                0
            }
            fn is_visible(&self) -> bool {
                self.log.borrow_mut().push("visible");
                true
            }
            fn overlay(&self) -> Option<Ref<Control>> {
                self.log.borrow_mut().push("overlay");
                Some(self.control.clone())
            }
            fn blocks_input(&self) -> bool {
                self.log.borrow_mut().push("blocks");
                true
            }
            fn style_key(&self) -> Option<ferroui_base::BoxedValue> {
                self.log.borrow_mut().push("style");
                None
            }
        }
        let log = Rc::new(RefCell::new(Vec::new()));
        let source = OverlayLayerCollection::new();
        source.add(Some(Rc::new(Traced {
            log: log.clone(),
            control: Control::new(),
        })));
        let host = OverlayHost::new();
        host.set_use_service_layers(false);
        host.set_overlay_layers(Some(source));
        let w = window(host.upcast());
        assert_eq!(
            &*log.borrow(),
            &["z", "visible", "overlay", "overlay", "z", "visible", "blocks", "style"]
        );
        w.close();
    });
}

#[test]
fn custom_read_only_source_list_keeps_identity_and_direct_notification_subscription() {
    run(|| {
        use ferrodock_model::collections::{IList, ListHandle};
        use ferroui_system::collections::specialized::INotifyCollectionChanged;
        struct ReadOnly {
            items: ListHandle<Option<Ref<Control>>>,
        }
        impl IList<Option<Ref<Control>>> for ReadOnly {
            fn count(&self) -> usize {
                self.items.count()
            }
            fn is_read_only(&self) -> bool {
                true
            }
            fn get(&self, i: usize) -> Option<Ref<Control>> {
                self.items.get(i)
            }
            fn set(&self, _: usize, _: Option<Ref<Control>>) {
                panic!("read-only list")
            }
            fn insert(&self, _: usize, _: Option<Ref<Control>>) {
                panic!("read-only list")
            }
            fn remove_at(&self, _: usize) {
                panic!("read-only list")
            }
            fn clear(&self) {
                panic!("read-only list")
            }
            fn index_of(&self, item: &Option<Ref<Control>>) -> Option<usize> {
                self.items.index_of(item)
            }
            fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
                self.items.as_notify_collection_changed()
            }
        }
        let items = ListHandle::new();
        let original = Rc::new(ReadOnly {
            items: items.clone(),
        });
        let list = OverlayControlList::General(ListHandle::from_list(original.clone()));
        assert!(list.is_read_only());
        let host = OverlayHost::new();
        host.set_use_service_layers(false);
        host.set_overlays(Some(list.clone()));
        let event = items
            .as_notify_collection_changed()
            .unwrap()
            .collection_changed();
        let before = event.handler_count();
        let w = window(host.clone().upcast());
        assert_eq!(event.handler_count(), before + 1);
        let control = Border::new();
        items.add(Some(control.clone().upcast()));
        update(&w);
        assert!(contains(&host, &control));
        if let Some(OverlayControlList::General(value)) = host.overlays() {
            assert!(std::ptr::addr_eq(
                Rc::as_ptr(&value.as_list()),
                Rc::as_ptr(&original)
            ));
        } else {
            panic!("source backend was replaced");
        }
        w.close();
        assert_eq!(event.handler_count(), before);
    });
}
