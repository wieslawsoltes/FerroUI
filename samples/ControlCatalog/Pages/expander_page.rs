//! Port of `Pages/ExpanderPage.xaml.cs`: the class of the document
//! `Pages/ExpanderPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::ExpanderPageViewModel;
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, Expander};
use std::{cell::RefCell, rc::Rc};

#[repr(C)]
pub struct ExpanderPage {
    base: ContentPage,
    subscriptions: RefCell<Option<Rc<ExpanderSubscriptions>>>,
}

struct ExpanderSubscriptions {
    collapsing: Ref<Expander>,
    expanding: Ref<Expander>,
    collapsing_token: RoutedEventHandlerToken,
    expanding_token: RoutedEventHandlerToken,
}
impl ExpanderSubscriptions {
    fn new(collapsing: Ref<Expander>, expanding: Ref<Expander>) -> Rc<Self> {
        let collapsing_token = collapsing.collapsing(|_, e| e.set_cancel(true));
        let expanding_token = expanding.expanding(|_, e| e.set_cancel(true));
        Rc::new(Self {
            collapsing,
            expanding,
            collapsing_token,
            expanding_token,
        })
    }
}
impl Drop for ExpanderSubscriptions {
    fn drop(&mut self) {
        self.collapsing
            .remove_handler(&Expander::collapsing_event(), self.collapsing_token);
        self.expanding
            .remove_handler(&Expander::expanding_event(), self.expanding_token);
    }
}

content_page_class!(ExpanderPage);
ferro_class_info!(ExpanderPage {
    new: ExpanderPage::new
});
xaml_class!(ExpanderPage, "/Pages/ExpanderPage.xaml");

impl ExpanderPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            subscriptions: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ExpanderPageViewModel::new() as BoxedValue));

        *this.subscriptions.borrow_mut() = Some(ExpanderSubscriptions::new(
            this.collapsing_disabled_expander(),
            this.expanding_disabled_expander(),
        ));
        this
    }

    fn collapsing_disabled_expander(&self) -> Ref<Expander> {
        self.get_control::<Expander>("CollapsingDisabledExpander")
    }

    fn expanding_disabled_expander(&self) -> Ref<Expander> {
        self.get_control::<Expander>("ExpandingDisabledExpander")
    }
}

#[cfg(all(feature = "hot-reload", debug_assertions))]
mod reload {
    use super::*;
    use ferroui_base::{controls::NameScopeRef, StyledElement, WeakRef};
    use ferroui_hot_reload::{ComponentRefresh, PreparedReload, ReloadError};

    struct Refresh(WeakRef<ExpanderPage>);
    struct Change {
        page: Ref<ExpanderPage>,
        before: Option<Rc<ExpanderSubscriptions>>,
        after: Rc<ExpanderSubscriptions>,
    }
    impl ComponentRefresh for Refresh {
        fn prepare(
            &self,
            _: &Ref<StyledElement>,
            scope: Option<&NameScopeRef>,
        ) -> Result<Box<dyn PreparedReload>, ReloadError> {
            let page = self
                .0
                .upgrade()
                .ok_or_else(|| ReloadError::Failed("ExpanderPage closed".into()))?;
            let find = |name| {
                scope
                    .and_then(|scope| scope.find(name))
                    .and_then(|control| control.cast::<Expander>())
                    .ok_or_else(|| ReloadError::Failed(format!("{name} must remain an Expander")))
            };
            // Subscribe only to detached candidate children. No live values or
            // view-model state are modified during preparation.
            let after = ExpanderSubscriptions::new(
                find("CollapsingDisabledExpander")?,
                find("ExpandingDisabledExpander")?,
            );
            let before = page.subscriptions.borrow().clone();
            Ok(Box::new(Change {
                page,
                before,
                after,
            }))
        }
    }
    impl PreparedReload for Change {
        fn commit(&mut self) -> Result<(), ReloadError> {
            *self.page.subscriptions.borrow_mut() = Some(self.after.clone());
            Ok(())
        }
        fn rollback(&mut self) {
            *self.page.subscriptions.borrow_mut() = self.before.clone();
        }
        fn finish(&mut self) {
            self.before.take();
        }
    }
    impl ExpanderPage {
        pub(crate) fn reload_refresh(&self) -> Rc<dyn ComponentRefresh> {
            Rc::new(Refresh(self.to_ref().downgrade()))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ferroui_base::{
            controls::NameScope, interactivity::CancelRoutedEventArgs, threading::Dispatcher,
        };

        fn candidate() -> (NameScopeRef, Ref<Expander>, Ref<Expander>) {
            let scope = NameScopeRef::new(NameScope::new());
            let collapsing = Expander::new();
            let expanding = Expander::new();
            scope
                .try_register("CollapsingDisabledExpander", collapsing.clone().upcast())
                .unwrap();
            scope
                .try_register("ExpandingDisabledExpander", expanding.clone().upcast())
                .unwrap();
            (scope, collapsing, expanding)
        }
        fn cancelled(control: &Expander, expanding: bool) -> bool {
            let event = if expanding {
                Expander::expanding_event()
            } else {
                Expander::collapsing_event()
            };
            let args = CancelRoutedEventArgs::with_event(&event);
            control.raise_event(&args);
            args.cancel()
        }

        #[test]
        fn expander_refresh_releases_retired_handlers_and_restores_on_rollback() {
            let _dispatcher = Dispatcher::unit_test_scope();
            let page = instantiate(ExpanderPage::construct());
            let (_, mut old_collapsing, mut old_expanding) = candidate();
            *page.subscriptions.borrow_mut() = Some(ExpanderSubscriptions::new(
                old_collapsing.clone(),
                old_expanding.clone(),
            ));
            let refresh = page.reload_refresh();
            for _ in 0..10 {
                let (scope, collapsing, expanding) = candidate();
                let mut change = refresh
                    .prepare(&page.clone().upcast(), Some(&scope))
                    .unwrap();
                assert!(cancelled(&old_collapsing, false));
                change.commit().unwrap();
                assert!(page
                    .subscriptions
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .expanding
                    .ptr_eq(&expanding));
                assert!(
                    cancelled(&old_expanding, true),
                    "rollback graph must retain its handlers"
                );
                change.rollback();
                drop(change);
                assert!(!cancelled(&collapsing, false));
                assert!(!cancelled(&expanding, true));
                assert!(page
                    .subscriptions
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .expanding
                    .ptr_eq(&old_expanding));

                let mut change = refresh
                    .prepare(&page.clone().upcast(), Some(&scope))
                    .unwrap();
                change.commit().unwrap();
                change.finish();
                drop(change);
                assert!(!cancelled(&old_collapsing, false));
                assert!(!cancelled(&old_expanding, true));
                assert!(cancelled(&collapsing, false));
                assert!(cancelled(&expanding, true));
                let retired = old_expanding.downgrade();
                old_collapsing = collapsing;
                old_expanding = expanding;
                assert!(
                    retired.upgrade().is_none(),
                    "old subscription must not retain detached control"
                );
            }
            assert!(refresh.prepare(&page.clone().upcast(), None).is_err());
            assert!(cancelled(&old_expanding, true));
            drop(page);
            assert!(
                !cancelled(&old_expanding, true),
                "refresh adapter must not retain the page"
            );
        }
    }
}
