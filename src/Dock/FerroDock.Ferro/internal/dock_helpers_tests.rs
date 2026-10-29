// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original hit-test/model helpers on real windows and production engine objects.
use super::*;
use ferrodock_model::core::IFactory;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, ProportionalDock, RootDock},
    core::{DockableBase, DockableBaseImpl},
    Factory,
};
use ferroui_base::{
    input::RawInputModifiers,
    media::Brushes,
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{AppBuilder, Application, Border, Panel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "dock-helpers",
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
struct Scene(Ref<Window>);
impl Scene {
    fn new(control: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(400.0);
        w.set_height(300.0);
        w.set_content(Some(Control::boxed(control)));
        w.show();
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        w.mouse_move(Point::new(20.0, 20.0), RawInputModifiers::NONE);
        Self(w)
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
        self.0.set_content(None);
    }
}
fn area() -> Ref<Border> {
    let b = Border::new();
    b.set_background(Some(Brushes::transparent()));
    DockProperties::set_is_drop_area(&b, true);
    b
}
fn hit(v: &Visual) -> Option<Ref<Control>> {
    DockHelpers::get_control(
        Some(v),
        Point::new(20.0, 20.0),
        DockProperties::is_drop_area_property(),
    )
}
#[test]
fn original_hidden_window_does_not_expose_retained_drop_area() {
    run(|| {
        let drop = area();
        let s = Scene::new(drop.clone().upcast());
        assert_eq!(hit(&drop), Some(drop.clone().upcast()));
        s.0.hide();
        assert!(drop.is_visible());
        assert!(!drop.is_effectively_visible());
        assert!(hit(&drop).is_none());
        s.0.show();
        s.0.mouse_move(Point::new(20.0, 20.0), RawInputModifiers::NONE);
        assert_eq!(hit(&drop), Some(drop.upcast()));
    });
}
#[test]
fn original_to_dock_point_returns_same_coordinates_and_invariant_string() {
    let p = DockHelpers::to_dock_point(Point::new(10.0, 20.0));
    assert_eq!(p.x, 10.0);
    assert_eq!(p.y, 20.0);
    assert_eq!(p.to_string(), "10, 20");
    let p = DockHelpers::to_dock_point(Point::new(f64::NAN, f64::INFINITY));
    assert!(p.x.is_nan());
    assert_eq!(p.y, f64::INFINITY);
}
#[test]
fn original_find_proportional_dock_uses_original_factory_search() {
    let f = Factory::new();
    let root = RootDock::new();
    root.set_factory(Some(f.clone()));
    let docs = DocumentDock::new();
    docs.set_factory(Some(f.clone()));
    f.add_dockable(Some(root.clone()), Some(docs.clone()));
    let proportional = ProportionalDock::new();
    f.add_dockable(Some(docs.clone()), Some(proportional.clone()));
    assert!(Rc::ptr_eq(
        &DockHelpers::find_proportional_dock(docs.clone()).unwrap(),
        &(proportional as Rc<dyn IProportionalDock>)
    ));
    docs.set_factory(None);
    assert!(DockHelpers::find_proportional_dock(docs).is_none());
}
#[test]
fn native_get_control_preserves_topmost_explicit_property_and_raw_fallback_rules() {
    run(|| {
        let panel = Panel::new();
        let lower = area();
        let upper = area();
        panel.children().add(lower.clone().upcast::<Control>());
        panel.children().add(upper.clone().upcast::<Control>());
        let _s = Scene::new(panel.clone().upcast());
        assert_eq!(hit(&panel), Some(upper.clone().upcast()));
        upper.set_value(DockProperties::is_drop_area_property(), false);
        assert_eq!(hit(&panel), Some(lower.clone().upcast()));
        // A filtered hit exists (the panel), so raw fallback must not select the excluded child.
        lower.set_is_hit_test_visible(false);
        assert!(hit(&panel).is_none());
        // With no filtered hits, source raw fallback intentionally does not recheck input state.
        panel.set_is_hit_test_visible(false);
        assert_eq!(hit(&panel), Some(lower.clone().upcast()));
        panel.set_is_hit_test_visible(true);
        lower.set_is_hit_test_visible(true);
        lower.set_is_enabled(false);
        assert!(hit(&panel).is_none());
        panel.set_is_enabled(false);
        assert_eq!(hit(&panel), Some(lower.clone().upcast()));
        upper.set_is_visible(false);
        lower.set_is_visible(false);
        assert!(hit(&panel).is_none());
    });
}
#[test]
fn native_missing_input_root_outside_point_and_inherited_default_do_not_fabricate_a_drop() {
    run(|| {
        assert!(DockHelpers::get_control(
            None,
            Point::default(),
            DockProperties::is_drop_area_property()
        )
        .is_none());
        let detached = area();
        assert!(!DockHelpers::is_hit_test_visible(&detached));
        assert!(hit(&detached).is_none());
        let no_mark = Border::new();
        no_mark.set_background(Some(Brushes::transparent()));
        let s = Scene::new(no_mark.clone().upcast());
        assert!(hit(&no_mark).is_none());
        assert!(DockHelpers::get_control(
            Some(&no_mark),
            Point::new(900.0, 900.0),
            DockProperties::is_drop_area_property()
        )
        .is_none());
        s.0.set_is_visible(false);
        assert!(hit(&no_mark).is_none());
    });
}
#[test]
fn native_screen_point_uses_actual_top_level_screen_scaling_and_geometry() {
    run(|| {
        let b = area();
        let _s = Scene::new(b.clone().upcast());
        let point = Point::new(11.5, 19.75);
        let top = TopLevel::get_top_level(Some(&b)).unwrap();
        let scale = top
            .screens()
            .and_then(|s| s.screen_from_visual(&b))
            .map(|s| s.scaling())
            .unwrap_or_else(|| top.render_scaling());
        assert_eq!(
            DockHelpers::get_screen_point(&b, point),
            b.point_to_screen(point).to_point(scale)
        );
        assert_eq!(
            DockHelpers::get_screen_point(&top, Point::default()),
            top.point_to_screen(Point::default()).to_point(scale)
        );
    });
}
struct OwnerBase {
    inner: DockableBase,
    reads: Cell<usize>,
    answers: RefCell<VecDeque<Option<Rc<dyn IDockable>>>>,
}
impl std::ops::Deref for OwnerBase {
    type Target = DockableBase;
    fn deref(&self) -> &DockableBase {
        &self.inner
    }
}
impl AsRef<ferroui_reactiveui::ReactiveObject> for OwnerBase {
    fn as_ref(&self) -> &ferroui_reactiveui::ReactiveObject {
        self.inner.as_ref()
    }
}
impl OwnerBase {
    fn owner(&self) -> Option<Rc<dyn IDockable>> {
        self.reads.set(self.reads.get() + 1);
        self.answers
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| self.inner.owner())
    }
}
struct OwnerProbe {
    base: OwnerBase,
}
ferroui_reactiveui::reactive_object!(OwnerProbe, base);
impl DockableBaseImpl for OwnerProbe {}
ferrodock_model_reactiveui::impl_dockable!(@base OwnerProbe,OwnerBase);
impl OwnerProbe {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: OwnerBase {
                inner: DockableBase::construct(runtime.clone()),
                reads: Cell::new(0),
                answers: Default::default(),
            },
        })
    }
}
struct Settings {
    managed: bool,
    mode: DockFloatingWindowHostMode,
    diagnostics: bool,
    logger: Option<ferrodock_settings::DiagnosticsLogHandler>,
}
impl Settings {
    fn save() -> Self {
        Self {
            managed: DockSettings::use_managed_windows(),
            mode: DockSettings::floating_window_host_mode(),
            diagnostics: DockSettings::enable_diagnostics_logging(),
            logger: DockSettings::diagnostics_log_handler(),
        }
    }
}
impl Drop for Settings {
    fn drop(&mut self) {
        DockSettings::set_use_managed_windows(self.managed);
        DockSettings::set_floating_window_host_mode(self.mode);
        DockSettings::set_enable_diagnostics_logging(self.diagnostics);
        DockSettings::set_diagnostics_log_handler(self.logger.clone());
    }
}
#[test]
fn source_model_host_mode_root_owner_factory_and_global_precedence_retain_owner_pattern_read_count()
{
    run(|| {
        let _settings = Settings::save();
        DockSettings::set_floating_window_host_mode(DockFloatingWindowHostMode::Default);
        DockSettings::set_use_managed_windows(false);
        let root = RootDock::new();
        root.set_floating_window_host_mode(DockFloatingWindowHostMode::Managed);
        assert!(DockHelpers::is_managed_window_hosting_enabled_for_dockable(
            Some(root.clone())
        ));
        let next = RootDock::new();
        next.set_floating_window_host_mode(DockFloatingWindowHostMode::Native);
        let probe = OwnerProbe::new();
        probe
            .base
            .answers
            .borrow_mut()
            .extend([Some(root.clone() as Rc<dyn IDockable>), Some(next)]);
        assert!(DockHelpers::is_managed_window_hosting_enabled_for_dockable(
            Some(probe.clone())
        ));
        assert_eq!(probe.base.reads.get(), 1);
        let probe = OwnerProbe::new();
        probe
            .base
            .answers
            .borrow_mut()
            .extend([Some(DocumentDock::new() as Rc<dyn IDockable>), None]);
        assert_eq!(
            DockHelpers::resolve_floating_window_host_mode_for_dockable(Some(probe.clone())),
            DockFloatingWindowHostMode::Native
        );
        assert_eq!(probe.base.reads.get(), 2);
        DockSettings::set_floating_window_host_mode(DockFloatingWindowHostMode::Managed);
        assert!(DockHelpers::is_managed_window_hosting_enabled_for_dockable(
            None
        ));
        let f = Factory::new();
        let parent = DocumentDock::new();
        let d = Document::new();
        let r = f.create_layout().unwrap();
        r.set_floating_window_host_mode(DockFloatingWindowHostMode::Native);
        f.add_dockable(Some(r.clone()), Some(parent.clone()));
        f.add_dockable(Some(parent), Some(d.clone()));
        f.init_layout(r.clone());
        assert!(!DockHelpers::is_managed_window_hosting_enabled_for_dockable(Some(d)));
    });
}
#[test]
fn source_diagnostics_and_native_exception_message_obey_existing_logger_gate() {
    run(|| {
        let _settings = Settings::save();
        let logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = logs.clone();
        DockSettings::set_diagnostics_log_handler(Some(std::sync::Arc::new(move |s| {
            seen.lock().unwrap().push(s)
        })));
        DockSettings::set_enable_diagnostics_logging(false);
        DockHelpers::log_drop_areas(None, "disabled");
        DockHelpers::print_panic(&"failure");
        assert!(logs.lock().unwrap().is_empty());
        DockSettings::set_enable_diagnostics_logging(true);
        DockHelpers::log_drop_areas(None, "root");
        let panel = Panel::new();
        let b = area();
        panel.children().add(b.upcast::<Control>());
        DockHelpers::log_drop_areas(Some(&panel), "tree");
        DockHelpers::print_panic(&"failure");
        assert!(logs
            .lock()
            .unwrap()
            .iter()
            .any(|s| s == "[Dock] Exception: failure"));
        #[cfg(debug_assertions)]
        {
            assert!(logs
                .lock()
                .unwrap()
                .iter()
                .any(|s| s == "[Dock] DropSearch: [root] Root visual is null."));
            assert!(logs
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.contains("Found 1 drop area controls under Panel.")));
        }
    });
}

struct Host {
    presented: RefCell<Vec<bool>>,
}
impl ferrodock_model::core::IHostWindow for Host {
    fn host_window_state(&self) -> Option<Rc<dyn ferrodock_model::core::IHostWindowState>> {
        None
    }
    fn is_tracked(&self) -> bool {
        false
    }
    fn set_is_tracked(&self, _: bool) {}
    fn window(&self) -> Option<Rc<dyn ferrodock_model::core::IDockWindow>> {
        None
    }
    fn set_window(&self, _: Option<Rc<dyn ferrodock_model::core::IDockWindow>>) {}
    fn present(&self, dialog: bool) {
        self.presented.borrow_mut().push(dialog);
    }
    fn exit(&self) {}
    fn set_position(&self, _: f64, _: f64) {}
    fn get_position(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_size(&self, _: f64, _: f64) {}
    fn get_size(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_window_state(&self, _: ferrodock_model::core::DockWindowState) {}
    fn get_window_state(&self) -> ferrodock_model::core::DockWindowState {
        ferrodock_model::core::DockWindowState::Normal
    }
    fn set_title(&self, _: Option<String>) {}
    fn set_layout(&self, _: Rc<dyn IDock>) {}
    fn set_active(&self) {}
}
#[test]
fn source_show_windows_runs_only_active_nested_root_original_reactive_command() {
    use ferrodock_model::core::IDockWindow;
    let factory = Factory::new();
    let outer = RootDock::new();
    let inner = RootDock::new();
    outer.set_active_dockable(Some(inner.clone()));
    outer.set_factory(Some(factory.clone()));
    let d = Document::new();
    d.set_owner(Some(outer.clone()));
    let host = Rc::new(Host {
        presented: Default::default(),
    });
    let window = ferrodock_model_reactiveui::core::DockWindow::new();
    window.set_host(Some(host.clone()));
    window.set_layout(Some(RootDock::new()));
    window.set_is_modal(true);
    inner.set_windows(Some(ferrodock_model::collections::ListHandle::from_plain(
        vec![Some(window as Rc<dyn IDockWindow>)],
    )));
    DockHelpers::show_windows(d.clone());
    assert_eq!(&*host.presented.borrow(), &[true]);
    outer.set_active_dockable(Some(DocumentDock::new()));
    DockHelpers::show_windows(d.clone());
    assert_eq!(host.presented.borrow().len(), 1);
    d.set_owner(Some(Document::new()));
    DockHelpers::show_windows(d.clone());
    d.set_owner(None);
    DockHelpers::show_windows(d);
    assert_eq!(host.presented.borrow().len(), 1);
    inner.set_windows(None);
}
