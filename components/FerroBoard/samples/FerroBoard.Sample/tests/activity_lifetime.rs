//! Activity recreation must own independent sessions instead of reparenting one process view.
use ferroboard_controls::ScreenKeyboardHost;
use ferroboard_sample::{
    App, MainView,
    sample_storage::{SampleStorage, SampleStoragePlatform, StorageResult},
};
use ferroui_base::{Ref, controls::NameScope, interactivity::RoutedEventArgs};
use ferroui_controls::{
    AppBuilder, Button, Control, UserControl,
    application_lifetimes::{
        IActivityApplicationLifetime, IApplicationLifetime, ISingleViewApplicationLifetime,
    },
};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};
#[derive(Default)]
struct Storage {
    value: RefCell<Option<String>>,
    writes: Cell<usize>,
    on_write: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl SampleStoragePlatform for Storage {
    fn get_setting(&self, _: &str) -> StorageResult<Option<String>> {
        Ok(self.value.borrow().clone())
    }
    fn set_setting(&self, _: &str, v: &str) -> StorageResult<()> {
        self.value.replace(Some(v.to_owned()));
        self.writes.set(self.writes.get() + 1);
        let callback = self.on_write.take();
        if let Some(callback) = callback {
            callback();
        }
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(None)
    }
    fn log(&self, m: &str) {
        panic!("unexpected storage error: {m}");
    }
}
type ViewFactory = Rc<dyn Fn() -> Ref<Control>>;
#[derive(Default)]
struct ActivityLifetime {
    factory: RefCell<Option<ViewFactory>>,
}
impl IApplicationLifetime for ActivityLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_activity_application_lifetime(&self) -> Option<&dyn IActivityApplicationLifetime> {
        Some(self)
    }
    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }
}
impl IActivityApplicationLifetime for ActivityLifetime {
    fn main_view_factory(&self) -> Option<Rc<dyn Fn() -> Ref<Control>>> {
        self.factory.borrow().clone()
    }
    fn set_main_view_factory(&self, v: Option<Rc<dyn Fn() -> Ref<Control>>>) {
        self.factory.replace(v);
    }
}
impl ISingleViewApplicationLifetime for ActivityLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        panic!("activity must use its factory")
    }
    fn set_main_view(&self, _: Option<Ref<Control>>) {
        panic!("activity must use its factory")
    }
}
fn view(root: &Ref<Control>) -> Ref<MainView> {
    root.cast::<UserControl>()
        .unwrap()
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
        .and_then(|c| c.cast::<MainView>())
        .unwrap()
}
fn host(view: &Ref<MainView>) -> Ref<ScreenKeyboardHost> {
    NameScope::get_name_scope(view)
        .unwrap()
        .find("Host")
        .unwrap()
        .cast::<ScreenKeyboardHost>()
        .unwrap()
}
fn reset(view: &Ref<MainView>) -> Ref<Button> {
    use ferroui_base::metadata::from_markup_value;
    use ferroui_controls::{Panel, ScrollViewer};
    fn find(root: Ref<Control>) -> Option<Ref<Button>> {
        if let Some(button) = root.cast::<Button>()
            && from_markup_value::<String>(&button.content()).as_deref() == Some("Reset settings")
        {
            return Some(button);
        }
        if let Some(panel) = root.cast::<Panel>() {
            return panel.children().to_vec().into_iter().find_map(find);
        }
        if let Some(scroll) = root.cast::<ScrollViewer>() {
            return from_markup_value::<Ref<Control>>(&scroll.content()).and_then(find);
        }
        None
    }
    let panel = NameScope::get_name_scope(view)
        .unwrap()
        .find("SettingsPane")
        .unwrap()
        .cast::<ferroboard_sample::SettingsPanel>()
        .unwrap();
    find(from_markup_value::<Ref<Control>>(&panel.content()).unwrap()).unwrap()
}
#[test]
fn activities_get_independent_sessions_retire_on_drop_and_allow_reentrant_release() {
    let lifetime = Rc::new(ActivityLifetime::default());
    let storage = Rc::new(Storage::default());
    let builder = AppBuilder::configure::<App>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with(SampleStorage::new(storage.clone()));
    builder.setup_with_lifetime(lifetime.clone());
    let app = builder.instance().unwrap().cast::<App>().unwrap();
    let factory = lifetime.main_view_factory().unwrap();
    let first = factory();
    let first_view = view(&first);
    let first_host = host(&first_view);
    let first_reset = reset(&first_view);
    let second = factory();
    let second_view = view(&second);
    let second_host = host(&second_view);
    assert!(!std::rc::Rc::ptr_eq(
        &first_host.keyboard().engine(),
        &second_host.keyboard().engine()
    ));
    first_host.keyboard().settings().set_key_height(81.0);
    second_host.keyboard().settings().set_key_height(93.0);
    let before = storage.writes.get();
    app.save_session();
    assert!(storage.writes.get() >= before + 2);
    let before = storage.writes.get();
    drop(first);
    assert!(
        storage.writes.get() > before,
        "native root drop must save and retire its session despite retained child controls"
    );
    first_reset.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        first_host.keyboard().settings().key_height(),
        81.0,
        "retired commands must be detached"
    );
    let second_reset = reset(&second_view);
    second_reset.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_ne!(
        second_host.keyboard().settings().key_height(),
        93.0,
        "surviving activity remains live"
    );
    let app_weak = app.downgrade();
    storage.on_write.replace(Some(Box::new(move || {
        app_weak.upgrade().unwrap().release_session()
    })));
    app.save_session();
    let writes = storage.writes.get();
    app.save_session();
    assert_eq!(
        storage.writes.get(),
        writes,
        "release during save retires all live sessions without RefCell reentry"
    );
    drop(second);
    assert_eq!(
        storage.writes.get(),
        writes,
        "already released owner must not save twice"
    );
    lifetime.set_main_view_factory(None);
}
