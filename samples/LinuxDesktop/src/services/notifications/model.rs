//! Notification lifetime is independent from the short-lived banner and the history card.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const CAPACITY: usize = 64;
const BANNER_TIME: Duration = Duration::from_secs(6);

pub(crate) struct Request {
    pub app: String,
    pub title: String,
    pub body: String,
    pub actions: Vec<(String, String)>,
    pub timeout_ms: i32,
    pub critical: bool,
    pub transient: bool,
    pub resident: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Notice {
    pub id: u32,
    pub app: String,
    pub title: String,
    pub body: String,
    pub actions: Vec<(String, String)>,
    pub active: bool,
    pub critical: bool,
    resident: bool,
    transient: bool,
    expires: Option<Instant>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Event {
    Closed(u32, u32),
    Action(u32, String),
}

#[derive(Default)]
pub(crate) struct History {
    items: VecDeque<Notice>,
    next_id: u32,
    banner: Option<(u32, Instant)>,
    pub quiet: bool,
}

impl History {
    pub fn items(&self) -> impl Iterator<Item = &Notice> {
        self.items.iter()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn banner(&self) -> Option<&Notice> {
        self.banner
            .and_then(|(id, _)| self.items.iter().find(|item| item.id == id))
    }
    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.items
            .iter()
            .filter_map(|n| n.expires)
            .chain(self.banner.map(|(_, until)| until))
            .min()
    }
    pub fn hide_banner(&mut self) -> bool {
        self.banner.take().is_some()
    }
    pub fn toggle_quiet(&mut self) {
        self.quiet = !self.quiet;
        self.hide_banner();
    }

    pub(super) fn notify(
        &mut self,
        replace: u32,
        req: Request,
        now: Instant,
    ) -> Result<(u32, Vec<Event>), &'static str> {
        let id = if replace == 0 {
            loop {
                self.next_id = self.next_id.wrapping_add(1).max(1);
                if !self.items.iter().any(|item| item.id == self.next_id) {
                    break self.next_id;
                }
            }
        } else {
            let pos = self
                .items
                .iter()
                .position(|n| n.id == replace && n.active)
                .ok_or("The notification to replace is no longer active")?;
            self.items.remove(pos);
            replace
        };
        let expires = match req.timeout_ms {
            0 => None,
            n if n < 0 && req.critical => None,
            n if n < 0 => Some(now + BANNER_TIME),
            n => Some(now + Duration::from_millis(n as u64)),
        };
        let mut events = Vec::new();
        if self.items.len() == CAPACITY {
            // Keep active notifications in preference to inactive history entries.
            let i = self
                .items
                .iter()
                .rposition(|entry| !entry.active)
                .unwrap_or(CAPACITY - 1);
            let item = self.items.remove(i).unwrap();
            if item.active {
                events.push(Event::Closed(item.id, 4));
            }
        }
        if !self.quiet {
            self.banner = Some((
                id,
                expires.map_or(now + BANNER_TIME, |end| end.min(now + BANNER_TIME)),
            ));
        }
        self.items.push_front(Notice {
            id,
            app: req.app,
            title: req.title,
            body: req.body,
            actions: req.actions,
            critical: req.critical,
            active: true,
            transient: req.transient,
            resident: req.resident,
            expires,
        });
        Ok((id, events))
    }

    pub(super) fn expire(&mut self, now: Instant) -> Vec<Event> {
        if self.banner.is_some_and(|(_, until)| until <= now) {
            self.hide_banner();
        }
        let mut events = Vec::new();
        self.items.retain_mut(|item| {
            if item.expires.is_some_and(|until| until <= now) {
                item.expires = None;
                item.active = false;
                item.actions.clear();
                events.push(Event::Closed(item.id, 1));
                !item.transient
            } else {
                true
            }
        });
        events
    }

    pub(super) fn remove(&mut self, id: u32, reason: u32) -> Option<Vec<Event>> {
        let i = self
            .items
            .iter()
            .position(|n| n.id == id && (reason != 3 || n.active))?;
        let item = self.items.remove(i).unwrap();
        if self.banner.is_some_and(|(current, _)| current == id) {
            self.hide_banner();
        }
        Some(if item.active {
            vec![Event::Closed(id, reason)]
        } else {
            Vec::new()
        })
    }

    pub(super) fn invoke(&mut self, id: u32, key: &str) -> Vec<Event> {
        let Some(item) = self.items.iter().find(|n| n.id == id && n.active) else {
            return vec![];
        };
        if !item.actions.iter().any(|(action, _)| action == key) {
            return vec![];
        }
        let mut result = vec![Event::Action(id, key.to_owned())];
        if !item.resident {
            result.extend(self.remove(id, 2).unwrap_or_default());
        }
        result
    }

    pub(super) fn clear(&mut self) -> Vec<Event> {
        self.hide_banner();
        self.items
            .drain(..)
            .filter(|item| item.active)
            .map(|item| Event::Closed(item.id, 2))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Request {
        Request {
            app: "Mail".into(),
            title: "Message".into(),
            body: "Body".into(),
            actions: vec![("open".into(), "Open".into())],
            critical: false,
            resident: false,
            transient: false,
            timeout_ms: 1000,
        }
    }

    #[test]
    fn replacement_resets_the_deadline_and_expiry_invalidates_the_id() {
        let mut h = History::default();
        let t = Instant::now();
        let (id, _) = h.notify(0, request(), t).unwrap();
        assert_eq!(
            h.notify(id, request(), t + Duration::from_millis(500))
                .unwrap()
                .0,
            id
        );
        assert!(h.expire(t + Duration::from_secs(1)).is_empty());
        assert_eq!(h.expire(t + Duration::from_secs(2)), [Event::Closed(id, 1)]);
        assert_eq!(h.len(), 1);
        assert!(h.notify(id, request(), t).is_err());
        assert!(h.invoke(id, "open").is_empty());
        assert!(h.remove(id, 3).is_none());
        assert!(h.clear().is_empty());
    }

    #[test]
    fn sticky_resident_transient_and_quiet_have_distinct_lifetimes() {
        let mut h = History::default();
        let t = Instant::now();
        let (id, _) = h
            .notify(
                0,
                Request {
                    timeout_ms: 0,
                    resident: true,
                    ..request()
                },
                t,
            )
            .unwrap();
        assert_eq!(h.invoke(id, "open"), [Event::Action(id, "open".into())]);
        h.expire(t + BANNER_TIME);
        assert!(h.banner().is_none());
        assert!(h.items().next().unwrap().active);
        h.toggle_quiet();
        let (other, _) = h
            .notify(
                0,
                Request {
                    transient: true,
                    ..request()
                },
                t,
            )
            .unwrap();
        assert!(h.banner().is_none());
        assert_eq!(h.expire(t + BANNER_TIME), [Event::Closed(other, 1)]);
        assert_eq!(h.clear(), [Event::Closed(id, 2)]);
    }

    #[test]
    fn eviction_and_actions_signal_once_in_order() {
        let t = Instant::now();
        let mut h = History::default();
        for _ in 0..CAPACITY {
            h.notify(0, request(), t).unwrap();
        }
        let (id, events) = h.notify(0, request(), t).unwrap();
        assert_eq!(events, [Event::Closed(1, 4)]);
        assert_eq!(h.len(), CAPACITY);
        assert_eq!(
            h.invoke(id, "open"),
            [Event::Action(id, "open".into()), Event::Closed(id, 2)]
        );
        assert!(h.remove(id, 3).is_none());
    }
}
