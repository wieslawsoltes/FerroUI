use super::*;
use crate::test_scheduler as scheduler;
use crate::view_fixture::*;
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::LanguageModels,
    layouts::{ComponentName, KeyboardResources, Subtype, SubtypeLayoutMap},
    settings::KeyboardSettings,
};
use ferroui_base::{
    Point,
    input::{MouseButton, RawInputModifiers},
    threading::Dispatcher,
};
use ferroui_controls::Window;
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
struct Fixture {
    engine: Rc<KeyboardEngine>,
    target: Rc<TextBufferTarget>,
    clock: Rc<scheduler::ManualScheduler>,
    clipboard: Ref<ClipboardPanelView>,
    editing: Ref<EditingPanelView>,
    picker: Ref<SubtypePickerView>,
}
impl Fixture {
    fn new() -> Self {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        engine.set_clock(Rc::new(|| {
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(10)
        }));
        let target = Rc::new(TextBufferTarget::new("alpha beta\ngamma🙂".into(), None));
        engine.attach(target.clone()).unwrap();
        let clock = scheduler::ManualScheduler::new();
        let clipboard = ClipboardPanelView::new(engine.clone(), clock.clone());
        let editing = EditingPanelView::new(engine.clone(), clock.clone());
        let picker = SubtypePickerView::new(engine.clone(), clock.clone());
        Self {
            engine,
            target,
            clock,
            clipboard,
            editing,
            picker,
        }
    }
    fn subtypes(&self) -> Vec<Arc<Subtype>> {
        let german = Subtype::from_values(ferroboard_core::layouts::SubtypeValues {
            display_name: Some("Deutsch🙂".into()),
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("org.florisboard.layouts", "qwertz"),
                    ..Default::default()
                },
            ))),
            ..Subtype::new("de-DE".into()).values()
        });
        let duplicate = Subtype::from_values(ferroboard_core::layouts::SubtypeValues {
            display_name: Some("Alternate same id".into()),
            ..(*Subtype::shared_default()).values()
        });
        let french = Subtype::from_values(ferroboard_core::layouts::SubtypeValues {
            display_name: Some("Français".into()),
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("org.florisboard.layouts", "absent"),
                    ..Default::default()
                },
            ))),
            ..Subtype::new("fr-FR".into()).values()
        });
        vec![
            Subtype::shared_default(),
            Arc::new(german),
            Arc::new(duplicate),
            Arc::new(french),
        ]
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.clipboard.release_subscriptions();
        self.editing.release_subscriptions();
        self.picker.release_visual_children();
        self.engine.dispose().unwrap();
    }
}
fn show(c: Ref<Control>, width: f64, height: f64) -> Ref<Window> {
    let w = Window::new();
    w.set_width(width);
    w.set_height(height);
    w.set_content(Some(Control::boxed(c)));
    w.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    w
}
fn click(c: &Control, w: &Window) {
    Dispatcher::current_dispatcher().run_jobs(None);
    let point = c
        .translate_point(
            Point::new(c.bounds().width / 2.0, c.bounds().height / 2.0),
            w,
        )
        .unwrap();
    w.mouse_down(point, MouseButton::Left, RawInputModifiers::empty());
    w.mouse_up(point, MouseButton::Left, RawInputModifiers::empty());
}
fn state(f: &Fixture) -> Value {
    let editor = f.engine.editor();
    json!({"text":f.target.text().units(),"start":f.target.selection_start(),"length":f.target.selection_length(),"selecting":editor.as_ref().map(|e|e.is_selecting()),"panel":f.engine.ui_mode().0,"canUndo":editor.as_ref().map(|e|e.can_undo()),"canRedo":editor.as_ref().map(|e|e.can_redo())})
}
#[test]
fn unchanged_source_panel_trees_and_editing_callback_corpus() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let input = include_str!("../../../tests/fixtures/panels.jsonl");
        assert_eq!(input.lines().count(), 60);
        let mut presets = None;
        let mut window = None;
        for line in input.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            match row["op"].as_str().unwrap() {
                "clipboard" => {
                    f.clipboard.set_is_visible(false);
                    f.engine.clipboard_history().clear(true);
                    f.engine
                        .settings()
                        .set_clipboard_history_enabled(row["enabled"].as_bool().unwrap());
                    for i in 0..row["count"].as_u64().unwrap() {
                        let text = match i {
                            0 => "one\ntwo🙂".into(),
                            1 => "pinned".into(),
                            _ => Utf16Text::from("x".repeat(400).as_str()),
                        };
                        let item = f
                            .engine
                            .clipboard_history()
                            .add(Some(&text))
                            .unwrap()
                            .unwrap();
                        if i == 1 {
                            f.engine.clipboard_history().set_pinned(&item, true);
                        }
                    }
                    f.clipboard.set_header_height(float(&row["height"]));
                    f.clipboard.set_is_visible(true);
                    f.clipboard
                        .set_theme(KeyboardThemes::get(row["theme"].as_str().unwrap()).unwrap())
                        .unwrap();
                    compare(&f.clipboard, &row["tree"]);
                }
                "editing" => {
                    f.engine
                        .settings()
                        .set_key_repeat_interval(row["interval"].as_i64().unwrap() as i32);
                    f.engine
                        .input_key(&*TextKeyDataFactory::function(KeyCode::ClipboardSelect))
                        .unwrap();
                    if f.engine.editor().unwrap().is_selecting()
                        != row["selecting"].as_bool().unwrap()
                    {
                        f.engine
                            .input_key(&*TextKeyDataFactory::function(KeyCode::ClipboardSelect))
                            .unwrap();
                    }
                    f.editing
                        .set_theme(KeyboardThemes::get(row["theme"].as_str().unwrap()).unwrap())
                        .unwrap();
                    compare(&f.editing, &row["tree"]);
                }
                "picker" => {
                    if presets.is_none() {
                        let p = f.subtypes();
                        f.engine.set_subtypes(p.clone()).unwrap();
                        presets = Some(p);
                    }
                    let p = presets.as_ref().unwrap();
                    f.engine
                        .set_active_subtype(p[row["active"].as_u64().unwrap() as usize].clone())
                        .unwrap();
                    f.picker
                        .set_theme(KeyboardThemes::get(row["theme"].as_str().unwrap()).unwrap());
                    f.picker.open().unwrap();
                    compare(&f.picker, &row["tree"]);
                    f.picker.close();
                }
                "picker-close" => assert_eq!(json!(f.picker.is_visible()), row["visible"]),
                "editing-click" => {
                    if window.is_none() {
                        f.engine
                            .set_active_subtype(Subtype::shared_default())
                            .unwrap();
                        f.engine.attach(f.target.clone()).unwrap();
                        f.target
                            .replace(
                                0,
                                f.target.text().len() as i32,
                                Some(&"alpha beta\ngamma🙂".into()),
                            )
                            .unwrap();
                        f.target.select(f.target.text().len() as i32, 0);
                        if f.engine.editor().unwrap().is_selecting() {
                            f.engine
                                .input_key(&*TextKeyDataFactory::function(KeyCode::ClipboardSelect))
                                .unwrap();
                        }
                        f.editing.set_theme(KeyboardThemes::floris_day()).unwrap();
                        window = Some(show(f.editing.clone().upcast(), 700.0, 300.0));
                    }
                    let button = f
                        .editing
                        .content_grid()
                        .children()
                        .get(row["index"].as_u64().unwrap() as usize);
                    click(&button, window.as_ref().unwrap());
                    assert_eq!(state(&f), row["state"], "click index {}", row["index"]);
                }
                _ => panic!("unknown fixture op"),
            }
        }
        if let Some(w) = window {
            w.close();
        }
    });
}

fn clipboard_cards(f: &Fixture) -> Ref<UniformWrapPanel> {
    Control::from_boxed(f.clipboard.scroller.content().as_ref().unwrap())
        .unwrap()
        .downcast::<UniformWrapPanel>()
        .unwrap()
}
fn card_actions(card: &TouchButton) -> Ref<StackPanel> {
    let content = card.content_grid().children().get(0);
    let grid = content.downcast_ref::<PaddedGrid>().unwrap().content_grid();
    grid.children().get(1).downcast::<StackPanel>().unwrap()
}
fn picker_list(f: &Fixture) -> Ref<StackPanel> {
    let card = f.picker.children().get(0);
    let scroller = card.downcast_ref::<Border>().unwrap().child().unwrap();
    Control::from_boxed(
        scroller
            .downcast_ref::<ScrollViewer>()
            .unwrap()
            .content()
            .as_ref()
            .unwrap(),
    )
    .unwrap()
    .downcast::<StackPanel>()
    .unwrap()
}
#[test]
fn raw_clipboard_nested_pin_delete_paste_clear_and_visibility_guard() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.target
            .replace(0, f.target.text().len() as i32, Some(&Utf16Text::default()))
            .unwrap();
        let first = f
            .engine
            .clipboard_history()
            .add(Some(&"first🙂".into()))
            .unwrap()
            .unwrap();
        let pinned = f
            .engine
            .clipboard_history()
            .add(Some(&"pinned".into()))
            .unwrap()
            .unwrap();
        f.engine.clipboard_history().set_pinned(&pinned, true);
        f.clipboard.set_theme(KeyboardThemes::floris_day()).unwrap();
        let window = show(f.clipboard.clone().upcast(), 600.0, 300.0);
        let cards = clipboard_cards(&f);
        assert_eq!(cards.children().count(), 2);
        let card = cards.children().get(1).downcast::<TouchButton>().unwrap();
        let pin = card_actions(&card).children().get(0);
        click(&pin, &window);
        assert!(first.is_pinned());
        assert!(f.target.text().is_empty());
        let cards = clipboard_cards(&f);
        let card = cards.children().get(1).downcast::<TouchButton>().unwrap();
        click(&card, &window);
        assert_eq!(f.target.text(), "first🙂".into());
        let remove = card_actions(&card).children().get(1);
        click(&remove, &window);
        assert_eq!(f.target.text(), "first🙂".into());
        assert!(
            !f.engine
                .clipboard_history()
                .items()
                .unwrap()
                .iter()
                .any(|i| Rc::ptr_eq(i, &first))
        );
        click(&f.clipboard.header.children().get(3), &window);
        assert_eq!(f.engine.clipboard_history().items().unwrap().len(), 1);
        assert!(Rc::ptr_eq(
            &f.engine.clipboard_history().items().unwrap()[0],
            &pinned
        ));
        let previous =
            Control::from_boxed(f.clipboard.scroller.content().as_ref().unwrap()).unwrap();
        f.clipboard.set_is_visible(false);
        f.engine
            .clipboard_history()
            .add(Some(&"hidden change".into()))
            .unwrap();
        let current =
            Control::from_boxed(f.clipboard.scroller.content().as_ref().unwrap()).unwrap();
        assert_eq!(previous, current);
        f.clipboard.set_is_visible(true);
        assert_eq!(clipboard_cards(&f).children().count(), 1);
        f.clipboard.refresh().unwrap();
        assert_eq!(clipboard_cards(&f).children().count(), 2);
        click(&f.clipboard.header.children().get(2), &window);
        assert!(!f.engine.settings().clipboard_history_enabled());
        f.engine.clipboard_history().clear(true);
        let empty = Control::from_boxed(f.clipboard.scroller.content().as_ref().unwrap()).unwrap();
        assert_eq!(
            empty.downcast_ref::<TextBlock>().unwrap().text(),
            nullable_text(KeyboardStrings::clipboard_disabled()).unwrap()
        );
        window.close();
    });
}
#[test]
fn editing_repeat_thresholds_settings_snapshots_and_padded_native_layout() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine.settings().set_key_repeat_delay(300);
        f.engine.settings().set_key_repeat_interval(15);
        f.editing.set_theme(KeyboardThemes::floris_day()).unwrap();
        let window = show(f.editing.clone().upcast(), 700.0, 300.0);
        let grid = f.editing.content_grid();
        let arrow = grid.children().get(3).downcast::<TouchButton>().unwrap();
        assert_eq!(arrow.repeat_interval(), Duration::from_millis(40));
        assert_eq!(arrow.repeat_delay(), Some(Duration::from_millis(300)));
        let body = f.editing.body.bounds();
        assert_eq!(body.width, 700.0);
        assert_eq!(grid.bounds().x, 4.0);
        assert_eq!(grid.bounds().y, 4.0);
        assert_eq!(grid.bounds().width, 692.0);
        assert_eq!(grid.bounds().height, 292.0);
        let point = arrow
            .translate_point(
                Point::new(arrow.bounds().width / 2.0, arrow.bounds().height / 2.0),
                &window,
            )
            .unwrap();
        let before = f.target.selection_start();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::empty());
        f.clock.advance(2_999_999);
        assert_eq!(f.target.selection_start(), before);
        f.clock.advance(1);
        assert!(f.target.selection_start() < before);
        let once = f.target.selection_start();
        f.clock.advance(399_999);
        assert_eq!(f.target.selection_start(), once);
        f.clock.advance(1);
        assert!(f.target.selection_start() < once);
        let twice = f.target.selection_start();
        window.mouse_up(point, MouseButton::Left, RawInputModifiers::empty());
        assert_eq!(f.target.selection_start(), twice);
        f.clock.advance(4_000_000);
        assert_eq!(f.target.selection_start(), twice);
        f.engine.settings().set_key_repeat_delay(500);
        f.engine.settings().set_key_repeat_interval(80);
        assert_eq!(arrow.repeat_interval(), Duration::from_millis(40));
        f.editing.set_theme(KeyboardThemes::floris_day()).unwrap();
        assert_eq!(
            grid.children()
                .get(3)
                .downcast_ref::<TouchButton>()
                .unwrap()
                .repeat_interval(),
            Duration::from_millis(80)
        );
        window.close();
    });
}
#[test]
fn picker_raw_capture_card_background_close_and_manage_event_order() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let presets = f.subtypes();
        f.engine.set_subtypes(presets.clone()).unwrap();
        f.picker.set_theme(KeyboardThemes::floris_day());
        f.picker.open().unwrap();
        let window = show(f.picker.clone().upcast(), 700.0, 360.0);
        let list = picker_list(&f);
        assert_eq!(list.children().count(), 5);
        let card = f.picker.children().get(0);
        let origin = card
            .translate_point(Point::new(card.bounds().width / 2.0, 2.0), &window)
            .unwrap();
        window.mouse_down(origin, MouseButton::Left, RawInputModifiers::empty());
        window.mouse_up(origin, MouseButton::Left, RawInputModifiers::empty());
        assert!(f.picker.is_visible());
        click(&list.children().get(1), &window);
        assert!(!f.picker.is_visible());
        assert_eq!(f.engine.active_subtype().id(), presets[1].id());
        f.picker.open().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let subscription = f.picker.manage_requested(move |view| {
            assert!(!view.is_visible());
            count.set(count.get() + 1);
            view.set_is_visible(true);
        });
        let list = picker_list(&f);
        click(&list.children().get(4), &window);
        assert_eq!(calls.get(), 1);
        assert!(f.picker.is_visible());
        f.picker.remove_manage_requested(subscription);
        window.mouse_down(
            Point::new(5.0, 5.0),
            MouseButton::Left,
            RawInputModifiers::empty(),
        );
        assert!(!f.picker.is_visible());
        window.mouse_up(
            Point::new(5.0, 5.0),
            MouseButton::Left,
            RawInputModifiers::empty(),
        );
        window.close();
    });
}
#[test]
fn native_weak_subscriptions_preserve_partial_failures_and_cleanup_on_drop() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.clipboard.set_theme(KeyboardThemes::floris_day()).unwrap();
        let old = f.clipboard.scroller.content();
        f.clipboard.set_is_visible(false);
        f.engine
            .clipboard_history()
            .add(Some(&Utf16Text::from_units(vec![0xd800])))
            .unwrap();
        assert!(matches!(
            f.clipboard.refresh(),
            Err(ViewError::UnpairedSurrogate)
        ));
        let old = Control::from_boxed(old.as_ref().unwrap()).unwrap();
        let retained =
            Control::from_boxed(f.clipboard.scroller.content().as_ref().unwrap()).unwrap();
        assert_eq!(old, retained);
        let history = f.engine.clipboard_history();
        let count = history.changed().handler_count();
        let view = ClipboardPanelView::new(f.engine.clone(), f.clock.clone());
        let weak = view.downgrade();
        drop(view);
        assert!(weak.upgrade().is_none());
        assert_eq!(history.changed().handler_count(), count);
        let state_count = f.engine.state_changed().handler_count();
        let view = EditingPanelView::new(f.engine.clone(), f.clock.clone());
        view.set_theme(KeyboardThemes::floris_day()).unwrap();
        let weak = view.downgrade();
        drop(view);
        assert!(weak.upgrade().is_none());
        assert_eq!(f.engine.state_changed().handler_count(), state_count);
    });
}

#[test]
fn native_editor_focus_survives_clipboard_editing_and_subtype_panel_actions() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_controls::TextBox;
        let f = Fixture::new();
        let text = TextBox::new();
        text.set_text(Some("abc"));
        text.set_height(36.0);
        let adapter = TextBoxInputTarget::new(text.clone(), None).unwrap();
        adapter.select(3, 0);
        f.engine.attach(adapter.clone()).unwrap();
        f.engine
            .clipboard_history()
            .add(Some(&"Z🙂".into()))
            .unwrap();
        f.clipboard.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.clipboard.set_height(300.0);
        f.editing.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.editing.set_height(300.0);
        f.editing.set_is_visible(false);
        f.picker.set_height(300.0);
        f.picker.set_is_visible(false);
        let body = Grid::new();
        body.children().add(f.clipboard.clone());
        body.children().add(f.editing.clone());
        body.children().add(f.picker.clone());
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(body);
        let window = show(panel.upcast(), 700.0, 336.0);
        assert!(text.focus());
        click(&clipboard_cards(&f).children().get(0), &window);
        assert_eq!(text.text().as_deref(), Some("abcZ🙂"));
        assert!(text.is_focused());
        f.clipboard.set_is_visible(false);
        f.editing.set_is_visible(true);
        click(&f.editing.content_grid().children().get(3), &window);
        assert_eq!(adapter.selection_start(), 4);
        assert!(text.is_focused());
        click(&f.editing.content_grid().children().get(4), &window);
        assert!(f.engine.editor().unwrap().is_selecting());
        assert!(text.is_focused());
        let presets = f.subtypes();
        f.engine.set_subtypes(presets.clone()).unwrap();
        f.editing.set_is_visible(false);
        f.picker.open().unwrap();
        click(&picker_list(&f).children().get(1), &window);
        assert_eq!(f.engine.active_subtype().id(), presets[1].id());
        assert!(text.is_focused());
        window.close();
        f.engine.detach().unwrap();
        adapter.detach();
    });
}

#[test]
fn subtype_picker_automation_names_actions_and_localization_preserve_editor_focus() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_controls::{
            TextBox,
            automation::{
                peers::{AutomationControlType, ControlAutomationPeer},
                provider::IInvokeProvider,
            },
        };
        struct RestoreManageLabel(Option<Utf16Text>);
        impl Drop for RestoreManageLabel {
            fn drop(&mut self) {
                KeyboardStrings::set_manage_languages(self.0.take());
            }
        }
        let _restore = RestoreManageLabel(KeyboardStrings::manage_languages());
        let f = Fixture::new();
        let presets = f.subtypes();
        f.engine.set_subtypes(presets.clone()).unwrap();
        let text = TextBox::new();
        text.set_text(Some("abc🙂"));
        text.set_height(36.0);
        let adapter = TextBoxInputTarget::new(text.clone(), None).unwrap();
        f.engine.attach(adapter.clone()).unwrap();
        adapter.select(1, 1);
        f.picker.set_height(300.0);
        let root = StackPanel::new();
        root.children().add(text.clone());
        root.children().add(f.picker.clone());
        let window = show(root.upcast(), 700.0, 336.0);
        assert!(text.focus());
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let token = f.picker.manage_requested(move |view| {
            assert!(!view.is_visible(), "close precedes ManageRequested");
            count.set(count.get() + 1);
        });
        for (iteration, label) in [
            Some("Gérer les langues 🌍"),
            Some("إدارة اللغات 🌍"),
            Some(""),
            None,
        ]
        .into_iter()
        .enumerate()
        {
            KeyboardStrings::set_manage_languages(label.map(Into::into));
            f.picker.open().unwrap();
            let list = picker_list(&f);
            let expected = [
                "English (United States)  ·  QWERTY",
                "Deutsch🙂  ·  QWERTZ",
                "Alternate same id  ·  QWERTY",
                "Français  ·  absent",
            ];
            for (i, expected) in expected.into_iter().enumerate() {
                let peer = ControlAutomationPeer::create_peer_for_element(&list.children().get(i));
                assert_eq!(
                    peer.get_automation_control_type(),
                    AutomationControlType::Button
                );
                assert_eq!(peer.get_name(), expected);
                assert!(!peer.is_keyboard_focusable());
            }
            let choice = ControlAutomationPeer::create_peer_for_element(&list.children().get(1));
            choice
                .get_provider::<dyn IInvokeProvider>()
                .unwrap()
                .invoke()
                .unwrap();
            assert_eq!(f.engine.active_subtype().id(), presets[1].id());
            assert!(!f.picker.is_visible());
            assert!(text.is_focused());
            assert_eq!(text.text().as_deref(), Some("abc🙂"));
            assert_eq!(
                (adapter.selection_start(), adapter.selection_length()),
                (1, 1)
            );
            f.picker.open().unwrap();
            let list = picker_list(&f);
            let manage = ControlAutomationPeer::create_peer_for_element(&list.children().get(4));
            assert_eq!(
                manage.get_automation_control_type(),
                AutomationControlType::Button
            );
            assert_eq!(manage.get_name(), label.unwrap_or(""));
            assert!(!manage.is_keyboard_focusable());
            manage
                .get_provider::<dyn IInvokeProvider>()
                .unwrap()
                .invoke()
                .unwrap();
            assert_eq!(calls.get(), iteration + 1);
            assert!(!f.picker.is_visible());
            assert!(text.is_focused());
            assert_eq!(text.text().as_deref(), Some("abc🙂"));
            assert_eq!(
                (adapter.selection_start(), adapter.selection_length()),
                (1, 1)
            );
        }
        f.picker.remove_manage_requested(token);
        window.close();
        f.engine.detach().unwrap();
        adapter.detach();
    });
}

#[test]
fn subtype_picker_preserves_null_label_fallback_and_empty_label() {
    crate::test_support::run(|| {
        use ferroboard_core::layouts::{KeyboardLayout, LayoutArrangement};
        let f = Fixture::new();
        for component in [None, Some(""), Some("fallback")] {
            for label in [None, Some(""), Some("label")] {
                let name =
                    ComponentName::new_nullable(Some("custom".into()), component.map(Into::into));
                let layout = KeyboardLayout::new_nullable(
                    LayoutType::Characters,
                    name.extension_id.clone(),
                    name.component_id.clone(),
                    Some(LayoutArrangement::empty()),
                );
                layout.set_label(label.map(Into::into));
                f.engine.resources().add_layout(Rc::new(layout));
                let subtype = Subtype::from_values(ferroboard_core::layouts::SubtypeValues {
                    display_name: Some("Test".into()),
                    layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                        ferroboard_core::layouts::SubtypeLayoutMapValues {
                            characters: name,
                            ..Default::default()
                        },
                    ))),
                    ..Subtype::new("en-US".into()).values()
                });
                f.engine.set_subtypes([Arc::new(subtype)]).unwrap();
                f.picker.open().unwrap();
                let list = picker_list(&f);
                let first = list.children().get(0);
                let button = first.downcast_ref::<TouchButton>().unwrap();
                let content = button.content_grid().children().get(0);
                assert_eq!(
                    content.downcast_ref::<TextBlock>().unwrap().text(),
                    Some(format!("Test  ·  {}", label.or(component).unwrap_or("")))
                );
                assert_eq!(
                    ferroui_controls::automation::peers::ControlAutomationPeer::create_peer_for_element(button)
                        .get_name(),
                    format!("Test  ·  {}", label.or(component).unwrap_or(""))
                );
                f.picker.close();
            }
        }
    });
}
