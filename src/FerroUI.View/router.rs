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
use std::str::FromStr;

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
    /// For a typed route, whether the values of a match parse; a match that does not is not one.
    accepts: Option<fn(&RouteMatch) -> bool>,
    view: Rc<dyn Fn(&RouteMatch) -> Element>,
}

impl RouteDef {
    /// The pattern, as it was given.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// What the pattern matches in `path`, if it matches. For a typed route, the values must
    /// also parse.
    pub fn matches(&self, path: &str) -> Option<RouteMatch> {
        let matched = match_segments(&self.segments, path)?;
        match self.accepts {
            Some(accepts) if !accepts(&matched) => None,
            _ => Some(matched),
        }
    }
}

/// What `pattern` matches in `path`.
fn match_segments(pattern: &[Segment], path: &str) -> Option<RouteMatch> {
    let parts = segments(path);
    let mut params = Vec::new();
    for (index, segment) in pattern.iter().enumerate() {
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
    (parts.len() == pattern.len()).then(|| RouteMatch { path: path.to_string(), params })
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
    RouteDef { pattern: pattern.to_string(), segments: parse(pattern), accepts: None, view: Rc::new(view) }
}

/// A route as a Rust type: its pattern, the path of a value, and the value of a match. Written
/// with `#[derive(Route)]`, which checks the pattern against the fields when the crate is compiled:
///
/// ```
/// use ferroui_view::prelude::*;
///
/// #[derive(Route, Debug, PartialEq)]
/// #[route("/people/:id")]
/// struct PersonRoute { id: u32 }
///
/// assert_eq!(PersonRoute { id: 42 }.to_path(), "/people/42");
/// assert_eq!(PersonRoute::PATTERN, "/people/:id");
/// ```
///
/// A parameter with no field does not compile:
///
/// ```compile_fail
/// use ferroui_view::prelude::*;
/// #[derive(Route)]
/// #[route("/people/:id")]
/// struct PersonRoute { name: String }
/// ```
///
/// Nor does a value of the wrong type:
///
/// ```compile_fail
/// use ferroui_view::prelude::*;
/// #[derive(Route)]
/// #[route("/people/:id")]
/// struct PersonRoute { id: u32 }
/// let _ = PersonRoute { id: "42" }.to_path();
/// ```
///
/// Paths stay the entries of the stack, so a typed route and a text route can share a router, and
/// a deep link is an ordinary path. A path whose value does not parse (`/people/abc`) does not
/// match the typed route: the router goes on to the next route, usually a last `route("*", ..)`.
///
/// A value is escaped in its segment (`/` and `%`, and an empty value is a lone `%`), and read back
/// unescaped, so any text survives the path. The field of a `*` holds the rest of the path as it
/// is, slashes included.
pub trait TypedRoute: Sized + 'static {
    /// The pattern, in the syntax of [`route`].
    const PATTERN: &'static str;

    /// The path of this value, for [`Router::navigate_to`].
    fn to_path(&self) -> String;

    /// The value of a match of the pattern, or the parameter that does not parse.
    fn from_match(matched: &RouteMatch) -> Result<Self, RouteParamError>;
}

/// A parameter of a path that is not a value of its field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteParamError {
    name: String,
    value: String,
}

impl RouteParamError {
    /// The name of the parameter.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The text of the parameter in the path.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for RouteParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the route parameter `{}` cannot be read from {:?}", self.name, self.value)
    }
}

impl std::error::Error for RouteParamError {}

/// Writes `text` as one segment of a path.
fn push_escaped(path: &mut String, text: &str) {
    if text.is_empty() {
        // A path has no empty segments; `%` alone is not an escape, so it can mean this.
        path.push('%');
        return;
    }
    for c in text.chars() {
        match c {
            '/' => path.push_str("%2F"),
            '%' => path.push_str("%25"),
            _ => path.push(c),
        }
    }
}

/// The text of a segment that [`push_escaped`] wrote; `None` for an escape that is not valid.
fn unescaped(segment: &str) -> Option<String> {
    if segment == "%" {
        return Some(String::new());
    }
    if !segment.contains('%') {
        return Some(segment.to_string());
    }
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let digits = segment.get(index + 1..index + 3)?;
            if !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(digits, 16).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Writes a parameter of a typed route; for the expansion of `#[derive(Route)]`.
#[doc(hidden)]
pub fn __push_param<T: fmt::Display>(path: &mut String, value: &T) {
    path.push('/');
    push_escaped(path, &value.to_string());
}

/// Writes the rest of the path of a typed route; for the expansion of `#[derive(Route)]`.
#[doc(hidden)]
pub fn __push_rest<T: fmt::Display>(path: &mut String, value: &T) {
    let text = value.to_string();
    let text = text.trim_start_matches('/');
    if !text.is_empty() {
        path.push('/');
        path.push_str(text);
    }
}

fn parse_value<T: FromStr>(name: &str, raw: &str, text: Option<String>) -> Result<T, RouteParamError> {
    text.and_then(|text| text.parse().ok())
        .ok_or_else(|| RouteParamError { name: name.to_string(), value: raw.to_string() })
}

/// Reads a parameter of a typed route; for the expansion of `#[derive(Route)]`.
#[doc(hidden)]
pub fn __param<T: FromStr>(matched: &RouteMatch, name: &str) -> Result<T, RouteParamError> {
    let raw = matched.param(name).unwrap_or_default();
    parse_value(name, raw, unescaped(raw))
}

/// Reads the rest of the path of a typed route; for the expansion of `#[derive(Route)]`.
#[doc(hidden)]
pub fn __rest<T: FromStr>(matched: &RouteMatch) -> Result<T, RouteParamError> {
    let raw = matched.param("*").unwrap_or_default();
    parse_value("*", raw, Some(raw.to_string()))
}

/// A route of the type `T`: the view gets the parsed value.
///
/// ```ignore
/// typed_route::<PersonRoute>(|route| rsx! { Person { id: route.id } })
/// ```
///
/// A path whose values do not parse does not match it (see [`TypedRoute`]).
pub fn typed_route<T: TypedRoute>(view: impl Fn(T) -> Element + 'static) -> RouteDef {
    typed_route_boxed::<T>(Rc::new(view))
}

fn typed_route_boxed<T: TypedRoute>(view: Rc<dyn Fn(T) -> Element>) -> RouteDef {
    RouteDef {
        pattern: T::PATTERN.to_string(),
        segments: parse(T::PATTERN),
        accepts: Some(|matched| T::from_match(matched).is_ok()),
        view: Rc::new(move |matched| match T::from_match(matched) {
            Ok(route) => view(route),
            Err(_) => Element::Empty,
        }),
    }
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

    /// Shows the view of a typed route, and puts its path on the stack.
    pub fn navigate_to<T: TypedRoute>(&self, route: &T) {
        self.navigate(&route.to_path());
    }

    /// Shows the view of a typed route with a stack that holds its path alone.
    pub fn navigate_and_reset_to<T: TypedRoute>(&self, route: &T) {
        self.navigate_and_reset(&route.to_path());
    }

    /// The current path as the route `T`, when it matches and its values parse; reactive.
    pub fn current_route<T: TypedRoute>(&self) -> Option<T> {
        let matched = match_segments(&parse(T::PATTERN), &self.current())?;
        T::from_match(&matched).ok()
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
