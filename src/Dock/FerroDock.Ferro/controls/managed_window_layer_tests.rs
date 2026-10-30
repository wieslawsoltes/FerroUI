// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original layer/registry scenarios through real native controls and production window models.
use super::*;
use crate::{
    controls::ManagedWindowDock,
    mdi::{MdiLayoutEntry, MdiResizeDirection},
};
use ferrodock_model::{controls::IMdiDocument, core::IFactory};
use ferrodock_model_reactiveui::{controls::Document, core::DockWindow, Factory};
use ferroui_base::{
    controls::ResourceKey,
    styling::{ControlTheme, Setter},
    threading::CancellationToken,
    Vector,
};
use ferroui_controls::{templates::FuncControlTemplate, AppBuilder, Application, Border, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::component_model::INotifyPropertyChanged;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "managed-window-layer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f()
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap()
}
fn document(id: &str, topmost: bool) -> (Rc<DockWindow>, Rc<ManagedDockWindowDocument>) {
    let w = DockWindow::new();
    w.set_id(id.into());
    w.set_topmost(topmost);
    let d = ManagedDockWindowDocument::new(w.clone());
    (w, d)
}
fn template() -> Rc<dyn ferroui_controls::templates::IControlTemplate> {
    FuncControlTemplate::for_type::<ManagedWindowLayer>(|_, scope| {
        let canvas = Canvas::new();
        canvas.set_name(Some("PART_OverlayCanvas".into()));
        scope.register("PART_OverlayCanvas", canvas.clone().upcast());
        canvas.upcast()
    })
}
struct Scene {
    window: Ref<Window>,
    layer: Ref<ManagedWindowLayer>,
}
impl Scene {
    fn new(layer: Ref<ManagedWindowLayer>) -> Self {
        layer.set_template(Some(template()));
        let w = Window::new();
        w.set_width(400.0);
        w.set_height(300.0);
        w.set_content(Some(Control::boxed(layer.clone())));
        w.show();
        layer.apply_template();
        w.update_layout();
        Self { window: w, layer }
    }
    fn canvas(&self) -> Ref<Canvas> {
        self.layer.overlay_canvas.borrow().clone().unwrap()
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
        self.window.set_content(None);
    }
}
#[derive(Default)]
struct RecordingManager {
    lists: RefCell<Vec<ListHandle<Rc<dyn IMdiDocument>>>>,
    updates: RefCell<Vec<(Vec<String>, Option<String>)>>,
    hook: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IMdiLayoutManager for RecordingManager {
    fn arrange(&self, _: &ListHandle<MdiLayoutEntry>, _: Size) {}
    fn get_drag_bounds(
        &self,
        _: Rc<dyn IMdiDocument>,
        b: Rect,
        _: Vector,
        _: Size,
        _: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        b
    }
    fn get_resize_bounds(
        &self,
        _: Rc<dyn IMdiDocument>,
        b: Rect,
        _: Vector,
        _: MdiResizeDirection,
        _: Size,
        _: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        b
    }
    fn update_z_order(
        &self,
        d: &ListHandle<Rc<dyn IMdiDocument>>,
        active: Option<Rc<dyn IMdiDocument>>,
    ) {
        self.lists.borrow_mut().push(d.clone());
        self.updates
            .borrow_mut()
            .push((d.iter().map(|d| d.id()).collect(), active.map(|d| d.id())));
        let hook = self.hook.borrow_mut().take();
        if let Some(hook) = hook {
            hook()
        }
    }
}
#[test]
fn original_defaults_properties_native_registration_and_non_attached_dock_subscription() {
    run(|| {
        let l = ManagedWindowLayer::new();
        assert!(l.dock().is_none());
        assert!(Rc::ptr_eq(
            &l.layout_manager().unwrap(),
            &(ClassicMdiLayoutManager::instance() as Rc<dyn IMdiLayoutManager>)
        ));
        assert_eq!(l.get_type().namespace(), "FerroDock.Ferro.Controls");
        let dock = ManagedWindowDock::new();
        l.set_dock(Some(dock.clone()));
        let ctx = crate::model_projection::dockable(l.data_context().as_ref()).unwrap();
        assert!(Rc::ptr_eq(&ctx, &(dock.clone() as Rc<dyn IDockable>)));
        assert!(l.dock_subscription.borrow().is_some());
        assert!(l.dockables_subscription.borrow().is_some());
        l.set_dock(None);
        assert!(l.data_context().is_none());
        assert!(l.dock_subscription.borrow().is_none());
        assert!(l.dockables_subscription.borrow().is_none());
        let manager = Rc::new(RecordingManager::default());
        l.set_layout_manager(Some(manager.clone()));
        l.set_dock(Some(dock.clone()));
        assert_eq!(manager.updates.borrow().len(), 1);
        l.set_layout_manager(None);
        assert_eq!(manager.updates.borrow().len(), 1);
        l.set_dock(None);
    });
}
#[test]
fn original_registry_register_unregister_replaces_and_hides_only_the_registered_layer() {
    run(|| {
        let factory: Rc<dyn IFactory> = Factory::new();
        let a = ManagedWindowLayer::new();
        let b = ManagedWindowLayer::new();
        ManagedWindowRegistry::register_layer(&factory, &a);
        let dock = ManagedWindowRegistry::get_or_create_dock(&factory);
        assert_eq!(dock.id(), "ManagedWindowDock");
        assert_eq!(dock.title(), "Managed Windows");
        assert!(Rc::ptr_eq(&dock.factory().unwrap(), &factory));
        assert!(Rc::ptr_eq(
            &a.dock().unwrap(),
            &(dock.clone() as Rc<dyn IDock>)
        ));
        a.set_is_visible(false);
        ManagedWindowRegistry::register_layer(&factory, &a);
        assert!(!a.is_visible());
        ManagedWindowRegistry::register_layer(&factory, &b);
        assert!(a.dock().is_none());
        assert!(!a.is_visible());
        assert!(Rc::ptr_eq(&b.dock().unwrap(), &(dock as Rc<dyn IDock>)));
        ManagedWindowRegistry::unregister_layer(&factory, &a);
        assert_eq!(
            ManagedWindowRegistry::try_get_layer(&factory),
            Some(b.clone())
        );
        ManagedWindowRegistry::unregister_layer(&factory, &b);
        assert!(b.dock().is_none());
        assert!(!b.is_visible());
        assert!(ManagedWindowRegistry::try_get_layer(&factory).is_none());
    });
}
#[test]
fn original_registry_try_get_layer_and_independent_factories_keep_original_identity() {
    run(|| {
        let f: Rc<dyn IFactory> = Factory::new();
        let g: Rc<dyn IFactory> = Factory::new();
        assert!(ManagedWindowRegistry::try_get_layer(&f).is_none());
        let a = ManagedWindowLayer::new();
        let b = ManagedWindowLayer::new();
        ManagedWindowRegistry::register_layer(&f, &a);
        ManagedWindowRegistry::register_layer(&g, &b);
        assert_eq!(ManagedWindowRegistry::try_get_layer(&f), Some(a.clone()));
        assert_eq!(ManagedWindowRegistry::try_get_layer(&g), Some(b.clone()));
        assert!(!Rc::ptr_eq(
            &ManagedWindowRegistry::get_or_create_dock(&f),
            &ManagedWindowRegistry::get_or_create_dock(&g)
        ));
        ManagedWindowRegistry::unregister_layer(&f, &a);
        ManagedWindowRegistry::unregister_layer(&g, &b);
    });
}
#[test]
fn original_layer_updates_zorder_for_topmost_active_and_live_topmost_reference() {
    run(|| {
        let dock = ManagedWindowDock::new();
        let (w1, d1) = document("one", false);
        let (_w2, d2) = document("two", true);
        let (_w3, d3) = document("three", true);
        let list = dock.visible_dockables().unwrap();
        list.add(Some(d1.clone()));
        list.add(Some(d2.clone()));
        list.add(Some(d3.clone()));
        dock.set_active_dockable(Some(d2.clone()));
        let l = ManagedWindowLayer::new();
        l.set_dock(Some(dock.clone()));
        assert!(d2.mdi_z_index() > d3.mdi_z_index());
        assert!(d3.mdi_z_index() > d1.mdi_z_index());
        w1.set_topmost(true);
        dock.set_active_dockable(Some(d1.clone()));
        assert!(d1.mdi_z_index() > d2.mdi_z_index());
        w1.set_topmost(false);
        assert!(d1.mdi_z_index() < d2.mdi_z_index());
        assert!(d1.mdi_z_index() < d3.mdi_z_index());
        l.set_dock(None);
    });
}
#[test]
fn original_collection_add_remove_replace_reset_and_move_preserve_window_subscriptions() {
    run(|| {
        let dock = ManagedWindowDock::new();
        let list = dock.visible_dockables().unwrap();
        let l = ManagedWindowLayer::new();
        l.set_dock(Some(dock.clone()));
        let (_w1, d1) = document("one", false);
        let (_w2, d2) = document("two", false);
        let (_w3, d3) = document("three", false);
        list.add(Some(d1.clone()));
        assert_eq!(l.window_subscriptions.borrow().len(), 1);
        assert!(l
            .window_subscriptions
            .borrow()
            .contains_key(&(d1.clone() as Rc<dyn IDockable>)));
        list.add(Some(d2.clone()));
        assert_eq!(l.window_subscriptions.borrow().len(), 2);
        list.remove(&Some((d1.clone() as Rc<dyn IDockable>).clone()));
        assert_eq!(l.window_subscriptions.borrow().len(), 1);
        assert!(!l
            .window_subscriptions
            .borrow()
            .contains_key(&(d1 as Rc<dyn IDockable>)));
        list.set(0, Some(d3.clone()));
        assert_eq!(l.window_subscriptions.borrow().len(), 1);
        assert!(!l
            .window_subscriptions
            .borrow()
            .contains_key(&(d2.clone() as Rc<dyn IDockable>)));
        assert!(l
            .window_subscriptions
            .borrow()
            .contains_key(&(d3 as Rc<dyn IDockable>)));
        list.add(Some(d2));
        let tokens: Vec<_> = l.window_subscriptions.borrow().values().copied().collect();
        list.as_observable_collection().unwrap().move_item(0, 1);
        assert_eq!(
            l.window_subscriptions
                .borrow()
                .values()
                .copied()
                .collect::<Vec<_>>(),
            tokens
        );
        list.clear();
        assert!(l.window_subscriptions.borrow().is_empty());
        l.set_dock(None);
    });
}
#[test]
fn original_overlay_show_hide_retains_keyed_control_geometry_and_null_size() {
    run(|| {
        let s = Scene::new(ManagedWindowLayer::new());
        let overlay = Border::new();
        s.layer.show_overlay(
            "TestOverlay",
            overlay.clone().upcast(),
            Rect::new(10.0, 20.0, 120.0, 80.0),
            true,
        );
        let canvas = s.canvas();
        assert!(canvas.children().contains(&overlay.clone().upcast()));
        assert_eq!(Canvas::get_left(&overlay), 10.0);
        assert_eq!(Canvas::get_top(&overlay), 20.0);
        assert_eq!(overlay.width(), 120.0);
        assert_eq!(overlay.height(), 80.0);
        assert!(overlay.is_hit_test_visible());
        let ignored = Border::new();
        s.layer.show_overlay_at(
            "TestOverlay",
            ignored.clone().upcast(),
            Point::new(-4.0, 5.0),
            None,
            false,
        );
        assert_eq!(canvas.children().count(), 1);
        assert!(ignored.visual_parent().is_none());
        assert_eq!(Canvas::get_left(&overlay), -4.0);
        assert!(overlay.width().is_nan() && overlay.height().is_nan());
        assert!(!overlay.is_hit_test_visible());
        s.layer.hide_overlay("absent");
        assert_eq!(canvas.children().count(), 1);
        s.layer.hide_overlay("TestOverlay");
        assert!(!canvas.children().contains(&overlay.upcast()));
        assert!(s.layer.overlays.borrow().is_empty());
    });
}
#[test]
fn source_missing_canvas_never_fabricates_an_overlay_or_offset() {
    run(|| {
        let l = ManagedWindowLayer::new();
        l.show_overlay(
            "missing",
            Border::new().upcast(),
            Rect::new(0.0, 0.0, 10.0, 10.0),
            true,
        );
        l.hide_overlay("missing");
        assert!(l.overlay_canvas.borrow().is_none());
        assert!(l.overlays.borrow().is_empty());
        assert_eq!(l.try_get_window_content_offset(), None);
    });
}
#[test]
fn original_window_name_filter_and_old_list_window_detach_use_original_event_tokens() {
    run(|| {
        let dock = ManagedWindowDock::new();
        let (window, d) = document("managed", false);
        let old = dock.visible_dockables().unwrap();
        old.add(Some(d));
        let l = ManagedWindowLayer::new();
        let manager = Rc::new(RecordingManager::default());
        l.set_layout_manager(Some(manager.clone()));
        l.set_dock(Some(dock.clone()));
        manager.updates.borrow_mut().clear();
        window.property_changed().raise("Title");
        assert!(manager.updates.borrow().is_empty());
        window.property_changed().raise("");
        assert_eq!(manager.updates.borrow().len(), 1);
        manager.updates.borrow_mut().clear();
        let replacement = ListHandle::new();
        dock.set_visible_dockables(Some(replacement));
        assert!(l.window_subscriptions.borrow().is_empty());
        manager.updates.borrow_mut().clear();
        window.set_topmost(true);
        old.clear();
        assert!(manager.updates.borrow().is_empty());
        dock.as_notify_property_changed()
            .unwrap()
            .property_changed()
            .raise("");
        assert!(manager.updates.borrow().is_empty());
        l.set_dock(None);
    });
}
#[test]
fn source_zorder_partition_snapshot_and_reentrant_manager_calls_preserve_order() {
    run(|| {
        let dock = ManagedWindowDock::new();
        let (_w1, normal) = document("normal", false);
        let (_w2, top) = document("top", true);
        dock.visible_dockables().unwrap().add(Some(normal.clone()));
        dock.visible_dockables().unwrap().add(Some(top.clone()));
        dock.set_active_dockable(Some(top.clone()));
        let l = ManagedWindowLayer::new();
        let manager = Rc::new(RecordingManager::default());
        l.set_layout_manager(Some(manager.clone()));
        l.set_dock(Some(dock));
        assert_eq!(
            &*manager.updates.borrow(),
            &[
                (vec!["normal".into()], None),
                (vec!["top".into()], Some("top".into()))
            ]
        );
        assert_eq!(top.mdi_z_index(), 1);
        let second = ManagedWindowDock::new();
        let d = Document::new();
        d.set_id("replacement".into());
        second.visible_dockables().unwrap().add(Some(d));
        manager.updates.borrow_mut().clear();
        let owner = l.clone();
        *manager.hook.borrow_mut() = Some(Rc::new(move || owner.set_dock(Some(second.clone()))));
        l.update_z_order();
        assert_eq!(
            &*manager.updates.borrow(),
            &[
                (vec!["normal".into()], None),
                (vec!["replacement".into()], None),
                (vec!["top".into()], Some("top".into()))
            ]
        );
        assert_eq!(top.mdi_z_index(), 2);
        l.set_dock(None);
    });
}
#[test]
fn native_detach_unregisters_layer_and_releases_original_subscriptions_with_dependencies_alive() {
    run(|| {
        let f: Rc<dyn IFactory> = Factory::new();
        let layer = ManagedWindowLayer::new();
        let weak = layer.downgrade();
        ManagedWindowRegistry::register_layer(&f, &layer);
        let dock = ManagedWindowRegistry::get_or_create_dock(&f);
        let (window, d) = document("retained source", false);
        dock.add_window(Some(d));
        let scene = Scene::new(layer.clone());
        drop(layer);
        drop(scene);
        assert!(ManagedWindowRegistry::try_get_layer(&f).is_none());
        assert!(weak.upgrade().is_none());
        window.set_topmost(true);
    });
}
#[test]
fn native_registry_prunes_expired_noncyclic_keys_on_access_without_claiming_ephemeron_gc() {
    run(|| {
        let f: Rc<dyn IFactory> = Factory::new();
        let weak_factory = Rc::downgrade(&f);
        let layer = ManagedWindowLayer::new();
        let weak_layer = layer.downgrade();
        ManagedWindowRegistry::register_layer(&f, &layer);
        layer.set_dock(None);
        drop(layer);
        let dock = ManagedWindowRegistry::get_or_create_dock(&f);
        drop(f);
        // The original managed model deliberately retains Factory strongly. Native
        // weak keys cannot collect this value-to-key cycle as CLR ephemerons do.
        assert!(weak_factory.upgrade().is_some());
        dock.set_factory(None);
        drop(dock);
        assert!(weak_factory.upgrade().is_none());
        assert!(weak_layer.upgrade().is_some());
        let other: Rc<dyn IFactory> = Factory::new();
        assert!(ManagedWindowRegistry::try_get_layer(&other).is_none());
        assert!(weak_layer.upgrade().is_none());
    });
}

fn window_template(point: Point) -> Rc<dyn ferroui_controls::templates::IControlTemplate> {
    FuncControlTemplate::for_type::<MdiDocumentWindow>(move |_, scope| {
        let canvas = Canvas::new();
        let border = Border::new();
        border.set_width(200.0);
        border.set_height(150.0);
        scope.register("PART_ContentBorder", border.clone().upcast());
        Canvas::set_left(&border, point.x);
        Canvas::set_top(&border, point.y);
        canvas.children().add(border.upcast::<Control>());
        canvas.upcast()
    })
}
fn add_probe_theme(layer: &ManagedWindowLayer, point: Point) {
    let theme = ControlTheme::for_type::<MdiDocumentWindow>();
    let value: Option<Rc<dyn ferroui_controls::templates::IControlTemplate>> =
        Some(window_template(point));
    theme.setters().add(Setter::new_untyped(
        TemplatedControl::template_property().as_property(),
        Rc::new(value),
    ));
    layer
        .resources()
        .add_value(ResourceKey::from(MdiDocumentWindow::TYPE), theme);
}
#[test]
fn original_offset_probe_retries_failed_measurement_cleans_overlay_and_caches_success() {
    run(|| {
        let s = Scene::new(ManagedWindowLayer::new());
        assert_eq!(s.layer.try_get_window_content_offset(), None);
        assert!(s.layer.overlays.borrow().is_empty());
        assert_eq!(s.canvas().children().count(), 0);
        add_probe_theme(&s.layer, Point::new(9.0, 23.0));
        assert_eq!(
            s.layer.try_get_window_content_offset(),
            Some(Point::new(9.0, 23.0))
        );
        assert!(s.layer.overlays.borrow().is_empty());
        assert_eq!(s.canvas().children().count(), 0);
        assert_eq!(
            s.layer.cached_window_content_offset.get(),
            Some(Point::new(9.0, 23.0))
        );
    });
}
#[test]
fn original_content_offset_prefers_first_visible_frame_and_only_template_apply_clears_cache() {
    run(|| {
        let s = Scene::new(ManagedWindowLayer::new());
        let a = MdiDocumentWindow::new();
        a.set_template(Some(window_template(Point::new(1.0, 2.0))));
        a.set_is_visible(false);
        let b = MdiDocumentWindow::new();
        b.set_template(Some(window_template(Point::new(3.0, 4.0))));
        s.layer.show_overlay(
            "a",
            a.clone().upcast(),
            Rect::new(0.0, 0.0, 300.0, 200.0),
            true,
        );
        s.layer.show_overlay(
            "b",
            b.clone().upcast(),
            Rect::new(0.0, 0.0, 300.0, 200.0),
            true,
        );
        s.window.update_layout();
        assert_eq!(
            s.layer.try_get_window_content_offset(),
            Some(Point::new(3.0, 4.0))
        );
        a.set_is_visible(true);
        s.window.update_layout();
        s.layer.set_dock(Some(ManagedWindowDock::new()));
        assert_eq!(
            s.layer.try_get_window_content_offset(),
            Some(Point::new(3.0, 4.0))
        );
        add_probe_theme(&s.layer, Point::new(7.0, 8.0));
        s.layer.set_template(Some(template()));
        s.layer.apply_template();
        assert!(s.layer.cached_window_content_offset.get().is_none());
        assert_eq!(
            s.layer.try_get_window_content_offset(),
            Some(Point::new(7.0, 8.0))
        );
    });
}
#[test]
fn source_overlay_key_survives_template_replacement_without_moving_its_existing_control() {
    run(|| {
        let s = Scene::new(ManagedWindowLayer::new());
        let old = s.canvas();
        let control = Border::new();
        s.layer.show_overlay(
            "stable",
            control.clone().upcast(),
            Rect::new(0.0, 0.0, 20.0, 20.0),
            true,
        );
        s.layer.set_template(Some(template()));
        s.layer.apply_template();
        let new = s.canvas();
        assert_ne!(old, new);
        s.layer.show_overlay(
            "stable",
            Border::new().upcast(),
            Rect::new(1.0, 2.0, 30.0, 40.0),
            false,
        );
        assert_eq!(new.children().count(), 0);
        assert!(old.children().contains(&control.clone().upcast()));
        assert_eq!(control.width(), 30.0);
        s.layer.hide_overlay("stable");
        assert!(s.layer.overlays.borrow().is_empty());
        assert!(old.children().contains(&control.upcast()));
    });
}

#[test]
fn native_styled_nullable_dock_conversion_accepts_original_raw_and_handle_references() {
    run(|| {
        use ferroui_base::data::core::{ValueType, ValueTypes};
        let dock = ManagedWindowDock::new();
        let target = ValueType::of::<Option<Rc<dyn IDock>>>();
        for value in [
            dock.clone() as ferroui_base::BoxedValue,
            Rc::new(dock.clone()) as ferroui_base::BoxedValue,
            Rc::new(Some(dock.clone())) as ferroui_base::BoxedValue,
        ] {
            let result = ValueTypes::try_convert(Some(&value), target)
                .unwrap()
                .unwrap();
            let actual = result
                .downcast_ref::<Option<Rc<dyn IDock>>>()
                .unwrap()
                .as_ref()
                .unwrap();
            assert!(Rc::ptr_eq(actual, &(dock.clone() as Rc<dyn IDock>)));
        }
    });
}

#[test]
fn source_partition_lists_retain_identity_when_custom_manager_mutates_the_original_snapshot() {
    run(|| {
        let dock = ManagedWindowDock::new();
        let (_normal_window, normal) = document("normal", false);
        let (_top_window, top) = document("top", true);
        dock.visible_dockables().unwrap().add(Some(normal));
        dock.visible_dockables().unwrap().add(Some(top.clone()));
        let manager = Rc::new(RecordingManager::default());
        let extra: Rc<dyn IMdiDocument> = Document::new();
        let recorded = manager.clone();
        *manager.hook.borrow_mut() = Some(Rc::new(move || {
            recorded.lists.borrow()[0].add(extra.clone())
        }));
        let l = ManagedWindowLayer::new();
        l.set_layout_manager(Some(manager.clone()));
        l.set_dock(Some(dock));
        assert_eq!(manager.lists.borrow()[0].count(), 2);
        assert_eq!(top.mdi_z_index(), 2);
        l.set_dock(None);
    });
}
