//! The badge of the application over a test double of the platform's service: nothing reaches
//! the Dock, the taskbar or the home screen of the system.

use super::*;
use crate::FerroLocator;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A badge service that records what it is asked to show.
#[derive(Default)]
struct RecordingBadge {
    supported: Cell<bool>,
    calls: RefCell<Vec<String>>,
}

impl IBadgeNotificationManagerImpl for RecordingBadge {
    fn is_supported(&self) -> bool {
        self.supported.get()
    }

    fn set_count(&self, count: u32) {
        self.calls.borrow_mut().push(format!("count {count}"));
    }

    fn set_glyph(&self, glyph: BadgeNotificationGlyph) {
        self.calls.borrow_mut().push(format!("glyph {}", glyph.name()));
    }

    fn clear(&self) {
        self.calls.borrow_mut().push("clear".into());
    }
}

fn with_service(supported: bool, test: impl FnOnce(&RecordingBadge)) {
    let scope = FerroLocator::enter_scope();
    let service = Rc::new(RecordingBadge::default());
    service.supported.set(supported);
    register_badge_notification_manager_impl(service.clone());
    test(&service);
    scope.dispose();
}

#[test]
fn without_a_service_the_badge_is_not_supported() {
    let scope = FerroLocator::enter_scope();
    assert!(badge_notification_manager_impl().is_none());
    assert!(!BadgeNotificationManager::is_supported());
    let badge = BadgeNotificationManager::current();
    badge.set_badge_as_count(3);
    badge.set_badge_as_glyph(BadgeNotificationGlyph::Alert);
    badge.clear_badge();
    scope.dispose();
}

#[test]
fn a_service_that_cannot_show_the_badge_is_not_used() {
    with_service(false, |service| {
        let badge = BadgeNotificationManager::current();
        assert!(!BadgeNotificationManager::is_supported());
        badge.set_badge_as_count(3);
        badge.set_badge_as_glyph(BadgeNotificationGlyph::Busy);
        badge.clear_badge();
        assert!(service.calls.borrow().is_empty());
    });
}

#[test]
fn counts_and_glyphs_reach_the_service_and_zero_and_none_clear_the_badge() {
    with_service(true, |service| {
        let badge = BadgeNotificationManager::current();
        assert!(BadgeNotificationManager::is_supported());
        badge.set_badge_as_count(5);
        badge.set_badge_as_count(0);
        badge.set_badge_as_glyph(BadgeNotificationGlyph::NewMessage);
        badge.set_badge_as_glyph(BadgeNotificationGlyph::None);
        badge.clear_badge();
        assert_eq!(
            *service.calls.borrow(),
            ["count 5", "clear", "glyph NewMessage", "clear", "clear"]
        );
    });
}

#[test]
fn the_glyphs_are_in_the_order_of_their_values_with_the_names_of_the_badge_xml() {
    assert_eq!(BadgeNotificationGlyph::VALUES.len(), 13);
    assert_eq!(BadgeNotificationGlyph::VALUES[0], BadgeNotificationGlyph::default());
    assert_eq!(BadgeNotificationGlyph::VALUES[1], BadgeNotificationGlyph::Activity);
    assert_eq!(BadgeNotificationGlyph::VALUES[12], BadgeNotificationGlyph::Unavailable);
    for glyph in BadgeNotificationGlyph::VALUES {
        let name = glyph.name();
        // The value of the XML is the name with a small first letter.
        let mut value = name[..1].to_ascii_lowercase();
        value.push_str(&name[1..]);
        assert_eq!(glyph.badge_value(), value);
    }
}
