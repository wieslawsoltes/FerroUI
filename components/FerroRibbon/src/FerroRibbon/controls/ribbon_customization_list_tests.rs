use super::*;
use ferroui_base::{input::Key, interactivity::RoutedEventArgs, threading::Dispatcher};
use ferroui_controls::{TextBox, Window};
use serde_json::{Value, json};
fn row(
    id: Option<&str>,
    label: &str,
    level: i32,
    checked: Option<bool>,
) -> Rc<RibbonCustomizationListRow> {
    Rc::new(RibbonCustomizationListRow {
        id: id.map(Into::into),
        label: Some(label.into()),
        detail: Some("detail".into()),
        icon: None,
        level,
        is_checked: checked,
    })
}
fn buttons(list: &RibbonCustomizationList) -> Vec<Ref<Button>> {
    list.panel
        .children()
        .snapshot()
        .iter()
        .map(|c| c.cast::<Button>().unwrap())
        .collect()
}
fn snapshot(
    list: &RibbonCustomizationList,
    name: &str,
    handled: Option<bool>,
    events: &[(Ref<Button>, Rc<std::cell::Cell<usize>>)],
) -> Value {
    let buttons = buttons(list);
    let focused = buttons
        .iter()
        .position(|b| b.is_focused())
        .map_or(-1, |i| i as i32);
    let accent = RibbonTheme::get_brush(Some(&list.root), Some("RibbonAccentSubtleBrush"))
        .unwrap()
        .unwrap();
    // Compare native highlight colors against the source-selected row identity.
    let highlights: Vec<_> = buttons
        .iter()
        .map(|b| {
            b.background().is_some_and(|b| {
                b.as_solid_color_brush().is_some_and(|brush| {
                    brush.color()
                        == accent
                            .cast::<ferroui_base::media::SolidColorBrush>()
                            .unwrap()
                            .color()
                })
            })
        })
        .collect();
    let event_counts: Vec<_> = buttons
        .iter()
        .map(|button| {
            events
                .iter()
                .find(|(b, _)| b.ptr_eq(button))
                .map_or(0, |(_, count)| count.get())
        })
        .collect();
    json!({"name":name,"selectedLabel":list.selected().and_then(|r|r.label.clone()),"handled":handled,"focused":focused,"highlights":highlights,"events":event_counts})
}
fn key(
    list: &RibbonCustomizationList,
    name: &str,
    key: Key,
    source: Option<Ref<Control>>,
    events: &[(Ref<Button>, Rc<std::cell::Cell<usize>>)],
) -> Value {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    if let Some(source) = source {
        args.set_source(source.upcast::<ferroui_base::FerroObject>());
    } else {
        args.set_source(list.root.clone().upcast::<ferroui_base::FerroObject>());
    }
    // Compare the source handler boundary. Raising on Root replaces native
    // event Source and lets ancestor navigation run after this handler, so it
    // cannot represent the source's supplied OriginalSource/Handled UI seam.
    // The separate routed input test exercises the actual native event route.
    list.on_key_down(&args);
    snapshot(list, name, Some(args.handled()), events)
}
#[test]
fn customization_list_native_handler_duplicates_retained_rows_and_checkbox_match_source() {
    crate::group_tests::run(|| {
        let list = RibbonCustomizationList::new();
        list.set_name(None);
        assert_eq!(
            AutomationProperties::get_name(&list.panel).as_deref(),
            Some("")
        );
        let outside = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(outside.clone());
        panel.children().add(list.root());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        list.fill([
            row(Some("duplicate"), "First", 0, Some(true)),
            row(Some("middle"), "Middle", 1, None),
            row(Some("duplicate"), "Last", 2, None),
            row(None, "Null", 0, None),
        ]);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(outside.focus());
        let mut peers = Vec::new();
        let mut events = Vec::new();
        let mut subscriptions = Vec::new();
        for button in buttons(&list) {
            let count = Rc::new(std::cell::Cell::new(0));
            let c = count.clone();
            let peer = ControlAutomationPeer::create_peer_for_element(&button);
            subscriptions.push(peer.property_changed(move |e| {
                if std::ptr::eq(
                    e.property(),
                    SelectionItemPatternIdentifiers::is_selected_property(),
                ) {
                    assert_eq!(e.new_value().unwrap().downcast_ref::<bool>(), Some(&true));
                    c.set(c.get() + 1);
                }
            }));
            events.push((button, count));
            peers.push(peer);
        }
        let mut cases = vec![snapshot(&list, "fill", None, &events)];
        for (name, k) in [
            ("up-without-selection", Key::Up),
            ("home-at-first", Key::Home),
            ("down", Key::Down),
            ("down-to-duplicate", Key::Down),
            ("down-to-duplicate-again", Key::Down),
            ("end", Key::End),
            ("end-at-last", Key::End),
            ("unrelated", Key::A),
        ] {
            cases.push(key(&list, name, k, None, &events));
        }
        let first = buttons(&list)[0].clone();
        let content = ValueTypes::as_object(&*first.content().unwrap())
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        let check = content.children().get(0).cast::<CheckBox>().unwrap();
        cases.push(key(
            &list,
            "checkbox-key",
            Key::Up,
            Some(check.clone().upcast()),
            &events,
        ));
        list.select(Some(&"missing".into()));
        cases.push(snapshot(&list, "missing-id", None, &events));
        list.select(None);
        cases.push(snapshot(&list, "null-id", None, &events));
        let toggles = Rc::new(RefCell::new(Vec::<Value>::new()));
        let observed = toggles.clone();
        let token = list.toggle_changed(move |id, visible| {
            observed
                .borrow_mut()
                .push(json!({"Id":id.map(|id|id.to_utf8().unwrap()),"Visible":visible}))
        });
        check.set_is_checked(Some(false));
        check.set_is_checked(None);
        check.set_is_checked(Some(true));
        let old = buttons(&list)[2].clone();
        list.fill([row(Some("duplicate"), "Replacement", 0, None)]);
        old.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        cases.push(snapshot(&list, "retained-old-button", None, &events));
        let weak = Rc::downgrade(&list);
        list.fill(std::iter::once_with(move || {
            let list = weak.upgrade().unwrap();
            assert!(list.selected().is_none());
            assert!(list.panel.children().is_empty());
            row(Some("iterator"), "Iterator", 0, None)
        }));
        cases.push(snapshot(&list, "clear-before-enumeration", None, &events));
        list.fill([]);
        cases.push(key(&list, "empty-list", Key::Down, None, &events));
        let mut expected: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/customization-list.json"
        ))
        .unwrap();
        // Source harness counts StartBringIntoView calls at a UI seam; native
        // requests/extent are tested separately, not equated to that fake sink.
        for case in expected["cases"].as_array_mut().unwrap() {
            case.as_object_mut().unwrap().remove("bring");
        }
        assert_eq!(
            json!({"cases":cases,"toggles":*toggles.borrow(),"name":AutomationProperties::get_name(&list.panel)}),
            expected
        );
        token.dispose();
        for subscription in subscriptions {
            subscription.dispose();
        }
        window.close();
    });
}
#[test]
fn customization_list_native_row_visuals_and_lifetimes() {
    crate::group_tests::run(|| {
        let list = RibbonCustomizationList::new();
        list.set_name(Some("Main tabs"));
        let r = row(Some("tab"), "Home", 0, Some(true));
        list.fill([r.clone(), row(Some("group"), "Clipboard", 1, None)]);
        assert_eq!(list.root.height(), 380.);
        assert_eq!(
            list.root.border_thickness(),
            ferroui_base::Thickness::uniform(1.)
        );
        assert_eq!(list.root.corner_radius(), CornerRadius::uniform(4.));
        let rows = buttons(&list);
        for (i, button) in rows.iter().enumerate() {
            assert_eq!(
                AutomationProperties::get_position_in_set(button),
                i as i32 + 1
            );
            assert_eq!(AutomationProperties::get_size_of_set(button), 2);
            assert_eq!(
                AutomationProperties::get_help_text(button).as_deref(),
                Some("detail")
            );
        }
        let content = ValueTypes::as_object(&*rows[0].content().unwrap())
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        let check = content.children().get(0).cast::<CheckBox>().unwrap();
        let label = content.children().get(1).cast::<TextBlock>().unwrap();
        assert_eq!(label.font_weight(), FontWeight::SemiBold);
        assert_eq!(check.min_width(), 0.);
        list.select(Some(&"tab".into()));
        assert!(Rc::ptr_eq(&list.selected().unwrap(), &r));
        let observed = Rc::new(std::cell::Cell::new(0));
        let calls = observed.clone();
        let token = list.toggle_changed(move |_, _| calls.set(calls.get() + 1));
        check.set_is_checked(Some(false));
        assert_eq!(observed.get(), 1);
        token.dispose();
        check.set_is_checked(Some(true));
        assert_eq!(observed.get(), 1);
        let retained_root = list.root();
        let weak = Rc::downgrade(&list);
        let weak_row = Rc::downgrade(&r);
        drop(r);
        drop(list);
        assert!(
            weak.upgrade().is_none(),
            "retained root, row controls and disposal token cannot retain the list owner"
        );
        assert!(
            weak_row.upgrade().is_none(),
            "row handlers retain only ID, not the entire row"
        );
        check.set_is_checked(Some(false));
        rows[0].raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(observed.get(), 1);
        drop(retained_root);
    });
}

#[test]
fn customization_list_native_end_focus_requests_scrolling() {
    crate::group_tests::run(|| {
        let list = RibbonCustomizationList::new();
        list.fill((0..60).map(|i| {
            row(
                Some(&format!("command-{i}")),
                &format!("Command {i}"),
                0,
                None,
            )
        }));
        let requests = Rc::new(std::cell::Cell::new(0));
        let seen = requests.clone();
        let token = list
            .root
            .add_handler(Control::request_bring_into_view_event(), move |_, _| {
                seen.set(seen.get() + 1)
            });
        let window = Window::new();
        window.set_width(600.);
        window.set_height(450.);
        window.set_content(Some(Control::boxed(list.root())));
        window.show();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let mut args = KeyEventArgs::new();
        args.key = Key::End;
        args.set_routed_event(Some(InputElement::key_down_event()));
        list.root.raise_event(&args);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(args.handled());
        assert_eq!(
            list.selected().unwrap().label.as_deref(),
            Some("Command 59")
        );
        assert!(buttons(&list)[59].is_focused());
        assert!(
            requests.get() > 0,
            "native focus/BringIntoView produces a routed request"
        );
        let scroll = list.root.child().unwrap().cast::<ScrollViewer>().unwrap();
        assert!(
            scroll.offset().y > 0.,
            "the native ScrollViewer actually reveals the final row"
        );
        list.root
            .remove_handler(Control::request_bring_into_view_event(), token);
        window.close();
    });
}
