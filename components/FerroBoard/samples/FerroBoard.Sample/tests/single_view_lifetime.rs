//! One setup per process: exercise the real AppBuilder lifetime initialization contract.
use ferroboard_controls::ScreenKeyboardHost;
use ferroboard_sample::{
    App, MainView,
    sample_storage::{SampleStorage, SampleStoragePlatform, StorageResult},
};
use ferroui_base::{Ref, controls::NameScope};
use ferroui_controls::{
    AppBuilder, Control,
    application_lifetimes::{IApplicationLifetime, ISingleViewApplicationLifetime},
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
    settings: RefCell<Option<String>>,
    writes: Cell<usize>,
    on_write: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl SampleStoragePlatform for Storage {
    fn get_setting(&self, _: &str) -> StorageResult<Option<String>> {
        Ok(self.settings.borrow().clone())
    }
    fn set_setting(&self, _: &str, value: &str) -> StorageResult<()> {
        *self.settings.borrow_mut() = Some(value.to_owned());
        self.writes.set(self.writes.get() + 1);
        let callback = self.on_write.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(None)
    }
    fn log(&self, message: &str) {
        panic!("unexpected storage error: {message}");
    }
}
#[derive(Default)]
struct SingleLifetime {
    view: RefCell<Option<Ref<Control>>>,
}
impl IApplicationLifetime for SingleLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }
}
impl ISingleViewApplicationLifetime for SingleLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.view.borrow().clone()
    }
    fn set_main_view(&self, value: Option<Ref<Control>>) {
        *self.view.borrow_mut() = value;
    }
}
#[test]
fn single_view_app_builder_initializes_shared_ui_and_allows_reentrant_storage_release() {
    let lifetime = Rc::new(SingleLifetime::default());
    let storage = Rc::new(Storage::default());
    let builder = AppBuilder::configure::<App>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with(SampleStorage::new(storage.clone()));
    builder.setup_with_lifetime(lifetime.clone());
    let app = builder.instance().unwrap().cast::<App>().unwrap();
    let view = lifetime.main_view().unwrap().cast::<MainView>().unwrap();
    let scope = NameScope::get_name_scope(&view).unwrap();
    let host = scope
        .find("Host")
        .unwrap()
        .cast::<ScreenKeyboardHost>()
        .unwrap();
    host.keyboard().settings().set_key_height(91.0);
    app.save_session();
    let saved = storage.settings.borrow().clone().unwrap();
    assert_eq!(
        ferroboard_core::settings::KeyboardSettings::from_json(&saved)
            .unwrap()
            .key_height(),
        91.0
    );
    let weak_app = app.downgrade();
    *storage.on_write.borrow_mut() = Some(Box::new(move || {
        weak_app.upgrade().unwrap().release_session();
    }));
    let before = storage.writes.get();
    app.save_session();
    assert!(
        storage.writes.get() >= before + 2,
        "save and reentrant final release both persist"
    );
    let retired = storage.writes.get();
    app.save_session();
    assert_eq!(
        storage.writes.get(),
        retired,
        "released application session stays retired"
    );
    lifetime.set_main_view(None);
}
