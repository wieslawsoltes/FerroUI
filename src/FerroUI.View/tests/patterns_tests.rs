//! Phase 6: patterns of larger applications. Futures run by a component and
//! their state, error boundaries, and the router.

use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{
    clear_diagnostics_handler, error_boundary, rerender, route, router, set_diagnostics_handler, show, suspense,
    try_use_router, use_future, use_hook, use_router, AsyncState, Diagnostic, Element, ErrorInfo, IntoElement, RouteDef,
    Router,
};
use ferroui_base::Ref;
use ferroui_controls::{Control, StackPanel, TextBlock};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn children(panel: &Ref<Control>) -> Vec<Ref<Control>> {
    panel.cast::<StackPanel>().expect("a panel").children().to_vec().into_iter().collect()
}

fn text_of(control: &Ref<Control>) -> String {
    control.cast::<TextBlock>().map(|text| text.text().unwrap_or_default()).unwrap_or_else(|| String::from("?"))
}

fn texts(panel: &Ref<Control>) -> Vec<String> {
    children(panel).iter().map(text_of).collect()
}

/// A future that ends when the test opens it.
struct Gate<T> {
    inner: Rc<RefCell<(Option<Result<T, String>>, Option<Waker>)>>,
}

impl<T> Clone for Gate<T> {
    fn clone(&self) -> Self {
        Gate { inner: self.inner.clone() }
    }
}

impl<T: 'static> Gate<T> {
    fn new() -> Self {
        Gate { inner: Rc::new(RefCell::new((None, None))) }
    }

    /// Ends the futures waiting on the gate with `outcome`; the executor
    /// polls them at once.
    fn open(&self, outcome: Result<T, String>) {
        let waker = {
            let mut inner = self.inner.borrow_mut();
            inner.0 = Some(outcome);
            inner.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn wait(&self) -> impl Future<Output = Result<T, String>> + 'static {
        let inner = self.inner.clone();
        std::future::poll_fn(move |context: &mut Context<'_>| {
            let mut inner = inner.borrow_mut();
            match inner.0.take() {
                Some(outcome) => Poll::Ready(outcome),
                None => {
                    inner.1 = Some(context.waker().clone());
                    Poll::Pending
                }
            }
        })
    }
}

fn describe(state: &AsyncState<i32>) -> String {
    match state {
        AsyncState::Pending => String::from("pending"),
        AsyncState::Ready(value) => format!("ready {value}"),
        AsyncState::Failed(error) => format!("failed {error}"),
    }
}

#[test]
fn a_future_is_pending_until_it_ends_ready_or_failed() {
    let owner = Owner::new_root();
    let (ok, bad) = (Gate::<i32>::new(), Gate::<i32>::new());
    let panel = owner.with(|| {
        let (ok, bad) = (ok.clone(), bad.clone());
        stack_panel()
            .child(rerender(move || {
                let ok = ok.clone();
                let state = use_future((), move |_| ok.wait());
                text_block().text(describe(&state.get()))
            }))
            .child(rerender(move || {
                let bad = bad.clone();
                let state = use_future((), move |_| bad.wait());
                text_block().text(describe(&state.get()))
            }))
            .into_element()
            .mount()
    });
    assert_eq!(texts(&panel), vec!["pending", "pending"]);
    ok.open(Ok(7));
    assert_eq!(texts(&panel), vec!["ready 7", "pending"]);
    bad.open(Err(String::from("offline")));
    assert_eq!(texts(&panel), vec!["ready 7", "failed offline"]);

    assert!(AsyncState::<i32>::Pending.is_pending());
    assert_eq!(AsyncState::Ready(1).value(), Some(&1));
    assert_eq!(AsyncState::<i32>::Failed(String::from("x")).error(), Some("x"));
    owner.dispose();
}

#[test]
fn new_dependencies_start_a_new_future_and_abandon_the_old_one() {
    let owner = Owner::new_root();
    let gates: Rc<Vec<Gate<String>>> = Rc::new((0..3).map(|_| Gate::new()).collect());
    let started = Rc::new(Cell::new(0));
    let (query, panel) = owner.with(|| {
        let query = signal(0_usize);
        let (gates, started) = (gates.clone(), started.clone());
        let panel = stack_panel()
            .child(rerender(move || {
                let (gates, started) = (gates.clone(), started.clone());
                let state = use_future(query.get(), move |query: &usize| {
                    started.set(started.get() + 1);
                    gates[*query].wait()
                });
                let text = match state.get() {
                    AsyncState::Pending => String::from("pending"),
                    AsyncState::Ready(text) => text,
                    AsyncState::Failed(error) => error,
                };
                text_block().text(text)
            }))
            .into_element()
            .mount();
        (query, panel)
    });
    assert_eq!(started.get(), 1);
    query.set(1);
    assert_eq!(started.get(), 2, "other dependencies start another future");
    gates[0].open(Ok(String::from("old")));
    assert_eq!(texts(&panel), vec!["pending"], "the result of an abandoned future is ignored");
    gates[1].open(Ok(String::from("one")));
    assert_eq!(texts(&panel), vec!["one"]);
    query.set(2);
    assert_eq!(texts(&panel), vec!["pending"], "new dependencies make the state pending again");
    gates[2].open(Ok(String::from("two")));
    assert_eq!(texts(&panel), vec!["two"]);
    assert_eq!(started.get(), 3);
    owner.dispose();
}

#[test]
fn a_future_whose_component_ended_is_dropped_and_changes_nothing() {
    struct DropFlag(Rc<Cell<bool>>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let owner = Owner::new_root();
    let gate = Gate::<i32>::new();
    let dropped = Rc::new(Cell::new(false));
    let (visible, panel) = owner.with(|| {
        let visible = signal(true);
        let (gate, dropped) = (gate.clone(), dropped.clone());
        let panel = stack_panel()
            .child(show(
                move || visible.get(),
                move || {
                    let (gate, dropped) = (gate.clone(), dropped.clone());
                    rerender(move || {
                        let (gate, dropped) = (gate.clone(), dropped.clone());
                        let state = use_future((), move |_| {
                            let flag = DropFlag(dropped.clone());
                            let wait = gate.wait();
                            async move {
                                let _flag = flag;
                                wait.await
                            }
                        });
                        text_block().text(describe(&state.get()))
                    })
                },
            ))
            .into_element()
            .mount();
        (visible, panel)
    });
    assert_eq!(texts(&panel), vec!["pending"]);
    assert!(!dropped.get());
    visible.set(false);
    assert!(texts(&panel).is_empty());
    assert!(dropped.get(), "the executor dropped the future of the component that ended");
    gate.open(Ok(1));
    assert!(texts(&panel).is_empty(), "a future that ends after its component changes nothing");
    owner.dispose();
}

#[test]
fn suspense_shows_the_three_states() {
    let owner = Owner::new_root();
    let (state, panel) = owner.with(|| {
        let state = signal(AsyncState::<i32>::Pending);
        let panel = stack_panel()
            .child(suspense(
                state,
                || text_block().text("loading").into_element(),
                |value| text_block().text(format!("value {value}")).into_element(),
                |error| text_block().text(format!("error {error}")).into_element(),
            ))
            .into_element()
            .mount();
        (state, panel)
    });
    assert_eq!(texts(&panel), vec!["loading"]);
    let first = children(&panel)[0].clone();
    state.set(AsyncState::Ready(3));
    assert_eq!(texts(&panel), vec!["value 3"]);
    assert!(first.ptr_eq(&children(&panel)[0]), "the reconciler patches the controls of the states");
    state.set(AsyncState::Failed(String::from("offline")));
    assert_eq!(texts(&panel), vec!["error offline"]);
    state.set(AsyncState::Pending);
    assert_eq!(texts(&panel), vec!["loading"]);
    owner.dispose();
}

fn fallback(info: ErrorInfo) -> Element {
    text_block().text(format!("fallback: {}", info.message())).into_element()
}

#[test]
fn a_panic_on_the_first_run_shows_the_fallback() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    set_diagnostics_handler(move |diagnostic| sink.borrow_mut().push(diagnostic.clone()));
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(text_block().text("before"))
            .child(error_boundary(|| rerender(|| -> Element { panic!("broken") }), fallback))
            .child(text_block().text("after"))
            .into_element()
            .mount()
    });
    assert_eq!(texts(&panel), vec!["before", "fallback: broken", "after"]);
    assert!(
        seen.borrow().iter().any(|d| matches!(d, Diagnostic::ErrorCaught { message, .. } if message == "broken")),
        "{:?}",
        seen.borrow()
    );
    clear_diagnostics_handler();
    owner.dispose();
}

#[test]
fn a_panic_on_a_later_run_shows_the_fallback_and_the_rest_still_works() {
    let owner = Owner::new_root();
    let (fail, other, panel) = owner.with(|| {
        let fail = signal(false);
        let other = signal(0);
        let panel = stack_panel()
            .child(error_boundary(
                move || {
                    rerender(move || {
                        if fail.get() {
                            panic!("later");
                        }
                        text_block().text("child").into_element()
                    })
                },
                fallback,
            ))
            .child(rerender(move || text_block().text(format!("other {}", other.get()))))
            .into_element()
            .mount();
        (fail, other, panel)
    });
    assert_eq!(texts(&panel), vec!["child", "other 0"]);
    fail.set(true);
    assert_eq!(texts(&panel), vec!["fallback: later", "other 0"]);
    other.set(1);
    assert_eq!(texts(&panel), vec!["fallback: later", "other 1"], "the components beside the boundary still run");
    owner.dispose();
}

#[test]
fn reset_mounts_the_child_again() {
    let owner = Owner::new_root();
    let last: Rc<RefCell<Option<ErrorInfo>>> = Rc::new(RefCell::new(None));
    let mounts = Rc::new(Cell::new(0));
    let (broken, panel) = owner.with(|| {
        let broken = signal(true);
        let (slot, mounts) = (last.clone(), mounts.clone());
        let panel = stack_panel()
            .child(error_boundary(
                move || {
                    let mounts = mounts.clone();
                    rerender(move || {
                        use_hook(|| mounts.set(mounts.get() + 1));
                        if broken.get() {
                            panic!("not yet");
                        }
                        text_block().text("child").into_element()
                    })
                },
                move |info| {
                    *slot.borrow_mut() = Some(info.clone());
                    fallback(info)
                },
            ))
            .into_element()
            .mount();
        (broken, panel)
    });
    assert_eq!(texts(&panel), vec!["fallback: not yet"]);
    assert_eq!(mounts.get(), 1);
    broken.set(false);
    assert_eq!(texts(&panel), vec!["fallback: not yet"], "the child that panicked was taken out");
    let info = last.borrow().clone().expect("the fallback was given its information");
    info.reset();
    assert_eq!(texts(&panel), vec!["child"]);
    assert_eq!(mounts.get(), 2, "a reset mounts the child anew");
    owner.dispose();
}

#[test]
fn the_nearest_boundary_catches_and_a_panic_of_a_fallback_goes_to_the_one_above() {
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(error_boundary(
                || {
                    stack_panel()
                        .child(text_block().text("outer child"))
                        .child(error_boundary(|| panic!("inner"), |info| {
                            text_block().text(format!("inner fallback: {}", info.message())).into_element()
                        }))
                        .into_element()
                },
                |info| text_block().text(format!("outer fallback: {}", info.message())).into_element(),
            ))
            .child(error_boundary(
                || error_boundary(|| panic!("first"), |_| panic!("the fallback too")),
                |info| text_block().text(format!("outer fallback: {}", info.message())).into_element(),
            ))
            .into_element()
            .mount()
    });
    let shown = children(&panel);
    assert_eq!(texts(&shown[0]), vec!["outer child", "inner fallback: inner"], "the inner boundary caught");
    assert_eq!(text_of(&shown[1]), "outer fallback: the fallback too");
    owner.dispose();
}

#[test]
#[should_panic(expected = "no boundary")]
fn without_a_boundary_a_panic_propagates() {
    let owner = Owner::new_root();
    owner.with(|| {
        let _ = stack_panel().child(rerender(|| -> Element { panic!("no boundary") })).into_element().mount();
    });
}

fn routes(found: Rc<Cell<Option<Router>>>) -> Vec<RouteDef> {
    vec![
        route("/", move |_| {
            found.set(Some(use_router()));
            text_block().text("home").into_element()
        }),
        route("/users/:id", |m| text_block().text(format!("user {}", m.param("id").unwrap_or("?"))).into_element()),
        route("/files/*", |m| text_block().text(format!("file {}", m.param("*").unwrap_or(""))).into_element()),
        route("*", |m| text_block().text(format!("not found {}", m.path())).into_element()),
    ]
}

#[test]
fn a_router_shows_the_view_of_the_current_path_and_navigates() {
    let owner = Owner::new_root();
    let found = Rc::new(Cell::new(None));
    let panel = owner.with(|| stack_panel().child(router("/", routes(found.clone()))).into_element().mount());
    assert_eq!(texts(&panel), vec!["home"]);
    let nav = found.get().expect("the view found its router");
    assert_eq!(nav.stack(), vec!["/"]);
    assert!(!nav.can_go_back());
    assert!(!nav.navigate_back(), "the only path stays");

    nav.navigate("/users/42");
    assert_eq!(texts(&panel), vec!["user 42"]);
    let user = children(&panel)[0].clone();
    nav.navigate("/users/7");
    assert_eq!(texts(&panel), vec!["user 7"]);
    assert!(user.ptr_eq(&children(&panel)[0]), "the same route with other parameters patches its view");
    assert_eq!(nav.stack(), vec!["/", "/users/42", "/users/7"]);
    assert!(nav.can_go_back());

    assert!(nav.navigate_back());
    assert_eq!(texts(&panel), vec!["user 42"]);
    assert_eq!(nav.current(), "/users/42");
    nav.navigate("/files/a/b.txt");
    assert_eq!(texts(&panel), vec!["file a/b.txt"]);
    nav.navigate("/nowhere");
    assert_eq!(texts(&panel), vec!["not found /nowhere"]);
    nav.navigate_and_reset("/");
    assert_eq!(texts(&panel), vec!["home"]);
    assert_eq!(nav.stack(), vec!["/"]);
    assert!(!nav.can_go_back());
    owner.dispose();
}

#[test]
fn a_component_that_reads_the_current_path_runs_again() {
    let owner = Owner::new_root();
    let found = Rc::new(Cell::new(None));
    let _panel = owner.with(|| stack_panel().child(router("/", routes(found.clone()))).into_element().mount());
    let nav = found.get().expect("the view found its router");
    let runs = Rc::new(Cell::new(0));
    let status = owner.with(|| {
        let runs = runs.clone();
        rerender(move || {
            runs.set(runs.get() + 1);
            text_block().text(format!("at {}", nav.current()))
        })
        .into_element()
        .mount()
    });
    assert_eq!(text_of(&status), "at /");
    nav.navigate("/users/1");
    assert_eq!(text_of(&status), "at /users/1");
    assert_eq!(runs.get(), 2);
    owner.dispose();
}

#[test]
fn a_path_no_route_matches_shows_nothing_and_a_router_is_found_only_below_one() {
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(router("/missing", vec![route("/", |_| text_block().text("home").into_element())]))
            .into_element()
            .mount()
    });
    assert!(texts(&panel).is_empty());
    assert!(owner.with(try_use_router).is_none());
    let segments = route("/a/:b/*", |_| Element::Empty);
    let matched = segments.matches("/a/x/y/z").expect("the pattern matches");
    assert_eq!(matched.param("b"), Some("x"));
    assert_eq!(matched.param("*"), Some("y/z"));
    assert!(segments.matches("/a").is_none());
    assert!(route("/a/:b", |_| Element::Empty).matches("/a/x/y").is_none());
    owner.dispose();
}

#[test]
#[should_panic(expected = "there is no router above")]
fn use_router_without_a_router_panics() {
    let owner = Owner::new_root();
    owner.with(|| {
        let _ = use_router();
    });
}
