#[cfg(not(target_os = "ios"))]
fn main() -> std::process::ExitCode {
    eprintln!("FerroBoard iOS requires an iOS device or simulator target");
    std::process::ExitCode::FAILURE
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application_with(std::rc::Rc::new(host::AppDelegate::default()));
}

#[cfg(target_os = "ios")]
mod host {
    use ferroboard_ios::{lifecycle::BackgroundPersistence, storage::application_data_folder};
    use ferroboard_sample::{
        App,
        sample_storage::{FileSampleStoragePlatform, SampleStorage},
    };
    use ferroui_base::utilities::CultureInfo;
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_ios::{FerroApplicationDelegate, IFerroAppDelegate, IosApplicationExtensions};
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default)]
    pub struct AppDelegate {
        persistence: RefCell<Option<BackgroundPersistence>>,
    }
    impl FerroApplicationDelegate for AppDelegate {
        fn create_app_builder(&self, delegate: Rc<dyn IFerroAppDelegate>) -> AppBuilder {
            let locale = ferroboard_core::text::locale_text::system_locale_name()
                .expect("FerroBoard iOS could not read the system locale");
            CultureInfo::set_current_formatting(CultureInfo::get_culture_info(&locale));
            let folder = application_data_folder().expect(
                "FerroBoard iOS could not resolve its native application support directory",
            );
            let storage = SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(folder)));
            let persistence = BackgroundPersistence::new(
                delegate.as_ref(),
                Rc::new(|| {
                    if let Some(app) = Application::current().and_then(|app| app.cast::<App>()) {
                        app.save_session();
                    }
                }),
            );
            let previous = self.persistence.replace(Some(persistence));
            drop(previous);
            AppBuilder::configure::<App>()
                .use_ios_with_delegate(delegate)
                .with(storage)
                .with(Rc::new(
                    ferroboard_controls::services::KeyboardHapticService::native(),
                ))
        }
    }
}
