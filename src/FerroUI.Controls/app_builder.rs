use crate::application_lifetimes::IApplicationLifetime;
use crate::{Application, NewApplication};
use ferroui_base::media::FontManager;
use ferroui_base::platform::StandardRuntimePlatformServices;
use ferroui_base::{FerroLocator, ObjectType, Ref, StaticType, TypeInfo, Upcast};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    static SETUP_WAS_ALREADY_CALLED: Cell<bool> = const { Cell::new(false) };
}

type Initializer = Rc<dyn Fn()>;
type BuilderCallback = Rc<dyn Fn(&AppBuilder)>;

/// Initializes platform-specific services for an [`Application`].
///
/// The builder is a shared handle: the configuration methods return a clone
/// of it so that calls can be chained, and the callbacks registered with it
/// receive it back.
#[derive(Clone)]
pub struct AppBuilder {
    inner: Rc<Inner>,
}

struct Inner {
    options_initializers: RefCell<Vec<Initializer>>,
    app_factory: RefCell<Option<Rc<dyn Fn() -> Ref<Application>>>>,
    lifetime: RefCell<Option<Rc<dyn IApplicationLifetime>>>,
    runtime_platform_services_initializer: RefCell<Option<Initializer>>,
    runtime_platform_services_name: RefCell<Option<String>>,
    instance: RefCell<Option<Ref<Application>>>,
    application_type: Cell<Option<&'static TypeInfo>>,
    windowing_subsystem_initializer: RefCell<Option<Initializer>>,
    windowing_subsystem_name: RefCell<Option<String>>,
    rendering_subsystem_initializer: RefCell<Option<Initializer>>,
    rendering_subsystem_name: RefCell<Option<String>>,
    text_shaping_subsystem_initializer: RefCell<Option<Initializer>>,
    text_shaping_subsystem_name: RefCell<Option<String>>,
    after_setup_callback: RefCell<Vec<BuilderCallback>>,
    after_application_setup_callback: RefCell<Vec<BuilderCallback>>,
    after_platform_services_setup_callback: RefCell<Vec<BuilderCallback>>,
}

impl AppBuilder {
    /// Initializes a new instance of the [`AppBuilder`] class.
    fn new() -> Self {
        Self {
            inner: Rc::new(Inner {
                options_initializers: RefCell::new(Vec::new()),
                app_factory: RefCell::new(None),
                lifetime: RefCell::new(None),
                runtime_platform_services_initializer: RefCell::new(None),
                runtime_platform_services_name: RefCell::new(None),
                instance: RefCell::new(None),
                application_type: Cell::new(None),
                windowing_subsystem_initializer: RefCell::new(None),
                windowing_subsystem_name: RefCell::new(None),
                rendering_subsystem_initializer: RefCell::new(None),
                rendering_subsystem_name: RefCell::new(None),
                text_shaping_subsystem_initializer: RefCell::new(None),
                text_shaping_subsystem_name: RefCell::new(None),
                after_setup_callback: RefCell::new(Vec::new()),
                after_application_setup_callback: RefCell::new(Vec::new()),
                after_platform_services_setup_callback: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The method to call to initialize the runtime platform services (e. g.
    /// the asset loader).
    pub fn runtime_platform_services_initializer(&self) -> Option<Rc<dyn Fn()>> {
        self.inner.runtime_platform_services_initializer.borrow().clone()
    }

    /// The name of the currently selected runtime platform subsystem.
    pub fn runtime_platform_services_name(&self) -> Option<String> {
        self.inner.runtime_platform_services_name.borrow().clone()
    }

    /// The [`Application`] instance being initialized.
    pub fn instance(&self) -> Option<Ref<Application>> {
        self.inner.instance.borrow().clone()
    }

    /// The type of the instance (even if it's not created yet).
    pub fn application_type(&self) -> Option<&'static TypeInfo> {
        self.inner.application_type.get()
    }

    /// The method to call to initialize the windowing subsystem.
    pub fn windowing_subsystem_initializer(&self) -> Option<Rc<dyn Fn()>> {
        self.inner.windowing_subsystem_initializer.borrow().clone()
    }

    /// The name of the currently selected windowing subsystem.
    pub fn windowing_subsystem_name(&self) -> Option<String> {
        self.inner.windowing_subsystem_name.borrow().clone()
    }

    /// The method to call to initialize the rendering subsystem.
    pub fn rendering_subsystem_initializer(&self) -> Option<Rc<dyn Fn()>> {
        self.inner.rendering_subsystem_initializer.borrow().clone()
    }

    /// The name of the currently selected rendering subsystem.
    pub fn rendering_subsystem_name(&self) -> Option<String> {
        self.inner.rendering_subsystem_name.borrow().clone()
    }

    /// The method to call to initialize the text shaping subsystem.
    pub fn text_shaping_subsystem_initializer(&self) -> Option<Rc<dyn Fn()>> {
        self.inner.text_shaping_subsystem_initializer.borrow().clone()
    }

    /// The name of the currently selected text shaping subsystem.
    pub fn text_shaping_subsystem_name(&self) -> Option<String> {
        self.inner.text_shaping_subsystem_name.borrow().clone()
    }

    /// The method to call after the [`Application`] is set up: it calls the
    /// callbacks registered with [`after_setup`](Self::after_setup) in
    /// order.
    pub fn after_setup_callback(&self) -> Rc<dyn Fn(&AppBuilder)> {
        Self::combined(&self.inner.after_setup_callback)
    }

    /// The method to call after the platform services are set up: it calls
    /// the callbacks registered with
    /// [`after_platform_services_setup`](Self::after_platform_services_setup)
    /// in order.
    pub fn after_platform_services_setup_callback(&self) -> Rc<dyn Fn(&AppBuilder)> {
        Self::combined(&self.inner.after_platform_services_setup_callback)
    }

    fn combined(callbacks: &RefCell<Vec<BuilderCallback>>) -> Rc<dyn Fn(&AppBuilder)> {
        let callbacks = callbacks.borrow().clone();
        Rc::new(move |builder| {
            for callback in &callbacks {
                callback(builder);
            }
        })
    }

    fn invoke(&self, callbacks: &RefCell<Vec<BuilderCallback>>) {
        let callbacks = callbacks.borrow().clone();
        for callback in &callbacks {
            callback(self);
        }
    }

    /// Begin configuring an [`Application`].
    ///
    /// `TApp` is the subclass of [`Application`] to configure.
    pub fn configure<TApp: NewApplication>() -> AppBuilder {
        Self::configure_with(TApp::new_application)
    }

    /// Begin configuring an [`Application`] created by `app_factory`.
    ///
    /// `app_factory` is useful for passing of dependencies to `TApp`.
    pub fn configure_with<TApp: ObjectType + Upcast<Application>>(
        app_factory: impl Fn() -> Ref<TApp> + 'static,
    ) -> AppBuilder {
        let builder = Self::new();
        builder.inner.application_type.set(Some(<TApp as StaticType>::TYPE));
        *builder.inner.app_factory.borrow_mut() = Some(Rc::new(move || app_factory().upcast()));
        builder
    }

    /// Registers a callback to call after the [`Application`] is set up.
    pub fn after_setup(&self, callback: impl Fn(&AppBuilder) + 'static) -> AppBuilder {
        self.inner.after_setup_callback.borrow_mut().push(Rc::new(callback));
        self.clone()
    }

    /// Registers a callback to call after the [`Application`] is
    /// initialized and before the callbacks of
    /// [`after_setup`](Self::after_setup). Commonly used by backends to
    /// initialize their views.
    pub fn after_application_setup(&self, callback: impl Fn(&AppBuilder) + 'static) -> AppBuilder {
        self.inner.after_application_setup_callback.borrow_mut().push(Rc::new(callback));
        self.clone()
    }

    /// Registers a callback to call after the platform services are set up
    /// and before the [`Application`] is created.
    pub fn after_platform_services_setup(&self, callback: impl Fn(&AppBuilder) + 'static) -> AppBuilder {
        self.inner.after_platform_services_setup_callback.borrow_mut().push(Rc::new(callback));
        self.clone()
    }

    /// Sets up the application and calls `main` with it. What runs the
    /// application (a lifetime, a platform loop, nothing) is up to `main`.
    pub fn start(&self, main: impl FnOnce(&Ref<Application>, &[String]), args: &[String]) {
        self.setup();
        let instance = self.instance().expect("the application has been set up");
        main(&instance, args);
    }

    /// Sets up the platform-specific services for the application, but does
    /// not run it.
    pub fn setup_without_starting(&self) -> AppBuilder {
        self.setup();
        self.clone()
    }

    /// Sets up the platform-specific services for the application and
    /// initializes it with a particular lifetime, but does not run it.
    pub fn setup_with_lifetime(&self, lifetime: Rc<dyn IApplicationLifetime>) -> AppBuilder {
        *self.inner.lifetime.borrow_mut() = Some(lifetime);
        self.setup();
        self.clone()
    }

    /// Specifies a windowing subsystem to use.
    ///
    /// `initializer` is the method to call to initialize the windowing
    /// subsystem and `name` the name of the windowing subsystem.
    pub fn use_windowing_subsystem(&self, initializer: impl Fn() + 'static, name: &str) -> AppBuilder {
        *self.inner.windowing_subsystem_initializer.borrow_mut() = Some(Rc::new(initializer));
        *self.inner.windowing_subsystem_name.borrow_mut() = Some(name.to_string());
        self.clone()
    }

    /// Specifies a rendering subsystem to use.
    ///
    /// `initializer` is the method to call to initialize the rendering
    /// subsystem and `name` the name of the rendering subsystem.
    pub fn use_rendering_subsystem(&self, initializer: impl Fn() + 'static, name: &str) -> AppBuilder {
        *self.inner.rendering_subsystem_initializer.borrow_mut() = Some(Rc::new(initializer));
        *self.inner.rendering_subsystem_name.borrow_mut() = Some(name.to_string());
        self.clone()
    }

    /// Specifies a text shaping subsystem to use.
    ///
    /// `initializer` is the method to call to initialize the text shaping
    /// subsystem and `name` the name of the text shaping subsystem.
    pub fn use_text_shaping_subsystem(&self, initializer: impl Fn() + 'static, name: &str) -> AppBuilder {
        *self.inner.text_shaping_subsystem_initializer.borrow_mut() = Some(Rc::new(initializer));
        *self.inner.text_shaping_subsystem_name.borrow_mut() = Some(name.to_string());
        self.clone()
    }

    /// Specifies a runtime platform subsystem to use.
    ///
    /// `initializer` is the method to call to initialize the runtime
    /// platform subsystem and `name` the name of the runtime platform
    /// subsystem.
    pub fn use_runtime_platform_subsystem(&self, initializer: impl Fn() + 'static, name: &str) -> AppBuilder {
        *self.inner.runtime_platform_services_initializer.borrow_mut() = Some(Rc::new(initializer));
        *self.inner.runtime_platform_services_name.borrow_mut() = Some(name.to_string());
        self.clone()
    }

    /// Specifies a standard runtime platform subsystem to use.
    pub fn use_standard_runtime_platform_subsystem(&self) -> AppBuilder {
        *self.inner.runtime_platform_services_initializer.borrow_mut() = Some(Rc::new(|| {
            StandardRuntimePlatformServices::register(None);
        }));
        *self.inner.runtime_platform_services_name.borrow_mut() = Some("StandardRuntimePlatform".to_string());
        self.clone()
    }

    /// Configures platform-specific options: registers `options` as the
    /// service of type `T`, which may be a trait object type.
    pub fn with<T: ?Sized + 'static>(&self, options: Rc<T>) -> AppBuilder {
        self.inner.options_initializers.borrow_mut().push(Rc::new(move || {
            FerroLocator::current_mutable().bind::<T>().to_constant(options.clone());
        }));
        self.clone()
    }

    /// Configures platform-specific options: registers `options` as the
    /// function that produces the service of type `T` on every lookup.
    pub fn with_func<T: ?Sized + 'static>(&self, options: impl Fn() -> Rc<T> + 'static) -> AppBuilder {
        let options = Rc::new(options);
        self.inner.options_initializers.borrow_mut().push(Rc::new(move || {
            let options = options.clone();
            FerroLocator::current_mutable().bind::<T>().to_func(move || Some(options()));
        }));
        self.clone()
    }

    /// Registers an action that is executed with the current font manager.
    pub fn configure_fonts(&self, action: impl Fn(&Rc<FontManager>) + 'static) -> AppBuilder {
        self.after_setup(move |_| action(&FontManager::current()))
    }

    /// Sets up the platform-specific services for the [`Application`].
    fn setup(&self) {
        if self.inner.runtime_platform_services_initializer.borrow().is_none() {
            panic!("No runtime platform services configured.");
        }

        if self.inner.windowing_subsystem_initializer.borrow().is_none() {
            panic!("No windowing system configured.");
        }

        if self.inner.rendering_subsystem_initializer.borrow().is_none() {
            panic!("No rendering system configured.");
        }

        if self.inner.text_shaping_subsystem_initializer.borrow().is_none() {
            panic!("No text shaping system configured.");
        }

        if self.inner.app_factory.borrow().is_none() {
            panic!("No Application factory configured.");
        }

        if SETUP_WAS_ALREADY_CALLED.get() {
            panic!("Setup was already called on one of AppBuilder instances");
        }

        SETUP_WAS_ALREADY_CALLED.set(true);
        self.setup_unsafe();
    }

    /// Allows the setup to be called again after it has already been called
    /// once.
    #[doc(hidden)]
    pub fn reset_setup_for_unit_tests() {
        SETUP_WAS_ALREADY_CALLED.set(false);
    }

    /// Setup method that doesn't check for the initializers being set, nor
    /// for the setup having been called before. For platform backends and
    /// tests.
    #[doc(hidden)]
    pub fn setup_unsafe(&self) {
        let lifetime = self.inner.lifetime.borrow().clone();
        let setup_lifetime = lifetime.as_ref().filter(|lifetime| lifetime.as_setup_application_lifetime().is_some());
        if let Some(setup_lifetime) = setup_lifetime.and_then(|lifetime| lifetime.as_setup_application_lifetime()) {
            setup_lifetime.before_app_init();
        }

        let options_initializers = self.inner.options_initializers.borrow().clone();
        for initializer in &options_initializers {
            initializer();
        }
        Self::invoke_initializer(&self.inner.runtime_platform_services_initializer);
        Self::invoke_initializer(&self.inner.text_shaping_subsystem_initializer);
        Self::invoke_initializer(&self.inner.rendering_subsystem_initializer);
        Self::invoke_initializer(&self.inner.windowing_subsystem_initializer);
        self.invoke(&self.inner.after_platform_services_setup_callback);
        let app_factory = self.inner.app_factory.borrow().clone().expect("No Application factory configured.");
        let instance = app_factory();
        *self.inner.instance.borrow_mut() = Some(instance.clone());
        instance.set_application_lifetime(lifetime.clone());
        Application::bind_current(instance.clone());
        instance.register_services();
        instance.initialize();
        self.invoke(&self.inner.after_application_setup_callback);
        self.invoke(&self.inner.after_setup_callback);
        instance.on_framework_initialization_completed();

        if let Some(setup_lifetime) = setup_lifetime.and_then(|lifetime| lifetime.as_setup_application_lifetime()) {
            setup_lifetime.after_app_init();
        }
    }

    fn invoke_initializer(initializer: &RefCell<Option<Initializer>>) {
        let initializer = initializer.borrow().clone();
        if let Some(initializer) = initializer {
            initializer();
        }
    }
}
