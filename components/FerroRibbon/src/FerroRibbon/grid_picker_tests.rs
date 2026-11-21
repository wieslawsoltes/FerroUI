use super::*;
use crate::controls::{IRibbonItemOwner, Ribbon};
use ferroui_base::{
    Point, Rect, Size, StaticType,
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    input::{
        KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType,
        TappedEventArgs,
    },
    reactive::{Disposable, IDisposable},
    styling::ControlTheme,
};
use ferroui_controls::{PlacementMode, TopLevel, Window, automation::peers::AutomationControlType};
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/grid-picker.json")).unwrap()
}
fn cases(name: &str) -> Vec<Value> {
    corpus()["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["name"] == name)
        .cloned()
        .collect()
}
fn state(p: &RibbonGridPicker) -> Value {
    let on = RibbonTheme::get_brush(Some(p), Some("RibbonAccentSubtleStrongBrush"))
        .unwrap()
        .map(Rc::<dyn ferroui_base::media::IBrush>::from);
    let active = p
        .grid
        .children()
        .to_vec()
        .into_iter()
        .filter(|c| c.cast::<Border>().unwrap().background() == on)
        .count();
    let peer = ControlAutomationPeer::create_peer_for_element(p);
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::Group
    );
    assert!(peer.is_control_element());
    assert!(peer.is_content_element());
    json!({"Rows":p.rows(),"Columns":p.columns(),"hover":p.highlighted_size().to_string(),"caption":p.caption.text(),"AutomationName":peer.get_name(),"rowDefinitions":p.grid.row_definitions().count(),"columnDefinitions":p.grid.column_definitions().count(),"cells":p.grid.children().count(),"active":active})
}
fn theme(p: &RibbonGridPicker) {
    let v = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/GridPicker.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*v)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let t = d
        .try_get_resource(&<RibbonGridPicker as StaticType>::TYPE.into(), None)
        .flatten()
        .unwrap();
    let t = ValueTypes::as_object(&*t)
        .unwrap()
        .cast::<ControlTheme>()
        .unwrap();
    p.set_resources(d);
    p.set_theme(t);
}
fn key(p: &RibbonGridPicker, k: Key) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = k;
    e.set_routed_event(Some(InputElement::key_down_event()));
    p.raise_event(&e);
    e.handled()
}
#[test]
fn original_grid_highlight_build_negative_dimensions_and_keyboard_states() {
    crate::group_tests::run(|| {
        let p = RibbonGridPicker::new();
        let mut states = vec![state(&p)];
        for size in [
            RibbonGridSize::new(2, 3),
            RibbonGridSize::new(99, 99),
            RibbonGridSize::new(0, 4),
            RibbonGridSize::new(-1, 2),
        ] {
            p.highlight(size);
            states.push(state(&p));
        }
        p.set_rows(0);
        states.push(state(&p));
        p.set_columns(-2);
        states.push(state(&p));
        p.set_rows(3);
        p.set_columns(4);
        states.push(state(&p));
        assert_eq!(json!(states), cases("highlight_build")[0]["states"]);
        for source in cases("keyboard") {
            let p = RibbonGridPicker::new();
            p.set_rows(source["rows"].as_i64().unwrap() as i32);
            p.set_columns(source["columns"].as_i64().unwrap() as i32);
            let mut states = vec![];
            for (name, k) in [
                ("Right", Key::Right),
                ("Down", Key::Down),
                ("Left", Key::Left),
                ("Up", Key::Up),
                ("Home", Key::Home),
                ("Escape", Key::Escape),
                ("Enter", Key::Enter),
                ("Space", Key::Space),
            ] {
                let handled = key(&p, k);
                states.push(json!({"key":name,"Handled":handled,"state":state(&p)}));
            }
            assert_eq!(json!(states), source["states"]);
        }
        let p = RibbonGridPicker::new();
        let mut values = vec![];
        p.highlight(RibbonGridSize::new(i32::MAX, i32::MAX));
        key(&p, Key::Right);
        values.push(p.highlighted_size().to_string());
        key(&p, Key::Down);
        values.push(p.highlighted_size().to_string());
        p.highlight(RibbonGridSize::new(i32::MIN, i32::MIN));
        key(&p, Key::Left);
        values.push(p.highlighted_size().to_string());
        key(&p, Key::Up);
        values.push(p.highlighted_size().to_string());
        assert_eq!(json!(values), cases("unchecked_edges")[0]["values"]);
    });
}
#[test]
fn original_grid_focus_pointer_and_retained_root_retemplate() {
    crate::group_tests::run(|| {
        let p = RibbonGridPicker::new();
        p.set_rows(2);
        p.set_columns(3);
        let mut states = vec![];
        for navigation in [NavigationMethod::Unspecified, NavigationMethod::Tab] {
            let mut e = FocusChangedEventArgs::new(InputElement::got_focus_event());
            e.navigation_method = navigation;
            p.raise_event(&e);
            states.push(state(&p));
        }
        p.highlight(RibbonGridSize::new(2, 2));
        let mut e = FocusChangedEventArgs::new(InputElement::got_focus_event());
        e.navigation_method = NavigationMethod::Tab;
        p.raise_event(&e);
        states.push(state(&p));
        assert_eq!(json!(states), cases("focus")[0]["states"]);
        p.highlight(RibbonGridSize::new(0, 0));
        let cell = p.grid.children().get(4);
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_entered_event()),
            cell.clone(),
            pointer,
            None,
            Point::default(),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
        );
        cell.raise_event(&e);
        assert_eq!(state(&p), cases("pointer")[0]["hover"]);
        let picked = Rc::new(RefCell::new(vec![]));
        let out = picked.clone();
        p.add_size_picked(Some(MarkupDelegate::new(move |a| {
            out.borrow_mut().push(
                a[1].as_ref()
                    .unwrap()
                    .downcast_ref::<RibbonGridSize>()
                    .unwrap()
                    .to_string(),
            );
            None
        })));
        cell.raise_event(&TappedEventArgs::new(
            Some(InputElement::tapped_event()),
            &e,
        ));
        assert_eq!(json!(*picked.borrow()), cases("pointer")[0]["picks"]);
        assert_eq!(state(&p), cases("pointer")[0]["state"]);
        let logical = ferroui_controls::ContentControl::new();
        logical.set_content(Some(Control::boxed(p.clone())));
        assert!(p.visual_parent().is_none());
        assert!(!logical.is_visual_ancestor_of(&p));
        assert_eq!(
            p.is_within(&logical),
            cases("logical_only_ancestor")[0]["within"]
                .as_bool()
                .unwrap()
        );
        logical.set_content(None);
        theme(&p);
        let w = Window::new();
        w.set_content(Some(Control::boxed(p.clone())));
        w.show();
        w.measure(Size::new(300., 300.));
        w.arrange(Rect::new(0., 0., 300., 300.));
        let root = p.root.clone();
        let old = root.visual_parent().unwrap().cast::<Border>().unwrap();
        let first_cell = p.grid.children().get(0);
        p.highlight(RibbonGridSize::new(2, 3));
        // A second compiled dictionary supplies a distinct actual template.
        let v =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/GridPicker.xaml")
                .unwrap()
                .unwrap();
        let d = ValueTypes::as_object(&*v)
            .unwrap()
            .cast::<ResourceDictionary>()
            .unwrap();
        let v = d
            .try_get_resource(&<RibbonGridPicker as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap();
        let next = ValueTypes::as_object(&*v)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap();
        p.set_theme(next);
        p.apply_template();
        w.update_layout();
        assert!(old.child().is_none());
        assert!(!root.visual_parent().unwrap().ptr_eq(&old));
        assert!(p.root.ptr_eq(&root));
        assert!(p.grid.children().get(0).ptr_eq(&first_cell));
        assert_eq!(p.highlighted_size(), RibbonGridSize::new(2, 3));
        w.close();
    });
}

#[test]
fn standard_grid_flyout_receives_raw_keys_in_overlay_and_separate_roots() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::standard_grid_flyout_receives_raw_keys_in_overlay_and_separate_roots"
    )) {
        return;
    }
    use ferroui_base::{
        input::{PhysicalKey, RawInputModifiers},
        threading::{CancellationToken, Dispatcher},
    };
    use ferroui_controls::{AppBuilder, Application, Flyout};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
        HeadlessWindowExtensions,
    };
    fn overlay_app() -> AppBuilder {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    }
    fn separate_app() -> AppBuilder {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions {
            overlay_popups: false,
            ..Default::default()
        })
    }
    for overlay in [true, false] {
        let session =
            HeadlessUnitTestSession::start_new(if overlay { overlay_app } else { separate_app });
        let result = session
            .dispatch(
                move || {
                    crate::register_types();
                    Application::current()
                        .unwrap()
                        .styles()
                        .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                    for opening_key in [None, Some(Key::Space), Some(Key::Enter)] {
                        let picker = RibbonGridPicker::new();
                        theme(&picker);
                        let flyout = Flyout::new();
                        flyout.set_content(Some(Control::boxed(picker.clone())));
                        let button = crate::controls::RibbonDropDownButton::new();
                        button.set_label(Some("Table".into()));
                        button.set_flyout(flyout.clone());
                        let window = Window::new();
                        window.set_width(600.);
                        window.set_height(400.);
                        window.set_content(Some(Control::boxed(button.clone())));
                        window.show();
                        window.measure(Size::new(600., 400.));
                        window.arrange(Rect::new(0., 0., 600., 400.));
                        Dispatcher::ui_thread().run_jobs(None);
                        let closed = Rc::new(std::cell::Cell::new(0));
                        let out = closed.clone();
                        window.closed(move || out.set(out.get() + 1)).detach();
                        let picked = Rc::new(RefCell::new(Vec::new()));
                        let out = picked.clone();
                        picker.add_size_picked(Some(MarkupDelegate::new(move |args| {
                            out.borrow_mut().push(
                                *args[1]
                                    .as_ref()
                                    .unwrap()
                                    .downcast_ref::<RibbonGridSize>()
                                    .unwrap(),
                            );
                            None
                        })));
                        for cycle in 0..2 {
                            assert!(button.focus());
                            if let Some(key) = opening_key {
                                // Exercise the actual Button input route, including its
                                // key-up activation, before inspecting popup focus.
                                window.key_press(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                                window.key_release(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                            } else {
                                flyout.show_at(&button);
                            }
                            Dispatcher::ui_thread().run_jobs(None);
                            assert!(flyout.is_open());
                            assert!(
                                picker.is_focused(),
                                "picker focus, overlay={overlay}, opening={opening_key:?}, cycle={cycle}; focused={:?}",
                                window
                                    .focus_manager()
                                    .get_focused_element()
                                    .map(|c| c.get_type().name())
                            );
                            let top = TopLevel::get_top_level(Some(&picker)).unwrap();
                            assert_eq!(
                                top.cast::<ferroui_controls::primitives::PopupRoot>()
                                    .is_none(),
                                overlay
                            );
                            for key in [Key::Right, Key::Down, Key::Enter] {
                                top.key_press(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                                // Enter disposes a separate PopupRoot synchronously. Its
                                // following key-up belongs to the restored owner window.
                                if flyout.is_open() {
                                    top.key_release(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                                } else {
                                    window.key_release(
                                        key,
                                        RawInputModifiers::NONE,
                                        PhysicalKey::None,
                                        None,
                                    );
                                }
                                Dispatcher::ui_thread().run_jobs(None);
                            }
                            assert_eq!(
                                *picked.borrow(),
                                vec![RibbonGridSize::new(2, 2); cycle + 1],
                                "exactly one pick per opening, overlay={overlay}, opening={opening_key:?}, cycle={cycle}"
                            );
                            assert!(!flyout.is_open());
                            assert_eq!(picker.highlighted_size(), RibbonGridSize::new(0, 0));
                            assert_eq!(
                                closed.get(),
                                0,
                                "picker input cannot close its owning window"
                            );
                            assert!(
                                Control::from_boxed(&flyout.content().unwrap())
                                    .unwrap()
                                    .ptr_eq(&picker)
                            );
                        }
                        window.close();
                        window.set_content(None);
                        Dispatcher::ui_thread().run_jobs(None);
                    }
                },
                CancellationToken::none(),
            )
            .wait();
        session.dispose();
        result.unwrap();
    }
}
struct TraceCommand {
    trace: Rc<RefCell<Vec<String>>>,
    allowed: bool,
}
impl ICommand for TraceCommand {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        self.trace.borrow_mut().push(format!(
            "can:{}",
            p.unwrap().downcast_ref::<RibbonGridSize>().unwrap()
        ));
        self.allowed
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        self.trace.borrow_mut().push(format!(
            "execute:{}",
            p.unwrap().downcast_ref::<RibbonGridSize>().unwrap()
        ));
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
#[test]
fn original_grid_pick_order_captured_owner_command_replacement_and_popup_close() {
    crate::group_tests::run(|| {
        for source in cases("pick_order") {
            let p = RibbonGridPicker::new();
            p.set_rows(2);
            p.set_columns(3);
            theme(&p);
            let target = Border::new();
            let w = Window::new();
            w.set_content(Some(Control::boxed(target.clone())));
            w.show();
            let popup = Popup::new();
            popup.set_placement_target(Some(target.upcast::<Control>()));
            popup.set_placement(PlacementMode::Bottom);
            // Source fixture has an attached XamlRoot. Native standalone Popup
            // requires an explicit logical parent (framework popup_tests.rs).
            popup.set_parent(popup.placement_target().unwrap());
            popup.set_child(Some(p.clone().upcast::<Control>()));
            popup.set_is_open(true);
            w.update_layout();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            let host = popup.host().expect("open popup host");
            if let Some(c) = host.as_control().cast::<TemplatedControl>() {
                c.apply_template();
            }

            host.as_control().measure(Size::new(300., 300.));
            host.as_control().arrange(Rect::new(0., 0., 300., 300.));
            assert!(
                TopLevel::get_top_level(Some(&p)).is_some(),
                "picker must be attached before testing CloseHostPopup"
            );
            assert!(popup.is_inside_popup(&p));

            let r = Ribbon::new();
            let owner = ValueTypes::try_cast(
                &Control::boxed(r.clone()),
                ValueType::of::<Rc<dyn IRibbonItemOwner>>(),
            )
            .unwrap();
            RibbonItemHelper::set_owner(
                &p,
                Some(
                    owner
                        .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
                        .unwrap()
                        .clone(),
                ),
            )
            .unwrap();
            let trace = Rc::new(RefCell::new(vec![]));
            let out = trace.clone();
            r.add_item_invoked(Some(crate::root_command_tests::invocation(move |a| {
                out.borrow_mut().push(format!(
                    "owner:{}",
                    a.parameter
                        .as_ref()
                        .unwrap()
                        .downcast_ref::<RibbonGridSize>()
                        .unwrap()
                ))
            })));
            let out = trace.clone();
            popup
                .closed(move || out.borrow_mut().push("close".to_owned()))
                .detach();
            let out = trace.clone();
            let weak = p.downgrade();
            let allowed = source["allowed"].as_bool().unwrap();
            p.add_size_picked(Some(MarkupDelegate::new(move |a| {
                let p = weak.upgrade().unwrap();
                out.borrow_mut().push(format!(
                    "picked:{}",
                    a[1].as_ref()
                        .unwrap()
                        .downcast_ref::<RibbonGridSize>()
                        .unwrap()
                ));
                RibbonItemHelper::set_owner(&p, None).unwrap();
                p.set_command(Some(Rc::new(TraceCommand {
                    trace: out.clone(),
                    allowed,
                })));
                None
            })));
            p.pick(RibbonGridSize::new(4, -2));
            assert_eq!(json!(*trace.borrow()), source["trace"]);
            assert_eq!(popup.is_open(), source["IsOpen"].as_bool().unwrap());
            assert_eq!(state(&p), source["state"]);
            w.close();
        }
    });
}

struct CoreTraceCommand(Rc<RefCell<Vec<String>>>);
impl ferroribbon_core::commands::ICommand for CoreTraceCommand {
    fn can_execute(&self, p: Option<&dyn std::any::Any>) -> bool {
        self.0.borrow_mut().push(format!(
            "can:{}",
            p.unwrap().downcast_ref::<RibbonGridSize>().unwrap()
        ));
        true
    }
    fn execute(&self, p: Option<&dyn std::any::Any>) {
        self.0.borrow_mut().push(format!(
            "execute:{}",
            p.unwrap().downcast_ref::<RibbonGridSize>().unwrap()
        ));
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
#[test]
fn original_grid_factory_binding_command_order_and_disposal() {
    crate::group_tests::run(|| {
        use crate::mvvm::{RibbonElementFactory, RibbonModelBindings};
        use ferroribbon_core::model::*;
        let source = cases("factory")[0].clone();
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonGridPickerModel::new(Some("grid".into()), Some("Table".into()));
        m.set_rows(3);
        m.set_columns(4);
        let e = f
            .create_grid_picker(m.clone(), b.clone())
            .cast::<crate::controls::RibbonDropDownButton>()
            .unwrap();
        let flyout = e
            .flyout()
            .unwrap()
            .cast::<ferroui_controls::Flyout>()
            .unwrap();
        let p = Control::from_boxed(&flyout.content().unwrap())
            .unwrap()
            .cast::<RibbonGridPicker>()
            .unwrap();
        theme(&p);
        assert_eq!(state(&p), source["initial"]);
        m.set_rows(0);
        m.set_columns(2);
        assert_eq!(state(&p), source["changed"]);
        let w = Window::new();
        w.set_content(Some(Control::boxed(e.clone())));
        w.show();
        w.update_layout();
        let trace = Rc::new(RefCell::new(vec![]));
        m.set_command(Some(Rc::new(CoreTraceCommand(trace.clone()))));
        let open = Rc::new(RefCell::new(vec![]));
        let out = open.clone();
        let weak = flyout.downgrade();
        p.add_size_picked(Some(MarkupDelegate::new(move |_| {
            out.borrow_mut().push(weak.upgrade().unwrap().is_open());
            None
        })));
        flyout.show_at(&e);
        p.pick(RibbonGridSize::new(2, 3));
        b.dispose();
        m.set_rows(8);
        m.set_command(None);
        flyout.show_at(&e);
        p.pick(RibbonGridSize::new(1, 1));
        assert_eq!(json!(*open.borrow()), source["openAtEvent"]);
        assert_eq!(json!(*trace.borrow()), source["trace"]);
        assert_eq!(state(&p), source["disposed"]);
        w.close();
    });
}
