use super::*;
use crate::{test_scheduler::ManualScheduler, themes::KeyboardThemes};
use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{
    engine::{KeyboardEngine, LanguageModels},
    input::KeyboardTouchProcessor,
    layouts::KeyboardResources,
    settings::KeyboardSettings,
};
use std::rc::Rc;
use std::time::Duration;
fn engine() -> Rc<KeyboardEngine> {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap()
}
#[test]
fn independently_created_views_drop_while_engine_and_touch_are_retained() {
    crate::test_support::run(|| {
        let e = engine();
        let clock = ManualScheduler::new();
        let touch = KeyboardTouchProcessor::new(e.clone(), clock.clone());
        let keys = KeysView::new(e.clone(), touch.clone(), clock.clone());
        let weak = keys.downgrade();
        drop(keys);
        assert!(
            weak.upgrade().is_none(),
            "touch events must not retain KeysView"
        );
        assert!(!touch.key_pressed().has_handlers());
        assert!(!touch.glide_ended().has_handlers());
        let emoji = EmojiPanelView::new(e.clone(), clock.clone());
        emoji.set_theme(KeyboardThemes::floris_day()).unwrap();
        emoji.on_shown().unwrap();
        let weak = emoji.downgrade();
        drop(emoji);
        assert!(
            weak.upgrade().is_none(),
            "emoji history/buttons must not retain panel"
        );
        assert_eq!(e.emoji_history().changed().handler_count(), 0);
        let clipboard = ClipboardPanelView::new(e.clone(), clock.clone());
        clipboard.set_theme(KeyboardThemes::floris_day()).unwrap();
        let weak = clipboard.downgrade();
        drop(clipboard);
        assert!(
            weak.upgrade().is_none(),
            "clipboard history/buttons must not retain panel"
        );
        assert_eq!(
            e.clipboard_history().changed().handler_count(),
            1,
            "only engine's own history hook remains"
        );
        let editing = EditingPanelView::new(e.clone(), clock.clone());
        editing.set_theme(KeyboardThemes::floris_day()).unwrap();
        let weak = editing.downgrade();
        drop(editing);
        assert!(
            weak.upgrade().is_none(),
            "state/buttons must not retain editing panel"
        );
        assert_eq!(e.state_changed().handler_count(), 0);
        let picker = SubtypePickerView::new(e.clone(), clock.clone());
        picker.open().unwrap();
        let weak = picker.downgrade();
        drop(picker);
        assert!(weak.upgrade().is_none());
        let smartbar = SmartbarView::new(e.clone(), clock);
        smartbar
            .set_theme(KeyboardThemes::floris_day(), 44.0)
            .unwrap();
        let weak = smartbar.downgrade();
        drop(smartbar);
        assert!(weak.upgrade().is_none());
        e.dispose().unwrap();
    });
}

use ferroboard_core::input::IKeyboardScheduler;
use ferroui_base::{
    Point, Ref,
    input::{MouseButton, RawInputModifiers},
    threading::Dispatcher,
};
use ferroui_controls::{Border, Control, ScrollViewer, StackPanel, Window};
use ferroui_headless::HeadlessWindowExtensions;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
#[derive(Default)]
struct SavedScheduler {
    actions: RefCell<Vec<Rc<dyn Fn()>>>,
    canceled: Rc<Cell<usize>>,
}
impl IKeyboardScheduler for SavedScheduler {
    fn schedule(&self, _: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        self.actions.borrow_mut().push(action);
        let count = self.canceled.clone();
        ScheduledAction::new(move || count.set(count.get() + 1))
    }
}
#[test]
fn retained_canceled_fade_callback_cannot_keep_or_restart_dropped_view() {
    crate::test_support::run(|| {
        let e = engine();
        let scheduler = Rc::new(SavedScheduler::default());
        let touch = KeyboardTouchProcessor::new(e.clone(), ManualScheduler::new());
        let view = KeysView::new(e.clone(), touch.clone(), scheduler.clone());
        touch.glide_ended().raise(touch.as_ref());
        assert_eq!(scheduler.actions.borrow().len(), 1);
        let callback = scheduler.actions.borrow()[0].clone();
        let weak = view.downgrade();
        drop(view);
        assert!(weak.upgrade().is_none());
        assert_eq!(scheduler.canceled.get(), 1);
        callback();
        assert_eq!(
            scheduler.actions.borrow().len(),
            1,
            "stale callback must not schedule more work"
        );
        assert!(!touch.key_pressed().has_handlers());
        e.dispose().unwrap();
    });
}
#[test]
fn retained_picker_button_does_not_keep_view_or_invoke_expired_manage_handler() {
    crate::test_support::run(|| {
        let e = engine();
        let view = SubtypePickerView::new(e.clone(), ManualScheduler::new());
        view.open().unwrap();
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        let _handler = view.manage_requested(move |_| seen.set(seen.get() + 1));
        let card = view.children().get(0).cast::<Border>().unwrap();
        let scroller = card.child().unwrap().cast::<ScrollViewer>().unwrap();
        let list = Control::from_boxed(scroller.content().as_ref().unwrap())
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        let button = list.children().get(list.children().count() - 1);
        drop(list);
        drop(scroller);
        drop(card);
        let weak = view.downgrade();
        drop(view);
        assert!(weak.upgrade().is_none());
        let window = Window::new();
        window.set_width(300.0);
        window.set_height(100.0);
        window.set_content(Some(Control::boxed(button.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let p = button
            .translate_point(
                Point::new(button.bounds().width / 2.0, button.bounds().height / 2.0),
                &window,
            )
            .unwrap();
        window.mouse_down(p, MouseButton::Left, RawInputModifiers::NONE);
        window.mouse_up(p, MouseButton::Left, RawInputModifiers::NONE);
        assert_eq!(calls.get(), 0);
        window.close();
        e.dispose().unwrap();
    });
}
#[test]
fn earlier_history_observer_can_release_view_during_notification() {
    crate::test_support::run(|| {
        let e = engine();
        let history = e.clipboard_history();
        let slot = Rc::new(RefCell::new(None::<Ref<ClipboardPanelView>>));
        let owner = Rc::downgrade(&slot);
        let token = history.changed().add(Rc::new(move |_| {
            owner.upgrade().unwrap().borrow_mut().take();
        }));
        let view = ClipboardPanelView::new(e.clone(), ManualScheduler::new());
        view.set_theme(KeyboardThemes::floris_day()).unwrap();
        let weak = view.downgrade();
        *slot.borrow_mut() = Some(view);
        history.add(Some(&"changed".into())).unwrap();
        assert!(weak.upgrade().is_none());
        assert_eq!(history.changed().handler_count(), 2);
        history.changed().remove(token);
        e.dispose().unwrap();
    });
}
