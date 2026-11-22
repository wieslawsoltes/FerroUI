//! Worker-side facade for `zwp_text_input_v3` (the port of
//! `WaylandTextInputV3.cs`). Owns the per-seat protocol objects and the
//! per-surface state cache. Bridges incoming compositor events into the
//! broker of the UI thread via `IWaylandTextInputV3Events`.
//!
//! All methods must be called on the Wayland worker thread (posted from the
//! UI thread by the proxy of a surface).
//!
//! How the port is written. The reference holds the generated protocol
//! objects and is itself their listener. Here the facade is written against
//! three small contracts, so that the whole of it (the enable and disable
//! rules, the done and serial protocol, the conversions between bytes and
//! characters) runs in tests on every system, without a compositor:
//!
//! - [`ITextInputV3`]: the requests of a `zwp_text_input_v3` object;
//! - [`ITextInputManagerV3`]: the manager, which makes one per seat;
//! - [`ITextInputSurfaces`]: what the facade reads of a surface of the worker
//!   (the reference reads them of the `WSurface` object it keys its state by;
//!   the port keys by the number of the surface).
//!
//! The listener of the reference (`SeatListener`) is the `on_*` methods; on
//! Linux the `Dispatch` implementation of the protocol object calls them
//! (the module `imp` at the end, with the implementations of the contracts
//! over the bindings).
//!
//! One difference of behaviour, for compositors that take the protocol at its
//! word (wlroots; measured under sway 1.9). The state a client sends with the
//! commit that enables it is what the compositor supports for that session:
//! "the client must set the features it supports in the same commit as
//! enable", and wlroots ignores a surrounding text, a content type or a cursor
//! rectangle of a later commit when the enabling one had none. The reference
//! enables at once when the client is set (`SetActive`), before the UI thread
//! has sent the state of the client, so on such a compositor the input method
//! never hears of the text around the cursor and cannot place its popup. Here
//! `set_active` only marks the surface, and [`WaylandTextInputV3::flush`],
//! which the worker calls after it ran the commands of the UI thread, sends
//! `enable` with everything that arrived with it, in one commit. A kind of
//! state that arrives for the first time after that is sent by enabling again
//! (the sequence of a reset), also at the flush.
//!
//! One difference of representation: the options of a text input client hold
//! a reference counted list (the locale hints) and cannot cross to the
//! worker, so the proxy converts them on the UI thread and the facade keeps
//! the converted pair where the reference keeps the options.

use super::i_wayland_text_input_v3_events::IWaylandTextInputV3Events;
use crate::server::persistent::w_surface::WSurfaceId;
use crate::text_input_options_converter::{ContentHint, ContentPurpose};
use crate::wayland_text_utils;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::{Rect, Thickness};
use std::collections::{BTreeMap, HashMap};

/// `zwp_text_input_v3.change_cause`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ChangeCause {
    InputMethod = 0,
    Other = 1,
}

/// The requests of a `zwp_text_input_v3` object.
pub trait ITextInputV3 {
    fn enable(&self);
    fn disable(&self);
    fn set_content_type(&self, hint: ContentHint, purpose: ContentPurpose);
    fn set_cursor_rectangle(&self, x: i32, y: i32, width: i32, height: i32);
    fn set_surrounding_text(&self, text: &str, cursor: i32, anchor: i32);
    fn set_text_change_cause(&self, cause: ChangeCause);
    fn commit(&self);
    fn destroy(&self);
}

/// The `zwp_text_input_manager_v3` global.
pub trait ITextInputManagerV3 {
    /// A `wl_seat` of the connection.
    type Seat;
    type TextInput: ITextInputV3;

    /// `get_text_input`: the text input object of a seat. Its events are the `on_*` methods
    /// of the facade, with the registry name of the seat.
    fn get_text_input(&self, seat_global_name: u32, seat: &Self::Seat) -> Self::TextInput;
    fn destroy(&self);
}

/// What the facade reads of a surface of the worker.
pub trait ITextInputSurfaces {
    /// The shadow extents of a shell surface (`WXdgShellSurface.ShadowExtents`).
    fn shadow_extents(&self, surface: WSurfaceId) -> Thickness;
    /// `WSurface.TextInputSessionToken`.
    fn text_input_session_token(&self, surface: WSurfaceId) -> i32;
}

/// The state of the text input of a surface as a test sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputSurfaceSnapshot {
    pub surface: WSurfaceId,
    /// The registry names of the seats whose text input entered the surface, in the order
    /// they entered.
    pub entered_seats: Vec<u32>,
    pub has_client: bool,
    pub last_sent_enabled: bool,
    /// The cursor rectangle as it is sent to the compositor (`set_cursor_rectangle`).
    pub cursor_rectangle: Option<(i32, i32, i32, i32)>,
    pub content_type: Option<(ContentHint, ContentPurpose)>,
    /// The surrounding window prepared for the compositor.
    pub surrounding_text: Option<String>,
}

pub struct WaylandTextInputV3<M: ITextInputManagerV3, S: IWaylandTextInputV3Events> {
    manager: M,
    /// By the registry name of the seat. The reference keeps them in the order they were
    /// added; the order of the names is the same order on a compositor that numbers its
    /// globals upwards, and a stable one everywhere.
    seats: BTreeMap<u32, PerSeat<M::TextInput>>,
    surfaces: HashMap<WSurfaceId, PerSurface<S>>,
}

impl<M: ITextInputManagerV3, S: IWaylandTextInputV3Events> WaylandTextInputV3<M, S> {
    pub fn new(manager: M) -> Self {
        Self { manager, seats: BTreeMap::new(), surfaces: HashMap::new() }
    }

    pub fn on_seat_added(&mut self, seat_global_name: u32, wl_seat: &M::Seat) {
        if self.seats.contains_key(&seat_global_name) {
            return;
        }
        let proxy = self.manager.get_text_input(seat_global_name, wl_seat);
        self.seats.insert(seat_global_name, PerSeat::new(seat_global_name, proxy));
    }

    pub fn on_seat_removed(&mut self, seat_global_name: u32) {
        let Some(seat) = self.seats.remove(&seat_global_name) else {
            return;
        };
        if let Some(per_surface) = seat.focused_surface.and_then(|surface| self.surfaces.get_mut(&surface)) {
            per_surface.entered_seats.retain(|name| *name != seat_global_name);
        }
        seat.proxy.destroy();
    }

    pub fn dispose(&mut self) {
        for seat in self.seats.values() {
            seat.proxy.destroy();
        }
        self.seats.clear();
        self.surfaces.clear();
        self.manager.destroy();
    }

    pub fn register_surface(&mut self, s: WSurfaceId, sink: S) {
        if self.surfaces.contains_key(&s) {
            return;
        }
        let mut ps = PerSurface::new(s, sink);
        // Reconcile: if any seat's text-input-v3 already has focus on this
        // surface (enter event arrived before the broker registered), record
        // it now so a subsequent SetActive(true) can issue Enable+Commit.
        for seat in self.seats.values() {
            if seat.focused_surface == Some(s) {
                ps.entered_seats.push(seat.global_name);
            }
        }
        self.surfaces.insert(s, ps);
    }

    pub fn unregister_surface(&mut self, s: WSurfaceId) {
        let Some(per_surface) = self.surfaces.remove(&s) else {
            return;
        };
        // For each seat currently entered on this surface, send disable+commit
        // so the compositor stops routing IME events to a now-dead surface.
        for name in &per_surface.entered_seats {
            let Some(seat) = self.seats.get_mut(name) else {
                continue;
            };
            if seat.focused_surface == Some(s) {
                seat.focused_surface = None;
            }
            Self::disable_and_clear(seat, &per_surface);
        }
    }

    pub fn set_active(
        &mut self,
        s: WSurfaceId,
        has_client: bool,
        supports_preedit: bool,
        supports_surrounding_text: bool,
    ) {
        let Some(ps) = self.surfaces.get_mut(&s) else {
            return;
        };
        ps.has_client = has_client;
        ps.supports_preedit = supports_preedit;
        ps.supports_surrounding_text = supports_surrounding_text;
        // The reference pushes the state to the entered seats here. The port does it at the
        // flush, with the state of the client that arrives after this call (see the head of
        // the file).
        ps.needs_push = true;
    }

    /// Sends what was held back: `enable` or `disable` for the surfaces whose client changed
    /// since the last flush, and `enable` again for those that got a kind of state the
    /// enabling commit did not have. The worker calls it after the commands of the UI thread.
    pub fn flush(&mut self, surfaces: &dyn ITextInputSurfaces) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let mut pending: Vec<WSurfaceId> =
            per_surfaces.values().filter(|ps| ps.needs_push || ps.needs_reenable).map(|ps| ps.surface).collect();
        pending.sort();
        for s in pending {
            let Some(ps) = per_surfaces.get_mut(&s) else {
                continue;
            };
            if std::mem::take(&mut ps.needs_push) {
                ps.needs_reenable = false;
                Self::push_state_to_entered_seats(seats, s, ps, surfaces);
            } else if std::mem::take(&mut ps.needs_reenable) {
                Self::reset_surface(seats, s, ps, surfaces);
            }
        }
    }

    /// Aborts any in-flight IME composition for this surface: sends
    /// disable+commit on every entered seat and clears the per-seat
    /// last-delivered state so the next preedit is not deduped away.
    /// Also clears cached surrounding text so the next client starts clean.
    /// Called when the broker's text input client changes.
    pub fn abort_composition(&mut self, s: WSurfaceId) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(ps) = per_surfaces.get_mut(&s) else {
            return;
        };
        for name in &ps.entered_seats {
            let Some(seat) = seats.get_mut(name) else {
                continue;
            };
            Self::disable_and_clear(seat, ps);
            seat.clear_last_delivered();
        }
        ps.last_sent_enabled = false;
        ps.surrounding_text = None;
        ps.cursor_rect = None;
        ps.options = None;
        ps.has_client = false;
        ps.declared = Features::default();
        ps.needs_reenable = false;
    }

    pub fn set_cursor_rect(&mut self, s: WSurfaceId, logical: Rect, surfaces: &dyn ITextInputSurfaces) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(ps) = per_surfaces.get_mut(&s) else {
            return;
        };
        ps.cursor_rect = Some(logical);
        if !ps.last_sent_enabled {
            return;
        }
        if !ps.declared.cursor_rectangle {
            ps.needs_reenable = true;
            return;
        }
        let (x, y, w, h) = translate_cursor_rect(surfaces, s, logical);
        for name in &ps.entered_seats {
            let Some(seat) = seats.get_mut(name) else {
                continue;
            };
            seat.proxy.set_cursor_rectangle(x, y, w, h);
            seat.commit();
        }
    }

    /// Sets the content type of the text input of a surface: the options of the client as
    /// `text_input_options_converter::convert` gave them on the UI thread.
    pub fn set_options(&mut self, s: WSurfaceId, options: (ContentHint, ContentPurpose)) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(ps) = per_surfaces.get_mut(&s) else {
            return;
        };
        ps.options = Some(options);
        if !ps.last_sent_enabled {
            return;
        }
        if !ps.declared.content_type {
            ps.needs_reenable = true;
            return;
        }
        let (hint, purpose) = options;
        for name in &ps.entered_seats {
            let Some(seat) = seats.get_mut(name) else {
                continue;
            };
            seat.proxy.set_content_type(hint, purpose);
            seat.commit();
        }
    }

    pub fn set_surrounding_text(&mut self, s: WSurfaceId, text: String, cursor_char: i32, anchor_char: i32) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(ps) = per_surfaces.get_mut(&s) else {
            return;
        };
        ps.surrounding_text = surrounding_window(&text, cursor_char, anchor_char)
            .map(|(text, cursor, anchor)| SurroundingText { text: text.to_owned(), cursor, anchor });
        if !ps.last_sent_enabled || !ps.supports_surrounding_text {
            return;
        }
        if ps.declared.surrounding_text != ps.surrounding_text.is_some() {
            ps.needs_reenable = true;
            return;
        }
        let Some(surrounding) = &ps.surrounding_text else {
            return;
        };
        for name in &ps.entered_seats {
            let Some(seat) = seats.get_mut(name) else {
                continue;
            };
            seat.proxy.set_surrounding_text(&surrounding.text, surrounding.cursor, surrounding.anchor);
            seat.proxy.set_text_change_cause(ChangeCause::Other);
            seat.commit();
        }
    }

    pub fn reset(&mut self, s: WSurfaceId, surfaces: &dyn ITextInputSurfaces) {
        // v3 has no reset request. Closest safe behaviour: drop pending preedit
        // by issuing disable + enable + commit. Compositor treats this as a fresh
        // session and discards in-flight preedit.
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(ps) = per_surfaces.get_mut(&s) else {
            return;
        };
        Self::reset_surface(seats, s, ps, surfaces);
    }

    fn reset_surface(
        seats: &mut BTreeMap<u32, PerSeat<M::TextInput>>,
        s: WSurfaceId,
        ps: &mut PerSurface<S>,
        surfaces: &dyn ITextInputSurfaces,
    ) {
        if !ps.has_client {
            return;
        }
        ps.declared = Features::of(ps);
        let mut had_delivered_preedit = false;
        for name in &ps.entered_seats {
            let Some(seat) = seats.get_mut(name) else {
                continue;
            };
            if seat.last_delivered_preedit.as_deref().is_some_and(|preedit| !preedit.is_empty()) {
                had_delivered_preedit = true;
            }
            seat.clear_pending();
            seat.clear_last_delivered();
            seat.proxy.disable();
            seat.commit();
            seat.proxy.enable();
            Self::push_seat_state_locked(seat, s, ps, surfaces);
            seat.commit();
        }
        // Clear stale composition on the client side too — otherwise the visible
        // preedit lingers until the IME (re-)sends one, which may be deduped.
        if had_delivered_preedit {
            ps.sink.on_ime_update(surfaces.text_input_session_token(ps.surface), 0, 0, None, None, -1, -1);
        }
    }

    /// The state of the text input of every registered surface, for a test.
    pub fn snapshot(&self, surfaces: &dyn ITextInputSurfaces) -> Vec<TextInputSurfaceSnapshot> {
        let mut snapshots: Vec<TextInputSurfaceSnapshot> = self
            .surfaces
            .values()
            .map(|ps| TextInputSurfaceSnapshot {
                surface: ps.surface,
                entered_seats: ps.entered_seats.clone(),
                has_client: ps.has_client,
                last_sent_enabled: ps.last_sent_enabled,
                cursor_rectangle: ps.cursor_rect.map(|rect| translate_cursor_rect(surfaces, ps.surface, rect)),
                content_type: ps.options,
                surrounding_text: ps.surrounding_text.as_ref().map(|surrounding| surrounding.text.clone()),
            })
            .collect();
        snapshots.sort_by_key(|snapshot| snapshot.surface);
        snapshots
    }

    /// The number of seats that have a text input object.
    pub fn seat_count(&self) -> usize {
        self.seats.len()
    }

    /// The serial of the last `commit` request of a seat and the serial of its last `done`
    /// event, for a test.
    pub fn seat_serials(&self, seat_global_name: u32) -> Option<(u32, u32)> {
        self.seats.get(&seat_global_name).map(|seat| (seat.commit_serial, seat.last_done_serial))
    }

    fn disable_and_clear(seat: &mut PerSeat<M::TextInput>, ps: &PerSurface<S>) {
        if ps.last_sent_enabled {
            seat.proxy.disable();
            seat.commit();
        }
        seat.clear_pending();
    }

    fn push_state_to_entered_seats(
        seats: &mut BTreeMap<u32, PerSeat<M::TextInput>>,
        s: WSurfaceId,
        ps: &mut PerSurface<S>,
        surfaces: &dyn ITextInputSurfaces,
    ) {
        let enable = ps.has_client;
        for name in &ps.entered_seats {
            let Some(seat) = seats.get_mut(name) else {
                continue;
            };
            if enable {
                seat.proxy.enable();
                Self::push_seat_state_locked(seat, s, ps, surfaces);
            } else if ps.last_sent_enabled {
                seat.proxy.disable();
            }
            seat.commit();
        }
        ps.last_sent_enabled = enable;
        ps.declared = if enable { Features::of(ps) } else { Features::default() };
    }

    fn push_seat_state_locked(seat: &PerSeat<M::TextInput>, s: WSurfaceId, ps: &PerSurface<S>, surfaces: &dyn ITextInputSurfaces) {
        if let Some((hint, purpose)) = ps.options {
            seat.proxy.set_content_type(hint, purpose);
        }
        if let Some(rect) = ps.cursor_rect {
            let (x, y, w, h) = translate_cursor_rect(surfaces, s, rect);
            seat.proxy.set_cursor_rectangle(x, y, w, h);
        }
        if let (true, Some(surrounding)) = (ps.supports_surrounding_text, &ps.surrounding_text) {
            seat.proxy.set_surrounding_text(&surrounding.text, surrounding.cursor, surrounding.anchor);
            seat.proxy.set_text_change_cause(ChangeCause::Other);
        }
    }

    // ----- the listener of a seat's text input (`SeatListener`) -----

    /// `zwp_text_input_v3.enter`. `surface` is the shell surface the `wl_surface` of the
    /// event belongs to, `None` for a surface that is none of the worker's.
    pub fn on_enter(&mut self, seat_global_name: u32, surface: Option<WSurfaceId>, surfaces: &dyn ITextInputSurfaces) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(seat) = seats.get_mut(&seat_global_name) else {
            return;
        };
        let Some(ws) = surface else {
            return;
        };
        seat.focused_surface = Some(ws);
        let Some(ps) = per_surfaces.get_mut(&ws) else {
            return;
        };
        ps.entered_seats.retain(|name| *name != seat_global_name);
        ps.entered_seats.push(seat_global_name);
        seat.clear_pending();

        // A client that was set and not flushed yet is enabled by the flush, which finds the
        // seat entered.
        if ps.has_client && !ps.needs_push {
            seat.proxy.enable();
            Self::push_seat_state_locked(seat, ws, ps, surfaces);
            seat.commit();
            ps.last_sent_enabled = true;
            ps.declared = Features::of(ps);
        }
    }

    /// `zwp_text_input_v3.leave`.
    pub fn on_leave(&mut self, seat_global_name: u32, surface: Option<WSurfaceId>, surfaces: &dyn ITextInputSurfaces) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(seat) = seats.get_mut(&seat_global_name) else {
            return;
        };
        if surface.is_none() && seat.focused_surface.is_none() {
            return;
        }
        let ws = surface.or(seat.focused_surface);
        seat.focused_surface = None;
        seat.clear_pending();
        if let Some(ps) = ws.and_then(|ws| per_surfaces.get_mut(&ws)) {
            ps.entered_seats.retain(|name| *name != seat_global_name);
            if ps.entered_seats.is_empty() {
                ps.last_sent_enabled = false;
            }

            // Per spec, leaving invalidates any in-flight preedit. If we
            // had delivered preedit text from this seat, clear it on the
            // client side so the user doesn't see stale composition text.
            if seat.last_delivered_preedit.as_deref().is_some_and(|preedit| !preedit.is_empty()) {
                ps.sink.on_ime_update(surfaces.text_input_session_token(ps.surface), 0, 0, None, None, -1, -1);
            }
        }
        seat.clear_last_delivered();
    }

    /// `zwp_text_input_v3.preedit_string`.
    pub fn on_preedit_string(&mut self, seat_global_name: u32, text: Option<String>, cursor_begin: i32, cursor_end: i32) {
        let Some(seat) = self.seats.get_mut(&seat_global_name) else {
            return;
        };
        seat.pending_preedit = text.unwrap_or_default();
        seat.pending_preedit_cursor_begin_byte = cursor_begin;
        seat.pending_preedit_cursor_end_byte = cursor_end;
    }

    /// `zwp_text_input_v3.commit_string`.
    pub fn on_commit_string(&mut self, seat_global_name: u32, text: Option<String>) {
        let Some(seat) = self.seats.get_mut(&seat_global_name) else {
            return;
        };
        seat.pending_commit = text.unwrap_or_default();
    }

    /// `zwp_text_input_v3.delete_surrounding_text`.
    pub fn on_delete_surrounding_text(&mut self, seat_global_name: u32, before_length: u32, after_length: u32) {
        let Some(seat) = self.seats.get_mut(&seat_global_name) else {
            return;
        };
        seat.has_delete = true;
        seat.pending_delete_before_bytes = before_length;
        seat.pending_delete_after_bytes = after_length;
    }

    /// `zwp_text_input_v3.done`.
    pub fn on_done(&mut self, seat_global_name: u32, serial: u32, surfaces: &dyn ITextInputSurfaces) {
        let Self { seats, surfaces: per_surfaces, .. } = self;
        let Some(seat) = seats.get_mut(&seat_global_name) else {
            return;
        };
        seat.last_done_serial = serial;

        // Snapshot pending state, then clear immediately. The protocol's
        // double-buffered model requires pending preedit/commit/delete
        // to be consumed atomically on each done — so a subsequent done
        // with no preedit_string event clears the preedit.
        let pending_preedit = std::mem::take(&mut seat.pending_preedit);
        let pending_preedit_begin_byte = seat.pending_preedit_cursor_begin_byte;
        let pending_preedit_end_byte = seat.pending_preedit_cursor_end_byte;
        let pending_commit = std::mem::take(&mut seat.pending_commit);
        let has_delete = seat.has_delete;
        let pending_delete_before = seat.pending_delete_before_bytes;
        let pending_delete_after = seat.pending_delete_after_bytes;
        seat.clear_pending();

        // Only the active seat for its focused surface surfaces IME state.
        let ps = seat
            .focused_surface
            .and_then(|surface| per_surfaces.get(&surface))
            .filter(|ps| ps.entered_seats.last() == Some(&seat_global_name));
        let Some(ps) = ps else {
            if let Some(log) = Logger::try_get(LogEventLevel::Verbose, "Wayland.TextInput") {
                log.log(None, &format!("Dropping IME 'done' from non-active seat {seat_global_name}"));
            }
            return;
        };

        // Per spec (zwp_text_input_v3.done): "The serial number reflects
        // the last state of zwp_text_input_v3.commit request. The client
        // is expected to compare this serial with the most recent commit's
        // serial it has sent. If they don't match the client must ignore
        // the changes." Every enable/disable/state push we make ends in a
        // commit() that bumps commit_count, so a stale serial means the
        // batch was generated against state we've already superseded
        // (e.g. a prior focus/client session). Drop the whole batch.
        if serial != seat.commit_serial {
            if let Some(log) = Logger::try_get(LogEventLevel::Verbose, "Wayland.TextInput") {
                log.log(
                    None,
                    &format!(
                        "Dropping stale text-input-v3 done batch (done.serial={serial}, commit_count={})",
                        seat.commit_serial
                    ),
                );
            }
            return;
        }

        // Spec event order on the client side:
        //   1. clear current preedit (implicit before commit/delete);
        //   2. apply delete_surrounding_text;
        //   3. insert commit text;
        //   4. apply new preedit.
        // We deliver all four operations to the UI thread as a SINGLE
        // proxy call so they apply atomically (no other UI work can
        // interleave between them).
        let commit_text = (!pending_commit.is_empty()).then_some(pending_commit);
        let has_commit = commit_text.is_some();

        let (mut delete_before_chars, mut delete_after_chars) = (0, 0);
        if has_delete && ps.declared.surrounding_text {
            (delete_before_chars, delete_after_chars) = ps.surrounding_text.as_ref()
                .and_then(|surrounding| surrounding.deletion(pending_delete_before, pending_delete_after))
                .unwrap_or_default();
        }

        // Compute new preedit (null = clear).
        let (begin_char, end_char) = if pending_preedit_begin_byte < 0 || pending_preedit_end_byte < 0 {
            (-1, -1)
        } else {
            (
                wayland_text_utils::char_index_from_utf8_offset(&pending_preedit, pending_preedit_begin_byte, 0),
                wayland_text_utils::char_index_from_utf8_offset(&pending_preedit, pending_preedit_end_byte, 0),
            )
        };
        let new_preedit = (!pending_preedit.is_empty()).then_some(pending_preedit);

        let preedit_unchanged = seat.last_delivered_preedit == new_preedit
            && seat.last_delivered_cursor_begin_char == begin_char
            && seat.last_delivered_cursor_end_char == end_char;

        // Skip the entire batch only when nothing observable happens: no
        // commit, no delete, and the preedit is byte-for-byte identical
        // to what we last sent.
        let has_observable_delete = delete_before_chars > 0 || delete_after_chars > 0;
        if !has_commit && !has_observable_delete && preedit_unchanged {
            return;
        }

        ps.sink.on_ime_update(
            surfaces.text_input_session_token(ps.surface),
            delete_before_chars,
            delete_after_chars,
            commit_text,
            new_preedit.clone(),
            begin_char,
            end_char,
        );

        seat.last_delivered_preedit = new_preedit;
        seat.last_delivered_cursor_begin_char = begin_char;
        seat.last_delivered_cursor_end_char = end_char;
    }
}

struct SurroundingText {
    text: String,
    cursor: i32,
    anchor: i32,
}

impl SurroundingText {
    fn deletion(&self, before_bytes: u32, after_bytes: u32) -> Option<(i32, i32)> {
        // The lengths extend the editor's selection on either side. Reject
        // a request past the published window or inside a code point.
        let low = self.cursor.min(self.anchor) as usize;
        let high = self.cursor.max(self.anchor) as usize;
        let start = low.checked_sub(before_bytes as usize)?;
        let end = high.checked_add(after_bytes as usize)?;
        let before = self.text.get(start..low)?.encode_utf16().count() as i32;
        let after = self.text.get(high..end)?.encode_utf16().count() as i32;
        Some((before, after))
    }
}

/// The cursor rectangle of the client as `set_cursor_rectangle` takes it.
pub fn translate_cursor_rect(surfaces: &dyn ITextInputSurfaces, s: WSurfaceId, logical: Rect) -> (i32, i32, i32, i32) {
    // The cursor rectangle is in surface-local logical units. Under fractional
    // scaling this matches wp_viewport.set_destination units (also surface-local
    // logical). Add shadow extents so the candidate window appears next to the
    // editable content area, not next to the drop shadow.
    let shadow = surfaces.shadow_extents(s);
    let x = logical.x.round_ties_even() as i32 + shadow.left.round_ties_even() as i32;
    let y = logical.y.round_ties_even() as i32 + shadow.top.round_ties_even() as i32;
    let w = (logical.width.round_ties_even() as i32).max(1);
    let h = (logical.height.round_ties_even() as i32).max(1);
    (x, y, w, h)
}

/// Up to 4000 bytes of surrounding text, with directed UTF-8 selection offsets.
/// An unrepresentable selection yields an empty window.
pub fn clamp_surrounding_for_wayland(text: &str, cursor_char: i32, anchor_char: i32) -> (&str, i32, i32) {
    surrounding_window(text, cursor_char, anchor_char).unwrap_or_default()
}

fn surrounding_window(text: &str, cursor: i32, anchor: i32) -> Option<(&str, i32, i32)> {
    const MAX_BYTES: usize = 4000;
    let (cursor, anchor) = (usize::try_from(cursor).ok()?, usize::try_from(anchor).ok()?);
    if cursor.abs_diff(anchor) > MAX_BYTES {
        return None;
    }
    let mut position = 0;
    let (mut cursor_byte, mut anchor_byte) = (None, None);
    for (byte, character) in text.char_indices().chain(std::iter::once((text.len(), '\0'))) {
        if position == cursor {
            cursor_byte = Some(byte);
        }
        if position == anchor {
            anchor_byte = Some(byte);
        }
        if position >= cursor.max(anchor) {
            break;
        }
        position += character.len_utf16();
    }
    let (cursor, anchor) = (cursor_byte?, anchor_byte?);
    let (low, high) = (cursor.min(anchor), cursor.max(anchor));
    let context = MAX_BYTES.checked_sub(high - low)?;
    let mut start = low.saturating_sub(context / 2).min(text.len().saturating_sub(MAX_BYTES));
    let mut end = (start + MAX_BYTES).min(text.len());
    while !text.is_char_boundary(start) {
        start += 1;
    }
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    debug_assert!(start <= low && high <= end);
    // The Wayland wire type is a C string. End the context at a NUL and
    // withdraw it if that would split the actual selection.
    if text[low..high].contains('\0') {
        return None;
    }
    if let Some(nul) = text[start..low].rfind('\0') {
        start += nul + 1;
    }
    if let Some(nul) = text[high..end].find('\0') {
        end = high + nul;
    }
    Some((&text[start..end], (cursor - start) as i32, (anchor - start) as i32))
}

// ----- inner state types -----

/// The kinds of state a commit that enables carries: what the compositor supports for the
/// session it starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Features {
    content_type: bool,
    cursor_rectangle: bool,
    surrounding_text: bool,
}

impl Features {
    /// What `push_seat_state_locked` sends for the surface as it is.
    fn of<S>(ps: &PerSurface<S>) -> Self {
        Self {
            content_type: ps.options.is_some(),
            cursor_rectangle: ps.cursor_rect.is_some(),
            surrounding_text: ps.supports_surrounding_text && ps.surrounding_text.is_some(),
        }
    }
}

struct PerSurface<S> {
    surface: WSurfaceId,
    sink: S,
    entered_seats: Vec<u32>,
    /// The client changed and the seats were not told yet (the port's own: see `flush`).
    needs_push: bool,
    /// A kind of state arrived that the enabling commit did not have (the port's own).
    needs_reenable: bool,
    /// What the enabling commit had (the port's own).
    declared: Features,
    has_client: bool,
    #[allow(dead_code)] // Kept as the reference keeps it: nothing reads it there either.
    supports_preedit: bool,
    supports_surrounding_text: bool,
    last_sent_enabled: bool,
    cursor_rect: Option<Rect>,
    options: Option<(ContentHint, ContentPurpose)>,
    surrounding_text: Option<SurroundingText>,
}

impl<S> PerSurface<S> {
    fn new(surface: WSurfaceId, sink: S) -> Self {
        Self {
            surface,
            sink,
            entered_seats: Vec::new(),
            needs_push: false,
            needs_reenable: false,
            declared: Features::default(),
            has_client: false,
            supports_preedit: false,
            supports_surrounding_text: false,
            last_sent_enabled: false,
            cursor_rect: None,
            options: None,
            surrounding_text: None,
        }
    }
}

struct PerSeat<P: ITextInputV3> {
    global_name: u32,
    proxy: P,
    focused_surface: Option<WSurfaceId>,
    commit_serial: u32,
    last_done_serial: u32,

    // Pending state, accumulated until a done event. The protocol uses
    // a double-buffered model: preedit_string / commit_string /
    // delete_surrounding_text events stage values that are applied — and
    // reset — atomically on each done. So a done with no preceding
    // preedit_string event clears the preedit; this is how the protocol
    // model works, not a special-case rule.
    pending_preedit: String,
    pending_preedit_cursor_begin_byte: i32,
    pending_preedit_cursor_end_byte: i32,
    pending_commit: String,
    has_delete: bool,
    pending_delete_before_bytes: u32,
    pending_delete_after_bytes: u32,

    // Last delivered preedit (for done-event dedup).
    last_delivered_preedit: Option<String>,
    last_delivered_cursor_begin_char: i32,
    last_delivered_cursor_end_char: i32,
}

impl<P: ITextInputV3> PerSeat<P> {
    fn new(global_name: u32, proxy: P) -> Self {
        Self {
            global_name,
            proxy,
            focused_surface: None,
            commit_serial: 0,
            last_done_serial: 0,
            pending_preedit: String::new(),
            pending_preedit_cursor_begin_byte: 0,
            pending_preedit_cursor_end_byte: -1,
            pending_commit: String::new(),
            has_delete: false,
            pending_delete_before_bytes: 0,
            pending_delete_after_bytes: 0,
            last_delivered_preedit: None,
            last_delivered_cursor_begin_char: 0,
            last_delivered_cursor_end_char: 0,
        }
    }

    fn commit(&mut self) {
        self.commit_serial = self.commit_serial.wrapping_add(1);
        self.proxy.commit();
    }

    fn clear_pending(&mut self) {
        self.pending_preedit.clear();
        self.pending_preedit_cursor_begin_byte = 0;
        self.pending_preedit_cursor_end_byte = -1;
        self.pending_commit.clear();
        self.has_delete = false;
        self.pending_delete_before_bytes = 0;
        self.pending_delete_after_bytes = 0;
    }

    fn clear_last_delivered(&mut self) {
        self.last_delivered_preedit = None;
        self.last_delivered_cursor_begin_char = 0;
        self.last_delivered_cursor_end_char = 0;
    }
}

#[cfg(target_os = "linux")]
pub use imp::*;

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use crate::server::persistent::w_surface::{shell_surface_of, WXdgPopup, WXdgTopLevel};
    use crate::server::transient::i_wayland_text_input_v3_events::WaylandTextInputV3EventsProxy;
    use crate::server::transient::wayland_input_dispatcher::find_surface_for_wl_surface;
    use crate::server::wayland_worker::WaylandWorkerState;
    use wayland_client::protocol::wl_seat::WlSeat;
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
    use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
    use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{self, ZwpTextInputV3};

    /// The facade of a connection.
    pub type WorkerTextInputV3 = WaylandTextInputV3<TextInputManager, WaylandTextInputV3EventsProxy>;

    /// The manager of a connection with the queue its objects are made on.
    pub struct TextInputManager {
        manager: ZwpTextInputManagerV3,
        queue_handle: QueueHandle<WaylandWorkerState>,
    }

    impl TextInputManager {
        pub fn new(manager: ZwpTextInputManagerV3, queue_handle: QueueHandle<WaylandWorkerState>) -> Self {
            Self { manager, queue_handle }
        }
    }

    impl ITextInputManagerV3 for TextInputManager {
        type Seat = WlSeat;
        type TextInput = ZwpTextInputV3;

        fn get_text_input(&self, seat_global_name: u32, seat: &WlSeat) -> ZwpTextInputV3 {
            // The user data of the object is the registry name of its seat, which is how its
            // events find the state of the seat.
            self.manager.get_text_input(seat, &self.queue_handle, seat_global_name)
        }

        fn destroy(&self) {
            self.manager.destroy();
        }
    }

    impl ITextInputV3 for ZwpTextInputV3 {
        fn enable(&self) {
            ZwpTextInputV3::enable(self);
        }

        fn disable(&self) {
            ZwpTextInputV3::disable(self);
        }

        fn set_content_type(&self, hint: ContentHint, purpose: ContentPurpose) {
            // Both types of this crate have the values of the protocol.
            let hint = zwp_text_input_v3::ContentHint::from_bits_truncate(hint.bits());
            let purpose = zwp_text_input_v3::ContentPurpose::try_from(purpose as u32)
                .unwrap_or(zwp_text_input_v3::ContentPurpose::Normal);
            ZwpTextInputV3::set_content_type(self, hint, purpose);
        }

        fn set_cursor_rectangle(&self, x: i32, y: i32, width: i32, height: i32) {
            ZwpTextInputV3::set_cursor_rectangle(self, x, y, width, height);
        }

        fn set_surrounding_text(&self, text: &str, cursor: i32, anchor: i32) {
            ZwpTextInputV3::set_surrounding_text(self, text.to_string(), cursor, anchor);
        }

        fn set_text_change_cause(&self, cause: ChangeCause) {
            let cause = match cause {
                ChangeCause::InputMethod => zwp_text_input_v3::ChangeCause::InputMethod,
                ChangeCause::Other => zwp_text_input_v3::ChangeCause::Other,
            };
            ZwpTextInputV3::set_text_change_cause(self, cause);
        }

        fn commit(&self) {
            ZwpTextInputV3::commit(self);
        }

        fn destroy(&self) {
            ZwpTextInputV3::destroy(self);
        }
    }

    /// The surfaces of the worker as the facade reads them.
    pub struct WorkerTextInputSurfaces<'a> {
        pub top_levels: &'a HashMap<WSurfaceId, WXdgTopLevel>,
        pub popups: &'a HashMap<WSurfaceId, WXdgPopup>,
    }

    impl ITextInputSurfaces for WorkerTextInputSurfaces<'_> {
        fn shadow_extents(&self, surface: WSurfaceId) -> Thickness {
            shell_surface_of(self.top_levels, self.popups, surface).map(|shell| shell.shadow_extents()).unwrap_or_default()
        }

        fn text_input_session_token(&self, surface: WSurfaceId) -> i32 {
            shell_surface_of(self.top_levels, self.popups, surface)
                .map(|shell| shell.surface().text_input_session_token())
                .unwrap_or_default()
        }
    }

    impl Dispatch<ZwpTextInputManagerV3, ()> for WaylandWorkerState {
        fn event(
            _: &mut Self,
            _: &ZwpTextInputManagerV3,
            _: <ZwpTextInputManagerV3 as Proxy>::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    /// The listener of the text input of a seat (`SeatListener` of the reference): the user
    /// data is the registry name of the seat.
    impl Dispatch<ZwpTextInputV3, u32> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _: &ZwpTextInputV3,
            event: zwp_text_input_v3::Event,
            seat: &u32,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let WaylandWorkerState { globals, top_levels, popups, .. } = state;
            let Some(text_input) = globals.as_mut().and_then(|globals| globals.input_dispatcher.text_input_v3_mut()) else {
                return;
            };
            let surfaces = WorkerTextInputSurfaces { top_levels, popups };
            match event {
                zwp_text_input_v3::Event::Enter { surface } => {
                    text_input.on_enter(*seat, find_surface_for_wl_surface(&surface), &surfaces);
                }
                zwp_text_input_v3::Event::Leave { surface } => {
                    text_input.on_leave(*seat, find_surface_for_wl_surface(&surface), &surfaces);
                }
                zwp_text_input_v3::Event::PreeditString { text, cursor_begin, cursor_end } => {
                    text_input.on_preedit_string(*seat, text, cursor_begin, cursor_end);
                }
                zwp_text_input_v3::Event::CommitString { text } => text_input.on_commit_string(*seat, text),
                zwp_text_input_v3::Event::DeleteSurroundingText { before_length, after_length } => {
                    text_input.on_delete_surrounding_text(*seat, before_length, after_length);
                }
                zwp_text_input_v3::Event::Done { serial } => text_input.on_done(*seat, serial, &surfaces),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Debug, PartialEq)]
    enum Request {
        Enable,
        Disable,
        ContentType(ContentHint, ContentPurpose),
        CursorRectangle(i32, i32, i32, i32),
        SurroundingText(String, i32, i32),
        ChangeCause(ChangeCause),
        Commit,
        Destroy,
        DestroyManager,
    }

    /// What a seat's object was asked, with the registry name of the seat.
    type Requests = Rc<RefCell<Vec<(u32, Request)>>>;

    struct FakeTextInput {
        seat: u32,
        requests: Requests,
    }

    impl FakeTextInput {
        fn push(&self, request: Request) {
            self.requests.borrow_mut().push((self.seat, request));
        }
    }

    impl ITextInputV3 for FakeTextInput {
        fn enable(&self) {
            self.push(Request::Enable);
        }
        fn disable(&self) {
            self.push(Request::Disable);
        }
        fn set_content_type(&self, hint: ContentHint, purpose: ContentPurpose) {
            self.push(Request::ContentType(hint, purpose));
        }
        fn set_cursor_rectangle(&self, x: i32, y: i32, width: i32, height: i32) {
            self.push(Request::CursorRectangle(x, y, width, height));
        }
        fn set_surrounding_text(&self, text: &str, cursor: i32, anchor: i32) {
            self.push(Request::SurroundingText(text.to_string(), cursor, anchor));
        }
        fn set_text_change_cause(&self, cause: ChangeCause) {
            self.push(Request::ChangeCause(cause));
        }
        fn commit(&self) {
            self.push(Request::Commit);
        }
        fn destroy(&self) {
            self.push(Request::Destroy);
        }
    }

    struct FakeManager {
        requests: Requests,
    }

    impl ITextInputManagerV3 for FakeManager {
        type Seat = ();
        type TextInput = FakeTextInput;

        fn get_text_input(&self, seat_global_name: u32, _: &()) -> FakeTextInput {
            FakeTextInput { seat: seat_global_name, requests: self.requests.clone() }
        }

        fn destroy(&self) {
            self.requests.borrow_mut().push((0, Request::DestroyManager));
        }
    }

    /// One call of the sink: the token, the characters to delete, the commit, the preedit
    /// and its cursor.
    type Update = (i32, i32, i32, Option<String>, Option<String>, i32, i32);

    #[derive(Clone, Default)]
    struct FakeSink {
        updates: Rc<RefCell<Vec<Update>>>,
    }

    impl IWaylandTextInputV3Events for FakeSink {
        fn on_ime_update(
            &self,
            session_token: i32,
            delete_before_chars: i32,
            delete_after_chars: i32,
            commit_text: Option<String>,
            preedit_text: Option<String>,
            preedit_cursor_begin_char: i32,
            preedit_cursor_end_char: i32,
        ) {
            self.updates.borrow_mut().push((
                session_token,
                delete_before_chars,
                delete_after_chars,
                commit_text,
                preedit_text,
                preedit_cursor_begin_char,
                preedit_cursor_end_char,
            ));
        }
    }

    struct FakeSurfaces {
        shadow: Thickness,
        token: i32,
    }

    impl ITextInputSurfaces for FakeSurfaces {
        fn shadow_extents(&self, _: WSurfaceId) -> Thickness {
            self.shadow
        }
        fn text_input_session_token(&self, _: WSurfaceId) -> i32 {
            self.token
        }
    }

    const SEAT: u32 = 7;
    const TOKEN: i32 = 3;

    struct Fixture {
        text_input: WaylandTextInputV3<FakeManager, FakeSink>,
        requests: Requests,
        sink: FakeSink,
        surfaces: FakeSurfaces,
        surface: WSurfaceId,
    }

    impl Fixture {
        /// A facade with one seat and one registered surface.
        fn new() -> Self {
            let requests: Requests = Rc::default();
            let mut text_input = WaylandTextInputV3::new(FakeManager { requests: requests.clone() });
            text_input.on_seat_added(SEAT, &());
            let sink = FakeSink::default();
            let surface = WSurfaceId::new();
            text_input.register_surface(surface, sink.clone());
            Self { text_input, requests, sink, surfaces: FakeSurfaces { shadow: Thickness::default(), token: TOKEN }, surface }
        }

        /// The surface has the focus of the seat and a client that supports everything, which
        /// was enabled with a content type, a cursor rectangle and an empty surrounding text.
        fn active() -> Self {
            let mut fixture = Self::new();
            let surface = fixture.surface;
            fixture.text_input.on_enter(SEAT, Some(surface), &fixture.surfaces);
            fixture.text_input.set_active(surface, true, true, true);
            fixture.text_input.set_options(surface, (ContentHint::NONE, ContentPurpose::Normal));
            fixture.text_input.set_cursor_rect(surface, Rect::new(0.0, 0.0, 1.0, 1.0), &fixture.surfaces);
            fixture.text_input.set_surrounding_text(surface, String::new(), 0, 0);
            fixture.flush();
            fixture.take();
            fixture
        }

        /// The surface has the focus of the seat and a client that was enabled with no state.
        fn bare() -> Self {
            let mut fixture = Self::new();
            fixture.text_input.on_enter(SEAT, Some(fixture.surface), &fixture.surfaces);
            fixture.text_input.set_active(fixture.surface, true, true, true);
            fixture.flush();
            fixture.take();
            fixture
        }

        fn flush(&mut self) {
            self.text_input.flush(&self.surfaces);
        }

        /// The requests of the seat since the last call.
        fn take(&self) -> Vec<Request> {
            self.requests.borrow_mut().drain(..).map(|(_, request)| request).collect()
        }

        fn updates(&self) -> Vec<Update> {
            self.sink.updates.borrow_mut().drain(..).collect()
        }

        fn serial(&self) -> u32 {
            self.text_input.seat_serials(SEAT).expect("the seat is there").0
        }

        fn done(&mut self) {
            let serial = self.serial();
            self.text_input.on_done(SEAT, serial, &self.surfaces);
        }
    }

    #[test]
    fn a_client_on_a_focused_surface_enables_the_text_input_with_one_commit() {
        let mut fixture = Fixture::new();
        fixture.text_input.on_enter(SEAT, Some(fixture.surface), &fixture.surfaces);
        // The focus alone asks for nothing: no client yet.
        assert_eq!(fixture.take(), vec![]);

        // The client alone asks for nothing either: the flush does.
        fixture.text_input.set_active(fixture.surface, true, true, true);
        assert_eq!(fixture.take(), vec![]);
        fixture.flush();
        assert_eq!(fixture.take(), vec![Request::Enable, Request::Commit]);
        assert_eq!(fixture.serial(), 1);
        // A flush with nothing held back sends nothing.
        fixture.flush();
        assert_eq!(fixture.take(), vec![]);

        // The client goes: disable and a commit.
        fixture.text_input.set_active(fixture.surface, false, false, false);
        fixture.flush();
        assert_eq!(fixture.take(), vec![Request::Disable, Request::Commit]);
        assert_eq!(fixture.serial(), 2);

        // Without a client and without having been enabled there is nothing to disable, but
        // the state is committed.
        fixture.text_input.set_active(fixture.surface, false, false, false);
        fixture.flush();
        assert_eq!(fixture.take(), vec![Request::Commit]);
    }

    #[test]
    fn the_state_that_arrives_with_the_client_is_in_the_commit_that_enables() {
        let mut fixture = Fixture::new();
        let surface = fixture.surface;
        fixture.text_input.on_enter(SEAT, Some(surface), &fixture.surfaces);
        // The order of the UI thread: the client, then its state.
        fixture.text_input.set_active(surface, true, true, true);
        fixture.text_input.set_options(surface, (ContentHint::MULTILINE, ContentPurpose::Normal));
        fixture.text_input.set_surrounding_text(surface, "abc".to_string(), 3, 3);
        fixture.text_input.set_cursor_rect(surface, Rect::new(7.0, 8.0, 1.0, 14.0), &fixture.surfaces);
        assert_eq!(fixture.take(), vec![]);
        fixture.flush();
        assert_eq!(
            fixture.take(),
            vec![
                Request::Enable,
                Request::ContentType(ContentHint::MULTILINE, ContentPurpose::Normal),
                Request::CursorRectangle(7, 8, 1, 14),
                Request::SurroundingText("abc".to_string(), 3, 3),
                Request::ChangeCause(ChangeCause::Other),
                Request::Commit,
            ]
        );
        assert_eq!(fixture.serial(), 1);
    }

    #[test]
    fn a_kind_of_state_the_enabling_commit_did_not_have_enables_again() {
        let mut fixture = Fixture::bare();
        let surface = fixture.surface;
        let before = fixture.serial();

        // The cursor rectangle arrives a frame after the client: the compositor would ignore
        // it in a commit of its own.
        fixture.text_input.set_cursor_rect(surface, Rect::new(7.0, 8.0, 1.0, 14.0), &fixture.surfaces);
        assert_eq!(fixture.take(), vec![]);
        fixture.flush();
        assert_eq!(
            fixture.take(),
            vec![Request::Disable, Request::Commit, Request::Enable, Request::CursorRectangle(7, 8, 1, 14), Request::Commit]
        );
        assert_eq!(fixture.serial(), before + 2);

        // From then on a change of it is a commit of its own.
        fixture.text_input.set_cursor_rect(surface, Rect::new(9.0, 8.0, 1.0, 14.0), &fixture.surfaces);
        assert_eq!(fixture.take(), vec![Request::CursorRectangle(9, 8, 1, 14), Request::Commit]);
        fixture.flush();
        assert_eq!(fixture.take(), vec![]);
    }

    #[test]
    fn the_focus_that_arrives_after_the_client_enables_with_the_state_of_the_client() {
        let mut fixture = Fixture::new();
        let surface = fixture.surface;
        fixture.text_input.set_active(surface, true, true, true);
        fixture.text_input.set_options(surface, (ContentHint::MULTILINE, ContentPurpose::Email));
        fixture.text_input.set_cursor_rect(surface, Rect::new(10.0, 20.0, 1.0, 16.0), &fixture.surfaces);
        fixture.text_input.set_surrounding_text(surface, "hello".to_string(), 5, 2);
        fixture.flush();
        // No seat entered the surface: nothing was sent.
        assert_eq!(fixture.take(), vec![]);

        fixture.text_input.on_enter(SEAT, Some(surface), &fixture.surfaces);
        assert_eq!(
            fixture.take(),
            vec![
                Request::Enable,
                Request::ContentType(ContentHint::MULTILINE, ContentPurpose::Email),
                Request::CursorRectangle(10, 20, 1, 16),
                Request::SurroundingText("hello".to_string(), 5, 2),
                Request::ChangeCause(ChangeCause::Other),
                Request::Commit,
            ]
        );
        let snapshot = fixture.text_input.snapshot(&fixture.surfaces);
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot[0].last_sent_enabled && snapshot[0].has_client);
        assert_eq!(snapshot[0].entered_seats, vec![SEAT]);
        assert_eq!(snapshot[0].cursor_rectangle, Some((10, 20, 1, 16)));
    }

    #[test]
    fn a_surface_registered_after_the_focus_arrived_knows_the_seat() {
        let requests: Requests = Rc::default();
        let mut text_input = WaylandTextInputV3::new(FakeManager { requests: requests.clone() });
        text_input.on_seat_added(SEAT, &());
        let surfaces = FakeSurfaces { shadow: Thickness::default(), token: TOKEN };
        let surface = WSurfaceId::new();
        text_input.on_enter(SEAT, Some(surface), &surfaces);
        text_input.register_surface(surface, FakeSink::default());
        text_input.set_active(surface, true, true, true);
        text_input.flush(&surfaces);
        let sent: Vec<Request> = requests.borrow().iter().map(|(_, request)| request.clone()).collect();
        assert_eq!(sent, vec![Request::Enable, Request::Commit]);
    }

    #[test]
    fn state_changes_of_an_enabled_text_input_are_sent_each_with_a_commit() {
        let mut fixture = Fixture::active();
        let surface = fixture.surface;

        fixture.text_input.set_options(surface, (ContentHint::NONE, ContentPurpose::Normal));
        assert_eq!(fixture.take(), vec![Request::ContentType(ContentHint::NONE, ContentPurpose::Normal), Request::Commit]);

        fixture.text_input.set_cursor_rect(surface, Rect::new(4.5, 5.5, 0.0, 17.4), &fixture.surfaces);
        // Halves round to even; a rectangle is never empty.
        assert_eq!(fixture.take(), vec![Request::CursorRectangle(4, 6, 1, 17), Request::Commit]);

        // The cursor and the anchor are byte indices: two characters of three bytes each.
        fixture.text_input.set_surrounding_text(surface, "\u{4f60}\u{597d}".to_string(), 2, 1);
        assert_eq!(
            fixture.take(),
            vec![
                Request::SurroundingText("\u{4f60}\u{597d}".to_string(), 6, 3),
                Request::ChangeCause(ChangeCause::Other),
                Request::Commit
            ]
        );
        assert_eq!(fixture.serial(), 4);
        // Nothing was held back.
        fixture.flush();
        assert_eq!(fixture.take(), vec![]);
    }

    #[test]
    fn the_cursor_rectangle_is_moved_by_the_shadow_of_the_surface() {
        let mut fixture = Fixture::active();
        fixture.surfaces.shadow = Thickness::new(8.0, 12.0, 8.0, 8.0);
        fixture.text_input.set_cursor_rect(fixture.surface, Rect::new(100.0, 50.0, 2.0, 20.0), &fixture.surfaces);
        assert_eq!(fixture.take(), vec![Request::CursorRectangle(108, 62, 2, 20), Request::Commit]);
    }

    #[test]
    fn a_client_that_has_no_surrounding_text_sends_none() {
        let mut fixture = Fixture::new();
        let surface = fixture.surface;
        fixture.text_input.on_enter(SEAT, Some(surface), &fixture.surfaces);
        fixture.text_input.set_active(surface, true, true, false);
        fixture.flush();
        fixture.take();
        fixture.text_input.set_surrounding_text(surface, "text".to_string(), 0, 0);
        fixture.flush();
        assert_eq!(fixture.take(), vec![]);
    }

    #[test]
    fn a_done_applies_the_pending_preedit_and_commit_as_one_update() {
        let mut fixture = Fixture::active();

        // A preedit of two characters of three bytes, the cursor after it.
        fixture.text_input.on_preedit_string(SEAT, Some("\u{4f60}\u{597d}".to_string()), 6, 6);
        assert_eq!(fixture.updates(), vec![]);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, Some("\u{4f60}\u{597d}".to_string()), 2, 2)]);

        // The same preedit again is not delivered twice.
        fixture.text_input.on_preedit_string(SEAT, Some("\u{4f60}\u{597d}".to_string()), 6, 6);
        fixture.done();
        assert_eq!(fixture.updates(), vec![]);

        // A commit without a preedit: the text, and the preedit is cleared.
        fixture.text_input.on_commit_string(SEAT, Some("\u{4f60}\u{597d}".to_string()));
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, Some("\u{4f60}\u{597d}".to_string()), None, -1, -1)]);

        // A done with nothing pending and nothing delivered changes nothing.
        fixture.done();
        assert_eq!(fixture.updates(), vec![]);
    }

    #[test]
    fn a_done_without_a_preedit_clears_the_delivered_one() {
        let mut fixture = Fixture::active();
        fixture.text_input.on_preedit_string(SEAT, Some("ni".to_string()), 2, 2);
        fixture.done();
        fixture.updates();
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, None, -1, -1)]);
    }

    #[test]
    fn a_preedit_with_a_hidden_cursor_has_the_cursor_minus_one() {
        let mut fixture = Fixture::active();
        fixture.text_input.on_preedit_string(SEAT, Some("ni".to_string()), -1, -1);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, Some("ni".to_string()), -1, -1)]);
    }

    #[test]
    fn a_done_with_a_serial_other_than_that_of_the_last_commit_is_dropped_with_its_batch() {
        let mut fixture = Fixture::active();
        let stale = fixture.serial();
        // One more commit of this client: the compositor has not seen it when it sends done.
        fixture.text_input.set_cursor_rect(fixture.surface, Rect::new(0.0, 0.0, 1.0, 1.0), &fixture.surfaces);
        fixture.text_input.on_commit_string(SEAT, Some("x".to_string()));
        fixture.text_input.on_done(SEAT, stale, &fixture.surfaces);
        assert_eq!(fixture.updates(), vec![]);
        assert_eq!(fixture.text_input.seat_serials(SEAT), Some((stale + 1, stale)));

        // The batch is gone with the done: the next one does not carry the commit. (The
        // first done that is taken says "no preedit" once, which the broker ignores.)
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, None, -1, -1)]);
    }

    #[test]
    fn deleting_around_the_cursor_is_converted_from_bytes_to_characters() {
        let mut fixture = Fixture::active();
        // "a", a character of two bytes, one of three, a surrogate pair (four bytes), "b";
        // the cursor after the character of three bytes (index 3).
        let text = "a\u{439}\u{65e5}\u{1F30D}b";
        fixture.text_input.set_surrounding_text(fixture.surface, text.to_string(), 3, 3);
        fixture.take();
        fixture.done();
        fixture.updates();

        // Five bytes before the cursor are two characters, four bytes after it the pair.
        fixture.text_input.on_delete_surrounding_text(SEAT, 5, 4);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 2, 2, None, None, -1, -1)]);
    }

    #[test]
    fn deleting_without_surrounding_text_is_dropped() {
        let mut fixture = Fixture::bare();
        fixture.text_input.on_delete_surrounding_text(SEAT, 5, 4);
        fixture.done();
        // Nothing to delete arrives (the first done that is taken says "no preedit" once).
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, None, -1, -1)]);
    }

    #[test]
    fn surrounding_text_beyond_the_limit_keeps_the_whole_directed_selection() {
        let text: String = "\u{439}".repeat(3000);
        for (cursor, anchor) in [(900, 2500), (2500, 900)] {
            let mut fixture = Fixture::active();
            fixture.text_input.set_surrounding_text(fixture.surface, text.clone(), cursor, anchor);
            let (cursor_byte, anchor_byte) = if cursor < anchor { (400, 3600) } else { (3600, 400) };
            assert_eq!(fixture.take(), vec![
                Request::SurroundingText("\u{439}".repeat(2000), cursor_byte, anchor_byte),
                Request::ChangeCause(ChangeCause::Other),
                Request::Commit,
            ]);
        }

        // A text within the limit is sent whole.
        let (whole, cursor, anchor) = clamp_surrounding_for_wayland("\u{4f60}\u{597d}", 1, 2);
        assert_eq!((whole, cursor, anchor), ("\u{4f60}\u{597d}", 3, 6));
        // Cropping can land in the middle of a UTF-8 code point on each side.
        let astral = "🌐".repeat(1100);
        let (window, cursor, anchor) = clamp_surrounding_for_wayland(&astral, 1001, 1000);
        assert_eq!((window, cursor, anchor), ("", 0, 0));
        let (window, cursor, anchor) = clamp_surrounding_for_wayland(&astral, 1002, 1000);
        assert_eq!((window.len(), cursor, anchor), (3996, 2000, 1996));
    }

    #[test]
    fn a_selection_larger_than_the_window_withdraws_surrounding_text_until_it_fits() {
        let mut fixture = Fixture::active();
        let surface = fixture.surface;
        let text = "🌐".repeat(1001);
        fixture.text_input.set_surrounding_text(surface, text.clone(), 0, 2002);
        fixture.flush();
        assert_eq!(fixture.take(), vec![
            Request::Disable, Request::Commit, Request::Enable,
            Request::ContentType(ContentHint::NONE, ContentPurpose::Normal),
            Request::CursorRectangle(0, 0, 1, 1), Request::Commit,
        ]);
        // The IME can keep committing text while contextual deletion is unavailable.
        fixture.text_input.on_delete_surrounding_text(SEAT, 0, 4);
        fixture.text_input.on_commit_string(SEAT, Some("z".to_owned()));
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, Some("z".to_owned()), None, -1, -1)]);

        fixture.text_input.set_surrounding_text(surface, text, 0, 2000);
        fixture.flush();
        assert_eq!(fixture.take(), vec![
            Request::Disable, Request::Commit, Request::Enable,
            Request::ContentType(ContentHint::NONE, ContentPurpose::Normal),
            Request::CursorRectangle(0, 0, 1, 1),
            Request::SurroundingText("🌐".repeat(1000), 0, 4000),
            Request::ChangeCause(ChangeCause::Other), Request::Commit,
        ]);
        fixture.text_input.on_delete_surrounding_text(SEAT, 0, 4);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, None, -1, -1)]);

        fixture.text_input.set_surrounding_text(surface, "🌐text".to_owned(), 2, 2);
        fixture.take();
        fixture.text_input.on_delete_surrounding_text(SEAT, 4, 1);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 2, 1, None, None, -1, -1)]);
    }

    #[test]
    fn surrounding_text_never_places_a_nul_in_the_wayland_string() {
        let text = "before\0after";
        assert_eq!(surrounding_window(text, 6, 6), Some(("before", 6, 6)));
        assert_eq!(surrounding_window(text, 7, 7), Some(("after", 0, 0)));
        assert_eq!(surrounding_window(text, 3, 10), None);
    }

    #[test]
    fn deletion_counts_bytes_from_both_ends_of_a_directed_selection() {
        let mut fixture = Fixture::active();
        for (cursor, anchor) in [(3, 2), (2, 3)] {
            fixture.text_input.set_surrounding_text(fixture.surface, "🌐x界".to_owned(), cursor, anchor);
            fixture.take();
            fixture.text_input.on_delete_surrounding_text(SEAT, 4, 3);
            fixture.done();
            assert_eq!(fixture.updates(), vec![(TOKEN, 2, 1, None, None, -1, -1)]);
        }
    }

    #[test]
    fn deletion_outside_the_sent_window_or_inside_a_code_point_is_dropped() {
        let mut fixture = Fixture::active();
        fixture.text_input.set_surrounding_text(fixture.surface, "🌐".repeat(1500), 1500, 1500);
        fixture.take();
        fixture.done();
        fixture.updates();
        for (before, after) in [(2004, 0), (0, 2004), (1, 0), (4, 2), (0, u32::MAX)] {
            fixture.text_input.on_delete_surrounding_text(SEAT, before, after);
            fixture.done();
            assert!(fixture.updates().is_empty());
        }
        fixture.text_input.on_delete_surrounding_text(SEAT, 4, 4);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 2, 2, None, None, -1, -1)]);
    }

    #[test]
    fn leaving_clears_the_preedit_that_was_delivered() {
        let mut fixture = Fixture::active();
        fixture.text_input.on_preedit_string(SEAT, Some("ni".to_string()), 2, 2);
        fixture.done();
        fixture.updates();

        fixture.text_input.on_leave(SEAT, Some(fixture.surface), &fixture.surfaces);
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, None, -1, -1)]);
        // Leaving sends nothing to the compositor, and the surface is no longer enabled.
        assert_eq!(fixture.take(), vec![]);
        assert!(!fixture.text_input.snapshot(&fixture.surfaces)[0].last_sent_enabled);

        // The focus comes back: enabled again, the state of the client sent again.
        fixture.text_input.on_enter(SEAT, Some(fixture.surface), &fixture.surfaces);
        let sent = fixture.take();
        assert_eq!(sent.first(), Some(&Request::Enable));
        assert_eq!(sent.last(), Some(&Request::Commit));
        assert!(sent.contains(&Request::CursorRectangle(0, 0, 1, 1)), "{sent:?}");
    }

    #[test]
    fn a_leave_without_its_surface_leaves_the_focused_one() {
        let mut fixture = Fixture::active();
        fixture.text_input.on_leave(SEAT, None, &fixture.surfaces);
        assert_eq!(fixture.text_input.snapshot(&fixture.surfaces)[0].entered_seats, Vec::<u32>::new());
    }

    #[test]
    fn aborting_a_composition_disables_and_forgets_the_state_of_the_client() {
        let mut fixture = Fixture::active();
        let surface = fixture.surface;
        fixture.text_input.set_surrounding_text(surface, "abc".to_string(), 1, 1);
        fixture.take();

        fixture.text_input.abort_composition(surface);
        assert_eq!(fixture.take(), vec![Request::Disable, Request::Commit]);
        let snapshot = fixture.text_input.snapshot(&fixture.surfaces);
        assert!(!snapshot[0].has_client && !snapshot[0].last_sent_enabled);
        assert_eq!(snapshot[0].surrounding_text, None);

        // The next client starts clean: enabled, nothing of the old state.
        fixture.text_input.set_active(surface, true, true, true);
        fixture.flush();
        assert_eq!(fixture.take(), vec![Request::Enable, Request::Commit]);
    }

    #[test]
    fn a_reset_is_disable_and_enable_and_clears_the_preedit_of_the_client() {
        let mut fixture = Fixture::active();
        let surface = fixture.surface;
        fixture.text_input.on_preedit_string(SEAT, Some("ni".to_string()), 2, 2);
        fixture.done();
        fixture.updates();
        fixture.take();
        let before = fixture.serial();

        fixture.text_input.reset(surface, &fixture.surfaces);
        assert_eq!(
            fixture.take(),
            vec![
                Request::Disable,
                Request::Commit,
                Request::Enable,
                Request::ContentType(ContentHint::NONE, ContentPurpose::Normal),
                Request::CursorRectangle(0, 0, 1, 1),
                Request::SurroundingText(String::new(), 0, 0),
                Request::ChangeCause(ChangeCause::Other),
                Request::Commit
            ]
        );
        assert_eq!(fixture.serial(), before + 2);
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, None, -1, -1)]);

        // The same preedit after a reset is delivered again.
        fixture.text_input.on_preedit_string(SEAT, Some("ni".to_string()), 2, 2);
        fixture.done();
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, None, Some("ni".to_string()), 2, 2)]);
    }

    #[test]
    fn a_reset_without_a_client_does_nothing() {
        let mut fixture = Fixture::new();
        fixture.text_input.on_enter(SEAT, Some(fixture.surface), &fixture.surfaces);
        fixture.text_input.reset(fixture.surface, &fixture.surfaces);
        assert_eq!(fixture.take(), vec![]);
    }

    #[test]
    fn only_the_seat_that_entered_last_delivers() {
        const OTHER: u32 = 9;
        let mut fixture = Fixture::active();
        fixture.text_input.on_seat_added(OTHER, &());
        fixture.text_input.on_enter(OTHER, Some(fixture.surface), &fixture.surfaces);
        fixture.take();

        // The first seat is no longer the active one of the surface.
        fixture.text_input.on_commit_string(SEAT, Some("first".to_string()));
        fixture.done();
        assert_eq!(fixture.updates(), vec![]);

        fixture.text_input.on_commit_string(OTHER, Some("second".to_string()));
        let serial = fixture.text_input.seat_serials(OTHER).expect("the seat is there").0;
        fixture.text_input.on_done(OTHER, serial, &fixture.surfaces);
        assert_eq!(fixture.updates(), vec![(TOKEN, 0, 0, Some("second".to_string()), None, -1, -1)]);
    }

    #[test]
    fn a_surface_that_goes_disables_and_a_seat_that_goes_is_destroyed() {
        let mut fixture = Fixture::active();
        fixture.text_input.unregister_surface(fixture.surface);
        assert_eq!(fixture.take(), vec![Request::Disable, Request::Commit]);
        assert_eq!(fixture.text_input.snapshot(&fixture.surfaces), vec![]);

        // An event for a surface that is not registered changes nothing.
        fixture.text_input.on_commit_string(SEAT, Some("x".to_string()));
        fixture.done();
        assert_eq!(fixture.updates(), vec![]);

        fixture.text_input.on_seat_removed(SEAT);
        assert_eq!(fixture.take(), vec![Request::Destroy]);
        assert_eq!(fixture.text_input.seat_count(), 0);

        fixture.text_input.on_seat_added(SEAT, &());
        fixture.text_input.dispose();
        assert_eq!(fixture.take(), vec![Request::Destroy, Request::DestroyManager]);
    }

    #[test]
    fn the_token_of_an_update_is_that_of_the_surface_at_the_time() {
        let mut fixture = Fixture::active();
        fixture.surfaces.token = 11;
        fixture.text_input.on_commit_string(SEAT, Some("x".to_string()));
        fixture.done();
        assert_eq!(fixture.updates(), vec![(11, 0, 0, Some("x".to_string()), None, -1, -1)]);
    }
}
