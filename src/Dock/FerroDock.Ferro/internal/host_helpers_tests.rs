//! Original host cleanup, preview, state and drag resolution through native controls.
use super::{
    dock_window_state_helper::DockWindowStateHelper as States,
    drag_dockable_resolver::DragDockableResolver as Resolver,
    drag_preview_context::DragPreviewContext as Preview,
    host_window_content_cleanup::HostWindowContentCleanup as Cleanup,
    tool_menu_lifetime::ToolMenuLifetime,
    visual_root_compat_extensions::VisualRootCompatExtensions as Roots,
};
use crate::{
    controls::{ManagedDockableBase, ManagedDockableBaseImpl},
    model_projection,
};
use ferrodock_model::{collections::ListHandle, core::*, BoxedValue as ModelValue};
use ferrodock_model_reactiveui::{controls::*, Factory};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::{BindingOperations, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElement,
    StyledElementImpl,
};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::{
    AppBuilder, Application, Button, ContentControl, Control, Flyout, Panel, Window, WindowState,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-host-helpers",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene(Ref<Window>);
impl Scene {
    fn new(content: Ref<Control>) -> Self {
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(400.0);
        window.set_content(Some(Control::boxed(content)));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        Self(window)
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
#[test]
fn window_state_helper_preserves_every_native_and_mdi_mapping_including_fullscreen_loss() {
    for (native, dock) in [
        (WindowState::Normal, DockWindowState::Normal),
        (WindowState::Minimized, DockWindowState::Minimized),
        (WindowState::Maximized, DockWindowState::Maximized),
        (WindowState::FullScreen, DockWindowState::FullScreen),
    ] {
        assert_eq!(States::to_dock_window_state(native), dock);
        assert_eq!(States::to_ferro_window_state(dock), native);
    }
    for (dock, mdi) in [
        (DockWindowState::Normal, MdiWindowState::Normal),
        (DockWindowState::Minimized, MdiWindowState::Minimized),
        (DockWindowState::Maximized, MdiWindowState::Maximized),
        (DockWindowState::FullScreen, MdiWindowState::Normal),
    ] {
        assert_eq!(States::to_mdi_window_state(dock), mdi);
    }
    for (mdi, dock) in [
        (MdiWindowState::Normal, DockWindowState::Normal),
        (MdiWindowState::Minimized, DockWindowState::Minimized),
        (MdiWindowState::Maximized, DockWindowState::Maximized),
    ] {
        assert_eq!(States::to_dock_window_state_from_mdi(mdi), dock);
    }
}
#[test]
fn preview_context_is_gated_by_active_and_original_reference_identity_and_clear_releases_it() {
    Preview::clear();
    let doc = Document::new() as Rc<dyn IDockable>;
    let other = Document::new() as Rc<dyn IDockable>;
    let weak = Rc::downgrade(&doc);
    Preview::set_dockable(Some(doc.clone()));
    assert!(!Preview::is_active());
    assert!(!Preview::is_previewing(&doc));
    Preview::set_is_active(true);
    assert!(Preview::is_previewing(&doc));
    assert!(!Preview::is_previewing(&other));
    assert!(Rc::ptr_eq(&Preview::dockable().unwrap(), &doc));
    drop(doc);
    assert!(weak.upgrade().is_some());
    Preview::clear();
    assert!(!Preview::is_active());
    assert!(Preview::dockable().is_none());
    assert!(weak.upgrade().is_none());
    Preview::set_is_active(true);
    assert!(!Preview::is_previewing(&other));
    Preview::clear();
}
#[test]
fn source_pinned_preview_self_drop_is_no_op_for_all_operations() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let preview = factory.create_tool_dock();
    preview.set_visible_dockables(Some(factory.create_list(Vec::new())));
    let tool = factory.create_tool();
    factory.add_dockable(Some(preview.clone()), Some(tool.clone()));
    preview.set_active_dockable(Some(tool.clone()));
    root.set_pinned_dock(Some(preview.clone()));
    factory.init_dockable(Some(root.clone()), None);
    let source = preview.clone() as Rc<dyn IDockable>;
    assert!(Rc::ptr_eq(
        &Resolver::resolve(source.clone()),
        &(tool.clone() as Rc<dyn IDockable>)
    ));
    assert!(Resolver::is_self_drop(source.clone(), source.clone()));
    assert!(Resolver::is_self_drop(source.clone(), tool.clone()));
    for raw in 0..=10 {
        let op = DockOperation::try_from(raw).unwrap();
        assert!(Resolver::is_no_op_drop(source.clone(), source.clone(), op));
        assert!(Resolver::is_no_op_drop(source.clone(), tool.clone(), op));
    }
    root.set_pinned_dock(None);
    assert!(Rc::ptr_eq(&Resolver::resolve(source.clone()), &source));
}
#[test]
fn source_regular_tool_dock_preserves_whole_dock_drag() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    root.set_visible_dockables(Some(factory.create_list(Vec::new())));
    let dock = factory.create_tool_dock();
    dock.set_visible_dockables(Some(factory.create_list(Vec::new())));
    let tool = factory.create_tool();
    factory.add_dockable(Some(dock.clone()), Some(tool.clone()));
    dock.set_active_dockable(Some(tool));
    factory.add_dockable(Some(root.clone()), Some(dock.clone()));
    let source = dock as Rc<dyn IDockable>;
    assert!(Rc::ptr_eq(&Resolver::resolve(source.clone()), &source));
}
#[test]
fn source_regular_dockable_owner_is_not_self_drop_and_only_fill_is_a_noop() {
    let factory = Factory::new();
    let dock = factory.create_document_dock();
    dock.set_visible_dockables(Some(factory.create_list(Vec::new())));
    let document = factory.create_document();
    factory.add_dockable(Some(dock.clone()), Some(document.clone()));
    let source = document as Rc<dyn IDockable>;
    assert!(Resolver::is_self_drop(source.clone(), source.clone()));
    assert!(!Resolver::is_self_drop(source.clone(), dock));
    for raw in 0..=10 {
        let op = DockOperation::try_from(raw).unwrap();
        assert_eq!(
            Resolver::is_no_op_drop(source.clone(), source.clone(), op),
            op == DockOperation::Fill
        );
    }
}
#[test]
fn preview_resolution_requires_root_identity_active_document_and_matching_pinned_dock() {
    let root = RootDock::new();
    let dock = ToolDock::new();
    let other = ToolDock::new();
    let tool = Tool::new();
    let source = dock.clone() as Rc<dyn IDockable>;
    root.set_pinned_dock(Some(dock.clone()));
    dock.set_active_dockable(Some(tool.clone()));
    assert!(Rc::ptr_eq(&Resolver::resolve(source.clone()), &source));
    dock.set_owner(Some(root.clone()));
    dock.set_active_dockable(None);
    assert!(Rc::ptr_eq(&Resolver::resolve(source.clone()), &source));
    dock.set_active_dockable(Some(tool.clone()));
    root.set_pinned_dock(Some(other));
    assert!(Rc::ptr_eq(&Resolver::resolve(source.clone()), &source));
    root.set_pinned_dock(Some(dock.clone()));
    tool.set_owner(None);
    assert!(Rc::ptr_eq(
        &Resolver::resolve(source.clone()),
        &(tool.clone() as Rc<dyn IDockable>)
    ));
    assert!(!Resolver::is_self_drop(source, root));
}
fn presenter(value: ferroui_base::BoxedValue) -> Ref<ContentControl> {
    let p = ContentControl::new();
    p.set_content(Some(value));
    p
}
#[test]
fn cleanup_snapshots_descendants_before_reverse_clearing_and_skips_nonmodels_and_host() {
    run(|| {
        let panel = Panel::new();
        let first = presenter(model_projection::to_dockable_value(Document::new()));
        let last = presenter(model_projection::to_dockable_value(Tool::new()));
        let plain = presenter(Rc::new(String::from("ordinary content")));
        panel.children().add(first.clone());
        panel.children().add(last.clone());
        panel.children().add(plain.clone());
        let scene = Scene::new(panel.clone().upcast());
        let order = Rc::new(RefCell::new(Vec::new()));
        let recorded = order.clone();
        let weak = panel.downgrade();
        let a = last.property_changed(move |e| {
            if e.property() == ContentControl::content_property().as_property() {
                recorded.borrow_mut().push("last");
                weak.upgrade().unwrap().children().clear();
            }
        });
        let recorded = order.clone();
        let b = first.property_changed(move |e| {
            if e.property() == ContentControl::content_property().as_property() {
                recorded.borrow_mut().push("first");
            }
        });
        Cleanup::release_models(&panel, None);
        assert_eq!(&*order.borrow(), &["last", "first"]);
        assert!(first.content().is_none() && last.content().is_none());
        assert!(plain.content().is_some());
        a.dispose();
        b.dispose();
        drop(scene);
        let host = presenter(model_projection::to_dockable_value(Document::new()));
        let original = host.content().unwrap();
        Cleanup::release_models(&host, None);
        assert!(Rc::ptr_eq(&host.content().unwrap(), &original));
    });
}
#[repr(C)]
struct LogicalElement {
    base: StyledElement,
    model: RefCell<Weak<LogicalModel>>,
}
ferro_class!(LogicalElement:StyledElement);
ferro_impl_classes!(LogicalElement:FerroObjectImpl,StyledElementImpl);
struct LogicalModel {
    base: ManagedDockableBase,
    element: Ref<LogicalElement>,
}
crate::impl_managed_dockable!(
    LogicalModel,
    fn boxed_reference(self: Rc<Self>) -> ModelValue {
        Rc::new(self.element.clone())
    }
);
impl ManagedDockableBaseImpl for LogicalModel {}
impl LogicalModel {
    fn new() -> Rc<Self> {
        let element = instantiate(LogicalElement {
            base: StyledElement::construct(),
            model: RefCell::new(Weak::new()),
        });
        let model = Rc::new_cyclic(|runtime: &std::rc::Weak<Self>| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
            element: element.clone(),
        });
        *element.model.borrow_mut() = Rc::downgrade(&model);
        ValueTypes::register_cast::<Ref<LogicalElement>, Rc<dyn IDockable>>(|element| {
            element.model.borrow().upgrade().unwrap()
        });
        model
    }
    fn presenter(self: &Rc<Self>) -> Ref<ContentControl> {
        presenter(Rc::new(self.element.clone()))
    }
}
#[test]
fn cleanup_finds_detached_logical_presenters_in_all_original_root_and_split_collections_and_handles_cycles(
) {
    run(|| {
        let host = Window::new();
        let root = RootDock::new();
        let split = SplitViewDock::new();
        let pinned = ToolDock::new();
        let models: Vec<_> = (0..9).map(|_| LogicalModel::new()).collect();
        let presenters: Vec<_> = models.iter().map(LogicalModel::presenter).collect();
        for (model, presenter) in models.iter().zip(&presenters) {
            assert_eq!(
                model.element.parent().unwrap(),
                presenter.clone().upcast::<StyledElement>()
            );
            assert!(presenter.visual_root().is_none());
        }
        root.set_visible_dockables(Some(ListHandle::from_plain(vec![
            Some(split.clone()),
            Some(models[0].clone()),
        ])));
        root.set_hidden_dockables(Some(ListHandle::from_plain(vec![Some(models[1].clone())])));
        root.set_left_pinned_dockables(Some(ListHandle::from_plain(vec![Some(models[2].clone())])));
        root.set_right_pinned_dockables(Some(ListHandle::from_plain(vec![Some(
            models[3].clone(),
        )])));
        root.set_top_pinned_dockables(Some(ListHandle::from_plain(vec![Some(models[4].clone())])));
        root.set_bottom_pinned_dockables(Some(ListHandle::from_plain(vec![Some(
            models[5].clone(),
        )])));
        pinned.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(models[6].clone())])));
        root.set_pinned_dock(Some(pinned));
        split.set_pane_dockable(Some(models[7].clone()));
        split.set_content_dockable(Some(models[8].clone()));
        split.set_visible_dockables(Some(ListHandle::from_plain(vec![
            Some(root.clone()),
            Some(models[0].clone()),
        ])));
        let seen = Rc::new(Cell::new(0));
        let mut subscriptions = Vec::new();
        for p in &presenters {
            let seen = seen.clone();
            subscriptions.push(p.property_changed(move |e| {
                if e.property() == ContentControl::content_property().as_property() {
                    seen.set(seen.get() + 1);
                }
            }));
        }
        Cleanup::release_models(&host, Some(root.clone()));
        assert_eq!(seen.get(), 9);
        for p in presenters {
            assert!(p.content().is_none());
        }
        for m in models {
            assert!(m.element.parent().is_none());
        }
        // Release the intentionally cyclic source layout after validating visited identity.
        split.set_visible_dockables(None);
        root.set_visible_dockables(None);
        for s in subscriptions {
            s.dispose();
        }
        host.close();
    });
}
#[test]
fn cleanup_preserves_logical_presenters_in_other_windows_and_the_host_itself() {
    run(|| {
        let model = LogicalModel::new();
        let parent = model.presenter();
        let other = Scene::new(parent.clone().upcast());
        let host = Window::new();
        let root = RootDock::new();
        root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(model.clone())])));
        Cleanup::release_models(&host, Some(root.clone()));
        assert!(parent.content().is_some());
        Cleanup::release_models(&parent, Some(root));
        assert!(parent.content().is_some());
        drop(other);
        host.close();
    });
}
#[test]
fn cleanup_releases_nested_and_deferred_source_presenters_from_a_single_snapshot() {
    run(|| {
        use ferrodock_controls_deferred_content_control::DeferredContentControl;
        let outer = ContentControl::new();
        let inner = ContentControl::new();
        let deferred = DeferredContentControl::new();
        let template_panel = Panel::new();
        template_panel.children().add(inner.clone());
        template_panel.children().add(deferred.clone());
        let root = template_panel.clone();
        outer.set_template(Some(FuncControlTemplate::for_type::<ContentControl>(
            move |_, _| root.clone().upcast(),
        )));
        outer.set_content(Some(model_projection::to_dockable_value(Document::new())));
        inner.set_content(Some(model_projection::to_dockable_value(Document::new())));
        deferred.set_content(Some(model_projection::to_dockable_value(Document::new())));
        let scene = Scene::new(outer.clone().upcast());
        let record = Rc::new(RefCell::new(Vec::new()));
        let mut subscriptions = Vec::new();
        for (name, p) in [
            ("outer", outer.clone()),
            ("inner", inner.clone()),
            ("deferred", deferred.clone().upcast::<ContentControl>()),
        ] {
            let r = record.clone();
            subscriptions.push(p.property_changed(move |e| {
                if e.property() == ContentControl::content_property().as_property() {
                    r.borrow_mut().push(name);
                }
            }));
        }
        Cleanup::release_models(&scene.0, None);
        assert_eq!(&*record.borrow(), &["deferred", "inner", "outer"]);
        assert!(deferred.content().is_none());
        for s in subscriptions {
            s.dispose();
        }
    });
}
#[test]
fn tool_menu_detach_and_attach_preserve_original_flyout_instance_and_replacement_policy() {
    run(|| {
        let lifetime = ToolMenuLifetime::new();
        lifetime.attach();
        lifetime.detach();
        lifetime.set_button(None);
        let button = Button::new();
        let original = Flyout::new().upcast::<FlyoutBase>();
        button.set_flyout(Some(original.clone()));
        lifetime.set_button(Some(button.clone()));
        lifetime.detach();
        assert!(button.flyout().is_none());
        lifetime.detach();
        lifetime.attach();
        assert_eq!(button.flyout(), Some(original.clone()));
        lifetime.detach();
        let other = Flyout::new().upcast::<FlyoutBase>();
        button.set_flyout(Some(other.clone()));
        lifetime.attach();
        assert_eq!(button.flyout(), Some(other));
        lifetime.set_button(None);
        assert!(button.flyout().is_none());
        lifetime.attach();
        assert!(button.flyout().is_none());
        let replacement = Button::new();
        replacement.set_flyout(Some(original.clone()));
        lifetime.set_button(Some(replacement.clone()));
        lifetime.detach();
        lifetime.attach();
        assert_eq!(replacement.flyout(), Some(original));
    });
}
#[test]
fn tool_menu_reattaches_through_original_compiled_binding_and_retains_the_binding() {
    run(|| {
        let source = Button::new();
        let button = Button::new();
        let original = Flyout::new().upcast::<FlyoutBase>();
        source.set_flyout(Some(original.clone()));
        let binding = CompiledBinding::new(
            CompiledBindingPathBuilder::new()
                .ferro_property(Button::flyout_property().as_property())
                .build(),
        )
        .with_source(Some(Rc::new(source.clone())));
        let subscription = button.bind_binding(Button::flyout_property().as_property(), &*binding);
        let lifetime = ToolMenuLifetime::new();
        lifetime.set_button(Some(button.clone()));
        assert_eq!(button.flyout(), Some(original.clone()));
        lifetime.detach();
        assert!(button.flyout().is_none());
        assert!(BindingOperations::get_binding_expression_base(
            &button,
            Button::flyout_property().as_property()
        )
        .is_some());
        lifetime.attach();
        assert_eq!(button.flyout(), Some(original));
        let replacement = Flyout::new().upcast::<FlyoutBase>();
        source.set_flyout(Some(replacement.clone()));
        assert_eq!(button.flyout(), Some(replacement));
        subscription.dispose();
    });
}
#[test]
fn nullable_visual_root_uses_the_actual_presentation_source_and_clears_after_detach() {
    run(|| {
        assert!(Roots::get_visual_root(None).is_none());
        let child = Panel::new();
        assert!(Roots::get_visual_root(Some(&child)).is_none());
        let scene = Scene::new(child.clone().upcast());
        assert_eq!(
            Roots::get_visual_root(Some(&child)),
            child.presentation_source().unwrap().root_visual()
        );
        scene.0.set_content(None);
        assert!(Roots::get_visual_root(Some(&child)).is_none());
    });
}

#[test]
fn tool_menu_property_callback_can_replace_the_button_during_detach() {
    run(|| {
        let lifetime = ToolMenuLifetime::new();
        let old = Button::new();
        let new = Button::new();
        let old_menu = Flyout::new().upcast::<FlyoutBase>();
        let new_menu = Flyout::new().upcast::<FlyoutBase>();
        old.set_flyout(Some(old_menu));
        new.set_flyout(Some(new_menu.clone()));
        lifetime.set_button(Some(old.clone()));
        let weak = Rc::downgrade(&lifetime);
        let next = new.clone();
        let subscription = old.property_changed(move |e| {
            if e.property() == Button::flyout_property().as_property() {
                weak.upgrade().unwrap().set_button(Some(next.clone()));
            }
        });
        lifetime.detach();
        lifetime.attach();
        assert!(old.flyout().is_none());
        assert_eq!(new.flyout(), Some(new_menu));
        subscription.dispose();
    });
}

#[test]
fn cleanup_clears_realized_deferred_content_without_removing_original_binding() {
    run(|| {
        use ferrodock_controls_deferred_content_control::DeferredContentControl;
        let source = ContentControl::new();
        source.set_content(Some(model_projection::to_dockable_value(Document::new())));
        let deferred = DeferredContentControl::new();
        let binding = CompiledBinding::new(
            CompiledBindingPathBuilder::new()
                .ferro_property(ContentControl::content_property().as_property())
                .build(),
        )
        .with_source(Some(Rc::new(source.clone())));
        let subscription =
            deferred.bind_binding(ContentControl::content_property().as_property(), &*binding);
        let scene = Scene::new(deferred.clone().upcast());
        Dispatcher::ui_thread().run_jobs(None);
        scene.0.update_layout();
        let materialized = deferred.presenter().unwrap();
        assert!(materialized.child().is_some());
        Cleanup::release_models(&scene.0, None);
        assert!(deferred.content().is_none());
        assert!(materialized.content().is_none());
        assert!(materialized.child().is_none());
        // Source SetValue clears current content while retaining its binding.
        // A later source update can refill an attached presenter; do not unbind here.
        assert!(BindingOperations::get_binding_expression_base(
            &deferred,
            ContentControl::content_property().as_property()
        )
        .is_some());
        let next = Tool::new() as Rc<dyn IDockable>;
        source.set_content(Some(model_projection::to_dockable_value(next.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        scene.0.update_layout();
        assert!(Rc::ptr_eq(
            &model_projection::dockable(deferred.content().as_ref()).unwrap(),
            &next
        ));
        Cleanup::release_models(&scene.0, None);
        assert!(deferred.content().is_none());
        assert!(materialized.child().is_none());
        subscription.dispose();
    });
}
