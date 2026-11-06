use super::*;
use crate::{
    test_scheduler::ManualScheduler,
    view_fixture::{bits, compare, float},
};
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::LanguageModels,
    layouts::KeyboardResources,
    settings::KeyboardSettings,
};
use ferroui_base::{
    Rect, Size,
    input::{MouseButton, RawInputModifiers},
    threading::Dispatcher,
};
use ferroui_controls::{Window, automation::AutomationProperties};
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::{Value, json};
struct Fixture {
    engine: Rc<KeyboardEngine>,
    target: Rc<TextBufferTarget>,
    clock: Rc<ManualScheduler>,
    view: Ref<EmojiPanelView>,
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
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        let clock = ManualScheduler::new();
        let view = EmojiPanelView::new(engine.clone(), clock.clone());
        Self {
            engine,
            target,
            clock,
            view,
        }
    }
    fn page(&self) -> Ref<UniformWrapPanel> {
        Control::from_boxed(self.view.scroller.content().as_ref().unwrap())
            .unwrap()
            .downcast()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.view.release_subscriptions();
        self.engine.dispose().unwrap();
    }
}
fn page_state(f: &Fixture, source: &Value) {
    let Some(content) = f
        .view
        .scroller
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
    else {
        assert_eq!(source["present"], false);
        return;
    };
    let page = content.downcast_ref::<UniformWrapPanel>().unwrap();
    assert_eq!(source["present"], true);
    assert_eq!(bits(page.item_width()), source["width"]);
    assert_eq!(bits(page.item_height()), source["height"]);
    assert_eq!(
        page.children().count(),
        source["count"].as_u64().unwrap() as usize
    );
    let labels: Vec<_> = page
        .children()
        .snapshot()
        .iter()
        .map(|c| {
            c.downcast_ref::<TouchButton>().map(|b| {
                AutomationProperties::get_name(b).map(|s| s.encode_utf16().collect::<Vec<_>>())
            })
        })
        .collect();
    assert_eq!(json!(labels), source["labels"]);
    for sample in source["samples"].as_array().unwrap() {
        compare(
            &page
                .children()
                .get(sample["index"].as_u64().unwrap() as usize),
            &sample["tree"],
        );
    }
}
fn state(f: &Fixture, source: &Value, with_popup: bool) {
    assert_eq!(json!(f.view.current_tab.borrow().units()), source["tab"]);
    // Private cache storage order is not a UI contract. Preserve exact membership
    // and page identity while using native Rust hashing; rendered order stays exact.
    let mut cached: Vec<_> = f
        .view
        .pages
        .borrow()
        .keys()
        .map(|k| k.to_utf8().unwrap())
        .collect();
    let mut expected: Vec<_> = source["cache"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap().to_owned())
        .collect();
    cached.sort_unstable();
    expected.sort_unstable();
    assert_eq!(cached, expected);
    compare(&f.view.bottom_bar, &source["bar"]);
    page_state(f, &source["page"]);
    if with_popup {
        let popup = f.view.tone_popup.borrow().clone();
        if let Some(popup) = popup {
            compare(&popup, &source["popup"]);
        } else {
            assert!(source["popup"].is_null());
        }
    }
}
fn show(view: &Ref<EmojiPanelView>, width: f64, height: f64) -> Ref<Window> {
    view.set_width(width);
    view.set_height(height);
    view.set_use_layout_rounding(false);
    let w = Window::new();
    w.set_width(width);
    w.set_height(height);
    w.set_content(Some(Control::boxed(view.clone())));
    w.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    w
}
#[test]
fn unchanged_source_emoji_categories_tabs_cache_popup_and_enum_corpus() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let fixture = include_str!("../../../tests/fixtures/emoji-panel.jsonl");
        assert_eq!(fixture.lines().count(), 105);
        let mut last_theme = String::new();
        let mut last_bar = 0.0;
        let mut window = None;
        for line in fixture.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            match row["op"].as_str().unwrap() {
                "tab" => {
                    let theme = row["theme"].as_str().unwrap();
                    let bar = float(&row["bar"]);
                    if theme != last_theme || bar != last_bar {
                        f.view.set_bar_height(bar);
                        f.view
                            .set_theme(KeyboardThemes::get(theme).unwrap())
                            .unwrap();
                        last_theme = theme.to_owned();
                        last_bar = bar;
                    }
                    f.view
                        .show_tab(row["tab"].as_str().unwrap().into())
                        .unwrap();
                    state(&f, &row["state"], true);
                }
                "history-add" => {
                    f.view.show_tab("Recent".into()).unwrap();
                    let old = f.page();
                    f.engine
                        .emoji_history()
                        .add(&row["emoji"].as_str().unwrap().into())
                        .unwrap();
                    assert_eq!(old, f.page());
                    state(&f, &row["state"], true);
                }
                "shown" => {
                    f.view.on_shown().unwrap();
                    state(&f, &row["state"], true);
                }
                "history-category" => {
                    f.view.show_tab("SmileysEmotion".into()).unwrap();
                    let old = f.page();
                    f.engine
                        .emoji_history()
                        .add(&row["emoji"].as_str().unwrap().into())
                        .unwrap();
                    f.view.on_shown().unwrap();
                    assert_eq!(old, f.page());
                    state(&f, &row["state"], true);
                }
                "popup" => {
                    if window.is_none() {
                        window = Some(show(&f.view, 640.25, 300.0));
                    }
                    f.view.set_width(float(&row["width"]));
                    window.as_ref().unwrap().set_width(float(&row["width"]));
                    f.view
                        .show_tab(
                            if row["recent"] == true {
                                "Recent"
                            } else {
                                "PeopleBody"
                            }
                            .into(),
                        )
                        .unwrap();
                    let anchor = TouchButton::new();
                    anchor.set_use_layout_rounding(false);
                    anchor.set_width(44.5);
                    anchor.set_height(42.25);
                    Canvas::set_left(&anchor, float(&row["x"]));
                    Canvas::set_top(&anchor, float(&row["y"]));
                    f.view.overlay.children().add(anchor.clone());
                    Dispatcher::current_dispatcher().run_jobs(None);
                    let emoji = if row["tone"] == true { "👍" } else { "🙂" };
                    f.view
                        .on_emoji_long_press(
                            &anchor,
                            emoji.into(),
                            f.engine.emoji_catalog().unwrap().get(&emoji.into()),
                        )
                        .unwrap();
                    state(&f, &row["state"], true);
                    f.view.overlay.children().remove(anchor);
                }
                "variant-reentry" => {
                    let window = window.as_ref().unwrap();
                    f.view.set_width(640.25);
                    window.set_width(640.25);
                    f.view.show_tab("PeopleBody".into()).unwrap();
                    let anchor = TouchButton::new();
                    anchor.set_use_layout_rounding(false);
                    anchor.set_width(44.5);
                    anchor.set_height(42.25);
                    Canvas::set_left(&anchor, 20.0);
                    Canvas::set_top(&anchor, 60.0);
                    f.view.overlay.children().add(anchor.clone());
                    Dispatcher::current_dispatcher().run_jobs(None);
                    f.view
                        .on_emoji_long_press(
                            &anchor,
                            "👍".into(),
                            f.engine.emoji_catalog().unwrap().get(&"👍".into()),
                        )
                        .unwrap();
                    let view = f.view.clone();
                    let token = f
                        .engine
                        .text_committed()
                        .add(Rc::new(move |_| view.show_tab("Recent".into()).unwrap()));
                    click(&popup_row(&f).children().get(1), window);
                    f.engine.text_committed().remove(token);
                    assert_eq!(json!(f.target.text().units()), row["text"]);
                    state(&f, &row["state"], true);
                    f.view.overlay.children().remove(anchor);
                }
                "enum" => {
                    let id = Utf16Text::from_units(
                        row["id"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|n| n.as_u64().unwrap() as u16)
                            .collect::<Vec<_>>(),
                    );
                    let parsed = parse_enum_case_sensitive(Some(&id), &CATEGORY_NAMES);
                    assert_eq!(parsed.is_some(), row["success"].as_bool().unwrap());
                    assert_eq!(parsed.unwrap_or(0), row["value"].as_i64().unwrap() as i32);
                }
                "inside" => {
                    let parent = Grid::new();
                    let mid = Grid::new();
                    let child = TextBlock::new();
                    if row["related"] == true {
                        parent.children().add(mid.clone());
                    }
                    mid.children().add(child.clone());
                    assert_eq!(
                        EmojiPanelView::is_inside(&parent, Some(child.upcast())),
                        row["result"].as_bool().unwrap()
                    );
                }
                _ => panic!("unknown source op"),
            }
        }
        if let Some(w) = window {
            w.close();
        }
    });
}

fn button_for(page: &UniformWrapPanel, value: &str) -> Ref<TouchButton> {
    page.children()
        .snapshot()
        .iter()
        .find(|c| {
            c.downcast_ref::<TouchButton>()
                .is_some_and(|b| AutomationProperties::get_name(b).as_deref() == Some(value))
        })
        .unwrap_or_else(|| panic!("missing emoji cell {value}"))
        .clone()
        .downcast()
        .unwrap()
}
fn popup_row(f: &Fixture) -> Ref<StackPanel> {
    f.view
        .tone_popup
        .borrow()
        .as_ref()
        .unwrap()
        .child()
        .unwrap()
        .downcast()
        .unwrap()
}
fn point(c: &Control, w: &Window) -> Point {
    c.translate_point(
        Point::new(c.bounds().width / 2.0, c.bounds().height / 2.0),
        w,
    )
    .unwrap()
}
fn click(c: &Control, w: &Window) {
    Dispatcher::current_dispatcher().run_jobs(None);
    let p = point(c, w);
    w.mouse_down(p, MouseButton::Left, RawInputModifiers::empty());
    w.mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
}
fn long_press(f: &Fixture, c: &Control, w: &Window) {
    Dispatcher::current_dispatcher().run_jobs(None);
    let p = point(c, w);
    w.mouse_down(p, MouseButton::Left, RawInputModifiers::empty());
    f.clock.advance(4_500_000);
    w.mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
}
#[test]
fn raw_long_press_variant_selection_and_recent_pin_remove_follow_source_order() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine.emoji_history().add(&"👍".into()).unwrap();
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.view.on_shown().unwrap();
        let window = show(&f.view, 640.0, 300.0);
        let anchor = button_for(&f.page(), "👍");
        let p = point(&anchor, &window);
        window.mouse_down(p, MouseButton::Left, RawInputModifiers::empty());
        f.clock.advance(4_499_999);
        assert!(f.view.tone_popup.borrow().is_none());
        f.clock.advance(1);
        assert_eq!(popup_row(&f).children().count(), 8);
        window.mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
        assert!(f.target.text().is_empty());
        let row = popup_row(&f);
        let variant = row.children().get(1);
        let p = point(&variant, &window);
        window.mouse_down(p, MouseButton::Left, RawInputModifiers::empty());
        assert!(f.view.tone_popup.borrow().is_some());
        window.mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
        assert!(f.view.tone_popup.borrow().is_none());
        assert_eq!(f.target.text(), "👍🏻".into());
        assert_eq!(
            f.engine.emoji_history().get_preferred_tone(&"👍".into()),
            Some(ferroboard_core::emoji::EmojiSkinTone::Light)
        );
        assert!(
            !f.view
                .pages
                .borrow()
                .contains_key(&Utf16Text::from("Recent"))
        );
        assert_eq!(
            AutomationProperties::get_name(&f.page().children().get(0)).as_deref(),
            Some("👍")
        );
        f.view.on_shown().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        let anchor = button_for(&f.page(), "👍🏻");
        long_press(&f, &anchor, &window);
        let row = popup_row(&f);
        click(&row.children().get(6), &window);
        assert!(
            f.engine
                .emoji_history()
                .pinned()
                .lock()
                .clone()
                .contains(&Some("👍🏻".into()))
        );
        assert!(f.view.tone_popup.borrow().is_none());
        let anchor = button_for(&f.page(), "👍🏻");
        long_press(&f, &anchor, &window);
        let events = Rc::new(Cell::new(0));
        let observed = events.clone();
        let token = f
            .engine
            .emoji_history()
            .changed()
            .add(Rc::new(move |_| observed.set(observed.get() + 1)));
        click(&popup_row(&f).children().get(7), &window);
        assert_eq!(events.get(), 2);
        assert!(
            !f.engine
                .emoji_history()
                .recent()
                .lock()
                .clone()
                .contains(&Some("👍🏻".into()))
        );
        assert!(
            !f.engine
                .emoji_history()
                .pinned()
                .lock()
                .clone()
                .contains(&Some("👍🏻".into()))
        );
        assert!(f.target.text() == Utf16Text::from("👍🏻"));
        f.engine.emoji_history().changed().remove(token);
        window.close();
    });
}
#[test]
fn category_cache_tone_precedence_scroll_reset_and_outside_handled_press() {
    crate::test_support::run(|| {
        use ferroboard_core::emoji::EmojiSkinTone;
        let f = Fixture::new();
        install_scroll_template(&f.view.scroller);
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.view.on_shown().unwrap();
        assert_eq!(
            *f.view.current_tab.borrow(),
            Utf16Text::from("SmileysEmotion")
        );
        let window = show(&f.view, 640.0, 300.0);
        f.view.show_tab("PeopleBody".into()).unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        let cached = f.page();
        assert!(
            cached
                .children()
                .snapshot()
                .iter()
                .any(|c| AutomationProperties::get_name(c).as_deref() == Some("👍"))
        );
        f.engine
            .settings()
            .set_emoji_skin_tone(EmojiSkinTone::Light);
        f.engine.emoji_history().add(&"👍🏿".into()).unwrap();
        f.view.show_tab("PeopleBody".into()).unwrap();
        assert_eq!(cached, f.page());
        assert!(
            cached
                .children()
                .snapshot()
                .iter()
                .any(|c| AutomationProperties::get_name(c).as_deref() == Some("👍"))
        );
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        assert_ne!(cached, f.page());
        let anchor = button_for(&f.page(), "👍🏿");
        f.view.scroller.set_offset(Vector::new(0.0, 120.0));
        Dispatcher::current_dispatcher().run_jobs(None);
        let old = f.view.scroller.offset().y;
        assert!(old > 0.0);
        f.view.show_tab("PeopleBody".into()).unwrap();
        assert_eq!(f.view.scroller.offset().y, old);
        f.view.show_tab("SmileysEmotion".into()).unwrap();
        assert_eq!(f.view.scroller.offset().y, 0.0);
        f.view.show_tab("PeopleBody".into()).unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        // Use an existing cell's source callback path after scrolling back to its visible row.
        let index = f
            .page()
            .children()
            .snapshot()
            .iter()
            .position(|c| c == &anchor.clone().upcast::<Control>())
            .unwrap();
        let columns = (f.view.scroller.viewport().width / f.page().item_width())
            .floor()
            .max(1.0) as usize;
        f.view.scroller.set_offset(Vector::new(
            0.0,
            (index / columns) as f64 * f.page().item_height(),
        ));
        Dispatcher::current_dispatcher().run_jobs(None);
        long_press(&f, &anchor, &window);
        assert!(f.view.tone_popup.borrow().is_some());
        let abc = f.view.bottom_bar.children().get(0);
        let p = point(&abc, &window);
        window.mouse_down(p, MouseButton::Left, RawInputModifiers::empty());
        assert!(f.view.tone_popup.borrow().is_none());
        window.mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
        window.close();
    });
}
#[test]
fn raw_emoticon_search_backspace_and_disabled_history_use_original_engine_paths() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine.settings().set_key_repeat_delay(300);
        f.engine.settings().set_key_repeat_interval(15);
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.view.show_tab("emoticons".into()).unwrap();
        let window = show(&f.view, 640.0, 300.0);
        let first = f.page().children().get(0);
        let label = AutomationProperties::get_name(&first).unwrap();
        click(&first, &window);
        assert_eq!(f.target.text(), label.as_str().into());
        assert!(f.engine.emoji_history().recent().lock().is_empty());
        let searches = Rc::new(Cell::new(0));
        let out = searches.clone();
        let sub = f.view.search_requested(move |_| out.set(out.get() + 1));
        click(&f.view.bottom_bar.children().get(2), &window);
        assert_eq!(searches.get(), 1);
        f.view.remove_search_requested(sub);
        let back = f.view.bottom_bar.children().get(3);
        let p = point(&back, &window);
        let before = f.target.text();
        window.mouse_down(p, MouseButton::Left, RawInputModifiers::empty());
        f.clock.advance(2_999_999);
        assert_eq!(f.target.text(), before);
        f.clock.advance(1);
        assert_ne!(f.target.text(), before);
        let after = f.target.text();
        window.mouse_up(p, MouseButton::Left, RawInputModifiers::empty());
        assert_eq!(f.target.text(), after);
        f.engine.emoji_history().add(&"🙂".into()).unwrap();
        f.view.show_tab("Recent".into()).unwrap();
        f.engine.settings().set_emoji_history_enabled(false);
        let prior = f.engine.emoji_history().recent().lock().clone();
        click(&f.page().children().get(0), &window);
        assert_eq!(f.engine.emoji_history().recent().lock().clone(), prior);
        f.engine.settings().set_emoji_history_enabled(true);
        f.engine.settings().set_incognito_mode(true);
        click(&f.page().children().get(0), &window);
        assert_eq!(f.engine.emoji_history().recent().lock().clone(), prior);
        window.close();
    });
}
#[test]
fn native_drop_removes_history_and_cell_hooks_and_invalid_text_is_reported() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let before = f.engine.emoji_history().changed().handler_count();
        let view = EmojiPanelView::new(f.engine.clone(), f.clock.clone());
        view.set_theme(KeyboardThemes::floris_day()).unwrap();
        view.show_tab("PeopleBody".into()).unwrap();
        let weak = view.downgrade();
        drop(view);
        assert!(weak.upgrade().is_none());
        assert_eq!(f.engine.emoji_history().changed().handler_count(), before);
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.view.on_shown().unwrap();
        let old = f.page();
        f.engine
            .emoji_history()
            .recent()
            .lock()
            .push(Some(Utf16Text::from_units(vec![0xd800])));
        assert!(matches!(
            f.view.show_tab("Recent".into()),
            Err(ViewError::UnpairedSurrogate)
        ));
        assert_eq!(f.page(), old);
        assert_eq!(*f.view.current_tab.borrow(), Utf16Text::from("Recent"));
        assert!(
            !f.view
                .pages
                .borrow()
                .contains_key(&Utf16Text::from("Recent"))
        );
    });
}

// The base Headless app has no application control theme. Use the pinned framework's own
// native scrolling-test composition for hidden bars, not a fake extent/offset implementation.
fn install_scroll_template(scroller: &ScrollViewer) {
    use ferroui_controls::{
        presenters::ScrollContentPresenter,
        templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions},
    };
    scroller.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        |_, scope| {
            let p = ScrollContentPresenter::new();
            p.set_name(Some("PART_ContentPresenter".into()));
            p.register_in_name_scope(&**scope).upcast()
        },
    )));
}

#[test]
fn native_text_target_keeps_focus_during_emoji_click_and_popup_pin() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_controls::TextBox;
        let f = Fixture::new();
        let text = TextBox::new();
        text.set_text(Some("a"));
        text.set_height(36.0);
        let target = TextBoxInputTarget::new(text.clone(), None).unwrap();
        target.select(1, 0);
        f.engine.attach(target.clone()).unwrap();
        f.engine.emoji_history().add(&"🙂".into()).unwrap();
        f.view.set_theme(KeyboardThemes::floris_day()).unwrap();
        f.view.on_shown().unwrap();
        f.view.set_height(300.0);
        let panel = StackPanel::new();
        panel.children().add(text.clone());
        panel.children().add(f.view.clone());
        let window = Window::new();
        window.set_width(640.0);
        window.set_height(336.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(text.focus());
        let anchor = button_for(&f.page(), "🙂");
        click(&anchor, &window);
        assert_eq!(text.text().as_deref(), Some("a🙂"));
        assert_eq!(target.selection_start(), 3);
        assert!(text.is_focused());
        f.view.on_shown().unwrap();
        let anchor = button_for(&f.page(), "🙂");
        long_press(&f, &anchor, &window);
        assert_eq!(popup_row(&f).children().count(), 2);
        assert!(text.is_focused());
        click(&popup_row(&f).children().get(0), &window);
        assert!(
            f.engine
                .emoji_history()
                .pinned()
                .lock()
                .clone()
                .contains(&Some("🙂".into()))
        );
        assert!(text.is_focused());
        assert_eq!(text.text().as_deref(), Some("a🙂"));
        window.close();
        f.engine.detach().unwrap();
        target.detach();
    });
}
