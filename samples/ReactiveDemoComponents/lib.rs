//! reactive-demo-components
//!
//! `ReactiveDemo`, the getting-started sample of ReactiveUI (a browser of the crates of
//! crates.io with a throttled search, ported with XAML and view models in `reactive-demo`),
//! written again with the declarative API of `ferroui-view`: components that run again, with
//! their state in hooks, instead of documents, view models and bindings. It runs in the WinUI 3
//! theme.
//!
//! What the reactive pipeline of the view model does, the components say with the API of
//! phase 6 (`docs/declarative-ui.md`, section 14):
//!
//! - the search is a future of the trimmed search text (`use_future`): it waits 800 ms (the
//!   throttle of the reference) and asks the search service of `reactive-demo`; a new text drops
//!   the future of the previous one, so only the text the user stopped typing at is searched;
//! - `suspense` shows a progress bar while the search runs, the results when it ends, its error
//!   when it fails;
//! - a result opens the page of its crate, a route of the router (`/crate/:name`); the search is
//!   the route `/`;
//! - the page of a crate is inside an error boundary, whose replacement shows the error and
//!   tries again.
//!
//! - [`view`]: the components ([`App`] and those below it) and the state they share
//!   ([`Browser`]).
//! - [`run`]: the application in a window, and its picture with
//!   `FERROUI_REACTIVE_DEMO_SHOTS=<directory>`.
//!
//! `cargo run -p reactive-demo-components`.

mod shots;
pub mod view;

pub use shots::{capture_if_asked, run};
pub use view::{app, details_path, App, Browser, THROTTLE};

#[cfg(test)]
mod tests;
