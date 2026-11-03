//! Clipboard/ClipboardHistory.cs. The history is single-threaded; no new scheduling is introduced.
use super::timestamp::clipboard_now;
use super::{
    OffsetDateTime, TimestampError, format_clipboard_timestamp, parse_clipboard_timestamp,
    parse_clipboard_timestamp_in_zone,
};
use crate::{
    serialization::{JsonBridgeError, StrictJsonValue, write_json_string},
    text::Utf16Text,
};
use ferroui_system::events::Event;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    time::Duration,
};
pub use uuid::Uuid as ClipboardItemId;
// Static concrete type identity supports the source owner's `is InMemoryClipboardService`
// check. This exposes no runtime member lookup/reflection or generated proxy dispatch.
/// Native, single-threaded clipboard operation. Dropping a pending future cancels its waiter.
pub type ClipboardServiceError = std::sync::Arc<dyn std::error::Error + Send + Sync>;
pub type ClipboardFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, ClipboardServiceError>> + 'static>>;
/// Runtime injection for pending keyboard-command work; Core does not depend on a UI executor.
pub type ClipboardJob = Pin<Box<dyn Future<Output = ()> + 'static>>;
pub type ClipboardSpawner = Rc<dyn Fn(ClipboardJob)>;
pub trait IClipboardService: std::any::Any {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>>;
    fn set_text_async(&self, text: Option<Utf16Text>) -> ClipboardFuture<()>;
}
#[derive(Default)]
pub struct InMemoryClipboardService {
    text: RefCell<Option<Utf16Text>>,
}
impl IClipboardService for InMemoryClipboardService {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        Box::pin(std::future::ready(Ok(self.text.borrow().clone())))
    }
    fn set_text_async(&self, text: Option<Utf16Text>) -> ClipboardFuture<()> {
        *self.text.borrow_mut() = text;
        Box::pin(std::future::ready(Ok(())))
    }
}
pub struct ClipboardItem {
    pub id: ClipboardItemId,
    pub text: Utf16Text,
    timestamp: Cell<OffsetDateTime>,
    is_pinned: Cell<bool>,
}
impl ClipboardItem {
    pub fn new(text: Utf16Text) -> Self {
        Self {
            id: ClipboardItemId::new_v4(),
            text,
            timestamp: Cell::new(clipboard_now()),
            is_pinned: Cell::new(false),
        }
    }
    pub fn timestamp(&self) -> OffsetDateTime {
        self.timestamp.get()
    }
    pub fn set_timestamp(&self, value: OffsetDateTime) {
        self.timestamp.set(value);
    }
    pub fn is_pinned(&self) -> bool {
        self.is_pinned.get()
    }
    pub fn set_is_pinned(&self, value: bool) {
        self.is_pinned.set(value);
    }
}
pub type ClipboardClock = Rc<dyn Fn() -> OffsetDateTime>;
#[derive(Debug)]
pub enum ClipboardError {
    LimitIndex,
    Json(JsonBridgeError),
    MissingProperty,
    Timestamp(TimestampError),
}
impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LimitIndex => f.write_str("Clipboard history limit must not be negative"),
            Self::MissingProperty => f.write_str("Clipboard history entry is missing its text"),
            Self::Json(error) => write!(f, "Invalid clipboard history: {error}"),
            Self::Timestamp(error) => write!(f, "Invalid clipboard history timestamp: {error}"),
        }
    }
}
impl std::error::Error for ClipboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Timestamp(error) => Some(error),
            _ => None,
        }
    }
}
impl From<JsonBridgeError> for ClipboardError {
    fn from(e: JsonBridgeError) -> Self {
        Self::Json(e)
    }
}
pub struct ClipboardHistory {
    items: RefCell<Vec<Rc<ClipboardItem>>>,
    changed: Event<Self>,
    max_items: Cell<i32>,
    expire_after: Cell<Option<Duration>>,
    clock: RefCell<ClipboardClock>,
}
impl Default for ClipboardHistory {
    fn default() -> Self {
        Self {
            items: RefCell::new(Vec::new()),
            changed: Event::new(),
            max_items: Cell::new(30),
            expire_after: Cell::new(Some(Duration::from_secs(3600))),
            clock: RefCell::new(Rc::new(clipboard_now)),
        }
    }
}
impl ClipboardHistory {
    pub fn changed(&self) -> &Event<Self> {
        &self.changed
    }
    pub fn max_items(&self) -> i32 {
        self.max_items.get()
    }
    pub fn set_max_items(&self, v: i32) {
        self.max_items.set(v);
    }
    pub fn expire_after(&self) -> Option<Duration> {
        self.expire_after.get()
    }
    pub fn set_expire_after(&self, v: Option<Duration>) {
        self.expire_after.set(v);
    }
    pub fn clock(&self) -> ClipboardClock {
        self.clock.borrow().clone()
    }
    pub fn set_clock(&self, v: ClipboardClock) {
        let previous = self.clock.replace(v);
        drop(previous);
    }
    fn now(&self) -> OffsetDateTime {
        let clock = self.clock();
        clock()
    }
    pub fn items(&self) -> Result<Vec<Rc<ClipboardItem>>, ClipboardError> {
        self.prune()?;
        let items = self.items.borrow();
        Ok(items
            .iter()
            .filter(|i| i.is_pinned())
            .chain(items.iter().filter(|i| !i.is_pinned()))
            .cloned()
            .collect())
    }
    pub fn primary(&self) -> Option<Rc<ClipboardItem>> {
        let items = self.items.borrow();
        items
            .iter()
            .find(|i| !i.is_pinned())
            .or_else(|| items.first())
            .cloned()
    }
    pub fn add(
        &self,
        text: Option<&Utf16Text>,
    ) -> Result<Option<Rc<ClipboardItem>>, ClipboardError> {
        let Some(text) = text.filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let existing = self
            .items
            .borrow()
            .iter()
            .find(|i| &i.text == text)
            .cloned();
        let item = if let Some(existing) = existing {
            self.items
                .borrow_mut()
                .retain(|i| !Rc::ptr_eq(i, &existing));
            existing.set_timestamp(self.now());
            existing
        } else {
            let item = Rc::new(ClipboardItem::new(text.clone()));
            item.set_timestamp(self.now());
            item
        };
        self.items.borrow_mut().insert(0, item.clone());
        self.prune()?;
        self.changed.raise(self);
        Ok(Some(item))
    }
    pub fn set_pinned(&self, item: &ClipboardItem, pinned: bool) {
        item.set_is_pinned(pinned);
        self.changed.raise(self);
    }
    pub fn remove(&self, item: &Rc<ClipboardItem>) {
        let mut items = self.items.borrow_mut();
        let n = items.len();
        items.retain(|i| !Rc::ptr_eq(i, item));
        let removed = n != items.len();
        drop(items);
        if removed {
            self.changed.raise(self);
        }
    }
    pub fn clear(&self, include_pinned: bool) {
        self.items
            .borrow_mut()
            .retain(|i| !include_pinned && i.is_pinned());
        self.changed.raise(self);
    }
    fn prune(&self) -> Result<(), ClipboardError> {
        let now = self.now();
        if let Some(expiry) = self.expire_after() {
            self.items.borrow_mut().retain(|i| {
                i.is_pinned()
                    || now.unix_timestamp_nanos() - i.timestamp().unix_timestamp_nanos()
                        <= expiry.as_nanos() as i128
            });
        }
        let unpinned: Vec<_> = self
            .items
            .borrow()
            .iter()
            .filter(|i| !i.is_pinned())
            .cloned()
            .collect();
        let max = self.max_items();
        if max < 0 {
            return Err(ClipboardError::LimitIndex);
        }
        for item in unpinned.iter().skip(max as usize) {
            self.items.borrow_mut().retain(|i| !Rc::ptr_eq(i, item));
        }
        Ok(())
    }
    pub fn export(&self, pinned_only: bool) -> Result<String, ClipboardError> {
        let items = self.items.borrow();
        let mut output = String::from("[");
        let mut first = true;
        for item in items.iter().filter(|i| !pinned_only || i.is_pinned()) {
            if !first {
                output.push(',');
            }
            first = false;
            output.push_str("{\"text\":");
            write_json_string(&mut output, &item.text);
            output.push_str(",\"timestamp\":");
            output.push('"');
            output.push_str(
                &format_clipboard_timestamp(item.timestamp()).map_err(ClipboardError::Timestamp)?,
            );
            output.push('"');
            output.push_str(",\"pinned\":");
            output.push_str(if item.is_pinned() { "true" } else { "false" });
            output.push('}');
        }
        output.push(']');
        Ok(output)
    }
    pub fn import(&self, json: &str) -> Result<(), ClipboardError> {
        self.import_with_timestamp_parser(json, parse_clipboard_timestamp)
    }
    /// Resolves offset-less history timestamps in the supplied native zone.
    /// Explicit wire offsets retain precedence; ordering and partial-import errors
    /// follow the ordinary import path.
    pub fn import_with_time_zone(
        &self,
        json: &str,
        zone: &jiff::tz::TimeZone,
    ) -> Result<(), ClipboardError> {
        self.import_with_timestamp_parser(json, |text| {
            parse_clipboard_timestamp_in_zone(text, zone)
        })
    }
    fn import_with_timestamp_parser(
        &self,
        json: &str,
        parse_timestamp: impl Fn(&str) -> Result<OffsetDateTime, TimestampError>,
    ) -> Result<(), ClipboardError> {
        let doc = StrictJsonValue::parse(json)?;
        self.items.borrow_mut().clear();
        for el in doc.array()? {
            let text = el
                .get("text")?
                .ok_or(ClipboardError::MissingProperty)?
                .string()?
                .unwrap_or_default();
            let item = Rc::new(ClipboardItem::new(text));
            let timestamp = if let Some(v) = el.get("timestamp")? {
                let s = v.string()?.ok_or(JsonBridgeError::InvalidOperation)?;
                parse_timestamp(&s.to_utf8().map_err(|_| JsonBridgeError::InvalidString)?)
                    .map_err(ClipboardError::Timestamp)?
            } else {
                self.now()
            };
            item.set_timestamp(timestamp);
            let pinned = if let Some(v) = el.get("pinned")? {
                v.boolean()?
            } else {
                false
            };
            item.set_is_pinned(pinned);
            self.items.borrow_mut().push(item);
        }
        self.changed.raise(self);
        Ok(())
    }
}
