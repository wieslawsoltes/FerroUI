//! A Windows input method editor based on Windows Input Method Manager
//! (IMM32) (the port of `Input/Imm32InputMethod.cs`).
//!
//! The input method is written against the calls it makes of the system
//! ([`ImmSystem`]) and against what it needs of the window it serves
//! ([`Imm32Parent`]), so that what it does with a composition is tested on
//! every host: on Windows the first is the system and the second the
//! window implementation.

#[path = "imm32_input_options.rs"]
mod input_options;

use input_options::ImmInputOptions;
use super::imm32_caret_manager::Imm32CaretManager;
use super::imm32_context::{ImmContext, ImmWindowContext, OwnedImmContext};
use crate::interop::unmanaged_methods::{
    CANDIDATEFORM, CFS_EXCLUDE, CFS_POINT, COMPOSITIONFORM, CPS_CANCEL, CPS_COMPLETE, GCS, LANG_KO,
    NI_COMPOSITIONSTR, POINT, RECT, lgid, primary_lang_id,
};
use ferroui_base::Rect;
use ferroui_base::input::text_input::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodComposition,
    TextInputMethodConnection, TextInputOptions,
};
use ferroui_base::input::{Key, PhysicalKey};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The calls the input method makes of the system.
pub(crate) trait ImmSystem {
    fn imm_get_context(&self, hwnd: isize) -> isize;
    fn imm_create_context(&self) -> isize;
    fn imm_destroy_context(&self, himc: isize);
    fn imm_associate_context(&self, hwnd: isize, himc: isize) -> isize;
    fn imm_release_context(&self, hwnd: isize, himc: isize);
    fn imm_notify_ime(&self, himc: isize, action: i32, index: i32, value: i32);
    fn imm_set_candidate_window(&self, himc: isize, candidate: &CANDIDATEFORM);
    fn imm_set_composition_window(&self, himc: isize, form: &COMPOSITIONFORM);
    fn imm_set_composition_font(&self, himc: isize, height: i32, quality: u8);
    /// A string of the composition; `None` when there is none.
    fn imm_get_composition_string(&self, himc: isize, flag: u32) -> Option<String>;
    /// A number of the composition (the cursor position); negative for an
    /// error.
    fn imm_get_composition_value(&self, himc: isize, flag: u32) -> i32;
    fn create_caret(&self, hwnd: isize, width: i32, height: i32) -> bool;
    fn set_caret_pos(&self, x: i32, y: i32);
    fn destroy_caret(&self);
    fn get_active_window(&self) -> isize;
    /// Sets or clears the scope association on a window of the calling thread.
    fn set_input_scopes(&self, hwnd: isize, scopes: &[i32]) -> bool;
    /// Tests the next queued message without removing it (Korean END-before-RESULT).
    fn next_message_is_composition_result(&self, _hwnd: isize) -> bool {
        false
    }
    /// Maps a two-point RECT in physical pixels from the semantic source
    /// window into the actual IMM recipient (including mirrored windows).
    fn map_window_rect(&self, from: isize, to: isize, rect: RECT) -> Option<RECT> {
        (from == to).then_some(rect)
    }
    /// Runs an action later on the UI thread (`Dispatcher.UIThread.Post`).
    fn post(&self, action: Box<dyn FnOnce()>);
}

/// What the input method needs of the window it serves (the reference
/// holds the window implementation).
pub(crate) trait Imm32Parent {
    fn hwnd(&self) -> isize;
    fn desktop_scaling(&self) -> f64;
    /// The next `WM_CHAR` message of the window is not text.
    fn set_ignore_wm_char(&self, value: bool);
    /// Raw text input for the window. False when the window has no input
    /// callback.
    fn raw_text_input(&self, timestamp: u64, text: &str) -> bool;
    /// A key pressed and released, without modifiers, for the window.
    fn raw_key_press(&self, key: Key, physical_key: PhysicalKey);
}

const CARET_MARGIN: i32 = 1;

/// The rectangle of the text cursor in device pixels of the client area:
/// its two corners scaled and truncated.
pub(crate) fn cursor_rect_in_pixels(rect: Rect, scaling: f64) -> (i32, i32, i32, i32) {
    let p1 = rect.top_left();
    let p2 = rect.bottom_right();
    (
        (p1.x * scaling) as i32,
        (p1.y * scaling) as i32,
        (p2.x * scaling) as i32,
        (p2.y * scaling) as i32,
    )
}

/// The rectangle the candidate window of an input method keeps clear of,
/// for a text cursor in pixels and the primary language of the keyboard
/// layout.
// Chinese, Japanese, and Korean(CJK) IMEs also use the rectangle given to
// ::ImmSetCandidateWindow() with its 'dwStyle' parameter CFS_EXCLUDE
// to move their candidate windows when a user disables TSF and CUAS.
// Therefore, we also set this parameter here.
pub(crate) fn candidate_exclude_rectangle(
    x1: i32,
    y1: i32,
    x2: i32,
    mut y2: i32,
    lang_id: u16,
) -> CANDIDATEFORM {
    if i32::from(lang_id) == LANG_KO {
        // Chinese IMEs and Japanese IMEs require the upper-left corner of
        // the caret to move the position of their candidate windows.
        // On the other hand, Korean IMEs require the lower-left corner of the
        // caret to move their candidate windows.
        y2 += CARET_MARGIN;
    }

    CANDIDATEFORM {
        dwIndex: 0,
        dwStyle: CFS_EXCLUDE,
        ptCurrentPos: POINT { x: x1, y: y1 },
        rcArea: RECT {
            left: x1,
            top: y1,
            right: x2,
            bottom: y2 + CARET_MARGIN,
        },
    }
}

/// The low 32 bits of a message parameter (`ToInt32`).
fn to_int32(ptr: isize) -> i32 {
    (ptr as i64 & 0xffff_ffff) as u32 as i32
}

/// A Windows input method editor based on Windows Input Method Manager
/// (IMM32).
pub(crate) struct Imm32InputMethod {
    this: Weak<Imm32InputMethod>,
    system: Rc<dyn ImmSystem>,
    hwnd: Cell<isize>,
    current_himc: Cell<isize>,
    contexts: RefCell<Vec<Rc<ImmWindowContext>>>,
    context_generation: Cell<u64>,
    parent: RefCell<Option<Weak<dyn Imm32Parent>>>,
    source_site: RefCell<Option<(Weak<()>, Weak<dyn Imm32Parent>)>>,
    input_options: ImmInputOptions,
    caret_manager: Imm32CaretManager,
    lang_id: Cell<u16>,
    ignore_composition: Cell<bool>,
    composition_cursor_position: Cell<Option<i32>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
    composition_session: RefCell<Option<TextInputMethodComposition>>,
    generation: Cell<u64>,
    end_before_result: Cell<bool>,
    managed_input: Cell<bool>,
    native_composition_cancelled: Cell<bool>,
    completion_rejected: Cell<bool>,
    is_composing: Cell<bool>,
    composition: RefCell<Option<String>>,
}

impl Imm32InputMethod {
    pub fn new(system: Rc<dyn ImmSystem>) -> Rc<Imm32InputMethod> {
        Rc::new_cyclic(|this| Imm32InputMethod {
            this: this.clone(),
            system,
            hwnd: Cell::new(0),
            current_himc: Cell::new(0),
            contexts: RefCell::new(Vec::new()),
            context_generation: Cell::new(0),
            parent: RefCell::new(None),
            source_site: RefCell::new(None),
            input_options: ImmInputOptions::default(),
            caret_manager: Imm32CaretManager::default(),
            lang_id: Cell::new(0),
            ignore_composition: Cell::new(false),
            composition_cursor_position: Cell::new(None),
            client: RefCell::new(None),
            connection: RefCell::new(None),
            composition_session: RefCell::new(None),
            generation: Cell::new(0),
            end_before_result: Cell::new(false),
            managed_input: Cell::new(false),
            native_composition_cancelled: Cell::new(false),
            completion_rejected: Cell::new(false),
            is_composing: Cell::new(false),
            composition: RefCell::new(None),
        })
    }

    /// A stable per-window facade over the thread's IMM transport. A stale
    /// facade cannot detach, reset, or reposition a later owner's client.
    pub(crate) fn for_window(
        self: &Rc<Self>,
        parent: Weak<dyn Imm32Parent>,
    ) -> Rc<dyn ITextInputMethodImpl> {
        Rc::new(Imm32InputMethodSite {
            transport: self.clone(),
            identity: Rc::new(()),
            parent,
            options: RefCell::new(None),
        })
    }

    fn owns_site(&self, identity: &Rc<()>) -> bool {
        self.source_site
            .borrow()
            .as_ref()
            .and_then(|(site, _)| site.upgrade())
            .is_some_and(|site| Rc::ptr_eq(&site, identity))
    }

    /// The window the input method serves; 0 for none (`Hwnd`).
    pub fn hwnd(&self) -> isize {
        self.hwnd.get()
    }

    pub fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    pub fn has_managed_connection(&self) -> bool {
        self.managed_input.get()
    }

    pub fn is_active(&self) -> bool {
        self.client.borrow().is_some()
    }

    pub fn is_composing(&self) -> bool {
        self.is_composing.get()
    }

    #[allow(dead_code)] // A member of the reference without a caller there.
    pub fn set_is_composing(&self, value: bool) {
        self.is_composing.set(value);
    }

    pub fn show_composition_window(&self) -> bool {
        false
    }

    /// The composition string as it was last reported (`Composition`).
    pub fn composition(&self) -> Option<String> {
        self.composition.borrow().clone()
    }

    /// Sets the composition string (the internal setter of `Composition`).
    pub(crate) fn set_composition(&self, composition: Option<String>) {
        *self.composition.borrow_mut() = composition;
    }

    fn parent(&self) -> Option<Rc<dyn Imm32Parent>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }

    #[allow(dead_code)] // A member of the reference without a caller there.
    pub fn create_caret(&self) {
        self.caret_manager
            .try_create(&*self.system, self.hwnd.get());
    }

    fn current_window_context(&self) -> Option<Rc<ImmWindowContext>> {
        let hwnd = self.hwnd.get();
        if hwnd == 0 {
            return None;
        }
        let parent = self.parent.borrow().clone();
        let existing = self
            .contexts
            .borrow()
            .iter()
            .find(|record| record.hwnd == hwnd)
            .cloned();
        if let Some(existing) = existing {
            return (!existing.retired.get() && existing.same_parent(parent.as_ref()))
                .then_some(existing);
        }
        let record = ImmWindowContext::new(self.system.clone(), hwnd, parent);
        self.contexts.borrow_mut().push(record.clone());
        Some(record)
    }

    fn acquire_context(&self, hwnd: isize) -> Option<ImmContext> {
        let record = self
            .contexts
            .borrow()
            .iter()
            .find(|record| record.hwnd == hwnd)
            .cloned();
        if let Some(record) = record {
            if record.retired.get() {
                return None;
            }
            record.acquire()
        } else {
            ImmContext::acquire(self.system.clone(), hwnd, None)
        }
    }

    fn next_context_generation(&self) -> u64 {
        let generation = self.context_generation.get().wrapping_add(1);
        self.context_generation.set(generation);
        generation
    }

    fn context_operation_is_current(
        &self,
        generation: u64,
        context_generation: u64,
        record: &ImmWindowContext,
    ) -> bool {
        self.generation.get() == generation
            && self.context_generation.get() == context_generation
            && self.hwnd.get() == record.hwnd
            && !record.retired.get()
            && record.same_parent(self.parent.borrow().as_ref())
    }

    pub fn enable_imm(&self) {
        let context_generation = self.next_context_generation();
        let generation = self.generation.get();
        let Some(record) = self.current_window_context() else {
            return;
        };
        let acquired = record.acquire();
        let mut himc = acquired.as_ref().map_or(0, |context| context.handle);
        // Even a repeated enable releases its successful native acquisition.
        drop(acquired);
        if !self.context_operation_is_current(generation, context_generation, &record) {
            return;
        }

        if himc == 0 {
            let mut owned = record.owned.borrow().clone();
            himc = record
                .suspended
                .get()
                .filter(|handle| *handle != 0)
                .or_else(|| owned.as_ref().map(|owned| owned.handle))
                .unwrap_or(0);
            let created = himc == 0;
            if created {
                owned = OwnedImmContext::create(self.system.clone());
                if !self.context_operation_is_current(generation, context_generation, &record) {
                    return;
                }
                let Some(context) = owned.as_ref() else {
                    return;
                };
                himc = context.handle;
                // Publish ownership before an association can invoke native callbacks.
                let previous = record.owned.replace(owned.clone());
                drop(previous);
            }
            record.suspended.set(None);
            self.current_himc.set(himc);
            let previous = self.system.imm_associate_context(record.hwnd, himc);
            if !self.context_operation_is_current(generation, context_generation, &record) {
                return;
            }
            if created {
                record.original.set(previous);
            }
        } else {
            record.suspended.set(None);
            self.current_himc.set(himc);
        }
        self.caret_manager.try_create(&*self.system, record.hwnd);
    }

    pub fn disable_imm(&self) -> bool {
        let context_generation = self.next_context_generation();
        let Some(record) = self.current_window_context() else {
            return false;
        };
        let Some((generation, managed)) = self.prepare_reset() else {
            return false;
        };
        // Complete/cancel while the old HWND still has its context associated.
        self.reset_native(generation, managed);
        if !self.context_operation_is_current(generation, context_generation, &record) {
            return false;
        }
        self.caret_manager.try_destroy(&*self.system);
        if !self.context_operation_is_current(generation, context_generation, &record) {
            return false;
        }
        let acquired = record.acquire();
        let himc = acquired.as_ref().map_or(0, |context| context.handle);
        drop(acquired);
        if !self.context_operation_is_current(generation, context_generation, &record) {
            return false;
        }
        self.current_himc.set(0);
        if himc == 0 {
            return true;
        }
        // Reentrant enable can see the saved association immediately.
        record.suspended.set(Some(himc));
        let previous = self.system.imm_associate_context(record.hwnd, 0);
        if !self.context_operation_is_current(generation, context_generation, &record) {
            return false;
        }
        if previous != 0 {
            record.suspended.set(Some(previous));
        }
        true
    }

    pub fn set_language_and_window(&self, parent: Weak<dyn Imm32Parent>, hwnd: isize, hkl: isize) {
        let lang_id = primary_lang_id(lgid(hkl));
        let same_window = self.hwnd.get() == hwnd
            && self
                .parent
                .borrow()
                .as_ref()
                .is_some_and(|old| Weak::ptr_eq(old, &parent));
        if same_window && lang_id == self.lang_id.get() {
            return;
        }
        if same_window && self.is_active() {
            if !self.disable_imm() {
                return;
            }
            if self.hwnd.get() != hwnd
                || !self
                    .parent
                    .borrow()
                    .as_ref()
                    .is_some_and(|old| Weak::ptr_eq(old, &parent))
            {
                return;
            }
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let context_generation = self.next_context_generation();
        self.hwnd.set(hwnd);
        *self.parent.borrow_mut() = Some(parent);
        self.current_himc.set(0);
        self.lang_id.set(lang_id);
        self.caret_manager.try_destroy(&*self.system);
        if self.generation.get() != generation
            || self.context_generation.get() != context_generation
        {
            return;
        }
        // A new physical window needs its own association even when HKL did not change.
        if self.is_active() {
            self.enable_imm();
        }
        self.input_options.apply(self);
    }

    pub fn clear_language_and_window(&self) {
        let hwnd = self.hwnd.get();
        let parent = self.parent.borrow().clone();
        self.release_physical_window(hwnd, parent.as_ref());
    }

    /// Releases one native window, including inactive windows whose disabled or
    /// owned contexts must not be confused with the thread's current recipient.
    pub(crate) fn release_window(&self, hwnd: isize, parent: &Weak<dyn Imm32Parent>) {
        self.release_physical_window(hwnd, Some(parent));
    }

    fn release_physical_window(&self, hwnd: isize, parent: Option<&Weak<dyn Imm32Parent>>) {
        if hwnd == 0 {
            return;
        }
        // Native cleanup may run user callbacks. Stage any replacement scope
        // until the outgoing association and physical ownership are retired.
        let options_guard = self.input_options.pause();
        let record = self
            .contexts
            .borrow()
            .iter()
            .find(|record| record.hwnd == hwnd && record.same_parent(parent))
            .cloned();
        if record
            .as_ref()
            .is_some_and(|record| record.retired.replace(true))
        {
            return;
        }
        let same_parent = match (self.parent.borrow().as_ref(), parent) {
            (Some(a), Some(b)) => Weak::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let source_is_closing = self
            .source_site
            .borrow()
            .as_ref()
            .is_some_and(|(_, source)| parent.is_some_and(|parent| Weak::ptr_eq(source, parent)));
        if self.hwnd.get() == hwnd && same_parent {
            let generation = self.generation.get().wrapping_add(1);
            self.generation.set(generation);
            self.next_context_generation();
            self.hwnd.set(0);
            self.parent.take();
            self.current_himc.set(0);
            self.lang_id.set(0);
            let retain_owner = !source_is_closing
                && self.source_site.borrow().is_some()
                && self
                    .connection
                    .borrow()
                    .as_ref()
                    .is_some_and(|connection| connection.is_current());
            let previous = if retain_owner {
                None
            } else {
                self.source_site.take();
                self.input_options.update(None);
                self.connection.take();
                self.composition_session.take();
                self.is_composing.set(false);
                self.composition_cursor_position.set(None);
                self.set_composition(None);
                self.client.take()
            };
            self.input_options.retire_window(hwnd, parent, &*self.system);
            if self.generation.get() == generation {
                self.caret_manager.try_destroy(&*self.system);
            }
            if self.generation.get() == generation {
                if let Some(previous) = previous {
                    previous.set_preedit_text_with_cursor(None, None);
                }
            }
            if !retain_owner && self.generation.get() == generation {
                if let Some(context) = record.as_ref().and_then(|record| record.acquire()) {
                    if self.generation.get() == generation {
                        self.system.imm_notify_ime(
                            context.handle,
                            NI_COMPOSITIONSTR,
                            CPS_CANCEL,
                            0,
                        );
                    }
                }
            }
        } else if source_is_closing {
            self.source_site.take();
            self.set_client(None);
        }
        self.input_options.retire_window(hwnd, parent, &*self.system);
        if let Some(record) = record {
            record.dispose();
            self.contexts
                .borrow_mut()
                .retain(|current| !Rc::ptr_eq(current, &record));
        }
        drop(options_guard);
        self.input_options.apply(self);
    }

    // see: https://chromium.googlesource.com/experimental/chromium/src/+/bf09a5036ccfb77d2277247c66dc55daf41df3fe/chrome/browser/ime_input.cc
    // see: https://engine.chinmaygarde.com/window__win32_8cc_source.html
    fn move_ime_window(&self, rect: Rect, himc: isize) {
        let generation = self.generation.get();
        let context_generation = self.context_generation.get();
        let hwnd = self.hwnd.get();
        let is_current = || {
            self.generation.get() == generation
                && self.context_generation.get() == context_generation
                && self.hwnd.get() == hwnd
        };
        let source_site = self.source_site.borrow().clone();
        let source = match source_site {
            Some((site, parent)) if site.upgrade().is_some() => parent.upgrade(),
            Some(_) => return,
            None => self.parent(),
        };
        let Some(source) = source else {
            return;
        };
        let source_hwnd = source.hwnd();
        if source_hwnd == 0 {
            return;
        }
        let (x1, y1, x2, y2) = cursor_rect_in_pixels(rect, source.desktop_scaling());
        if !is_current() {
            return;
        }
        let Some(mapped) = self.system.map_window_rect(
            source_hwnd,
            hwnd,
            RECT {
                left: x1,
                top: y1,
                right: x2,
                bottom: y2,
            },
        ) else {
            return;
        };
        if !is_current() {
            return;
        }
        let (x1, y1, x2, y2) = (mapped.left, mapped.top, mapped.right, mapped.bottom);

        self.caret_manager.try_move(&*self.system, x2, y2);
        if !is_current() {
            return;
        }

        if self.show_composition_window() {
            self.configure_composition_window(x1, y1, himc, y2 - y1);
            // Don't need to set the position of candidate window.
            return;
        }

        let exclude_rectangle = candidate_exclude_rectangle(x1, y1, x2, y2, self.lang_id.get());

        self.system
            .imm_set_candidate_window(himc, &exclude_rectangle);
    }

    fn configure_composition_window(&self, x1: i32, y1: i32, himc: isize, height: i32) {
        let comp_form = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: POINT { x: x1, y: y1 },
            ..Default::default()
        };

        self.system.imm_set_composition_window(himc, &comp_form);

        // lfQuality: 5, CLEARTYPE_QUALITY
        self.system.imm_set_composition_font(himc, height, 5);
    }

    pub fn composition_changed(&self, composition: Option<String>, cursor_position: Option<i32>) {
        if self.managed_input.get() {
            let generation = self.generation.get();
            let session = self.composition_session.borrow().clone();
            if let (Some(session), Some(text)) = (session, composition) {
                if session.update(&text, cursor_position).is_ok()
                    && self.generation.get() == generation
                {
                    self.set_composition(Some(text));
                    self.composition_cursor_position.set(cursor_position);
                }
            }
        } else {
            self.set_composition(composition.clone());
            self.composition_cursor_position.set(cursor_position);
            if let Some(client) = self.client().filter(|client| client.supports_preedit()) {
                client.set_preedit_text_with_cursor(composition.as_deref(), cursor_position);
            }
        }
    }

    pub fn get_composition_string(&self, flag: u32) -> Option<String> {
        if !self.is_composing.get()
            && !(self.managed_input.get() && self.composition_session.borrow().is_some())
        {
            return None;
        }
        let generation = self.generation.get();
        let context_generation = self.context_generation.get();
        let context = self.acquire_context(self.hwnd.get())?;
        if self.generation.get() != generation
            || self.context_generation.get() != context_generation
        {
            return None;
        }
        let result = self.system.imm_get_composition_string(context.handle, flag);
        drop(context);
        if self.generation.get() != generation
            || self.context_generation.get() != context_generation
        {
            return None;
        }
        result
    }

    fn get_composition_cursor_position(&self) -> Option<i32> {
        if !self.is_composing.get() {
            return None;
        }
        let generation = self.generation.get();
        let context_generation = self.context_generation.get();
        let context = self.acquire_context(self.hwnd.get())?;
        if self.generation.get() != generation
            || self.context_generation.get() != context_generation
        {
            return None;
        }
        let cursor = self
            .system
            .imm_get_composition_value(context.handle, GCS::GCS_CURSORPOS);
        drop(context);
        if self.generation.get() != generation
            || self.context_generation.get() != context_generation
        {
            return None;
        }
        (cursor >= 0).then_some(cursor)
    }

    pub fn handle_composition_start(&self) {
        self.native_composition_cancelled.set(false);
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        self.end_before_result.set(false);
        self.completion_rejected.set(false);
        self.set_composition(None);
        self.composition_cursor_position.set(None);

        let connection = self.connection.borrow().clone();
        if let Some(connection) = connection {
            self.composition_session.take();
            let result = connection.begin_composition();
            if self.generation.get() != generation {
                return;
            }
            let Ok(session) = result else {
                self.is_composing.set(false);
                return;
            };
            *self.composition_session.borrow_mut() = Some(session);
            self.is_composing.set(true);
            return;
        }
        if let Some(client) = self.client() {
            client.set_preedit_text_with_cursor(None, None);
            if self.generation.get() != generation {
                return;
            }

            let selection = client.selection();
            if client.supports_surrounding_text() && selection.start != selection.end {
                self.key_press(Key::Delete, PhysicalKey::Delete);
            }
        }
        if self.generation.get() == generation {
            self.is_composing.set(true);
        }
    }

    pub fn handle_composition_end(&self, timestamp: u64) {
        let generation = self.generation.get();
        // END without RESULT cancels a managed preview. A cached preedit is
        // never promoted into a commit, and a rejected RESULT can be retried.
        if self.managed_input.get() {
            if self.completion_rejected.get() {
                self.is_composing.set(false);
                return;
            }
            if i32::from(self.lang_id.get()) == LANG_KO
                && self
                    .system
                    .next_message_is_composition_result(self.hwnd.get())
            {
                self.end_before_result.set(true);
                return;
            }
            self.is_composing.set(false);
            let session = self.composition_session.take();
            if let Some(session) = session {
                let _ = session.cancel();
            }
            if self.generation.get() != generation {
                return;
            }
            self.set_composition(None);
            self.composition_cursor_position.set(None);
            return;
        }
        self.is_composing.set(false);

        let composition = self.composition();
        if let (Some(parent), Some(composition)) =
            (self.parent(), composition.filter(|text| !text.is_empty()))
        {
            if parent.raw_text_input(timestamp, &composition) {
                parent.set_ignore_wm_char(true);
            }
        }

        self.set_composition(None);
        self.composition_cursor_position.set(None);

        if let Some(client) = self.client() {
            client.set_preedit_text_with_cursor(None, None);
        }
    }

    pub fn handle_composition(&self, _w_param: usize, l_param: isize, timestamp: u64) {
        if self.ignore_composition.replace(false) {
            return;
        }

        let flags = to_int32(l_param) as u32;
        let result_changed = (flags & GCS::GCS_RESULTSTR) != 0;
        if self.managed_input.get() {
            self.handle_managed_composition(flags, timestamp);
            return;
        }
        let generation = self.generation.get();

        if flags == 0 {
            self.composition_changed(Some(String::new()), None);
            return;
        }

        if result_changed {
            let result_string = self.get_composition_string(GCS::GCS_RESULTSTR);
            if self.generation.get() != generation {
                return;
            }

            self.set_composition(None);
            self.composition_cursor_position.set(None);

            if let Some(client) = self.client() {
                client.set_preedit_text_with_cursor(None, None);
            }
            if self.generation.get() != generation {
                return;
            }

            if let (Some(parent), Some(result_string)) =
                (self.parent(), result_string.filter(|text| !text.is_empty()))
            {
                if parent.raw_text_input(timestamp, &result_string) {
                    parent.set_ignore_wm_char(true);
                }
            }
            if self.generation.get() != generation {
                return;
            }
        }

        let composition_changed = (flags & GCS::GCS_COMPSTR) != 0;
        let cursor_position_changed = (flags & GCS::GCS_CURSORPOS) != 0;

        if composition_changed || (cursor_position_changed && !result_changed) {
            let composition_string = if composition_changed {
                self.get_composition_string(GCS::GCS_COMPSTR)
            } else {
                self.composition()
            };

            let cursor_position = if cursor_position_changed {
                self.get_composition_cursor_position()
            } else {
                self.composition_cursor_position.get()
            };
            if self.generation.get() != generation {
                return;
            }
            self.composition_changed(composition_string, cursor_position);
        }
    }

    fn handle_managed_composition(&self, flags: u32, timestamp: u64) {
        let generation = self.generation.get();
        if flags == 0 {
            let session = self.composition_session.take();
            if let Some(session) = session {
                let _ = session.cancel();
            }
            if self.generation.get() != generation {
                return;
            }
            self.set_composition(None);
            self.composition_cursor_position.set(None);
            return;
        }
        if flags & GCS::GCS_RESULTSTR != 0 {
            let Some(result) = self.get_composition_string(GCS::GCS_RESULTSTR) else {
                return;
            };
            if self.generation.get() != generation {
                return;
            }
            let session = self.composition_session.borrow().clone();
            let Some(session) = session else {
                return;
            };
            let result = session.commit(&result, timestamp);
            if self.generation.get() != generation {
                return;
            }
            if result.is_err() {
                self.completion_rejected.set(session.is_current());
                return;
            }
            self.completion_rejected.set(false);
            self.composition_session.take();
            self.set_composition(None);
            self.composition_cursor_position.set(None);
            if self.end_before_result.replace(false) && flags & GCS::GCS_COMPSTR == 0 {
                self.is_composing.set(false);
            }
            if let Some(parent) = self.parent() {
                parent.set_ignore_wm_char(true);
            }
            if flags & GCS::GCS_COMPSTR != 0 {
                let connection = self.connection.borrow().clone();
                if let Some(connection) = connection {
                    let session = connection.begin_composition();
                    if self.generation.get() != generation {
                        return;
                    }
                    if let Ok(session) = session {
                        *self.composition_session.borrow_mut() = Some(session);
                    }
                }
            }
        }
        if flags & (GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) != 0 {
            let text = if flags & GCS::GCS_COMPSTR != 0 {
                self.get_composition_string(GCS::GCS_COMPSTR)
            } else {
                self.composition()
            };
            let cursor = if flags & GCS::GCS_CURSORPOS != 0 {
                self.get_composition_cursor_position()
            } else {
                self.composition_cursor_position.get()
            };
            if self.generation.get() != generation {
                return;
            }
            if text.is_some() {
                self.composition_changed(text, cursor);
            }
        }
    }

    fn key_press(&self, key: Key, physical_key: PhysicalKey) {
        if let Some(parent) = self.parent() {
            parent.raw_key_press(key, physical_key);
        }
    }
}

struct Imm32InputMethodSite {
    transport: Rc<Imm32InputMethod>,
    identity: Rc<()>,
    parent: Weak<dyn Imm32Parent>,
    // The manager sends options before SetClient establishes site ownership.
    options: RefCell<Option<TextInputOptions>>,
}

impl ITextInputMethodImpl for Imm32InputMethodSite {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        if client.is_some() {
            if !self
                .parent
                .upgrade()
                .is_some_and(|parent| parent.hwnd() != 0)
            {
                return;
            }
            *self.transport.source_site.borrow_mut() =
                Some((Rc::downgrade(&self.identity), self.parent.clone()));
            self.transport.input_options.update(self.options.borrow().clone());
            self.transport.set_client(client);
        } else if self.transport.owns_site(&self.identity) {
            self.transport.source_site.take();
            self.transport.set_client(None);
        }
    }
    fn set_cursor_rect(&self, rect: Rect) {
        if self.transport.owns_site(&self.identity) {
            self.transport.set_cursor_rect(rect);
        }
    }
    fn set_options(&self, options: &TextInputOptions) {
        self.options.replace(Some(options.clone()));
        if self.transport.owns_site(&self.identity) {
            self.transport.set_options(options);
        }
    }
    fn reset(&self) {
        if self.transport.owns_site(&self.identity) {
            self.transport.reset();
        }
    }
}

impl Drop for Imm32InputMethodSite {
    fn drop(&mut self) {
        self.set_client(None);
    }
}

impl ITextInputMethodImpl for Imm32InputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        let cancel_native = self.managed_input.get()
            && (self.is_composing.get() || self.composition_session.borrow().is_some());
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        if client.is_none() {
            self.input_options.update(None);
        }
        self.composition_session.take();
        self.connection.take();
        if self.managed_input.get()
            || client
                .as_ref()
                .is_some_and(|client| client.input_method_editing().is_some())
        {
            self.is_composing.set(false);
        }
        if cancel_native {
            // Different per-window facades share this transport. The next owner
            // can invalidate the posted disable, so retire the old native
            // composition while its HWND/HIMC still identifies the old owner.
            self.reset_native(generation, true);
            if self.generation.get() != generation {
                return;
            }
        }
        if let Some(old) = self.client() {
            self.set_composition(None);
            self.composition_cursor_position.set(None);

            old.set_preedit_text_with_cursor(None, None);
        }

        if self.generation.get() != generation {
            return;
        }
        let connection = client.as_ref().and_then(|client| {
            client
                .input_method_editing()
                .and_then(|_| client.input_method_connection())
        });
        if client.is_some() {
            self.managed_input.set(connection.is_some());
        }
        *self.connection.borrow_mut() = connection;
        *self.client.borrow_mut() = client;

        self.input_options.apply(self);
        if self.generation.get() != generation {
            return;
        }
        let this = self.this.clone();
        self.system.post(Box::new(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            if this.generation.get() != generation {
                return;
            }
            if this.is_active() {
                this.enable_imm();
            } else {
                // A renderer process have moved its input focus to a password input
                // when there is an ongoing composition, e.g. a user has clicked a
                // mouse button and selected a password input while composing a text.
                // For this case, we have to complete the ongoing composition and
                // clean up the resources attached to this object BEFORE DISABLING THE IME.

                this.disable_imm();
            }
        }));
    }

    fn set_cursor_rect(&self, rect: Rect) {
        let focused = self.system.get_active_window() == self.hwnd.get();

        if !focused {
            return;
        }

        let this = self.this.clone();
        let generation = self.generation.get();
        self.system.post(Box::new(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            if this.generation.get() != generation {
                return;
            }
            let Some(context) = this.acquire_context(this.hwnd.get()) else {
                return;
            };
            if this.generation.get() != generation {
                return;
            }
            this.move_ime_window(rect, context.handle);
        }));
    }

    fn set_options(&self, options: &TextInputOptions) {
        self.input_options.update(Some(options.clone()));
        self.input_options.apply(self);
    }

    fn reset(&self) {
        let Some((generation, managed)) = self.prepare_reset() else {
            return;
        };
        if managed {
            // Cancel while the original native context is still associated;
            // disabling a window must not queue cancellation after detaching it.
            self.reset_native(generation, true);
        } else {
            let this = self.this.clone();
            self.system.post(Box::new(move || {
                if let Some(this) = this.upgrade() {
                    this.reset_native(generation, false);
                }
            }));
        }
    }
}

impl Imm32InputMethod {
    fn prepare_reset(&self) -> Option<(u64, bool)> {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let managed = self.managed_input.get();
        self.completion_rejected.set(false);
        self.composition_session.take();
        if managed {
            self.is_composing.set(false);
            self.set_composition(None);
            if let Some(client) = self.client() {
                client.set_preedit_text_with_cursor(None, None);
            }
        }
        if self.generation.get() != generation {
            return None;
        }
        self.composition_cursor_position.set(None);
        Some((generation, managed))
    }

    fn reset_native(&self, generation: u64, managed: bool) {
        if self.generation.get() != generation
            || (managed && self.native_composition_cancelled.get())
        {
            return;
        }
        let Some(context) = self.acquire_context(self.hwnd.get()) else {
            return;
        };
        if self.generation.get() != generation {
            return;
        }
        if !managed && self.is_composing.get() {
            self.ignore_composition.set(true);
        }
        if let Some(parent) = self.parent() {
            parent.set_ignore_wm_char(true);
        }
        if self.generation.get() != generation {
            return;
        }
        if managed {
            // Publish before a native callback can start a replacement session.
            self.native_composition_cancelled.set(true);
        }
        self.system.imm_notify_ime(
            context.handle,
            NI_COMPOSITIONSTR,
            if managed { CPS_CANCEL } else { CPS_COMPLETE },
            0,
        );
        if self.generation.get() != generation {
            return;
        }
        self.is_composing.set(false);
        self.set_composition(None);
        self.composition_cursor_position.set(None);
    }
}

impl Drop for Imm32InputMethod {
    fn drop(&mut self) {
        self.input_options.dispose(&*self.system);
        self.caret_manager.try_destroy(&*self.system);
        let contexts = self.contexts.take();
        for context in contexts {
            context.dispose();
        }
    }
}

#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use imp::Win32ImmSystem;

#[cfg(windows)]
mod imp {
    use super::{Imm32InputMethod, ImmSystem};
    use crate::interop::unmanaged_methods as native;
    use crate::interop::unmanaged_methods::{CANDIDATEFORM, COMPOSITIONFORM};
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use std::rc::Rc;

    /// The system.
    pub(crate) struct Win32ImmSystem;

    impl ImmSystem for Win32ImmSystem {
        fn imm_get_context(&self, hwnd: isize) -> isize {
            native::imm_get_context(hwnd)
        }

        fn imm_create_context(&self) -> isize {
            native::imm_create_context()
        }

        fn imm_destroy_context(&self, himc: isize) {
            native::imm_destroy_context(himc);
        }

        fn imm_associate_context(&self, hwnd: isize, himc: isize) -> isize {
            native::imm_associate_context(hwnd, himc)
        }

        fn imm_release_context(&self, hwnd: isize, himc: isize) {
            native::imm_release_context(hwnd, himc);
        }

        fn imm_notify_ime(&self, himc: isize, action: i32, index: i32, value: i32) {
            native::imm_notify_ime(himc, action, index, value);
        }

        fn imm_set_candidate_window(&self, himc: isize, candidate: &CANDIDATEFORM) {
            native::imm_set_candidate_window(himc, candidate);
        }

        fn imm_set_composition_window(&self, himc: isize, form: &COMPOSITIONFORM) {
            native::imm_set_composition_window(himc, form);
        }

        fn imm_set_composition_font(&self, himc: isize, height: i32, quality: u8) {
            native::imm_set_composition_font(himc, height, quality);
        }

        fn imm_get_composition_string(&self, himc: isize, flag: u32) -> Option<String> {
            native::imm_get_composition_string(himc, flag)
        }

        fn imm_get_composition_value(&self, himc: isize, flag: u32) -> i32 {
            native::imm_get_composition_value(himc, flag)
        }

        fn create_caret(&self, hwnd: isize, width: i32, height: i32) -> bool {
            native::create_caret(hwnd, width, height)
        }

        fn set_caret_pos(&self, x: i32, y: i32) {
            native::set_caret_pos(x, y);
        }

        fn destroy_caret(&self) {
            native::destroy_caret();
        }

        fn get_active_window(&self) -> isize {
            native::get_active_window()
        }

        fn set_input_scopes(&self, hwnd: isize, scopes: &[i32]) -> bool {
            native::set_input_scopes(hwnd, scopes)
        }

        fn next_message_is_composition_result(&self, hwnd: isize) -> bool {
            native::next_message_is_composition_result(hwnd)
        }
        fn map_window_rect(
            &self,
            from: isize,
            to: isize,
            rect: super::RECT,
        ) -> Option<super::RECT> {
            native::map_window_rect(from, to, rect)
        }

        fn post(&self, action: Box<dyn FnOnce()>) {
            Dispatcher::ui_thread().post_local(action, DispatcherPriority::default());
        }
    }

    impl Imm32InputMethod {
        //Dependant on CurrentThread. When the toolkit will support Multiple Dispatchers -
        //every Dispatcher should have their own InputMethod.
        /// The input method of the calling thread (`Current`).
        pub fn current() -> Rc<Imm32InputMethod> {
            ferroui_base::ferro_thread_local! {
                static CURRENT: Rc<Imm32InputMethod> = Imm32InputMethod::new(Rc::new(Win32ImmSystem));
            }
            CURRENT.with(Rc::clone)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from upstream: the reference has no tests of the input method.
    use super::*;
    use ferroui_base::input::text_input::{TextInputMethodClientEvents, TextSelection};
    use ferroui_base::{Ref, Visual};

    #[derive(Clone, Debug, PartialEq)]
    pub(crate) enum Call {
        GetContext(isize),
        CreateContext,
        DestroyContext(isize),
        AssociateContext(isize, isize),
        ReleaseContext(isize, isize),
        NotifyIme(isize, i32, i32, i32),
        SetCandidateWindow(isize, CANDIDATEFORM),
        SetCompositionWindow(isize, COMPOSITIONFORM),
        SetCompositionFont(isize, i32, u8),
        CreateCaret(isize, i32, i32),
        SetCaretPos(i32, i32),
        DestroyCaret,
        SetInputScopes(isize, Vec<i32>),
    }

    /// The system as a record of the calls made of it, with the answers a
    /// test gives it.
    pub(crate) struct FakeSystem {
        pub calls: RefCell<Vec<Call>>,
        pub posted: RefCell<Vec<Box<dyn FnOnce()>>>,
        /// The default association of a window not explicitly changed yet.
        pub context: Cell<isize>,
        pub associations: RefCell<std::collections::BTreeMap<isize, isize>>,
        pub borrows: RefCell<std::collections::BTreeMap<(isize, isize), usize>>,
        created_contexts: RefCell<std::collections::BTreeSet<isize>>,
        pub next_created_context: Cell<isize>,
        pub next_hook: RefCell<Option<(fn(&Call) -> bool, Box<dyn FnOnce()>)>>,
        pub active_window: Cell<isize>,
        pub refuse_caret: Cell<bool>,
        pub composition_string: RefCell<Option<String>>,
        pub result_string: RefCell<Option<String>>,
        pub cursor: Cell<i32>,
        pub next_result: Cell<bool>,
        pub refuse_input_scopes: Cell<bool>,
        pub complete_input_scopes_after_callback: Cell<bool>,
        pub input_scopes: RefCell<std::collections::BTreeMap<isize, Vec<i32>>>,
    }

    impl FakeSystem {
        pub fn new() -> Rc<FakeSystem> {
            Rc::new(FakeSystem {
                calls: RefCell::new(Vec::new()),
                posted: RefCell::new(Vec::new()),
                context: Cell::new(0x1C),
                associations: RefCell::new(std::collections::BTreeMap::new()),
                borrows: RefCell::new(std::collections::BTreeMap::new()),
                created_contexts: RefCell::new(std::collections::BTreeSet::new()),
                next_created_context: Cell::new(0x2C),
                next_hook: RefCell::new(None),
                active_window: Cell::new(0),
                refuse_caret: Cell::new(false),
                composition_string: RefCell::new(None),
                result_string: RefCell::new(None),
                cursor: Cell::new(-1),
                next_result: Cell::new(false),
                refuse_input_scopes: Cell::new(false),
                complete_input_scopes_after_callback: Cell::new(false),
                input_scopes: RefCell::new(std::collections::BTreeMap::new()),
            })
        }

        pub fn take_calls(&self) -> Vec<Call> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }

        /// Runs what was posted, and what that posts.
        pub fn run_posted(&self) {
            loop {
                let posted = std::mem::take(&mut *self.posted.borrow_mut());
                if posted.is_empty() {
                    return;
                }
                for action in posted {
                    action();
                }
            }
        }

        fn record(&self, call: Call) {
            self.calls.borrow_mut().push(call.clone());
            let hook = self.next_hook.take();
            if let Some((matches, action)) = hook {
                if matches(&call) {
                    action();
                } else {
                    *self.next_hook.borrow_mut() = Some((matches, action));
                }
            }
        }
    }

    impl ImmSystem for FakeSystem {
        fn imm_get_context(&self, hwnd: isize) -> isize {
            let handle = if hwnd == 0 {
                0
            } else {
                self.associations
                    .borrow()
                    .get(&hwnd)
                    .copied()
                    .unwrap_or(self.context.get())
            };
            if handle != 0 {
                *self.borrows.borrow_mut().entry((hwnd, handle)).or_default() += 1;
            }
            self.record(Call::GetContext(hwnd));
            handle
        }

        fn imm_create_context(&self) -> isize {
            let handle = self.next_created_context.get();
            if handle != 0 {
                self.next_created_context.set(handle + 1);
                assert!(self.created_contexts.borrow_mut().insert(handle));
            }
            self.record(Call::CreateContext);
            handle
        }

        fn imm_destroy_context(&self, himc: isize) {
            assert!(
                self.created_contexts.borrow_mut().remove(&himc),
                "destroying a context not owned by the transport or destroying twice"
            );
            assert!(
                self.associations
                    .borrow()
                    .values()
                    .all(|handle| *handle != himc),
                "destroying an associated context"
            );
            assert!(
                self.borrows
                    .borrow()
                    .iter()
                    .all(|((_, handle), count)| *handle != himc || *count == 0),
                "destroying a borrowed context"
            );
            self.record(Call::DestroyContext(himc));
        }

        fn imm_associate_context(&self, hwnd: isize, himc: isize) -> isize {
            let previous = self
                .associations
                .borrow_mut()
                .insert(hwnd, himc)
                .unwrap_or(self.context.get());
            self.record(Call::AssociateContext(hwnd, himc));
            previous
        }

        fn imm_release_context(&self, hwnd: isize, himc: isize) {
            {
                let mut borrows = self.borrows.borrow_mut();
                let count = borrows
                    .get_mut(&(hwnd, himc))
                    .expect("releasing an unacquired context");
                assert!(*count > 0, "releasing a context twice");
                *count -= 1;
            }
            self.record(Call::ReleaseContext(hwnd, himc));
        }

        fn imm_notify_ime(&self, himc: isize, action: i32, index: i32, value: i32) {
            self.record(Call::NotifyIme(himc, action, index, value));
        }

        fn imm_set_candidate_window(&self, himc: isize, candidate: &CANDIDATEFORM) {
            self.record(Call::SetCandidateWindow(himc, *candidate));
        }

        fn imm_set_composition_window(&self, himc: isize, form: &COMPOSITIONFORM) {
            self.record(Call::SetCompositionWindow(himc, *form));
        }

        fn imm_set_composition_font(&self, himc: isize, height: i32, quality: u8) {
            self.record(Call::SetCompositionFont(himc, height, quality));
        }

        fn imm_get_composition_string(&self, _himc: isize, flag: u32) -> Option<String> {
            match flag {
                GCS::GCS_COMPSTR => self.composition_string.borrow().clone(),
                GCS::GCS_RESULTSTR => self.result_string.borrow().clone(),
                _ => None,
            }
        }

        fn imm_get_composition_value(&self, _himc: isize, _flag: u32) -> i32 {
            self.cursor.get()
        }

        fn create_caret(&self, hwnd: isize, width: i32, height: i32) -> bool {
            self.record(Call::CreateCaret(hwnd, width, height));
            !self.refuse_caret.get()
        }

        fn set_caret_pos(&self, x: i32, y: i32) {
            self.record(Call::SetCaretPos(x, y));
        }

        fn destroy_caret(&self) {
            self.record(Call::DestroyCaret);
        }

        fn get_active_window(&self) -> isize {
            self.active_window.get()
        }

        fn set_input_scopes(&self, hwnd: isize, scopes: &[i32]) -> bool {
            let success = !self.refuse_input_scopes.get();
            let delayed = self.complete_input_scopes_after_callback.get();
            let apply = || {
                if success {
                    if scopes == [0] {
                        self.input_scopes.borrow_mut().remove(&hwnd);
                    } else {
                        self.input_scopes.borrow_mut().insert(hwnd, scopes.to_vec());
                    }
                }
            };
            if !delayed {
                apply();
            }
            self.record(Call::SetInputScopes(hwnd, scopes.to_vec()));
            if delayed {
                apply();
            }
            success
        }

        fn next_message_is_composition_result(&self, _hwnd: isize) -> bool {
            self.next_result.get()
        }

        fn post(&self, action: Box<dyn FnOnce()>) {
            self.posted.borrow_mut().push(action);
        }
    }

    #[derive(Default)]
    struct FakeParent {
        scaling: Cell<f64>,
        no_input_callback: Cell<bool>,
        ignore_wm_char: Cell<Option<bool>>,
        text: RefCell<Vec<(u64, String)>>,
        keys: RefCell<Vec<(Key, PhysicalKey)>>,
    }

    impl Imm32Parent for FakeParent {
        fn hwnd(&self) -> isize {
            HWND
        }
        fn desktop_scaling(&self) -> f64 {
            self.scaling.get()
        }

        fn set_ignore_wm_char(&self, value: bool) {
            self.ignore_wm_char.set(Some(value));
        }

        fn raw_text_input(&self, timestamp: u64, text: &str) -> bool {
            if self.no_input_callback.get() {
                return false;
            }
            self.text.borrow_mut().push((timestamp, text.to_owned()));
            true
        }

        fn raw_key_press(&self, key: Key, physical_key: PhysicalKey) {
            self.keys.borrow_mut().push((key, physical_key));
        }
    }

    /// A text input client that records the preedit text it is given.
    pub(crate) struct Client {
        events: TextInputMethodClientEvents,
        supports_preedit: bool,
        supports_surrounding_text: bool,
        selection: Cell<TextSelection>,
        preedit: RefCell<Vec<(Option<String>, Option<i32>)>>,
    }

    impl Client {
        pub(crate) fn new() -> Rc<Client> {
            Rc::new(Client {
                events: TextInputMethodClientEvents::new(),
                supports_preedit: true,
                supports_surrounding_text: true,
                selection: Cell::new(TextSelection::new(0, 0)),
                preedit: RefCell::new(Vec::new()),
            })
        }

        fn take_preedit(&self) -> Vec<(Option<String>, Option<i32>)> {
            std::mem::take(&mut *self.preedit.borrow_mut())
        }
    }

    impl TextInputMethodClient for Client {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Option<Ref<Visual>> {
            None
        }

        fn supports_preedit(&self) -> bool {
            self.supports_preedit
        }

        fn supports_surrounding_text(&self) -> bool {
            self.supports_surrounding_text
        }

        fn surrounding_text(&self) -> String {
            String::new()
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            self.selection.get()
        }

        fn set_selection(&self, value: TextSelection) {
            self.selection.set(value);
        }

        fn set_preedit_text_with_cursor(
            &self,
            preedit_text: Option<&str>,
            cursor_pos: Option<i32>,
        ) {
            self.preedit
                .borrow_mut()
                .push((preedit_text.map(str::to_owned), cursor_pos));
        }
    }

    const HWND: isize = 0x77;
    /// A keyboard layout of Japanese (0x0411) and one of Korean (0x0412).
    const HKL_JA: isize = 0x0411_0411;
    const HKL_KO: isize = 0x0412_0412;

    struct Setup {
        system: Rc<FakeSystem>,
        parent: Rc<FakeParent>,
        input_method: Rc<Imm32InputMethod>,
        client: Rc<Client>,
    }

    /// An input method of a window with a client, enabled.
    fn setup(hkl: isize) -> Setup {
        let system = FakeSystem::new();
        let parent = Rc::new(FakeParent::default());
        parent.scaling.set(1.0);
        let input_method = Imm32InputMethod::new(system.clone());
        let weak: Weak<dyn Imm32Parent> = Rc::downgrade(&(parent.clone() as Rc<dyn Imm32Parent>));
        input_method.set_language_and_window(weak, HWND, hkl);
        let client = Client::new();
        input_method.set_client(Some(client.clone()));
        system.run_posted();
        system.take_calls();
        Setup {
            system,
            parent,
            input_method,
            client,
        }
    }

    #[test]
    fn the_language_of_a_keyboard_layout_is_the_low_word_and_its_primary_language_the_low_ten_bits()
    {
        assert_eq!(0x0411, lgid(HKL_JA));
        assert_eq!(0x11, primary_lang_id(lgid(HKL_JA)));
        assert_eq!(LANG_KO as u16, primary_lang_id(lgid(HKL_KO)));
        // A layout handle of a 64-bit system with the sign extended.
        assert_eq!(0x0804, lgid(0xFFFF_FFFF_E008_0804_u64 as isize));
        assert_eq!(0x04, primary_lang_id(0x0804));
    }

    #[test]
    fn the_forms_have_the_layout_of_the_system() {
        assert_eq!(32, std::mem::size_of::<CANDIDATEFORM>());
        assert_eq!(28, std::mem::size_of::<COMPOSITIONFORM>());
        assert_eq!(16, std::mem::offset_of!(CANDIDATEFORM, rcArea));
        assert_eq!(12, std::mem::offset_of!(COMPOSITIONFORM, rcArea));
    }

    #[test]
    fn the_cursor_rectangle_is_scaled_to_pixels_and_truncated() {
        assert_eq!(
            (10, 20, 12, 36),
            cursor_rect_in_pixels(Rect::new(10.0, 20.0, 2.0, 16.0), 1.0)
        );
        assert_eq!(
            (15, 30, 18, 54),
            cursor_rect_in_pixels(Rect::new(10.0, 20.0, 2.0, 16.0), 1.5)
        );
        assert_eq!(
            (21, 41, 25, 73),
            cursor_rect_in_pixels(Rect::new(10.6, 20.6, 2.0, 16.0), 2.0)
        );
    }

    #[test]
    fn the_candidate_window_keeps_clear_of_the_caret_and_of_a_line_more_for_korean() {
        let japanese = candidate_exclude_rectangle(10, 20, 12, 36, 0x11);
        assert_eq!(CFS_EXCLUDE, japanese.dwStyle);
        assert_eq!(0, japanese.dwIndex);
        assert_eq!(POINT { x: 10, y: 20 }, japanese.ptCurrentPos);
        assert_eq!(
            RECT {
                left: 10,
                top: 20,
                right: 12,
                bottom: 37
            },
            japanese.rcArea
        );

        let korean = candidate_exclude_rectangle(10, 20, 12, 36, LANG_KO as u16);
        assert_eq!(
            RECT {
                left: 10,
                top: 20,
                right: 12,
                bottom: 38
            },
            korean.rcArea
        );
    }

    #[test]
    fn a_client_enables_the_input_context_of_the_window_and_creates_the_caret() {
        let system = FakeSystem::new();
        let input_method = Imm32InputMethod::new(system.clone());
        let parent = Rc::new(FakeParent::default());
        let weak: Weak<dyn Imm32Parent> = Rc::downgrade(&(parent.clone() as Rc<dyn Imm32Parent>));
        input_method.set_language_and_window(weak, HWND, HKL_JA);
        assert!(system.take_calls().is_empty());

        input_method.set_client(Some(Client::new()));
        // Nothing before the dispatcher runs the posted action.
        assert!(system.take_calls().is_empty());
        system.run_posted();

        assert_eq!(
            vec![
                Call::GetContext(HWND),
                Call::ReleaseContext(HWND, 0x1C),
                Call::CreateCaret(HWND, 2, 2)
            ],
            system.take_calls()
        );

        // A repeated acquisition still needs its matching release.
        input_method.enable_imm();
        assert_eq!(
            vec![Call::GetContext(HWND), Call::ReleaseContext(HWND, 0x1C)],
            system.take_calls()
        );
    }

    #[test]
    fn a_window_without_an_input_context_gets_a_new_one() {
        let system = FakeSystem::new();
        system.context.set(0);
        let input_method = Imm32InputMethod::new(system.clone());
        input_method.hwnd.set(HWND);

        input_method.enable_imm();

        assert_eq!(
            vec![
                Call::GetContext(HWND),
                Call::CreateContext,
                Call::AssociateContext(HWND, 0x2C),
                Call::CreateCaret(HWND, 2, 2)
            ],
            system.take_calls()
        );
    }

    #[test]
    fn no_client_completes_the_composition_and_takes_the_input_context_from_the_window() {
        let Setup {
            system,
            parent,
            input_method,
            client,
        } = setup(HKL_JA);
        input_method.handle_composition_start();
        client.take_preedit();

        input_method.set_client(None);
        // The old client loses its preedit text at once.
        assert_eq!(vec![(None, None)], client.take_preedit());
        system.run_posted();

        assert_eq!(
            vec![
                // Complete while the context is still associated with its HWND.
                Call::GetContext(HWND),
                Call::NotifyIme(0x1C, NI_COMPOSITIONSTR, CPS_COMPLETE, 0),
                Call::ReleaseContext(HWND, 0x1C),
                Call::DestroyCaret,
                Call::GetContext(HWND),
                Call::ReleaseContext(HWND, 0x1C),
                Call::AssociateContext(HWND, 0),
            ],
            system.take_calls()
        );
        assert!(!input_method.is_composing());
        assert_eq!(Some(true), parent.ignore_wm_char.get());
        // The WM_IME_COMPOSITION message the completion sends is not a
        // composition of the next client.
        *system.result_string.borrow_mut() = Some("x".to_owned());
        input_method.handle_composition(0, GCS::GCS_RESULTSTR as isize, 1);
        assert!(parent.text.borrow().is_empty());
    }

    #[test]
    fn the_composition_string_reaches_the_client_with_the_cursor_position() {
        let Setup {
            system,
            parent,
            input_method,
            client,
        } = setup(HKL_JA);

        input_method.handle_composition_start();
        assert!(input_method.is_composing());
        assert_eq!(vec![(None, None)], client.take_preedit());
        // No selection: nothing is deleted.
        assert!(parent.keys.borrow().is_empty());

        *system.composition_string.borrow_mut() = Some("にほ".to_owned());
        system.cursor.set(2);
        input_method.handle_composition(0, (GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) as isize, 10);
        assert_eq!(
            vec![(Some("にほ".to_owned()), Some(2))],
            client.take_preedit()
        );
        assert_eq!(Some("にほ".to_owned()), input_method.composition());

        // The cursor alone moves: the string is the one kept.
        system.cursor.set(1);
        input_method.handle_composition(0, GCS::GCS_CURSORPOS as isize, 11);
        assert_eq!(
            vec![(Some("にほ".to_owned()), Some(1))],
            client.take_preedit()
        );

        // The string alone changes: the cursor is the one kept.
        *system.composition_string.borrow_mut() = Some("にほん".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 12);
        assert_eq!(
            vec![(Some("にほん".to_owned()), Some(1))],
            client.take_preedit()
        );

        // A cursor position the system cannot give is no position.
        system.cursor.set(-1);
        input_method.handle_composition(0, (GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) as isize, 13);
        assert_eq!(
            vec![(Some("にほん".to_owned()), None)],
            client.take_preedit()
        );
        assert!(parent.text.borrow().is_empty());
    }

    #[test]
    fn the_result_string_is_raw_text_input_and_the_next_character_message_is_ignored() {
        let Setup {
            system,
            parent,
            input_method,
            client,
        } = setup(HKL_JA);
        input_method.handle_composition_start();
        *system.composition_string.borrow_mut() = Some("にほん".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);
        client.take_preedit();

        *system.result_string.borrow_mut() = Some("日本".to_owned());
        *system.composition_string.borrow_mut() = None;
        // The flags of a result as a Japanese input method sends them:
        // the result string, and the cursor of a composition that is over.
        input_method.handle_composition(0, (GCS::GCS_RESULTSTR | GCS::GCS_CURSORPOS) as isize, 42);

        assert_eq!(vec![(42, "日本".to_owned())], *parent.text.borrow());
        assert_eq!(Some(true), parent.ignore_wm_char.get());
        // The preedit text is taken away, and not set again.
        assert_eq!(vec![(None, None)], client.take_preedit());
        assert_eq!(None, input_method.composition());

        // The end of the composition has nothing left to commit.
        input_method.handle_composition_end(43);
        assert!(!input_method.is_composing());
        assert_eq!(1, parent.text.borrow().len());
        assert_eq!(vec![(None, None)], client.take_preedit());
    }

    #[test]
    fn a_result_together_with_a_new_composition_commits_and_goes_on_composing() {
        // A Korean input method commits a syllable and starts the next in
        // one message.
        let Setup {
            system,
            parent,
            input_method,
            client,
        } = setup(HKL_KO);
        input_method.handle_composition_start();
        client.take_preedit();

        *system.result_string.borrow_mut() = Some("한".to_owned());
        *system.composition_string.borrow_mut() = Some("ㄱ".to_owned());
        system.cursor.set(1);
        input_method.handle_composition(
            0,
            (GCS::GCS_RESULTSTR | GCS::GCS_COMPSTR | GCS::GCS_CURSORPOS) as isize,
            5,
        );

        assert_eq!(vec![(5, "한".to_owned())], *parent.text.borrow());
        assert_eq!(
            vec![(None, None), (Some("ㄱ".to_owned()), Some(1))],
            client.take_preedit()
        );
    }

    #[test]
    fn a_composition_that_ends_with_text_left_commits_it() {
        let Setup {
            system,
            parent,
            input_method,
            client,
        } = setup(HKL_KO);
        input_method.handle_composition_start();
        *system.composition_string.borrow_mut() = Some("가".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);
        client.take_preedit();

        input_method.handle_composition_end(9);

        assert_eq!(vec![(9, "가".to_owned())], *parent.text.borrow());
        assert_eq!(Some(true), parent.ignore_wm_char.get());
        assert_eq!(vec![(None, None)], client.take_preedit());
        assert_eq!(None, input_method.composition());
        assert!(!input_method.is_composing());
    }

    #[test]
    fn a_window_without_an_input_callback_commits_nothing_and_ignores_no_character() {
        let Setup {
            system,
            parent,
            input_method,
            ..
        } = setup(HKL_KO);
        parent.no_input_callback.set(true);
        input_method.handle_composition_start();
        *system.composition_string.borrow_mut() = Some("가".to_owned());
        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);

        input_method.handle_composition_end(9);

        assert_eq!(None, parent.ignore_wm_char.get());
    }

    #[test]
    fn a_composition_message_without_flags_empties_the_preedit_text() {
        let Setup {
            input_method,
            client,
            ..
        } = setup(HKL_JA);
        input_method.handle_composition_start();
        client.take_preedit();

        input_method.handle_composition(0, 0, 1);

        assert_eq!(vec![(Some(String::new()), None)], client.take_preedit());
    }

    #[test]
    fn the_start_of_a_composition_deletes_the_selection_of_a_client_with_surrounding_text() {
        let Setup {
            parent,
            input_method,
            client,
            ..
        } = setup(HKL_JA);
        client.selection.set(TextSelection::new(2, 5));

        input_method.handle_composition_start();

        assert_eq!(
            vec![(Key::Delete, PhysicalKey::Delete)],
            *parent.keys.borrow()
        );
    }

    #[test]
    fn a_client_without_preedit_is_told_no_composition_string() {
        let system = FakeSystem::new();
        let input_method = Imm32InputMethod::new(system.clone());
        input_method.hwnd.set(HWND);
        let client = Rc::new(Client {
            supports_preedit: false,
            ..Rc::try_unwrap(Client::new()).ok().expect("one owner")
        });
        input_method.set_client(Some(client.clone()));
        input_method.set_is_composing(true);
        *system.composition_string.borrow_mut() = Some("a".to_owned());

        input_method.handle_composition(0, GCS::GCS_COMPSTR as isize, 1);

        assert!(client.take_preedit().is_empty());
        assert_eq!(Some("a".to_owned()), input_method.composition());
    }

    #[test]
    fn the_cursor_rectangle_moves_the_caret_and_the_candidate_window_of_the_active_window() {
        let Setup {
            system,
            parent,
            input_method,
            ..
        } = setup(HKL_JA);
        parent.scaling.set(2.0);

        // Another window is active: nothing is asked of the system.
        system.active_window.set(0x99);
        input_method.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
        system.run_posted();
        assert!(system.take_calls().is_empty());

        system.active_window.set(HWND);
        input_method.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
        assert!(system.take_calls().is_empty());
        system.run_posted();

        assert_eq!(
            vec![
                Call::GetContext(HWND),
                // The caret goes to the lower right corner of the cursor.
                Call::SetCaretPos(22, 72),
                Call::SetCandidateWindow(
                    0x1C,
                    CANDIDATEFORM {
                        dwIndex: 0,
                        dwStyle: CFS_EXCLUDE,
                        ptCurrentPos: POINT { x: 20, y: 40 },
                        rcArea: RECT {
                            left: 20,
                            top: 40,
                            right: 22,
                            bottom: 73
                        },
                    }
                ),
                Call::ReleaseContext(HWND, 0x1C),
            ],
            system.take_calls()
        );
    }

    #[test]
    fn a_change_of_the_language_makes_the_input_context_again() {
        let Setup {
            system,
            parent,
            input_method,
            ..
        } = setup(HKL_JA);
        let weak: Weak<dyn Imm32Parent> = Rc::downgrade(&(parent.clone() as Rc<dyn Imm32Parent>));

        // The same primary language: nothing.
        input_method.set_language_and_window(weak.clone(), HWND, HKL_JA);
        assert!(system.take_calls().is_empty());

        input_method.set_language_and_window(weak, HWND, HKL_KO);
        let calls = system.take_calls();
        assert_eq!(
            vec![
                Call::GetContext(HWND),
                Call::NotifyIme(0x1C, NI_COMPOSITIONSTR, CPS_COMPLETE, 0),
                Call::ReleaseContext(HWND, 0x1C),
                Call::DestroyCaret,
                Call::GetContext(HWND),
                Call::ReleaseContext(HWND, 0x1C),
                Call::AssociateContext(HWND, 0),
                Call::GetContext(HWND),
                Call::AssociateContext(HWND, 0x1C),
                Call::CreateCaret(HWND, 2, 2),
            ],
            calls
        );
    }

    #[test]
    fn a_window_that_is_destroyed_gives_up_the_input_method() {
        let Setup {
            system,
            input_method,
            ..
        } = setup(HKL_JA);
        input_method.handle_composition_start();

        input_method.clear_language_and_window();

        assert_eq!(0, input_method.hwnd());
        assert!(!input_method.is_active());
        assert!(!input_method.is_composing());
        assert_eq!(
            vec![
                Call::DestroyCaret,
                Call::GetContext(HWND),
                Call::NotifyIme(0x1C, NI_COMPOSITIONSTR, CPS_CANCEL, 0),
                Call::ReleaseContext(HWND, 0x1C),
            ],
            system.take_calls()
        );
        // Cleanup never posts a reset against a null or replacement HWND.
        system.context.set(0);
        system.run_posted();
        assert!(system.take_calls().is_empty());
    }
}

#[cfg(test)]
#[path = "imm32_rich_text_tests.rs"]
mod rich_text_tests;

#[cfg(test)]
#[path = "imm32_context_tests.rs"]
mod context_tests;
