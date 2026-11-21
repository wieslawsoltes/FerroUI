use super::*;
use crate::controls::{RibbonButton, RibbonContextualTabGroup, RibbonQuickAccessToolBar};
use ferroui_base::data::core::ValueTypes;
use serde_json::{Value, json};
fn menu_controls(menu: &MenuFlyout) -> Vec<Ref<Control>> {
    menu.items()
        .iter()
        .map(|v| {
            ValueTypes::as_object(&*v.unwrap())
                .unwrap()
                .cast::<Control>()
                .unwrap()
        })
        .collect()
}
fn describe(item: &Control) -> Value {
    if item.is::<MenuFlyoutSeparator>() {
        return json!({"kind":"separator","text":null,"glyph":null,"checked":null,"children":null});
    }
    if let Some(item) = item.to_ref().cast::<MenuFlyoutSubItem>() {
        return json!({"kind":"submenu","text":item.text(),"glyph":null,"checked":null,"children":item.items().iter().flatten().map(|item|describe(&item)).collect::<Vec<_>>()});
    }
    let item = item.to_ref().cast::<MenuFlyoutItem>().unwrap();
    let toggle = item.cast::<ToggleMenuFlyoutItem>();
    json!({"kind":if toggle.is_some(){"toggle"}else{"item"},"text":item.text(),"glyph":item.icon().and_then(|i|ValueTypes::as_object(&*i)).and_then(|i|i.cast::<crate::compat::FontIcon>()).and_then(|i|i.glyph()),"checked":toggle.map(|t|t.is_checked()),"children":null})
}
fn sample() -> (
    Ref<Ribbon>,
    Ref<RibbonTab>,
    Ref<RibbonGroup>,
    Ref<RibbonButton>,
) {
    let root = Ribbon::new();
    let tab = RibbonTab::new();
    tab.set_id(Some("home".into()));
    tab.set_header(Some("Home".into()));
    let group = RibbonGroup::new();
    group.set_id(Some("edit".into()));
    group.set_header(Some("Edit".into()));
    let button = RibbonButton::new();
    button.set_id(Some("save".into()));
    button.set_label(Some("Save".into()));
    group.items().add(Some(button.clone().upcast()));
    tab.groups().add(Some(group.clone()));
    root.tabs().add(Some(tab.clone()));
    root.set_selected_tab(Some(tab.clone()));
    root.set_quick_access_tool_bar(Some(RibbonQuickAccessToolBar::new()));
    (root, tab, group, button)
}
type CapturedMenu = Rc<RefCell<Option<Ref<MenuFlyout>>>>;
fn capture(root: &Ribbon) -> (CapturedMenu, Rc<dyn IDisposable>) {
    let slot = Rc::new(RefCell::new(None));
    let observed = slot.clone();
    let token = root.context_menu_opening(move |args| {
        observed.replace(Some(args.menu.clone()));
        args.handled.set(true);
    });
    (slot, token)
}
#[test]
fn context_menu_native_builders_match_480_unchanged_source_snapshots() {
    crate::group_tests::run(|| {
        let app = ferroui_controls::Application::current().unwrap();
        let resources = crate::theming::RibbonThemeResources::new();
        let provider: Rc<dyn ferroui_base::controls::IResourceProvider> = resources
            .upcast::<ferroui_base::controls::ResourceDictionary>()
            .into();
        app.resources().merged_dictionaries().add(provider.clone());
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/context-menu.json")).unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 480);
        for case in cases {
            let (root, tab, group, source) = sample();
            root.set_can_customize(case["customize"].as_bool().unwrap());
            root.set_is_collapsible(case["collapsible"].as_bool().unwrap());
            let mode = case["mode"].as_i64().unwrap() as i32;
            root.set_visibility_mode(RibbonVisibilityMode::from_raw(mode));
            root.set_can_float_groups(case["canFloat"].as_bool().unwrap());
            root.set_is_visibility_menu_enabled(case["visibility"].as_bool().unwrap());
            root.set_quick_access_position(if mode % 2 == 0 {
                RibbonQuickAccessPosition::AboveRibbon
            } else {
                RibbonQuickAccessPosition::BelowRibbon
            });
            group.set_is_floating(case["floating"].as_bool().unwrap());
            let other = RibbonGroup::new();
            other.set_id(Some("other".into()));
            other.set_is_floating(true);
            tab.groups().add(Some(other));
            tab.groups().add(Some(RibbonGroup::new()));
            let ctx = RibbonContextualTabGroup::new();
            ctx.set_id(Some("tools".into()));
            root.contextual_groups().add(Some(ctx));
            let contextual = RibbonTab::new();
            contextual.set_id(Some("ctx".into()));
            contextual.set_header(Some("Context".into()));
            contextual.set_contextual_group_id(Some("tools".into()));
            root.tabs().add(Some(contextual));
            root.tabs().add(Some(RibbonTab::new()));
            root.set_selected_tab(Some(tab));
            root.customization()
                .hidden_tab_ids()
                .unwrap()
                .add(Some("home".into()));
            root.customization()
                .hidden_group_ids()
                .unwrap()
                .add(Some("edit".into()));
            if mode % 2 != 0 {
                assert!(root.add_to_quick_access(&source));
            }
            let element = match case["kind"].as_str().unwrap() {
                "item" => source.clone().upcast(),
                "linked" => {
                    let linked = RibbonButton::new();
                    RibbonItemHelper::set_source_item(&linked, source.clone().upcast::<Control>());
                    linked.upcast()
                }
                _ => ferroui_controls::Border::new().upcast(),
            };
            let (slot, token) = capture(&root);
            let result = root
                .show_item_context_menu(element, Point::new(12.25, 18.75))
                .unwrap();
            assert_eq!(result, case["result"].as_bool().unwrap());
            let menu = slot.borrow().clone().unwrap();
            assert_eq!(
                json!(
                    menu_controls(&menu)
                        .iter()
                        .map(|item| describe(item))
                        .collect::<Vec<_>>()
                ),
                case["rows"],
                "{case}"
            );
            assert_eq!(
                menu.flyout_presenter_theme().is_some(),
                case["themed"].as_bool().unwrap()
            );
            token.dispose();
        }
        app.resources().merged_dictionaries().remove(&provider);
    });
}
#[test]
fn context_menu_native_interception_disabled_order_empty_and_visibility_clone() {
    crate::group_tests::run(|| {
        let (root, tab, _, source) = sample();
        assert!(root.is_context_menu_enabled());
        assert!(root.can_customize());
        assert!(!root.is_visibility_menu_enabled());
        let calls = Rc::new(RefCell::new(Vec::new()));
        let seen = calls.clone();
        let a = root.context_menu_opening(move |args| {
            seen.borrow_mut().push(args.handled.get());
            args.handled.set(true);
        });
        let seen = calls.clone();
        let b = root.context_menu_opening(move |args| seen.borrow_mut().push(args.handled.get()));
        root.set_is_context_menu_enabled(false);
        assert!(
            !root
                .show_item_context_menu(source.clone().upcast(), Point::default())
                .unwrap()
        );
        assert!(calls.borrow().is_empty());
        root.set_is_context_menu_enabled(true);
        assert!(
            root.show_item_context_menu(source.clone().upcast(), Point::default())
                .unwrap()
        );
        assert_eq!(*calls.borrow(), [false, true]);
        a.dispose();
        b.dispose();
        let empty = root.context_menu_opening(|args| args.menu.items().clear());
        assert!(
            !root
                .show_item_context_menu(source.upcast(), Point::default())
                .unwrap()
        );
        empty.dispose();
        let old = root.customization();
        let menu = root.build_show_tabs_menu().unwrap();
        let entry = menu
            .items()
            .get(0)
            .unwrap()
            .cast::<ToggleMenuFlyoutItem>()
            .unwrap();
        assert!(entry.is_checked());
        entry.invoke_menu_core();
        assert!(!Rc::ptr_eq(&old, &root.customization()));
        assert!(old.hidden_tab_ids().unwrap().is_empty());
        assert_eq!(
            root.customization().hidden_tab_ids().unwrap().get(0),
            Some("home".into())
        );
        entry.invoke_menu_core();
        assert!(root.customization().hidden_tab_ids().unwrap().is_empty());
        let panels = root.build_show_panels_menu(tab).unwrap();
        let entry = panels
            .items()
            .get(0)
            .unwrap()
            .cast::<ToggleMenuFlyoutItem>()
            .unwrap();
        entry.invoke_menu_core();
        assert_eq!(
            root.customization().hidden_group_ids().unwrap().get(0),
            Some("edit".into())
        );
        let no_tabs = Ribbon::new();
        no_tabs.customization().set_hidden_tab_ids(None);
        assert_eq!(no_tabs.build_show_tabs_menu().unwrap().items().count(), 0);
    });
}
#[test]
fn context_menu_native_linked_qat_actions_use_original_source_and_live_state() {
    crate::group_tests::run(|| {
        let (root, _, _, source) = sample();
        let linked = RibbonButton::new();
        RibbonItemHelper::set_source_item(&linked, source.clone().upcast::<Control>());
        let (slot, token) = capture(&root);
        root.show_item_context_menu(linked.clone().upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        menu_controls(&menu)[0]
            .cast::<MenuFlyoutItem>()
            .unwrap()
            .invoke_menu_core();
        assert!(root.is_in_quick_access(&source));
        root.show_item_context_menu(linked.upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        menu_controls(&menu)[0]
            .cast::<MenuFlyoutItem>()
            .unwrap()
            .invoke_menu_core();
        assert!(!root.is_in_quick_access(&source));
        token.dispose();
    });
}

#[test]
fn context_menu_native_right_tap_handling_popup_point_and_owner_cleanup() {
    crate::group_tests::run(|| {
        use ferroui_base::input::{
            InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties,
            PointerType, TappedEventArgs,
        };
        use ferroui_base::threading::Dispatcher;
        let (root, _, group, button) = sample();
        let window = crate::ribbon_tests::mount(&root);
        let right_tap = |target: &Control, handled: bool| {
            let position = target
                .translate_point(Point::new(7.25, 5.75), &window)
                .unwrap();
            let pointer = PointerEventArgs::new(
                Some(InputElement::pointer_released_event()),
                target.to_ref(),
                Pointer::new(61, PointerType::Mouse, true),
                Some(&window),
                position,
                0,
                PointerPointProperties::NONE,
                KeyModifiers::NONE,
            );
            let event = TappedEventArgs::new(Some(InputElement::right_tapped_event()), &pointer);
            event.set_handled(handled);
            target.raise_event(&event);
            event
        };
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        let token = root.context_menu_opening(move |args| {
            seen.set(seen.get() + 1);
            args.handled.set(true);
        });
        assert!(right_tap(&button, false).handled());
        assert_eq!(
            calls.get(),
            1,
            "item then group bubbling must invoke only once"
        );
        right_tap(&button, true);
        assert_eq!(calls.get(), 1);
        root.set_is_context_menu_enabled(false);
        assert!(!right_tap(&button, false).handled());
        assert!(!right_tap(&group, false).handled());
        let header = root.tab_headers()[0].clone();
        assert!(
            right_tap(&header, false).handled(),
            "source header always consumes right-tap even when menu is disabled"
        );
        assert_eq!(calls.get(), 1);
        token.dispose();
        root.set_is_context_menu_enabled(true);
        let slot = Rc::new(RefCell::new(None));
        let captured = slot.clone();
        let token = root.context_menu_opening(move |args| {
            captured.replace(Some(args.menu.clone()));
        });
        assert!(
            root.show_item_context_menu(button.clone().upcast(), Point::new(13.25, 19.75))
                .unwrap()
        );
        let menu = slot.take().unwrap();
        assert!(menu.is_open());
        assert_eq!(
            menu.popup().placement_rect(),
            Some(Rect::new(13.25, 19.75, 0., 0.))
        );
        assert!(menu.target().unwrap().ptr_eq(&button));
        let weak_menu = menu.downgrade();
        drop(menu);
        assert!(weak_menu.upgrade().is_some());
        weak_menu.upgrade().unwrap().hide();
        assert!(
            weak_menu.upgrade().is_none(),
            "ribbon releases closed menu lease"
        );
        token.dispose();
        window.close();
        window.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
    });
}

#[test]
fn context_menu_native_rust_and_markup_handlers_share_order_args_and_removal() {
    crate::group_tests::run(|| {
        let (root, _, _, button) = sample();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let first = trace.clone();
        let a = root.context_menu_opening(move |args| {
            first.borrow_mut().push(("rust-a", args.handled.get()));
        });
        let stored = Rc::new(RefCell::new(None));
        let observed = stored.clone();
        let middle = trace.clone();
        let handler = MarkupDelegate::new(move |values| {
            let args = values[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonContextMenuEventArgs>>()
                .unwrap()
                .clone();
            middle.borrow_mut().push(("markup", args.handled.get()));
            args.handled.set(true);
            observed.replace(Some(args));
            None
        });
        root.add_context_menu_opening(Some(handler.clone()));
        let last = trace.clone();
        let held = stored.clone();
        let b = root.context_menu_opening(move |args| {
            let recorded = held.borrow().clone().unwrap();
            assert!(std::ptr::eq(args, &*recorded));
            last.borrow_mut().push(("rust-b", args.handled.get()));
        });
        root.show_item_context_menu(button.clone().upcast(), Point::default())
            .unwrap();
        assert_eq!(
            *trace.borrow(),
            [("rust-a", false), ("markup", false), ("rust-b", true)]
        );
        b.dispose();
        root.remove_context_menu_opening(Some(handler));
        a.dispose();
        let suppress = root.context_menu_opening(|args| args.handled.set(true));
        trace.borrow_mut().clear();
        root.show_item_context_menu(button.upcast(), Point::default())
            .unwrap();
        assert!(trace.borrow().is_empty());
        suppress.dispose();
    });
}

/// Callback outcomes come from the six unchanged source bodies and real Core
/// serialization. The C# UI/click boundary is a development seam, not UI proof.
#[test]
fn context_menu_native_callbacks_match_unchanged_source_live_state_actions() {
    crate::group_tests::run(|| {
        use crate::controls::RibbonCustomizePage;
        use ferroribbon_core::state::RibbonCustomization;
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/context-menu-actions.json"
        ))
        .unwrap();
        let actions = &fixture["actions"];
        for case in actions["liveVisibility"].as_array().unwrap() {
            let root = Ribbon::new();
            root.set_can_customize(false);
            root.set_visibility_mode(RibbonVisibilityMode::from_raw(
                case["built"].as_i64().unwrap() as i32,
            ));
            let (slot, token) = capture(&root);
            root.show_item_context_menu(ferroui_controls::Border::new().upcast(), Point::default())
                .unwrap();
            let menu = slot.take().unwrap();
            root.set_visibility_mode(RibbonVisibilityMode::from_raw(
                case["live"].as_i64().unwrap() as i32,
            ));
            let item = menu_controls(&menu)
                .last()
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            for expected in case["values"].as_array().unwrap() {
                item.invoke_menu_core();
                assert_eq!(json!(root.visibility_mode().raw()), *expected, "{case}");
            }
            token.dispose();
        }
        for case in actions["livePosition"].as_array().unwrap() {
            let root = Ribbon::new();
            root.set_can_customize(false);
            root.set_is_collapsible(false);
            root.set_quick_access_position(RibbonQuickAccessPosition::from_raw(
                case["built"].as_i64().unwrap() as i32,
            ));
            let (slot, token) = capture(&root);
            root.show_item_context_menu(ferroui_controls::Border::new().upcast(), Point::default())
                .unwrap();
            let menu = slot.take().unwrap();
            root.set_quick_access_position(RibbonQuickAccessPosition::from_raw(
                case["live"].as_i64().unwrap() as i32,
            ));
            let item = menu_controls(&menu)[0].cast::<MenuFlyoutItem>().unwrap();
            for expected in case["values"].as_array().unwrap() {
                item.invoke_menu_core();
                assert_eq!(
                    json!(root.quick_access_position().raw()),
                    *expected,
                    "{case}"
                );
            }
            token.dispose();
        }
        let ids = |state: &RibbonCustomization| {
            let values = |list: ferroribbon_core::collections::RibbonList<Option<RibbonString>>| {
                list.iter()
                    .map(|value| value.map(|s| s.to_utf8().unwrap()))
                    .collect::<Vec<_>>()
            };
            json!({"tabs":values(state.hidden_tab_ids().unwrap()),"groups":values(state.hidden_group_ids().unwrap())})
        };
        for case in actions["visibility"].as_array().unwrap() {
            let (root, tab, _, _) = sample();
            let before = root.customization();
            for id in ["home", "home", "keep-tab"] {
                before.hidden_tab_ids().unwrap().add(Some(id.into()));
            }
            for id in ["edit", "edit", "keep-group"] {
                before.hidden_group_ids().unwrap().add(Some(id.into()));
            }
            let menu = if case["isTab"].as_bool().unwrap() {
                root.build_show_tabs_menu().unwrap()
            } else {
                root.build_show_panels_menu(tab).unwrap()
            };
            let entry = menu
                .items()
                .get(0)
                .unwrap()
                .cast::<ToggleMenuFlyoutItem>()
                .unwrap();
            let live = RibbonCustomization::new();
            for id in ["late-tab", "home", "home"] {
                live.hidden_tab_ids().unwrap().add(Some(id.into()));
            }
            for id in ["late-group", "edit", "edit"] {
                live.hidden_group_ids().unwrap().add(Some(id.into()));
            }
            root.apply_customization(Some(live)).unwrap();
            for expected in case["states"].as_array().unwrap() {
                let old = root.customization();
                let old_ids = ids(&old);
                entry.invoke_menu_core();
                let current = root.customization();
                let mut actual = ids(&current);
                actual["IsChecked"] = json!(entry.is_checked());
                actual["replaced"] = json!(!Rc::ptr_eq(&old, &current));
                actual["oldUnchanged"] = json!(ids(&old) == old_ids);
                assert_eq!(actual, *expected, "{case}");
            }
            assert_eq!(ids(&before)["tabs"], case["originalTabs"]);
            assert_eq!(ids(&before)["groups"], case["originalGroups"]);
        }
        for case in actions["liveTitles"].as_array().unwrap() {
            let root = Ribbon::new();
            root.set_can_customize(false);
            root.set_is_collapsible(false);
            root.set_is_visibility_menu_enabled(true);
            root.set_show_group_captions(case["built"].as_bool().unwrap());
            let (slot, token) = capture(&root);
            root.show_item_context_menu(ferroui_controls::Border::new().upcast(), Point::default())
                .unwrap();
            let menu = slot.take().unwrap();
            let entry = menu_controls(&menu)
                .iter()
                .find_map(|c| c.cast::<ToggleMenuFlyoutItem>())
                .unwrap();
            root.set_show_group_captions(case["live"].as_bool().unwrap());
            for expected in case["values"].as_array().unwrap() {
                entry.invoke_menu_core();
                assert_eq!(json!(root.show_group_captions()), *expected, "{case}");
            }
            token.dispose();
        }
        for case in actions["linkedQat"].as_array().unwrap() {
            let (root, _, _, source) = sample();
            root.set_can_customize(false);
            root.set_is_collapsible(false);
            let linked = RibbonButton::new();
            RibbonItemHelper::set_source_item(&linked, source.clone().upcast::<Control>());
            if case["built"].as_bool().unwrap() {
                root.add_to_quick_access(&source);
            }
            let (slot, token) = capture(&root);
            root.show_item_context_menu(linked.clone().upcast(), Point::default())
                .unwrap();
            let menu = slot.take().unwrap();
            if case["live"].as_bool().unwrap() {
                root.add_to_quick_access(&source);
            } else {
                root.remove_from_quick_access(&source);
            }
            let entry = menu_controls(&menu)[0].cast::<MenuFlyoutItem>().unwrap();
            for expected in case["values"].as_array().unwrap() {
                entry.invoke_menu_core();
                let source_present = root.is_in_quick_access(&source);
                assert_eq!(
                    json!({"source":source_present,"linked":root.quick_access_tool_bar().unwrap().items().contains(&Some(linked.clone().upcast())),"count":root.quick_access_tool_bar().unwrap().items().count()}),
                    *expected,
                    "{case}"
                );
            }
            token.dispose();
        }
        let root = Ribbon::new();
        root.set_is_collapsible(false);
        let requests = Rc::new(RefCell::new(Vec::new()));
        let observed = requests.clone();
        let requested = root.customize_requested(move |args| {
            observed.borrow_mut().push(match args.page {
                RibbonCustomizePage::Ribbon => "Ribbon",
                RibbonCustomizePage::QuickAccessToolbar => "QuickAccessToolbar",
            });
            args.handled.set(true);
        });
        let (slot, token) = capture(&root);
        root.show_item_context_menu(ferroui_controls::Border::new().upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        // Source blank menu: customize QAT, position, separator, customize ribbon.
        for index in [0, 3] {
            let entry = menu_controls(&menu)[index]
                .cast::<MenuFlyoutItem>()
                .unwrap();
            entry.invoke_menu_core();
            entry.invoke_menu_core();
        }
        assert_eq!(json!(*requests.borrow()), actions["customizationRequests"]);
        requested.dispose();
        token.dispose();
    });
}

#[test]
fn context_menu_native_reentrancy_snapshot_removal_and_owner_lifetime() {
    crate::group_tests::run(|| {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/context-menu-actions.json"
        ))
        .unwrap();
        let expected = &fixture["actions"]["reentrant"];
        let root = Ribbon::new();
        root.set_can_customize(false);
        let target = ferroui_controls::Border::new();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let results = Rc::new(RefCell::new(Vec::new()));
        let removed: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let added: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let once = Rc::new(Cell::new(false));
        let weak_root = root.downgrade();
        let a_trace = trace.clone();
        let a_results = results.clone();
        let b_token = removed.clone();
        let c_token = added.clone();
        let inner_target = target.clone();
        let a = root.context_menu_opening(move |args| {
            a_trace
                .borrow_mut()
                .push(format!("a:{}", args.handled.get()));
            args.handled.set(true);
            if !once.replace(true) {
                b_token.borrow().as_ref().unwrap().dispose();
                let c_trace = a_trace.clone();
                let root = weak_root.upgrade().unwrap();
                c_token.replace(Some(root.context_menu_opening(move |args| {
                    c_trace
                        .borrow_mut()
                        .push(format!("c:{}", args.handled.get()));
                })));
                let result = root
                    .show_item_context_menu(inner_target.clone().upcast(), Point::default())
                    .unwrap();
                a_results.borrow_mut().push(result);
            }
        });
        let released = Rc::new(());
        let in_b = released.clone();
        let b_trace = trace.clone();
        removed.replace(Some(root.context_menu_opening(move |args| {
            let _retained = &in_b;
            b_trace
                .borrow_mut()
                .push(format!("b:{}", args.handled.get()));
        })));
        for _ in 0..2 {
            let result = root
                .show_item_context_menu(target.clone().upcast(), Point::default())
                .unwrap();
            results.borrow_mut().push(result);
        }
        assert_eq!(
            json!({"trace":*trace.borrow(),"results":*results.borrow()}),
            *expected
        );
        assert_eq!(
            Rc::strong_count(&released),
            1,
            "removed callback releases captured state after outer invocation snapshot ends"
        );
        removed.borrow().as_ref().unwrap().dispose();
        a.dispose();
        added.borrow().as_ref().unwrap().dispose();
        let (slot, retained_token) = capture(&root);
        root.show_item_context_menu(target.upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        let weak = root.downgrade();
        drop(root);
        assert!(
            weak.upgrade().is_none(),
            "captured menu and removal handle must not keep Ribbon alive"
        );
        // Its weak-root callback safely expires when a consumer retains the menu.
        menu_controls(&menu)[0]
            .cast::<MenuFlyoutItem>()
            .unwrap()
            .invoke_menu_core();
        retained_token.dispose();
    });
}

#[test]
fn context_menu_native_float_return_and_reentrant_return_all_preserve_presentations() {
    crate::group_tests::run(|| {
        use ferroui_base::threading::Dispatcher;
        let (root, tab, group, button) = sample();
        group.set_header(Some("a".into()));
        let other = RibbonGroup::new();
        other.set_id(Some("other".into()));
        other.set_header(Some("b".into()));
        let late = RibbonGroup::new();
        late.set_id(Some("late".into()));
        late.set_header(Some("late".into()));
        tab.groups().add(Some(other.clone()));
        tab.groups().add(Some(late.clone()));
        root.set_can_float_groups(true);
        root.set_can_customize(false);
        let window = crate::ribbon_tests::mount(&root);
        let panel = group.items_panel();
        let original_parent = panel.visual_parent().unwrap();
        let (slot, opening) = capture(&root);
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        let changed = MarkupDelegate::new(move |_| {
            observed.set(observed.get() + 1);
            None
        });
        group.add_floating_changed(Some(changed.clone()));
        root.show_item_context_menu(group.clone().upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        // A group has no QAT entry; its first entry is the source Float action.
        let float = menu_controls(&menu)[0].cast::<MenuFlyoutItem>().unwrap();
        float.invoke_menu_core();
        assert!(group.is_floating());
        assert!(group.is_floating_panel_open());
        assert!(group.items_panel().ptr_eq(&panel));
        assert!(!panel.visual_parent().unwrap().ptr_eq(&original_parent));
        assert!(panel.children().get(0).ptr_eq(&button));
        float.invoke_menu_core();
        assert_eq!(
            changes.get(),
            1,
            "repeating the captured Float action must not duplicate the state notification"
        );
        root.show_item_context_menu(group.clone().upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        let return_one = menu_controls(&menu)[0].cast::<MenuFlyoutItem>().unwrap();
        return_one.invoke_menu_core();
        return_one.invoke_menu_core();
        assert_eq!(changes.get(), 2);
        assert!(!group.is_floating_panel_open());
        assert!(panel.visual_parent().unwrap().ptr_eq(&original_parent));
        assert!(panel.children().get(0).ptr_eq(&button));
        group.remove_floating_changed(Some(changed));
        group.float(Some(Point::new(40., 80.)));
        other.float(Some(Point::new(80., 120.)));
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let seen = trace.clone();
        let weak_group = group.downgrade();
        let weak_late = late.downgrade();
        let a = MarkupDelegate::new(move |_| {
            let group = weak_group.upgrade().unwrap();
            seen.borrow_mut().push(format!("a:{}", group.is_floating()));
            if !group.is_floating() {
                weak_late
                    .upgrade()
                    .unwrap()
                    .float(Some(Point::new(120., 160.)));
            }
            None
        });
        group.add_floating_changed(Some(a.clone()));
        let seen = trace.clone();
        let weak_other = other.downgrade();
        let b = MarkupDelegate::new(move |_| {
            seen.borrow_mut()
                .push(format!("b:{}", weak_other.upgrade().unwrap().is_floating()));
            None
        });
        other.add_floating_changed(Some(b.clone()));
        let seen = trace.clone();
        let weak_late = late.downgrade();
        let c = MarkupDelegate::new(move |_| {
            seen.borrow_mut().push(format!(
                "late:{}",
                weak_late.upgrade().unwrap().is_floating()
            ));
            None
        });
        late.add_floating_changed(Some(c.clone()));
        let snapshot = root.floating_groups();
        root.show_item_context_menu(group.clone().upcast(), Point::default())
            .unwrap();
        let menu = slot.take().unwrap();
        // Source order: Return this group, Return all groups, separator.
        let return_all = menu_controls(&menu)[1].cast::<MenuFlyoutItem>().unwrap();
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/floating.json")).unwrap();
        let expected = &fixture["corpus"]["reentrantReturnAll"];
        for phase in ["first", "second"] {
            return_all.invoke_menu_core();
            let actual = json!({"states":[group.is_floating(),other.is_floating(),late.is_floating()],"trace":*trace.borrow(),"snapshot":snapshot.iter().map(|g|g.unwrap().header()).collect::<Vec<_>>(),"left":root.floating_groups().iter().map(|g|g.unwrap().header()).collect::<Vec<_>>()});
            assert_eq!(actual, expected[phase]);
        }
        assert!(group.items_panel().ptr_eq(&panel));
        assert!(panel.visual_parent().unwrap().ptr_eq(&original_parent));
        assert!(panel.children().get(0).ptr_eq(&button));
        group.remove_floating_changed(Some(a));
        other.remove_floating_changed(Some(b));
        late.remove_floating_changed(Some(c));
        opening.dispose();
        window.close();
        window.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
    });
}
