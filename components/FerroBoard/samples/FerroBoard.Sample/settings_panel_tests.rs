use super::*;
use ferroboard_core::{
    engine::LanguageModels,
    layouts::{ComponentName, KeyboardResources, Subtype, SubtypeLayoutMap},
    settings::ThemeMode,
};
use ferroui_base::{
    Visual, interactivity::RoutedEventArgs, metadata::into_markup_value, threading::Dispatcher,
};
use ferroui_controls::{
    CheckBox, ComboBox, Control, ScrollViewer, SelectionChangedEventArgs, Slider, ToggleSwitch,
    Window,
    primitives::{SelectingItemsControl, ToggleButton},
};
use serde_json::{Value, json};
use std::sync::Arc;
fn resources(name: &str) -> Rc<KeyboardResources> {
    let r = Rc::new(KeyboardResources::create_empty());
    let s = Subtype::new("en-US".into());
    s.set_display_name(Some(name.into()));
    s.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
        ferroboard_core::layouts::SubtypeLayoutMapValues {
            characters: ComponentName::new("test", name),
            ..SubtypeLayoutMap::default().values()
        },
    ))));
    r.add_subtype_preset(Arc::new(s));
    r
}
fn state(s: &KeyboardSettings, m: &Arc<LanguageModels>) -> Value {
    json!({"numberRow":s.number_row(),"keyHeight":s.key_height(),"theme":s.theme_mode().0,"subtypes":s.subtypes().borrow().iter().map(|s|s.as_deref().map(|s|s.encode_utf16().collect::<Vec<_>>())).collect::<Vec<_>>(),"words":m.get_user_dictionary(&"en".into()).get_count(&"keepword".into())})
}
fn field(panel: &SettingsPanel, label: &str) -> Rc<SettingsFieldView> {
    panel
        .fields
        .borrow()
        .iter()
        .find(|v| v.model.label() == label)
        .unwrap()
        .clone()
}
fn find(root: Ref<Visual>, name: &str) -> Option<Ref<Control>> {
    if root.get_type().name() == name {
        return root.cast::<Control>();
    }
    for i in 0..root.visual_children().count() {
        if let Some(found) = find(root.visual_children().get(i), name) {
            return Some(found);
        }
    }
    None
}
fn compare_state(a: Value, b: &Value, operation: &str) {
    assert_eq!(
        a["keyHeight"].as_f64().unwrap().to_bits(),
        b["keyHeight"].as_f64().unwrap().to_bits(),
        "{operation}: {a} != {b}"
    );
    for key in ["numberRow", "theme", "subtypes", "words"] {
        assert_eq!(a[key], b[key], "{key}");
    }
}
#[test]
fn public_constructor_attach_duplicate_subscriptions_and_retained_native_controls_match_source_trace()
 {
    crate::tests::run(|| {
        let a = Rc::new(KeyboardSettings::default());
        a.set_number_row(true);
        a.set_key_height(42.0);
        a.set_theme_mode(ThemeMode::Dark);
        let b = Rc::new(KeyboardSettings::default());
        b.set_key_height(87.0);
        b.set_theme_mode(ThemeMode::Light);
        let ma = LanguageModels::new();
        let mb = LanguageModels::new();
        let ea = KeyboardEngine::new_explicit(a.clone(), resources("alpha"), ma.clone()).unwrap();
        let eb = KeyboardEngine::new_explicit(b.clone(), resources("beta"), mb.clone()).unwrap();
        let ka = OnScreenKeyboard::new(Some(ea.clone())).unwrap();
        let kb = OnScreenKeyboard::new(Some(eb.clone())).unwrap();
        let panel = SettingsPanel::new();
        let original_content = panel.content();
        let original_body = panel.body();
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(1000.0);
        window.set_content(into_markup_value(panel.clone()));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let trace = Rc::new(RefCell::new(Vec::new()));
        let mut toggle = None;
        let mut slider = None;
        let mut combo = None;
        let mut old_filter = None;
        let mut old_check = None;
        let mut reset = None;
        let mut forget = None;
        for line in include_str!("../../tests/fixtures/settings-panel-lifecycle.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let op = row["operation"].as_str().unwrap();
            trace.borrow_mut().clear();
            let error = match op {
                "constructor" => None,
                "attach-a" => {
                    panel.attach(ka.clone()).unwrap();
                    ea.dispose().unwrap();
                    Dispatcher::ui_thread().run_jobs(None);
                    toggle = Some(
                        field(&panel, "Number row")
                            .input
                            .cast::<ToggleSwitch>()
                            .unwrap(),
                    );
                    slider = Some(field(&panel, "Key height").input.cast::<Slider>().unwrap());
                    combo = Some(
                        field(&panel, "Theme mode")
                            .input
                            .cast::<ComboBox>()
                            .unwrap(),
                    );
                    let language = panel.language.borrow();
                    let language = language.as_ref().unwrap();
                    old_filter = Some(language._filter.clone());
                    old_check = Some(
                        find(language._choices.clone().upcast(), "LanguageCheckBox")
                            .unwrap()
                            .cast::<CheckBox>()
                            .unwrap(),
                    );
                    let group = panel
                        .body()
                        .children()
                        .get(52)
                        .cast::<StackPanel>()
                        .unwrap();
                    reset = Some(group.children().get(0).cast::<Button>().unwrap());
                    forget = Some(group.children().get(1).cast::<Button>().unwrap());
                    None
                }
                "attach-b" => {
                    panel.attach(kb.clone()).unwrap();
                    eb.dispose().unwrap();
                    None
                }
                "old-a-setting" => {
                    a.set_font_scale(1.4);
                    None
                }
                "current-b-setting" => {
                    b.set_font_scale(1.3);
                    None
                }
                "attach-b-again" => {
                    panel.attach(kb.clone()).unwrap();
                    None
                }
                "b-two-refreshes" => {
                    b.set_font_scale(1.2);
                    None
                }
                "old-toggle-a" => {
                    let t = toggle.as_ref().unwrap();
                    t.raise_event(&RoutedEventArgs::with_event(
                        ToggleButton::is_checked_changed_event(),
                    ));
                    None
                }
                "old-slider-a" => {
                    slider.as_ref().unwrap().set_range_value(61.25);
                    None
                }
                "set-b-raw" => {
                    b.set_key_height(24.0);
                    assert_eq!(
                        field(&panel, "Key height")
                            .input
                            .cast::<Slider>()
                            .unwrap()
                            .value(),
                        36.0
                    );
                    None
                }
                "old-slider-a-to-boundary" => {
                    slider.as_ref().unwrap().set_range_value(36.0);
                    None
                }
                "old-combo-a" => {
                    combo
                        .as_ref()
                        .unwrap()
                        .raise_event(&SelectionChangedEventArgs::new(
                            Some(SelectingItemsControl::selection_changed_event()),
                            Vec::new(),
                            Vec::new(),
                        ));
                    None
                }
                "old-language-a" => {
                    old_check.as_ref().unwrap().set_is_checked(Some(true));
                    None
                }
                "old-language-filter-a" => {
                    old_filter.as_ref().unwrap().set_text(Some("alpha"));
                    Dispatcher::ui_thread().run_jobs(None);
                    None
                }
                "old-forget-a" => {
                    ma.get_user_dictionary(&"en".into())
                        .learn(&"keepword".into(), None);
                    mb.get_user_dictionary(&"en".into())
                        .learn(&"keepword".into(), None);
                    forget
                        .as_ref()
                        .unwrap()
                        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
                    None
                }
                "old-reset-a" => {
                    reset
                        .as_ref()
                        .unwrap()
                        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
                    None
                }
                "attach-null" => {
                    assert!(matches!(
                        panel.attach_nullable(None),
                        Err(SettingsPanelError::NullReference)
                    ));
                    Some("NullReferenceException")
                }
                "old-toggle-after-null" => {
                    let t = toggle.as_ref().unwrap();
                    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        t.set_is_checked(Some(false))
                    }));
                    assert!(failed.is_err());
                    Some("NullReferenceException")
                }
                "recover-a" => {
                    panel.attach(ka.clone()).unwrap();
                    None
                }
                "a-duplicate-refresh" => {
                    a.set_font_scale(1.1);
                    None
                }
                _ => panic!("unknown source operation {op}"),
            };
            assert_eq!(json!(error), row["error"], "{op}");
            compare_state(state(&a, &ma), &row["a"], op);
            compare_state(state(&b, &mb), &row["b"], op);
            assert_eq!(
                panel.body().children().count(),
                row["children"].as_u64().unwrap() as usize,
                "{op}"
            );
            let expected_current = row["current"].as_str();
            let current = panel.keyboard.borrow();
            assert_eq!(
                current.as_ref().map(|k| if *k == ka {
                    "a"
                } else if *k == kb {
                    "b"
                } else {
                    "unexpected"
                }),
                expected_current,
                "{op}"
            );
            drop(current);
            if row["children"] == 54 {
                let expected_fields = row["controls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| {
                        ["toggle", "slider", "combo"].contains(&c["kind"].as_str().unwrap())
                    })
                    .collect::<Vec<_>>();
                let fields = panel.fields.borrow();
                assert_eq!(fields.len(), expected_fields.len());
                for (field, expected) in fields.iter().zip(expected_fields) {
                    assert_eq!(
                        field.model.label(),
                        expected["label"].as_str().unwrap(),
                        "{op}"
                    );
                    let value = match field.model.value() {
                        crate::settings_fields::FieldValue::Bool(v) => json!(v),
                        crate::settings_fields::FieldValue::Number(v) => json!(v),
                        crate::settings_fields::FieldValue::Index(v) => json!(v),
                    };
                    // The source recorder has no native range coercion; compare its raw
                    // assigned values with the adapter's raw presentation projection.
                    if let Some(number) = value.as_f64() {
                        assert_eq!(
                            number.to_bits(),
                            expected["value"].as_f64().unwrap().to_bits(),
                            "{op}: {}",
                            field.model.label()
                        );
                    } else {
                        assert_eq!(value, expected["value"], "{op}: {}", field.model.label());
                    }
                }
            }
            assert_eq!(
                panel.refreshers.len(),
                row["refreshers"].as_u64().unwrap() as usize,
                "{op}"
            );
            let subscriptions = panel.subscriptions.borrow();
            assert_eq!(
                json!([
                    subscriptions
                        .iter()
                        .filter(|(s, _)| Rc::ptr_eq(s, &a))
                        .count(),
                    subscriptions
                        .iter()
                        .filter(|(s, _)| Rc::ptr_eq(s, &b))
                        .count()
                ]),
                row["subscriptions"],
                "{op}"
            );
            drop(subscriptions);
            assert_eq!(panel.body(), original_body);
            assert!(
                ferroui_base::metadata::from_markup_value::<Ref<ScrollViewer>>(&panel.content())
                    == ferroui_base::metadata::from_markup_value::<Ref<ScrollViewer>>(
                        &original_content
                    )
            );
            assert!(!panel.context.is_refreshing());
            assert_eq!(json!(*trace.borrow()), row["refresh"], "{op}");
            if ["attach-a", "attach-b", "attach-b-again", "recover-a"].contains(&op) {
                let recorded = trace.clone();
                let context = panel.context.clone();
                panel.refreshers.observe_actions(move |index, action| {
                    let recorded = recorded.clone();
                    let context = context.clone();
                    Rc::new(move || {
                        recorded.borrow_mut().push(format!(
                            "{index}:{}",
                            if context.is_refreshing() {
                                "True"
                            } else {
                                "False"
                            }
                        ));
                        action()
                    })
                });
            }
        }
        panel.release_native_ownership();
        assert!(panel.subscriptions.borrow().is_empty());
        window.close();
        ka.detach().unwrap();
        kb.detach().unwrap();
    });
}

fn retained_control(panel: &SettingsPanel, kind: &str) -> Ref<Control> {
    match kind {
        "toggle" => field(panel, "Number row").input.clone(),
        "slider" => field(panel, "Key height").input.clone(),
        "combo" => field(panel, "Theme mode").input.clone(),
        "theme" => field(panel, "Light theme").input.clone(),
        "language" => find(
            panel
                .language
                .borrow()
                .as_ref()
                .unwrap()
                ._choices
                .clone()
                .upcast(),
            "LanguageCheckBox",
        )
        .unwrap(),
        "filter" => panel
            .language
            .borrow()
            .as_ref()
            .unwrap()
            ._filter
            .clone()
            .upcast(),
        "reset" | "forget" => panel
            .body()
            .children()
            .get(52)
            .cast::<StackPanel>()
            .unwrap()
            .children()
            .get(if kind == "reset" { 0 } else { 1 }),
        _ => panic!("unknown source retention kind {kind}"),
    }
}
struct RetentionGraph {
    retained: Option<Ref<Control>>,
    panel: ferroui_base::WeakRef<SettingsPanel>,
    first: ferroui_base::WeakRef<OnScreenKeyboard>,
    current: ferroui_base::WeakRef<OnScreenKeyboard>,
}
impl RetentionGraph {
    fn state(&self) -> Value {
        json!({"panel":self.panel.upgrade().is_some(),"firstKeyboard":self.first.upgrade().is_some(),"currentKeyboard":self.current.upgrade().is_some()})
    }
}
fn retention_graph(kind: &str) -> RetentionGraph {
    let ea = KeyboardEngine::new_explicit(
        Rc::new(KeyboardSettings::default()),
        resources("alpha"),
        LanguageModels::new(),
    )
    .unwrap();
    let eb = KeyboardEngine::new_explicit(
        Rc::new(KeyboardSettings::default()),
        resources("beta"),
        LanguageModels::new(),
    )
    .unwrap();
    let ka = OnScreenKeyboard::new(Some(ea.clone())).unwrap();
    let kb = OnScreenKeyboard::new(Some(eb.clone())).unwrap();
    let panel = SettingsPanel::new();
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(1000.0);
    window.set_content(into_markup_value(panel.clone()));
    window.show();
    panel.attach(ka.clone()).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    let retained = retained_control(&panel, kind);
    panel.attach(kb.clone()).unwrap();
    window.set_content(None);
    window.close();
    Dispatcher::ui_thread().run_jobs(None);
    RetentionGraph {
        retained: Some(retained),
        panel: panel.downgrade(),
        first: ka.downgrade(),
        current: kb.downgrade(),
    }
}
#[test]
fn native_retained_widgets_release_panel_and_keyboard_without_explicit_shutdown() {
    crate::tests::run(|| {
        for kind in [
            "toggle", "slider", "combo", "theme", "language", "filter", "reset", "forget",
        ] {
            let mut graph = retention_graph(kind);
            assert_eq!(
                graph.state(),
                json!({"panel":false,"firstKeyboard":false,"currentKeyboard":false}),
                "retained {kind}"
            );
            graph.retained.take();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                graph.state(),
                json!({"panel":false,"firstKeyboard":false,"currentKeyboard":false}),
                "released {kind}"
            );
        }
    });
}
#[test]
fn native_callback_keeps_owner_during_reentrant_explicit_release_and_reattach_starts_a_new_owner() {
    crate::tests::run(|| {
        // Isolate the in-flight root from source settings-event snapshots and UI cycles.
        let bare = SettingsPanel::new();
        let weak_bare = bare.downgrade();
        let capture = SettingsCallbackOwner::new(bare.clone());
        let frame = capture.retain().unwrap();
        drop(bare);
        capture.release();
        assert!(weak_bare.upgrade().is_some());
        drop(frame);
        assert!(weak_bare.upgrade().is_none());
        let engine = KeyboardEngine::new_explicit(
            Rc::new(KeyboardSettings::default()),
            resources("alpha"),
            LanguageModels::new(),
        )
        .unwrap();
        let keyboard = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        let panel = SettingsPanel::new();
        panel.attach(keyboard.clone()).unwrap();
        let owner = panel.callback_owner();
        let weak_owner = Rc::downgrade(&owner);
        drop(owner);
        let toggle = field(&panel, "Number row")
            .input
            .cast::<ToggleSwitch>()
            .unwrap();
        let holder = Rc::new(RefCell::new(Some(panel.clone())));
        let weak_panel = panel.downgrade();
        let observed = Rc::new(std::cell::Cell::new(false));
        let settings = engine.settings();
        let token = settings.property_changed().add(Rc::new({
            let holder = holder.clone();
            let weak_panel = weak_panel.clone();
            let observed = observed.clone();
            move |name| {
                if name == "NumberRow" {
                    let panel = holder.take().unwrap();
                    panel.release_native_ownership();
                    assert!(panel.callback_owner.borrow().upgrade().is_none());
                    drop(panel);
                    assert!(
                        weak_panel.upgrade().is_some(),
                        "the callback's native owner is still rooted on its stack"
                    );
                    observed.set(true);
                }
            }
        }));
        drop(panel);
        toggle.set_is_checked(Some(true));
        assert!(observed.get());
        assert!(settings.number_row());
        assert!(
            weak_panel.upgrade().is_none(),
            "the in-flight callback releases its local strong owner on return"
        );
        assert!(weak_owner.upgrade().unwrap().retain().is_none());
        settings.property_changed().remove(token);
        let fresh = SettingsPanel::new();
        fresh.attach(keyboard.clone()).unwrap();
        let previous = fresh.callback_owner();
        fresh.release_native_ownership();
        fresh.release_native_ownership();
        fresh.attach(keyboard.clone()).unwrap();
        let next = fresh.callback_owner();
        assert!(!Rc::ptr_eq(&previous, &next));
        assert!(previous.retain().is_none());
        assert_eq!(next.retain(), Some(fresh.clone()));
        fresh.release_native_ownership();
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}

#[test]
fn native_panel_drop_removes_settings_hooks_and_retained_widgets_are_inactive() {
    crate::tests::run(|| {
        let settings = Rc::new(KeyboardSettings::default());
        let engine = KeyboardEngine::new_explicit(
            settings.clone(),
            resources("alpha"),
            LanguageModels::new(),
        )
        .unwrap();
        let keyboard = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        let before = settings.property_changed().handler_count();
        let panel = SettingsPanel::new();
        panel.attach(keyboard.clone()).unwrap();
        panel.attach(keyboard.clone()).unwrap();
        assert_eq!(settings.property_changed().handler_count(), before + 2);
        let toggle = field(&panel, "Number row")
            .input
            .cast::<ToggleSwitch>()
            .unwrap();
        let weak = panel.downgrade();
        drop(panel);
        assert!(
            weak.upgrade().is_none(),
            "settings and widget callbacks must not retain SettingsPanel"
        );
        assert_eq!(settings.property_changed().handler_count(), before);
        assert!(!settings.number_row());
        toggle.set_is_checked(Some(true));
        assert!(
            !settings.number_row(),
            "a retained widget must not dispatch through an expired panel"
        );
        use ferroboard_core::editor::{ITextInputTarget, TextBufferTarget};
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        engine.input_text(&"still usable".into()).unwrap();
        assert_eq!(target.text(), "still usable".into());
    });
}

#[test]
fn expired_panel_widgets_and_language_bindings_cannot_change_settings_or_learning() {
    crate::tests::run(|| {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_key_height(67.0);
        settings.set_theme_mode(ThemeMode::Dark);
        let models = LanguageModels::new();
        let dictionary = models.get_user_dictionary(&"en".into());
        dictionary.learn(&"keepword".into(), None);
        let learned = dictionary.get_count(&"keepword".into());
        assert!(learned > 0);
        let engine =
            KeyboardEngine::new_explicit(settings.clone(), resources("alpha"), models).unwrap();
        let keyboard = OnScreenKeyboard::new(Some(engine)).unwrap();
        let panel = SettingsPanel::new();
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(1000.0);
        window.set_content(into_markup_value(panel.clone()));
        window.show();
        panel.attach(keyboard).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let controls = [
            "toggle", "slider", "combo", "theme", "language", "filter", "reset", "forget",
        ]
        .map(|kind| (kind, retained_control(&panel, kind)));
        let bindings = panel.presentation().unwrap().languages;
        let filter = bindings.filter().unwrap();
        let context = panel.context.clone();
        let weak = panel.downgrade();
        window.set_content(None);
        window.close();
        Dispatcher::ui_thread().run_jobs(None);
        drop(panel);
        assert!(weak.upgrade().is_none());
        assert!(context.try_settings().is_err());
        let before = settings.to_json().unwrap();
        for (kind, control) in controls {
            match kind {
                "toggle" => control
                    .cast::<ToggleSwitch>()
                    .unwrap()
                    .set_is_checked(Some(true)),
                "slider" => control.cast::<Slider>().unwrap().set_range_value(80.0),
                "combo" | "theme" => {
                    let combo = control.cast::<ComboBox>().unwrap();
                    combo.set_selected_index(0);
                    combo.set_selected_index(1);
                }
                "language" => control
                    .cast::<CheckBox>()
                    .unwrap()
                    .set_is_checked(Some(false)),
                "filter" => control
                    .cast::<TextBox>()
                    .unwrap()
                    .set_text(Some("no matches")),
                "reset" | "forget" => {
                    control.raise_event(&RoutedEventArgs::with_event(Button::click_event()))
                }
                _ => unreachable!(),
            }
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(settings.to_json().unwrap(), before, "expired {kind}");
            assert_eq!(
                dictionary.get_count(&"keepword".into()),
                learned,
                "expired {kind}"
            );
        }
        bindings.set_filter(Some("also inactive".into()));
        assert_eq!(bindings.filter().unwrap(), filter);
    });
}

#[test]
fn earlier_settings_observer_can_release_panel_before_its_snapshot_callback() {
    crate::tests::run(|| {
        let settings = Rc::new(KeyboardSettings::default());
        let engine = KeyboardEngine::new_explicit(
            settings.clone(),
            resources("alpha"),
            LanguageModels::new(),
        )
        .unwrap();
        let keyboard = OnScreenKeyboard::new(Some(engine)).unwrap();
        let before = settings.property_changed().handler_count();
        let slot = Rc::new(RefCell::new(None::<Ref<SettingsPanel>>));
        let weak_slot = Rc::downgrade(&slot);
        let token = settings.property_changed().add(Rc::new(move |name| {
            if name == "NumberRow"
                && let Some(slot) = weak_slot.upgrade()
            {
                slot.borrow_mut().take();
            }
        }));
        let panel = SettingsPanel::new();
        panel.attach(keyboard).unwrap();
        let weak = panel.downgrade();
        *slot.borrow_mut() = Some(panel);
        settings.set_number_row(true);
        assert!(weak.upgrade().is_none());
        // The panel owned the keyboard too; only our earlier observer remains.
        assert_eq!(settings.property_changed().handler_count(), 1);
        assert!(before > 0);
        settings.property_changed().remove(token);
        assert_eq!(settings.property_changed().handler_count(), 0);
    });
}
