//! Observed pinned-framework limitations, not assertions of upstream native equivalence.
#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::TextBoxInputTarget;
use ferroui_base::{
    Point,
    input::{KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType},
    threading::Dispatcher,
};
use ferroui_controls::{Control, Grid, StackPanel, TextBox, Window};
use std::{cell::RefCell, rc::Rc};

#[test]
fn framework_pointer_focus_precedes_bubble_handling() {
    headless::run(|| {
        let text = TextBox::new();
        let grid = Grid::new();
        grid.set_width(80.0);
        grid.set_height(40.0);
        grid.set_focusable(true);
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(grid.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(text.focus());
        grid.pointer_pressed(|_, e| e.set_handled(true));
        let position = grid.translate_point(Point::new(5.0, 5.0), &window).unwrap();
        let args = PointerPressedEventArgs::new(
            &grid,
            Pointer::new(1, PointerType::Mouse, true),
            &window,
            position,
            0,
            PointerPointProperties {
                is_left_button_pressed: true,
                ..Default::default()
            },
            KeyModifiers::empty(),
            1,
        );
        grid.raise_event(&args);
        // Source FocusNeutral can disable interaction focus while retaining programmatic focus.
        // Handling the bubble event is insufficient in FerroUI: focus already moved in tunnel.
        assert!(args.handled());
        assert!(!text.is_focused());
        assert!(grid.is_focused());
        window.close();
    });
}
#[test]
fn native_range_selection_exposes_intermediate_endpoint_properties() {
    headless::run(|| {
        let text = TextBox::new();
        text.set_text(Some("abcdef"));
        let window = Window::new();
        window.set_content(Some(Control::boxed(text.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let target = TextBoxInputTarget::new(text.clone(), None).unwrap();
        let states = Rc::new(RefCell::new(Vec::new()));
        let observed = states.clone();
        let weak = text.downgrade();
        let subscription = text.property_changed(move |e| {
            if (e.property() == TextBox::selection_start_property().as_property()
                || e.property() == TextBox::selection_end_property().as_property())
                && let Some(text) = weak.upgrade()
            {
                observed.borrow_mut().push((
                    text.selection_start(),
                    text.selection_end(),
                    text.caret_index(),
                ));
            }
        });
        target.select(1, 3);
        assert_eq!(
            (target.selection_start(), target.selection_length()),
            (1, 3)
        );
        assert!(states.borrow().len() > 1);
        assert!(states.borrow().contains(&(4, 4, 4)));
        assert_eq!(states.borrow().last(), Some(&(1, 4, 4)));
        subscription.dispose();
        target.detach();
        window.close();
    });
}

#[test]
fn removed_scroller_keeps_shared_child_until_content_is_explicitly_cleared() {
    headless::run(|| {
        use ferroui_controls::ScrollViewer;
        let tabs = StackPanel::new();
        let first = ScrollViewer::new();
        first.set_content(Some(Control::boxed(tabs.clone())));
        let host = Grid::new();
        host.children().add(first.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(host.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let parent = tabs.visual_parent();
        assert!(parent.is_some());
        host.children().clear();
        assert_eq!(tabs.visual_parent(), parent);
        first.set_content(None);
        assert!(tabs.visual_parent().is_none());
        let next = ScrollViewer::new();
        next.set_content(Some(Control::boxed(tabs.clone())));
        host.children().add(next.clone());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(tabs.visual_parent().is_some());
        assert_ne!(tabs.visual_parent(), parent);
        window.close();
    });
}
#[test]
fn focus_manager_lookup_returns_current_focus_from_another_top_level() {
    headless::run(|| {
        let first = Window::new();
        let second = Window::new();
        let a = TextBox::new();
        let b = TextBox::new();
        first.set_content(Some(Control::boxed(a)));
        second.set_content(Some(Control::boxed(b.clone())));
        first.show();
        second.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(b.focus());
        // Calling this method via the first window does not scope the underlying keyboard lookup.
        assert_eq!(
            first.focus_manager().get_focused_element(),
            Some(b.clone().upcast())
        );
        assert_ne!(
            ferroui_controls::TopLevel::get_top_level(Some(&b)),
            Some(first.clone().upcast())
        );
        first.close();
        second.close();
    });
}
#[test]
fn native_bring_into_view_uses_minimal_scroll_without_alignment_options() {
    headless::run(|| {
        use ferroui_controls::{
            Border, ScrollViewer,
            presenters::ScrollContentPresenter,
            primitives::ScrollBarVisibility,
            templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions},
        };
        let viewer = ScrollViewer::new();
        viewer.set_width(200.0);
        viewer.set_height(100.0);
        viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
            |_, scope| {
                let presenter = ScrollContentPresenter::new();
                presenter.set_name(Some("PART_ContentPresenter".into()));
                presenter.register_in_name_scope(&**scope).upcast()
            },
        )));
        let content = StackPanel::new();
        let spacer = Border::new();
        spacer.set_height(400.0);
        let target = Border::new();
        target.set_height(20.0);
        let after = Border::new();
        after.set_height(600.0);
        content.children().add(spacer);
        content.children().add(target.clone());
        content.children().add(after);
        viewer.set_content(Some(Control::boxed(content)));
        let window = Window::new();
        window.set_width(200.0);
        window.set_height(100.0);
        window.set_content(Some(Control::boxed(viewer.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(viewer.viewport().height, 100.0);
        target.bring_into_view();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(viewer.offset().y, 320.0);
        window.close();
    });
}

#[test]
fn masked_native_text_box_and_password_companion_hide_automation_values() {
    headless::run(|| {
        use ferroboard_controls::input::ProtectedPasswordTextBox;
        use ferroui_controls::automation::{
            peers::{AutomationControlType, ControlAutomationPeer, TextBoxAutomationPeer},
            provider::IValueProvider,
        };
        let raw = TextBox::new();
        raw.set_password_char('●');
        raw.set_text(Some("local fixture"));
        let peer = TextBoxAutomationPeer::new(&raw);
        assert_eq!(peer.value(), None);
        assert!(peer.get_provider::<dyn IValueProvider>().is_none());
        let protected = ProtectedPasswordTextBox::new();
        protected.set_text(Some("local fixture"));
        let peer = ControlAutomationPeer::create_peer_for_element(&protected);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Edit
        );
        assert!(peer.get_provider::<dyn IValueProvider>().is_none());
        assert!(peer.get_children().is_empty());
        assert_eq!(protected.style_key(), TextBox::TYPE);
        assert_ne!(protected.password_char(), '\0');
        assert_eq!(protected.text().as_deref(), Some("local fixture"));
    });
}
