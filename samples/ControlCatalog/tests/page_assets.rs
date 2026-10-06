//! Tests of the hook through which the host of the catalog makes the assets
//! of a page available before the catalog creates the page ([`PageAssets`]).
//!
//! Not ports: the hook is an addition of the port.

use super::support::*;
use crate::models::PageItem;
use crate::view_models::MainWindowViewModel;
use crate::{IPageAssets, PageAssets, PageAssetsFuture};
use ferroui_base::threading::Dispatcher;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// The main window with the view model as its data context, shown.
pub(super) fn show_main_window(view_model: &Rc<MainWindowViewModel>) -> ferroui_base::Ref<crate::MainWindow> {
    let window = crate::MainWindow::new();
    window.set_data_context(Some(view_model.clone() as ferroui_base::BoxedValue));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

/// The pages of the drawer other than the home page and the settings page.
pub(super) fn listed_pages(view_model: &MainWindowViewModel) -> Vec<Rc<PageItem>> {
    crate::smoke::pages(view_model)
        .into_iter()
        .filter(|item| !Rc::ptr_eq(item, &view_model.home_item()) && !Rc::ptr_eq(item, &view_model.settings_item()))
        .collect()
}
/// A future that completes when the test opens it.
#[derive(Clone, Default)]
struct Gate(Rc<RefCell<(Option<Result<(), String>>, Option<Waker>)>>);

impl Gate {
    fn open(&self, outcome: Result<(), String>) {
        let waker = {
            let mut state = self.0.borrow_mut();
            state.0 = Some(outcome);
            state.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Future for Gate {
    type Output = Result<(), String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.borrow_mut();
        match state.0.take() {
            Some(outcome) => Poll::Ready(outcome),
            None => {
                state.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// A host whose page `waiting` has assets that arrive when the test opens
/// the gate; it records the pages it was asked for.
struct GatedHost {
    waiting: String,
    gate: Gate,
    asked: RefCell<Vec<String>>,
}

impl IPageAssets for GatedHost {
    fn ensure_page_assets(&self, page: &str) -> Option<PageAssetsFuture> {
        self.asked.borrow_mut().push(page.to_string());
        (page == self.waiting).then(|| Box::pin(self.gate.clone()) as PageAssetsFuture)
    }
}

fn is_current(view_model: &MainWindowViewModel, item: &Rc<PageItem>) -> bool {
    view_model.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, item))
}

#[test]
fn a_page_is_created_once_the_host_has_its_assets() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();
    let window = show_main_window(&view_model);
    let pages = listed_pages(&view_model);
    let (first, second) = (pages[0].clone(), pages[1].clone());
    let host = Rc::new(GatedHost { waiting: first.header(), gate: Gate::default(), asked: RefCell::new(Vec::new()) });
    PageAssets::set_implementation(Some(host.clone()));

    view_model.navigate_to_item(&first);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(vec![first.header()], *host.asked.borrow());
    assert!(is_current(&view_model, &view_model.home_item()), "the page was shown before its assets arrived");

    host.gate.open(Ok(()));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(is_current(&view_model, &first), "the page is not shown once its assets arrived");

    // A page whose assets are there is shown right away.
    view_model.navigate_to_item(&second);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(is_current(&view_model, &second));
    PageAssets::set_implementation(None);
    window.close();
}

#[test]
fn a_navigation_asked_for_while_assets_arrive_wins() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();
    let window = show_main_window(&view_model);
    let pages = listed_pages(&view_model);
    let (slow, fast) = (pages[0].clone(), pages[1].clone());
    let host = Rc::new(GatedHost { waiting: slow.header(), gate: Gate::default(), asked: RefCell::new(Vec::new()) });
    PageAssets::set_implementation(Some(host.clone()));

    view_model.navigate_to_item(&slow);
    Dispatcher::ui_thread().run_jobs(None);
    view_model.navigate_to_item(&fast);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(is_current(&view_model, &fast));

    host.gate.open(Ok(()));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(is_current(&view_model, &fast), "the earlier navigation replaced the page asked for later");
    PageAssets::set_implementation(None);
    window.close();
}

#[test]
fn a_page_whose_assets_do_not_arrive_is_not_created() {
    let _app = start_catalog_application();
    let view_model = MainWindowViewModel::new();
    let window = show_main_window(&view_model);
    let page = listed_pages(&view_model)[0].clone();
    let host = Rc::new(GatedHost { waiting: page.header(), gate: Gate::default(), asked: RefCell::new(Vec::new()) });
    PageAssets::set_implementation(Some(host.clone()));

    view_model.navigate_to_item(&page);
    Dispatcher::ui_thread().run_jobs(None);
    host.gate.open(Err("404 Not Found".to_string()));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(is_current(&view_model, &view_model.home_item()));
    PageAssets::set_implementation(None);
    window.close();
}
