use super::*;
use ferroribbon_core::model::{RibbonModel, RibbonModelExt};
use ferroui_base::{
    Rect, Size, controls::ResourceDictionary, data::core::ValueTypes, styling::ControlTheme,
    threading::Dispatcher,
};
use ferroui_controls::{MenuFlyout, StackPanel, Window};
fn mount(c: &RibbonTitleBar, ribbon: &Ribbon) -> Ref<Window> {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Title.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    d.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let theme = d
        .try_get_resource(&RibbonTitleBar::TYPE.into(), None)
        .flatten()
        .unwrap();
    c.set_resources(d);
    c.set_theme(
        ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
    let panel = StackPanel::new();
    panel.children().add(c.to_ref());
    panel.children().add(ribbon.to_ref());
    let w = Window::new();
    w.set_width(1400.);
    w.set_content(Some(Control::boxed(panel)));
    w.show();
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn title_cases() -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(include_str!("../../tests/fixtures/title-bar.json"))
        .unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}
#[test]
fn title_precedence_model_replacement_and_unloaded_subscription_cleanup() {
    crate::group_tests::run(|| {
        let c = RibbonTitleBar::new();
        let r = Ribbon::new();
        let model = RibbonModel::new();
        model.set_title(Some("Model".into()));
        r.set_model(Some(model.clone()));
        c.set_ribbon(Some(r.clone()));
        assert_eq!(c.display_title().as_deref(), Some("Model"));
        let w = mount(&c, &r);
        model.set_title(Some("Updated".into()));
        assert_eq!(c.display_title().as_deref(), Some("Updated"));
        c.set_title(Some("".into()));
        model.set_title(Some("Hidden".into()));
        assert_eq!(c.display_title().as_deref(), Some(""));
        c.set_title(None);
        let replacement = RibbonModel::new();
        replacement.set_title(Some("Replacement".into()));
        r.set_model(Some(replacement.clone()));
        model.set_title(Some("Stale".into()));
        assert_eq!(c.display_title().as_deref(), Some("Replacement"));
        w.set_content(None);
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        replacement.set_title(Some("Detached".into()));
        assert_eq!(c.display_title().as_deref(), Some("Replacement"));
        w.close();
    });
}
#[test]
fn original_adaptive_boundaries_use_actual_control_bounds() {
    crate::group_tests::run(|| {
        let c = RibbonTitleBar::new();
        let r = Ribbon::new();
        let w = mount(&c, &r);
        for width in [0., 480., 480.001, 720., 720.001, 1000., 1400.] {
            c.measure(Size::new(width, 44.));
            c.arrange(Rect::new(0., 0., width, 44.));
            c.update_adaptive();
            let case = title_cases()
                .into_iter()
                .find(|v| v["name"] == "adaptive" && v["width"].as_f64() == Some(width))
                .unwrap();
            assert_eq!(
                c.search_box().unwrap().is_visible(),
                case["state"]["searchVisible"].as_bool().unwrap()
            );
            assert_eq!(
                c.parts.borrow().title.as_ref().unwrap().is_visible(),
                case["state"]["titleVisible"].as_bool().unwrap()
            );
            assert_eq!(
                c.search_box().unwrap().width(),
                case["state"]["searchWidth"].as_f64().unwrap()
            );
        }
        c.set_search_width(120.);
        c.update_adaptive();
        assert_eq!(c.search_box().unwrap().width(), 120.);
        w.close();
    });
}
#[test]
fn actual_qat_identity_reparenting_and_app_icon_cancellation() {
    crate::group_tests::run(|| {
        let c = RibbonTitleBar::new();
        let r = Ribbon::new();
        let q = RibbonQuickAccessToolBar::new();
        r.set_quick_access_tool_bar(Some(q.clone()));
        c.set_ribbon(Some(r.clone()));
        let w = mount(&c, &r);
        assert!(
            c.parts
                .borrow()
                .quick_access
                .as_ref()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&q)
        );
        assert!(r.is_quick_access_hosted_externally());
        r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        assert!(
            c.parts
                .borrow()
                .quick_access
                .as_ref()
                .unwrap()
                .child()
                .is_none()
        );
        r.set_quick_access_position(RibbonQuickAccessPosition::AboveRibbon);
        assert!(
            c.parts
                .borrow()
                .quick_access
                .as_ref()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&q)
        );
        let menu = MenuFlyout::new();
        r.set_application_menu(Some(menu.clone().upcast()));
        c.set_is_app_icon_menu_enabled(true);
        let count = Rc::new(Cell::new(0));
        let out = count.clone();
        let token = c.app_icon_click(move |_, args| {
            out.set(out.get() + 1);
            args.set_handled(true);
        });
        ferroui_controls::automation::peers::ButtonAutomationPeer::new(&c.app_button().unwrap())
            .invoke()
            .unwrap();
        assert_eq!(count.get(), 1);
        assert!(!menu.is_open());
        token.dispose();
        ferroui_controls::automation::peers::ButtonAutomationPeer::new(&c.app_button().unwrap())
            .invoke()
            .unwrap();
        assert!(menu.is_open());
        menu.hide();
        w.close();
    });
}

#[test]
fn native_chrome_roles_distinguish_drag_area_and_interactive_content() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Point,
            input::{IInputRoot, WindowDecorationsElementRole},
            media::Brushes,
        };
        use ferroui_controls::{Grid, chrome::WindowDecorationProperties};
        let root = Grid::new();
        let drag = Border::new();
        drag.set_background(Some(Brushes::transparent()));
        WindowDecorationProperties::set_element_role(&drag, WindowDecorationsElementRole::TitleBar);
        let interactive = Border::new();
        interactive.set_background(Some(Brushes::blue()));
        interactive.set_width(60.);
        interactive.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
        WindowDecorationProperties::set_element_role(
            &interactive,
            WindowDecorationsElementRole::User,
        );
        root.children().add(drag);
        root.children().add(interactive);
        let w = Window::new();
        w.set_width(400.);
        w.set_height(100.);
        w.set_extend_client_area_to_decorations_hint(true);
        w.set_content(Some(Control::boxed(root)));
        w.show();
        w.update_layout();
        // Ticks alone do not run the dispatcher completions that release the
        // next composition batch in the shared Headless application. Use the
        // SDK's actual in-flight signal and process both sides of each tick.
        let media = ferroui_base::media::MediaContext::instance();
        for _ in 0..10 {
            Dispatcher::ui_thread().run_jobs(None);
            ferroui_headless::FerroHeadlessPlatform::force_render_timer_tick(1);
            Dispatcher::ui_thread().run_jobs(None);
            if !media.has_composition_batches_in_flight() {
                break;
            }
        }
        assert!(
            !media.has_composition_batches_in_flight(),
            "chrome scene composition must settle before hit testing"
        );
        let source = w.presentation_source();
        let input: &dyn IInputRoot = &**source;
        assert_eq!(
            input.hit_test_chrome_element(Point::new(20., 50.)),
            Some(WindowDecorationsElementRole::User)
        );
        assert_eq!(
            input.hit_test_chrome_element(Point::new(200., 50.)),
            Some(WindowDecorationsElementRole::TitleBar)
        );
        w.close();
    });
}

#[test]
fn native_utf8_title_boundary_preserves_model_units_and_renders_invalid_units_explicitly() {
    crate::group_tests::run(|| {
        let bar = RibbonTitleBar::new();
        let ribbon = Ribbon::new();
        let model = RibbonModel::new();
        ribbon.set_model(Some(model.clone()));
        bar.set_ribbon(Some(ribbon.clone()));
        let w = mount(&bar, &ribbon);
        let text = "日本語 e\u{301} 😀 עברית";
        model.set_title(Some(text.into()));
        assert_eq!(bar.display_title().as_deref(), Some(text));
        let raw = vec![0xd800, b'A' as u16, 0xdc00];
        model.set_title(Some(ferroribbon_core::text::RibbonString::from_utf16(
            raw.clone(),
        )));
        assert_eq!(model.title().unwrap().as_utf16(), raw);
        assert_eq!(bar.display_title().as_deref(), Some("�A�"));
        bar.set_title(Some("Explicit".into()));
        assert_eq!(bar.display_title().as_deref(), Some("Explicit"));
        assert_eq!(model.title().unwrap().as_utf16(), raw);
        w.close();
    });
}

#[test]
fn native_foreground_binding_release_rehost_and_reentrant_ribbon_choice() {
    crate::group_tests::run(|| {
        use ferroui_base::media::Brushes;
        let bar = RibbonTitleBar::new();
        let ribbon = Ribbon::new();
        let qat = RibbonQuickAccessToolBar::new();
        ribbon.set_quick_access_tool_bar(Some(qat.clone()));
        bar.set_ribbon(Some(ribbon.clone()));
        let w = mount(&bar, &ribbon);
        let red: Rc<dyn ferroui_base::media::IBrush> = Brushes::red();
        let green: Rc<dyn ferroui_base::media::IBrush> = Brushes::green();
        bar.set_foreground(Some(green.clone()));
        assert!(Rc::ptr_eq(&qat.foreground().unwrap(), &green));
        ribbon.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        qat.set_foreground(Some(red.clone()));
        bar.set_foreground(Some(Brushes::blue()));
        assert!(Rc::ptr_eq(&qat.foreground().unwrap(), &red));
        ribbon.set_quick_access_position(RibbonQuickAccessPosition::AboveRibbon);
        assert!(Rc::ptr_eq(
            &qat.foreground().unwrap(),
            &bar.foreground().unwrap()
        ));
        let old_menu = MenuFlyout::new();
        ribbon.set_application_menu(Some(old_menu.clone().upcast()));
        let replacement = Ribbon::new();
        let new_menu = MenuFlyout::new();
        replacement.set_application_menu(Some(new_menu.clone().upcast()));
        bar.set_is_app_icon_menu_enabled(true);
        let weak = replacement.downgrade();
        let token = bar.app_icon_click(move |sender, _| sender.set_ribbon(weak.upgrade()));
        ferroui_controls::automation::peers::ButtonAutomationPeer::new(&bar.app_button().unwrap())
            .invoke()
            .unwrap();
        assert!(!old_menu.is_open());
        assert!(
            new_menu.is_open(),
            "default invocation rereads Ribbon after handlers"
        );
        assert!(!ribbon.is_quick_access_hosted_externally());
        token.dispose();
        new_menu.hide();
        w.close();
    });
}
