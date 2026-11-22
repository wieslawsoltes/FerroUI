//! Typed routes: the path of a value, the value of a path, and what a path that does not parse
//! does.

use crate::generated::builders::*;
use crate::reactive::Owner;
use crate::router::{TypedRoute, __param};
use crate::{route, router, typed_route, use_router, IntoElement, Route, RouteDef, Router};
use ferroui_base::Ref;
use ferroui_controls::{Control, StackPanel, TextBlock};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Route, Debug, PartialEq)]
#[route("/")]
struct Home;

#[derive(Route, Debug, PartialEq)]
#[route("/people/:id")]
struct Person {
    id: u32,
}

#[derive(Route, Debug, PartialEq)]
#[route("/teams/:team/people/:name")]
struct Member {
    team: String,
    name: String,
}

#[derive(Route, Debug, PartialEq)]
#[route("/files/*")]
struct File {
    #[route(rest)]
    path: String,
}

fn children(panel: &Ref<Control>) -> Vec<Ref<Control>> {
    panel.cast::<StackPanel>().expect("a panel").children().to_vec().into_iter().collect()
}

fn texts(panel: &Ref<Control>) -> Vec<String> {
    children(panel)
        .iter()
        .map(|control| control.cast::<TextBlock>().map(|text| text.text().unwrap_or_default()).unwrap_or_default())
        .collect()
}

fn routes(found: Rc<Cell<Option<Router>>>) -> Vec<RouteDef> {
    vec![
        typed_route::<Home>(move |_| {
            found.set(Some(use_router()));
            text_block().text("home").into_element()
        }),
        typed_route::<Person>(|person| text_block().text(format!("person {}", person.id + 1)).into_element()),
        typed_route::<Member>(|m| text_block().text(format!("{} of {}", m.name, m.team)).into_element()),
        typed_route::<File>(|file| text_block().text(format!("file {}", file.path)).into_element()),
        route("/text/:id", |m| text_block().text(format!("text {}", m.param("id").unwrap_or("?"))).into_element()),
        route("*", |m| text_block().text(format!("not found {}", m.path())).into_element()),
    ]
}

#[test]
fn a_route_writes_its_path() {
    assert_eq!(Home.to_path(), "/");
    assert_eq!(Person { id: 42 }.to_path(), "/people/42");
    assert_eq!(Member { team: "core".into(), name: "ada".into() }.to_path(), "/teams/core/people/ada");
    assert_eq!(File { path: "a/b.txt".into() }.to_path(), "/files/a/b.txt");
    assert_eq!(File { path: String::new() }.to_path(), "/files");
    assert_eq!(Person::PATTERN, "/people/:id");
}

#[test]
fn a_router_navigates_with_typed_routes() {
    let owner = Owner::new_root();
    let found = Rc::new(Cell::new(None));
    let panel = owner.with(|| stack_panel().child(router("/", routes(found.clone()))).into_element().mount());
    assert_eq!(texts(&panel), ["home"]);
    let nav = found.get().expect("the view found its router");

    nav.navigate_to(&Person { id: 41 });
    assert_eq!(texts(&panel), ["person 42"], "the view gets the parsed value");
    assert_eq!(nav.current(), "/people/41", "the stack holds paths");
    assert_eq!(nav.current_route::<Person>(), Some(Person { id: 41 }));
    assert_eq!(nav.current_route::<Home>(), None);

    nav.navigate_to(&File { path: "a/b.txt".into() });
    assert_eq!(texts(&panel), ["file a/b.txt"]);
    assert!(nav.navigate_back());
    assert_eq!(texts(&panel), ["person 42"]);
    nav.navigate_and_reset_to(&Home);
    assert_eq!(nav.stack(), ["/"]);
    owner.dispose();
}

#[test]
fn a_deep_link_with_an_invalid_value_goes_to_the_next_route() {
    let owner = Owner::new_root();
    let found = Rc::new(Cell::new(None));
    let panel = owner.with(|| stack_panel().child(router("/", routes(found.clone()))).into_element().mount());
    let nav = found.get().expect("the view found its router");

    nav.navigate("/people/7");
    assert_eq!(texts(&panel), ["person 8"], "a deep link is an ordinary path");
    nav.navigate("/people/abc");
    assert_eq!(texts(&panel), ["not found /people/abc"]);
    assert_eq!(nav.current_route::<Person>(), None);
    nav.navigate("/teams/core/people/%ZZ");
    assert_eq!(texts(&panel), ["not found /teams/core/people/%ZZ"], "an escape that is not valid");
    owner.dispose();
}

#[test]
fn text_routes_behave_as_before_beside_typed_ones() {
    let owner = Owner::new_root();
    let found = Rc::new(Cell::new(None));
    let panel = owner.with(|| stack_panel().child(router("/", routes(found.clone()))).into_element().mount());
    let nav = found.get().expect("the view found its router");
    nav.navigate("/text/a%2Fb");
    assert_eq!(texts(&panel), ["text a%2Fb"], "a text route gets the segment as it is");
    nav.navigate("/nowhere");
    assert_eq!(texts(&panel), ["not found /nowhere"]);
    owner.dispose();
}

#[test]
fn any_text_survives_the_path() {
    for text in ["plain", "a/b", "50%", "%2F", "", "%", "with space", "naïve/日本語"] {
        let route = Member { team: text.to_string(), name: "x".to_string() };
        let path = route.to_path();
        let definition = typed_route::<Member>(|_| crate::Element::Empty);
        let matched = definition.matches(&path).unwrap_or_else(|| panic!("{path} matches"));
        assert_eq!(Member::from_match(&matched).unwrap(), route, "{text:?} through {path}");
    }
}

#[test]
fn a_parameter_that_does_not_parse_is_named() {
    let definition = route("/people/:id", |_| crate::Element::Empty);
    let matched = definition.matches("/people/abc").unwrap();
    let error = __param::<u32>(&matched, "id").unwrap_err();
    assert_eq!((error.name(), error.value()), ("id", "abc"));
}
