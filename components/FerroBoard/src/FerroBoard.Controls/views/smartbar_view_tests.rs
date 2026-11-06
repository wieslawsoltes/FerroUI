use super::*;
use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::LanguageModels,
    layouts::KeyboardResources,
    text::{
        ISuggestionProvider, SuggestionError, SuggestionList, SuggestionRequest, SuggestionResult,
        SuggestionValues,
    },
};
use ferroui_base::{
    Point,
    input::{MouseButton, RawInputModifiers},
    threading::Dispatcher,
};
use ferroui_controls::Window;
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::{Value, json};
use std::time::Duration;
struct Clock;
impl IKeyboardScheduler for Clock {
    fn schedule(&self, _: Duration, _: Rc<dyn Fn()>) -> ScheduledAction {
        panic!("unexpected timer in smartbar construction test")
    }
}
struct Provider(RefCell<Vec<Rc<Suggestion>>>);
impl PartialEq for Provider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for Provider {
    fn suggest(&self, _: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        Ok(Some(SuggestionList::new(self.0.borrow().clone())))
    }
    fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
}
fn engine() -> Rc<KeyboardEngine> {
    let settings = Rc::new(ferroboard_core::settings::KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    let e = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    e.attach(Rc::new(TextBufferTarget::default())).unwrap();
    e
}
use crate::view_fixture::*;
fn suggestions(count: usize) -> Vec<Rc<Suggestion>> {
    (0..count)
        .map(|i| {
            Rc::new(Suggestion::from_values(SuggestionValues {
                is_auto_commit: i == 0,
                is_removable: i == 0,
                ..SuggestionValues::new(
                    match i {
                        0 => "alpha",
                        1 => "🙂",
                        _ => "line\nnext\r\nend",
                    }
                    .into(),
                    match i {
                        0 => SuggestionKind::Correction,
                        1 => SuggestionKind::Emoji,
                        _ => SuggestionKind::Clipboard,
                    },
                )
            }))
        })
        .collect()
}
#[test]
fn unchanged_smartbar_source_view_search_actions_and_format_corpus() {
    crate::test_support::run(|| {
        let e = engine();
        let provider = Rc::new(Provider(RefCell::new(vec![])));
        let p = provider.clone();
        e.set_suggestion_provider_factory(Some(Rc::new(move |_| Ok(Some(p.clone())))));
        let view = SmartbarView::new(e.clone(), Rc::new(Clock));
        assert_eq!(
            include_str!("../../../tests/fixtures/smartbar.jsonl")
                .lines()
                .count(),
            153
        );
        for line in include_str!("../../../tests/fixtures/smartbar.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            match row["op"].as_str().unwrap() {
                "view" => {
                    view.set_emoji_search(None);
                    e.settings().set_smartbar_layout(SmartbarLayout(
                        row["layout"].as_i64().unwrap() as i32,
                    ));
                    *provider.0.borrow_mut() = suggestions(row["count"].as_u64().unwrap() as usize);
                    e.update_suggestions().unwrap();
                    view.actions_expanded
                        .set(row["expanded"].as_bool().unwrap());
                    let theme = KeyboardThemes::get(row["theme"].as_str().unwrap()).unwrap();
                    view.set_theme(theme, float(&row["height"])).unwrap();
                    compare(&view, &row["tree"]);
                }
                "font" => {
                    view.set_emoji_search(None);
                    e.settings()
                        .set_smartbar_layout(SmartbarLayout::SuggestionsOnly);
                    *provider.0.borrow_mut() = suggestions(3);
                    e.update_suggestions().unwrap();
                    view.actions_expanded.set(false);
                    let theme = KeyboardThemes::floris_day().clone_theme().unwrap();
                    theme.set_font_family(Some("Fixture Family".into()));
                    view.set_theme(theme, 44.0).unwrap();
                    compare(&view, &row["tree"]);
                }
                "localized" => {
                    let old = (
                        KeyboardStrings::quick_actions(),
                        KeyboardStrings::hide_keyboard(),
                        KeyboardStrings::paste_format(),
                        KeyboardStrings::undo(),
                    );
                    KeyboardStrings::set_quick_actions(Some("Actions🙂".into()));
                    KeyboardStrings::set_hide_keyboard(None);
                    KeyboardStrings::set_paste_format(Some("clip<{0}>".into()));
                    KeyboardStrings::set_undo(Some("Changed undo".into()));
                    view.refresh().unwrap();
                    let captured = view
                        .actions()
                        .unwrap()
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)[0]
                        .as_ref()
                        .unwrap()
                        .name
                        .clone();
                    // Restore process-wide values even if an assertion fails afterward.
                    KeyboardStrings::set_quick_actions(old.0);
                    KeyboardStrings::set_hide_keyboard(old.1);
                    KeyboardStrings::set_paste_format(old.2);
                    KeyboardStrings::set_undo(old.3);
                    assert_eq!(
                        json!(captured.as_ref().map(Utf16Text::units)),
                        row["capturedUndo"]
                    );
                    compare(&view, &row["tree"]);
                }
                "search" => {
                    let search = EmojiSearchState::new(Some(Rc::new(|_| Ok(()))));
                    *search.query.borrow_mut() = Some(row["query"].as_str().unwrap().into());
                    if row["count"] == 2 {
                        *search.results.borrow().as_ref().unwrap().borrow_mut() =
                            vec![Some("🙂".into()), Some("🙌".into())];
                    }
                    view.set_emoji_search(Some(search));
                    view.set_theme(KeyboardThemes::floris_day(), 44.0).unwrap();
                    compare(&view, &row["tree"]);
                }
                "action" => {
                    view.set_emoji_search(None);
                    let list = view.actions().unwrap();
                    let snapshot = list
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clone();
                    let action = snapshot
                        .into_iter()
                        .map(Option::unwrap)
                        .find(|a| a.id == Some(row["id"].as_str().unwrap().into()))
                        .unwrap();
                    assert_eq!(
                        json!(action.is_active.as_ref().map(|f| f(&e))),
                        row["before"]
                    );
                    assert_eq!(json!(action.icon.0), row["icon"]);
                    assert_eq!(
                        json!(action.name.as_ref().map(Utf16Text::units)),
                        row["name"]
                    );
                    action.execute.as_ref().unwrap()(&e).unwrap();
                    assert_eq!(
                        json!(action.is_active.as_ref().map(|f| f(&e))),
                        row["after"]
                    );
                    assert_eq!(e.mode().0, row["mode"].as_i64().unwrap() as i32);
                    assert_eq!(e.ui_mode().0, row["panel"].as_i64().unwrap() as i32);
                }
                "format" => {
                    let units = |v: &Value| {
                        v.as_array().map(|a| {
                            Utf16Text::from_units(
                                a.iter()
                                    .map(|n| n.as_u64().unwrap() as u16)
                                    .collect::<Vec<_>>(),
                            )
                        })
                    };
                    let format = units(&row["format"]);
                    let result = format_one(format.as_ref(), &units(&row["text"]).unwrap());
                    match result {
                        Ok(s) => {
                            assert!(row["error"].is_null(), "{row}");
                            assert_eq!(s.len(), row["length"].as_u64().unwrap() as usize);
                            if !row["result"].is_null() {
                                assert_eq!(json!(s.units()), row["result"]);
                            }
                            assert_eq!(json!(&s.units()[..s.len().min(8)]), row["prefix"]);
                            assert_eq!(
                                json!(&s.units()[s.len().saturating_sub(8)..]),
                                row["suffix"]
                            );
                        }
                        Err(e) => {
                            let name = match e {
                                ViewError::ArgumentNull => "ArgumentNullException",
                                ViewError::Format => "FormatException",
                                _ => panic!("{e}"),
                            };
                            assert_eq!(name, row["error"].as_str().unwrap(), "{row}");
                        }
                    }
                }
                _ => panic!("fixture operation"),
            }
        }
        // Current action buttons capture their view strongly as in source. Clearing source visual
        // children here releases test-owned cycles; the complete owner needs an explicit lifetime bridge.
        view.actions.children().clear();
        e.dispose().unwrap();
    });
}

use crate::test_scheduler as scheduler;
fn show(view: &Ref<SmartbarView>) -> Ref<Window> {
    view.set_width(700.0);
    let window = Window::new();
    window.set_width(700.0);
    window.set_height(44.0);
    window.set_content(Some(Control::boxed(view.clone())));
    window.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    window
}
fn point(button: &Control, window: &Window) -> Point {
    button
        .translate_point(
            Point::new(button.bounds().width / 2.0, button.bounds().height / 2.0),
            window,
        )
        .unwrap()
}
fn click(button: &Control, window: &Window) {
    Dispatcher::current_dispatcher().run_jobs(None);
    let point = point(button, window);
    window.mouse_down(point, MouseButton::Left, RawInputModifiers::empty());
    window.mouse_up(point, MouseButton::Left, RawInputModifiers::empty());
}
#[test]
fn raw_smartbar_click_order_search_restore_and_hide_events() {
    crate::test_support::run(|| {
        let engine = engine();
        let view = SmartbarView::new(engine.clone(), Rc::new(Clock));
        let records = Rc::new(RefCell::new(Vec::new()));
        let observed = records.clone();
        let action = SmartbarAction::new(
            Some("custom".into()),
            KeyIcon::Settings,
            Some("Custom action".into()),
            Some(Rc::new(move |e| {
                observed.borrow_mut().push((e.mode(), "execute"));
                Ok(())
            })),
            None,
        );
        let actions = ActionList::from(vec![Some(action)]);
        view.set_actions(Some(actions.clone()));
        assert!(std::sync::Arc::ptr_eq(&view.actions().unwrap(), &actions));
        view.actions_expanded.set(true);
        view.set_theme(KeyboardThemes::floris_day(), 44.0).unwrap();
        let window = show(&view);
        let button = view.actions.children().get(0);
        let weak = view.downgrade();
        click(&button, &window);
        assert_eq!(&*records.borrow(), &[(KeyboardMode::Characters, "execute")]);
        assert!(!view.actions_expanded.get());
        assert!(weak.upgrade().is_some());
        let back = Rc::new(Cell::new(0));
        let count = back.clone();
        let sub = view.search_back_requested(move |_| count.set(count.get() + 1));
        let search = EmojiSearchState::new(Some(Rc::new(|_| Ok(()))));
        *search.query.borrow_mut() = Some("x".into());
        view.set_emoji_search(Some(search.clone()));
        view.refresh().unwrap();
        click(&view.toggle, &window);
        assert_eq!(back.get(), 1);
        assert!(Rc::ptr_eq(&view.emoji_search().unwrap(), &search));
        view.set_emoji_search(None);
        view.refresh().unwrap();
        let hidden = Rc::new(Cell::new(0));
        let count = hidden.clone();
        let hide_sub = view.hide_requested(move |_| count.set(count.get() + 1));
        click(&view.hide, &window);
        assert_eq!(hidden.get(), 1);
        view.remove_search_back_requested(sub);
        view.remove_hide_requested(hide_sub);
        window.close();
        view.release_visual_children();
        drop(button);
        engine.dispose().unwrap();
    });
}
#[test]
fn raw_suggestions_commit_long_press_and_search_selection_use_source_objects() {
    crate::test_support::run(|| {
        struct Tracking {
            forgot: RefCell<Vec<Utf16Text>>,
            learned: RefCell<Vec<Utf16Text>>,
        }
        impl PartialEq for Tracking {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        }
        impl ISuggestionProvider for Tracking {
            fn suggest(
                &self,
                _: Option<&SuggestionRequest>,
            ) -> SuggestionResult<Option<SuggestionList>> {
                Ok(Some(SuggestionList::new(suggestions(1))))
            }
            fn learn(&self, w: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
                self.learned
                    .borrow_mut()
                    .push(w.expect("ordinary source case has nonnull word").clone());
                Ok(())
            }
            fn forget(&self, w: Option<&Utf16Text>) -> SuggestionResult<()> {
                self.forgot
                    .borrow_mut()
                    .push(w.expect("ordinary source case has nonnull word").clone());
                Ok(())
            }
        }
        let engine = engine();
        let provider = Rc::new(Tracking {
            forgot: RefCell::new(vec![]),
            learned: RefCell::new(vec![]),
        });
        let p = provider.clone();
        engine.set_suggestion_provider_factory(Some(Rc::new(move |_| Ok(Some(p.clone())))));
        engine
            .settings()
            .set_smartbar_layout(SmartbarLayout::SuggestionsOnly);
        engine.input_text(&"alp".into()).unwrap();
        engine.update_suggestions().unwrap();
        let clock = scheduler::ManualScheduler::new();
        let view = SmartbarView::new(engine.clone(), clock.clone());
        view.set_theme(KeyboardThemes::floris_day(), 44.0).unwrap();
        let window = show(&view);
        let button = view.suggestions.children().get(0);
        let pt = point(&button, &window);
        window.mouse_down(pt, MouseButton::Left, RawInputModifiers::empty());
        clock.advance(4_499_999);
        assert!(provider.forgot.borrow().is_empty());
        clock.advance(1);
        assert_eq!(&*provider.forgot.borrow(), &[Utf16Text::from("alpha")]);
        window.mouse_up(pt, MouseButton::Left, RawInputModifiers::empty());
        assert_eq!(engine.target().unwrap().text(), "alp".into());
        click(&button, &window);
        assert!(
            engine
                .target()
                .unwrap()
                .text()
                .to_utf8()
                .unwrap()
                .starts_with("alpha")
        );
        let selected = Rc::new(RefCell::new(Vec::new()));
        let out = selected.clone();
        let state = EmojiSearchState::new(Some(Rc::new(move |s| {
            out.borrow_mut().push(s);
            Ok(())
        })));
        *state.results.borrow().as_ref().unwrap().borrow_mut() = vec![Some("🙌".into())];
        view.set_emoji_search(Some(state));
        view.refresh().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        let scroller = view.search.children().get(1);
        let panel = Control::from_boxed(
            scroller
                .downcast_ref::<ScrollViewer>()
                .unwrap()
                .content()
                .as_ref()
                .unwrap(),
        )
        .unwrap();
        let result = panel
            .downcast_ref::<StackPanel>()
            .unwrap()
            .children()
            .get(0);
        click(&result, &window);
        assert_eq!(&*selected.borrow(), &[Some("🙌".into())]);
        window.close();
        view.release_visual_children();
        engine.dispose().unwrap();
    });
}
#[test]
fn custom_action_reentrancy_failures_and_nullable_search_are_explicit() {
    crate::test_support::run(|| {
        let engine = engine();
        let view = SmartbarView::new(engine.clone(), Rc::new(Clock));
        let list = ActionList::default();
        let mutate = list.clone();
        list.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Some(SmartbarAction::new(
                Some("mutate".into()),
                KeyIcon::Settings,
                Some("mutate".into()),
                None,
                Some(Rc::new(move |_| {
                    mutate
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clear();
                    true
                })),
            )));
        view.set_actions(Some(list));
        view.actions_expanded.set(true);
        view.refresh().unwrap();
        assert_eq!(
            view.actions.children().count(),
            1,
            "current build owns its action snapshot"
        );
        view.refresh().unwrap();
        assert_eq!(
            view.actions.children().count(),
            0,
            "next refresh observes the callback's clear"
        );
        view.set_actions(None);
        assert!(matches!(view.refresh(), Err(ViewError::NullReference)));
        view.set_actions(Some(ActionList::from(vec![None])));
        assert!(matches!(view.refresh(), Err(ViewError::NullReference)));
        let search = EmojiSearchState::new(None);
        *search.query.borrow_mut() = None;
        view.set_emoji_search(Some(search.clone()));
        assert!(matches!(view.refresh(), Err(ViewError::NullReference)));
        *search.query.borrow_mut() = Some(Utf16Text::from_units(vec![0xd800]));
        assert!(matches!(view.refresh(), Err(ViewError::UnpairedSurrogate)));
        *search.query.borrow_mut() = Some("a".into());
        *search.results.borrow_mut() = None;
        assert!(matches!(view.refresh(), Err(ViewError::NullReference)));
        view.release_visual_children();
        engine.dispose().unwrap();
    });
}

#[test]
fn native_text_adapter_focus_survives_suggestion_actions_and_emoji_result_clicks() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_controls::TextBox;
        let engine = engine();
        let text = TextBox::new();
        text.set_height(32.0);
        text.set_text(Some("wo"));
        let target = TextBoxInputTarget::new(text.clone(), None).unwrap();
        target.select(2, 0);
        engine.attach(target.clone()).unwrap();
        let provider = Rc::new(Provider(RefCell::new(vec![Rc::new(Suggestion::new(
            "word".into(),
            SuggestionKind::Completion,
        ))])));
        let p = provider.clone();
        engine.set_suggestion_provider_factory(Some(Rc::new(move |_| Ok(Some(p.clone())))));
        engine.update_suggestions().unwrap();
        let view = SmartbarView::new(engine.clone(), Rc::new(Clock));
        view.set_theme(KeyboardThemes::floris_day(), 44.0).unwrap();
        view.set_width(700.0);
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(view.clone());
        let window = Window::new();
        window.set_width(700.0);
        window.set_height(76.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(text.focus());
        click(&view.suggestions.children().get(0), &window);
        assert!(text.text().unwrap().starts_with("word"));
        assert!(text.is_focused());
        view.set_actions(Some(ActionList::from(vec![Some(SmartbarAction::new(
            Some("flag".into()),
            KeyIcon::Settings,
            Some("flag".into()),
            Some(Rc::new(|e| {
                e.settings().set_split_keyboard(true);
                Ok(())
            })),
            None,
        ))])));
        click(&view.toggle, &window);
        assert!(view.actions_scroller.is_visible());
        assert!(text.is_focused());
        click(&view.actions.children().get(0), &window);
        assert!(engine.settings().split_keyboard());
        assert!(text.is_focused());
        let e = engine.clone();
        let state = EmojiSearchState::new(Some(Rc::new(move |s| {
            e.input_text(&s.unwrap())?;
            Ok(())
        })));
        *state.results.borrow().as_ref().unwrap().borrow_mut() = vec![Some("🙂".into())];
        view.set_emoji_search(Some(state));
        view.refresh().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        let scroller = view.search.children().get(1);
        let results = Control::from_boxed(
            scroller
                .downcast_ref::<ScrollViewer>()
                .unwrap()
                .content()
                .as_ref()
                .unwrap(),
        )
        .unwrap();
        click(
            &results
                .downcast_ref::<StackPanel>()
                .unwrap()
                .children()
                .get(0),
            &window,
        );
        assert!(text.text().unwrap().contains("🙂"));
        assert!(text.is_focused());
        window.close();
        view.release_visual_children();
        engine.detach().unwrap();
        target.detach();
        engine.dispose().unwrap();
    });
}

#[test]
fn source_indexed_nullable_suggestions_preserve_partial_visual_state_and_recovery() {
    use ferroboard_core::text::{SuggestionIter, SuggestionSource};
    struct Indexed {
        profile: String,
        log: Rc<RefCell<Vec<String>>>,
        value: Rc<Suggestion>,
    }
    impl SuggestionSource for Indexed {
        fn len(&self) -> Result<usize, SuggestionError> {
            self.log.borrow_mut().push("count".into());
            match self.profile.as_str() {
                "count-fault" => Err(SuggestionError::Custom("provider failure".into())),
                _ => Ok(3),
            }
        }
        fn get(&self, index: usize) -> Result<Option<Rc<Suggestion>>, SuggestionError> {
            self.log.borrow_mut().push(format!("get:{index}"));
            if index == 1 {
                Err(SuggestionError::Custom("provider failure".into()))
            } else {
                Ok(Some(self.value.clone()))
            }
        }
        fn iter(&self) -> Result<SuggestionIter<'_>, SuggestionError> {
            panic!("source BuildSuggestions uses captured Count and indexed access")
        }
    }
    crate::test_support::run(|| {
        let mut matched = 0;
        let mut retired = 0;
        for line in include_str!("../../../tests/fixtures/suggestion-view.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let profile = row["profile"].as_str().unwrap();
            if profile == "negative-count" {
                retired += 1;
                continue;
            } // usize native readers cannot report negative lengths.
            let engine = engine();
            let view = SmartbarView::new(engine.clone(), Rc::new(Clock));
            let log = Rc::new(RefCell::new(Vec::new()));
            let a = Rc::new(Suggestion::new("first".into(), SuggestionKind::Completion));
            let b = Rc::new(Suggestion::new("second".into(), SuggestionKind::Correction));
            let ordinary = SuggestionList::new(vec![a.clone(), b.clone()]);
            let values = match profile {
                "empty" => SuggestionList::default(),
                "ordinary" => ordinary.clone(),
                "null-first" => SuggestionList::from_nullable(vec![None, Some(b)]),
                "null-second" => SuggestionList::from_nullable(vec![Some(a.clone()), None]),
                _ => SuggestionList::from_custom(Rc::new(Indexed {
                    profile: profile.into(),
                    log: log.clone(),
                    value: a,
                })),
            };
            let state = || json!({"columns":view.suggestions.column_definitions().count(),"children":(0..view.suggestions.children().count()).map(|i|{let c=view.suggestions.children().get(i);if c.downcast_ref::<TouchButton>().is_some(){"TouchButton"}else{assert!(c.downcast_ref::<Border>().is_some());"Border"}}).collect::<Vec<_>>()});
            let error = view.build_suggestions(&values).err().map(|e| match e {
                ViewError::NullReference => "NullReferenceException",
                ViewError::Engine(ferroboard_core::engine::EngineError::Suggestion(
                    SuggestionError::Custom(_),
                )) => "ApplicationException",
                e => panic!("unexpected view failure {e:?}"),
            });
            let failed = state();
            view.build_suggestions(&ordinary).unwrap();
            assert_eq!(
                json!({"profile":profile,"error":error,"log":*log.borrow(),"state":failed,"recovery":state()}),
                row
            );
            view.release_visual_children();
            engine.dispose().unwrap();
            matched += 1;
        }
        assert_eq!((matched, retired), (6, 1));
    });
}
