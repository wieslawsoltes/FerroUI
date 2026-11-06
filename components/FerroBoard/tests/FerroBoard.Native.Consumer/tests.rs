use crate::consumer::Consumer;
use ferroboard_controls::{ScreenKeyboardHost, SmartbarAction, input::ScreenKeyboardInput};
use ferroboard_core::{
    editor::{
        EnterAction, ITextInputTarget, InputAttributes, TargetReplaceResult, TextBufferTarget,
    },
    keys::{KeyCode, KeyIcon, KeyType},
    layouts::{
        LayoutType, Subtype, SubtypeLayoutMap, SubtypeLayoutMapValues, SubtypeValues, TextKeyData,
    },
    settings::KeyboardSettings,
    text::{
        ISuggestionProvider, Suggestion, SuggestionKind, SuggestionList, SuggestionRequest,
        SuggestionResult, SuggestionValues, Utf16Text,
    },
};
use ferroui_base::{BoxedValue, Ref, metadata::from_markup_value, threading::Dispatcher};
use ferroui_controls::{AppBuilder, Application, CheckBox, StackPanel, TextBlock, TextBox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use ferroui_system::{events::Event, threading::CancellationToken};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, OnceLock},
};

fn builder() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
#[derive(Clone, Copy, Debug)]
enum LoadMode {
    Compiled,
    Runtime,
}
fn run(action: impl Fn(LoadMode) + Send + Sync + 'static) {
    static SESSION: OnceLock<Arc<HeadlessUnitTestSession>> = OnceLock::new();
    let action = Arc::new(action);
    for mode in [LoadMode::Compiled, LoadMode::Runtime] {
        let action = action.clone();
        SESSION
            .get_or_init(|| HeadlessUnitTestSession::start_new(builder))
            .dispatch(
                move || {
                    ferroboard_controls::register_types();
                    ferroui_markup_xaml::register_types();
                    crate::consumer::register_types();
                    crate::compiled_markup::register();
                    eprintln!("downstream load mode: {mode:?}");
                    action(mode);
                },
                CancellationToken::none(),
            )
            .wait()
            .unwrap_or_else(|error| panic!("{mode:?} consumer scenario failed: {error:?}"));
    }
}
fn load(mode: LoadMode, name: &str) -> BoxedValue {
    match mode {
        LoadMode::Compiled => crate::compiled_xaml::try_load(
            None,
            &format!("{}{name}.xaml", crate::compiled_xaml::ROOT_URI),
        )
        .unwrap()
        .expect("compiled consumer document"),
        LoadMode::Runtime => {
            let source = match name {
                "Settings" => include_str!("Settings.xaml"),
                "Geometry" => include_str!("Geometry.xaml"),
                "TimeWindows" => include_str!("TimeWindows.xaml"),
                "Suggestion" => include_str!("Suggestion.xaml"),
                "Request" => include_str!("Request.xaml"),
                "LocaleModels" => include_str!("LocaleModels.xaml"),
                "Utf16Buffer" => include_str!("Utf16Buffer.xaml"),
                "Callbacks" => include_str!("Callbacks.xaml"),
                "Subtypes" => include_str!("Subtypes.xaml"),
                "CodePoints" => include_str!("CodePoints.xaml"),
                "PopupLookup" => include_str!("PopupLookup.xaml"),
                "DictionaryModels" => include_str!("DictionaryModels.xaml"),
                "KeyList" => include_str!("KeyList.xaml"),
                "SubtypeList" => include_str!("SubtypeList.xaml"),
                "PerformedActions" => include_str!("PerformedActions.xaml"),
                "TextKeyList" => include_str!("TextKeyList.xaml"),
                "PredictionList" => include_str!("PredictionList.xaml"),
                "SuggestionList" => include_str!("SuggestionList.xaml"),
                "PopupSnapshots" => include_str!("PopupSnapshots.xaml"),
                "ComputedRows" => include_str!("ComputedRows.xaml"),
                "PopupKeys" => include_str!("PopupKeys.xaml"),
                "EmptyArrangement" => include_str!("EmptyArrangement.xaml"),
                "NativeModels" => include_str!("NativeModels.xaml"),
                "ComputedSnapshot" => include_str!("ComputedSnapshot.xaml"),
                "Standalone" => include_str!("Standalone.xaml"),
                "Hosted" => include_str!("Hosted.xaml"),
                "PrivacyHosted" => include_str!("PrivacyHosted.xaml"),
                "Actions" => include_str!("Actions.xaml"),
                "TouchConfiguration" => include_str!("TouchConfiguration.xaml"),
                "ActionDetails" => include_str!("ActionDetails.xaml"),
                _ => panic!("unknown downstream document: {name}"),
            };
            ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
                None,
            )
            .unwrap()
        }
    }
}
fn dispose(consumer: &Consumer) {
    let keyboard = consumer.keyboard().unwrap();
    keyboard.detach().unwrap();
    keyboard.engine().dispose().unwrap();
    consumer.release();
}

#[test]
fn independent_compiled_settings_bindings_are_live_and_two_way() {
    run(|mode| {
        let value = load(mode, "Settings");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let settings = from_markup_value::<Rc<KeyboardSettings>>(&panel.data_context()).unwrap();
        let label = panel.children().get(0).cast::<TextBlock>().unwrap();
        let toggle = panel.children().get(1).cast::<CheckBox>().unwrap();
        assert_eq!(label.text().as_deref(), Some("54.25"));
        settings.set_key_height(67.75);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(label.text().as_deref(), Some("67.75"));
        toggle.set_is_checked(Some(true));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(settings.number_row());
        panel.set_data_context(None);
    });
}

#[test]
fn independent_touch_configuration_bindings_use_owned_processor_and_action_vector() {
    use ferroboard_controls::{OnScreenKeyboard, themes::ThemeList};
    use ferroboard_core::settings::SwipeAction;
    use ferroboard_core_ferro::KeyboardTouchProcessorRef;
    use ferroui_controls::Slider;
    run(|mode| {
        let value = load(mode, "TouchConfiguration");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let keyboard = from_markup_value::<Ref<OnScreenKeyboard>>(&panel.data_context()).unwrap();
        let touch = keyboard.touch_processor();
        let engine = keyboard.engine();
        let weak_touch = Rc::downgrade(&touch);
        let weak_engine = Rc::downgrade(&engine);
        let sliders: Vec<_> = (0..4)
            .map(|i| panel.children().get(i).cast::<Slider>().unwrap())
            .collect();
        assert_eq!(
            sliders.iter().map(|s| s.value()).collect::<Vec<_>>(),
            [14.0, 16.0, -48.0, 6.0]
        );
        let actions = keyboard.smartbar_actions().unwrap();
        let label = panel.children().get(4).cast::<TextBlock>().unwrap();
        let count = actions.lock().unwrap().len();
        assert_eq!(label.text().as_deref(), Some(count.to_string().as_str()));
        let actions_context = panel.children().get(5).cast::<TextBlock>().unwrap();
        let bound = from_markup_value::<Option<ThemeList<Option<Rc<SmartbarAction>>>>>(
            &actions_context.data_context(),
        )
        .unwrap()
        .unwrap();
        assert!(
            bound == actions,
            "markup must return the original shared vector"
        );
        let touch_context = panel.children().get(6).cast::<TextBlock>().unwrap();
        let bound =
            from_markup_value::<KeyboardTouchProcessorRef>(&touch_context.data_context()).unwrap();
        assert!(
            Rc::ptr_eq(&bound.original(), &touch),
            "markup must return the owned native processor"
        );
        for (slider, value) in sliders.iter().zip([7.0, 10.0, -60.0, 3.0]) {
            slider.set_current_value(
                ferroui_controls::primitives::RangeBase::value_property(),
                value,
            );
        }
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(touch.cursor_step_distance(), 7.0);
        assert_eq!(touch.delete_step_distance(), 10.0);
        assert_eq!(touch.popup_min_y(), -60.0);
        assert_eq!(touch.popup_max_columns(), 3);
        let target = Rc::new(TextBufferTarget::new("abcdefghijklmnop".into(), None));
        keyboard.attach(target.clone()).unwrap();
        engine.update_layout(720.0, 240.0, None).unwrap();
        let space = engine
            .keyboard()
            .unwrap()
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == KeyCode::Space)
            .unwrap();
        let b = space.touch_bounds();
        touch.pointer_down(1, b.center_x(), b.center_y()).unwrap();
        touch
            .pointer_move(1, b.center_x() - 40.0, b.center_y())
            .unwrap();
        touch
            .pointer_up(1, b.center_x() - 40.0, b.center_y())
            .unwrap();
        assert_eq!(
            target.selection_start(),
            11,
            "40 DIPs / configured7 = five cursor steps"
        );
        assert_eq!(target.text(), "abcdefghijklmnop".into());
        target.select(16, 0);
        engine
            .settings()
            .set_delete_key_swipe_left(SwipeAction::DeleteCharactersPrecisely);
        let delete = engine
            .keyboard()
            .unwrap()
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == KeyCode::Delete)
            .unwrap();
        let b = delete.touch_bounds();
        touch.pointer_down(2, b.center_x(), b.center_y()).unwrap();
        touch
            .pointer_move(2, b.center_x() - 40.0, b.center_y())
            .unwrap();
        touch
            .pointer_up(2, b.center_x() - 40.0, b.center_y())
            .unwrap();
        assert_eq!(
            target.text(),
            "abcdefghijkl".into(),
            "40 DIPs / configured10 = four deleted characters"
        );
        assert_eq!(target.selection_start(), 12);
        assert_eq!(touch.pressed_keys().into_iter().count(), 0);
        keyboard.detach().unwrap();
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
        panel.set_data_context(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        drop(bound);
        drop(touch_context);
        drop(actions_context);
        drop(label);
        drop(sliders);
        drop(touch);
        drop(engine);
        drop(keyboard);
        drop(value);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(
            weak_touch.upgrade().is_none(),
            "disposed bindings must release native touch handles"
        );
        assert!(
            weak_engine.upgrade().is_none(),
            "touch metadata must not retain the disposed engine"
        );
    });
}

#[test]
fn independent_action_details_bind_native_handles_and_release_replaced_context() {
    use ferroboard_controls::SmartbarActionRef;
    run(|mode| {
        let value = load(mode, "ActionDetails");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let id = panel.children().get(0).cast::<TextBlock>().unwrap();
        let name = panel.children().get(1).cast::<TextBlock>().unwrap();
        let action = SmartbarAction::new(
            Some("native".into()),
            KeyIcon::None,
            Some("Native 😀 action".into()),
            None,
            None,
        );
        let weak = Rc::downgrade(&action);
        let handle = SmartbarActionRef::from_original(action.clone());
        panel.set_data_context(Some(Rc::new(handle.clone()) as BoxedValue));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(id.text().as_deref(), Some("native"));
        assert_eq!(name.text().as_deref(), Some("Native 😀 action"));
        let replacement = SmartbarActionRef::from_original(SmartbarAction::new(
            None,
            KeyIcon::None,
            None,
            None,
            None,
        ));
        panel.set_data_context(Some(Rc::new(replacement) as BoxedValue));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(id.text().is_none());
        assert!(name.text().is_none());
        drop(handle);
        drop(action);
        assert!(
            weak.upgrade().is_none(),
            "binding context replacement must release native action handles"
        );
        panel.set_data_context(None);
    });
}

#[test]
fn independent_compiled_controls_themes_input_overrides_and_enter_handler() {
    run(|mode| {
        let value = load(mode, "Standalone");
        let consumer = value.downcast_ref::<Ref<Consumer>>().unwrap();
        let keyboard = consumer.keyboard().unwrap();
        let engine = keyboard.engine();
        assert!(!keyboard.auto_attach());
        assert_eq!(keyboard.settings().key_height(), 61.25);
        assert!(Rc::ptr_eq(&keyboard.settings(), &engine.settings()));
        assert_eq!(
            keyboard.theme().unwrap().id(),
            Some("native-consumer".into())
        );
        assert_eq!(keyboard.theme().unwrap().key_corner_radius(), 9.5);
        let hosted = load(mode, "Hosted");
        let host = hosted.downcast_ref::<Ref<ScreenKeyboardHost>>().unwrap();
        let editor = host.content().unwrap().cast::<TextBox>().unwrap();
        assert!(ScreenKeyboardInput::get_is_private(&editor));
        assert!(!ScreenKeyboardInput::get_suggestions_enabled(&editor));
        editor.set_selection_start(4);
        editor.set_selection_end(4);
        assert!(keyboard.attach_to(Some(editor.clone().upcast())).unwrap());
        assert!(engine.attributes().is_private());
        assert_eq!(
            engine.attributes().effective_enter_action(),
            EnterAction::Send
        );
        engine.input_text(&"😀".into()).unwrap();
        assert_eq!(editor.text().as_deref(), Some("mail😀"));
        assert_eq!(engine.target().unwrap().selection_start(), 6);
        engine
            .input_key(TextKeyData::function(KeyCode::Enter, KeyType::EnterEditing, None).as_ref())
            .unwrap();
        assert_eq!(consumer.enter_count.get(), 1);
        assert_eq!(consumer.last_action.get(), Some(EnterAction::Send));
        assert_eq!(editor.text().as_deref(), Some("mail😀"));
        // Compiled/runtime event bridges pass a cloned argument value. Handled must
        // reach the original invocation so Done does not fall through to hiding.
        let hides = Rc::new(std::cell::Cell::new(0));
        let count = hides.clone();
        let hide = keyboard
            .hide_requested()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        keyboard.detach().unwrap();
        ScreenKeyboardInput::set_enter_action(&editor, Some(Rc::new(EnterAction::Done)));
        keyboard.attach_to(Some(editor.clone().upcast())).unwrap();
        engine
            .input_key(TextKeyData::function(KeyCode::Enter, KeyType::EnterEditing, None).as_ref())
            .unwrap();
        assert_eq!(consumer.enter_count.get(), 2);
        assert_eq!(consumer.last_action.get(), Some(EnterAction::Done));
        assert_eq!(
            hides.get(),
            0,
            "Handled must survive the markup argument bridge"
        );
        assert_eq!(editor.text().as_deref(), Some("mail😀"));
        keyboard.hide_requested().remove(hide);
        dispose(consumer);
        host.release_native_subscriptions().unwrap();
        host.keyboard().engine().dispose().unwrap();
    });
}

struct RecordingTarget {
    buffer: TextBufferTarget,
    replacements: RefCell<Vec<(i32, i32, Utf16Text)>>,
    changed: Event<dyn ITextInputTarget>,
}
impl RecordingTarget {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            buffer: TextBufferTarget::new("ab".into(), None),
            replacements: RefCell::new(Vec::new()),
            changed: Event::new(),
        })
    }
    fn external_edit(&self, text: Utf16Text, caret: i32) {
        self.buffer.set_text(Some(text));
        self.buffer.select(caret, 0);
        self.changed.raise(self);
    }
}
impl ITextInputTarget for RecordingTarget {
    fn attributes(&self) -> Arc<InputAttributes> {
        self.buffer.attributes()
    }
    fn text(&self) -> Utf16Text {
        self.buffer.text()
    }
    fn selection_start(&self) -> i32 {
        self.buffer.selection_start()
    }
    fn selection_length(&self) -> i32 {
        self.buffer.selection_length()
    }
    fn replace(&self, start: i32, length: i32, text: Option<&Utf16Text>) -> TargetReplaceResult {
        self.buffer.replace(start, length, text)?;
        self.replacements
            .borrow_mut()
            .push((start, length, text.unwrap().clone()));
        self.changed.raise(self);
        Ok(())
    }
    fn select(&self, start: i32, length: i32) {
        self.buffer.select(start, length);
        self.changed.raise(self);
    }
    fn perform_enter_action(&self, action: EnterAction) -> bool {
        self.buffer.perform_enter_action(action)
    }
    fn changed(&self) -> &Event<dyn ITextInputTarget> {
        &self.changed
    }
}
#[derive(Default, PartialEq)]
struct ConsumerProvider {
    requests: RefCell<Vec<Utf16Text>>,
    learned: RefCell<Vec<Utf16Text>>,
    forgotten: RefCell<Vec<Utf16Text>>,
}
impl ISuggestionProvider for ConsumerProvider {
    fn suggest(
        &self,
        request: Option<&SuggestionRequest>,
    ) -> SuggestionResult<Option<SuggestionList>> {
        self.requests
            .borrow_mut()
            .push(request.unwrap().word().unwrap());
        Ok(Some(
            std::iter::once(Rc::new(Suggestion::from_values(SuggestionValues::new(
                "native".into(),
                SuggestionKind::Completion,
            ))))
            .collect(),
        ))
    }
    fn learn(&self, word: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        if let Some(word) = word {
            self.learned.borrow_mut().push(word.clone());
        }
        Ok(())
    }
    fn forget(&self, word: Option<&Utf16Text>) -> SuggestionResult<()> {
        if let Some(word) = word {
            self.forgotten.borrow_mut().push(word.clone());
        }
        Ok(())
    }
}
#[test]
fn independent_rust_targets_providers_quick_actions_and_external_utf16_edits() {
    run(|mode| {
        let value = load(mode, "Standalone");
        let consumer = value.downcast_ref::<Ref<Consumer>>().unwrap();
        let keyboard = consumer.keyboard().unwrap();
        let engine = keyboard.engine();
        let target = RecordingTarget::new();
        let provider = Rc::new(ConsumerProvider::default());
        let factory_provider = provider.clone();
        engine.set_suggestion_provider_factory(Some(Rc::new(move |subtype| {
            assert!(subtype.is_some());
            Ok(Some(factory_provider.clone()))
        })));
        engine.settings().set_show_suggestions(true);
        engine.settings().set_auto_correct(false);
        engine.settings().set_emoji_suggestions(false);
        keyboard.attach(target.clone()).unwrap();
        engine.type_text(&"c".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("abc"));
        assert_eq!(
            provider.requests.borrow().last().unwrap(),
            &Utf16Text::from("abc")
        );
        let candidate = engine.suggestions().snapshot().unwrap()[0].clone().unwrap();
        engine.commit_suggestion(&candidate).unwrap();
        assert_eq!(target.text(), Utf16Text::from("native "));
        assert!(provider.learned.borrow().is_empty());
        let action = SmartbarAction::new(
            Some("punctuate".into()),
            KeyIcon::None,
            Some("Punctuate".into()),
            Some(Rc::new(|engine| engine.input_text(&"!".into()))),
            None,
        );
        let actions = keyboard.smartbar_actions().unwrap();
        let count = actions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        actions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Some(action.clone()));
        assert_eq!(
            keyboard
                .smartbar_actions()
                .unwrap()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .len(),
            count + 1
        );
        action.execute.as_ref().unwrap()(&engine).unwrap();
        assert_eq!(target.text(), Utf16Text::from("native !"));
        target.external_edit("A😀B".into(), 1);
        engine.input_text(&"é".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("Aé😀B"));
        assert_eq!(target.selection_start(), 2);
        assert!(
            target
                .replacements
                .borrow()
                .iter()
                .any(|(start, len, text)| *start == 0
                    && *len == 3
                    && *text == Utf16Text::from("native"))
        );
        dispose(consumer);
        target.external_edit("detached".into(), 8);
        assert!(engine.target().is_none());
    });
}

#[test]
fn independent_custom_layout_registry_and_engine_geometry() {
    run(|mode| {
        let value = load(mode, "Standalone");
        let consumer = value.downcast_ref::<Ref<Consumer>>().unwrap();
        let engine = consumer.keyboard().unwrap().engine();
        let resources = engine.resources();
        let custom = resources.add_layout_json(r#"{"type":"characters","id":"consumer-layout","label":"Consumer","direction":"ltr","arrangement":[["a","b","β"]]}"#, Some("native-consumer".into())).unwrap();
        assert!(Rc::ptr_eq(
            &custom,
            &resources
                .get_layout(LayoutType::Characters, &custom.name())
                .unwrap()
                .unwrap()
        ));
        let subtype = Arc::new(Subtype::from_values(SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                SubtypeLayoutMapValues {
                    characters: custom.name(),
                    ..SubtypeLayoutMap::default().values()
                },
            ))),
            ..Subtype::new("en-US".into()).values()
        }));
        engine.set_subtypes([subtype]).unwrap();
        let computed = engine.keyboard().unwrap();
        computed.layout(513.5, 210.25, None).unwrap();
        let labels: Vec<_> = computed
            .character_keys()
            .unwrap()
            .iter()
            .map(|key| key.label())
            .collect();
        for expected in ["a", "b", "β"] {
            assert!(labels.contains(&Some(expected.into())));
        }
        assert!(
            computed.character_keys().unwrap().iter().all(|key| key
                .touch_bounds()
                .width
                .is_finite()
                && key.touch_bounds().width > 0.0)
        );
        dispose(consumer);
    });
}

#[test]
fn independent_rendered_quick_action_pointer_and_automation_preserve_native_focus() {
    run(|mode| {
        use ferroboard_controls::{
            OnScreenKeyboard, SmartbarActionRef, SmartbarActionsMetadata, primitives::TouchButton,
        };
        use ferroui_base::{
            Point, Visual,
            input::{MouseButton, RawInputModifiers},
            metadata::{MarkupTyped, into_markup_value},
        };
        use ferroui_controls::{
            Control, Window,
            automation::{
                AutomationProperties,
                peers::{AutomationControlType, ControlAutomationPeer},
                provider::IInvokeProvider,
            },
        };
        use ferroui_headless::HeadlessWindowExtensions;
        use std::cell::Cell;

        fn find_action(root: Ref<Visual>) -> Ref<TouchButton> {
            let mut remaining = vec![root];
            while let Some(visual) = remaining.pop() {
                if let Some(button) = visual.cast::<TouchButton>()
                    && AutomationProperties::get_name(&button).as_deref() == Some("Append marker")
                {
                    return button;
                }
                remaining.extend(visual.visual_children().to_vec());
            }
            panic!("custom action must be present in the actual keyboard visual tree");
        }
        let value = load(mode, "Actions");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let editor = panel.children().get(0).cast::<TextBox>().unwrap();
        let keyboard = panel.children().get(1).cast::<OnScreenKeyboard>().unwrap();
        let engine = keyboard.engine();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let actions = keyboard.smartbar_actions().unwrap();
        actions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        let action = SmartbarAction::new(
            Some("append-marker".into()),
            KeyIcon::None,
            Some("Append marker".into()),
            Some(Rc::new(move |engine| {
                observed.set(observed.get() + 1);
                engine.input_text(&"!".into())
            })),
            None,
        );
        let add = SmartbarActionsMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "Add")
            .unwrap();
        (add.invoke)(&[
            into_markup_value(actions.clone()),
            into_markup_value(Some(SmartbarActionRef::from_original(action))),
        ])
        .expect("native action collection metadata must populate the actual smartbar vector");
        let window = Window::new();
        window.set_width(720.0);
        window.set_height(400.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(editor.focus());
        editor.set_selection_start(3);
        editor.set_selection_end(3);
        assert!(keyboard.attach_to(Some(editor.clone().upcast())).unwrap());
        keyboard.apply_theme().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);

        let button = find_action(keyboard.clone().upcast());
        assert!(button.bounds().width > 0.0 && button.bounds().height > 0.0);
        assert!(!button.focusable());
        let point = button
            .translate_point(
                Point::new(button.bounds().width / 2.0, button.bounds().height / 2.0),
                &window,
            )
            .unwrap();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        assert_eq!(calls.get(), 0, "an action fires on release");
        assert_eq!(editor.text().as_deref(), Some("α😀"));
        window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(calls.get(), 1);
        assert_eq!(editor.text().as_deref(), Some("α😀!"));
        assert_eq!(engine.target().unwrap().selection_start(), 4);
        assert!(editor.is_focused());
        assert_eq!(keyboard.target_element(), Some(editor.clone().upcast()));
        drop(button);

        let button = find_action(keyboard.clone().upcast());
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Button
        );
        assert_eq!(peer.get_name(), "Append marker");
        assert!(!peer.is_keyboard_focusable());
        let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        invoke.invoke().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(calls.get(), 2);
        assert_eq!(editor.text().as_deref(), Some("α😀!!"));
        assert!(editor.is_focused());
        drop(button);

        let button = find_action(keyboard.clone().upcast());
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        keyboard.set_is_enabled(false);
        assert!(invoke.invoke().is_err());
        assert_eq!(calls.get(), 2);
        assert_eq!(editor.text().as_deref(), Some("α😀!!"));
        keyboard.set_is_enabled(true);
        invoke.invoke().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(calls.get(), 3);
        assert_eq!(editor.text().as_deref(), Some("α😀!!!"));
        assert_eq!(engine.target().unwrap().selection_start(), 6);
        assert!(editor.is_focused());
        window.mouse_move(Point::new(-1.0, -1.0), RawInputModifiers::NONE);
        window.close();
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}

#[test]
fn independent_resource_dictionaries_preserve_settings_copies_theme_identity_and_edits() {
    run(|mode| {
        use ferroboard_controls::OnScreenKeyboard;
        use ferroui_base::{media::Color, utilities::UriRef};
        let value = match mode {
            LoadMode::Compiled => crate::compiled_xaml::try_load(
                None,
                &format!(
                    "{}Views/ResourceKeyboard.xaml",
                    crate::compiled_xaml::ROOT_URI
                ),
            )
            .unwrap()
            .expect("compiled external-resource view"),
            LoadMode::Runtime => {
                let uri = UriRef::parse(&format!(
                    "{}Runtime/Views/ResourceKeyboard.fxaml",
                    crate::compiled_xaml::ROOT_URI
                ))
                .unwrap();
                for path in [
                    "Views/ResourceKeyboard",
                    "Resources/Keyboard",
                    "Resources/Defaults",
                ] {
                    let resource_uri =
                        format!("{}Runtime/{path}.fxaml", crate::compiled_xaml::ROOT_URI);
                    assert!(
                        crate::compiled_xaml::try_load(None, &resource_uri)
                            .unwrap()
                            .is_none(),
                        "runtime-only graph must not have compiled construction: {path}"
                    );
                }
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
                ferroui_markup_xaml::FerroXamlLoader::load(&uri, None).unwrap()
            }
        };
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let first = panel.children().get(0).cast::<OnScreenKeyboard>().unwrap();
        let second = panel.children().get(1).cast::<OnScreenKeyboard>().unwrap();
        let field = panel.children().get(2).cast::<TextBox>().unwrap();
        let settings = first.settings();
        let theme = first.theme().unwrap();
        let second_settings = second.settings();
        let resource = panel
            .try_get_resource(
                &ferroui_base::controls::ResourceKey::from("SharedSettings"),
                None,
            )
            .unwrap();
        let resource = from_markup_value::<Rc<KeyboardSettings>>(&resource).unwrap();
        let again = panel
            .try_get_resource(
                &ferroui_base::controls::ResourceKey::from("SharedSettings"),
                None,
            )
            .unwrap();
        assert!(Rc::ptr_eq(
            &resource,
            &from_markup_value::<Rc<KeyboardSettings>>(&again).unwrap()
        ));
        // Source OnSettingsReplaced copies into each engine and restores its own object.
        assert!(!Rc::ptr_eq(&settings, &second_settings));
        assert!(!Rc::ptr_eq(&settings, &resource));
        assert!(Rc::ptr_eq(&settings, &first.engine().settings()));
        assert!(Rc::ptr_eq(&second_settings, &second.engine().settings()));
        assert!(Arc::ptr_eq(&theme, &second.theme().unwrap()));
        assert!(Arc::ptr_eq(&theme, &first.effective_theme()));
        assert_eq!(
            theme.id(),
            Some("external-consumer".into()),
            "near dictionary must shadow merged base theme"
        );
        assert_eq!(theme.background().to_uint32(), 0xff102030);
        let rules = theme.key_rules().unwrap();
        assert!(Arc::ptr_eq(
            &rules,
            &second.theme().unwrap().key_rules().unwrap()
        ));
        let rule = rules.lock().unwrap()[0].clone().unwrap();
        assert_eq!(rule.font_size(), Some(31.0));
        assert_eq!(*rule.codes().unwrap().lock().unwrap(), vec![97, 98]);
        rule.codes()
            .unwrap()
            .lock()
            .unwrap()
            .retain(|code| *code == 98);
        assert!(
            !rule
                .matches(
                    97,
                    KeyType::Character,
                    ferroboard_core::keys::KeyboardMode::Characters,
                    ferroboard_core::keys::ShiftState::Unshifted
                )
                .unwrap()
        );
        assert!(
            rule.matches(
                98,
                KeyType::Character,
                ferroboard_core::keys::KeyboardMode::Characters,
                ferroboard_core::keys::ShiftState::Unshifted
            )
            .unwrap()
        );

        assert_eq!(settings.key_height(), 57.5);
        settings.set_key_height(73.25);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(settings.key_height(), 73.25);
        assert_eq!(second.settings().key_height(), 57.5);
        resource.set_key_height(88.0);
        assert_eq!(settings.key_height(), 73.25);
        assert_eq!(second.settings().key_height(), 57.5);
        second.set_settings(Some(resource.clone()));
        assert!(Rc::ptr_eq(&second_settings, &second.settings()));
        assert_eq!(second.settings().key_height(), 88.0);
        assert!(first.attach_to(Some(field.clone().upcast())).unwrap());
        assert!(first.engine().attributes().is_private());
        field.set_caret_index(4);
        first.engine().input_text(&"😀".into()).unwrap();
        assert_eq!(field.text().as_deref(), Some("mail😀"));
        assert_eq!(first.engine().target().unwrap().selection_start(), 6);
        theme.set_background(Color::from_uint32(0xff234567));
        first.apply_theme().unwrap();
        second.apply_theme().unwrap();
        assert_eq!(
            second.effective_theme().background().to_uint32(),
            0xff234567
        );
        for keyboard in [first, second] {
            keyboard.detach().unwrap();
            keyboard.engine().dispose().unwrap();
            keyboard.release_native_subscriptions().unwrap();
        }
        panel.children().clear();
    });
}

#[test]
fn independent_runtime_resource_errors_remain_visible_and_failed_include_can_retry() {
    run(|mode| {
        if matches!(mode, LoadMode::Compiled) {
            return;
        }
        use ferroui_base::{controls::ResourceKey, utilities::UriRef};
        use ferroui_markup_xaml::styling::ResourceInclude;
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
        let base = UriRef::parse_absolute(&format!(
            "{}Runtime/Views/ResourceKeyboard.fxaml",
            crate::compiled_xaml::ROOT_URI
        ))
        .unwrap();
        let include = ResourceInclude::new(Some(base.clone()));
        let error = include
            .try_loaded()
            .err()
            .expect("unset include Source must fail");
        assert!(error.to_string().contains("Source"));
        include.set_source(Some(UriRef::parse("../Resources/Absent.fxaml").unwrap()));
        let error = include
            .try_loaded()
            .err()
            .expect("missing embedded resource must fail");
        eprintln!("missing resource diagnostic: {error}");
        assert!(error.to_string().contains("Absent.fxaml"));
        include.set_source(Some(UriRef::parse("../Resources/Keyboard.fxaml").unwrap()));
        let loaded = include
            .try_loaded()
            .expect("failed include must release its loading guard and permit retry");
        assert!(
            loaded
                .try_get_resource(&ResourceKey::from("SharedSettings"), None)
                .is_some()
        );
        assert!(Rc::ptr_eq(&loaded, &include.try_loaded().unwrap()));
        let invalid = UriRef::parse(&format!(
            "{}Runtime/Resources/Invalid.fxaml",
            crate::compiled_xaml::ROOT_URI
        ))
        .unwrap();
        assert!(
            crate::compiled_xaml::try_load(None, invalid.to_string().as_str())
                .unwrap()
                .is_none()
        );
        let error = ferroui_markup_xaml::FerroXamlLoader::load(&invalid, None)
            .expect_err("invalid native setting must not become a successful default keyboard");
        eprintln!("invalid settings diagnostic: {error}");
        assert!(error.to_string().contains("not-a-number"));
        assert!(error.to_string().contains("Line 4"));
    });
}

#[test]
fn independent_password_controls_construct_with_protected_peers_and_native_targets() {
    use ferroboard_controls::input::ProtectedPasswordTextBox;
    use ferroboard_core::editor::InputKind;
    use ferroui_controls::automation::{peers::ControlAutomationPeer, provider::IValueProvider};
    run(|mode| {
        let value = load(mode, "PrivacyHosted");
        let host = value.downcast_ref::<Ref<ScreenKeyboardHost>>().unwrap();
        let fields = host.content().unwrap().cast::<StackPanel>().unwrap();
        for (index, kind, limit) in [
            (1, InputKind::Password, 4),
            (3, InputKind::NumericPassword, 3),
        ] {
            let field = fields
                .children()
                .get(index)
                .cast::<ProtectedPasswordTextBox>()
                .unwrap();
            assert_eq!(field.password_char(), '●');
            assert_eq!(field.max_length(), limit);
            assert!(
                host.keyboard()
                    .attach_to(Some(field.clone().upcast()))
                    .unwrap()
            );
            let engine = host.keyboard().engine();
            assert_eq!(engine.attributes().kind(), kind);
            assert!(engine.is_incognito());
            assert!(!engine.attributes().allow_suggestions());
            engine.input_text(&"12345".into()).unwrap();
            assert_eq!(engine.target().unwrap().text().len(), limit as usize);
            let before = engine.target().unwrap().text();
            field.set_is_read_only(true);
            engine
                .input_key(
                    TextKeyData::function(KeyCode::Delete, KeyType::EnterEditing, None).as_ref(),
                )
                .unwrap();
            assert_eq!(engine.target().unwrap().text(), before);
            field.set_is_read_only(false);
            field.set_is_enabled(false);
            engine
                .input_key(
                    TextKeyData::function(KeyCode::Delete, KeyType::EnterEditing, None).as_ref(),
                )
                .unwrap();
            assert_eq!(engine.target().unwrap().text(), before);
            field.set_is_enabled(true);
            engine
                .input_key(
                    TextKeyData::function(KeyCode::Delete, KeyType::EnterEditing, None).as_ref(),
                )
                .unwrap();
            assert_eq!(engine.target().unwrap().text().len(), limit as usize - 1);
            let peer = ControlAutomationPeer::create_peer_for_element(&field);
            assert!(peer.get_provider::<dyn IValueProvider>().is_none());
            assert!(peer.get_children().is_empty());
            field.set_reveal_password(true);
            assert!(peer.get_provider::<dyn IValueProvider>().is_none());
            assert!(peer.get_children().is_empty());
        }
        host.keyboard().detach().unwrap();
        host.release_native_subscriptions().unwrap();
        host.keyboard().engine().dispose().unwrap();
    });
}

#[test]
fn independent_suggestion_errors_propagate_through_standard_rust_error_contracts() {
    fn failing_read() -> Result<(), Box<dyn std::error::Error>> {
        SuggestionList::default().get(0)?;
        Ok(())
    }
    let error = failing_read().unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ferroboard_core::text::SuggestionError>(),
        Some(ferroboard_core::text::SuggestionError::SuggestionIndex(0))
    ));
    assert!(error.source().is_none());
    let custom = ferroboard_core::text::SuggestionError::Custom("native provider detail".into());
    assert_eq!(custom.to_string(), "native provider detail");
}

#[test]
fn native_geometry_metadata_constructs_values_and_preserves_shared_settings() {
    use ferroboard_core::layouts::{KeyRect, KeyboardGeometryOptions};
    use ferroboard_core_ferro::{KeyRectMetadata, KeyboardGeometryOptionsMetadata};
    use ferroui_base::metadata::MarkupTyped;
    use ferroui_controls::ContentControl;
    run(|mode| {
        let value = load(mode, "Geometry");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let geometry: Rc<KeyboardGeometryOptions> =
            from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(geometry.key_spacing_horizontal(), 2.5);
        assert_eq!(geometry.key_spacing_vertical(), 3.75);
        assert_eq!(geometry.split_gap(), 12.25);
        assert_eq!(geometry.units_per_row(), 13.5);
        let content = |index| {
            panel
                .children()
                .get(index)
                .cast::<ContentControl>()
                .unwrap()
                .content()
        };
        assert_eq!(from_markup_value::<f64>(&content(0)), Some(13.5));
        let rect: KeyRect = from_markup_value(&content(1)).unwrap();
        assert_eq!(rect, KeyRect::new(1.25, 2.5, 80.5, 25.75));
        let alias: Rc<KeyboardGeometryOptions> = from_markup_value(&content(2)).unwrap();
        assert!(Rc::ptr_eq(&geometry, &alias));
        alias.set_split_gap(18.5);
        assert_eq!(geometry.split_gap(), 18.5);
        let clone = (*geometry).clone();
        clone.set_split_gap(1.0);
        assert_eq!(geometry.split_gap(), 18.5);
        assert_eq!(clone.split_gap(), 1.0);
        for name in [
            "GetHashCode",
            "Equals",
            "ToString",
            "<Clone>$",
            "op_Equality",
            "op_Inequality",
        ] {
            assert!(
                KeyboardGeometryOptionsMetadata::MARKUP
                    .find_methods(name)
                    .next()
                    .is_none()
            );
            assert!(KeyRectMetadata::MARKUP.find_methods(name).next().is_none());
        }
        assert!(
            KeyRectMetadata::MARKUP
                .find_methods("Contains")
                .next()
                .is_some()
        );
        panel.set_data_context(None);
    });
}

#[test]
fn native_candidate_xaml_uses_properties_bindings_and_explicit_ownership() {
    use ferroui_controls::ContentControl;
    run(|mode| {
        let value = load(mode, "Suggestion");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let candidate: Rc<Suggestion> = from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(candidate.text(), Some("café 😀".into()));
        assert_eq!(candidate.kind(), SuggestionKind::Correction);
        assert_eq!(candidate.confidence(), 0.75);
        assert!(candidate.is_auto_commit());
        assert_eq!(candidate.description(), Some("custom provider".into()));
        assert!(candidate.is_removable());
        let content = |i| {
            panel
                .children()
                .get(i)
                .cast::<ContentControl>()
                .unwrap()
                .content()
        };
        assert_eq!(
            from_markup_value::<Option<String>>(&content(0)),
            Some(Some("café 😀".to_owned()))
        );
        assert_eq!(from_markup_value::<f64>(&content(1)), Some(0.75));
        let alias: Rc<Suggestion> = from_markup_value(&content(2)).unwrap();
        assert!(Rc::ptr_eq(&candidate, &alias));
        let copy = (*candidate).clone();
        alias.set_text(Some("changed".into()));
        assert_eq!(candidate.text(), Some("changed".into()));
        assert_eq!(copy.text(), Some("café 😀".into()));
        // Plain candidate properties do not promise live notifications. A context
        // refresh re-evaluates bindings after mutation.
        panel.set_data_context(None);
        panel.set_data_context(Some(candidate.clone() as BoxedValue));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(
            from_markup_value::<Option<String>>(&content(0)),
            Some(Some("changed".to_owned()))
        );
        let weak = Rc::downgrade(&candidate);
        drop(alias);
        drop(candidate);
        panel.set_data_context(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn native_request_xaml_preserves_names_and_shares_rust_callback_data() {
    use ferroboard_core_ferro::KeyProximityRef;
    use ferroui_controls::ContentControl;
    run(|mode| {
        let value = load(mode, "Request");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let request: SuggestionRequest = from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(request.word(), Some("İz 😀".into()));
        assert_eq!(request.preceding_text(), Some("say ".into()));
        assert_eq!(request.max_count(), 7);
        assert!(!request.allow_auto_correct());
        assert_eq!(request.culture().unwrap().name(), "tr");
        assert!(request.key_proximity().is_none());
        let content = |i| {
            panel
                .children()
                .get(i)
                .cast::<ContentControl>()
                .unwrap()
                .content()
        };
        assert_eq!(
            from_markup_value::<Option<String>>(&content(0)),
            Some(Some("İz 😀".to_owned()))
        );
        assert_eq!(
            from_markup_value::<Option<String>>(&content(1)),
            Some(Some("tr".to_owned()))
        );
        assert_eq!(from_markup_value::<i32>(&content(2)), Some(7));
        let alias: SuggestionRequest = from_markup_value(&content(4)).unwrap();
        assert!(request.ptr_eq(&alias));
        let weak = request.downgrade();
        let callback: ferroboard_core::text::KeyProximity = Rc::new(move |a, b| {
            weak.upgrade().unwrap().set_word(Some("updated".into()));
            if a == b { 0.0 } else { 1.25 }
        });
        request.set_key_proximity(Some(callback.clone()));
        panel.set_data_context(None);
        panel.set_data_context(Some(Rc::new(request.clone()) as BoxedValue));
        Dispatcher::current_dispatcher().run_jobs(None);
        let carried: Option<KeyProximityRef> = from_markup_value(&content(3)).unwrap();
        let carried = carried.unwrap();
        assert!(Rc::ptr_eq(&callback, &carried.callback()));
        assert_eq!(carried.callback()(65, 66), 1.25);
        assert_eq!(alias.word(), Some("updated".into()));
        let copy = SuggestionRequest::from_values(request.values());
        copy.set_word(None);
        assert_eq!(request.word(), Some("updated".into()));
        let weak = request.downgrade();
        panel.set_data_context(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        drop(alias);
        drop(request);
        assert!(weak.upgrade().is_none());
        drop(copy);
        drop(carried);
        drop(callback);
    });
}

#[test]
fn native_locale_models_construct_bind_and_dispatch_without_virtual_cultures() {
    use ferroboard_core::{
        keys::{KeyboardMode, ShiftState},
        layouts::{IKeyEvaluator, KeyComputeContext, KeyEvaluator},
        text::TextCulture,
    };
    use ferroboard_core_ferro::{KeyComputeContextMetadata, KeyEvaluatorMetadata, KeyEvaluatorRef};
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    use ferroui_controls::ContentControl;
    run(|mode| {
        let value = load(mode, "LocaleModels");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let context: Rc<KeyComputeContext> = from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(context.culture(), Some(TextCulture::get("tr-TR").unwrap()));
        assert_eq!(context.shift_state(), ShiftState::ShiftedManual);
        let content = |i| {
            panel
                .children()
                .get(i)
                .cast::<ContentControl>()
                .unwrap()
                .content()
        };
        assert_eq!(
            from_markup_value::<Option<String>>(&content(0)),
            Some(Some("tr-TR".to_owned()))
        );
        assert_eq!(
            from_markup_value::<Option<String>>(&content(2)),
            Some(Some("TR 😀".to_owned()))
        );
        let evaluator: KeyEvaluator = from_markup_value(&content(1)).unwrap();
        assert_eq!(evaluator.mode(), KeyboardMode::Symbols);
        assert_eq!(
            evaluator.culture(),
            Some(TextCulture::get("ja-JP").unwrap())
        );
        let boxed = into_markup_value(context.clone());
        let interface: KeyEvaluatorRef = from_markup_value(&boxed).unwrap();
        assert_eq!(interface.culture(), context.culture());
        assert!(Rc::ptr_eq(
            &interface.downcast::<KeyComputeContext>().unwrap(),
            &context
        ));
        let m = KeyComputeContextMetadata::MARKUP;
        let unshifted =
            (m.find_methods("Unshifted").next().unwrap().invoke)(std::slice::from_ref(&boxed))
                .unwrap();
        let unshifted: Rc<KeyComputeContext> = from_markup_value(&unshifted).unwrap();
        assert!(!Rc::ptr_eq(&unshifted, &context));
        assert_eq!(unshifted.shift_state(), ShiftState::Unshifted);
        assert_eq!(unshifted.culture(), context.culture());
        let property = m.find_property("Culture").unwrap();
        assert!(
            (property.set.unwrap())(&[
                boxed.clone(),
                into_markup_value(Some("invalid locale !!!".to_owned()))
            ])
            .is_err()
        );
        assert_eq!(context.culture(), Some(TextCulture::get("tr-TR").unwrap()));
        for metadata in [m, KeyEvaluatorMetadata::MARKUP] {
            for name in [
                "GetHashCode",
                "Equals",
                "ToString",
                "<Clone>$",
                "Deconstruct",
                "op_Equality",
                "op_Inequality",
            ] {
                assert!(
                    metadata.find_methods(name).next().is_none(),
                    "{}::{name}",
                    metadata.name
                );
            }
        }
        let weak = Rc::downgrade(&context);
        drop(interface);
        drop(boxed);
        drop(context);
        panel.set_data_context(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn native_code_unit_buffer_constructs_and_binds_without_system_string_alias() {
    use ferroboard_core_ferro::{TextBufferTargetMetadata, Utf16TextMetadata};
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    use ferroui_controls::ContentControl;
    run(|mode| {
        let value = load(mode, "Utf16Buffer");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let target: Rc<TextBufferTarget> = from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(target.text().units(), &[0x41, 0xd83d, 0xde00, 0x65, 0x301]);
        let content = panel
            .children()
            .get(0)
            .cast::<ContentControl>()
            .unwrap()
            .content();
        let carried: Option<String> = from_markup_value(&content).unwrap();
        assert_eq!(carried.as_deref(), Some("A😀é"));
        assert_eq!(
            panel
                .children()
                .get(1)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("A😀é")
        );
        assert_eq!(Utf16TextMetadata::MARKUP.namespace(), "FerroBoard.Text");
        let invalid = Utf16Text::from_units(vec![0xd800]);
        target.set_text(Some(invalid.clone()));
        let property = TextBufferTargetMetadata::MARKUP
            .find_property("Text")
            .unwrap();
        assert!((property.get.unwrap())(&[into_markup_value(target.clone())]).is_err());
        assert_eq!(target.text(), invalid);
        panel.set_data_context(None);
    });
}

#[test]
fn native_callback_properties_bind_actual_rust_closures_and_release_captures() {
    use ferroboard_core::{
        editor::EditorClock,
        engine::{KeyboardEngine, SuggestionProviderFactory},
        layouts::KeyboardResources,
    };
    use ferroboard_core_ferro::{ClockRef, KeyboardEngineMetadata, SuggestionProviderFactoryRef};
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    use ferroui_controls::ContentControl;
    run(|mode| {
        let engine =
            KeyboardEngine::new(None, Some(Rc::new(KeyboardResources::create_empty())), None)
                .unwrap();
        let clock: EditorClock =
            Rc::new(|| std::time::UNIX_EPOCH + std::time::Duration::from_secs(10));
        let provider: Rc<dyn ISuggestionProvider> = Rc::new(ConsumerProvider::default());
        let capture = Rc::new(());
        let weak = Rc::downgrade(&capture);
        let returned = provider.clone();
        let factory: SuggestionProviderFactory = Rc::new(move |subtype| {
            assert!(subtype.is_some());
            let _retained = &capture;
            Ok(Some(returned.clone()))
        });
        let metadata = KeyboardEngineMetadata::MARKUP;
        for (name, value) in [
            (
                "Clock",
                into_markup_value(Some(ClockRef::new(clock.clone()))),
            ),
            (
                "SuggestionProviderFactory",
                into_markup_value(Some(SuggestionProviderFactoryRef::new(factory.clone()))),
            ),
        ] {
            (metadata.find_property(name).unwrap().set.unwrap())(&[
                into_markup_value(engine.clone()),
                value,
            ])
            .unwrap();
        }
        let value = load(mode, "Callbacks");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(into_markup_value(engine.clone()));
        Dispatcher::current_dispatcher().run_jobs(None);
        let content = |i| {
            panel
                .children()
                .get(i)
                .cast::<ContentControl>()
                .unwrap()
                .content()
        };
        let bound_clock: ClockRef = from_markup_value(&content(0)).unwrap();
        let bound_factory: SuggestionProviderFactoryRef = from_markup_value(&content(1)).unwrap();
        assert!(Rc::ptr_eq(&bound_clock.callback(), &clock));
        assert!(Rc::ptr_eq(&bound_factory.callback(), &factory));
        assert_eq!(
            bound_clock.callback()(),
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(10)
        );
        assert!(Rc::ptr_eq(
            &engine.suggestion_provider().unwrap().unwrap(),
            &provider
        ));
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target).unwrap();
        let attached = engine.editor().unwrap();
        assert!(Rc::ptr_eq(&attached.clock().unwrap(), &clock));
        let replacement: EditorClock =
            Rc::new(|| std::time::UNIX_EPOCH + std::time::Duration::from_secs(20));
        engine.set_clock(replacement.clone());
        assert!(Rc::ptr_eq(&attached.clock().unwrap(), &clock));
        assert!(Rc::ptr_eq(&engine.clock().unwrap(), &replacement));
        drop(bound_factory);
        drop(factory);
        engine.set_suggestion_provider_factory(None);
        assert!(
            weak.upgrade().is_some(),
            "the bound view still owns its callback"
        );
        panel.set_data_context(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "no global adapter registry owns callback captures"
        );
        engine.dispose().unwrap();
    });
}

#[test]
fn native_subtypes_construct_bind_and_share_layouts_without_record_projections() {
    use ferroboard_core::layouts::ComponentName;
    use ferroboard_core_ferro::{SubtypeLayoutMapMetadata, SubtypeMetadata};
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    use ferroui_controls::ContentControl;
    run(|mode| {
        let value = load(mode, "Subtypes");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let subtype: Arc<Subtype> = from_markup_value(&panel.data_context()).unwrap();
        let map = subtype.layouts().unwrap();
        let copy = subtype.as_ref().clone();
        let independent_map = map.as_ref().clone();
        assert!(Arc::ptr_eq(&copy.layouts().unwrap(), &map));
        let label = |i| {
            panel
                .children()
                .get(i)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .unwrap()
        };
        assert_eq!(label(0), "tr-TR/qwerty");
        assert_eq!(label(1), "TR");
        assert_eq!(label(2), "qwerty");
        assert_eq!(label(5), SubtypeLayoutMap::CORE_LAYOUTS);
        let content = |i| {
            panel
                .children()
                .get(i)
                .cast::<ContentControl>()
                .unwrap()
                .content()
        };
        let bound_map: Arc<SubtypeLayoutMap> = from_markup_value(&content(3)).unwrap();
        assert!(Arc::ptr_eq(&bound_map, &map));
        let default: Arc<Subtype> = from_markup_value(&content(4)).unwrap();
        assert!(Arc::ptr_eq(&default, &Subtype::shared_default()));
        map.set_characters(ComponentName::new("custom", "mutated"));
        assert_eq!(copy.try_id().unwrap(), Utf16Text::from("tr-TR/mutated"));
        assert_eq!(
            independent_map.characters(),
            ComponentName::new("custom", "qwerty")
        );
        panel.set_data_context(None);
        panel.set_data_context(into_markup_value(subtype.clone()));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(label(0), "tr-TR/mutated");
        assert_eq!(label(2), "mutated");
        for metadata in [SubtypeMetadata::MARKUP, SubtypeLayoutMapMetadata::MARKUP] {
            for name in [
                "GetHashCode",
                "Equals",
                "ToString",
                "<Clone>$",
                "CloneRecord",
                "op_Equality",
                "op_Inequality",
            ] {
                assert!(
                    metadata.find_methods(name).next().is_none(),
                    "{}::{name}",
                    metadata.name
                );
            }
        }
        let weak = Arc::downgrade(&subtype);
        drop(subtype);
        panel.set_data_context(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn native_codepoint_storage_roundtrips_through_real_markup_properties_and_indexers() {
    use ferroboard_core::layouts::{CodePointArray, MultiTextKeyData};
    use ferroboard_core_ferro::MultiTextKeyDataMetadata;
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    run(|mode| {
        let array = CodePointArray::new(vec![65, 0x1f600]);
        let key = Rc::new(MultiTextKeyData::default());
        let property = MultiTextKeyDataMetadata::MARKUP
            .find_property("CodePoints")
            .unwrap();
        (property.set.unwrap())(&[
            into_markup_value(key.clone()),
            into_markup_value(Some(array.clone())),
        ])
        .unwrap();
        let value = (property.get.unwrap())(&[into_markup_value(key.clone())]).unwrap();
        let actual: CodePointArray = from_markup_value(&value).unwrap();
        assert!(array.ptr_eq(&actual));
        let document = load(mode, "CodePoints");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(into_markup_value(key.clone()));
        let label = |i| {
            panel
                .children()
                .get(i)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .unwrap()
        };
        assert_eq!(label(0), "2");
        assert_eq!(label(1), "128512");
        actual.set(1, 66).unwrap();
        assert_eq!(key.code_points().unwrap().snapshot(), vec![65, 66]);
        panel.set_data_context(None);
        panel.set_data_context(into_markup_value(key));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(label(1), "66");
        panel.set_data_context(None);
    });
}

#[test]
fn popup_lookup_result_is_consumable_by_independent_compiled_and_runtime_markup() {
    use ferroboard_core::layouts::{PopupSet, ResourceDictionary};
    use ferroboard_core_ferro::popup_dictionary_metadata::{
        PopupLabelLookup, PopupLabelsDictionary, PopupLabelsDictionaryMetadata,
    };
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    run(|mode| {
        let labels = ResourceDictionary::<Utf16Text, Rc<PopupSet>>::default();
        let popup = Rc::new(PopupSet::default());
        labels.set("a".into(), Some(popup.clone()));
        labels.set("null".into(), None);
        let dictionary = PopupLabelsDictionary::from_original(labels.clone().into());
        let method = PopupLabelsDictionaryMetadata::MARKUP
            .find_methods("TryGetValue")
            .next()
            .unwrap();
        let document = load(mode, "PopupLookup");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        let label = panel.children().get(0).cast::<TextBlock>().unwrap();
        for (key, expected, has_value) in [
            ("missing", "False", false),
            ("null", "True", false),
            ("a", "True", true),
        ] {
            let result = (method.invoke)(&[
                into_markup_value(dictionary.clone()),
                into_markup_value(Some(Utf16Text::from(key))),
            ])
            .unwrap();
            let typed: PopupLabelLookup = from_markup_value(&result).unwrap();
            assert_eq!(typed.value().is_some(), has_value);
            if let Some(value) = typed.value() {
                assert!(Rc::ptr_eq(&value, &popup));
            }
            panel.set_data_context(result);
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(label.text().as_deref(), Some(expected));
        }
        panel.set_data_context(None);
    });
}

#[test]
fn dictionary_metadata_bindings_use_actual_native_stores_and_release_their_owners() {
    use ferroboard_core::text::{DictionarySuggestionProvider, UserDictionary, WordDictionary};
    use ferroboard_core_ferro::WordDictionaryMetadata;
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    run(|mode| {
        let from_text = WordDictionaryMetadata::MARKUP
            .find_methods("FromText")
            .find(|m| m.parameters.len() == 1)
            .unwrap();
        let value =
            (from_text.invoke)(&[into_markup_value(Some(Utf16Text::from("hello 42")))]).unwrap();
        let dictionary: Arc<WordDictionary> = from_markup_value(&value).unwrap();
        let weak = Arc::downgrade(&dictionary);
        let user = UserDictionary::default();
        user.set_learn_threshold(1);
        let provider = Rc::new(DictionarySuggestionProvider::new_nullable(
            Some(dictionary.clone()),
            Some(user.clone()),
        ));
        let document = load(mode, "DictionaryModels");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(into_markup_value(provider.clone()));
        let text = |i| {
            panel
                .children()
                .get(i)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .unwrap()
        };
        assert_eq!(text(0), "en");
        assert_eq!(text(1), "1");
        assert_eq!(text(2), "1");
        let actual = provider.dictionary().unwrap();
        assert!(Arc::ptr_eq(&actual, &dictionary));
        actual.add(&"world".into(), 7).unwrap();
        provider.user_dictionary().set_learn_threshold(3);
        assert_eq!(user.learn_threshold(), 3);
        panel.set_data_context(None);
        panel.set_data_context(into_markup_value(provider.clone()));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(text(1), "2");
        assert_eq!(text(2), "3");
        panel.set_data_context(None);
        drop(actual);
        drop(dictionary);
        drop(value);
        drop(provider);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn native_settings_and_action_lists_bind_through_domain_names_and_items_sources() {
    use ferroboard_core_ferro::{PerformedActionList, SettingsSubtypeList};
    use ferroui_base::metadata::into_markup_value;
    use ferroui_controls::{ContentControl, ItemsControl, ListBox};
    run(|mode| {
        let subtypes = SettingsSubtypeList::new();
        subtypes.add(Some("en-US/qwerty".into()));
        subtypes.add(None);
        subtypes.add(Some("ja-JP/kana".into()));
        let target = TextBufferTarget::default();
        for action in [EnterAction::Go, EnterAction::Done, EnterAction::Search] {
            target.perform_enter_action(action);
        }
        let actions = PerformedActionList::from_original(target.performed_actions());
        for (name, value) in [
            ("SubtypeList", into_markup_value(subtypes.clone())),
            ("PerformedActions", into_markup_value(actions.clone())),
        ] {
            let document = load(mode, name);
            let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
            panel.set_data_context(value.clone());
            let count = panel.children().get(0).cast::<TextBlock>().unwrap();
            assert_eq!(count.text().as_deref(), Some("3"));
            let first = panel
                .children()
                .get(1)
                .cast::<ContentControl>()
                .unwrap()
                .content();
            if name == "SubtypeList" {
                assert_eq!(
                    from_markup_value::<String>(&first).as_deref(),
                    Some("en-US/qwerty")
                );
            } else {
                assert_eq!(
                    from_markup_value::<EnterAction>(&first),
                    Some(EnterAction::Go)
                );
            }
            let compiled = panel.children().get(2).cast::<ItemsControl>().unwrap();
            let runtime = panel.children().get(3).cast::<ItemsControl>().unwrap();
            let source = compiled.items_source().unwrap();
            assert!(source.ptr_eq(&runtime.items_source().unwrap()));
            assert_eq!(source.count(), 3);
            assert!(!source.is_notifying());
            if name == "SubtypeList" {
                assert!(source.get_at(1).is_none());
                subtypes.add(Some("de-DE/qwertz".into()));
            } else {
                assert_eq!(
                    from_markup_value::<EnterAction>(&source.get_at(1)),
                    Some(EnterAction::Done)
                );
                target.perform_enter_action(EnterAction::Next);
            }
            assert_eq!(source.count(), 4, "native vector reads remain live");
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(
                count.text().as_deref(),
                Some("3"),
                "source lists do not notify"
            );
            let list_box = panel.children().get(2).cast::<ListBox>().unwrap();
            list_box.set_selected_index(0);
            assert!(list_box.selected_item().is_some());
            panel.set_data_context(None);
            assert!(compiled.items_source().is_none());
            assert!(runtime.items_source().is_none());
            panel.set_data_context(value);
            assert_eq!(
                count.text().as_deref(),
                Some("4"),
                "explicit rebinding refreshes a non-notifying list"
            );
            panel.set_data_context(None);
        }
    });
}

#[test]
fn native_domain_list_bindings_share_real_keys_predictions_and_suggestions() {
    use ferroboard_core::layouts::{
        Key, PopupSet, ResourceObjectList, ResourceReadOnlyList, TextKey, TextKeyData,
    };
    use ferroboard_core::text::{PredictionListBuffer, SuggestionList};
    use ferroboard_core_ferro::{
        KeyReadOnlyList, SuggestionReadOnlyList, TextKeyReadOnlyList, native_string_list,
    };
    use ferroui_base::metadata::into_markup_value;
    use ferroui_controls::ContentControl;
    run(|mode| {
        let text: TextKey = Rc::new(TextKeyData::default());
        text.data().set_label(Some("key 😀".into()));
        let keys = ResourceObjectList::new(vec![text.clone()]);
        let text_view = TextKeyReadOnlyList::from_original(keys.clone().into());
        let base_view = text_view.as_keys();
        let candidate = Rc::new(Suggestion::new("word 😀".into(), SuggestionKind::Typed));
        let suggestions =
            SuggestionReadOnlyList::from_original(SuggestionList::new(vec![candidate.clone()]));
        let predictions = native_string_list(Rc::new(PredictionListBuffer::new(vec![
            "prediction 😀".into(),
        ])));
        for (name, value, expected) in [
            ("KeyList", into_markup_value(base_view.clone()), None),
            (
                "TextKeyList",
                into_markup_value(text_view.clone()),
                Some("key 😀"),
            ),
            (
                "SuggestionList",
                into_markup_value(suggestions.clone()),
                Some("word 😀"),
            ),
            (
                "PredictionList",
                into_markup_value(predictions.clone()),
                Some("prediction 😀"),
            ),
        ] {
            let document = load(mode, name);
            let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
            panel.set_data_context(value);
            assert_eq!(
                panel
                    .children()
                    .get(0)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("1")
            );
            if let Some(expected) = expected {
                assert_eq!(
                    panel
                        .children()
                        .get(1)
                        .cast::<TextBlock>()
                        .unwrap()
                        .text()
                        .as_deref(),
                    Some(expected)
                );
            } else {
                let actual = panel
                    .children()
                    .get(1)
                    .cast::<ContentControl>()
                    .unwrap()
                    .content();
                let actual: Key = from_markup_value(&actual).unwrap();
                let original: Key = text.clone();
                assert!(Rc::ptr_eq(&actual, &original));
            }
            for index in [2, 3] {
                let items = panel
                    .children()
                    .get(index)
                    .cast::<ferroui_controls::ItemsControl>()
                    .unwrap();
                let source = items.items_source().expect("native list binding");
                assert_eq!(source.count(), 1);
                let first = source.get_at(0);
                match name {
                    "KeyList" => {
                        let actual: Key = from_markup_value(&first).unwrap();
                        assert!(Rc::ptr_eq(&actual, &(text.clone() as Key)));
                    }
                    "TextKeyList" => {
                        let actual: TextKey = from_markup_value(&first).unwrap();
                        assert!(Rc::ptr_eq(&actual, &text));
                    }
                    "SuggestionList" => {
                        let actual: Rc<Suggestion> = from_markup_value(&first).unwrap();
                        assert!(Rc::ptr_eq(&actual, &candidate));
                    }
                    "PredictionList" => {
                        assert_eq!(from_markup_value::<String>(&first).as_deref(), expected)
                    }
                    _ => unreachable!(),
                }
            }
            let list_box = panel
                .children()
                .get(2)
                .cast::<ferroui_controls::ListBox>()
                .unwrap();
            list_box.set_selected_index(0);
            assert_eq!(list_box.selected_index(), 0);
            assert!(list_box.selected_item().is_some());
            panel.set_data_context(None);
            for index in [2, 3] {
                let items = panel
                    .children()
                    .get(index)
                    .cast::<ferroui_controls::ItemsControl>()
                    .unwrap();
                assert!(
                    items.items_source().is_none(),
                    "detached data context clears the source"
                );
            }
        }
        keys.push(text.clone());
        assert_eq!(text_view.count().unwrap(), 2);
        assert_eq!(base_view.count().unwrap(), 2);
        let document = load(mode, "TextKeyList");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        text.data().set_label(Some("changed".into()));
        panel.set_data_context(into_markup_value(text_view));
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            panel
                .children()
                .get(1)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("changed")
        );
        panel.set_data_context(None);
        let popup = Rc::new(PopupSet::default());
        popup.set_relevant(Some(base_view.original()));
        let roundtrip = KeyReadOnlyList::from_original(popup.relevant().unwrap());
        assert!(roundtrip == base_view);
        assert!(ResourceReadOnlyList::ptr_eq(
            &roundtrip.original(),
            &base_view.original()
        ));
    });
}

#[test]
fn popup_entry_snapshots_bind_native_keys_and_actual_shared_popup_values() {
    use ferroboard_core::layouts::{PopupSet, ResourceDictionary};
    use ferroboard_core_ferro::popup_dictionary_metadata::PopupLabelsDictionary;
    use ferroui_base::metadata::into_markup_value;
    use ferroui_controls::ContentControl;
    run(|mode| {
        let popup = Rc::new(PopupSet::default());
        let map = ResourceDictionary::<Utf16Text, Rc<PopupSet>>::default();
        map.set("a 😀".into(), Some(popup.clone()));
        let view = PopupLabelsDictionary::from_original(map.clone().into());
        let snapshot = view.snapshot().unwrap();
        let document = load(mode, "PopupSnapshots");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(into_markup_value(snapshot));
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            panel
                .children()
                .get(1)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("a 😀")
        );
        let item = panel
            .children()
            .get(2)
            .cast::<ContentControl>()
            .unwrap()
            .content();
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<PopupSet>>(&item).unwrap(),
            &popup
        ));
        map.set("b".into(), None);
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("1")
        );
        panel.set_data_context(into_markup_value(view.snapshot().unwrap()));
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("2")
        );
        panel.set_data_context(None);
    });
}

#[test]
fn empty_arrangement_static_property_preserves_native_identity_and_row_binding() {
    use ferroboard_core::layouts::LayoutArrangement;
    use ferroui_controls::ContentControl;
    run(|mode| {
        let expected = LayoutArrangement::empty();
        for _ in 0..2 {
            let document = load(mode, "EmptyArrangement");
            let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
            let context: Rc<LayoutArrangement> = from_markup_value(&panel.data_context()).unwrap();
            let content = panel.children().get(0).cast::<ContentControl>().unwrap();
            let value: Rc<LayoutArrangement> = from_markup_value(&content.content()).unwrap();
            assert!(Rc::ptr_eq(&expected, &context));
            assert!(Rc::ptr_eq(&expected, &value));
            assert_eq!(value.rows.as_ref().unwrap().count().unwrap(), 0);
            assert_eq!(
                panel
                    .children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("0")
            );
            panel.set_data_context(None);
            content.set_content(None);
        }
    });
}

#[test]
fn computed_rows_and_lazy_query_snapshots_work_through_independent_markup() {
    use ferroboard_core::keys::KeyboardMode;
    use ferroboard_core::layouts::{
        ComputedKeyboard, KeyComputeContext, KeyboardResources, LayoutComputer, Subtype,
    };
    use ferroboard_core_ferro::computed_collection_metadata::{
        ComputedKeyQuery, ComputedKeyQueryMetadata, ComputedKeyReadOnlyList,
    };
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    use ferroui_controls::ContentControl;
    run(|mode| {
        let computer = LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()));
        let board: Rc<ComputedKeyboard> = Rc::new(
            computer
                .compute(
                    KeyboardMode::Characters,
                    Arc::new(Subtype::default()),
                    false,
                )
                .unwrap(),
        );
        board
            .compute(Rc::new(KeyComputeContext::default()))
            .unwrap();
        let document = load(mode, "ComputedRows");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(into_markup_value(board.clone()));
        let text = |i| {
            panel
                .children()
                .get(i)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .unwrap()
        };
        assert_eq!(text(0), board.row_count().to_string());
        assert_eq!(text(1), board.rows().get(0).unwrap().len().to_string());
        assert_eq!(
            text(2),
            board
                .rows()
                .get(0)
                .unwrap()
                .get(0)
                .unwrap()
                .label()
                .unwrap()
                .to_utf8()
                .unwrap()
        );
        let value = panel
            .children()
            .get(3)
            .cast::<ContentControl>()
            .unwrap()
            .content();
        let query: ComputedKeyQuery = from_markup_value(&value).unwrap();
        let method = ComputedKeyQueryMetadata::MARKUP
            .find_methods("Snapshot")
            .next()
            .unwrap();
        let snapshot = (method.invoke)(std::slice::from_ref(&value)).unwrap();
        let list: ComputedKeyReadOnlyList = from_markup_value(&snapshot).unwrap();
        let expected = query.iter().count();
        assert_eq!(list.count().unwrap() as usize, expected);
        assert!(Rc::ptr_eq(
            &list.get(0).unwrap().unwrap(),
            &board.rows().get(0).unwrap().get(0).unwrap()
        ));
        let snapshot_document = load(mode, "ComputedSnapshot");
        let snapshot_panel = snapshot_document.downcast_ref::<Ref<StackPanel>>().unwrap();
        snapshot_panel.set_data_context(snapshot);
        assert_eq!(
            snapshot_panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some(expected.to_string().as_str())
        );
        board.rows().get(0).unwrap().push_nullable(None);
        assert_eq!(
            query.snapshot().unwrap().count().unwrap() as usize,
            expected + 1
        );
        assert_eq!(list.count().unwrap() as usize, expected);
        panel.set_data_context(None);
        snapshot_panel.set_data_context(None);
    });
}

#[test]
fn custom_scheduler_uses_duration_and_cancels_on_keyboard_drop() {
    use ferroboard_core::input::{IKeyboardScheduler, KeyboardTouchProcessor, ScheduledAction};
    use std::{cell::Cell, time::Duration};
    struct Work {
        delay: Duration,
        action: Rc<dyn Fn()>,
        cancelled: Rc<Cell<bool>>,
    }
    #[derive(Default)]
    struct Scheduler(RefCell<Vec<Work>>);
    impl IKeyboardScheduler for Scheduler {
        fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
            let cancelled = Rc::new(Cell::new(false));
            self.0.borrow_mut().push(Work {
                delay,
                action,
                cancelled: cancelled.clone(),
            });
            ScheduledAction::new(move || cancelled.set(true))
        }
    }
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_show_suggestions(false);
    let engine = ferroboard_core::engine::KeyboardEngine::new_explicit(
        settings.clone(),
        Rc::new(ferroboard_core::layouts::KeyboardResources::create_default().unwrap()),
        ferroboard_core::engine::LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone()).unwrap();
    engine.update_layout(800.0, 240.0, None).unwrap();
    let scheduler = Rc::new(Scheduler::default());
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
    let bounds = engine
        .keyboard()
        .unwrap()
        .find_key(i32::from(b'e'))
        .unwrap()
        .unwrap()
        .touch_bounds();
    touch
        .pointer_down(1, bounds.center_x(), bounds.center_y())
        .unwrap();
    assert_eq!(scheduler.0.borrow().len(), 1);
    let work = scheduler.0.borrow();
    assert_eq!(
        work[0].delay,
        Duration::from_millis(settings.long_press_delay() as u64)
    );
    assert!(!work[0].cancelled.get());
    let cancelled = work[0].cancelled.clone();
    let action = work[0].action.clone();
    drop(work);
    let weak = Rc::downgrade(&touch);
    drop(touch);
    assert!(weak.upgrade().is_none());
    assert!(cancelled.get());
    action(); // A retained callback from a faulty scheduler cannot revive the receiver.
    assert!(target.text().is_empty());
}

#[test]
fn native_duration_windows_cross_compiled_and_runtime_markup_without_wrappers() {
    use ferroboard_core::editor::EditorController;
    use ferroboard_core::engine::KeyboardEngine;
    use ferroui_controls::ContentControl;
    use std::time::Duration;
    run(|mode| {
        let value = load(mode, "TimeWindows");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let engine: Rc<KeyboardEngine> = from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(
            engine.double_space_window(),
            Duration::from_nanos(123_456_789)
        );
        let content = panel.children().get(0).cast::<ContentControl>().unwrap();
        assert_eq!(
            from_markup_value::<Duration>(&content.content()),
            Some(engine.double_space_window())
        );
        let editor = EditorController::new(Rc::new(TextBufferTarget::default()));
        editor.set_undo_group_timeout(Duration::MAX);
        assert_eq!(editor.undo_group_timeout(), Duration::MAX);
        engine.dispose().unwrap();
    });
}

#[test]
fn popup_alternatives_compile_and_bind_through_actual_native_shared_storage() {
    use ferroboard_core::{keys::KeyboardMode, layouts::*};
    use ferroboard_core_ferro::PopupKeysView;
    use ferroui_base::metadata::into_markup_value;
    run(|mode| {
        let board = LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
            .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
            .unwrap();
        board
            .compute(Rc::new(KeyComputeContext::default()))
            .unwrap();
        let key = board
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|key| key.has_popup())
            .unwrap();
        let values = key.popup_keys();
        let view = PopupKeysView::new(values.clone());
        assert!(view.source().ptr_eq(&values));
        assert!(view.get(-1).is_err());
        assert!(view.get(i32::MAX).is_err());
        let document = load(mode, "PopupKeys");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(into_markup_value(key.clone()));
        let text = |i| {
            panel
                .children()
                .get(i)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .unwrap()
        };
        assert_eq!(text(0), values.len().to_string());
        assert_eq!(text(1), values.get(0).unwrap().data().code().to_string());
        values.push(TextKeyData::char("z".into(), None).unwrap());
        assert_eq!(view.count().unwrap() as usize, values.len());
        panel.set_data_context(None);
        panel.set_data_context(into_markup_value(key));
        assert_eq!(text(0), values.len().to_string());
    });
}

#[test]
fn native_model_setters_construct_and_bind_without_init_only_metadata() {
    use ferroboard_core::editor::{CapitalizationMode, InputKind};
    use ferroboard_core_ferro::InputAttributesMetadata;
    use ferroui_base::metadata::MarkupTyped;
    use ferroui_controls::ContentControl;
    run(|mode| {
        let document = load(mode, "NativeModels");
        let panel = document.downcast_ref::<Ref<StackPanel>>().unwrap();
        let attributes: Arc<InputAttributes> = from_markup_value(&panel.data_context()).unwrap();
        assert_eq!(attributes.kind(), InputKind::Email);
        assert_eq!(attributes.enter_action(), EnterAction::Send);
        assert_eq!(attributes.capitalization(), CapitalizationMode::None);
        assert!(attributes.is_multiline());
        assert!(!attributes.allow_suggestions());
        assert!(!attributes.allow_auto_correct());
        assert!(attributes.is_private());
        let field = panel.children().get(0).cast::<ContentControl>().unwrap();
        assert_eq!(
            from_markup_value::<InputKind>(&field.content()),
            Some(InputKind::Email)
        );
        let field = panel.children().get(1).cast::<ContentControl>().unwrap();
        assert_eq!(from_markup_value::<bool>(&field.content()), Some(true));
        for property in &InputAttributesMetadata::MARKUP.properties[..7] {
            assert!(property.set.is_some());
            assert!(!property.attributes.iter().any(|a| a.name == "InitOnly"));
        }
        let snapshot = attributes.as_ref().clone();
        attributes.set_kind(InputKind::Search);
        let alias: Arc<InputAttributes> = from_markup_value(&panel.data_context()).unwrap();
        assert!(Arc::ptr_eq(&attributes, &alias));
        assert_eq!(alias.kind(), InputKind::Search);
        assert_eq!(snapshot.kind(), InputKind::Email);
    });
}
