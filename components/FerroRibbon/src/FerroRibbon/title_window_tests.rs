//! Public native window services, using the framework's existing compositing test provider.
use super::*;
use ferroui_base::{
    Rect, Thickness,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    input::{IInputRoot, WindowDecorationsElementRole as Role},
    media::FlowDirection,
    styling::ControlTheme,
};
use ferroui_controls::{
    Window,
    chrome::{TitleBarDecorations, WindowDecorationProperties, WindowDrawnDecorations},
    platform::{IWindowImpl, PlatformRequestedDrawnDecoration},
    testing::{CompositorTestServices, MockWindowingPlatform, TestServices},
};
fn theme(bar: &RibbonTitleBar) {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Title.xaml")
        .unwrap()
        .unwrap();
    let dictionary = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    dictionary.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let value = dictionary
        .try_get_resource(&RibbonTitleBar::TYPE.into(), None)
        .flatten()
        .unwrap();
    bar.set_resources(dictionary);
    bar.set_theme(
        ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
}
fn role_at_part(window: &Window, part: &Control) -> Option<Role> {
    let local = Rect::from_size(part.bounds().size()).center();
    let point = part.translate_point(local, window).unwrap();
    let source = window.presentation_source();
    let input: &dyn IInputRoot = &**source;
    input.hit_test_chrome_element(point)
}
#[test]
fn native_window_attachment_measures_caption_controls_and_releases_owned_state() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let platform = MockWindowingPlatform::create_window_mock();
    platform.needs_managed_decorations.set(true);
    platform
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    services.setup(&platform);
    let window = Window::with_impl(platform.clone());
    window.set_width(1000.);
    window.set_height(200.);
    let bar = RibbonTitleBar::new();
    bar.set_title(Some("Document".into()));
    theme(&bar);
    window.set_content(Some(Control::boxed(bar.clone())));
    window.show();
    platform.is_client_area_extended_to_decorations.set(true);
    platform
        .extend_client_area_to_decorations_changed()
        .unwrap()(true);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    let original_flags = WindowDrawnDecorations::get_title_bar_decorations(&window);
    assert_eq!(
        bar.attach_to_window(&window),
        Ok(RibbonTitleBarWindowState::Ready)
    );
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    // The independent public-service probe records four 45-DIP buttons and
    // No production constant supplies this value.
    assert_eq!(
        bar.caption_buttons_inset(),
        Thickness::new(0., 0., 186., 0.)
    );
    assert!(window.extend_client_area_to_decorations_hint());
    assert!(
        !WindowDrawnDecorations::get_title_bar_decorations(&window)
            .contains(TitleBarDecorations::TITLE)
    );
    let label = bar.parts.borrow().title.clone().unwrap();
    assert_eq!(role_at_part(&window, &label), Some(Role::TitleBar));
    let search = bar.search_box().unwrap();
    assert!(search.is_visible());
    assert_eq!(role_at_part(&window, &search), Some(Role::User));
    let close = window
        .presentation_source()
        .root_visual()
        .unwrap()
        .get_visual_descendants()
        .find(|v| WindowDecorationProperties::get_element_role(v) == Role::CloseButton)
        .unwrap()
        .cast::<Control>()
        .unwrap();
    close.set_width(63.);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    assert_eq!(bar.caption_buttons_inset().right, 204.);
    let caller_flags = WindowDrawnDecorations::get_title_bar_decorations(&window)
        & !TitleBarDecorations::MINIMIZE_BUTTON
        & !TitleBarDecorations::FULL_SCREEN_BUTTON;
    WindowDrawnDecorations::set_title_bar_decorations(&window, caller_flags);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    assert_eq!(bar.caption_buttons_inset().right, 110.);
    let custom = Thickness::new(3., 0., 7., 0.);
    bar.set_caption_buttons_inset(custom);
    close.set_width(80.);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    assert_eq!(
        bar.caption_buttons_inset(),
        custom,
        "explicit caller padding survives ordinary layout updates"
    );
    bar.set_flow_direction(FlowDirection::RightToLeft);
    bar.attach_to_window(&window).unwrap();
    services.run_jobs();
    assert_eq!(
        bar.caption_buttons_inset(),
        Thickness::new(127., 0., 0., 0.)
    );
    let old_drag = bar.parts.borrow().drag.clone().unwrap();
    bar.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
        |_, _| {
            let border = ferroui_controls::Border::new();
            border.set_background(Some(ferroui_base::media::Brushes::transparent()));
            border.upcast()
        },
    )));
    bar.apply_template();
    services.run_jobs();
    assert_eq!(
        WindowDecorationProperties::get_element_role(&old_drag),
        Role::None
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&bar),
        Role::TitleBar,
        "missing dedicated part falls back to the titlebar"
    );
    bar.detach_from_window();
    assert_eq!(
        bar.window_attachment_state(),
        RibbonTitleBarWindowState::Detached
    );
    assert_eq!(bar.caption_buttons_inset(), Thickness::default());
    assert!(!window.extend_client_area_to_decorations_hint());
    assert_eq!(
        WindowDrawnDecorations::get_title_bar_decorations(&window),
        caller_flags | (original_flags & TitleBarDecorations::TITLE)
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&bar),
        Role::None
    );
    bar.attach_to_window(&window).unwrap();
    window.close();
    assert_eq!(
        bar.window_attachment_state(),
        RibbonTitleBarWindowState::Detached
    );
    assert_eq!(bar.caption_buttons_inset(), Thickness::default());
    let unsupported_platform = MockWindowingPlatform::create_window_mock();
    services.setup(&unsupported_platform);
    let unsupported = Window::with_impl(unsupported_platform);
    assert_eq!(
        bar.attach_to_window(&unsupported),
        Err(RibbonTitleBarWindowError::ExtensionUnsupported)
    );
    assert!(!unsupported.extend_client_area_to_decorations_hint());
    unsupported.close();
    assert_eq!(
        bar.attach_to_window(&unsupported),
        Err(RibbonTitleBarWindowError::WindowClosed)
    );
    let early_platform = MockWindowingPlatform::create_window_mock();
    early_platform.needs_managed_decorations.set(true);
    early_platform
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    early_platform
        .is_client_area_extended_to_decorations
        .set(true);
    services.setup(&early_platform);
    let early_window = Window::with_impl(early_platform.clone());
    early_window.set_width(1000.);
    early_window.set_height(200.);
    early_platform
        .extend_client_area_to_decorations_changed()
        .unwrap()(true);
    let early_bar = RibbonTitleBar::new();
    theme(&early_bar);
    early_window.set_content(Some(Control::boxed(early_bar.clone())));
    assert_eq!(
        early_bar.attach_to_window(&early_window),
        Ok(RibbonTitleBarWindowState::AwaitingGeometry)
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&early_bar),
        Role::TitleBar
    );
    early_window.show();
    services.run_jobs();
    early_window.update_layout();
    services.run_jobs();
    assert_eq!(
        early_bar.window_attachment_state(),
        RibbonTitleBarWindowState::Ready
    );
    assert_eq!(early_bar.caption_buttons_inset().right, 186.);
    assert_eq!(
        WindowDecorationProperties::get_element_role(
            early_bar.parts.borrow().drag.as_ref().unwrap()
        ),
        Role::TitleBar
    );
    let weak_bar = early_bar.downgrade();
    early_window.set_content(None);
    drop(early_bar);
    services.run_jobs();
    assert!(
        weak_bar.upgrade().is_none(),
        "window subscriptions and role records must not retain a detached titlebar"
    );
    assert!(!early_window.extend_client_area_to_decorations_hint());
    early_window.close();
}

// Append to the staged title_window_tests.rs after the active consumer finishes.
#[test]
fn native_role_callback_detach_restores_all_owned_roles_and_window_preferences() {
    use std::{cell::Cell, rc::Rc};
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let platform = MockWindowingPlatform::create_window_mock();
    platform.needs_managed_decorations.set(true);
    platform
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    services.setup(&platform);
    let window = Window::with_impl(platform.clone());
    window.set_width(1000.);
    window.set_height(200.);
    let bar = RibbonTitleBar::new();
    theme(&bar);
    window.set_content(Some(Control::boxed(bar.clone())));
    window.show();
    platform.is_client_area_extended_to_decorations.set(true);
    platform
        .extend_client_area_to_decorations_changed()
        .unwrap()(true);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    let old_window_role = WindowDecorationProperties::get_element_role(&window);
    let old_bar_role = WindowDecorationProperties::get_element_role(&bar);
    let drag = bar.parts.borrow().drag.clone().unwrap();
    let old_drag_role = WindowDecorationProperties::get_element_role(&drag);
    let old_flags = WindowDrawnDecorations::get_title_bar_decorations(&window);
    let old_hint = window.extend_client_area_to_decorations_hint();
    let called = Rc::new(Cell::new(false));
    let entered = called.clone();
    let weak = bar.downgrade();
    let token = window.property_changed(move |event| {
        if event.property() == WindowDecorationProperties::element_role_property().as_property()
            && !entered.replace(true)
        {
            weak.upgrade().unwrap().detach_from_window();
        }
    });
    bar.attach_to_window(&window).unwrap();
    token.dispose();
    assert!(called.get());
    assert_eq!(
        bar.window_attachment_state(),
        RibbonTitleBarWindowState::Detached
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&window),
        old_window_role,
        "reentrant detach must restore the window role"
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&bar),
        old_bar_role
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&drag),
        old_drag_role
    );
    assert_eq!(
        WindowDrawnDecorations::get_title_bar_decorations(&window),
        old_flags
    );
    assert_eq!(window.extend_client_area_to_decorations_hint(), old_hint);
    window.close();
}

#[test]
fn native_role_restore_can_reattach_to_same_or_different_window() {
    use std::{cell::Cell, rc::Rc};
    for same_window in [true, false] {
        let services = CompositorTestServices::start(TestServices::styled_window());
        crate::register_types();
        let make_window = || {
            let platform = MockWindowingPlatform::create_window_mock();
            platform.needs_managed_decorations.set(true);
            platform
                .requested_drawn_decorations
                .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
            services.setup(&platform);
            let window = Window::with_impl(platform.clone());
            window.set_width(1000.);
            window.set_height(200.);
            window.show();
            platform.is_client_area_extended_to_decorations.set(true);
            platform
                .extend_client_area_to_decorations_changed()
                .unwrap()(true);
            window
        };
        let first = make_window();
        let second = if same_window {
            first.clone()
        } else {
            make_window()
        };
        let bar = RibbonTitleBar::new();
        theme(&bar);
        first.set_content(Some(Control::boxed(bar.clone())));
        services.run_jobs();
        first.update_layout();
        services.run_jobs();
        let first_flags = WindowDrawnDecorations::get_title_bar_decorations(&first);
        let second_flags = WindowDrawnDecorations::get_title_bar_decorations(&second);
        let first_role = WindowDecorationProperties::get_element_role(&first);
        let second_role = WindowDecorationProperties::get_element_role(&second);
        let bar_role = WindowDecorationProperties::get_element_role(&bar);
        let first_height = first.extend_client_area_title_bar_height_hint();
        let second_height = second.extend_client_area_title_bar_height_hint();
        bar.attach_to_window(&first).unwrap();
        let entered = Rc::new(Cell::new(false));
        let seen = entered.clone();
        let weak = bar.downgrade();
        let target = second.downgrade();
        let token = first.property_changed(move |event| {
            if event.property() == WindowDecorationProperties::element_role_property().as_property()
                && !seen.replace(true)
            {
                let bar = weak.upgrade().unwrap();
                assert_eq!(
                    bar.attach_to_window(&target.upgrade().unwrap()),
                    Ok(RibbonTitleBarWindowState::Pending)
                );
                assert_eq!(
                    bar.window_attachment_state(),
                    RibbonTitleBarWindowState::Pending
                );
            }
        });
        bar.detach_from_window();
        token.dispose();
        assert!(entered.get());
        assert_eq!(
            bar.window_attachment_state(),
            RibbonTitleBarWindowState::Ready
        );
        assert!(second.extend_client_area_to_decorations_hint());
        assert_eq!(
            WindowDecorationProperties::get_element_role(&second),
            Role::User
        );
        if !same_window {
            assert!(!first.extend_client_area_to_decorations_hint());
            assert_eq!(
                WindowDecorationProperties::get_element_role(&first),
                first_role
            );
            assert_eq!(
                WindowDrawnDecorations::get_title_bar_decorations(&first),
                first_flags
            );
            assert_eq!(
                first.extend_client_area_title_bar_height_hint(),
                first_height
            );
        }
        bar.detach_from_window();
        assert_eq!(
            bar.window_attachment_state(),
            RibbonTitleBarWindowState::Detached
        );
        assert_eq!(
            WindowDecorationProperties::get_element_role(&first),
            first_role
        );
        assert_eq!(
            WindowDecorationProperties::get_element_role(&second),
            second_role
        );
        assert_eq!(WindowDecorationProperties::get_element_role(&bar), bar_role);
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&first),
            first_flags
        );
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&second),
            second_flags
        );
        assert_eq!(
            first.extend_client_area_title_bar_height_hint(),
            first_height
        );
        assert_eq!(
            second.extend_client_area_title_bar_height_hint(),
            second_height
        );
        assert!(!second.extend_client_area_to_decorations_hint());
        first.close();
        if !same_window {
            second.close();
        }
    }
}
#[test]
fn native_queued_attachment_failure_is_observable_and_does_not_retain_window() {
    use std::{cell::Cell, rc::Rc};
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let platform = MockWindowingPlatform::create_window_mock();
    platform.is_client_area_extended_to_decorations.set(true);
    services.setup(&platform);
    let first = Window::with_impl(platform);
    let unsupported_platform = MockWindowingPlatform::create_window_mock();
    services.setup(&unsupported_platform);
    let unsupported = Window::with_impl(unsupported_platform);
    let weak_window = unsupported.downgrade();
    let bar = RibbonTitleBar::new();
    bar.attach_to_window(&first).unwrap();
    let entered = Rc::new(Cell::new(false));
    let seen = entered.clone();
    let weak = bar.downgrade();
    let target = unsupported.downgrade();
    let token = first.property_changed(move |event| {
        if event.property() == WindowDecorationProperties::element_role_property().as_property()
            && !seen.replace(true)
        {
            assert_eq!(
                weak.upgrade()
                    .unwrap()
                    .attach_to_window(&target.upgrade().unwrap()),
                Ok(RibbonTitleBarWindowState::Pending)
            );
        }
    });
    bar.detach_from_window();
    token.dispose();
    assert!(entered.get());
    assert_eq!(
        bar.window_attachment_state(),
        RibbonTitleBarWindowState::Failed(RibbonTitleBarWindowError::ExtensionUnsupported)
    );
    assert!(!first.extend_client_area_to_decorations_hint());
    assert!(!unsupported.extend_client_area_to_decorations_hint());
    unsupported.close();
    drop(unsupported);
    assert!(weak_window.upgrade().is_none());
    bar.detach_from_window();
    assert_eq!(
        bar.window_attachment_state(),
        RibbonTitleBarWindowState::Detached
    );
    first.close();
}

// Staged follow-up: append only after the existing strict consumer run terminates.
// The source Window.SetTitleBar selects one drag-region owner; an older native
// Rust cleanup scope must not disable a newer titlebar's accepted integration.
#[test]
fn older_titlebar_cleanup_does_not_disable_newer_window_owner() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let platform = MockWindowingPlatform::create_window_mock();
    platform.needs_managed_decorations.set(true);
    platform
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    services.setup(&platform);
    let window = Window::with_impl(platform.clone());
    window.set_width(1000.);
    window.set_height(200.);
    let first = RibbonTitleBar::new();
    theme(&first);
    let second = RibbonTitleBar::new();
    theme(&second);
    let panel = ferroui_controls::StackPanel::new();
    panel.children().add(first.clone().upcast::<Control>());
    panel.children().add(second.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    platform.is_client_area_extended_to_decorations.set(true);
    platform
        .extend_client_area_to_decorations_changed()
        .unwrap()(true);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    let flags = WindowDrawnDecorations::get_title_bar_decorations(&window);
    first.attach_to_window(&window).unwrap();
    second.attach_to_window(&window).unwrap();
    first.detach_from_window();
    assert!(
        window.extend_client_area_to_decorations_hint(),
        "old cleanup must not disable the newer owner"
    );
    assert!(
        !WindowDrawnDecorations::get_title_bar_decorations(&window)
            .contains(TitleBarDecorations::TITLE)
    );
    second.detach_from_window();
    assert!(!window.extend_client_area_to_decorations_hint());
    assert_eq!(
        WindowDrawnDecorations::get_title_bar_decorations(&window),
        flags
    );
    window.close();
}

fn ownership_window(services: &CompositorTestServices) -> Ref<Window> {
    let platform = MockWindowingPlatform::create_window_mock();
    platform.needs_managed_decorations.set(true);
    platform
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    services.setup(&platform);
    let window = Window::with_impl(platform.clone());
    window.set_width(1000.);
    window.set_height(250.);
    window.show();
    platform.is_client_area_extended_to_decorations.set(true);
    platform
        .extend_client_area_to_decorations_changed()
        .unwrap()(true);
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    window
}
fn fresh_drag_template(bar: &RibbonTitleBar) {
    use ferroui_controls::templates::FuncTemplateNameScopeExtensions;
    bar.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
        |_, scope| {
            let drag = ferroui_controls::Border::new();
            drag.set_name(Some("PART_DragRegion".into()));
            drag.set_background(Some(ferroui_base::media::Brushes::transparent()));
            drag.register_in_name_scope(&**scope).upcast()
        },
    )));
    bar.apply_template();
}
#[test]
fn unchanged_source_owner_sequence_preserves_insets_unload_and_retemplate_reclaim() {
    use std::{cell::RefCell, rc::Rc};
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let window = ownership_window(&services);
    let first = RibbonTitleBar::new();
    theme(&first);
    let second = RibbonTitleBar::new();
    theme(&second);
    let panel = ferroui_controls::StackPanel::new();
    panel.children().add(first.clone().upcast::<Control>());
    panel.children().add(second.clone().upcast::<Control>());
    window.set_content(Some(Control::boxed(panel.clone())));
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let seen = calls.clone();
    let weak_window = window.downgrade();
    let token = window.property_changed(move |event| {
        if event.property()
            == Window::extend_client_area_to_decorations_hint_property().as_property()
        {
            seen.borrow_mut().push(
                weak_window
                    .upgrade()
                    .unwrap()
                    .extend_client_area_to_decorations_hint(),
            );
        }
    });
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/title-window-owner.json")).unwrap();
    let mut position = 0;
    let mut observe = || {
        let row = &fixture["cases"][position];
        let first_active = WindowDecorationProperties::get_element_role(
            &first.parts.borrow().drag.clone().unwrap(),
        ) == Role::TitleBar;
        let second_active = WindowDecorationProperties::get_element_role(
            &second.parts.borrow().drag.clone().unwrap(),
        ) == Role::TitleBar;
        assert_eq!(
            first_active,
            row["firstDrag"].as_bool().unwrap(),
            "{}",
            row["name"]
        );
        assert_eq!(
            second_active,
            row["secondDrag"].as_bool().unwrap(),
            "{}",
            row["name"]
        );
        assert_eq!(
            window.extend_client_area_to_decorations_hint(),
            row["Extended"].as_bool().unwrap()
        );
        position += 1;
    };
    first.attach_to_window(&window).unwrap();
    observe();
    let first_inset = first.caption_buttons_inset();
    assert!(first_inset.right > 0.);
    second.attach_to_window(&window).unwrap();
    observe();
    assert_eq!(
        first.window_attachment_state(),
        RibbonTitleBarWindowState::Superseded
    );
    assert_eq!(
        first.caption_buttons_inset(),
        first_inset,
        "source keeps the older bar's inset"
    );
    let second_inset = second.caption_buttons_inset();
    panel.children().remove(first.clone());
    services.run_jobs();
    observe();
    assert_eq!(first.caption_buttons_inset(), first_inset);
    fresh_drag_template(&first);
    observe();
    assert_eq!(
        first.window_attachment_state(),
        RibbonTitleBarWindowState::Ready
    );
    assert_eq!(
        second.window_attachment_state(),
        RibbonTitleBarWindowState::Superseded
    );
    assert_eq!(second.caption_buttons_inset(), second_inset);
    fresh_drag_template(&second);
    observe();
    assert_eq!(position, fixture["cases"].as_array().unwrap().len());
    first.detach_from_window();
    assert!(window.extend_client_area_to_decorations_hint());
    second.detach_from_window();
    token.dispose();
    assert_eq!(
        *calls.borrow(),
        [true, false],
        "handover/retemplate must not temporarily disable extension"
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&window),
        Role::None
    );
    assert_eq!(first.caption_buttons_inset(), Thickness::default());
    assert_eq!(second.caption_buttons_inset(), Thickness::default());
    window.close();
}
#[test]
fn reentrant_third_owner_is_applied_after_the_current_handover() {
    use std::{cell::Cell, rc::Rc};
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let window = ownership_window(&services);
    let first = RibbonTitleBar::new();
    theme(&first);
    fresh_drag_template(&first);
    let second = RibbonTitleBar::new();
    theme(&second);
    fresh_drag_template(&second);
    let third = RibbonTitleBar::new();
    theme(&third);
    fresh_drag_template(&third);
    first.attach_to_window(&window).unwrap();
    let called = Rc::new(Cell::new(false));
    let seen = called.clone();
    let target = third.downgrade();
    let weak_window = window.downgrade();
    let token = first.property_changed(move |event| {
        if event.property() == WindowDecorationProperties::element_role_property().as_property()
            && !seen.replace(true)
        {
            assert_eq!(
                target
                    .upgrade()
                    .unwrap()
                    .attach_to_window(&weak_window.upgrade().unwrap()),
                Ok(RibbonTitleBarWindowState::Pending)
            );
        }
    });
    assert_eq!(
        second.attach_to_window(&window),
        Ok(RibbonTitleBarWindowState::Superseded)
    );
    token.dispose();
    assert!(called.get());
    assert_eq!(
        first.window_attachment_state(),
        RibbonTitleBarWindowState::Superseded
    );
    assert_eq!(
        third.window_attachment_state(),
        RibbonTitleBarWindowState::Ready
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&first.parts.borrow().drag.clone().unwrap()),
        Role::None
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&second.parts.borrow().drag.clone().unwrap()),
        Role::None
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&third.parts.borrow().drag.clone().unwrap()),
        Role::TitleBar
    );
    first.detach_from_window();
    second.detach_from_window();
    assert!(window.extend_client_area_to_decorations_hint());
    third.detach_from_window();
    assert!(!window.extend_client_area_to_decorations_hint());
    window.close();
}
#[test]
fn dropping_owner_during_another_mutation_releases_before_queued_claim_without_retaining_control() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let first_window = ownership_window(&services);
    let second_window = ownership_window(&services);
    let old = RibbonTitleBar::new();
    old.attach_to_window(&first_window).unwrap();
    let weak_old = old.downgrade();
    let holder = Rc::new(RefCell::new(Some(old)));
    let second = RibbonTitleBar::new();
    let next = RibbonTitleBar::new();
    let called = Rc::new(Cell::new(false));
    let seen = called.clone();
    let held = holder.clone();
    let target = next.downgrade();
    let target_window = first_window.downgrade();
    let token = second_window.property_changed(move |event| {
        if event.property() == WindowDecorationProperties::element_role_property().as_property()
            && !seen.replace(true)
        {
            held.borrow_mut().take();
            assert_eq!(
                target
                    .upgrade()
                    .unwrap()
                    .attach_to_window(&target_window.upgrade().unwrap()),
                Ok(RibbonTitleBarWindowState::Pending)
            );
        }
    });
    second.attach_to_window(&second_window).unwrap();
    token.dispose();
    assert!(called.get());
    assert!(weak_old.upgrade().is_none());
    assert!(first_window.extend_client_area_to_decorations_hint());
    next.detach_from_window();
    assert!(
        !first_window.extend_client_area_to_decorations_hint(),
        "queued replacement captures the restored original baseline"
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&first_window),
        Role::None
    );
    second.detach_from_window();
    first_window.close();
    second_window.close();
}
#[test]
fn handover_uses_intervening_caller_preferences_as_the_new_cleanup_baseline() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let window = ownership_window(&services);
    window.set_extend_client_area_to_decorations_hint(true);
    WindowDrawnDecorations::set_title_bar_decorations(&window, TitleBarDecorations::CLOSE_BUTTON);
    let first = RibbonTitleBar::new();
    first.arrange(Rect::new(0., 0., 1000., 44.));
    let second = RibbonTitleBar::new();
    second.arrange(Rect::new(0., 0., 1000., 52.));
    first.attach_to_window(&window).unwrap();
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 44.);
    window.set_extend_client_area_title_bar_height_hint(77.);
    window.set_extend_client_area_to_decorations_hint(false);
    WindowDrawnDecorations::set_title_bar_decorations(
        &window,
        TitleBarDecorations::TITLE | TitleBarDecorations::CLOSE_BUTTON,
    );
    WindowDecorationProperties::set_element_role(&window, Role::TitleBar);
    second.attach_to_window(&window).unwrap();
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 52.);
    first.detach_from_window();
    second.detach_from_window();
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 77.);
    assert!(!window.extend_client_area_to_decorations_hint());
    assert_eq!(
        WindowDrawnDecorations::get_title_bar_decorations(&window),
        TitleBarDecorations::TITLE | TitleBarDecorations::CLOSE_BUTTON
    );
    assert_eq!(
        WindowDecorationProperties::get_element_role(&window),
        Role::TitleBar
    );
    window.close();
}
#[test]
fn source_unsupported_attachment_clears_padding_and_template_retries_weak_association() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    crate::register_types();
    let platform = MockWindowingPlatform::create_window_mock();
    platform.needs_managed_decorations.set(true);
    platform
        .requested_drawn_decorations
        .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
    services.setup(&platform);
    let window = Window::with_impl(platform.clone());
    window.set_width(1000.);
    window.set_height(200.);
    let bar = RibbonTitleBar::new();
    theme(&bar);
    let before = Thickness::uniform(8.);
    bar.set_caption_buttons_inset(before);
    window.set_content(Some(Control::boxed(bar.clone())));
    window.show();
    services.run_jobs();
    window.update_layout();
    services.run_jobs();
    assert_eq!(
        bar.attach_to_window(&window),
        Err(RibbonTitleBarWindowError::ExtensionUnsupported)
    );
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/title-window-owner.json")).unwrap();
    let inset = &source["unsupportedInset"];
    assert_eq!(
        bar.caption_buttons_inset(),
        Thickness::new(
            inset["Left"].as_f64().unwrap(),
            inset["Top"].as_f64().unwrap(),
            inset["Right"].as_f64().unwrap(),
            inset["Bottom"].as_f64().unwrap()
        )
    );
    assert!(!window.extend_client_area_to_decorations_hint());
    assert_eq!(
        WindowDecorationProperties::get_element_role(&window),
        Role::None
    );
    platform.is_client_area_extended_to_decorations.set(true);
    platform
        .extend_client_area_to_decorations_changed()
        .unwrap()(true);
    fresh_drag_template(&bar);
    assert_eq!(
        bar.window_attachment_state(),
        RibbonTitleBarWindowState::Ready
    );
    assert!(bar.caption_buttons_inset().right > 0.);
    assert!(window.extend_client_area_to_decorations_hint());
    bar.detach_from_window();
    assert_eq!(
        bar.caption_buttons_inset(),
        before,
        "explicit Rust cleanup restores the preassociation value"
    );
    assert!(!window.extend_client_area_to_decorations_hint());
    window.close();
}
