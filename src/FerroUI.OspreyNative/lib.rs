//! An Osprey Native host on FerroUI (docs/osprey-native-host.md).
//!
//! [Osprey](https://github.com/Nimblesite/osprey) applications written with Osprey Native are
//! Elm-like: a `view` of the state and `update(message, store)`. Compiled as a C library for a
//! host (`osprey <app> --compile --target=macos`), such an application exports `start` and
//! `dispatch`, which take and return JSON envelopes: the regions of the user interface as node
//! trees, the capabilities the application asks for, and its theme. This crate is the seam
//! between that library and FerroUI, the same design as Osprey's own GPUI host:
//!
//! - [`boundary`]: the C calls into the linked application and the imports its runtime calls;
//! - [`protocol`] and [`schema`]: the envelope and the events, checked whole before use;
//! - [`session`]: the regions by key, the opaque session, the capabilities answered;
//! - [`view`]: one mapping per node kind to FerroUI controls; [`host`] keeps them in one tree
//!   that the reconciler of `ferroui-view` patches, so controls whose place stays are kept;
//! - [`scene`]: the scene primitives, drawn with FerroUI's drawing context;
//! - [`smoke`]: the windowless check of the real boundary.
//!
//! No rule of an application lives here: the host sends what a control delivered under the
//! ordinal the framework issued and draws what the framework returns.

pub mod boundary;
pub mod host;
pub mod protocol;
pub mod scene;
pub mod schema;
pub mod session;
pub mod smoke;
pub mod store;
pub mod symbols;
pub mod theme;
pub mod view;

#[cfg(feature = "desktop")]
pub mod desktop;

/// The launch context of this process: now, the local UTC offset, the platform as
/// `std::env::consts::OS` spells it, the home folder and the arguments after the program name.
pub fn launch_context(arguments: &[String]) -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    protocol::events::start_context(
        protocol::events::now(),
        local_offset(),
        std::env::consts::OS,
        &home,
        arguments,
    )
}

/// The local UTC offset in seconds, from `date +%z` where the system has it, else 0.
fn local_offset() -> i64 {
    let Ok(output) = std::process::Command::new("date").arg("+%z").output() else {
        return 0;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    let (sign, digits) = match text.as_bytes().first() {
        Some(b'-') => (-1, &text[1..]),
        Some(b'+') => (1, &text[1..]),
        _ => return 0,
    };
    let (Some(hours), Some(minutes)) = (digits.get(0..2), digits.get(2..4)) else {
        return 0;
    };
    match (hours.parse::<i64>(), minutes.parse::<i64>()) {
        (Ok(hours), Ok(minutes)) => sign * (hours * 3600 + minutes * 60),
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
