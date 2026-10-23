//! Scope delivery for a window served by the thread's IMM transport.
//!
//! `SetInputScopes` accepts the full ordered native scope array. Its window
//! association is separate from an IMM context and must be cleared before the
//! window is destroyed. Without a TSF Document Manager, Windows does not send
//! live scope-change notifications to text services such as the touch keyboard.

use super::{Imm32InputMethod, Imm32Parent, ImmSystem};
use ferroui_base::input::InputScopeNameValue;
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const IS_DEFAULT: i32 = 0;
const IS_PRIVATE: i32 = 61;

/// Values shared by the public scope enum and the Windows SDK InputScope enum.
/// Privacy is independent of the keyboard choice; it is a hint, not masking or
/// a guarantee that an input method will handle secrets securely.
fn native_scopes(options: &TextInputOptions) -> Rc<[i32]> {
    let names = options
        .input_scope_names
        .as_deref()
        .filter(|names| !names.is_empty());
    let mut scopes = match names {
        Some(names) => names.iter().map(|scope| *scope as i32).collect::<Vec<_>>(),
        None => vec![InputScopeNameValue::from(options.content_type) as i32],
    };
    let private = options.is_sensitive
        || matches!(
            options.content_type,
            TextInputContentType::Password | TextInputContentType::Pin
        )
        || names.is_some_and(|names| {
            names.iter().any(|scope| {
                matches!(
                    scope,
                    InputScopeNameValue::Password
                        | InputScopeNameValue::NumericPassword
                        | InputScopeNameValue::NumericPin
                        | InputScopeNameValue::AlphanumericPin
                )
            })
        });
    if private {
        scopes.push(IS_PRIVATE);
    }
    Rc::from(scopes)
}

#[derive(Clone)]
struct ScopeTarget {
    hwnd: isize,
    parent: Weak<dyn Imm32Parent>,
}

impl ScopeTarget {
    fn same(&self, other: &Self) -> bool {
        self.hwnd == other.hwnd && Weak::ptr_eq(&self.parent, &other.parent)
    }

    fn matches(&self, hwnd: isize, parent: Option<&Weak<dyn Imm32Parent>>) -> bool {
        self.hwnd == hwnd && parent.is_some_and(|parent| Weak::ptr_eq(&self.parent, parent))
    }

    fn is_live(&self) -> bool {
        self.hwnd != 0
            && self
                .parent
                .upgrade()
                .is_some_and(|parent| parent.hwnd() == self.hwnd)
    }
}

struct ScopeWindow {
    target: ScopeTarget,
    retired: Cell<bool>,
    // A failed setter may have changed the association before returning an
    // error. Such a window still needs best-effort cleanup and a later retry.
    touched: Cell<bool>,
    applied: RefCell<Option<Rc<[i32]>>>,
}

#[derive(Default)]
pub(super) struct ImmInputOptions {
    options: RefCell<Option<TextInputOptions>>,
    revision: Cell<u64>,
    applying: Cell<bool>,
    windows: RefCell<Vec<Rc<ScopeWindow>>>,
}

pub(super) struct ApplyingGuard<'a> {
    applying: &'a Cell<bool>,
    previous: bool,
}

impl Drop for ApplyingGuard<'_> {
    fn drop(&mut self) {
        self.applying.set(self.previous);
    }
}

impl ImmInputOptions {
    pub(super) fn update(&self, options: Option<TextInputOptions>) {
        if *self.options.borrow() != options {
            self.options.replace(options);
            self.revision.set(self.revision.get().wrapping_add(1));
        }
    }

    pub(super) fn pause(&self) -> ApplyingGuard<'_> {
        ApplyingGuard {
            applying: &self.applying,
            previous: self.applying.replace(true),
        }
    }

    fn target(&self, input: &Imm32InputMethod) -> Option<ScopeTarget> {
        if !input.is_active() {
            return None;
        }
        let connection = input.connection.borrow().clone();
        if connection
            .as_ref()
            .is_some_and(|connection| !connection.is_current())
        {
            return None;
        }
        let source_site = input.source_site.borrow().clone();
        if let Some((site, source)) = source_site {
            site.upgrade()?;
            if source.upgrade()?.hwnd() == 0 {
                return None;
            }
        }
        let target = ScopeTarget {
            hwnd: input.hwnd.get(),
            parent: input.parent.borrow().clone()?,
        };
        if !target.is_live()
            || self
                .windows
                .borrow()
                .iter()
                .any(|window| window.retired.get() && window.target.same(&target))
        {
            return None;
        }
        Some(target)
    }

    fn window(&self, target: ScopeTarget) -> Rc<ScopeWindow> {
        let mut windows = self.windows.borrow_mut();
        windows.retain(|window| !window.retired.get() || window.target.parent.strong_count() != 0);
        if let Some(window) = windows.iter().find(|window| window.target.same(&target)) {
            return window.clone();
        }
        let window = Rc::new(ScopeWindow {
            target,
            retired: Cell::new(false),
            touched: Cell::new(false),
            applied: RefCell::new(None),
        });
        windows.push(window.clone());
        window
    }

    fn clear_window(window: &ScopeWindow, system: &dyn ImmSystem) -> bool {
        window.applied.take();
        if !window.touched.replace(false) || !window.target.is_live() {
            return true;
        }
        let success = system.set_input_scopes(window.target.hwnd, &[IS_DEFAULT]);
        if !success && !window.retired.get() {
            window.touched.set(true);
        }
        success
    }

    /// Called on the UI thread. Setters may synchronously change options,
    /// clients, or windows; no RefCell borrow survives a native call.
    pub(super) fn apply(&self, input: &Imm32InputMethod) {
        if self.applying.get() {
            return;
        }
        let _guard = self.pause();
        'latest: loop {
            let stamp = (
                self.revision.get(),
                input.generation.get(),
                input.context_generation.get(),
            );
            let current = || {
                stamp
                    == (
                        self.revision.get(),
                        input.generation.get(),
                        input.context_generation.get(),
                    )
            };
            let scopes = self.options.borrow().as_ref().map(native_scopes);
            let target = scopes.as_ref().and_then(|_| self.target(input));
            if !current() {
                continue;
            }
            let windows = self.windows.borrow().clone();
            for window in windows {
                if window.retired.get()
                    || target
                        .as_ref()
                        .is_some_and(|target| window.target.same(target))
                {
                    continue;
                }
                if window.touched.get() {
                    let success = Self::clear_window(&window, &*input.system);
                    if !current() {
                        continue 'latest;
                    }
                    if !success {
                        return;
                    }
                }
            }
            let (Some(target), Some(scopes)) = (target, scopes) else {
                return;
            };
            let window = self.window(target);
            if window.retired.get() || window.applied.borrow().as_deref() == Some(scopes.as_ref()) {
                return;
            }
            // Publish ownership before native notifications can close the
            // window. A retired window never receives another delayed write.
            window.touched.set(true);
            window.applied.take();
            let success = input.system.set_input_scopes(window.target.hwnd, &scopes);
            if window.retired.get() {
                // A synchronous native setter can finish after its callback
                // released the window. Cleanup is needed both before window
                // destruction and after that in-flight write, if the original
                // identity is still live. New owner delivery stays queued.
                if scopes.as_ref() != [IS_DEFAULT] {
                    window.touched.set(true);
                    Self::clear_window(&window, &*input.system);
                }
            } else if success {
                window.touched.set(scopes.as_ref() != [IS_DEFAULT]);
                window.applied.replace(Some(scopes));
            }
            if !current() {
                continue;
            }
            // Failures remain uncached and retry on the next update, without
            // spinning on a native API failure in the same UI turn.
            return;
        }
    }

    /// Retires exactly this physical window identity, including a window for
    /// which no HIMC was ever acquired. A reused HWND belongs to its new parent.
    pub(super) fn retire_window(
        &self,
        hwnd: isize,
        parent: Option<&Weak<dyn Imm32Parent>>,
        system: &dyn ImmSystem,
    ) {
        let windows = self.windows.borrow().clone();
        for window in windows
            .iter()
            .filter(|window| window.target.matches(hwnd, parent))
        {
            if window.retired.replace(true) {
                continue;
            }
            let replaced = windows.iter().any(|other| {
                other.target.hwnd == hwnd
                    && !other.target.same(&window.target)
                    && !other.retired.get()
                    && other.touched.get()
            });
            if replaced {
                window.touched.set(false);
                window.applied.take();
            } else {
                Self::clear_window(window, system);
            }
        }
    }

    pub(super) fn dispose(&mut self, system: &dyn ImmSystem) {
        self.applying.set(true);
        for window in self.windows.get_mut().drain(..) {
            if !window.retired.replace(true) {
                Self::clear_window(&window, system);
            }
        }
    }
}

#[cfg(test)]
#[path = "imm32_input_options_tests.rs"]
mod tests;
