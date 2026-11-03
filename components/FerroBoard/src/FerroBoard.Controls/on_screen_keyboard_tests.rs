use super::*;
use crate::{test_scheduler::ManualScheduler, view_fixture::bits};
use ferroboard_core::{engine::LanguageModels, layouts::KeyboardResources};
use ferroui_base::{Rect, Size};
use ferroui_controls::Window;
use serde_json::{Value, json};
struct Fixture {
    owner: Ref<OnScreenKeyboard>,
    engine: Rc<KeyboardEngine>,
    target: Rc<TextBufferTarget>,
    clock: Rc<ManualScheduler>,
}
impl Fixture {
    fn new() -> Self {
        let settings = Rc::new(KeyboardSettings::new());
        settings.set_auto_capitalization(false);
        settings.set_haptic_feedback(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        let clock = ManualScheduler::new();
        let owner = OnScreenKeyboard::with_scheduler(Some(engine.clone()), clock.clone()).unwrap();
        owner.measure(Size::new(640.0, 300.0));
        owner.arrange(Rect::new(0.0, 0.0, 640.0, 300.0));
        Self {
            owner,
            engine,
            target,
            clock,
        }
    }
    fn state(&self, side_width_written: bool) -> Value {
        let v = &self.owner;
        let settings = self.engine.settings();
        json!({"theme":v.effective_theme().id().unwrap().to_utf8().unwrap(),"body":bits(v.body.height()),"main":bits(v.main.width()),"max":bits(v.main.max_width()),"align":format!("{:?}",v.main.horizontal_alignment()),"side":visible(v.side_panel.is_visible()),"sideWidth":side_width_written.then(||bits(v.side_panel.width())),"gap":bits(v.keys_view.geometry().split_gap()),"spacing":[bits(settings.key_spacing_horizontal()),bits(settings.key_spacing_vertical())],"font":bits(v.keys_view.font_scale()),"minY":bits(v.touch_processor.popup_min_y()),"ui":format!("{:?}",self.engine.ui_mode()),"visible":[visible(v.keys_view.is_visible()),visible(v.emoji_panel.is_visible()),visible(v.clipboard_panel.is_visible()),visible(v.editing_panel.is_visible())],"smart":visible(v.smartbar.is_visible())})
    }
    fn search(&self) -> Value {
        let state = self.owner.search_state.borrow().clone();
        let attrs = self.engine.target().map(|t| t.attributes());
        json!({"active":state.is_some(),"query":state.as_ref().and_then(|s|s.query.borrow().as_ref().map(|s|s.units().to_vec())),"results":state.as_ref().and_then(|s|s.results.borrow().as_ref().map(|r|r.borrow().iter().map(|s|s.as_ref().map(|s|s.units().to_vec())).collect::<Vec<_>>())),"text":self.target.text().units(),"privateTarget":attrs.as_ref().map(|a|a.is_private()),"kind":attrs.map(|a|format!("{:?}",a.kind())),"targetOriginal":self.engine.target().is_some_and(|t|Rc::ptr_eq(&t,&(self.target.clone() as Rc<dyn ITextInputTarget>)))})
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.owner.detach().unwrap();
        self.owner.release_native_subscriptions().unwrap();
        self.engine.dispose().unwrap();
    }
}
fn visible(value: bool) -> &'static str {
    if value { "Visible" } else { "Collapsed" }
}
fn hand(value: &str) -> OneHandedMode {
    match value {
        "Off" => OneHandedMode::Off,
        "Left" => OneHandedMode::Left,
        "Right" => OneHandedMode::Right,
        _ => panic!("{value}"),
    }
}
#[test]
fn source_owner_layout_panels_search_and_target_corpus() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let mut written = false;
        for row in include_str!("../../tests/fixtures/owner.jsonl").lines() {
            let row: Value = serde_json::from_str(row).unwrap();
            match row["op"].as_str().unwrap() {
                "layout" => {
                    let settings = f.engine.settings();
                    settings.set_number_row(row["number"].as_bool().unwrap());
                    settings.set_split_keyboard(row["split"].as_bool().unwrap());
                    let mode = hand(row["hand"].as_str().unwrap());
                    settings.set_one_handed_mode(mode);
                    settings.set_theme_mode(if row["dark"].as_bool().unwrap() {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    });
                    f.owner.apply_theme().unwrap();
                    written |= mode != OneHandedMode::Off;
                    assert_eq!(f.state(written), row["state"], "{row}");
                }
                "mode" => {
                    let settings = f.engine.settings();
                    settings.set_one_handed_mode(OneHandedMode::Off);
                    settings.set_split_keyboard(false);
                    settings.set_number_row(false);
                    let mode = match row["mode"].as_str().unwrap() {
                        "Text" => KeyboardUiMode::Text,
                        "Media" => KeyboardUiMode::Media,
                        "Clipboard" => KeyboardUiMode::Clipboard,
                        "Editing" => KeyboardUiMode::Editing,
                        _ => panic!(),
                    };
                    f.engine.set_ui_mode(mode).unwrap();
                    assert_eq!(f.state(written), row["state"]);
                }
                "search-begin" => {
                    f.engine.set_ui_mode(KeyboardUiMode::Text).unwrap();
                    f.owner.attach(f.target.clone()).unwrap();
                    f.owner.begin_emoji_search().unwrap();
                    assert_eq!(f.search(), row["state"]);
                }
                "search-query" => {
                    f.engine.input_text(&"thumb".into()).unwrap();
                    assert_eq!(f.search(), row["state"]);
                }
                "search-insert" => {
                    f.owner.insert_search_result("👍🏽".into()).unwrap();
                    assert_eq!(f.search(), row["state"]);
                }
                "search-end" => {
                    f.owner.end_emoji_search(true).unwrap();
                    assert_eq!(f.search(), row["state"]);
                    assert_eq!(format!("{:?}", f.engine.ui_mode()), row["ui"]);
                }
                "targets" => {
                    let changes = Rc::new(Cell::new(0));
                    let count = changes.clone();
                    let token = f
                        .owner
                        .target_changed()
                        .add(Rc::new(move |_| count.set(count.get() + 1)));
                    f.owner
                        .attach(Rc::new(TextBufferTarget::new("next".into(), None)))
                        .unwrap();
                    f.owner.detach().unwrap();
                    f.owner.detach().unwrap();
                    assert_eq!(changes.get(), row["changes"].as_u64().unwrap());
                    assert!(f.engine.target().is_none());
                    f.owner.target_changed().remove(token);
                }
                // Preview coordinate oracle uses explicit logical transforms. Actual native transforms are
                // exercised in the attached preview/popup test below; compare its dimensions/label there.
                "preview" | "preview-hide" => {}
                "enter" => {
                    let trace = Rc::new(RefCell::new(Vec::<String>::new()));
                    let log = trace.clone();
                    let handled = row["handled"].as_bool().unwrap();
                    let enter = f.owner.enter_action_requested().add(Rc::new(move |args| {
                        log.borrow_mut().push(format!("enter:{:?}", args.action));
                        args.handled.set(handled);
                    }));
                    let log = trace.clone();
                    let hide = f
                        .owner
                        .hide_requested()
                        .add(Rc::new(move |_| log.borrow_mut().push("hide".into())));
                    let action = if row["action"] == "Done" {
                        EnterAction::Done
                    } else {
                        EnterAction::Search
                    };
                    let result = f
                        .owner
                        .raise_enter_action(Grid::new().upcast(), action)
                        .unwrap();
                    assert_eq!(json!(result), row["result"]);
                    assert_eq!(json!(*trace.borrow()), row["trace"]);
                    f.owner.enter_action_requested().remove(enter);
                    f.owner.hide_requested().remove(hide);
                }
                "clipboard-identity" => {
                    // Custom service identity is additionally exercised by the native cycle test.
                    assert!(row["preserved"].as_bool().unwrap());
                    assert_eq!(
                        f.engine.clipboard_service().as_ref().type_id(),
                        TypeId::of::<SystemClipboardService>()
                    );
                    assert!(row["replaced"].as_bool().unwrap());
                }
                "search-reentry-detach" => {
                    f.owner.attach(f.target.clone()).unwrap();
                    f.owner.begin_emoji_search().unwrap();
                    let owner = f.owner.clone();
                    let token = f
                        .engine
                        .text_committed()
                        .add(Rc::new(move |_| owner.detach().unwrap()));
                    f.owner.insert_search_result("🙂".into()).unwrap();
                    f.engine.text_committed().remove(token);
                    assert_eq!(
                        json!(f.owner.search_state.borrow().is_some()),
                        row["active"]
                    );
                    assert_eq!(json!(f.engine.target().is_none()), row["detached"]);
                    assert_eq!(json!(f.target.text().units()), row["text"]);
                }
                op => panic!("{op}"),
            }
        }
    });
}
#[test]
fn native_owner_attachment_identity_settings_and_enter_actions() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let text = TextBox::new();
        text.set_text(Some("A😀"));
        assert!(f.owner.attach_to(Some(text.clone().upcast())).unwrap());
        let old = f.engine.target().unwrap();
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        f.owner
            .target_changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        assert!(f.owner.attach_to(Some(text.clone().upcast())).unwrap());
        assert!(Rc::ptr_eq(&old, &f.engine.target().unwrap()));
        assert_eq!(changes.get(), 0);
        f.owner.begin_emoji_search().unwrap();
        assert!(!f.owner.attach_to(Some(Grid::new().upcast())).unwrap());
        assert!(f.owner.search_state.borrow().is_none());
        assert!(Rc::ptr_eq(&old, &f.engine.target().unwrap()));
        let replacement = Rc::new(KeyboardSettings::new());
        replacement.set_key_height(60.5);
        replacement.set_number_row(true);
        f.owner.set_settings(Some(replacement));
        assert!(Rc::ptr_eq(&f.owner.settings(), &f.engine.settings()));
        assert_eq!(f.owner.body.height(), 308.5);
        f.owner.set_settings(None);
        assert!(Rc::ptr_eq(&f.owner.settings(), &f.engine.settings()));
        let hides = Rc::new(Cell::new(0));
        let n = hides.clone();
        f.owner
            .hide_requested()
            .add(Rc::new(move |_| n.set(n.get() + 1)));
        assert!(
            f.owner
                .raise_enter_action(text.clone().upcast(), EnterAction::Done)
                .unwrap()
        );
        assert_eq!(hides.get(), 1);
        let token = f
            .owner
            .enter_action_requested()
            .add(Rc::new(|a| a.handled.set(true)));
        assert!(
            f.owner
                .raise_enter_action(text.clone().upcast(), EnterAction::Done)
                .unwrap()
        );
        assert_eq!(hides.get(), 1);
        f.owner.enter_action_requested().remove(token);
        let masked = TextBox::new();
        masked.set_password_char('•');
        assert!(f.owner.attach_to(Some(masked.clone().upcast())).unwrap());
        assert!(f.engine.target().unwrap().attributes().is_password());
        text.set_is_read_only(true);
        assert!(!OnScreenKeyboard::is_text_input(Some(&text)));
        assert!(f.owner.attach_to(Some(text.clone().upcast())).unwrap()); // Source explicit attach ignores read-only eligibility.
        f.owner.detach().unwrap();
        let snapshot = old.text();
        text.set_text(Some("external"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_ne!(snapshot, old.text());
    });
}
#[test]
fn native_owner_focus_tracks_own_window_and_loaded_lifecycle() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let a = Window::new();
        let b = Window::new();
        let input = TextBox::new();
        let other = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(input.clone());
        panel.children().add(f.owner.clone());
        a.set_content(Some(Control::boxed(panel)));
        b.set_content(Some(Control::boxed(other.clone())));
        a.set_width(700.0);
        a.set_height(600.0);
        a.show();
        b.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(f.owner.loaded.get());
        assert!(f.owner.focus_tracking.get());
        input.focus();
        assert_eq!(f.owner.target_element(), Some(input.clone().upcast()));
        other.focus();
        assert_eq!(f.owner.target_element(), Some(input.clone().upcast()));
        f.owner.set_auto_attach(false);
        let same = f.engine.target().unwrap();
        input.focus();
        assert!(Rc::ptr_eq(&same, &f.engine.target().unwrap()));
        f.owner.set_auto_attach(true);
        assert!(f.owner.focus_tracking.get());
        assert_eq!(f.owner.target_element(), Some(input.clone().upcast()));
        a.set_content(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(!f.owner.loaded.get());
        assert!(f.owner.focus_subscription.borrow().is_none());
        a.close();
        b.close();
    });
}
#[test]
fn native_owner_preview_popup_resize_and_unload_cancel_timers() {
    crate::test_support::run(|| {
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        let window = Window::new();
        window.set_width(640.0);
        window.set_height(440.0);
        window.set_content(Some(Control::boxed(f.owner.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        f.engine.settings().set_theme_mode(ThemeMode::Dark);
        f.engine
            .update_layout(634.0, 200.0, Some(f.owner.keys_view.geometry()))
            .unwrap();
        let key = f
            .engine
            .keyboard()
            .unwrap()
            .keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == 113)
            .unwrap();
        let row: Value = include_str!("../../tests/fixtures/owner.jsonl")
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .find(|v| v["op"] == "preview")
            .unwrap();
        f.owner.show_preview(key.clone()).unwrap();
        let preview = &f.owner.preview;
        let actual = json!({"op":"preview","code":key.code(),"label":f.owner.preview_text.text().unwrap().encode_utf16().collect::<Vec<_>>(),"width":bits(preview.width()),"height":bits(preview.height()),"x":bits(Canvas::get_left(preview)),"y":bits(Canvas::get_top(preview)),"font":bits(f.owner.preview_text.font_size()),"visible":visible(preview.is_visible())});
        assert_eq!(actual, row);
        let other = f
            .engine
            .keyboard()
            .unwrap()
            .keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == 119)
            .unwrap();
        f.owner.hide_preview(Some(&other));
        assert!(preview.is_visible());
        f.owner.hide_preview(Some(&key));
        assert!(!preview.is_visible());
        let b = key.visible_bounds();
        let x = b.x + b.width / 2.0;
        let y = b.y + b.height / 2.0;
        f.owner.touch_processor.pointer_down(7, x, y).unwrap();
        assert!(preview.is_visible());
        let delay = i64::from(f.engine.settings().long_press_delay()) * 10_000;
        f.clock.advance(delay - 1);
        assert!(f.owner.touch_processor.popup().is_none());
        f.clock.advance(1);
        assert!(f.owner.touch_processor.popup().is_some());
        assert!(f.owner.popup.is_visible());
        assert!(!preview.is_visible());
        assert!(f.owner.popup_canvas.children().count() > 0);
        f.owner.touch_processor.pointer_cancel(7).unwrap();
        assert!(!f.owner.popup.is_visible());
        assert_eq!(f.owner.popup_canvas.children().count(), 0);
        f.owner.show_resize_overlay().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        let point = f
            .owner
            .resize_overlay
            .translate_point(Point::new(12.0, 12.0), &window)
            .unwrap();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        assert!(f.owner.resize_pointer.get().is_some());
        let start = f.engine.settings().height_scale();
        window.mouse_move(
            Point::new(point.x, point.y - 30.0),
            RawInputModifiers::LEFT_MOUSE_BUTTON,
        );
        assert!(f.engine.settings().height_scale() > start);
        window.mouse_up(
            Point::new(point.x, point.y - 30.0),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        assert!(f.owner.resize_pointer.get().is_none());
        {
            use ferroui_base::input::{
                IPointer, KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs,
                PointerType,
            };
            let pointer = Pointer::new(501, PointerType::Touch, true);
            let args = PointerPressedEventArgs::new(
                &f.owner.resize_overlay,
                pointer.clone(),
                &window,
                point,
                0,
                PointerPointProperties::default(),
                KeyModifiers::empty(),
                1,
            );
            f.owner.resize_overlay.raise_event(&args);
            assert_eq!(f.owner.resize_pointer.get(), Some(501));
            let other = f.owner.to_ref().upcast::<InputElement>();
            pointer.capture(Some(&other));
            assert!(f.owner.resize_pointer.get().is_none());
            assert_eq!(pointer.captured(), Some(other));
            pointer.capture(None);
        }
        f.owner.touch_processor.pointer_down(9, x, y).unwrap();
        window.set_content(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        f.clock.advance(delay * 2);
        assert!(f.owner.touch_processor.popup().is_none());
        window.close();
    });
}
#[test]
fn native_owner_custom_clipboard_theme_dispatch_and_explicit_cycle_release() {
    crate::test_support::run(|| {
        use ferroboard_core::clipboard::ClipboardFuture;
        use ferroboard_core::clipboard::IClipboardService;
        struct Custom;
        impl IClipboardService for Custom {
            fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
                Box::pin(std::future::ready(Ok(Some("custom".into()))))
            }
            fn set_text_async(&self, _: Option<Utf16Text>) -> ClipboardFuture<()> {
                Box::pin(std::future::ready(Ok(())))
            }
        }
        let standalone = OnScreenKeyboard::new(None).unwrap();
        assert!(standalone.auto_attach());
        assert!(Rc::ptr_eq(
            &standalone.settings(),
            &standalone.engine().settings()
        ));
        standalone.release_native_subscriptions().unwrap();
        standalone.engine().dispose().unwrap();
        let f = Fixture::new();
        assert_eq!(
            f.engine.clipboard_service().as_ref().type_id(),
            TypeId::of::<SystemClipboardService>()
        );
        let custom: Rc<dyn IClipboardService> = Rc::new(Custom);
        f.engine.set_clipboard_service(custom.clone());
        let second =
            OnScreenKeyboard::with_scheduler(Some(f.engine.clone()), f.clock.clone()).unwrap();
        assert!(Rc::ptr_eq(&custom, &f.engine.clipboard_service()));
        let theme = KeyboardTheme::new();
        theme.set_id(Some("owner-dispatch-check".into()));
        theme.set_key_font_size(31.25);
        f.engine
            .settings()
            .set_light_theme(Some("owner-dispatch-check".into()));
        f.engine.settings().set_theme_mode(ThemeMode::Light);
        std::thread::spawn({
            let theme = theme.clone();
            move || KeyboardThemes::register(theme).unwrap()
        })
        .join()
        .unwrap();
        assert!(!Arc::ptr_eq(&f.owner.effective_theme(), &theme));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(Arc::ptr_eq(&f.owner.effective_theme(), &theme));
        f.owner.set_theme(Some(KeyboardThemes::floris_night()));
        assert!(Arc::ptr_eq(
            &f.owner.effective_theme(),
            &KeyboardThemes::floris_night()
        ));
        f.owner.set_theme(None);
        assert!(Arc::ptr_eq(&f.owner.effective_theme(), &theme));
        second.release_native_subscriptions().unwrap();
        drop(second);
        let weak = f.owner.downgrade();
        f.owner.begin_emoji_search().unwrap();
        f.owner.release_native_subscriptions().unwrap();
        f.owner.release_native_subscriptions().unwrap();
        assert!(f.engine.target().is_some_and(|target| Rc::ptr_eq(
            &target,
            &(f.target.clone() as Rc<dyn ITextInputTarget>)
        )));
        assert!(f.owner.search_state.borrow().is_none());
        drop(f);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn native_owner_raw_typing_and_private_search_preserve_text_field_focus() {
    crate::test_support::run(|| {
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;
        let f = Fixture::new();
        let input = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(input.clone());
        panel.children().add(f.owner.clone());
        let window = Window::new();
        window.set_width(640.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(input.focus());
        assert_eq!(f.owner.target_element(), Some(input.clone().upcast()));
        let keyboard = f.engine.keyboard().unwrap();
        let key = keyboard
            .keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == 113)
            .unwrap();
        let b = key.visible_bounds();
        let point = f
            .owner
            .keys_view
            .translate_point(
                Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0),
                &window,
            )
            .unwrap();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        assert!(f.owner.preview.is_visible());
        assert_eq!(input.text().as_deref(), None);
        window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(input.text().as_deref(), Some("q"));
        assert!(input.is_focused());
        assert!(!f.owner.preview.is_visible());
        let external = f.engine.target().unwrap();
        f.owner.begin_emoji_search().unwrap();
        f.engine.input_text(&"rocket".into()).unwrap();
        assert_eq!(input.text().as_deref(), Some("q"));
        assert!(f.engine.target().unwrap().attributes().is_private());
        assert!(input.is_focused());
        f.owner.insert_search_result("🚀".into()).unwrap();
        assert_eq!(input.text().as_deref(), Some("q🚀"));
        assert_eq!(input.caret_index(), 3);
        assert_eq!(
            f.owner.search_buffer.borrow().as_ref().unwrap().text(),
            Utf16Text::from("rocket")
        );
        f.owner.end_emoji_search(true).unwrap();
        assert!(Rc::ptr_eq(&external, &f.engine.target().unwrap()));
        assert!(input.is_focused());
        assert_eq!(f.engine.ui_mode(), KeyboardUiMode::Media);
        window.close();
    });
}

#[test]
fn raw_panel_values_follow_source_visibility_and_keep_the_original_target() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine.settings().set_show_suggestions(false);
        for raw in [i32::MIN, -1, 4, 99, i32::MAX] {
            f.engine.set_ui_mode(KeyboardUiMode(raw)).unwrap();
            assert_eq!(f.engine.ui_mode(), KeyboardUiMode(raw));
            assert!(!f.owner.keys_view.is_visible());
            assert!(!f.owner.emoji_panel.is_visible());
            assert!(!f.owner.clipboard_panel.is_visible());
            assert!(!f.owner.editing_panel.is_visible());
            assert!(Rc::ptr_eq(
                &f.engine.target().unwrap(),
                &(f.target.clone() as Rc<dyn ITextInputTarget>)
            ));
            f.engine.set_ui_mode(KeyboardUiMode(raw)).unwrap();
            f.engine.set_ui_mode(KeyboardUiMode::Text).unwrap();
            assert!(f.owner.keys_view.is_visible());
            assert!(Rc::ptr_eq(
                &f.engine.target().unwrap(),
                &(f.target.clone() as Rc<dyn ITextInputTarget>)
            ));
        }
    });
}

#[test]
fn raw_key_popup_select_cancel_and_scaled_layout_preserve_native_text_focus() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_base::{
            input::{MouseButton, RawInputModifiers},
            media::ScaleTransform,
        };
        use ferroui_controls::{StackPanel, TextBox};
        use ferroui_headless::HeadlessWindowExtensions;
        for (scale, hand, split) in [
            (1.0, OneHandedMode::Off, false),
            (0.75, OneHandedMode::Right, true),
            (0.75, OneHandedMode::Off, true),
        ] {
            let f = Fixture::new();
            f.engine.settings().set_one_handed_mode(hand);
            f.engine.settings().set_split_keyboard(split);
            f.owner
                .set_render_transform(Some(ScaleTransform::with_scale(scale, scale).into()));
            let text = TextBox::new();
            text.set_text(Some("XY"));
            text.set_height(48.0);
            let target = TextBoxInputTarget::new(text.clone(), None).unwrap();
            f.owner.attach(target.clone()).unwrap();
            let panel = StackPanel::new();
            panel.children().add(text.clone());
            panel.children().add(f.owner.clone());
            let window = Window::new();
            window.set_width(1000.0);
            window.set_height(600.0);
            window.set_content(Some(Control::boxed(panel)));
            window.show();
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(
                f.owner.keys_view.geometry().split_gap() > 0.0,
                split && hand == OneHandedMode::Off,
                "one-handed mode suppresses the requested split gap"
            );
            assert!(text.focus());
            target.select(1, 0);
            let key = f
                .engine
                .keyboard()
                .unwrap()
                .visible_keys()
                .map(|key| key.unwrap().unwrap())
                .find(|k| k.code() == i32::from(b'a'))
                .unwrap();
            let bounds = key.visible_bounds();
            let key_point = f
                .owner
                .keys_view
                .translate_point(Point::new(bounds.center_x(), bounds.center_y()), &window)
                .unwrap();
            window.mouse_down(key_point, MouseButton::Left, RawInputModifiers::NONE);
            assert!(text.is_focused());
            assert_eq!(text.text().as_deref(), Some("XY"));
            let delay = i64::from(f.engine.settings().long_press_delay()) * 10_000;
            f.clock.advance(delay - 1);
            assert!(f.owner.touch_processor.popup().is_none());
            f.clock.advance(1);
            let popup = f
                .owner
                .touch_processor
                .popup()
                .expect("native press starts long-press alternatives");
            assert!(f.owner.popup.is_visible());
            assert!(!f.owner.preview.is_visible());
            let index = (0..popup.keys().len())
                .find(|&i| {
                    popup.keys().get(i).unwrap().as_string(false).unwrap() == Some("á".into())
                })
                .expect("bundled a popup contains á");
            Dispatcher::current_dispatcher().run_jobs(None);
            // Choose the actual rendered option, exercising overlay placement as well as
            // captured input. Core popup bounds do not supply the pointer coordinates.
            let cell = f.owner.popup_canvas.children().get(index);
            let cell_bounds = cell.bounds();
            assert!(cell_bounds.width > 0.0 && cell_bounds.height > 0.0);
            let selected_point = cell
                .translate_point(
                    Point::new(cell_bounds.width / 2.0, cell_bounds.height / 2.0),
                    &window,
                )
                .unwrap();
            window.mouse_move(selected_point, RawInputModifiers::LEFT_MOUSE_BUTTON);
            assert_eq!(popup.selected_index(), index as i32);
            assert!(text.is_focused());
            window.mouse_up(selected_point, MouseButton::Left, RawInputModifiers::NONE);
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(text.text().as_deref(), Some("XáY"));
            assert_eq!(target.selection_start(), 2);
            assert_eq!(target.selection_length(), 0);
            assert!(text.is_focused());
            assert!(f.owner.touch_processor.popup().is_none());
            assert!(!f.owner.popup.is_visible());
            // Moving below the source cancellation threshold must not commit another key.
            window.mouse_down(key_point, MouseButton::Left, RawInputModifiers::NONE);
            f.clock.advance(delay);
            assert!(f.owner.touch_processor.popup().is_some());
            let bounds = key.visible_bounds();
            let cancel_point = f
                .owner
                .keys_view
                .translate_point(
                    Point::new(bounds.center_x(), bounds.bottom() + bounds.height + 1.0),
                    &window,
                )
                .unwrap();
            window.mouse_move(cancel_point, RawInputModifiers::LEFT_MOUSE_BUTTON);
            assert_eq!(
                f.owner.touch_processor.popup().unwrap().selected_index(),
                -1
            );
            window.mouse_up(cancel_point, MouseButton::Left, RawInputModifiers::NONE);
            assert_eq!(text.text().as_deref(), Some("XáY"));
            assert!(text.is_focused());
            assert!(f.owner.touch_processor.popup().is_none());
            // Raw contact cancellation also dismisses the popup without insertion.
            let contact = window.touch_begin(key_point, RawInputModifiers::NONE);
            f.clock.advance(delay);
            assert!(f.owner.touch_processor.popup().is_some());
            contact.dispose();
            assert!(f.owner.touch_processor.popup().is_none());
            assert!(!f.owner.popup.is_visible());
            assert_eq!(text.text().as_deref(), Some("XáY"));
            assert!(text.is_focused());
            window.close();
            f.owner.detach().unwrap();
            target.detach();
        }
    });
}

#[test]
fn popup_alternative_growth_reports_geometry_mismatch_without_unwinding() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine.settings().set_glide_typing(false);
        f.engine
            .update_layout(634.0, 200.0, Some(f.owner.keys_view.geometry()))
            .unwrap();
        let key = f
            .engine
            .keyboard()
            .unwrap()
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|key| key.code() == 113 && key.has_popup())
            .expect("default keyboard offers hint alternatives");
        let bounds = key.visible_bounds();
        assert!(bounds.width > 0.0 && bounds.height > 0.0);
        assert!(Rc::ptr_eq(
            &f.engine
                .keyboard()
                .unwrap()
                .hit_test(bounds.center_x(), bounds.center_y())
                .unwrap()
                .unwrap(),
            &key
        ));
        f.owner
            .touch_processor
            .pointer_down(1, bounds.center_x(), bounds.center_y())
            .unwrap();
        f.clock
            .advance(i64::from(f.engine.settings().long_press_delay()) * 10_000);
        let popup = f
            .owner
            .touch_processor
            .popup()
            .expect("long press opens popup");
        assert!(f.owner.popup.is_visible());
        let alternatives = popup.keys();
        alternatives.push(alternatives.get(0).unwrap());
        let error = f
            .owner
            .update_popup()
            .expect_err("changed alternative count must report missing bounds");
        assert!(matches!(
            error.downcast_ref::<EngineError>(),
            Some(EngineError::Key(
                ferroboard_core::layouts::KeyDataError::Collection(
                    ferroboard_core::layouts::ResourceTextListError::IndexOutOfRange
                )
            ))
        ));
        f.owner.touch_processor.cancel_all().unwrap();
        f.owner.update_popup().unwrap();
        assert!(!f.owner.popup.is_visible());
        assert!(f.target.text().is_empty());
    });
}
