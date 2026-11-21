use super::{
    CadPage,
    cad_menus::{discover_menu_flyouts, rebuild_minimize_menu, walk},
};
use ferroribbon::{
    compat::{MenuFlyoutItem, RadioMenuFlyoutItem},
    controls::{
        RibbonButton, RibbonComboBox, RibbonItemHelper, RibbonSplitButton, RibbonStackPanel,
    },
};
use ferroui_base::{Ref, threading::CancellationToken, utilities::CancelEventArgs};
use ferroui_controls::{Application, Button, Control, MenuFlyout};
use ferroui_headless::HeadlessUnitTestSession;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn run(f: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::start_new(super::cad_page_tests::build);
    let result = session
        .dispatch(
            move || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                f();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
fn menu() -> Ref<MenuFlyout> {
    MenuFlyout::new()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/cad-live.json")).unwrap()
}

#[test]
fn source_menu_discovery_identity_lazy_phases_and_stack_recursion() {
    run(|| {
        for case in fixture()["discovery"].as_array().unwrap() {
            let scenario = case["scenario"].as_str().unwrap();
            let one = menu();
            let two = menu();
            let a = RibbonSplitButton::new();
            a.set_flyout(Some(one.clone().upcast()));
            let a: Ref<Control> = a.upcast();
            let b = Button::new();
            b.set_flyout(two.clone());
            let b: Ref<Control> = b.upcast();
            let c = Button::new();
            c.set_flyout(one.clone());
            let c: Ref<Control> = c.upcast();
            let body = Rc::new(RefCell::new(Vec::<Ref<Control>>::new()));
            let slide = Rc::new(RefCell::new(Vec::<Option<Ref<Control>>>::new()));
            let nav = Rc::new(RefCell::new(Vec::<Ref<Control>>::new()));
            let status = Rc::new(RefCell::new(Vec::<Ref<Control>>::new()));
            match scenario {
                "identity" => {
                    body.borrow_mut().extend([a.clone(), b.clone(), a.clone()]);
                    slide
                        .borrow_mut()
                        .extend([Some(a.clone()), Some(c.clone())]);
                    nav.borrow_mut().extend([b.clone(), c.clone()]);
                    status.borrow_mut().extend([a.clone(), c.clone()]);
                }
                "nested" => {
                    let inner = RibbonStackPanel::new();
                    for i in [&b, &a, &c] {
                        inner.items().add(Some(i.clone()));
                    }
                    let outer = RibbonStackPanel::new();
                    outer.items().add(Some(a.clone()));
                    outer.items().add(Some(inner.upcast()));
                    slide
                        .borrow_mut()
                        .extend([None, Some(outer.upcast()), Some(c.clone())]);
                }
                "same-menu" => body.borrow_mut().extend([a.clone(), c.clone()]),
                "large" => {
                    for _ in 0..1000 {
                        let button = Button::new();
                        button.set_flyout(one.clone());
                        let button: Ref<Control> = button.upcast();
                        body.borrow_mut().extend([button.clone(), button]);
                    }
                }
                "nav-stack" | "status-stack" => {
                    let stack = RibbonStackPanel::new();
                    stack.items().add(Some(a.clone()));
                    let target = if scenario == "nav-stack" {
                        &nav
                    } else {
                        &status
                    };
                    target.borrow_mut().extend([stack.upcast(), b.clone()]);
                }
                "late-phase" => body.borrow_mut().push(a.clone()),
                "empty" | "late-start" => {}
                _ => panic!("unmapped source case"),
            }
            let part = body.clone();
            let body_iter =
                std::iter::once_with(move || part.borrow().clone().into_iter()).flatten();
            let part = slide.clone();
            let slide_iter =
                std::iter::once_with(move || walk(part.borrow().clone().into_iter())).flatten();
            let part = nav.clone();
            let nav_iter =
                std::iter::once_with(move || part.borrow().clone().into_iter()).flatten();
            let part = status.clone();
            let status_iter =
                std::iter::once_with(move || part.borrow().clone().into_iter()).flatten();
            let mut iter = discover_menu_flyouts(
                body_iter
                    .chain(slide_iter)
                    .chain(nav_iter)
                    .chain(status_iter),
            );
            if scenario == "late-start" {
                body.borrow_mut().push(a.clone());
            }
            let mut observed = Vec::new();
            if let Some(first) = iter.next() {
                observed.push(if first.ptr_eq(&one) {
                    "one"
                } else {
                    assert!(first.ptr_eq(&two));
                    "two"
                });
                if scenario == "late-phase" {
                    slide.borrow_mut().push(Some(b.clone()));
                    nav.borrow_mut().push(c);
                    status.borrow_mut().push(b);
                }
            }
            observed.extend(iter.map(|m| {
                if m.ptr_eq(&one) {
                    "one"
                } else {
                    assert!(m.ptr_eq(&two));
                    "two"
                }
            }));
            assert_eq!(json!(observed), case["menus"], "{scenario}");
        }
    });
}

#[test]
fn source_complete_cad_menu_inventory_workspace_rebuild_and_converter_lifetimes() {
    run(|| {
        let page = CadPage::new();
        let menus: Vec<_> = page.enumerate_menus().collect();
        assert_eq!(
            menus.len(),
            30,
            "All28 source menus and2 constructor-built menus"
        );
        let mut expected: Vec<Option<String>> = fixture()["source_menu_first_texts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().map(str::to_owned))
            .collect();
        expected.extend([Some("Drafting & Annotation".into()), None]);
        expected.sort();
        let mut actual: Vec<_> = menus
            .iter()
            .map(|m| {
                m.items()
                    .iter()
                    .next()
                    .flatten()
                    .and_then(|v| Control::from_boxed(&v))
                    .and_then(|c| c.cast::<MenuFlyoutItem>())
                    .and_then(|i| i.text())
            })
            .collect();
        actual.sort();
        assert_eq!(actual, expected);
        let button = page.named::<RibbonButton>("WorkspaceStatusButton");
        let workspace = button.flyout().unwrap().cast::<MenuFlyout>().unwrap();
        for case in fixture()["workspace"].as_array().unwrap() {
            let text = case["text"].as_str().map(str::to_owned);
            page.named::<RibbonComboBox>("WorkspaceCombo")
                .set_text(text.clone());
            workspace.on_opening(&CancelEventArgs::new());
            let actual:Vec<_>=workspace.items().iter().flatten().map(|v|Control::from_boxed(&v).unwrap()).map(|c|{
            let item=c.cast::<MenuFlyoutItem>();let radio=c.cast::<RadioMenuFlyoutItem>();
            json!({"kind":if radio.is_some(){"RadioMenuFlyoutItem"}else if item.is_some(){"MenuFlyoutItem"}else{"MenuFlyoutSeparator"},"text":item.and_then(|i|i.text()),"group":radio.as_ref().and_then(|r|r.group_name()),"check":radio.map(|r|r.is_checked())})
        }).collect();
            assert_eq!(json!(actual), case["items"]);
        }
        page.apply_workspace(Some("Drafting & Annotation".into()));
        assert_eq!(
            page.named::<RibbonComboBox>("WorkspaceCombo")
                .text()
                .as_deref(),
            Some("Drafting & Annotation"),
            "ApplyWorkspace must restore text after arbitrary property changes"
        );
        let radio = workspace
            .items()
            .iter()
            .flatten()
            .filter_map(|v| Control::from_boxed(&v))
            .filter_map(|c| c.cast::<RadioMenuFlyoutItem>())
            .nth(2)
            .unwrap();
        radio.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            ferroui_controls::MenuItem::click_event(),
        ));
        assert_eq!(
            page.view_model().workspace().as_deref(),
            Some("3D Modeling")
        );
        let minimize = page
            .named::<RibbonButton>("MinimizeBehaviorButton")
            .flyout()
            .unwrap()
            .cast::<MenuFlyout>()
            .unwrap();
        minimize.on_opening(&CancelEventArgs::new());
        let prior: Vec<_> = minimize.items().iter().flatten().collect();
        assert!(!prior.is_empty());
        minimize.on_opening(&CancelEventArgs::new());
        let current: Vec<_> = minimize.items().iter().flatten().collect();
        assert_eq!(prior.len(), current.len());
        assert!(
            prior.iter().zip(&current).all(|(a, b)| !Rc::ptr_eq(a, b)),
            "Opening rebuilds native entries without reusing stale menu controls"
        );
        let weak = page.downgrade();
        drop(menus);
        drop(page);
        assert!(weak.upgrade().is_none());
        let icon = crate::cad::CadIcon::get("CircleCenterRadius").unwrap();
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new(icon)))
                .unwrap()
                .is_some(),
            "Global converter retains only independent theme state"
        );
        let page = CadPage::new();
        page.set_value(
            ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
            Some(ferroui_base::styling::ThemeVariant::new(
                "CAD inherited Light",
                Some(ferroui_base::styling::ThemeVariant::light()),
            )),
        );
        // Pinned Uno queues ActualThemeChanged; the native page adapter posts it too.
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(
            !page.menu_icons_dark.get(),
            "Map inherited native Light to source ElementTheme.Light"
        );
        let icon = crate::cad::CadIcon::get("CircleCenterRadius").unwrap();
        let converted = RibbonItemHelper::create_menu_icon(Some(Rc::new(icon.clone())))
            .unwrap()
            .unwrap()
            .cast::<ferroribbon::compat::ImageIcon>()
            .unwrap();
        let direct = crate::cad::CadMenuIcons::create(&icon, false)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(
            &converted.source().unwrap(),
            &direct.source().unwrap()
        ));
    });
}

#[test]
fn source_minimize_builder_replaces_previous_generation_from_same_native_objects() {
    run(|| {
        let target = MenuFlyout::new();
        for case in fixture()["minimize"].as_array().unwrap() {
            let source = MenuFlyout::new();
            for text in case["items"].as_array().unwrap() {
                let item = MenuFlyoutItem::new();
                item.set_text(Some(text.as_str().unwrap().to_owned()));
                source.items().add(Some(Control::boxed(item)));
            }
            let source_items: Vec<_> = source.items().iter().flatten().collect();
            rebuild_minimize_menu(&target, || source.clone());
            let actual: Vec<_> = target.items().iter().flatten().collect();
            assert_eq!(actual.len(), source_items.len());
            assert!(
                actual
                    .iter()
                    .zip(source_items)
                    .all(|(a, b)| Rc::ptr_eq(a, &b)),
                "Move the source entries by identity"
            );
            let text: Vec<_> = actual
                .iter()
                .map(|v| {
                    Control::from_boxed(v)
                        .unwrap()
                        .cast::<MenuFlyoutItem>()
                        .unwrap()
                        .text()
                })
                .collect();
            assert_eq!(json!(text), case["items"]);
        }
    });
}

#[test]
fn source_page_menu_iterator_observes_later_collection_changes_and_retains_owner() {
    run(|| {
        let page = CadPage::new();
        let weak = page.downgrade();
        let mut iter = page.enumerate_menus();
        assert!(iter.next().is_some());
        let late = MenuFlyout::new();
        let button = Button::new();
        button.set_flyout(late.clone());
        page.named::<ferroribbon::controls::RibbonToolBar>("NavToolBar")
            .items()
            .add(Some(button.upcast()));
        drop(page);
        assert!(
            weak.upgrade().is_some(),
            "Source iterator retains its owner"
        );
        assert!(
            iter.any(|menu| menu.ptr_eq(&late)),
            "Navigation enumerator starts after ribbon body"
        );
        drop(iter);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn workspace_text_reentry_preserves_inner_state_and_outer_echo_order() {
    run(|| {
        let page = CadPage::new();
        let combo = page.named::<RibbonComboBox>("WorkspaceCombo");
        let weak = page.downgrade();
        let entered = Rc::new(std::cell::Cell::new(false));
        let flag = entered.clone();
        let token = combo.add_property_changed_handler(Rc::new(move |args| {
            if args.property() == RibbonComboBox::text_property().as_property()
                && let Some(page) = weak.upgrade()
                && page
                    .named::<RibbonComboBox>("WorkspaceCombo")
                    .text()
                    .as_deref()
                    == Some("3D Modeling")
                && !flag.replace(true)
            {
                page.apply_workspace(Some("3D Basics".into()));
            }
        }));
        page.apply_workspace(Some("3D Modeling".into()));
        assert!(entered.get());
        assert_eq!(combo.text().as_deref(), Some("3D Basics"));
        assert_eq!(page.view_model().workspace().as_deref(), Some("3D Basics"));
        assert!(
            !page
                .cad_ribbon()
                .find_tab("solid")
                .unwrap()
                .is_tab_visible()
        );
        let history = page.view_model().history();
        assert_eq!(
            history[history.len() - 2..],
            [
                "Command: Workspace: 3D Basics",
                "Command: Workspace: 3D Modeling"
            ]
        );
        combo.remove_property_changed_handler(token);
    });
}
