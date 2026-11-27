//! The window of the host (feature `desktop`, docs/osprey-native-host.md, section 2): the
//! platform of the operating system, the Fluent theme, the application's regions as the window's
//! content, the frame clock on FerroUI's animation frames, and the window's light or dark
//! appearance choosing the theme's colours.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use ferroui_base::media::MediaContext;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::Ref;
use ferroui_controls::Window;
use ferroui_view::prelude::*;

use crate::boundary::Boundary;
use crate::host::Host;
use crate::protocol::events;
use crate::session::{Clock, Session};
use crate::store::Store;

/// The window's settings, from the environment: `OSPREY_NATIVE_TITLE` (default "Osprey"),
/// `OSPREY_NATIVE_WINDOW` as `WIDTHxHEIGHT` (default 1200x800), and `OSPREY_NATIVE_APP_NAME`,
/// the folder of the application's store (default "osprey").
struct Settings {
    title: String,
    width: f64,
    height: f64,
    app: String,
}

impl Settings {
    fn from_env() -> Self {
        let size = std::env::var("OSPREY_NATIVE_WINDOW")
            .ok()
            .and_then(|size| {
                let (width, height) = size.split_once('x')?;
                Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
            })
            .unwrap_or((1200.0, 800.0));
        Self {
            title: std::env::var("OSPREY_NATIVE_TITLE").unwrap_or_else(|_| "Osprey".to_owned()),
            width: size.0,
            height: size.1,
            app: std::env::var("OSPREY_NATIVE_APP_NAME").unwrap_or_else(|_| "osprey".to_owned()),
        }
    }
}

/// Starts the application, opens its window and runs until the window closes.
pub fn run(boundary: Box<dyn Boundary>, arguments: &[String]) -> i32 {
    let settings = Settings::from_env();
    let context = crate::launch_context(arguments);
    let session = match Session::start(boundary, &context, Store::for_app(&settings.app)) {
        Ok(session) => session,
        Err(problem) => {
            eprintln!("the Osprey application did not start: {problem}");
            return 1;
        }
    };
    let made: Rc<RefCell<Option<Rc<Host>>>> = Rc::new(RefCell::new(None));
    let kept = made.clone();
    ferroui_view::launch_with(
        move || {
            let host = Host::new(session);
            *kept.borrow_mut() = Some(host.clone());
            window()
                .title(settings.title)
                .width(settings.width)
                .height(settings.height)
                .child(host.element())
                .into()
        },
        move |window| {
            let Some(host) = made.borrow_mut().take() else {
                return;
            };
            follow_appearance(&host, window);
            let clock = Rc::new(FrameClock {
                scheduled: Cell::new(false),
                last: Cell::new(None),
            });
            host.on_clock(Box::new(move |host, change| {
                if change == Clock::On {
                    clock.clone().schedule(host.clone());
                }
            }));
            host.send(&events::lifecycle(events::now(), "active"));
        },
    );
    0
}

/// Keeps the theme's colours in the window's appearance: light, or dark.
fn follow_appearance(host: &Rc<Host>, window: &Ref<Window>) {
    let dark = |window: &Ref<Window>| window.actual_theme_variant() == Some(ThemeVariant::dark());
    host.set_dark(dark(window));
    let (weak, host) = (window.downgrade(), host.clone());
    let subscription = window.actual_theme_variant_changed(move || {
        if let Some(window) = weak.upgrade() {
            host.set_dark(dark(&window));
        }
    });
    // The window owns the host's tree; the subscription lives as long as the window.
    let subscription = Cell::new(Some(subscription));
    window.closed(move || drop(subscription.take())).detach();
}

/// The frame clock: while the application has it on, one `frame` event per animation frame of
/// FerroUI's media context, with the seconds since the one before (at most 0.1).
struct FrameClock {
    scheduled: Cell<bool>,
    last: Cell<Option<Instant>>,
}

impl FrameClock {
    fn schedule(self: Rc<Self>, host: Rc<Host>) {
        if self.scheduled.replace(true) {
            return;
        }
        MediaContext::instance().request_animation_frame(move |_| {
            self.scheduled.set(false);
            if !host.session().animating() {
                self.last.set(None);
                return;
            }
            let now = Instant::now();
            let elapsed = self
                .last
                .get()
                .map_or(1.0 / 60.0, |last| now.duration_since(last).as_secs_f64())
                .min(0.1);
            self.last.set(Some(now));
            host.send(&events::frame(events::now(), elapsed));
            self.schedule(host);
        });
    }
}
