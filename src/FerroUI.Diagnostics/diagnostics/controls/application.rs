//! Port of `Diagnostics/Controls/Application.cs`: the application as the root of the trees of
//! DevTools, a top-level group of its windows.

use super::TopLevelGroup;
use crate::diagnostics::{ClassicDesktopStyleApplicationLifetimeTopLevelGroup, IDevToolsTopLevelGroup};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::{ICloseable, InputManager};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{Styles, ThemeVariant};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use ferroui_controls::application_lifetimes::{ControlledApplicationLifetimeExitEventArgs, IApplicationLifetime};
use ferroui_controls::presentation_source::ITopLevelRenderer;
use ferroui_controls::templates::DataTemplates;
use ferroui_controls::{ThemeVariantScope, TopLevel};
use std::cell::RefCell;
use std::rc::Rc;

/// The application DevTools is attached to, as the root of its trees: the windows of the
/// application, and the members of the application that DevTools shows. It closes when the
/// application exits.
#[repr(C)]
pub struct Application {
    base: TopLevelGroup,
    application: Ref<ferroui_controls::Application>,
    closed: HandlerList<dyn Fn()>,
    renderer_root: RefCell<Option<Rc<dyn ITopLevelRenderer>>>,
    /// `_application.PropertyChanged += ApplicationOnPropertyChanged`.
    application_property_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(Application: TopLevelGroup);
ferro_class_info!(Application {});

ferro_properties! {
    impl Application {
        /// Defines the [`requested_theme_variant`](Application::requested_theme_variant) property.
        pub fn requested_theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            ThemeVariantScope::requested_theme_variant_property().add_owner::<Application>()
        }
    }
}

impl FerroObjectImpl for Application {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let application = this.application.clone();
        let lifetime = application.application_lifetime();

        if let Some(controller) = lifetime.as_ref().and_then(|lifetime| lifetime.as_controlled_application_lifetime()) {
            // `controller.Exit -= eh` from the handler itself.
            let subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
            let handler_subscription = subscription.clone();
            let weak = this.to_ref().downgrade();
            let exit = controller.exit(Rc::new(move |_: &ControlledApplicationLifetimeExitEventArgs| {
                let own = handler_subscription.borrow_mut().take();
                if let Some(own) = own {
                    own.dispose();
                }
                if let Some(this) = weak.upgrade() {
                    this.raise_closed();
                }
            }));
            *subscription.borrow_mut() = Some(exit);
        }

        let renderer_root = match lifetime.as_ref() {
            Some(lifetime) => {
                if let Some(classic) = lifetime.as_classic_desktop_style_application_lifetime() {
                    classic.main_window().map(|main_window| main_window.renderer())
                } else if let Some(single) = lifetime.as_single_view_application_lifetime() {
                    single
                        .main_view()
                        .and_then(|main_view| main_view.visual_root())
                        .and_then(|visual_root| visual_root.cast::<TopLevel>())
                        .map(|top_level| top_level.renderer())
                } else {
                    None
                }
            }
            None => None,
        };
        *this.renderer_root.borrow_mut() = renderer_root;

        this.set_current_value(Self::requested_theme_variant_property(), application.requested_theme_variant());

        let weak = this.to_ref().downgrade();
        let subscription = application.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.application_on_property_changed(e);
            }
        });
        *this.application_property_changed.borrow_mut() = Some(subscription);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::requested_theme_variant_property().as_property() {
            this.application.set_requested_theme_variant(change.get_new_value::<Option<ThemeVariant>>());
        }
    }
}

impl Application {
    pub fn construct(
        group: Rc<dyn IDevToolsTopLevelGroup>,
        application: Ref<ferroui_controls::Application>,
    ) -> Self {
        debug_assert!(
            group.as_any().is::<ClassicDesktopStyleApplicationLifetimeTopLevelGroup>(),
            "the group of an application is the group of the windows of its lifetime"
        );
        Self {
            base: TopLevelGroup::construct(group),
            application,
            closed: HandlerList::new(),
            renderer_root: RefCell::new(None),
            application_property_changed: RefCell::new(None),
        }
    }

    /// C# `new Application(ClassicDesktopStyleApplicationLifetimeTopLevelGroup group,
    /// Application application)`: `group` is the
    /// [`ClassicDesktopStyleApplicationLifetimeTopLevelGroup`] of the lifetime of `application`.
    pub fn new(group: Rc<dyn IDevToolsTopLevelGroup>, application: Ref<ferroui_controls::Application>) -> Ref<Self> {
        instantiate(Self::construct(group, application))
    }

    /// Raised when the application exits. Disposing the returned handle unsubscribes.
    pub fn closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.closed.remove(token);
            }
        })
    }

    fn raise_closed(&self) {
        for (_, handler) in self.closed.snapshot().iter() {
            handler();
        }
    }

    /// The application as `ICloseable` (the class implements it in the original).
    pub fn as_closeable(this: Ref<Self>) -> Rc<dyn ICloseable> {
        Rc::new(ApplicationHandle(this))
    }

    /// The application as `IDisposable` (the class implements it in the original).
    pub fn as_disposable(this: Ref<Self>) -> Rc<dyn IDisposable> {
        Rc::new(ApplicationHandle(this))
    }

    /// The application DevTools is attached to.
    pub fn instance(&self) -> Ref<ferroui_controls::Application> {
        self.application.clone()
    }

    /// The data context of the application.
    pub fn data_context(&self) -> Option<BoxedValue> {
        self.application.data_context()
    }

    /// Gets or sets the application's global data templates.
    pub fn data_templates(&self) -> DataTemplates {
        self.application.data_templates()
    }

    /// Gets the application's input manager.
    pub fn input_manager(&self) -> Option<Rc<InputManager>> {
        self.application.input_manager()
    }

    /// Gets the application's global resource dictionary.
    pub fn resources(&self) -> Ref<ResourceDictionary> {
        self.application.resources()
    }

    /// Gets the application's global styles.
    ///
    /// Global styles apply to all windows in the application.
    pub fn styles(&self) -> Ref<Styles> {
        self.application.styles()
    }

    /// Application lifetime, use it for things like setting the main window and exiting the app from code
    /// Currently supported lifetimes are:
    /// - `IClassicDesktopStyleApplicationLifetime`
    /// - `ISingleViewApplicationLifetime`
    /// - `IControlledApplicationLifetime`
    pub fn application_lifetime(&self) -> Option<Rc<dyn IApplicationLifetime>> {
        self.application.application_lifetime()
    }

    /// Application name to be used for various platform-specific purposes
    pub fn name(&self) -> Option<String> {
        self.application.name()
    }

    /// Gets the renderer of the root of the visual tree of the application: the renderer of
    /// its main window, or of the top level of its main view.
    pub fn renderer_root(&self) -> Option<Rc<dyn ITopLevelRenderer>> {
        self.renderer_root.borrow().clone()
    }

    /// The theme variant requested for the application (`ThemeVariantScope.RequestedThemeVariant`).
    pub fn requested_theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::requested_theme_variant_property())
    }

    pub fn set_requested_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::requested_theme_variant_property(), value);
    }

    /// `Dispose`: stops following the changes of the application.
    pub fn dispose(&self) {
        let subscription = self.application_property_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn application_on_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == ferroui_controls::Application::requested_theme_variant_property().as_property() {
            self.set_current_value(
                Self::requested_theme_variant_property(),
                e.get_new_value::<Option<ThemeVariant>>(),
            );
        }
    }
}

/// The `ICloseable` and `IDisposable` contracts of an [`Application`].
struct ApplicationHandle(Ref<Application>);

impl ICloseable for ApplicationHandle {
    fn closed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.closed(move || handler())
    }
}

impl IDisposable for ApplicationHandle {
    fn dispose(&self) {
        self.0.dispose();
    }
}
