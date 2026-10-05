use crate::application::Application;
use crate::platform::{
    register_native_menu_exporter_provider, INativeMenuExporter, INativeMenuExporterProvider, ITrayIconImpl,
    MacOSProperties, PlatformManager,
};
use crate::{Button, NativeMenu, Window, WindowIcon};
use ferroui_base::collections::{
    FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior,
};
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, AttachedProperty, BoxedValue, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
    StyledPropertyMetadata, Visual,
};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The collection of the tray icons of an application: a list of
/// [`TrayIcon`] whose clear is notified as a removal.
pub struct TrayIcons {
    items: FerroList<Ref<TrayIcon>>,
    /// The subscription of [`TrayIcon`] to the changes of the collection
    /// while it is the collection of the application.
    subscription: Cell<Option<u64>>,
}

impl TrayIcons {
    pub fn new() -> Rc<Self> {
        let items = FerroList::new();
        items.set_reset_behavior(ResetBehavior::Remove);
        Rc::new(Self { items, subscription: Cell::new(None) })
    }
}

impl Deref for TrayIcons {
    type Target = FerroList<Ref<TrayIcon>>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

/// Collections compare by reference.
impl PartialEq for TrayIcons {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// An icon in the notification area of the platform.
#[repr(C)]
pub struct TrayIcon {
    base: FerroObject,
    impl_: RefCell<Option<Rc<dyn ITrayIconImpl>>>,
    is_attached: Cell<bool>,
    is_disposed: Cell<bool>,
    clicked: HandlerList<dyn Fn(&TrayIcon)>,
}

ferro_class!(TrayIcon: FerroObject);
ferro_class_info!(TrayIcon { new: TrayIcon::new });

impl FerroObjectImpl for TrayIcon {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        let Some(impl_) = this.platform_impl() else { return };

        if property == Self::icon_property().as_property() {
            impl_.set_icon(this.icon().map(|icon| icon.platform_impl()));
        } else if property == Self::is_visible_property().as_property() {
            impl_.set_is_visible(change.get_new_value::<bool>());
        } else if property == Self::tool_tip_text_property().as_property() {
            impl_.set_tool_tip_text(change.get_new_value::<Option<String>>().as_deref());
        } else if property == Self::menu_property().as_property() {
            if let Some(exporter) = impl_.menu_exporter() {
                exporter.set_native_menu(change.get_new_value::<Option<Ref<NativeMenu>>>());
            }
        }
    }
}

/// The tray icon viewed through the interfaces it implements.
struct TrayIconHandle(Ref<TrayIcon>);

impl INativeMenuExporterProvider for TrayIconHandle {
    fn native_menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        self.0.native_menu_exporter()
    }
}

impl IDisposable for TrayIconHandle {
    fn dispose(&self) {
        self.0.dispose();
    }
}

ferro_properties! {
    impl TrayIcon {
        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            Button::command_property()
                .add_owner_with::<TrayIcon>(StyledPropertyMetadata::new(None).with_enable_data_validation(true))
        }

        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            Button::command_parameter_property().add_owner::<TrayIcon>()
        }

        /// Defines the `Icons` attached property.
        pub fn icons_property() -> AttachedProperty<Option<Rc<TrayIcons>>> {
            FerroProperty::register_attached::<TrayIcon, Application, _>("Icons", None)
        }

        /// Defines the `Menu` property.
        pub fn menu_property() -> StyledProperty<Option<Ref<NativeMenu>>> {
            FerroProperty::register::<TrayIcon, _>("Menu", None)
        }

        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<Rc<WindowIcon>>> {
            Window::icon_property().add_owner::<TrayIcon>()
        }

        /// Defines the `ToolTipText` property.
        pub fn tool_tip_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TrayIcon, _>("ToolTipText", None)
        }

        /// Defines the `IsVisible` property.
        pub fn is_visible_property() -> StyledProperty<bool> {
            Visual::is_visible_property().add_owner::<TrayIcon>()
        }
    }
}

impl TrayIcon {
    fn static_constructor() {
        register_native_menu_exporter_provider::<TrayIcon>(|tray_icon| Rc::new(TrayIconHandle(tray_icon)));

        Self::icons_property().changed().subscribe(|args| {
            if args.sender().is::<Application>() {
                let (old_value, new_value) = args.get_old_and_new_value::<Option<Rc<TrayIcons>>>();

                if let Some(old_value) = old_value {
                    if let Some(token) = old_value.subscription.take() {
                        old_value.remove_collection_changed(token);
                    }
                    Self::detach_icons(&old_value.to_vec());
                }

                if let Some(new_value) = new_value {
                    let token = new_value.add_collection_changed(Rc::new(Self::icons_collection_changed));
                    if let Some(old_token) = new_value.subscription.replace(Some(token)) {
                        new_value.remove_collection_changed(old_token);
                    }
                    Self::attach_icons(&new_value.to_vec());
                }
            } else {
                panic!("TrayIcon.Icons must be set on the Application.");
            }
        });

        let _ = Dispatcher::ui_thread().shutdown_started(|_| Self::shutdown_started());
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            impl_: RefCell::new(None),
            is_attached: Cell::new(false),
            is_disposed: Cell::new(false),
            clicked: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the TrayIcon is clicked.
    /// Note, this is only supported on Win32 and some Linux DEs,
    /// on OSX this event is not raised.
    pub fn clicked(&self, handler: impl Fn(&TrayIcon) + 'static) -> Rc<dyn IDisposable> {
        let token = self.clicked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.clicked.remove(token);
            }
        })
    }

    /// Sets the tray icons of the application.
    pub fn set_icons(o: &Application, tray_icons: Option<Rc<TrayIcons>>) {
        o.set_value(Self::icons_property(), tray_icons)
    }

    /// Gets the tray icons of the application.
    pub fn get_icons(o: &Application) -> Option<Rc<TrayIcons>> {
        o.get_value(Self::icons_property())
    }

    /// Gets the `Command` property of a TrayIcon.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// Gets the parameter to pass to the `Command` property of a
    /// [`TrayIcon`].
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// Gets the Menu of the TrayIcon.
    pub fn menu(&self) -> Option<Ref<NativeMenu>> {
        self.get_value(Self::menu_property())
    }

    pub fn set_menu(&self, value: Option<Ref<NativeMenu>>) {
        self.set_value(Self::menu_property(), value)
    }

    /// Gets the icon of the TrayIcon.
    pub fn icon(&self) -> Option<Rc<WindowIcon>> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<Rc<WindowIcon>>) {
        self.set_value(Self::icon_property(), value)
    }

    /// Gets the tooltip text of the TrayIcon.
    pub fn tool_tip_text(&self) -> Option<String> {
        self.get_value(Self::tool_tip_text_property())
    }

    pub fn set_tool_tip_text(&self, value: Option<String>) {
        self.set_value(Self::tool_tip_text_property(), value)
    }

    /// Gets the visibility of the TrayIcon.
    pub fn is_visible(&self) -> bool {
        self.get_value(Self::is_visible_property())
    }

    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value)
    }

    /// The native menu exporter of the platform implementation.
    pub fn native_menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        self.platform_impl().and_then(|impl_| impl_.menu_exporter())
    }

    /// The platform implementation; upstream `Impl`.
    pub(crate) fn platform_impl(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        self.impl_.borrow().clone()
    }

    fn shutdown_started() {
        let app = Application::current().unwrap_or_else(|| panic!("Application not yet initialized."));
        let tray_icons = Self::get_icons(&app);

        if let Some(tray_icons) = tray_icons {
            Self::dispose_icons(&tray_icons.to_vec());
        }
    }

    fn icons_collection_changed(e: &NotifyCollectionChangedEventArgs<'_, Ref<TrayIcon>>) {
        if e.action == NotifyCollectionChangedAction::Move {
            return;
        }

        Self::detach_icons(e.old_items);
        Self::attach_icons(e.new_items);
    }

    fn attach_icons(icons: &[Ref<TrayIcon>]) {
        for icon in icons {
            icon.attach();
        }
    }

    fn detach_icons(icons: &[Ref<TrayIcon>]) {
        for icon in icons {
            icon.detach();
        }
    }

    fn dispose_icons(icons: &[Ref<TrayIcon>]) {
        for icon in icons {
            icon.dispose();
        }
    }

    fn attach(&self) {
        if self.is_attached.get() || self.is_disposed.get() {
            return;
        }
        self.is_attached.set(true);

        let impl_ = PlatformManager::create_tray_icon();
        *self.impl_.borrow_mut() = impl_.clone();
        let Some(impl_) = impl_ else { return };

        // Keep the native icon hidden until all properties have been initialized.
        impl_.set_is_visible(false);
        // The implementation is owned by the icon, so its callback holds
        // the icon weakly.
        let weak = self.to_ref().downgrade();
        impl_.set_on_clicked(Some(Rc::new(move || {
            let Some(this) = weak.upgrade() else { return };

            for (_, handler) in this.clicked.snapshot().iter() {
                handler(&this);
            }

            // The command and its parameter are read again for the
            // execution, as in the reference.
            if this.command().is_some_and(|command| command.can_execute(this.command_parameter().as_ref())) {
                if let Some(command) = this.command() {
                    command.execute(this.command_parameter().as_ref());
                }
            }
        })));

        impl_.set_icon(self.icon().map(|icon| icon.platform_impl()));
        impl_.set_tool_tip_text(self.tool_tip_text().as_deref());
        if let Some(exporter) = impl_.menu_exporter() {
            exporter.set_native_menu(self.menu());
        }

        if let Some(template_impl) = impl_.as_tray_icon_with_is_template_impl() {
            template_impl.set_is_template_icon(MacOSProperties::get_is_template_icon(self));
        }

        impl_.set_is_visible(self.is_visible());
    }

    fn detach(&self) {
        if !self.is_attached.get() {
            return;
        }

        self.is_attached.set(false);
        let impl_ = self.impl_.borrow_mut().take();
        if let Some(impl_) = impl_ {
            impl_.dispose();
        }
    }

    /// Disposes the tray icon (removing it from the tray area).
    pub fn dispose(&self) {
        if self.is_disposed.get() {
            return;
        }

        self.is_disposed.set(true);
        self.detach();
    }

    /// The tray icon viewed as a disposable.
    pub fn to_disposable(&self) -> Rc<dyn IDisposable> {
        Rc::new(TrayIconHandle(self.to_ref()))
    }
}
