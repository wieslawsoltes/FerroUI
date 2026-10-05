use super::{
    IManagedNotificationManager, INotification, INotificationManager, Notification, NotificationCard,
    NotificationOverrides, NotificationPosition, NotificationType, ReversibleStackPanel, WindowNotificationManager,
};
use crate::primitives::{AdornerLayer, VisualLayerManager};
use crate::test_support::{boxed_str, test_scope};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, Control, Window};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn text(value: &str) -> BoxedValue {
    boxed_str(value).expect("a boxed string")
}

/// A notification with the given message.
fn notification(message: &str) -> Rc<Notification> {
    let notification = Notification::empty();
    notification.set_message(Some(message.to_string()));
    notification
}

mod window_notification_manager_tests {
    use super::*;

    #[test]
    fn show_notifications_with_same_string() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();

        manager.show_content(text("Notification text"));
        manager.show_content(text("Notification text"));
        manager.show_content(text("Notification text"));

        assert_eq!(manager.notifications().len(), 3);
    }

    #[test]
    fn show_and_close_notification() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();

        manager.show_content(text("Notification text"));

        assert_eq!(manager.notifications().len(), 1);

        manager.close_content(&text("Notification text"));

        assert!(!manager.notifications().iter().any(|x| !x.is_closing()));
    }

    #[test]
    fn show_and_close_all_notifications() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();

        manager.show_content(text("Notification 1"));
        manager.show_content(text("Notification 2"));

        assert_eq!(manager.notifications().len(), 2);

        manager.close_all();

        assert!(!manager.notifications().iter().any(|x| !x.is_closing()));
    }
}

mod i_notification_manager_tests {
    use super::*;

    #[test]
    fn show_notifications_with_same_content() {
        let _scope = test_scope();
        let target = WindowNotificationManager::new();
        let manager: Rc<dyn INotificationManager> = target.as_notification_manager();

        let notification = notification("Notification text");

        manager.show(notification.clone());
        manager.show(notification.clone());
        manager.show(notification);

        assert_eq!(target.notifications().len(), 3);
    }

    #[test]
    fn show_and_close_notification() {
        let _scope = test_scope();
        let target = WindowNotificationManager::new();
        let manager: Rc<dyn INotificationManager> = target.as_notification_manager();

        let notification: Rc<dyn INotification> = notification("Notification text");

        manager.show(notification.clone());

        assert_eq!(target.notifications().len(), 1);

        manager.close(&notification);

        assert!(!target.notifications().iter().any(|x| !x.is_closing()));
    }

    #[test]
    fn show_and_close_all_notifications() {
        let _scope = test_scope();
        let target = WindowNotificationManager::new();
        let manager: Rc<dyn INotificationManager> = target.as_notification_manager();

        let notification1 = notification("Notification text");
        let notification2 = notification("Notification text");

        manager.show(notification1);
        manager.show(notification2);

        assert_eq!(target.notifications().len(), 2);

        manager.close_all();

        assert!(!target.notifications().iter().any(|x| !x.is_closing()));
    }
}

mod notification_card_tests {
    use super::*;

    #[test]
    fn should_update_pseudoclasses_when_notification_type_changes() {
        fn assert_pseudo_classes(target: &NotificationCard, expected: &str) {
            assert!(target.classes().contains(expected));

            for pseudoclass in [":error", ":information", ":success", ":warning"] {
                if pseudoclass != expected {
                    assert!(!target.classes().contains(pseudoclass));
                }
            }
        }

        let _scope = test_scope();
        let target = NotificationCard::new();

        target.set_notification_type(NotificationType::Error);
        assert_pseudo_classes(&target, ":error");

        target.set_notification_type(NotificationType::Information);
        assert_pseudo_classes(&target, ":information");

        target.set_notification_type(NotificationType::Success);
        assert_pseudo_classes(&target, ":success");

        target.set_notification_type(NotificationType::Warning);
        assert_pseudo_classes(&target, ":warning");
    }
}

/// Additional tests (not ports of upstream tests).
mod additional_tests {
    use super::*;

    #[test]
    fn closing_only_closes_the_cards_with_equal_content() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();
        let first = notification("Notification text");
        let second = notification("Notification text");

        manager.show(first.clone());
        manager.show(second);
        manager.show_content(text("a"));
        manager.show_content(text("b"));

        // Notifications compare by identity, text by value.
        let first: Rc<dyn INotification> = first;
        manager.close(&first);
        manager.close_content(&text("b"));

        let closing: Vec<bool> = manager.notifications().iter().map(|card| card.is_closing()).collect();
        assert_eq!(closing, [true, false, false, true]);
    }

    #[test]
    fn a_notification_is_recognised_in_untyped_content() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();
        let managed: Rc<dyn IManagedNotificationManager> = manager.as_managed_notification_manager();
        let notification = Notification::new(
            Some("Title".to_string()),
            Some("Message".to_string()),
            NotificationType::Warning,
            Some(Duration::ZERO),
            None,
            None,
        );
        let content: BoxedValue = notification.clone();

        managed.show_content(content.clone());

        let cards = manager.notifications();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].notification_type(), NotificationType::Warning);
        assert!(cards[0].classes().contains(":warning"));
        assert!(cards[0].content().is_some_and(|card_content| *card_content == *content));
        // A zero expiration starts no timer.
        assert!(Dispatcher::timers_for_unit_tests().is_empty());

        managed.close_content(&content);
        assert!(cards[0].is_closing());
    }

    #[test]
    fn the_expiration_timer_closes_the_card() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();

        manager.show_content(text("default"));
        manager.show(Notification::empty().with_expiration(Duration::from_secs(2)));

        let timers = Dispatcher::timers_for_unit_tests();
        assert_eq!(timers.len(), 2);
        assert_eq!(timers[0].interval(), Duration::from_secs(5));
        assert_eq!(timers[1].interval(), Duration::from_secs(2));

        let cards = manager.notifications();
        Dispatcher::force_fire_timer_for_unit_tests(&timers[1]);
        assert!(!cards[0].is_closing());
        assert!(cards[1].is_closing());
        // The timer runs once.
        assert_eq!(Dispatcher::timers_for_unit_tests().len(), 1);
    }

    #[test]
    fn showing_more_than_the_maximum_closes_the_first_open_card() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();
        manager.set_max_items(2);

        manager.show_content(text("1"));
        manager.show_content(text("2"));
        assert!(!manager.notifications().iter().any(|card| card.is_closing()));

        manager.show_content(text("3"));
        manager.show_content(text("4"));

        let closing: Vec<bool> = manager.notifications().iter().map(|card| card.is_closing()).collect();
        assert_eq!(closing, [true, true, false, false]);
    }

    #[test]
    fn a_closed_card_is_removed_and_its_close_action_runs() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();
        let closed = Rc::new(Cell::new(0));
        let on_close: Rc<dyn Fn()> = {
            let closed = closed.clone();
            Rc::new(move || closed.set(closed.get() + 1))
        };
        manager.show(Notification::new(None, None, NotificationType::Success, None, None, Some(on_close)));
        manager.show_content(text("other"));

        let card = manager.notifications()[0].clone();
        let raised = Rc::new(Cell::new(0));
        card.notification_closed({
            let raised = raised.clone();
            move |_, _| raised.set(raised.get() + 1)
        });

        card.set_is_closed(true);
        assert_eq!((raised.get(), closed.get()), (1, 1));
        assert_eq!(manager.notifications().len(), 1);

        // Resetting `IsClosed` on a card that is not closing raises nothing.
        let other = manager.notifications()[0].clone();
        let other_raised = Rc::new(Cell::new(0));
        other.notification_closed({
            let other_raised = other_raised.clone();
            move |_, _| other_raised.set(other_raised.get() + 1)
        });
        other.set_is_closed(true);
        other.set_is_closed(false);
        assert_eq!(other_raised.get(), 1);
        assert!(manager.notifications().is_empty());

        // On a closing card it raises again.
        other.close();
        other.set_is_closed(true);
        other.set_is_closed(false);
        assert_eq!(other_raised.get(), 3);
    }

    #[test]
    fn close_on_click_closes_the_card_the_button_is_in() {
        let _scope = test_scope();
        let card = NotificationCard::new();
        let button = Button::new();
        card.set_content(Some(Control::boxed(button.clone())));
        assert!(!NotificationCard::get_close_on_click(&button));

        // Without the property a click does nothing.
        button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
        assert!(!card.is_closing());

        NotificationCard::set_close_on_click(&button, true);
        NotificationCard::set_close_on_click(&button, false);
        button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
        assert!(!card.is_closing());

        NotificationCard::set_close_on_click(&button, true);
        let is_closing_changes = Rc::new(Cell::new(0));
        let changes = is_closing_changes.clone();
        card.property_changed(move |e| {
            if e.property() == NotificationCard::is_closing_property().as_property() {
                changes.set(changes.get() + 1);
            }
        });
        button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
        assert!(card.is_closing());

        // Closing again changes nothing.
        card.close();
        assert_eq!(is_closing_changes.get(), 1);
    }

    #[test]
    fn position_updates_the_pseudo_classes() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();
        let all = [":topleft", ":topright", ":bottomleft", ":bottomright", ":topcenter", ":bottomcenter"];
        let active = |manager: &WindowNotificationManager| -> Vec<&str> {
            all.iter().copied().filter(|class| manager.classes().contains(class)).collect()
        };

        assert_eq!(active(&manager), [":topright"]);

        for (position, class) in [
            (NotificationPosition::TopLeft, ":topleft"),
            (NotificationPosition::BottomLeft, ":bottomleft"),
            (NotificationPosition::BottomRight, ":bottomright"),
            (NotificationPosition::TopCenter, ":topcenter"),
            (NotificationPosition::BottomCenter, ":bottomcenter"),
            (NotificationPosition::TopRight, ":topright"),
        ] {
            manager.set_position(position);
            assert_eq!(active(&manager), [class]);
        }
    }

    #[test]
    fn notification_raises_property_changed_for_title_and_message() {
        struct Recorder(RefCell<Vec<String>>);

        impl NotificationOverrides for Recorder {
            fn on_property_changed(&self, notification: &Notification, property_name: Option<&str>) {
                self.0.borrow_mut().push(format!("override:{}", property_name.unwrap_or_default()));
                notification.base_on_property_changed(property_name);
            }
        }

        let recorder = Rc::new(Recorder(RefCell::new(Vec::new())));
        let target = Notification::with_overrides(
            Some("title".to_string()),
            None,
            NotificationType::Information,
            None,
            None,
            None,
            recorder.clone(),
        );
        // The constructor assigns through the setters.
        assert_eq!(*recorder.0.borrow(), ["override:Title"]);
        assert_eq!(target.expiration(), Duration::from_secs(5));

        let raised = Rc::new(RefCell::new(Vec::new()));
        let sink = raised.clone();
        target.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        target.set_title(Some("title".to_string()));
        target.set_message(Some("message".to_string()));
        target.set_title(None);
        target.set_type(NotificationType::Error);

        assert_eq!(*raised.borrow(), ["Message", "Title"]);
        assert_eq!(*recorder.0.borrow(), ["override:Title", "override:Message", "override:Title"]);
    }

    /// The global clock of a test: pulses every subscriber.
    struct MockGlobalClock {
        subject: LightweightSubject<TimeSpan>,
        play_state: Cell<PlayState>,
    }

    impl MockGlobalClock {
        fn pulse(&self, time: TimeSpan) {
            self.subject.on_next(time);
        }
    }

    impl IObservable<TimeSpan> for MockGlobalClock {
        fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
            self.subject.subscribe(observer)
        }
    }

    impl IClock for MockGlobalClock {
        fn play_state(&self) -> PlayState {
            self.play_state.get()
        }

        fn set_play_state(&self, value: PlayState) {
            self.play_state.set(value)
        }
    }

    impl IGlobalClock for MockGlobalClock {}

    /// Shows a window of the test theme and returns its adorner layer.
    fn show_window() -> (Ref<Window>, Ref<AdornerLayer>) {
        let window = Window::new();
        window.show();
        window.layout_manager().execute_initial_layout_pass();
        let adorner_layer = window
            .find_descendant_of_type::<VisualLayerManager>(false)
            .and_then(|manager| manager.adorner_layer())
            .expect("the window template has an adorner layer");
        (window, adorner_layer)
    }

    #[test]
    fn the_manager_installs_itself_in_the_adorner_layer_of_its_host() {
        let clock = Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
        let global_clock: Rc<dyn IGlobalClock> = clock.clone();
        let _app = UnitTestApplication::start(TestServices::styled_window().with_global_clock(global_clock));
        let (window, adorner_layer) = show_window();

        let manager = WindowNotificationManager::with_host(Some(&window.clone().upcast()));

        assert!(adorner_layer.children().snapshot().iter().any(|child| child.ptr_eq(&manager)));
        assert!(AdornerLayer::get_adorned_element(&manager).is_some_and(|adorned| adorned.ptr_eq(&adorner_layer)));

        window.layout_manager().execute_layout_pass();

        // The template of the theme is applied: the cards are the children
        // of its items panel.
        manager.show_content(text("Notification text"));
        manager.show(notification("Notification text"));
        let items = manager
            .find_descendant_of_type::<ReversibleStackPanel>(false)
            .expect("the template of the manager has an items panel");
        assert_eq!(items.name().as_deref(), Some("PART_Items"));
        assert_eq!(items.children().count(), 2);
        assert_eq!(manager.notifications().len(), 2);

        // The theme closes a closing card when its animation ends; a closed
        // card leaves the panel.
        let card = manager.notifications()[0].clone();
        card.close();
        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(1.0));
        assert!(!card.is_closed());
        assert_eq!(items.children().count(), 2);
        clock.pulse(TimeSpan::from_seconds(1.25));
        assert!(card.is_closed());
        assert_eq!(items.children().count(), 1);

        // Detaching the manager drops its cards.
        adorner_layer.children().remove(manager.clone());
        assert_eq!(items.children().count(), 0);

        window.close();
    }

    #[test]
    fn the_manager_is_installed_when_the_template_of_its_host_is_applied() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let window = Window::new();

        // The window has no template yet: nothing to install into.
        let manager = WindowNotificationManager::with_host(Some(&window.clone().upcast()));
        assert!(manager.parent().is_none());

        window.show();
        window.layout_manager().execute_initial_layout_pass();

        let adorner_layer = window
            .find_descendant_of_type::<VisualLayerManager>(false)
            .and_then(|manager| manager.adorner_layer())
            .expect("the window template has an adorner layer");
        assert!(adorner_layer.children().snapshot().iter().any(|child| child.ptr_eq(&manager)));

        window.close();
    }

    /// Additional test (not a port): the reinstall path of the template
    /// applied handler when the host gets another template.
    #[test]
    fn the_manager_is_reinstalled_when_another_template_of_its_host_is_applied() {
        fn layer_of(window: &Ref<Window>) -> Ref<AdornerLayer> {
            window
                .find_descendant_of_type::<VisualLayerManager>(false)
                .and_then(|manager| manager.adorner_layer())
                .expect("the window template has an adorner layer")
        }

        fn occurrences(layer: &AdornerLayer, manager: &Ref<WindowNotificationManager>) -> usize {
            layer.children().snapshot().iter().filter(|child| child.ptr_eq(manager)).count()
        }

        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (window, first_layer) = show_window();

        let manager = WindowNotificationManager::with_host(Some(&window.clone().upcast()));
        window.layout_manager().execute_layout_pass();
        assert_eq!(occurrences(&first_layer, &manager), 1);

        manager.show_content(text("Notification text"));
        assert_eq!(manager.notifications().len(), 1);

        // Another template instance: the template is applied again.
        window.set_template(Some(crate::testing::top_level_template()));
        window.apply_template();

        let second_layer = layer_of(&window);
        assert!(!second_layer.ptr_eq(&first_layer));
        // Removed from the layer of the previous template ...
        assert_eq!(occurrences(&first_layer, &manager), 0);
        // ... and installed, once, in the layer of the new one.
        assert_eq!(occurrences(&second_layer, &manager), 1);
        assert!(manager.parent().is_some_and(|parent| parent.ptr_eq(&second_layer)));
        assert!(AdornerLayer::get_adorned_element(&manager).is_some_and(|adorned| adorned.ptr_eq(&second_layer)));
        // Leaving the visual tree with the previous template dropped the cards.
        assert!(manager.notifications().is_empty());

        // The handler was subscribed again, exactly once: a third template
        // moves the manager once more.
        window.set_template(Some(crate::testing::top_level_template()));
        window.apply_template();

        let third_layer = layer_of(&window);
        assert!(!third_layer.ptr_eq(&second_layer));
        assert_eq!(occurrences(&second_layer, &manager), 0);
        assert_eq!(occurrences(&third_layer, &manager), 1);
        assert!(AdornerLayer::get_adorned_element(&manager).is_some_and(|adorned| adorned.ptr_eq(&third_layer)));

        // The manager still works in its new place.
        window.layout_manager().execute_layout_pass();
        manager.show_content(text("Notification text"));
        assert_eq!(manager.notifications().len(), 1);

        window.close();
    }

    /// Additional test (not a port): a notification is found by `close`
    /// whether it was shown as the object or in a box of its contract handle,
    /// and whichever of the two forms is closed.
    #[test]
    fn closing_finds_a_notification_in_either_untyped_form() {
        let _scope = test_scope();
        let manager = WindowNotificationManager::new();
        let first = Notification::new(None, Some("first".to_string()), NotificationType::Warning, None, None, None);
        let second = notification("second");
        let other = notification("other");
        let first_contract: Rc<dyn INotification> = first.clone();
        let second_contract: Rc<dyn INotification> = second.clone();
        let first_in_a_box: BoxedValue = Rc::new(first_contract.clone());
        let second_in_a_box: BoxedValue = Rc::new(second_contract.clone());

        // Shown in a box of the contract handle: still a notification.
        manager.show_content(first_in_a_box);
        manager.show(second);
        manager.show(other);
        let cards = manager.notifications();
        assert_eq!(cards[0].notification_type(), NotificationType::Warning);

        // Shown boxed, closed as the object.
        manager.close(&first_contract);
        let closing: Vec<bool> = cards.iter().map(|card| card.is_closing()).collect();
        assert_eq!(closing, [true, false, false]);

        // Shown as the object, closed boxed.
        manager.close_content(&second_in_a_box);
        let closing: Vec<bool> = cards.iter().map(|card| card.is_closing()).collect();
        assert_eq!(closing, [true, true, false]);
    }
}
