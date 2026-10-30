use super::*;
use crate::test_environment::Environment;
use ferrodock_model::services::*;
use ferroui_reactive::concurrency::HistoricalScheduler;
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

macro_rules! scopes {
    ($defaults:ident,$queued:ident,$reentrant:ident,$lifetime:ident,$service:ident,$contract:ident,$get:ident,$property:literal,$default:literal) => {
        #[test]
        fn $defaults() {
            let _environment = Environment::new();
            for message in [None, Some(""), Some(" \t\u{00a0}\u{3000}")] {
                let service: Rc<dyn $contract> = $service::new(message.map(String::from), None);
                assert!(!service.$get());
                assert!(service.message().is_none());
                let handle = service.begin(None);
                assert!(service.$get());
                assert_eq!(service.message().as_deref(), Some($default));
                handle.dispose();
            }
            let service = $service::new(Some(" custom ".into()), None);
            let first = service.begin(None);
            let second = service.begin(Some("".into()));
            assert_eq!(service.message().as_deref(), Some(""));
            first.dispose();
            first.dispose();
            assert!(service.$get());
            assert_eq!(service.message().as_deref(), Some(""));
            second.dispose();
            assert!(!service.$get());
            assert!(service.message().is_none());
            let handle = service.begin(None);
            assert_eq!(service.message().as_deref(), Some(" custom "));
            handle.dispose();
        }
        #[test]
        fn $queued() {
            let _environment = Environment::new();
            let scheduler = HistoricalScheduler::historical();
            RxSchedulers::set_main_thread_scheduler(scheduler.clone());
            let service = $service::new(None, None);
            let changes = Rc::new(RefCell::new(Vec::new()));
            let seen = changes.clone();
            let weak = Rc::downgrade(&service);
            service.property_changed().add(Rc::new(move |name| {
                let service = weak.upgrade().unwrap();
                seen.borrow_mut()
                    .push((name.to_owned(), service.$get(), service.message()));
            }));
            let first = service.begin(None);
            let second = service.begin(None);
            first.dispose();
            second.dispose();
            assert!(!service.$get());
            assert!(changes.borrow().is_empty());
            scheduler.start();
            assert_eq!(
                &*changes.borrow(),
                &[
                    ($property.into(), true, None),
                    ("Message".into(), true, Some($default.into())),
                    ($property.into(), false, Some($default.into())),
                    ("Message".into(), false, None)
                ]
            );
        }
        #[test]
        fn $reentrant() {
            let _environment = Environment::new();
            let service = $service::new(None, None);
            let reopened = Rc::new(RefCell::new(None));
            let handle = reopened.clone();
            let weak = Rc::downgrade(&service);
            let once = Cell::new(false);
            service.property_changed().add(Rc::new(move |name| {
                let service = weak.upgrade().unwrap();
                if name == $property && !service.$get() && !once.replace(true) {
                    handle.replace(Some(service.begin(Some("reopened".into()))));
                }
            }));
            let first = service.begin(None);
            first.dispose();
            assert!(service.$get()); // The outer UpdateState still clears Message after reentry.
            assert!(service.message().is_none());
            reopened.borrow_mut().take().unwrap().dispose();
            assert!(!service.$get());
        }
        #[test]
        fn $lifetime() {
            let _environment = Environment::new();
            let service = $service::new(None, None);
            let weak = Rc::downgrade(&service);
            let scope = service.begin(None);
            drop(service);
            assert!(weak.upgrade().is_some());
            scope.dispose();
            assert!(weak.upgrade().is_none());
            let service = $service::new(None, None);
            drop(service.begin(None));
            assert!(service.$get());
        }
    };
}
scopes!(
    dialog_defaults_and_nested_scopes,
    dialog_queued_notification_order,
    dialog_reentry_keeps_source_assignment_order,
    dialog_scope_lifetimes,
    DockGlobalDialogService,
    IDockGlobalDialogService,
    is_dialog_open,
    "IsDialogOpen",
    "Dialog open in another window."
);
scopes!(
    confirmation_defaults_and_nested_scopes,
    confirmation_queued_notification_order,
    confirmation_reentry_keeps_source_assignment_order,
    confirmation_scope_lifetimes,
    DockGlobalConfirmationService,
    IDockGlobalConfirmationService,
    is_confirmation_open,
    "IsConfirmationOpen",
    "Confirmation open in another window."
);
