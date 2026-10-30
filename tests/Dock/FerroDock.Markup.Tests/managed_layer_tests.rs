// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! The complete source managed-layer dictionary in a downstream compiled consumer.
use ferrodock_ferro::controls::{
    ManagedDockWindowDocument, ManagedWindowDock, ManagedWindowLayer, MdiDocumentWindow,
};
use ferrodock_model::core::{DockRect, IDock, IDockable};
use ferrodock_model_reactiveui::core::DockWindow;
use ferroui_base::{
    controls::ResourceKey,
    styling::ControlTheme,
    threading::{CancellationToken, Dispatcher},
    Rect, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, Border, Canvas, Control, ItemsControl, Panel, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "managed-layer-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                f()
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene {
    window: Ref<Window>,
    panel: Ref<Panel>,
    layer: Ref<ManagedWindowLayer>,
    dock: Rc<ManagedWindowDock>,
}
impl Scene {
    fn new() -> Self {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/ManagedLayerConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<Panel>>().unwrap().clone();
        let layer = panel
            .find_descendant_of_type::<ManagedWindowLayer>(false)
            .unwrap();
        let dock = ManagedWindowDock::new();
        panel.set_data_context(Some(Rc::new(dock.clone())));
        let theme = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(
            &panel.find_resource(&ResourceKey::from(ManagedWindowLayer::TYPE)),
        )
        .unwrap();
        layer.set_theme(Some(theme));
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        Self {
            window,
            panel,
            layer,
            dock,
        }
    }
    fn items(&self) -> Ref<ItemsControl> {
        self.layer
            .find_descendant_of_type_where::<ItemsControl>(false, |i| {
                i.name().as_deref() == Some("PART_Windows")
            })
            .unwrap()
    }
    fn frame(&self, id: &str) -> Ref<MdiDocumentWindow> {
        self.layer
            .find_descendant_of_type_where::<MdiDocumentWindow>(false, |w| {
                ferroui_base::metadata::from_markup_value::<Rc<dyn IDockable>>(&w.data_context())
                    .is_some_and(|d| d.id() == id)
            })
            .unwrap()
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
        self.window.set_content(None);
    }
}
fn document(id: &str, topmost: bool) -> (Rc<DockWindow>, Rc<ManagedDockWindowDocument>) {
    let w = DockWindow::new();
    w.set_id(id.into());
    w.set_topmost(topmost);
    let d = ManagedDockWindowDocument::new(w.clone());
    d.set_mdi_bounds(DockRect::new(10.0, 20.0, 280.0, 220.0));
    (w, d)
}
#[test]
fn complete_layer_theme_binds_original_managed_models_and_live_topmost_zorder() {
    run(|| {
        let s = Scene::new();
        let (_w1, d1) = document("first", false);
        let (w2, d2) = document("second", false);
        s.dock.add_window(Some(d1.clone()));
        s.dock.add_window(Some(d2.clone()));
        s.window.update_layout();
        assert!(Rc::ptr_eq(
            &s.layer.dock().unwrap(),
            &(s.dock.clone() as Rc<dyn IDock>)
        ));
        assert_eq!(s.items().items().count(), 2);
        let first = s.frame("first");
        let second = s.frame("second");
        assert_eq!(first.z_index(), d1.mdi_z_index());
        assert_eq!(second.z_index(), d2.mdi_z_index());
        s.dock.set_active_dockable(Some(d1.clone()));
        w2.set_topmost(true);
        assert!(d2.mdi_z_index() > d1.mdi_z_index());
        assert_eq!(second.z_index(), d2.mdi_z_index());
        assert_eq!(first.z_index(), d1.mdi_z_index());
        d2.set_mdi_state(ferrodock_model::core::MdiWindowState::Minimized);
        assert_eq!(
            second.mdi_state(),
            ferrodock_model::core::MdiWindowState::Minimized
        );
        s.dock.remove_window(Some(d1));
        s.window.update_layout();
        assert_eq!(s.items().items().count(), 1);
        assert!(s
            .layer
            .find_descendant_of_type_where::<MdiDocumentWindow>(false, |w| std::ptr::eq(w, &*first))
            .is_none());
    });
}
#[test]
fn complete_layer_theme_contains_original_overlay_canvas_and_rebinds_its_source_dock() {
    run(|| {
        let s = Scene::new();
        let canvas = s
            .layer
            .find_descendant_of_type_where::<Canvas>(false, |c| {
                c.name().as_deref() == Some("PART_OverlayCanvas")
            })
            .unwrap();
        assert_eq!(canvas.z_index(), 10000);
        let b = Border::new();
        s.layer.show_overlay(
            "preview",
            b.clone().upcast(),
            Rect::new(10.0, 20.0, 120.0, 80.0),
            true,
        );
        assert!(canvas.children().contains(&b.clone().upcast()));
        assert_eq!(b.width(), 120.0);
        assert_eq!(Canvas::get_top(&b), 20.0);
        s.layer.hide_overlay("preview");
        assert_eq!(canvas.children().count(), 0);
        let (_w, d) = document("old", false);
        s.dock.add_window(Some(d));
        s.window.update_layout();
        assert_eq!(s.items().items().count(), 1);
        let replacement = ManagedWindowDock::new();
        let (_w2, d2) = document("new", true);
        replacement.add_window(Some(d2));
        s.panel.set_data_context(Some(Rc::new(replacement.clone())));
        s.window.update_layout();
        assert!(Rc::ptr_eq(
            &s.layer.dock().unwrap(),
            &(replacement as Rc<dyn IDock>)
        ));
        assert_eq!(s.items().items().count(), 1);
        assert!(s.frame("new").is_visible());
        s.dock.visible_dockables().unwrap().clear();
        assert_eq!(s.items().items().count(), 1);
    });
}
