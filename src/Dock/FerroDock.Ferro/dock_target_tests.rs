use crate::controls::*;
use ferrodock_model::core::{DockOperation as Op, DragAction};
use ferroui_base::controls::{INameScope, NameScope};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::transformation::TransformOperationsBuilder;
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
    Thickness, VisualImpl,
};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::IInvokeProvider;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::templates::{FuncControlTemplate, IControlTemplate};
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
    name: "ferrodock-dock-target",
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
    fn new(c: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(300.0);
        w.set_height(200.0);
        w.set_content(Some(Control::boxed(c)));
        w.show();
        let s = Self(w);
        s.update();
        s
    }
    fn update(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.0.update_layout();
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
    }
}
fn target(global: bool) -> Ref<DockTargetBase> {
    if global {
        GlobalDockTarget::new().upcast()
    } else {
        DockTarget::new().upcast()
    }
}
fn resolve(t: &DockTargetBase, p: Point) -> Op {
    t.get_dock_operation(p, t, t, DragAction::MOVE, &|_, _, _, _| true, None)
}
type Parts = (
    Rc<dyn IControlTemplate>,
    Vec<Ref<Control>>,
    Vec<Ref<Control>>,
);
fn parts(t: &DockTargetBase, operations: &[(Op, &'static str)]) -> Parts {
    let controls: Vec<_> = operations
        .iter()
        .map(|_| {
            let c = Border::new();
            c.set_width(50.0);
            c.set_height(50.0);
            c.upcast::<Control>()
        })
        .collect();
    let indicators: Vec<_> = operations
        .iter()
        .map(|_| {
            let c = Panel::new();
            c.set_is_hit_test_visible(false);
            c.upcast::<Control>()
        })
        .collect();
    let selected = controls.clone();
    let indicated = indicators.clone();
    let operations = operations.to_vec();
    let template = FuncControlTemplate::for_type::<DockTargetBase>(move |_, scope| {
        let panel = Panel::new();
        for (i, (operation, part)) in operations.iter().enumerate() {
            ferrodock_settings::DockProperties::set_indicator_dock_operation(
                &selected[i],
                *operation,
            );
            ferrodock_settings::DockProperties::set_indicator_dock_operation(
                &indicated[i],
                *operation,
            );
            scope.register(
                &format!("PART_{part}Selector"),
                selected[i].clone().upcast(),
            );
            scope.register(
                &format!("PART_{part}Indicator"),
                indicated[i].clone().upcast(),
            );
            panel.children().add(indicated[i].clone());
            panel.children().add(selected[i].clone());
        }
        panel.upcast()
    });
    t.set_template(Some(template.clone()));
    (template, controls, indicators)
}
#[test]
fn source_default_operations_and_property_pseudo_class_observations() {
    run(|| {
        for global in [false, true] {
            let t = target(global);
            assert_eq!(
                resolve(&t, Point::default()),
                if global { Op::None } else { Op::Window }
            );
            assert!(!t.show_indicators_only());
            assert!(t.show_horizontal_targets());
            assert!(t.show_vertical_targets());
            assert!(!t.is_global_dock_available());
            assert!(!t.is_global_dock_active());
            assert!(t.classes().contains(":horizontal"));
            assert!(t.classes().contains(":vertical"));
            t.set_show_horizontal_targets(false);
            t.set_show_vertical_targets(false);
            t.set_is_global_dock_available(true);
            t.set_is_global_dock_active(true);
            assert!(!t.classes().contains(":horizontal"));
            assert!(!t.classes().contains(":vertical"));
            assert!(t.classes().contains(":global-available"));
            assert!(t.classes().contains(":global-active"));
        }
    });
}
#[test]
fn source_indicator_selector_mapping_missing_and_noncontrol_parts() {
    run(|| {
        let t = DockTarget::new();
        let scope = NameScope::new();
        let c = Border::new();
        ferrodock_settings::DockProperties::set_indicator_dock_operation(&c, Op::Top);
        scope.register("custom", c.clone().upcast());
        t.add_indicator("custom", &scope);
        t.add_selector("custom", &scope);
        assert_eq!(t.indicator_operations().borrow().len(), 1);
        assert_eq!(t.get_selector(Op::Top), Some(c.clone().upcast()));
        assert!(t.is_dock_target_selector(&c));
        t.add_indicator("missing", &scope);
        t.add_selector("missing", &scope);
        assert_eq!(t.selectors_operations().borrow().len(), 1);
        scope.register("wrong", ferroui_base::FerroObject::new());
        t.add_selector("wrong", &scope);
        assert_eq!(t.selectors_operations().borrow().len(), 1);
        assert!(!t.is_dock_target_selector(&Border::new()));
    });
}
#[test]
fn source_same_template_restores_current_visual_parts_before_layout() {
    run(|| {
        for operation in [
            Op::Fill,
            Op::OuterLeft,
            Op::OuterTop,
            Op::OuterRight,
            Op::OuterBottom,
        ] {
            let t = DockTarget::new();
            let (template, selectors, _) = parts(&t, &[(operation, "Center")]);
            let scene = Scene::new(t.clone().upcast());
            assert_eq!(t.get_selector(operation), Some(selectors[0].clone()));
            t.set_template(None);
            assert!(t.get_selector(operation).is_none());
            t.set_template(Some(template));
            assert_eq!(t.get_selector(operation), Some(selectors[0].clone()));
            scene.update();
            assert_eq!(resolve(&t, Point::new(150.0, 100.0)), operation);
        }
    });
}
#[test]
fn source_retemplate_discards_registered_operations_and_manual_parts_clear_on_template_change() {
    run(|| {
        for global in [false, true] {
            let t = target(global);
            let (_, selectors, _) = parts(&t, &[(Op::Right, "Left")]);
            let scene = Scene::new(t.clone().upcast());
            assert_eq!(t.get_selector(Op::Right), Some(selectors[0].clone()));
            assert!(t.get_selector(Op::Left).is_none());
            selectors[0].set_is_visible(false);
            assert_eq!(t.get_selector(Op::Right), Some(selectors[0].clone()));
            selectors[0].set_is_visible(true);
            assert_eq!(resolve(&t, Point::new(150.0, 100.0)), Op::Right);
            t.set_template(Some(FuncControlTemplate::for_type::<DockTargetBase>(
                |_, _| Border::new().upcast(),
            )));
            scene.update();
            assert!(t.get_selector(Op::Right).is_none());
            assert!(t.indicator_operations().borrow().is_empty());
            assert_eq!(
                resolve(&t, Point::default()),
                if global { Op::None } else { Op::Window }
            );
            let scope = NameScope::new();
            scope.register("custom", selectors[0].clone().upcast());
            t.add_selector("custom", &scope);
            assert!(t.get_selector(Op::Right).is_some());
            t.set_template(None);
            assert!(t.get_selector(Op::Right).is_none());
        }
    });
}
#[test]
fn registered_selector_order_controls_result_and_only_winning_indicator_remains_active() {
    run(|| {
        let t = DockTarget::new();
        let (_, selectors, indicators) = parts(
            &t,
            &[(Op::Top, "Top"), (Op::Left, "Left"), (Op::Bottom, "Bottom")],
        );
        let _scene = Scene::new(t.clone().upcast());
        let calls = RefCell::new(Vec::new());
        let validate = |_: Point, op, _, _: &ferroui_base::Visual| {
            calls.borrow_mut().push((op, 'v'));
            true
        };
        let visible = |_: Point, op, _, _: &ferroui_base::Visual| {
            calls.borrow_mut().push((op, 's'));
            true
        };
        assert_eq!(
            t.get_dock_operation(
                Point::new(150.0, 100.0),
                &t,
                &t,
                DragAction::COPY,
                &validate,
                Some(&visible)
            ),
            Op::Left
        );
        // Registration order is the original static part order, not insertion in the test template.
        assert_eq!(
            &*calls.borrow(),
            &[
                (Op::Top, 's'),
                (Op::Top, 'v'),
                (Op::Bottom, 's'),
                (Op::Bottom, 'v'),
                (Op::Left, 's'),
                (Op::Left, 'v')
            ]
        );
        assert_eq!(indicators[0].opacity(), 0.0);
        assert_eq!(indicators[1].opacity(), 0.5);
        assert_eq!(indicators[2].opacity(), 0.0);
        assert!(selectors.iter().all(|s| s.opacity() == 1.0));
    });
}
#[test]
fn direction_flags_disable_all_inner_and_outer_operations_without_calling_policy() {
    run(|| {
        for (operation, part, horizontal) in [
            (Op::Top, "Top", false),
            (Op::Bottom, "Bottom", false),
            (Op::Left, "Left", true),
            (Op::Right, "Right", true),
            (Op::OuterLeft, "OuterLeft", true),
            (Op::OuterTop, "OuterTop", false),
            (Op::OuterRight, "OuterRight", true),
            (Op::OuterBottom, "OuterBottom", false),
        ] {
            let t = DockTarget::new();
            let (_, selectors, indicators) = parts(&t, &[(operation, part)]);
            let _scene = Scene::new(t.clone().upcast());
            if horizontal {
                t.set_show_horizontal_targets(false);
            } else {
                t.set_show_vertical_targets(false);
            }
            let calls = Cell::new(0);
            let policy = |_, _, _, _: &ferroui_base::Visual| {
                calls.set(calls.get() + 1);
                true
            };
            assert_eq!(
                t.get_dock_operation(
                    Point::new(150.0, 100.0),
                    &t,
                    &t,
                    DragAction::MOVE,
                    &policy,
                    Some(&policy)
                ),
                Op::Window
            );
            assert_eq!(calls.get(), 0);
            assert_eq!(selectors[0].opacity(), 0.0);
            assert_eq!(indicators[0].opacity(), 0.0);
        }
    });
}
#[test]
fn visibility_rejects_before_validation_and_validation_requires_selector_hit() {
    run(|| {
        let t = DockTarget::new();
        let (_, selectors, indicators) = parts(&t, &[(Op::Fill, "Center")]);
        let _scene = Scene::new(t.clone().upcast());
        let calls = Cell::new(0);
        let validate = |_, _, _, _: &ferroui_base::Visual| {
            calls.set(calls.get() + 1);
            true
        };
        assert_eq!(
            t.get_dock_operation(
                Point::new(150.0, 100.0),
                &t,
                &t,
                DragAction::MOVE,
                &validate,
                Some(&|_, _, _, _| false)
            ),
            Op::Window
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(selectors[0].opacity(), 0.0);
        assert_eq!(
            t.get_dock_operation(
                Point::new(-20.0, -20.0),
                &t,
                &t,
                DragAction::MOVE,
                &validate,
                None
            ),
            Op::Window
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(selectors[0].opacity(), 1.0);
        assert_eq!(indicators[0].opacity(), 0.0);
        assert_eq!(
            t.get_dock_operation(
                Point::new(150.0, 100.0),
                &t,
                &t,
                DragAction::MOVE,
                &|_, _, _, _| false,
                None
            ),
            Op::Window
        );
        assert_eq!(indicators[0].opacity(), 0.0);
    });
}
#[test]
fn indicators_only_bypasses_validate_and_hit_testing_but_applies_visibility_and_direction() {
    run(|| {
        let t = DockTarget::new();
        let (_, selectors, indicators) = parts(&t, &[(Op::Left, "Left"), (Op::Top, "Top")]);
        let _scene = Scene::new(t.clone().upcast());
        t.set_show_indicators_only(true);
        ferrodock_settings::DockProperties::set_indicator_dock_operation(&t, Op::Left);
        let validate =
            |_, _, _, _: &ferroui_base::Visual| panic!("source indicators-only never validates");
        assert_eq!(
            t.get_dock_operation(
                Point::new(-1000.0, -1000.0),
                &t,
                &t,
                DragAction::LINK,
                &validate,
                None
            ),
            Op::Left
        );
        assert_eq!(indicators[0].opacity(), 0.5);
        assert_eq!(indicators[1].opacity(), 0.0);
        assert_eq!(
            t.get_dock_operation(
                Point::default(),
                &t,
                &t,
                DragAction::MOVE,
                &validate,
                Some(&|_, _, _, _| false)
            ),
            Op::Window
        );
        t.set_show_horizontal_targets(false);
        assert_eq!(
            t.get_dock_operation(Point::default(), &t, &t, DragAction::MOVE, &validate, None),
            Op::Window
        );
        assert_eq!(indicators[0].opacity(), 0.0);
        assert_eq!(selectors[0].opacity(), 0.0);
    });
}
#[test]
fn template_restore_skips_parts_removed_from_current_visual_tree() {
    run(|| {
        let t = DockTarget::new();
        let (template, selectors, _) = parts(&t, &[(Op::Fill, "Center")]);
        let _scene = Scene::new(t.clone().upcast());
        let parent = selectors[0]
            .visual_parent()
            .unwrap()
            .cast::<Panel>()
            .unwrap();
        parent.children().remove(&selectors[0]);
        t.set_template(None);
        t.set_template(Some(template));
        assert!(t.get_selector(Op::Fill).is_none());
    });
}
#[test]
fn source_reset_clears_all_parts_and_flags_but_preserves_indicators_only_and_margin() {
    run(|| {
        let t = DockTarget::new();
        let (_, selectors, indicators) = parts(&t, &[(Op::Fill, "Center")]);
        let _scene = Scene::new(t.clone().upcast());
        t.set_show_indicators_only(true);
        t.set_target_margin(Thickness::uniform(12.0));
        t.set_show_horizontal_targets(false);
        t.set_show_vertical_targets(false);
        t.set_is_global_dock_available(true);
        t.set_is_global_dock_active(true);
        selectors[0].set_opacity(0.7);
        indicators[0].set_opacity(0.5);
        let contract = ferroui_base::metadata::from_markup_value::<Rc<dyn IDockTarget>>(&Some(
            Control::boxed(t.clone()),
        ))
        .unwrap();
        contract.reset();
        assert!(t.show_indicators_only());
        assert_eq!(t.target_margin(), Thickness::uniform(12.0));
        assert!(t.show_horizontal_targets());
        assert!(t.show_vertical_targets());
        assert!(!t.is_global_dock_available());
        assert!(!t.is_global_dock_active());
        assert_eq!(selectors[0].opacity(), 0.0);
        assert_eq!(indicators[0].opacity(), 0.0);
    });
}
struct Provider(Ref<Control>);
impl IDockAdornerHostProvider for Provider {
    fn get_adorner_host(&self, _: &ferroui_base::Visual) -> Option<Ref<Control>> {
        Some(self.0.clone())
    }
}
#[test]
fn inherited_provider_and_theme_keep_native_identity_and_margin_is_readonly_notifying() {
    run(|| {
        let parent = Border::new();
        let child = DockTarget::new();
        parent.set_child(Some(child.clone().upcast()));
        let _scene = Scene::new(parent.clone().upcast());
        let provider: Rc<dyn IDockAdornerHostProvider> = Rc::new(Provider(parent.clone().upcast()));
        let theme = ControlTheme::new();
        DockTarget::set_adorner_host_provider(&parent, Some(provider.clone()));
        DockTarget::set_target_theme(&parent, Some(theme.clone()));
        let inherited = DockTarget::get_adorner_host_provider(&child).unwrap();
        assert!(Rc::ptr_eq(&provider, &inherited));
        assert_eq!(
            inherited.get_adorner_host(&child),
            Some(parent.clone().upcast())
        );
        assert_eq!(DockTarget::get_target_theme(&child), Some(theme));
        DockTarget::set_adorner_host_provider(&child, None);
        assert!(DockTarget::get_adorner_host_provider(&child).is_none());
        child.clear_value(DockTarget::adorner_host_provider_property());
        assert!(DockTarget::get_adorner_host_provider(&child).is_some());
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        let subscription = child.property_changed(move |e| {
            if e.property() == DockTarget::target_margin_property().as_property() {
                count.set(count.get() + 1);
            }
        });
        child.set_target_margin(Thickness::uniform(4.0));
        child.set_target_margin(Thickness::uniform(4.0));
        assert_eq!(changes.get(), 1);
        assert!(DockTarget::target_margin_property().is_read_only());
        drop(subscription);
    });
}
#[test]
fn native_peer_exposes_original_class_name_state_visibility_and_invoke_contract() {
    run(|| {
        for global in [false, true] {
            let t = target(global);
            let _scene = Scene::new(t.clone().upcast());
            let peer = ControlAutomationPeer::create_peer_for_element(&t);
            assert_eq!(
                peer.get_class_name(),
                if global {
                    "GlobalDockTarget"
                } else {
                    "DockTarget"
                }
            );
            assert_eq!(
                peer.get_name(),
                if global {
                    "Global dock target"
                } else {
                    "Dock target"
                }
            );
            assert_eq!(
                peer.get_automation_control_type(),
                AutomationControlType::Pane
            );
            assert!(peer.get_help_text().contains("Horizontal=true"));
            t.set_show_horizontal_targets(false);
            assert!(peer.get_help_text().contains("Horizontal=false"));
            AutomationProperties::set_name(&t, Some("Explicit"));
            AutomationProperties::set_automation_id(&t, Some("target-id"));
            assert_eq!(peer.get_name(), "Explicit");
            assert_eq!(peer.get_automation_id().as_deref(), Some("target-id"));
            let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
            invoke.invoke().unwrap();
            t.set_is_visible(false);
            assert!(!peer.is_control_element());
            assert!(!peer.is_content_element());
        }
    });
}
#[repr(C)]
struct CustomTarget {
    base: DockTargetBase,
}
ferro_class!(CustomTarget:DockTargetBase);
ferroui_base::ferro_class_info!(CustomTarget {});
ferro_impl_classes!(CustomTarget:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl DockTargetBaseImpl for CustomTarget {
    fn default_dock_operation(_this: &Self) -> Op {
        Op::Left
    }
}
#[test]
fn native_virtual_default_and_replaced_protected_dictionaries_preserve_extension_points() {
    run(|| {
        let custom = instantiate(CustomTarget {
            base: DockTargetBase::construct(),
        });
        let base: Ref<DockTargetBase> = custom.clone().upcast();
        assert_eq!(resolve(&base, Point::default()), Op::Left);
        let map = Rc::new(RefCell::new(
            ferroui_system::collections::generic::Dictionary::new(),
        ));
        base.set_indicator_operations(map.clone());
        let other = Rc::new(RefCell::new(
            ferroui_system::collections::generic::Dictionary::new(),
        ));
        base.set_selectors_operations(other.clone());
        assert!(Rc::ptr_eq(&map, &base.indicator_operations()));
        assert!(Rc::ptr_eq(&other, &base.selectors_operations()));
    });
}
#[test]
fn nested_transform_and_separate_window_coordinates_resolve_original_selector() {
    run(|| {
        let t = DockTarget::new();
        let (_, selectors, _) = parts(&t, &[(Op::Fill, "Center")]);
        let target_scene = Scene::new(t.clone().upcast());
        let mut builder = TransformOperationsBuilder::new(1);
        builder.append_translate(20.0, 10.0);
        selectors[0].set_render_transform(Some(builder.build()));
        target_scene.update();
        let p = selectors[0]
            .translate_point(Point::new(25.0, 25.0), &t)
            .unwrap();
        assert_eq!(resolve(&t, p), Op::Fill);
        let source = Border::new();
        let source_scene = Scene::new(source.clone().upcast());
        let screen = selectors[0].point_to_screen(Point::new(25.0, 25.0));
        let relative = source.point_to_client(screen);
        assert_eq!(
            t.get_dock_operation(
                relative,
                &t,
                &source,
                DragAction::MOVE,
                &|_, _, _, _| true,
                None
            ),
            Op::Fill
        );
        source_scene.0.close();
    });
}
#[test]
fn detached_targets_and_old_parts_release_while_window_remains_alive() {
    run(|| {
        for global in [false, true] {
            let t = target(global);
            let (_, selectors, indicators) = parts(&t, &[(Op::Fill, "Center")]);
            let scene = Scene::new(t.clone().upcast());
            t.set_show_horizontal_targets(false);
            t.set_show_vertical_targets(false);
            t.set_is_global_dock_available(true);
            t.set_is_global_dock_active(true);
            t.set_show_indicators_only(true);
            let weak = t.downgrade();
            scene.0.set_content(None);
            scene.update();
            drop(t);
            drop(selectors);
            drop(indicators);
            Dispatcher::ui_thread().run_jobs(None);
            assert!(weak.upgrade().is_none());
            assert!(scene.0.is_visible());
        }
    });
}
struct WindowFactory {
    calls: Cell<usize>,
}
impl IDockAdornerWindowFactory for WindowFactory {
    fn create_window(
        &self,
        _: &ferroui_base::Visual,
        _: &Control,
    ) -> Option<Ref<DockAdornerWindow>> {
        self.calls.set(self.calls.get() + 1);
        None
    }
}
#[test]
fn adorner_window_factory_is_inherited_and_native_peer_remains_decorative() {
    run(|| {
        let parent = Border::new();
        let child = DockTarget::new();
        parent.set_child(Some(child.clone().upcast()));
        let _scene = Scene::new(parent.clone().upcast());
        let factory = Rc::new(WindowFactory {
            calls: Cell::new(0),
        });
        let erased: Rc<dyn IDockAdornerWindowFactory> = factory.clone();
        DockAdornerWindow::set_factory(&parent, Some(erased.clone()));
        let inherited = DockAdornerWindow::get_factory(&child).unwrap();
        assert!(Rc::ptr_eq(&erased, &inherited));
        assert!(inherited.create_window(&child, &child).is_none());
        assert_eq!(factory.calls.get(), 1);
        DockAdornerWindow::set_factory(&child, None);
        assert!(DockAdornerWindow::get_factory(&child).is_none());
        child.clear_value(DockAdornerWindow::factory_property());
        assert!(DockAdornerWindow::get_factory(&child).is_some());
        let host = DockAdornerWindow::new();
        host.set_title(Some("".into()));
        let peer = ControlAutomationPeer::create_peer_for_element(&host);
        assert_eq!(peer.get_class_name(), "DockAdornerWindow");
        assert_eq!(peer.get_name(), "Dock adorner window");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert!(!peer.is_control_element());
        assert!(!peer.is_content_element());
        assert!(peer.get_help_text().contains("Visible=false"));
        assert!(peer.get_help_text().contains("WindowState=Normal"));
        host.show();
        assert!(peer.get_help_text().contains("Visible=true"));
        host.close();
    });
}
