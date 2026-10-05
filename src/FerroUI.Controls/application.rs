use crate::application_lifetimes::{IActivatableLifetime, IApplicationLifetime};
use crate::templates::{DataTemplates, IDataTemplateHost};
use crate::IGlobalDataTemplates;
use ferroui_base::animation::IGlobalClock;
use ferroui_base::controls::{
    IResourceHost, IResourceNode, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceValue,
    ResourcesChangedEventArgs,
};
use ferroui_base::input::raw::IDragDropDevice;
use ferroui_base::input::{
    AccessKeyHandler, DragDropDevice, IAccessKeyHandler, IInputManager, IKeyboardNavigationHandler, InputManager,
    KeyboardNavigationHandler,
};
use ferroui_base::media::MediaContext;
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformSettings};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{
    GlobalStylesHandler, IGlobalStyles, IStyle, IStyleHost, IThemeVariantHost, IThemeVariantRoot, StyleHostRef,
    Styles, ThemeVariant,
};
use ferroui_base::threading::FerroSynchronizationContext;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, BoxedValue, DirectProperty, FerroLocator, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions,
    ObjectType, Ref, StyledElement, StyledProperty, Upcast, WeakRef,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

/// Encapsulates a FerroUI application.
///
/// The [`Application`] class encapsulates application-specific
/// functionality, including:
/// - A global set of [`data_templates`](Application::data_templates).
/// - A global set of [`styles`](Application::styles).
/// - An input manager.
/// - Registers services needed by the rest of the framework in the
///   [`register_services`](Application::register_services) method.
/// - Tracks the lifetime of the application.
///
/// An application class derives from it and overrides the members of
/// [`ApplicationImpl`]:
///
/// ```ignore
/// #[repr(C)]
/// pub struct App { base: Application }
///
/// ferro_class!(App: Application);
/// ferro_impl_classes!(App: FerroObjectImpl);
///
/// impl ApplicationImpl for App {
///     fn initialize(this: &Self) {
///         this.styles().add(theme());
///     }
/// }
///
/// impl NewApplication for App {
///     fn new_application() -> Ref<Self> {
///         instantiate(Self { base: Application::construct() })
///     }
/// }
/// ```
#[repr(C)]
pub struct Application {
    base: FerroObject,
    host: Rc<ApplicationHost>,
    application_lifetime: RefCell<Option<Rc<dyn IApplicationLifetime>>>,
    setup_completed: Cell<bool>,
    input_manager: RefCell<Option<Rc<InputManager>>>,
    name: RefCell<Option<String>>,
}

ferro_class! {
    Application: FerroObject, virtuals ApplicationImpl: FerroObjectImpl {
        /// Initializes the application by loading its styles and resources.
        fn initialize(this);
        /// Registers the services needed by the framework.
        fn register_services(this);
        /// Called when the framework has completed its initialization: the
        /// place to create the main window or view of the lifetime.
        fn on_framework_initialization_completed(this);
    }
}
ferroui_base::ferro_class_info!(Application {
    new: Application::new,
    interfaces: [
        Rc<dyn IResourceHost> => |application: Ref<Application>| application.as_resource_host(),
        Rc<dyn IStyleHost> => |application: Ref<Application>| application.as_style_host(),
        Rc<dyn IGlobalStyles> => |application: Ref<Application>| application.as_global_styles(),
        Rc<dyn IThemeVariantHost> => |application: Ref<Application>| application.as_theme_variant_host(),
        ResourceHostRef => |application: Ref<Application>| ResourceHostRef::Other(application.as_resource_host()),
        StyleHostRef => |application: Ref<Application>| StyleHostRef::Other(application.as_style_host()),
    ],
});

/// Creates an application class with its default constructor, for
/// [`AppBuilder::configure`](crate::AppBuilder::configure).
pub trait NewApplication: ObjectType + Upcast<Application> {
    /// Creates the application.
    fn new_application() -> Ref<Self>;
}

impl NewApplication for Application {
    fn new_application() -> Ref<Self> {
        Application::new()
    }
}

impl FerroObjectImpl for Application {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let _ = this.host.owner.set(this.to_ref().downgrade());
        this.set_name(Some("FerroUI Application".to_string()));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::actual_theme_variant_property().as_property() {
            for (_, handler) in this.host.actual_theme_variant_changed.snapshot().iter() {
                handler();
            }
        }
    }
}

impl ApplicationImpl for Application {
    fn initialize(_this: &Self) {}

    fn register_services(this: &Self) {
        FerroSynchronizationContext::install_if_needed();
        let input_manager = Rc::new(InputManager::new());
        *this.input_manager.borrow_mut() = Some(input_manager.clone());

        this.initialize_theme_variant();

        let host = this.host.clone();
        let locator = FerroLocator::current_mutable();
        locator
            .bind::<dyn IAccessKeyHandler>()
            .to_func(|| Some(AccessKeyHandler::new() as Rc<dyn IAccessKeyHandler>))
            .bind::<dyn IGlobalDataTemplates>()
            .to_constant(host.clone())
            .bind::<dyn IGlobalStyles>()
            .to_constant(host.clone())
            .bind::<dyn IThemeVariantHost>()
            .to_constant(host)
            .bind::<dyn IInputManager>()
            .to_constant(input_manager.clone())
            .bind::<dyn crate::IToolTipService>()
            .to_constant(crate::ToolTipService::new(&(input_manager as Rc<dyn IInputManager>)))
            .bind::<dyn IKeyboardNavigationHandler>()
            .to_func(|| Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>))
            .bind::<dyn IDragDropDevice>()
            .to_constant(DragDropDevice::instance() as Rc<dyn IDragDropDevice>);

        // As in the reference, the in-process drag source is kept as the
        // default for platforms that do not register one.
        {
            use ferroui_base::input::platform::IPlatformDragSource;
            use ferroui_base::LocatorExtensions;

            if FerroLocator::current().get_service::<dyn IPlatformDragSource>().is_none() {
                locator.bind::<dyn IPlatformDragSource>().to_func(|| {
                    Some(crate::platform::InProcessDragSource::new() as Rc<dyn IPlatformDragSource>)
                });
            }
        }

        locator.bind::<dyn IGlobalClock>().to_constant(MediaContext::instance().clock());

        this.setup_completed.set(true);
    }

    fn on_framework_initialization_completed(_this: &Self) {}
}

ferroui_base::ferro_properties! { impl Application {
    ferro_property!(
        /// Defines the `DataContext` property.
        pub fn data_context_property() -> StyledProperty<Option<BoxedValue>> {
            StyledElement::data_context_property().add_owner::<Application>()
        }
    );

    ferro_property!(
        /// Defines the `ActualThemeVariant` property: the UI theme variant
        /// that is currently used by the application.
        pub fn actual_theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            ThemeVariant::actual_theme_variant_property().add_owner::<Application>()
        }
    );

    ferro_property!(
        /// Defines the `RequestedThemeVariant` property: the UI theme
        /// variant that is requested for the application.
        pub fn requested_theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            ThemeVariant::requested_theme_variant_property().add_owner::<Application>()
        }
    );

    ferro_property!(
        /// Defines the `Name` property.
        pub fn name_property() -> DirectProperty<Application, Option<String>> {
            FerroProperty::register_direct::<Application, _>("Name", |o| o.name(), Some(|o, v| o.set_name(v)), None)
        }
    );
} }

impl Application {
    fn static_constructor() {
        // The application is a root of theme variant resolution.
        ThemeVariant::register_theme_variant_root_type(<Application as ferroui_base::StaticType>::TYPE);
        // The application has a data context that can be used for binding.
        <dyn ferroui_base::IDataContextProvider>::register_type(<Application as ferroui_base::StaticType>::TYPE);
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            host: ApplicationHost::new(),
            application_lifetime: RefCell::new(None),
            setup_completed: Cell::new(false),
            input_manager: RefCell::new(None),
            name: RefCell::new(None),
        }
    }

    /// Creates an instance of the [`Application`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The application's data context.
    ///
    /// The data context property specifies the default object that will be
    /// used for data binding.
    pub fn data_context(&self) -> Option<BoxedValue> {
        self.get_value(Self::data_context_property())
    }

    /// Sets the application's data context.
    pub fn set_data_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::data_context_property(), value)
    }

    /// The UI theme variant that is requested for the application.
    pub fn requested_theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::requested_theme_variant_property())
    }

    /// Sets the UI theme variant that is requested for the application.
    pub fn set_requested_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::requested_theme_variant_property(), value)
    }

    /// The UI theme variant that is currently used by the application.
    ///
    /// `None` until the services of the application have been registered.
    pub fn actual_theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::actual_theme_variant_property())
    }

    /// The application is the root of the theme variant resolution.
    pub fn is_theme_variant_root(&self) -> bool {
        true
    }

    /// The current instance of the [`Application`] class: the application
    /// registered with the service locator (see
    /// [`bind_current`](Self::bind_current)).
    pub fn current() -> Option<Ref<Application>> {
        FerroLocator::current().get_service::<Ref<Application>>().map(|application| (*application).clone())
    }

    /// Registers `application` with the current service locator as the
    /// application returned by [`current`](Self::current).
    pub fn bind_current(application: Ref<Application>) {
        FerroLocator::current_mutable().bind_to_self(Rc::new(application));
    }

    /// The application's global data templates.
    pub fn data_templates(&self) -> DataTemplates {
        self.host.data_templates()
    }

    /// Whether the data templates collection has been created.
    pub fn is_data_templates_initialized(&self) -> bool {
        self.host.data_templates.get().is_some()
    }

    /// The application's input manager, once the services have been
    /// registered.
    pub fn input_manager(&self) -> Option<Rc<InputManager>> {
        self.input_manager.borrow().clone()
    }

    /// The application's global resource dictionary.
    pub fn resources(&self) -> Ref<ResourceDictionary> {
        self.host.resources()
    }

    /// Sets the application's global resource dictionary.
    pub fn set_resources(&self, value: Ref<ResourceDictionary>) {
        self.host.set_resources(value)
    }

    /// The application's global styles.
    ///
    /// Global styles apply to all windows in the application.
    pub fn styles(&self) -> Ref<Styles> {
        self.host.styles()
    }

    /// Whether the styles collection has been created.
    pub fn is_styles_initialized(&self) -> bool {
        self.host.styles.get().is_some()
    }

    /// Whether the application or its styles have resources.
    pub fn has_resources(&self) -> bool {
        self.host.has_resources()
    }

    /// The styling parent of the application, which is `None`.
    pub fn styling_parent(&self) -> Option<StyleHostRef> {
        None
    }

    /// The application lifetime; use it for things like setting the main
    /// window or view and exiting the application from code.
    pub fn application_lifetime(&self) -> Option<Rc<dyn IApplicationLifetime>> {
        self.application_lifetime.borrow().clone()
    }

    /// Sets the application lifetime.
    ///
    /// # Panics
    /// Panics when the application has already been initialized.
    pub fn set_application_lifetime(&self, value: Option<Rc<dyn IApplicationLifetime>>) {
        if self.setup_completed.get() {
            panic!("It's not possible to change ApplicationLifetime after Application was initialized.");
        }

        *self.application_lifetime.borrow_mut() = value;
    }

    /// The contract for accessing global platform-specific settings.
    ///
    /// It can be `None` only if the application wasn't initialized yet. The
    /// platform settings of a top-level are an equivalent which should
    /// always be preferred over the global one, as specific top-levels
    /// might have different settings set up.
    pub fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        self.try_get::<dyn IPlatformSettings>()
    }

    /// Raised when the resources of the application change. Disposing the
    /// returned handle unsubscribes.
    pub fn resources_changed(&self, handler: impl Fn(&ResourcesChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let handler: Rc<dyn Fn(&ResourcesChangedEventArgs)> = Rc::new(handler);
        self.host.subscribe(|host| &host.resources_changed, handler)
    }

    /// Raised when the actual theme variant of the application has changed.
    /// Disposing the returned handle unsubscribes.
    pub fn actual_theme_variant_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let handler: Rc<dyn Fn()> = Rc::new(handler);
        self.host.subscribe(|host| &host.actual_theme_variant_changed, handler)
    }

    /// Tries to find a resource within the application: in its resource
    /// dictionary, then in its styles.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.host.try_get_resource(key, theme)
    }

    /// The application name to be used for various platform-specific
    /// purposes.
    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    /// Sets the application name.
    pub fn set_name(&self, value: Option<String>) {
        self.set_and_raise(Self::name_property(), &self.name, value);
    }

    /// Queries for an optional feature.
    ///
    /// `feature_type` is the `TypeId` of the feature's trait object type;
    /// the returned value holds the `Rc<dyn ..>` handle of the feature.
    /// Features currently supported:
    ///
    /// - `dyn IPlatformSettings`
    /// - `dyn IActivatableLifetime`
    pub fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.host.try_get_feature(feature_type)
    }

    /// Queries for an optional feature registered as `Rc<T>`; see
    /// [`try_get_feature`](Self::try_get_feature).
    pub fn try_get<T: ?Sized + 'static>(&self) -> Option<Rc<T>> {
        self.try_get_feature(TypeId::of::<T>()).and_then(|feature| feature.downcast_ref::<Rc<T>>().cloned())
    }

    /// Establishes the actual theme variant of the application and keeps
    /// it up to date with the color values of the platform settings.
    ///
    /// The subscription lasts as long as the platform settings do; it does
    /// not keep the application alive.
    pub fn initialize_theme_variant(&self) {
        if let Some(platform_settings) = self.platform_settings() {
            let weak = self.to_ref().downgrade();
            platform_settings.color_values_changed(Rc::new(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.update_actual_theme_variant();
                }
            }));
        }
        self.update_actual_theme_variant();
    }

    fn update_actual_theme_variant(&self) {
        ThemeVariant::update_actual_theme_variant(self);
    }

    // --- interface handles ------------------------------------------------

    /// The application as the host of the global styles.
    pub fn as_global_styles(&self) -> Rc<dyn IGlobalStyles> {
        self.host.clone()
    }

    /// The application as a style host.
    pub fn as_style_host(&self) -> Rc<dyn IStyleHost> {
        self.host.clone()
    }

    /// The application as a resource host.
    pub fn as_resource_host(&self) -> Rc<dyn IResourceHost> {
        self.host.clone()
    }

    /// The application as a theme variant host.
    pub fn as_theme_variant_host(&self) -> Rc<dyn IThemeVariantHost> {
        self.host.clone()
    }

    /// The application as the host of the global data templates.
    pub fn as_global_data_templates(&self) -> Rc<dyn IGlobalDataTemplates> {
        self.host.clone()
    }

    /// The application as a provider of optional features.
    pub fn as_optional_feature_provider(&self) -> Rc<dyn IOptionalFeatureProvider> {
        self.host.clone()
    }

    /// The application as a theme variant root.
    pub fn as_theme_variant_root(&self) -> Rc<dyn IThemeVariantRoot> {
        self.host.clone()
    }

    /// The application as an element with a data context that can be used
    /// for binding.
    pub fn as_data_context_provider(&self) -> Rc<dyn ferroui_base::IDataContextProvider> {
        self.host.clone()
    }
}

/// The interfaces of an application object, and the state they work on.
///
/// The application hands this out wherever one of its contracts is needed
/// (`Rc<dyn IGlobalStyles>`, `Rc<dyn IResourceHost>`, ...); every handle of
/// one application is the same object, so that hosts compare by identity.
struct ApplicationHost {
    this: Weak<ApplicationHost>,
    owner: OnceCell<WeakRef<Application>>,
    /// The application-global data templates.
    data_templates: OnceCell<DataTemplates>,
    styles: OnceCell<Ref<Styles>>,
    resources: RefCell<Option<Ref<ResourceDictionary>>>,
    styles_added: HandlerList<GlobalStylesHandler>,
    styles_removed: HandlerList<GlobalStylesHandler>,
    resources_changed: HandlerList<dyn Fn(&ResourcesChangedEventArgs)>,
    actual_theme_variant_changed: HandlerList<dyn Fn()>,
}

impl ApplicationHost {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner: OnceCell::new(),
            data_templates: OnceCell::new(),
            styles: OnceCell::new(),
            resources: RefCell::new(None),
            styles_added: HandlerList::new(),
            styles_removed: HandlerList::new(),
            resources_changed: HandlerList::new(),
            actual_theme_variant_changed: HandlerList::new(),
        })
    }

    fn host_ref(&self) -> ResourceHostRef {
        let this: Rc<dyn IResourceHost> = self.this.upgrade().expect("the application host is alive");
        ResourceHostRef::Other(this)
    }

    fn data_templates(&self) -> DataTemplates {
        self.data_templates.get_or_init(DataTemplates::new).clone()
    }

    fn styles(&self) -> Ref<Styles> {
        self.styles.get_or_init(|| Styles::with_owner(self.host_ref())).clone()
    }

    fn resources(&self) -> Ref<ResourceDictionary> {
        if let Some(resources) = &*self.resources.borrow() {
            return resources.clone();
        }
        let resources = ResourceDictionary::with_owner(self.host_ref());
        *self.resources.borrow_mut() = Some(resources.clone());
        resources
    }

    fn set_resources(&self, value: Ref<ResourceDictionary>) {
        let this = self.host_ref();
        let old = self.resources.borrow_mut().take();
        if let Some(old) = old {
            old.remove_owner(&this);
        }
        *self.resources.borrow_mut() = Some(value.clone());
        value.add_owner(&this);
    }

    fn has_resources(&self) -> bool {
        self.resources.borrow().as_ref().is_some_and(|r| r.has_resources())
            || self.styles.get().is_some_and(|s| s.has_resources())
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let resources = self.resources.borrow().clone();
        if let Some(resources) = resources {
            if let Some(value) = resources.try_get_resource(key, theme) {
                return Some(value);
            }
        }
        self.styles().try_get_resource(key, theme)
    }

    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IPlatformSettings>() {
            return FerroLocator::current().get_service_untyped(TypeId::of::<dyn IPlatformSettings>());
        }

        if feature_type == TypeId::of::<dyn IActivatableLifetime>() {
            return FerroLocator::current().get_service_untyped(TypeId::of::<dyn IActivatableLifetime>());
        }

        // Do not return just any service from the locator.
        None
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&ApplicationHost) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }
}

impl IResourceNode for ApplicationHost {
    fn has_resources(&self) -> bool {
        ApplicationHost::has_resources(self)
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        ApplicationHost::try_get_resource(self, key, theme)
    }
}

impl IResourceHost for ApplicationHost {
    fn resources_changed(&self, handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|host| &host.resources_changed, handler)
    }

    fn notify_hosted_resources_changed(&self, e: ResourcesChangedEventArgs) {
        for (_, handler) in self.resources_changed.snapshot().iter() {
            handler(&e);
        }
    }

    fn as_style_host(&self) -> Option<&dyn IStyleHost> {
        Some(self)
    }

    fn as_theme_variant_host(&self) -> Option<&dyn IThemeVariantHost> {
        Some(self)
    }
}

impl IStyleHost for ApplicationHost {
    fn is_styles_initialized(&self) -> bool {
        self.styles.get().is_some()
    }

    fn styles(&self) -> Ref<Styles> {
        ApplicationHost::styles(self)
    }

    fn styling_parent(&self) -> Option<StyleHostRef> {
        None
    }

    fn styles_added(&self, styles: &[Rc<dyn IStyle>]) {
        for (_, handler) in self.styles_added.snapshot().iter() {
            handler(styles);
        }
    }

    fn styles_removed(&self, styles: &[Rc<dyn IStyle>]) {
        for (_, handler) in self.styles_removed.snapshot().iter() {
            handler(styles);
        }
    }
}

impl IGlobalStyles for ApplicationHost {
    fn global_styles_added(&self, handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable> {
        self.subscribe(|host| &host.styles_added, handler)
    }

    fn global_styles_removed(&self, handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable> {
        self.subscribe(|host| &host.styles_removed, handler)
    }
}

impl IThemeVariantHost for ApplicationHost {
    fn actual_theme_variant(&self) -> Option<ThemeVariant> {
        self.owner.get().and_then(WeakRef::upgrade).and_then(|application| application.actual_theme_variant())
    }

    fn actual_theme_variant_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.subscribe(|host| &host.actual_theme_variant_changed, handler)
    }
}

impl IThemeVariantRoot for ApplicationHost {
    fn is_theme_variant_root(&self) -> bool {
        true
    }
}

impl ferroui_base::IDataContextProvider for ApplicationHost {
    fn data_context(&self) -> Option<BoxedValue> {
        self.owner.get().and_then(WeakRef::upgrade).and_then(|owner| owner.data_context())
    }

    fn set_data_context(&self, value: Option<BoxedValue>) {
        if let Some(owner) = self.owner.get().and_then(WeakRef::upgrade) {
            owner.set_data_context(value);
        }
    }
}

impl IDataTemplateHost for ApplicationHost {
    fn data_templates(&self) -> DataTemplates {
        ApplicationHost::data_templates(self)
    }

    fn is_data_templates_initialized(&self) -> bool {
        self.data_templates.get().is_some()
    }
}

impl IGlobalDataTemplates for ApplicationHost {}

impl IOptionalFeatureProvider for ApplicationHost {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        ApplicationHost::try_get_feature(self, feature_type)
    }
}
