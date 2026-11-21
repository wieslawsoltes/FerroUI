//! Framework-only opt-in Thumb input policy; no FerroRibbon imports.
use ferroui_base::{
    Point, Ref,
    input::{KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType},
    threading::CancellationToken,
};
use ferroui_controls::{
    AppBuilder, Application, Control, Window,
    primitives::{Thumb, ThumbDragStartPolicy},
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use std::{cell::Cell, rc::Rc};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
fn run(f: impl FnOnce() + Send + 'static) {
    let s = HeadlessUnitTestSession::start_new(build);
    s.dispatch(
        move || {
            ferroui_controls::register_types();
            f();
        },
        CancellationToken::none(),
    )
    .wait()
    .unwrap();
    s.dispose();
}
fn mount() -> (Ref<Thumb>, Ref<Window>) {
    let t = Thumb::new();
    t.set_width(20.);
    t.set_height(20.);
    let w = Window::new();
    w.set_content(Some(Control::boxed(t.clone())));
    w.show();
    w.update_layout();
    (t, w)
}
fn press(t: &Thumb, w: &Window, left: bool, right: bool) -> bool {
    let e = PointerPressedEventArgs::new(
        t.to_ref(),
        Pointer::new(914, PointerType::Mouse, true),
        w,
        Point::new(5., 5.),
        0,
        PointerPointProperties {
            is_left_button_pressed: left,
            is_right_button_pressed: right,
            ..Default::default()
        },
        KeyModifiers::NONE,
        1,
    );
    t.raise_event(&e);
    e.handled()
}
#[test]
fn default_thumb_keeps_right_press_behavior_and_opt_in_filters_without_handling() {
    run(|| {
        let (t, w) = mount();
        assert_eq!(t.drag_start_policy(), ThumbDragStartPolicy::AnyPress);
        assert!(press(&t, &w, false, true));
        assert!(t.classes().contains(":pressed"));
        w.close();
        let (t, w) = mount();
        t.set_drag_start_policy(ThumbDragStartPolicy::PrimaryPressWhenIdle);
        assert!(!press(&t, &w, false, true));
        assert!(!t.classes().contains(":pressed"));
        assert!(press(&t, &w, true, false));
        assert!(t.classes().contains(":pressed"));
        w.close();
    });
}
#[test]
fn primary_policy_does_not_restart_active_thumb_and_can_be_changed_back() {
    run(|| {
        let (t, w) = mount();
        let count = Rc::new(Cell::new(0));
        let out = count.clone();
        t.drag_started(move |_, _| out.set(out.get() + 1));
        t.set_drag_start_policy(ThumbDragStartPolicy::PrimaryPressWhenIdle);
        assert!(press(&t, &w, true, false));
        assert!(!press(&t, &w, true, true));
        assert_eq!(count.get(), 1);
        t.set_drag_start_policy(ThumbDragStartPolicy::AnyPress);
        assert!(press(&t, &w, false, true));
        assert_eq!(count.get(), 2);
        w.close();
    });
}
