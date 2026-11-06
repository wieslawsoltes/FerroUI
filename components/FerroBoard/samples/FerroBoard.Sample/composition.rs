//! Application services/lifetime composition. Keyboard input and focus rules remain in the library.
use crate::{
    MainView, MainWindow,
    bindings::SampleBindings,
    sample_options::{OptionError, SampleOptions},
    sample_storage::SampleStorage,
    view_model::MainViewModel,
};
use ferroboard_controls::{OnScreenKeyboard, ScreenKeyboardHost};
use ferroboard_core::layouts::KeyboardResources;
use ferroui_base::controls::NameScope;
use ferroui_base::{
    FerroObjectImpl, StyledElementImpl, VisualImpl, input::InputElementImpl,
    interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_base::{Ref, metadata::into_markup_value};
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, TextBox, UserControl,
    primitives::TemplatedControlImpl,
};
use ferroui_system::{IDisposable, Subscription, component_model::INotifyPropertyChanged};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
pub struct SampleComposition;
impl SampleComposition {
    pub fn resources() -> ferroboard_core::layouts::ResourceResult<Rc<KeyboardResources>> {
        ferroui_controls::Application::current()
            .and_then(|app| app.cast::<crate::App>())
            .and_then(|app| app.keyboard_resources())
            .ok_or(
                ferroboard_core::layouts::ResourceError::UnsupportedRepresentation(
                    "sample App resource service is not initialized",
                ),
            )
    }
}
ferroui_base::ferro_markup_type!(static SampleComposition{
 namespace:"FerroBoard.Sample",static_properties:[Resources:Rc<KeyboardResources>{try_get:SampleComposition::resources}]
});
pub struct SampleSession {
    pub window: Ref<MainWindow>,
    pub bindings: Rc<SampleBindings>,
    pub keyboard: Ref<OnScreenKeyboard>,
    pub host: Ref<ScreenKeyboardHost>,
    pub settings_panel: Ref<crate::SettingsPanel>,
    pub(crate) content: Rc<SessionContent>,
}
impl SampleSession {
    pub fn new(storage: Rc<SampleStorage>) -> Rc<Self> {
        Self::with_options(storage, SampleOptions::shared())
            .unwrap_or_else(|error| panic!("{error}"))
    }
    pub fn with_options(
        storage: Rc<SampleStorage>,
        options: Arc<SampleOptions>,
    ) -> Result<Rc<Self>, OptionError> {
        let window = MainWindow::new();
        let content = SessionContent::with_root(
            window.clone().upcast(),
            window.main_view(),
            storage,
            options,
        )?;
        Ok(Rc::new(Self {
            window,
            bindings: content.bindings.clone(),
            keyboard: content.keyboard.clone(),
            host: content.host.clone(),
            settings_panel: content.settings_panel.clone(),
            content,
        }))
    }
    pub fn release(&self) {
        self.content.release();
    }
}
/// Retain the session while its view is hosted. Removing the view saves data;
/// final session release removes subscriptions and disposes the owned engine.
pub struct MainViewSession {
    pub view: Ref<MainView>,
    pub bindings: Rc<SampleBindings>,
    pub keyboard: Ref<OnScreenKeyboard>,
    pub host: Ref<ScreenKeyboardHost>,
    pub settings_panel: Ref<crate::SettingsPanel>,
    pub(crate) content: Rc<SessionContent>,
}
impl MainViewSession {
    pub fn new(storage: Rc<SampleStorage>) -> Rc<Self> {
        Self::with_options(storage, SampleOptions::shared())
            .unwrap_or_else(|error| panic!("{error}"))
    }
    pub fn with_options(
        storage: Rc<SampleStorage>,
        options: Arc<SampleOptions>,
    ) -> Result<Rc<Self>, OptionError> {
        let view = MainView::new();
        let content =
            SessionContent::with_root(view.clone().upcast(), view.clone(), storage, options)?;
        Ok(Rc::new(Self {
            view,
            bindings: content.bindings.clone(),
            keyboard: content.keyboard.clone(),
            host: content.host.clone(),
            settings_panel: content.settings_panel.clone(),
            content,
        }))
    }
    pub fn release(&self) {
        self.content.release();
    }
}
pub(crate) struct SessionContent {
    root: Ref<Control>,
    pub bindings: Rc<SampleBindings>,
    pub keyboard: Ref<OnScreenKeyboard>,
    pub host: Ref<ScreenKeyboardHost>,
    pub settings_panel: Ref<crate::SettingsPanel>,
    storage: Rc<SampleStorage>,
    tokens: RefCell<Vec<Subscription>>,
    released: Cell<bool>,
}
impl SessionContent {
    fn with_root(
        root: Ref<Control>,
        view: Ref<MainView>,
        storage: Rc<SampleStorage>,
        options: Arc<SampleOptions>,
    ) -> Result<Rc<Self>, OptionError> {
        let scope = NameScope::get_name_scope(&view).expect("compiled main view name scope");
        let host = scope
            .find("Host")
            .unwrap()
            .cast::<ScreenKeyboardHost>()
            .unwrap();
        let keyboard = host.keyboard();
        let message = scope.find("MessageBox").unwrap().cast::<TextBox>().unwrap();
        let settings_panel = scope
            .find("SettingsPane")
            .unwrap()
            .cast::<crate::SettingsPanel>()
            .unwrap();
        storage.load(&keyboard.engine());
        if let Err(error) = options.apply(&keyboard.settings()) {
            // A failed constructor cannot hand these native/engine subscriptions to a caller.
            // Release them explicitly in Rust; no settings save or successful session is invented.
            settings_panel.release_native_ownership();
            host.release_native_subscriptions().unwrap();
            keyboard.engine().dispose().unwrap();
            ferroboard_core_ferro::release_engine_markup_subscriptions(&keyboard.engine());
            return Err(error);
        }
        // Source order: storage -> options -> SettingsPane.Attach -> application subscriptions.
        let build_bindings = || -> Result<Rc<SampleBindings>, String> {
            settings_panel
                .attach(keyboard.clone())
                .map_err(|e| e.to_string())?;
            let presentation = settings_panel.presentation().map_err(|e| e.to_string())?;
            let model = MainViewModel::with_context(presentation.context, presentation.themes);
            model.set_status(None); // Source StatusText starts empty; Reset does not change it.
            SampleBindings::new(model, presentation.languages).map_err(|e| e.to_string())
        };
        let bindings = build_bindings().unwrap_or_else(|error| {
            settings_panel.release_native_ownership();
            host.release_native_subscriptions().unwrap();
            keyboard.engine().dispose().unwrap();
            ferroboard_core_ferro::release_engine_markup_subscriptions(&keyboard.engine());
            panic!("sample settings construction failed: {error}");
        });
        root.set_data_context(into_markup_value(bindings.clone()));
        let this = Rc::new(Self {
            root,
            bindings,
            keyboard,
            host,
            settings_panel,
            storage,
            tokens: RefCell::new(Vec::new()),
            released: Cell::new(false),
        });
        let weak_model = Rc::downgrade(&this.bindings.model());
        let settings_requested = this.keyboard.settings_requested().add(Rc::new(move |_| {
            if let Some(model) = weak_model.upgrade() {
                model.set_settings_pane_open(true);
            }
        }));
        let weak_keyboard = this.keyboard.downgrade();
        this.tokens.borrow_mut().push(
            ferroui_base::reactive::Disposable::create(move || {
                if let Some(keyboard) = weak_keyboard.upgrade() {
                    keyboard.settings_requested().remove(settings_requested);
                }
            })
            .into(),
        );
        let model = Rc::downgrade(&this.bindings.model());
        let enter = this
            .keyboard
            .enter_action_requested()
            .add(Rc::new(move |args| {
                if let Some(model) = model.upgrade() {
                    let name = args
                        .element()
                        .map(|element| element.name().unwrap_or_default());
                    model.set_status(Some(MainViewModel::enter_action_status(
                        args.action(),
                        name.as_deref(),
                    )));
                }
                if MainViewModel::handles_enter_action(args.action()) {
                    args.set_handled(true);
                }
            }));
        let keyboard = this.keyboard.downgrade();
        this.tokens.borrow_mut().push(
            ferroui_base::reactive::Disposable::create(move || {
                if let Some(k) = keyboard.upgrade() {
                    k.enter_action_requested().remove(enter);
                }
            })
            .into(),
        );
        let model = Rc::downgrade(&this.bindings.model());
        let target_changed = this.keyboard.target_changed().add(Rc::new(move |owner| {
            if let Some(model) = model.upgrade() {
                let description = owner
                    .target_element()
                    .and_then(|v| v.cast::<Control>())
                    .map(|control| {
                        let header = if control.cast::<TextBox>().is_some() {
                            ferroui_controls::automation::AutomationProperties::get_name(&control)
                        } else {
                            None
                        };
                        header.unwrap_or_else(|| control.get_type().name().to_string())
                    });
                model.set_status(Some(MainViewModel::target_status(description.as_deref())));
            }
        }));
        let keyboard = this.keyboard.downgrade();
        this.tokens.borrow_mut().push(
            ferroui_base::reactive::Disposable::create(move || {
                if let Some(keyboard) = keyboard.upgrade() {
                    keyboard.target_changed().remove(target_changed);
                }
            })
            .into(),
        );
        let weak_model = Rc::downgrade(&this.bindings.model());
        let size_changed = this.root.size_changed(move |_, args| {
            if let Some(model) = weak_model.upgrade() {
                model.update_width(args.new_size().width, args.previous_size().width);
            }
        });
        let weak_window = this.root.downgrade();
        this.tokens.borrow_mut().push(
            ferroui_base::reactive::Disposable::create(move || {
                if let Some(window) = weak_window.upgrade() {
                    window.remove_handler(
                        ferroui_controls::Control::size_changed_event(),
                        size_changed,
                    );
                }
            })
            .into(),
        );
        let weak_model = Rc::downgrade(&this.bindings.model());
        let shown = move || {
            if options.get(&"settings".into()) == Some("false".into())
                && let Some(model) = weak_model.upgrade()
            {
                model.set_settings_pane_open(false);
            }
            // Read at Loaded/Opened time, as source does, rather than snapshotting at Apply.
            let focus = options
                .get(&"focus".into())
                .and_then(|name| name.to_utf8().ok())
                .and_then(|name| scope.find(&name))
                .and_then(|value| value.cast::<Control>());
            focus.unwrap_or_else(|| message.clone().upcast()).focus();
        };
        if let Some(window) = this.root.cast::<MainWindow>() {
            this.tokens.borrow_mut().push(window.opened(shown));
            let weak = Rc::downgrade(&this);
            this.tokens.borrow_mut().push(window.closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.release();
                }
            }));
        } else {
            let loaded = this.root.loaded(move |_, _| shown());
            let root = this.root.downgrade();
            this.tokens.borrow_mut().push(
                ferroui_base::reactive::Disposable::create(move || {
                    if let Some(root) = root.upgrade() {
                        root.remove_handler(Control::loaded_event(), loaded);
                    }
                })
                .into(),
            );
            let weak = Rc::downgrade(&this);
            let unloaded = this.root.unloaded(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.save();
                }
            });
            let root = this.root.downgrade();
            this.tokens.borrow_mut().push(
                ferroui_base::reactive::Disposable::create(move || {
                    if let Some(root) = root.upgrade() {
                        root.remove_handler(Control::unloaded_event(), unloaded);
                    }
                })
                .into(),
            );
        }
        let settings = this.keyboard.settings();
        let storage = this.storage.clone();
        let weak_settings = Rc::downgrade(&settings);
        let weak_bindings = Rc::downgrade(&this.bindings);
        let changed = settings.property_changed().add(Rc::new(move |name| {
            if let Some(settings) = weak_settings.upgrade() {
                storage.save_settings(&settings);
            }
            if name == "ThemeMode"
                && let Some(bindings) = weak_bindings.upgrade()
            {
                bindings.notify_theme_changed();
            }
        }));
        this.tokens.borrow_mut().push(
            ferroui_base::reactive::Disposable::create(move || {
                settings.property_changed().remove(changed);
            })
            .into(),
        );
        let history = this.keyboard.engine().emoji_history();
        let storage = this.storage.clone();
        let weak_engine = Rc::downgrade(&this.keyboard.engine());
        let changed = history.changed().add(Rc::new(move |_| {
            if let Some(engine) = weak_engine.upgrade() {
                storage
                    .save(&engine)
                    .unwrap_or_else(|error| std::panic::panic_any(error));
            }
        }));
        this.tokens.borrow_mut().push(
            ferroui_base::reactive::Disposable::create(move || {
                history.changed().remove(changed);
            })
            .into(),
        );
        Ok(this)
    }
    pub fn save(&self) {
        if !self.released.get() {
            self.storage
                .save(&self.keyboard.engine())
                .unwrap_or_else(|error| std::panic::panic_any(error));
        }
    }
    pub fn release(&self) {
        if self.released.replace(true) {
            return;
        }
        // Source MainPage.Unloaded saves while the engine is still attached/alive.
        self.storage
            .save(&self.keyboard.engine())
            .unwrap_or_else(|error| std::panic::panic_any(error));
        for token in self.tokens.take() {
            token.dispose();
        }
        self.root.set_data_context(None);
        self.settings_panel.release_native_ownership();
        self.bindings.languages().release();
        self.host.release_native_subscriptions().unwrap();
        self.keyboard.engine().dispose().unwrap();
        ferroboard_core_ferro::release_engine_markup_subscriptions(&self.keyboard.engine());
        self.bindings.model().reset_settings().dispose();
        self.bindings.model().clear_learned_words().dispose();
    }
}

impl Drop for SessionContent {
    fn drop(&mut self) {
        self.release();
    }
}

/// Each native activity owns its session through this passive container. The application
/// keeps only weak session references, so removing a destroyed activity retires its engine
/// without a process-global view or a strong view/session ownership cycle.
#[repr(C)]
pub(crate) struct ActivityView {
    base: ferroui_controls::UserControl,
    _session: Rc<MainViewSession>,
}
ferroui_base::ferro_class!(ActivityView:UserControl);
ferroui_base::ferro_class_info!(ActivityView {});
ferroui_base::ferro_impl_classes!(ActivityView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl ActivityView {
    pub(crate) fn new(session: Rc<MainViewSession>) -> Ref<Self> {
        let this = ferroui_base::instantiate(Self {
            base: ferroui_controls::UserControl::construct(),
            _session: session.clone(),
        });
        this.set_content(Some(Control::boxed(session.view.clone())));
        this
    }
}
