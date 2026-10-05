use super::{INavigation, NavigatedFromEventArgs, NavigatedToEventArgs, NavigatingFromEventArgs};
use crate::automation::AutomationProperties;
use crate::i_headered::{register_headered, IHeadered};
use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::templates::IDataTemplate;
use crate::{ControlImpl, ControlImplExt};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, AnyValue, BoxedValue,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, VisualImpl,
};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// What a handler of the `Navigating` event of a page returns: the rest of
/// its (possibly asynchronous) work. The navigation awaits it before the
/// next handler runs.
pub type NavigatingTask = Pin<Box<dyn Future<Output = ()>>>;

type NavigatingHandler = dyn Fn(&NavigatingFromEventArgs) -> NavigatingTask;

/// Abstract base class for all page types.
#[repr(C)]
pub struct Page {
    base: TemplatedControl,
    navigation: RefCell<Option<Rc<dyn INavigation>>>,
    navigated_to: HandlerList<dyn Fn(&NavigatedToEventArgs)>,
    navigating: HandlerList<NavigatingHandler>,
    navigated_from: HandlerList<dyn Fn(&NavigatedFromEventArgs)>,
}

ferro_class! {
    Page: TemplatedControl, virtuals PageImpl: TemplatedControlImpl {
        /// Called when the page has been navigated to.
        fn on_navigated_to(this, args: &NavigatedToEventArgs);
        /// Called when the page is about to be navigated from.
        ///
        /// Setting the cancel flag of the args here prevents the
        /// `Navigating` asynchronous handlers from running and aborts the
        /// navigation. This method is called before the `Navigating` event.
        fn on_navigating_from(this, args: &NavigatingFromEventArgs);
        /// Called when the page has been navigated from.
        fn on_navigated_from(this, args: &NavigatedFromEventArgs);
        /// Called when the system back button is pressed. Returns true if
        /// the back press was handled.
        fn on_system_back_button_pressed(this) -> bool;
        /// Called when the safe-area padding changes.
        fn update_content_safe_area_padding(this);
        /// Called when the active child page changes.
        fn update_active_page(this);
    }
}

ferro_class_info!(Page {});

ferro_impl_classes!(
    Page: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for Page {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::safe_area_padding_property().as_property()
            || change.property() == TemplatedControl::padding_property().as_property()
        {
            this.update_content_safe_area_padding();
        }

        if change.property() == Self::header_property().as_property() {
            let header = change.get_new_value::<Option<BoxedValue>>();
            let name = header.as_ref().and_then(|header| {
                let header: &dyn AnyValue = &**header;
                header.downcast_ref::<String>().cloned()
            });
            AutomationProperties::set_name(this, Some(name.as_deref().unwrap_or("")));
        }
    }
}

impl ControlImpl for Page {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.update_content_safe_area_padding();
    }
}

impl PageImpl for Page {
    fn on_navigated_to(this: &Self, args: &NavigatedToEventArgs) {
        for (_, handler) in this.navigated_to.snapshot().iter() {
            handler(args);
        }
    }

    fn on_navigating_from(_this: &Self, _args: &NavigatingFromEventArgs) {}

    fn on_navigated_from(this: &Self, args: &NavigatedFromEventArgs) {
        for (_, handler) in this.navigated_from.snapshot().iter() {
            handler(args);
        }
    }

    fn on_system_back_button_pressed(_this: &Self) -> bool {
        false
    }

    fn update_content_safe_area_padding(_this: &Self) {}

    fn update_active_page(_this: &Self) {}
}

impl IHeadered for Page {
    fn header(&self) -> Option<BoxedValue> {
        Page::header(self)
    }

    fn set_header(&self, value: Option<BoxedValue>) {
        Page::set_header(self, value)
    }
}

ferro_properties! {
    impl Page {
        /// Defines the `SafeAreaPadding` property.
        pub fn safe_area_padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<Page, _>(
                "SafeAreaPadding",
                StyledPropertyOptions::new(Thickness::default()).validate(|value| {
                    TemplatedControl::padding_property().validate_value().is_none_or(|validate| validate(value))
                }),
            )
        }

        /// Defines the `Header` property.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Page, _>("Header", None)
        }

        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Page, _>("HeaderTemplate", None)
        }

        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Page, _>("Icon", None)
        }

        /// Defines the `IconTemplate` property.
        pub fn icon_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Page, _>("IconTemplate", None)
        }

        /// Defines the `CurrentPage` property.
        pub fn current_page_property() -> StyledProperty<Option<Ref<Page>>> {
            FerroProperty::register::<Page, _>("CurrentPage", None)
        }

        /// Defines the `IsInNavigationPage` property.
        pub fn is_in_navigation_page_property() -> StyledProperty<bool> {
            FerroProperty::register::<Page, _>("IsInNavigationPage", false)
        }

        /// Defines the `Navigation` property.
        pub fn navigation_property() -> DirectProperty<Page, Option<Rc<dyn INavigation>>> {
            FerroProperty::register_direct::<Page, _>(
                "Navigation",
                |o| o.navigation(),
                Some(|o, v| o.set_navigation(v)),
                None,
            )
        }
    }
}

impl Page {
    ferro_routed_event!(
        /// Defines the routed event raised when the system back button is
        /// pressed.
        pub fn page_navigation_system_back_button_pressed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Page, _>("PageNavigationSystemBackButtonPressed", RoutingStrategies::DIRECT)
        }
    );

    fn static_constructor() {
        register_headered::<Page>();

        Layoutable::affects_measure::<Page>(&[Self::safe_area_padding_property().as_property()]);
        Self::page_navigation_system_back_button_pressed_event().add_class_handler::<Page>(|sender, event_args| {
            if event_args.handled() {
                return;
            }

            if sender.on_system_back_button_pressed() {
                event_args.set_handled(true);
            }
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            navigation: RefCell::new(None),
            navigated_to: HandlerList::new(),
            navigating: HandlerList::new(),
            navigated_from: HandlerList::new(),
        }
    }

    /// The header content displayed in the navigation bar or tab strip.
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    /// The data template used to display the header.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    /// The icon displayed alongside the page header.
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }

    /// The data template used to display the icon.
    pub fn icon_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::icon_template_property())
    }

    pub fn set_icon_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::icon_template_property(), value)
    }

    /// The safe-area padding applied to this page's content.
    pub fn safe_area_padding(&self) -> Thickness {
        self.get_value(Self::safe_area_padding_property())
    }

    pub fn set_safe_area_padding(&self, value: Thickness) {
        self.set_value(Self::safe_area_padding_property(), value)
    }

    /// The currently active child page.
    pub fn current_page(&self) -> Option<Ref<Page>> {
        self.get_value(Self::current_page_property())
    }

    pub fn set_current_page(&self, value: impl Into<Nullable<Page>>) {
        self.set_value(Self::current_page_property(), value.into().0)
    }

    /// The navigation service provided by the parent navigation page.
    pub fn navigation(&self) -> Option<Rc<dyn INavigation>> {
        self.navigation.borrow().clone()
    }

    pub fn set_navigation(&self, value: Option<Rc<dyn INavigation>>) {
        self.set_and_raise(Self::navigation_property(), &self.navigation, value);
    }

    /// Whether this page is currently hosted inside a navigation page.
    pub fn is_in_navigation_page(&self) -> bool {
        self.get_value(Self::is_in_navigation_page_property())
    }

    pub fn set_is_in_navigation_page(&self, value: bool) {
        self.set_value(Self::is_in_navigation_page_property(), value)
    }

    /// Raised when the system back button is pressed while this page is
    /// active.
    pub fn page_navigation_system_back_button_pressed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::page_navigation_system_back_button_pressed_event(), handler)
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&Page) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Occurs when the page has been navigated to. Disposing the returned
    /// handle unsubscribes.
    pub fn navigated_to(&self, handler: impl Fn(&NavigatedToEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&NavigatedToEventArgs)>(|page| &page.navigated_to, Rc::new(handler))
    }

    /// Occurs when the page is about to be navigated from. Disposing the
    /// returned handle unsubscribes.
    ///
    /// Each subscriber is awaited in turn. Set the cancel flag of the args
    /// to abort the navigation; remaining subscribers are not invoked once
    /// cancellation is requested. If a subscriber panics, the panic
    /// propagates to the task of the calling navigation method (such as the
    /// push of a navigation page) and the navigation is aborted.
    pub fn navigating(
        &self,
        handler: impl Fn(&NavigatingFromEventArgs) -> NavigatingTask + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe::<NavigatingHandler>(|page| &page.navigating, Rc::new(handler))
    }

    /// Occurs when the page has been navigated from. Disposing the returned
    /// handle unsubscribes.
    pub fn navigated_from(&self, handler: impl Fn(&NavigatedFromEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&NavigatedFromEventArgs)>(|page| &page.navigated_from, Rc::new(handler))
    }

    pub(crate) fn send_navigated_to(&self, args: &NavigatedToEventArgs) {
        self.on_navigated_to(args)
    }

    /// Asks the page whether it may be navigated from: the virtual first,
    /// then each `Navigating` handler in turn, stopping at the first one
    /// that cancels.
    pub(crate) async fn send_navigating_async(this: Ref<Page>, args: NavigatingFromEventArgs) {
        this.on_navigating_from(&args);

        if args.cancel() {
            return;
        }

        let navigating = this.navigating.snapshot();
        for (_, handler) in navigating.iter() {
            handler(&args).await;
            if args.cancel() {
                return;
            }
        }
    }

    pub(crate) fn send_navigated_from(&self, args: &NavigatedFromEventArgs) {
        self.on_navigated_from(args)
    }

    pub(crate) fn set_in_navigation_page(&self, value: bool) {
        self.set_current_value(Self::is_in_navigation_page_property(), value)
    }
}
