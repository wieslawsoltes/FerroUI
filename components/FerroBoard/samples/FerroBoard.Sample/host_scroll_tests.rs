use crate::{composition::SampleSession, tests::run};
use ferroboard_controls::KeyboardVisibilityMode;
use ferroui_base::{Point, Ref, Vector, Visual, layout::ILayoutManager, threading::Dispatcher};
use ferroui_controls::{ScrollViewer, TextBox};
fn ancestor_scroll(field: &Ref<TextBox>) -> Ref<ScrollViewer> {
    let mut parent = field.visual_parent();
    while let Some(current) = parent {
        if let Some(scroll) = current.cast::<ScrollViewer>() {
            return scroll;
        }
        parent = current.visual_parent();
    }
    panic!("sample field must be inside a ScrollViewer");
}
fn settle(session: &SampleSession) {
    for _ in 0..3 {
        session.window.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
    }
}
fn visible(field: &Ref<TextBox>, scroll: &Ref<ScrollViewer>) {
    let top = field.translate_point(Point::default(), scroll).unwrap();
    let bottom = field
        .translate_point(Point::new(0.0, field.bounds().height), scroll)
        .unwrap();
    assert!(
        top.y >= -1.0 && bottom.y <= scroll.viewport().height + 1.0,
        "field={top:?}..{bottom:?};viewport={:?};offset={:?}",
        scroll.viewport(),
        scroll.offset()
    );
}
#[test]
fn native_host_keeps_focused_field_visible_when_keyboard_opens_and_resizes() {
    run(|| {
        let session = SampleSession::new(crate::storage_tests::file_storage());
        session.bindings.model().set_settings_pane_open(false);
        session.host.set_is_animated(false);
        session
            .host
            .set_visibility_mode(KeyboardVisibilityMode::Manual);
        session.window.set_width(900.0);
        session.window.set_height(700.0);
        session.window.show();
        settle(&session);
        session.host.set_is_keyboard_open(false);
        settle(&session);
        let field = crate::test_ui::ui(&session)
            .find("ChatBox")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        let scroll = ancestor_scroll(&field);
        scroll.set_bring_into_view_on_focus_change(false);
        assert!(field.focus());
        settle(&session);
        scroll.set_offset(Vector::default());
        settle(&session);
        let large_viewport = scroll.viewport().height;
        session.host.set_is_keyboard_open(true);
        settle(&session);
        assert!(scroll.viewport().height < large_viewport);
        assert!(scroll.offset().y > 0.0);
        visible(&field, &scroll);
        session.keyboard.settings().set_key_height(72.0);
        settle(&session);
        visible(&field, &scroll);
        session.window.set_height(650.0);
        settle(&session);
        visible(&field, &scroll);
        assert_eq!(
            session.keyboard.target_element(),
            Some(field.clone().upcast::<Visual>())
        );
        assert_eq!(
            session.window.focus_manager().get_focused_element(),
            Some(field.upcast())
        );
        session.window.close();
        session.release();
    });
}
