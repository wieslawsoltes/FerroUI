use super::platform_manager_tests::TestWindowImpl;
use super::*;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, PixelRect, Size};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

struct TestScreen {
    base: PlatformScreen,
    generation: Cell<i32>,
}

impl TestScreen {
    fn new(key: i32) -> Self {
        Self {
            base: PlatformScreen::new(Rc::new(PlatformHandle::new(key as isize, Some("TestHandle")))),
            generation: Cell::new(0),
        }
    }
}

impl AsRef<PlatformScreen> for TestScreen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

#[derive(Default)]
struct TestScreens {
    base: ScreensBase<i32, TestScreen>,
    keys: RefCell<Vec<i32>>,
    count: Cell<i32>,
    details_requests: Cell<i32>,
    created: Cell<i32>,
}

impl TestScreens {
    fn push_new_screens(self: &Rc<Self>, keys: &[i32]) {
        self.count.set(keys.len() as i32);
        *self.keys.borrow_mut() = keys.to_vec();
        self.on_changed();
    }

    fn get_screen(&self, key: i32) -> Option<Rc<TestScreen>> {
        self.try_get_screen(&key)
    }
}

impl ScreensBaseImpl for TestScreens {
    type Key = i32;
    type Screen = TestScreen;

    fn screens_base(&self) -> &ScreensBase<i32, TestScreen> {
        &self.base
    }

    fn get_screen_count(&self) -> i32 {
        self.count.get()
    }

    fn get_all_screen_keys(&self) -> Vec<i32> {
        self.keys.borrow().clone()
    }

    fn create_screen_from_key(&self, key: &i32) -> Rc<TestScreen> {
        self.created.set(self.created.get() + 1);
        Rc::new(TestScreen::new(*key))
    }

    fn screen_changed(&self, screen: &Rc<TestScreen>) {
        screen.generation.set(screen.generation.get() + 1);
    }

    fn screen_removed(&self, screen: &Rc<TestScreen>) {
        screen.generation.set(-1000);
    }

    fn request_screen_details_core(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        self.details_requests.set(self.details_requests.get() + 1);
        Box::pin(std::future::ready(false))
    }
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn should_preserve_old_screens_on_changes() {
    let _scope = Dispatcher::unit_test_scope();

    let screens = Rc::new(TestScreens::default());

    assert_eq!(screens.screen_count(), 0);
    assert!(screens.all_screens().is_empty());

    // Push 2 screens.
    screens.push_new_screens(&[1, 2]);
    run_jobs();

    assert_eq!(screens.screen_count(), 2);
    let s1 = screens.get_screen(1).unwrap();
    let s2 = screens.get_screen(2).unwrap();

    // Push 3 screens, while removing one old.
    screens.push_new_screens(&[2, 3, 4]);
    run_jobs();

    assert_eq!(screens.screen_count(), 3);
    assert!(screens.get_screen(1).is_none());
    assert!(Rc::ptr_eq(&screens.get_screen(2).unwrap(), &s2));
    let s3 = screens.get_screen(3).unwrap();
    let s4 = screens.get_screen(4).unwrap();

    assert_eq!(screens.all_screens().len(), 3);
    assert_eq!(screens.screen_count(), 3);

    // This screen was removed.
    assert!(s1.generation.get() < 0);
    // This screen survived the first change, the instance should be preserved.
    assert_eq!(s2.generation.get(), 2);
    assert_eq!(s3.generation.get(), 1);
    assert_eq!(s4.generation.get(), 1);
}

#[test]
fn should_preserve_old_screens_on_changes_same_instance() {
    let _scope = Dispatcher::unit_test_scope();

    let screens = Rc::new(TestScreens::default());

    assert_eq!(screens.screen_count(), 0);
    assert!(screens.all_screens().is_empty());

    screens.push_new_screens(&[1]);
    run_jobs();

    let screen = screens.get_screen(1).unwrap();
    assert_eq!(screen.generation.get(), 1);
    assert_eq!(screen.base.try_get_platform_handle().unwrap().handle(), 1);

    screens.push_new_screens(&[1]);
    run_jobs();

    assert_eq!(screen.generation.get(), 2);
    assert_eq!(screen.base.try_get_platform_handle().unwrap().handle(), 1);
    assert!(Rc::ptr_eq(&screens.get_screen(1).unwrap(), &screen));
    // The shared screen data is the same instance too.
    assert!(Rc::ptr_eq(&screens.all_screens()[0], screen.base.screen()));
}

#[test]
fn should_raise_event_and_update_screens_on_changed() {
    let _scope = Dispatcher::unit_test_scope();

    let has_changed_times = Rc::new(Cell::new(0));
    let screens = Rc::new(TestScreens::default());
    let counter = has_changed_times.clone();
    screens.set_changed(Some(Rc::new(move || counter.set(counter.get() + 1))));

    assert_eq!(screens.screen_count(), 0);
    assert!(screens.all_screens().is_empty());

    screens.push_new_screens(&[1, 2]);
    // on_changed can be triggered multiple times by different events.
    screens.push_new_screens(&[1, 2]);
    run_jobs();

    assert_eq!(screens.screen_count(), 2);
    assert!(!screens.all_screens().is_empty());

    assert_eq!(has_changed_times.get(), 1);
}

#[test]
fn should_trigger_changed_when_screen_removed() {
    let _scope = Dispatcher::unit_test_scope();

    let screens = Rc::new(TestScreens::default());
    screens.push_new_screens(&[1, 2]);
    run_jobs();

    let has_changed_times = Rc::new(Cell::new(0));
    let screen = screens.get_screen(2).unwrap();

    let counter = has_changed_times.clone();
    let removed = screen.clone();
    screens.set_changed(Some(Rc::new(move || {
        assert!(removed.generation.get() < 0);
        counter.set(counter.get() + 1);
    })));

    screens.push_new_screens(&[1]);
    run_jobs();

    assert_eq!(has_changed_times.get(), 1);
}

#[test]
fn changed_is_not_raised_without_subscriber_or_materialized_screens() {
    let _scope = Dispatcher::unit_test_scope();

    let screens = Rc::new(TestScreens::default());
    screens.push_new_screens(&[1, 2]);
    run_jobs();

    // Nothing asked for the screens yet, so none was created.
    assert_eq!(screens.created.get(), 0);
    assert_eq!(screens.all_screens().len(), 2);
    assert_eq!(screens.created.get(), 2);
}

#[test]
fn default_removal_empties_the_screen() {
    struct DefaultScreens {
        base: ScreensBase<i32, PlatformScreen>,
        keys: RefCell<Vec<i32>>,
    }

    impl ScreensBaseImpl for DefaultScreens {
        type Key = i32;
        type Screen = PlatformScreen;

        fn screens_base(&self) -> &ScreensBase<i32, PlatformScreen> {
            &self.base
        }

        fn get_all_screen_keys(&self) -> Vec<i32> {
            self.keys.borrow().clone()
        }

        fn create_screen_from_key(&self, key: &i32) -> Rc<PlatformScreen> {
            let screen = PlatformScreen::new(Rc::new(PlatformHandle::new(*key as isize, Some("TestHandle"))));
            screen.set_bounds(PixelRect::new(100 * key, 0, 100, 100));
            screen.set_scaling(2.0);
            Rc::new(screen)
        }
    }

    let _scope = Dispatcher::unit_test_scope();

    let screens = Rc::new(DefaultScreens { base: ScreensBase::new(), keys: RefCell::new(vec![0, 1]) });
    // The default count is the number of screens.
    assert_eq!(screens.screen_count(), 2);
    let second = screens.try_get_screen(&1).unwrap();
    assert_eq!(second.bounds(), PixelRect::new(100, 0, 100, 100));

    // The default lookups go through the screen helper.
    let found = screens.screen_from_point(PixelPoint::new(150, 50)).unwrap();
    assert!(Rc::ptr_eq(&found, second.screen()));
    let found = screens.screen_from_rect(PixelRect::new(90, 0, 100, 100)).unwrap();
    assert!(Rc::ptr_eq(&found, second.screen()));
    assert!(screens.screen_from_point(PixelPoint::new(500, 50)).is_none());

    // A window is looked up by its frame; other top-levels have no screen.
    let window = TestWindowImpl::new();
    window.position.set(PixelPoint::new(120, 10));
    window.client_size.set(Size::new(20.0, 20.0));
    let found = screens.screen_from_window(&*window).unwrap();
    assert!(Rc::ptr_eq(&found, second.screen()));
    window.is_window.set(false);
    assert!(screens.screen_from_top_level(&*window).is_none());

    // The default request for the screen details is granted.
    let request = screens.request_screen_details();
    let task = Dispatcher::ui_thread().invoke_async_task_local(move || request);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(task.result(), Ok(true));

    *screens.keys.borrow_mut() = vec![0];
    screens.on_changed();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(screens.screen_count(), 1);
    assert!(screens.try_get_screen(&1).is_none());
    assert_eq!(second.bounds(), PixelRect::default());
    assert_eq!(second.scaling(), 0.0);
}

#[test]
fn request_screen_details_caches_the_outcome() {
    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::ui_thread();

    let screens = Rc::new(TestScreens::default());

    let first = screens.request_screen_details();
    let task = dispatcher.invoke_async_task_local(move || first);
    dispatcher.run_jobs(None);
    assert_eq!(task.result(), Ok(false));
    assert_eq!(screens.details_requests.get(), 1);

    let second = screens.request_screen_details();
    let task = dispatcher.invoke_async_task_local(move || second);
    dispatcher.run_jobs(None);
    assert_eq!(task.result(), Ok(false));
    assert_eq!(screens.details_requests.get(), 1);
}
