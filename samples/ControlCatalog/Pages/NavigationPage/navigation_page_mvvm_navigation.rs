//! Port of `Pages/NavigationPage/NavigationPageMvvmNavigation.cs`: the navigation service and
//! the page factory of the MVVM sample of the navigation page.

use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::DispatcherTask;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{ContentPage, NavigationPage, Page};
use mini_mvvm::start_async;
use std::any::Any;
use std::rc::{Rc, Weak};

/// A view model of the sample (`ViewModelBase` in the original): the page factory tells the
/// classes apart.
pub(crate) type SampleViewModel = Rc<dyn Any>;

/// The navigation the view models of the sample ask for.
pub(crate) trait ISampleNavigationService {
    /// Occurs when the state of the navigation has changed. Disposing the returned handle
    /// unsubscribes.
    fn state_changed(&self, handler: Rc<dyn Fn(&NavigationStateChangedEventArgs)>) -> Rc<dyn IDisposable>;

    fn navigate_to_async(&self, view_model: SampleViewModel) -> DispatcherTask<()>;

    fn go_back_async(&self) -> DispatcherTask<()>;

    fn pop_to_root_async(&self) -> DispatcherTask<()>;
}

/// Resolves the page of a view model.
pub(crate) trait ISamplePageFactory {
    fn create_page(&self, view_model: SampleViewModel) -> Ref<ContentPage>;
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NavigationStateChangedEventArgs {
    current_page_header: String,
    navigation_depth: i32,
    last_action: String,
}

impl NavigationStateChangedEventArgs {
    pub(crate) fn new(current_page_header: &str, navigation_depth: i32, last_action: &str) -> Self {
        Self {
            current_page_header: current_page_header.to_string(),
            navigation_depth,
            last_action: last_action.to_string(),
        }
    }

    pub(crate) fn current_page_header(&self) -> &str {
        &self.current_page_header
    }

    pub(crate) fn navigation_depth(&self) -> i32 {
        self.navigation_depth
    }

    pub(crate) fn last_action(&self) -> &str {
        &self.last_action
    }
}

/// `$"{page?.Header}"`: the text of the header of a page, empty without a header.
fn header_text(page: &Page) -> String {
    value_text(&page.header()).unwrap_or_default()
}

pub(crate) struct SampleNavigationService {
    /// The navigation page is not owned by the service, which the view model of the view
    /// that holds the navigation page owns: held weakly.
    navigation_page: WeakRef<NavigationPage>,
    page_factory: Rc<dyn ISamplePageFactory>,
    state_changed: HandlerList<dyn Fn(&NavigationStateChangedEventArgs)>,
    this: Weak<SampleNavigationService>,
}

impl SampleNavigationService {
    pub(crate) fn new(
        navigation_page: &Ref<NavigationPage>,
        page_factory: Rc<dyn ISamplePageFactory>,
    ) -> Rc<SampleNavigationService> {
        let this = Rc::new_cyclic(|this: &Weak<SampleNavigationService>| Self {
            navigation_page: navigation_page.downgrade(),
            page_factory,
            state_changed: HandlerList::new(),
            this: this.clone(),
        });

        // The handlers belong to the navigation page: they hold the service weakly.
        let weak = Rc::downgrade(&this);
        navigation_page.pushed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.publish_state(&format!("Pushed {}", header_text(&e.page())));
            }
        });
        let weak = Rc::downgrade(&this);
        navigation_page.popped(move |e| {
            if let Some(this) = weak.upgrade() {
                this.publish_state(&format!("Popped {}", header_text(&e.page())));
            }
        });
        let weak = Rc::downgrade(&this);
        navigation_page.popped_to_root(move |_| {
            if let Some(this) = weak.upgrade() {
                this.publish_state("Popped to root");
            }
        });
        this
    }

    fn publish_state(&self, last_action: &str) {
        let Some(navigation_page) = self.navigation_page.upgrade() else {
            return;
        };
        let header = navigation_page
            .current_page()
            .and_then(|page| value_text(&page.header()))
            .unwrap_or_else(|| String::from("None"));

        let e = NavigationStateChangedEventArgs::new(
            &header,
            navigation_page.navigation_stack().len() as i32,
            last_action,
        );
        for (_, handler) in self.state_changed.snapshot().iter() {
            handler(&e);
        }
    }
}

impl ISampleNavigationService for SampleNavigationService {
    fn state_changed(&self, handler: Rc<dyn Fn(&NavigationStateChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.state_changed.add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.state_changed.remove(token);
            }
        })
    }

    fn navigate_to_async(&self, view_model: SampleViewModel) -> DispatcherTask<()> {
        let navigation_page = self.navigation_page.upgrade();
        let page_factory = self.page_factory.clone();
        start_async(async move {
            let Some(navigation_page) = navigation_page else {
                return;
            };
            let page = page_factory.create_page(view_model);
            let _ = navigation_page.push_async(page).await;
        })
    }

    fn go_back_async(&self) -> DispatcherTask<()> {
        let navigation_page = self.navigation_page.upgrade();
        if navigation_page.as_ref().is_some_and(|navigation_page| navigation_page.navigation_stack().len() <= 1) {
            self.publish_state("Already at the root page");
            return start_async(async {});
        }

        start_async(async move {
            if let Some(navigation_page) = navigation_page {
                let _ = navigation_page.pop_async().await;
            }
        })
    }

    fn pop_to_root_async(&self) -> DispatcherTask<()> {
        let navigation_page = self.navigation_page.upgrade();
        if navigation_page.as_ref().is_some_and(|navigation_page| navigation_page.navigation_stack().len() <= 1) {
            self.publish_state("Already at the root page");
            return start_async(async {});
        }

        start_async(async move {
            if let Some(navigation_page) = navigation_page {
                let _ = navigation_page.pop_to_root_async().await;
            }
        })
    }
}
