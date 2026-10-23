//! Navigation between the views of a window ([`router`]), modelled on the
//! routing of ReactiveUI (`RoutingState`, `docs/mvvm.md`, section 7).
//!
//! | ReactiveUI | Here |
//! |---|---|
//! | `RoutingState` | [`Router`], provided by [`router`] to the components below |
//! | `NavigationStack` | [`Router::stack`]: the paths, the current one last |
//! | `Navigate` | [`Router::navigate`]: pushes a path |
//! | `NavigateBack` | [`Router::navigate_back`]: pops the current path |
//! | `NavigateAndReset` | [`Router::navigate_and_reset`]: the stack becomes the path alone |
//! | `CurrentViewModel` | [`Router::current`]: the current path |
//! | `RoutedViewHost` and the view locator | the routes: the view of the first pattern that matches |
//!
//! The entries of the stack are paths rather than view models: a path names
//! its view and its parameters (`/people/42`), so the state of a screen that
//! outlives navigation lives in signals or contexts above the router, as the
//! view models of ReactiveUI live in the screen.

use crate::element::{rerender, with_key, Element, Key};
use crate::hooks::use_signal;
use crate::reactive::{provide_context, use_context, Signal};
use std::fmt;
use std::rc::Rc;

/// What a route's pattern matched: the path, and the values of the
/// parameters of the pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteMatch {
    path: String,
    params: Vec<(String, String)>,
}

impl RouteMatch {
    /// The path that matched.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The value of the parameter `name` (`:name` in the pattern). The rest
    /// of the path that `*` matched is the parameter `*`, without its first
    /// slash (empty when nothing was left).
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// The parameters in the order of the pattern.
    pub fn params(&self) -> &[(String, String)] {
        &self.params
    }
}

/// A segment of a pattern.
#[derive(Clone, Debug, PartialEq)]
enum Segment {
    /// Matches this text.
    Literal(String),
    /// Matches one segment and binds it.
    Param(String),
    /// Matches the rest of the path.
    Rest,
}

/// The segments of a path: the parts between slashes, empty ones left out,
/// so that `/a/b`, `a/b` and `/a/b/` are the same path.
fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|segment| !segment.is_empty()).collect()
}

fn parse(pattern: &str) -> Vec<Segment> {
    let segments: Vec<Segment> = segments(pattern)
        .into_iter()
        .map(|segment| match segment {
            "*" => Segment::Rest,
            _ => match segment.strip_prefix(':') {
                Some(name) => Segment::Param(name.to_string()),
                None => Segment::Literal(segment.to_string()),
            },
        })
        .collect();
    if let Some(rest) = segments.iter().position(|segment| *segment == Segment::Rest) {
        assert!(rest + 1 == segments.len(), "the pattern {pattern:?} has segments after `*`: `*` matches the rest of a path, so it comes last");
    }
    segments
}

/// A route: a pattern and the view it shows (see [`route`]).
pub struct RouteDef {
    pattern: String,
    segments: Vec<Segment>,
    view: Rc<dyn Fn(&RouteMatch) -> Element>,
}

impl RouteDef {
    /// The pattern, as it was given.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// What the pattern matches in `path`, if it matches.
    pub fn matches(&self, path: &str) -> Option<RouteMatch> {
        let parts = segments(path);
        let mut params = Vec::new();
        for (index, segment) in self.segments.iter().enumerate() {
            match segment {
                Segment::Rest => {
                    let rest = parts.get(index..).unwrap_or(&[]).join("/");
                    params.push((String::from("*"), rest));
                    return Some(RouteMatch { path: path.to_string(), params });
                }
                Segment::Literal(literal) => {
                    if parts.get(index).copied() != Some(literal.as_str()) {
                        return None;
                    }
                }
                Segment::Param(name) => match parts.get(index) {
                    Some(value) => params.push((name.clone(), (*value).to_string())),
                    None => return None,
                },
            }
        }
        (parts.len() == self.segments.len()).then(|| RouteMatch { path: path.to_string(), params })
    }
}

impl fmt::Debug for RouteDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteDef").field("pattern", &self.pattern).finish()
    }
}

/// A route: a pattern of segments separated by `/`, where `:name` matches one
/// segment and binds it to the parameter `name`, `*` matches the rest of the
/// path (and comes last), and any other segment matches itself. `view` makes
/// the element of the route from what the pattern matched.
///
/// ```ignore
/// route("/", |_| rsx! { Home {} })
/// route("/people/:id", |m| rsx! { Person { id: m.param("id").unwrap_or_default().to_string() } })
/// route("*", |m| rsx! { TextBlock { text: format!("Nothing at {}", m.path()) } })
/// ```
///
/// # Panics
/// When a segment follows `*`.
pub fn route(pattern: &str, view: impl Fn(&RouteMatch) -> Element + 'static) -> RouteDef {
    RouteDef { pattern: pattern.to_string(), segments: parse(pattern), view: Rc::new(view) }
}

/// The router of the nearest [`router`] above: its stack of paths, read
/// reactively and changed by navigation. A handle: `Copy`, cheap to clone,
/// and given to handlers as it is.
#[derive(Clone, Copy, PartialEq)]
pub struct Router {
    stack: Signal<Vec<String>>,
}

impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router").finish_non_exhaustive()
    }
}

impl Router {
    /// Shows the view of `path`, and puts it on the stack (`Navigate`).
    /// Nothing happens once the router was taken out.
    pub fn navigate(&self, path: &str) {
        if self.stack.is_alive() {
            self.stack.update(|stack| stack.push(path.to_string()));
        }
    }

    /// Goes back to the path before the current one, taking the current one
    /// off the stack (`NavigateBack`); false when the stack holds one path
    /// only, which stays.
    pub fn navigate_back(&self) -> bool {
        if !self.stack.is_alive() || self.stack.with_untracked(|stack| stack.len() < 2) {
            return false;
        }
        self.stack.update(|stack| {
            stack.pop();
        });
        true
    }

    /// Shows the view of `path` with a stack that holds `path` alone
    /// (`NavigateAndReset`).
    pub fn navigate_and_reset(&self, path: &str) {
        if self.stack.is_alive() {
            self.stack.set(vec![path.to_string()]);
        }
    }

    /// The current path: the last of the stack. Reading it makes the reader
    /// (a component, a memo, a property) follow it.
    pub fn current(&self) -> String {
        self.stack.with(|stack| stack.last().cloned().unwrap_or_default())
    }

    /// Whether there is a path to go back to; reactive.
    pub fn can_go_back(&self) -> bool {
        self.stack.with(|stack| stack.len() > 1)
    }

    /// The paths of the stack, the current one last (`NavigationStack`);
    /// reactive.
    pub fn stack(&self) -> Vec<String> {
        self.stack.get()
    }
}

/// Provides a [`Router`] to the components below, with `initial` as the only
/// path of its stack, and shows the view of the first route whose pattern
/// matches the current path: the view of a last route `route("*", ..)` when no
/// other matches, and nothing when none matches at all.
///
/// ```ignore
/// router("/", vec![
///     route("/", |_| rsx! { Home {} }),
///     route("/people/:id", |m| rsx! { Person { id: m.param("id").unwrap_or_default().to_string() } }),
///     route("*", |m| rsx! { NotFound { path: m.path().to_string() } }),
/// ])
/// ```
///
/// The view of the current route is a component of its own that runs again
/// when the path changes. Another route mounts its view anew, and the view
/// of the route before is taken out with its state. The same route with
/// other parameters runs its view again with the new [`RouteMatch`], and the
/// reconciler patches its controls. A component below finds the router with
/// [`use_router`] while it runs, and keeps the handle for its handlers:
///
/// ```ignore
/// #[component(rerender)]
/// fn Home() -> Element {
///     let router = use_router();
///     rsx! { Button { on_click: move |_| router.navigate("/people/42"), "Open" } }
/// }
/// ```
///
/// As any component, a router at the root of a mounted tree keeps the class
/// of the control its views make; it is usually the content of a window or a
/// child of a panel.
pub fn router(initial: &str, routes: Vec<RouteDef>) -> Element {
    let initial = initial.to_string();
    let routes: Rc<Vec<RouteDef>> = Rc::new(routes);
    rerender(move || {
        let initial = initial.clone();
        let stack = use_signal(move || vec![initial]);
        let router = Router { stack };
        provide_context(router);
        let path = router.current();
        let found = routes.iter().enumerate().find_map(|(index, route)| route.matches(&path).map(|matched| (index, matched)));
        match found {
            Some((index, matched)) => {
                let view = routes[index].view.clone();
                // The key is the route: another route mounts its view anew.
                with_key(rerender(move || view(&matched)), Key::Int(index as i64))
            }
            None => Element::Empty,
        }
    })
}

/// The router of the nearest [`router`] above the component that runs (or
/// the owner that is current).
///
/// # Panics
/// When there is no router above.
pub fn use_router() -> Router {
    try_use_router().unwrap_or_else(|| {
        panic!("use_router: there is no router above; call it in a component below `router(..)`, while it runs (or use try_use_router)")
    })
}

/// The router of the nearest [`router`] above, if any.
pub fn try_use_router() -> Option<Router> {
    use_context::<Router>()
}
