use ferroui_base::{FerroObjectImpl, LocatorExtensions, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use std::{cell::RefCell, rc::Rc};
#[repr(C)]
pub struct App {
    base: Application,
    session: RefCell<Option<Rc<crate::composition::SessionContent>>>,
    activity_sessions: RefCell<Vec<std::rc::Weak<crate::composition::MainViewSession>>>,
    keyboard_resources: RefCell<Option<Rc<ferroboard_core::layouts::KeyboardResources>>>,
}
ferroui_base::ferro_class!(App:Application);
ferroui_base::ferro_class_info!(App { new: App::new });
ferroui_base::ferro_impl_classes!(App:FerroObjectImpl);
impl App {
    pub fn new() -> Ref<Self> {
        let app = ferroui_base::instantiate(Self::construct());
        app.set_name(Some("FerroBoard".into()));
        app
    }
    /// Native base construction for application subclasses sharing the sample resources.
    pub fn construct() -> Self {
        Self {
            base: Application::construct(),
            session: RefCell::new(None),
            activity_sessions: RefCell::new(Vec::new()),
            keyboard_resources: RefCell::new(None),
        }
    }
    pub fn save_session(&self) {
        let session = self.session.borrow().clone();
        if let Some(session) = session {
            session.save();
        }
        // Snapshot outside callbacks: storage can synchronously release the application.
        let sessions: Vec<_> = self
            .activity_sessions
            .borrow()
            .iter()
            .filter_map(|s| s.upgrade())
            .collect();
        for session in sessions {
            session.content.save();
        }
    }
    pub fn release_session(&self) {
        let session = self.session.take();
        if let Some(session) = session {
            session.release();
        }
        let sessions = self.activity_sessions.take();
        for session in sessions.into_iter().filter_map(|s| s.upgrade()) {
            session.release();
        }
    }
    pub fn keyboard_resources(&self) -> Option<Rc<ferroboard_core::layouts::KeyboardResources>> {
        self.keyboard_resources.borrow().clone()
    }
}
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        crate::register_types();
        crate::compiled_app::populate(None, &this.to_ref()).unwrap_or_else(|e| panic!("{e}"));
        let resources = Rc::new(
            ferroboard_core::layouts::KeyboardResources::create_default()
                .expect("bundled resources"),
        );
        crate::custom_layouts::register(&resources).expect("source sample layout registration");
        *this.keyboard_resources.borrow_mut() = Some(resources);
        let value = this.resources().get(&"SunsetTheme".into());
        let theme = ferroui_base::metadata::from_markup_value::<
            std::sync::Arc<ferroboard_controls::themes::KeyboardTheme>,
        >(&value)
        .expect("original theme resource object");
        ferroboard_controls::themes::KeyboardThemes::register(theme)
            .expect("source sample theme registration");
    }
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let options = crate::sample_options::SampleOptions::shared();
            // Desktop lifetime arguments map the launch argument source. The original sample
            // parses environment command-line arguments second, overriding duplicate values.
            if let Some(arguments) = desktop.args() {
                options.parse(Some(&arguments.join(" ").as_str().into()));
            }
            options.parse(Some(
                &std::env::args()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_str()
                    .into(),
            ));
            let storage = ferroui_base::FerroLocator::current()
                .get_service::<crate::sample_storage::SampleStorage>()
                .unwrap_or_else(|| {
                    crate::sample_storage::SampleStorage::new(Rc::new(
                        crate::sample_storage::FileSampleStoragePlatform::from_environment(),
                    ))
                });
            let session = crate::composition::SampleSession::new(storage);
            desktop.set_main_window(Some(session.window.clone().upcast()));
            this.session.replace(Some(session.content.clone()));
            if let Ok(value) = std::env::var("FERROBOARD_SMOKE_EXIT_MS") {
                let milliseconds = value
                    .parse::<u64>()
                    .expect("FERROBOARD_SMOKE_EXIT_MS must be an integer");
                let window = session.window.clone();
                ferroui_base::threading::DispatcherTimer::run_once(
                    move || window.close(),
                    std::time::Duration::from_millis(milliseconds),
                    ferroui_base::threading::DispatcherPriority::NORMAL,
                )
                .detach();
            }
        } else if let Some(activity) = lifetime
            .as_ref()
            .and_then(|l| l.as_activity_application_lifetime())
        {
            let storage = ferroui_base::FerroLocator::current()
                .get_service::<crate::sample_storage::SampleStorage>()
                .expect("activity sample host must supply SampleStorage through AppBuilder::with");
            let app = this.to_ref().downgrade();
            activity.set_main_view_factory(Some(Rc::new(move || {
                let app = app
                    .upgrade()
                    .expect("sample application retired before activity factory");
                let session = crate::composition::MainViewSession::new(storage.clone());
                {
                    let mut sessions = app.activity_sessions.borrow_mut();
                    sessions.retain(|s| s.strong_count() > 0);
                    sessions.push(Rc::downgrade(&session));
                }
                crate::composition::ActivityView::new(session).upcast()
            })));
        } else if let Some(single) = lifetime
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            // Mobile/browser hosts supply their native sandbox storage at the composition root.
            let storage = ferroui_base::FerroLocator::current()
                .get_service::<crate::sample_storage::SampleStorage>()
                .expect(
                    "single-view sample host must supply SampleStorage through AppBuilder::with",
                );
            let options = crate::sample_options::SampleOptions::shared();
            options.parse(Some(
                &std::env::args()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .as_str()
                    .into(),
            ));
            let session = crate::composition::MainViewSession::with_options(storage, options)
                .unwrap_or_else(|error| panic!("{error}"));
            this.session.replace(Some(session.content.clone()));
            single.set_main_view(Some(session.view.clone().upcast()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
