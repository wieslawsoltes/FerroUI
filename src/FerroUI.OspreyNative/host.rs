//! The host: a running session and the FerroUI tree that shows it (docs/osprey-native-host.md,
//! section 3). The tree is one component that runs again whenever an envelope arrives; the
//! reconciler of `ferroui-view` patches the controls it made before into the new elements, so a
//! control whose place and class stay keeps its focus, scroll offset, caret and selection.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Color;
use ferroui_view::prelude::*;
use serde_json::Value;

use crate::session::{Clock, Session};
use crate::theme::Look;

/// What the window does when the application turns the frame clock on or off.
pub type ClockHandler = Box<dyn Fn(&Rc<Host>, Clock)>;

/// A running application and the state of its tree.
pub struct Host {
    session: RefCell<Session>,
    /// Bumped after every envelope: the tree's component reads it, so it runs again.
    version: Signal<u64>,
    /// Whether the window is in the dark appearance (the theme's dark colours apply).
    dark: Signal<bool>,
    /// True while the host itself changes controls (a patch): their change events are not the
    /// person's input and are not sent.
    patching: Cell<bool>,
    /// What the last render could not draw, shown in the diagnostics strip.
    rendered: RefCell<Vec<String>>,
    clock: RefCell<Option<ClockHandler>>,
}

impl Host {
    /// A host for a started session. Signals belong to the current owner, so this is called
    /// where the tree is mounted (`ferroui_view::launch`, `Element::mount_root`).
    pub fn new(session: Session) -> Rc<Self> {
        Rc::new(Self {
            session: RefCell::new(session),
            version: signal(0),
            dark: signal(false),
            patching: Cell::new(false),
            rendered: RefCell::new(Vec::new()),
            clock: RefCell::new(None),
        })
    }

    /// Sets what the window does when the clock changes, and applies the state it is in now.
    pub fn on_clock(self: &Rc<Self>, handler: ClockHandler) {
        let animating = self.session.borrow().animating();
        if animating {
            handler(self, Clock::On);
        }
        *self.clock.borrow_mut() = Some(handler);
    }

    /// The session, to read.
    pub fn session(&self) -> std::cell::Ref<'_, Session> {
        self.session.borrow()
    }

    /// Sends a control's value; a change the host made itself is not sent.
    pub fn control(self: &Rc<Self>, region: &str, revision: u64, ordinal: u64, value: Value) {
        if self.patching.get() {
            return;
        }
        let Ok(mut session) = self.session.try_borrow_mut() else {
            return;
        };
        session.control(region, revision, ordinal, value);
        drop(session);
        self.refresh();
    }

    /// Sends any event (a frame, a lifecycle phase) and shows what comes back.
    pub fn send(self: &Rc<Self>, event: &str) {
        let Ok(mut session) = self.session.try_borrow_mut() else {
            return;
        };
        session.send(event);
        drop(session);
        self.refresh();
    }

    /// Switches between the light and the dark colours of the theme.
    pub fn set_dark(&self, dark: bool) {
        if self.dark.get_untracked() != dark {
            self.patching.set(true);
            self.dark.set(dark);
            self.patching.set(false);
        }
    }

    /// Runs the tree again for the session as it is now, and tells the window about the clock.
    fn refresh(self: &Rc<Self>) {
        self.patching.set(true);
        self.version.update(|version| *version += 1);
        self.patching.set(false);
        let change = self.session.borrow_mut().take_clock();
        if let Some(change) = change {
            if let Some(handler) = self.clock.borrow().as_ref() {
                handler(self, change);
            }
        }
    }

    /// Whether a change event comes from the host's own patch.
    pub fn is_patching(&self) -> bool {
        self.patching.get()
    }

    /// Records what a render could not draw.
    pub(crate) fn refuse(&self, message: String) {
        let mut rendered = self.rendered.borrow_mut();
        if !rendered.contains(&message) {
            rendered.push(message);
        }
    }

    /// The element of the whole window: the root region above the diagnostics strip.
    pub fn element(self: &Rc<Self>) -> Element {
        let host = self.clone();
        // The root control stays; the component inside it runs again for each envelope.
        border()
            .child(rerender(move || {
                host.version.get();
                let dark = host.dark.get();
                host.rendered.borrow_mut().clear();
                let session = host.session.borrow();
                let look = Look {
                    theme: session.theme().cloned(),
                    dark,
                };
                let background = look.colour("background");
                let region = crate::view::root(&host, &session, look.clone());
                let mut messages: Vec<String> = session.diagnostics().to_vec();
                messages.extend(host.rendered.borrow().iter().cloned());
                drop(session);
                // The window frames the application as the GPUI host does: 16 points of padding, and the
                // diagnostics 16 points below the regions.
                let mut grid = grid()
                    .row_definitions("*,Auto")
                    .row_spacing(16.0)
                    .child(border().grid_row(0).child(region));
                if !messages.is_empty() {
                    grid = grid.child(diagnostics(&messages, &look));
                }
                let mut frame = border().padding(ferroui_base::Thickness::uniform(16.0));
                if let Some(background) = background {
                    frame = frame.background(background);
                }
                frame.child(grid)
            }))
            .into()
    }
}

/// The strip of what the framework reported and the host could not draw, the latest last.
fn diagnostics(messages: &[String], look: &Look) -> Element {
    let ink = look
        .colour("danger")
        .unwrap_or(Color::from_argb(0xFF, 0xC4, 0x2B, 0x1C));
    let recent = &messages[messages.len().saturating_sub(6)..];
    stack_panel()
        .key("host:diagnostics")
        .grid_row(1)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Bottom)
        .child(
            text_block()
                .text(format!("Diagnostics ({})", messages.len()))
                .font_size(12.0),
        )
        .children(
            recent
                .iter()
                .map(|message| {
                    text_block()
                        .text(message.clone())
                        .foreground(ink)
                        .font_size(12.0)
                        .into()
                })
                .collect::<Vec<Element>>(),
        )
        .into()
}
