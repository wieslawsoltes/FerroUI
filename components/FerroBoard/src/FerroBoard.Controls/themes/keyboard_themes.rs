//! Derived from Themes/KeyboardThemes.cs (Apache-2.0).
use super::{KeyboardTheme, ThemeError};
use ferroboard_core::text::{OrdinalKey, Utf16Text};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
type Changed = Arc<dyn Fn() + Send + Sync>;
struct Registry {
    indices: HashMap<OrdinalKey, usize>,
    themes: Vec<Arc<KeyboardTheme>>,
    handlers: Vec<(u64, Changed)>,
    next: u64,
}
fn registry() -> &'static Mutex<Registry> {
    static VALUE: OnceLock<Mutex<Registry>> = OnceLock::new();
    VALUE.get_or_init(|| {
        let mut result = Registry {
            indices: HashMap::new(),
            themes: Vec::new(),
            handlers: Vec::new(),
            next: 0,
        };
        for theme in create_built_in() {
            if theme.id() == Some("high_contrast".into()) {
                KeyboardThemes::apply_high_contrast_borders(&theme);
            }
            let key = OrdinalKey(theme.id().expect("pinned theme id"));
            result.indices.insert(key, result.themes.len());
            result.themes.push(theme);
        }
        Mutex::new(result)
    })
}
/// Process-wide, shared theme registry. Notifications run synchronously after releasing the lock.
/// Callbacks are Send+Sync because Register may run on another thread; UI queueing belongs to the
/// control subscriber as in source, not to this event. Native markup callback adaptation is pending.
pub struct KeyboardThemes;

/// Owns one synchronous theme-registry subscription.
///
/// Dropping this guard unsubscribes. Move it to transfer ownership, or call
/// `unsubscribe` to release it earlier. Callback captures are released outside
/// the registry lock, so their destructors may reenter the registry.
#[must_use = "dropping the theme subscription unsubscribes its callback"]
pub struct ThemeSubscription {
    token: Option<u64>,
}
impl ThemeSubscription {
    /// Removes this subscription once. Returns false if already removed.
    pub fn unsubscribe(&mut self) -> bool {
        // Publish removal before dropping caller-owned captures, including when
        // a capture's destructor panics. Drop must not retry that cleanup.
        self.token
            .take()
            .is_some_and(KeyboardThemes::remove_changed)
    }

    #[cfg(test)]
    pub(crate) fn test_token(&self) -> u64 {
        self.token.expect("active theme subscription")
    }
}
impl Drop for ThemeSubscription {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}

impl KeyboardThemes {
    pub fn all() -> Vec<Arc<KeyboardTheme>> {
        registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .themes
            .clone()
    }
    pub fn register(theme: Arc<KeyboardTheme>) -> Result<(), ThemeError> {
        Self::register_nullable(Some(theme))
    }
    pub fn register_nullable(theme: Option<Arc<KeyboardTheme>>) -> Result<(), ThemeError> {
        let handlers = {
            let mut state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let theme = theme.ok_or(ThemeError::NullReference)?;
            let key = OrdinalKey(theme.id().ok_or(ThemeError::ArgumentNull)?);
            if let Some(&index) = state.indices.get(&key) {
                state.themes[index] = theme;
            } else {
                let index = state.themes.len();
                state.indices.insert(key, index);
                state.themes.push(theme);
            }
            state
                .handlers
                .iter()
                .map(|(_, f)| f.clone())
                .collect::<Vec<_>>()
        };
        for handler in handlers {
            handler();
        }
        Ok(())
    }
    pub fn get(id: &str) -> Option<Arc<KeyboardTheme>> {
        Self::get_text(Some(&id.into())).expect("nonnull id")
    }
    pub fn get_text(id: Option<&Utf16Text>) -> Result<Option<Arc<KeyboardTheme>>, ThemeError> {
        let state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let key = OrdinalKey(id.ok_or(ThemeError::ArgumentNull)?.clone());
        Ok(state.indices.get(&key).map(|&i| state.themes[i].clone()))
    }
    pub fn floris_day() -> Arc<KeyboardTheme> {
        Self::get("floris_day").expect("registered default theme")
    }
    pub fn floris_night() -> Arc<KeyboardTheme> {
        Self::get("floris_night").expect("registered default theme")
    }
    pub fn changed(handler: impl Fn() + Send + Sync + 'static) -> ThemeSubscription {
        ThemeSubscription {
            token: Some(Self::add_changed(handler)),
        }
    }
    /// Token bridge for handlers that must unsubscribe during a synchronous cross-thread event.
    pub fn add_changed(handler: impl Fn() + Send + Sync + 'static) -> u64 {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let token = state.next;
        state.next = state.next.wrapping_add(1);
        state.handlers.push((token, Arc::new(handler)));
        token
    }
    pub fn remove_changed(token: u64) -> bool {
        let removed = {
            let mut state = registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state
                .handlers
                .iter()
                .position(|(id, _)| *id == token)
                .map(|index| state.handlers.remove(index))
        };
        // The callback can own user values whose Drop reads or mutates this
        // registry. Release the mutex before releasing the removed callback.
        removed.is_some()
    }
    pub(crate) fn apply_high_contrast_borders(theme: &KeyboardTheme) {
        theme.set_key_border_color(theme.key_foreground());
        theme.set_key_border_thickness(1.5);
    }
}
fn hex(text: &str) -> ferroui_base::media::Color {
    KeyboardTheme::hex(text).expect("pinned theme color")
}
include!("builtin_themes.rs");
