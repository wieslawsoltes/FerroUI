use super::*;
use crate::{
    serialization::{StrictJsonValue, write_json_string},
    text::Utf16Text,
};
use ferroui_system::events::Event;
use parking_lot::Mutex;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::Arc,
};
#[derive(Default)]
struct TonePreferences {
    indices: HashMap<Utf16Text, usize>,
    entries: Vec<(Utf16Text, EmojiSkinTone)>,
}
pub struct EmojiHistory {
    recent: Arc<Mutex<Vec<Option<Utf16Text>>>>,
    pinned: Arc<Mutex<Vec<Option<Utf16Text>>>>,
    tones: RefCell<TonePreferences>,
    max_recent: Cell<i32>,
    changed: Event<Self>,
}
impl Default for EmojiHistory {
    fn default() -> Self {
        Self {
            recent: Arc::default(),
            pinned: Arc::default(),
            tones: RefCell::new(TonePreferences::default()),
            max_recent: Cell::new(40),
            changed: Event::new(),
        }
    }
}
impl EmojiHistory {
    pub fn changed(&self) -> &Event<Self> {
        &self.changed
    }
    pub fn max_recent(&self) -> i32 {
        self.max_recent.get()
    }
    pub fn set_max_recent(&self, value: i32) {
        self.max_recent.set(value);
    }
    /// Live native vector. Direct edits preserve identity and do not emit history notifications.
    /// Release the guard before invoking history operations or event handlers.
    pub fn recent(&self) -> Arc<Mutex<Vec<Option<Utf16Text>>>> {
        self.recent.clone()
    }
    /// Live native vector, with the same guard/notification contract as `recent`.
    pub fn pinned(&self) -> Arc<Mutex<Vec<Option<Utf16Text>>>> {
        self.pinned.clone()
    }
    pub fn combined(&self) -> Vec<Option<Utf16Text>> {
        let pinned = self.pinned.lock().clone();
        let mut combined = pinned.clone();
        combined.extend(
            self.recent
                .lock()
                .clone()
                .into_iter()
                .filter(|v| !pinned.contains(v)),
        );
        combined
    }
    pub fn add(&self, emoji: &Utf16Text) -> EmojiResult<()> {
        self.add_nullable(Some(emoji))
    }
    pub fn add_nullable(&self, emoji: Option<&Utf16Text>) -> EmojiResult<()> {
        {
            let mut recent = self.recent.lock();
            let value = emoji.cloned();
            if let Some(index) = recent.iter().position(|entry| entry == &value) {
                recent.remove(index);
            }
            recent.insert(0, value);
            while recent.len() as i64 > self.max_recent() as i64 {
                recent.pop().ok_or(EmojiError::Index)?;
            }
        }
        let emoji = emoji.ok_or(EmojiError::NullReference)?;
        let base = EmojiCatalog::strip_skin_tone(emoji)?;
        if &base != emoji {
            for cp in crate::text::TextUtils::get_code_points(emoji) {
                if (0x1f3fb..=0x1f3ff).contains(&cp) {
                    self.remember_tone(base, EmojiSkinTone(cp - 0x1f3fb + 1));
                    break;
                }
            }
        }
        self.changed.raise(self);
        Ok(())
    }
    pub fn get_preferred_tone(&self, base: &Utf16Text) -> Option<EmojiSkinTone> {
        let tones = self.tones.borrow();
        tones.indices.get(base).map(|&index| tones.entries[index].1)
    }
    fn remember_tone(&self, base: Utf16Text, tone: EmojiSkinTone) {
        let mut tones = self.tones.borrow_mut();
        if let Some(&index) = tones.indices.get(&base) {
            tones.entries[index].1 = tone;
        } else {
            let index = tones.entries.len();
            tones.indices.insert(base.clone(), index);
            tones.entries.push((base, tone));
        }
    }
    pub fn pin(&self, emoji: &Utf16Text) -> EmojiResult<()> {
        let added = {
            let mut pinned = self.pinned.lock();
            let value = Some(emoji.clone());
            if pinned.contains(&value) {
                false
            } else {
                pinned.push(value);
                true
            }
        };
        if added {
            self.changed.raise(self);
        }
        Ok(())
    }
    pub fn unpin(&self, emoji: &Utf16Text) -> EmojiResult<()> {
        let removed = {
            let mut pinned = self.pinned.lock();
            let index = pinned
                .iter()
                .position(|entry| entry.as_ref() == Some(emoji));
            index.map(|index| pinned.remove(index)).is_some()
        };
        if removed {
            self.changed.raise(self);
        }
        Ok(())
    }
    pub fn remove(&self, emoji: &Utf16Text) -> EmojiResult<()> {
        let removed = {
            let mut recent = self.recent.lock();
            let index = recent
                .iter()
                .position(|entry| entry.as_ref() == Some(emoji));
            index.map(|index| recent.remove(index)).is_some()
        };
        if removed {
            self.changed.raise(self);
        }
        Ok(())
    }
    pub fn clear_recent(&self) -> EmojiResult<()> {
        self.recent.lock().clear();
        self.changed.raise(self);
        Ok(())
    }
    pub fn export(&self) -> String {
        let mut out = String::from("{\"recent\":[");
        for (i, e) in self.recent.lock().clone().iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            if let Some(e) = e {
                write_json_string(&mut out, e)
            } else {
                out.push_str("null")
            }
        }
        out.push_str("],\"pinned\":[");
        for (i, e) in self.pinned.lock().clone().iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            if let Some(e) = e {
                write_json_string(&mut out, e)
            } else {
                out.push_str("null")
            }
        }
        out.push_str("],\"tones\":{");
        for (i, (key, tone)) in self.tones.borrow().entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_json_string(&mut out, key);
            out.push(':');
            out.push_str(&tone.0.to_string());
        }
        out.push_str("}}");
        out
    }
    pub fn import(&self, json: &str) -> EmojiResult<()> {
        let root = StrictJsonValue::parse(json)?;
        self.recent.lock().clear();
        self.pinned.lock().clear();
        for (name, list) in [("recent", &self.recent), ("pinned", &self.pinned)] {
            if let Some(values) = root.get(name)? {
                for v in values.array()? {
                    let value = v.string()?.unwrap_or_default();
                    if !value.is_empty() {
                        list.lock().push(Some(value));
                    }
                }
            }
        }
        if let Some(tones) = root.get("tones")? {
            for (name, value) in tones.object()? {
                let name = StrictJsonValue::valid_string(&name.text)?;
                let value = EmojiSkinTone(value.int32()?);
                self.remember_tone(name, value);
            }
        }
        self.changed.raise(self);
        Ok(())
    }
}
