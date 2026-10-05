use super::{
    RefreshInfoProvider, RefreshRequestedEventArgs, RefreshVisualizer, ScrollViewerIRefreshInfoProviderAdapter,
};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ContentControl, ContentControlImpl, ControlImpl, Grid};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{InputElementImpl, PullDirection};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, DirectProperty, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Represents a container control that provides a [`RefreshVisualizer`] and
/// pull-to-refresh functionality for scrollable content.
#[repr(C)]
pub struct RefreshContainer {
    base: ContentControl,
    has_default_refresh_info_provider_adapter: Cell<bool>,
    refresh_info_provider_adapter: RefCell<Option<Rc<ScrollViewerIRefreshInfoProviderAdapter>>>,
    refresh_info_provider: RefCell<Option<Ref<RefreshInfoProvider>>>,
    visualizer_size_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visualizer_presenter: RefCell<Option<Ref<Grid>>>,
    refresh_visualizer: RefCell<Option<Ref<RefreshVisualizer>>>,
    /// The subscription to the refresh requested event of the visualizer.
    visualizer_refresh_requested: Cell<Option<RoutedEventHandlerToken>>,
    has_default_refresh_visualizer: Cell<bool>,
}

ferro_class!(RefreshContainer: ContentControl);
ferroui_base::ferro_class_info!(RefreshContainer { new: RefreshContainer::new });
ferro_impl_classes!(
    RefreshContainer: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    ContentControlImpl
);

/// Balances the deferral count of forwarded refresh requested args when
/// dropped: the `finally` block of the refresh requested handler.
struct DecrementCountGuard<'a>(&'a RefreshRequestedEventArgs);

impl Drop for DecrementCountGuard<'_> {
    fn drop(&mut self) {
        self.0.decrement_count();
    }
}

impl FerroObjectImpl for RefreshContainer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.has_default_refresh_info_provider_adapter.set(true);
        let adapter = ScrollViewerIRefreshInfoProviderAdapter::new(this.pull_direction(), this.is_mouse_enabled());
        drop(this.refresh_info_provider_adapter.replace(Some(adapter.clone())));
        this.raise_refresh_info_provider_adapter_changed(Some(adapter));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::refresh_info_provider_adapter_property().as_property() {
            let refresh_visualizer = this.refresh_visualizer.borrow().clone();
            if let Some(refresh_visualizer) = refresh_visualizer {
                let refresh_info_provider = this.refresh_info_provider.borrow().clone();
                if let Some(refresh_info_provider) = refresh_info_provider {
                    refresh_visualizer.set_refresh_info_provider(Some(refresh_info_provider));
                } else if let Some(adapter) = this.refresh_info_provider_adapter() {
                    let refresh_info_provider =
                        adapter.adapt_from_tree(this, Some(refresh_visualizer.bounds().size()));
                    drop(this.refresh_info_provider.replace(refresh_info_provider.clone()));

                    if let Some(refresh_info_provider) = refresh_info_provider {
                        refresh_visualizer.set_refresh_info_provider(Some(refresh_info_provider));
                        if let Some(adapter) = this.refresh_info_provider_adapter() {
                            adapter.set_animations(&refresh_visualizer);
                        }
                    }
                }
            }
        } else if change.property() == Self::visualizer_property().as_property() {
            let refresh_visualizer = this.refresh_visualizer.borrow().clone();

            let visualizer_presenter = this.visualizer_presenter.borrow().clone();
            if let Some(visualizer_presenter) = visualizer_presenter {
                visualizer_presenter.children().clear();
                if let Some(refresh_visualizer) = &refresh_visualizer {
                    visualizer_presenter.children().add(refresh_visualizer.clone());
                }
            }

            if let Some(refresh_visualizer) = refresh_visualizer {
                let weak = this.to_ref().downgrade();
                this.visualizer_refresh_requested.set(Some(refresh_visualizer.refresh_requested(move |_, e| {
                    if let Some(this) = weak.upgrade() {
                        this.visualizer_refresh_requested(e);
                    }
                })));

                let weak = this.to_ref().downgrade();
                let object: &FerroObject = &refresh_visualizer;
                let subscription =
                    FerroObjectExtensions::get_observable(object, Visual::bounds_property())
                        .subscribe_fn(move |bounds| {
                            if let Some(this) = weak.upgrade() {
                                this.on_visualizer_size_changed(bounds);
                            }
                        });
                drop(this.visualizer_size_subscription.replace(Some(subscription)));
            }
        } else if change.property() == Self::pull_direction_property().as_property() {
            this.on_pull_direction_changed();
        } else if change.property() == Self::is_mouse_enabled_property().as_property() {
            this.on_is_mouse_enabled_changed();
        }
    }
}

impl TemplatedControlImpl for RefreshContainer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let visualizer_presenter = e.name_scope().find_as::<Grid>("PART_RefreshVisualizerPresenter");
        drop(this.visualizer_presenter.replace(visualizer_presenter));

        let refresh_visualizer = this.refresh_visualizer.borrow().clone();
        match refresh_visualizer {
            None => {
                this.has_default_refresh_visualizer.set(true);
                this.set_visualizer(Some(RefreshVisualizer::new()));
            }
            Some(refresh_visualizer) => {
                this.has_default_refresh_visualizer.set(false);
                this.raise_property_changed(
                    Self::visualizer_property().as_property(),
                    Some(&None::<Ref<RefreshVisualizer>>),
                    &Some(refresh_visualizer),
                    BindingPriority::LocalValue,
                    true,
                );
            }
        }

        this.on_pull_direction_changed();
    }
}

ferroui_base::ferro_properties! { impl RefreshContainer {
    ferro_property!(
        pub(crate) fn refresh_info_provider_adapter_property()
            -> DirectProperty<RefreshContainer, Option<Rc<ScrollViewerIRefreshInfoProviderAdapter>>> {
            FerroProperty::register_direct::<RefreshContainer, _>(
                "RefreshInfoProviderAdapter",
                |s| s.refresh_info_provider_adapter(),
                Some(|s, o| s.set_refresh_info_provider_adapter(o)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Visualizer` property.
        pub fn visualizer_property() -> DirectProperty<RefreshContainer, Option<Ref<RefreshVisualizer>>> {
            FerroProperty::register_direct::<RefreshContainer, _>(
                "Visualizer",
                |s| s.visualizer(),
                Some(|s, o| s.set_visualizer(o)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `PullDirection` property.
        pub fn pull_direction_property() -> StyledProperty<PullDirection> {
            FerroProperty::register::<RefreshContainer, _>("PullDirection", PullDirection::TopToBottom)
        }
    );

    ferro_property!(
        /// Defines the `IsMouseEnabled` property.
        ///
        /// Allows to enable the pull to refresh gesture for devices using a
        /// mouse. Disabled by default.
        pub fn is_mouse_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<RefreshContainer, _>("IsMouseEnabled", false)
        }
    );
} }

impl RefreshContainer {
    pub(crate) const DEFAULT_PULL_DIMENSION_SIZE: i32 = 100;

    ferro_routed_event!(
        /// Defines the `RefreshRequested` event.
        pub fn refresh_requested_event() -> RoutedEvent<RefreshRequestedEventArgs> {
            RoutedEvent::register::<RefreshContainer, _>("RefreshRequested", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            has_default_refresh_info_provider_adapter: Cell::new(false),
            refresh_info_provider_adapter: RefCell::new(None),
            refresh_info_provider: RefCell::new(None),
            visualizer_size_subscription: RefCell::new(None),
            visualizer_presenter: RefCell::new(None),
            refresh_visualizer: RefCell::new(None),
            visualizer_refresh_requested: Cell::new(None),
            has_default_refresh_visualizer: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn refresh_info_provider_adapter(&self) -> Option<Rc<ScrollViewerIRefreshInfoProviderAdapter>> {
        self.refresh_info_provider_adapter.borrow().clone()
    }

    pub(crate) fn set_refresh_info_provider_adapter(&self, value: Option<Rc<ScrollViewerIRefreshInfoProviderAdapter>>) {
        self.has_default_refresh_info_provider_adapter.set(false);
        self.set_and_raise(Self::refresh_info_provider_adapter_property(), &self.refresh_info_provider_adapter, value);
    }

    /// Gets or sets a value that indicates whether the pull-to-refresh
    /// gesture is enabled for desktop devices. Allows to enable the pull to
    /// refresh gesture for devices using a mouse. Disabled by default.
    pub fn is_mouse_enabled(&self) -> bool {
        self.get_value(Self::is_mouse_enabled_property())
    }

    pub fn set_is_mouse_enabled(&self, value: bool) {
        self.set_value(Self::is_mouse_enabled_property(), value)
    }

    /// Gets or sets the [`RefreshVisualizer`] for this container.
    pub fn visualizer(&self) -> Option<Ref<RefreshVisualizer>> {
        self.refresh_visualizer.borrow().clone()
    }

    pub fn set_visualizer(&self, value: Option<Ref<RefreshVisualizer>>) {
        let refresh_visualizer = self.refresh_visualizer.borrow().clone();
        if let Some(refresh_visualizer) = refresh_visualizer {
            let subscription = self.visualizer_size_subscription.borrow().clone();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            if let Some(token) = self.visualizer_refresh_requested.take() {
                refresh_visualizer.remove_handler(RefreshVisualizer::refresh_requested_event(), token);
            }
        }

        self.set_and_raise(Self::visualizer_property(), &self.refresh_visualizer, value);
    }

    /// Gets or sets a value that specifies the direction to pull to
    /// initiate a refresh.
    pub fn pull_direction(&self) -> PullDirection {
        self.get_value(Self::pull_direction_property())
    }

    pub fn set_pull_direction(&self, value: PullDirection) {
        self.set_value(Self::pull_direction_property(), value)
    }

    /// Occurs when an update of the content has been initiated.
    pub fn refresh_requested(
        &self,
        handler: impl Fn(&Interactive, &RefreshRequestedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::refresh_requested_event(), handler)
    }

    /// Notifies a change of the adapter property from nothing to `adapter`,
    /// whether or not the adapter itself changed.
    fn raise_refresh_info_provider_adapter_changed(&self, adapter: Option<Rc<ScrollViewerIRefreshInfoProviderAdapter>>) {
        self.raise_property_changed(
            Self::refresh_info_provider_adapter_property().as_property(),
            Some(&None::<Rc<ScrollViewerIRefreshInfoProviderAdapter>>),
            &adapter,
            BindingPriority::LocalValue,
            true,
        );
    }

    fn on_visualizer_size_changed(&self, obj: Rect) {
        if self.has_default_refresh_info_provider_adapter.get() {
            if let Some(adapter) = self.refresh_info_provider_adapter() {
                adapter.update_visualizer_size(Some(obj.size()));
                self.raise_refresh_info_provider_adapter_changed(Some(adapter));
            }
        }
    }

    fn visualizer_refresh_requested(&self, e: &RefreshRequestedEventArgs) {
        // Guarantee the inner deferral is balanced even if a downstream
        // handler of the refresh requested event panics synchronously from
        // `raise_event`. Without this, a synchronously failing consumer
        // leaves the deferral count of the visualizer above zero forever, so
        // the refresh never completes and the spinner stays stuck in
        // Refreshing.
        let ev = RefreshRequestedEventArgs::with_deferral(e.get_deferral(), Some(Self::refresh_requested_event()));
        let _guard = DecrementCountGuard(&ev);
        self.raise_event(&ev);
    }

    fn on_pull_direction_changed(&self) {
        let visualizer_presenter = self.visualizer_presenter.borrow().clone();
        let refresh_visualizer = self.refresh_visualizer.borrow().clone();
        let (Some(visualizer_presenter), Some(refresh_visualizer)) = (visualizer_presenter, refresh_visualizer) else {
            return;
        };

        let has_default_refresh_visualizer = self.has_default_refresh_visualizer.get();
        let default_pull_dimension_size = f64::from(Self::DEFAULT_PULL_DIMENSION_SIZE);
        let pull_direction = self.pull_direction();

        match pull_direction {
            PullDirection::TopToBottom => {
                visualizer_presenter.set_vertical_alignment(VerticalAlignment::Top);
                visualizer_presenter.set_horizontal_alignment(HorizontalAlignment::Stretch);
                if has_default_refresh_visualizer {
                    refresh_visualizer.set_pull_direction(PullDirection::TopToBottom);
                    refresh_visualizer.set_height(default_pull_dimension_size);
                    refresh_visualizer.set_width(f64::NAN);
                }
            }
            PullDirection::BottomToTop => {
                visualizer_presenter.set_vertical_alignment(VerticalAlignment::Bottom);
                visualizer_presenter.set_horizontal_alignment(HorizontalAlignment::Stretch);
                if has_default_refresh_visualizer {
                    refresh_visualizer.set_pull_direction(PullDirection::BottomToTop);
                    refresh_visualizer.set_height(default_pull_dimension_size);
                    refresh_visualizer.set_width(f64::NAN);
                }
            }
            PullDirection::LeftToRight => {
                visualizer_presenter.set_vertical_alignment(VerticalAlignment::Stretch);
                visualizer_presenter.set_horizontal_alignment(HorizontalAlignment::Left);
                if has_default_refresh_visualizer {
                    refresh_visualizer.set_pull_direction(PullDirection::LeftToRight);
                    refresh_visualizer.set_width(default_pull_dimension_size);
                    refresh_visualizer.set_height(f64::NAN);
                }
            }
            PullDirection::RightToLeft => {
                visualizer_presenter.set_vertical_alignment(VerticalAlignment::Stretch);
                visualizer_presenter.set_horizontal_alignment(HorizontalAlignment::Right);
                if has_default_refresh_visualizer {
                    refresh_visualizer.set_pull_direction(PullDirection::RightToLeft);
                    refresh_visualizer.set_width(default_pull_dimension_size);
                    refresh_visualizer.set_height(f64::NAN);
                }
            }
        }

        if self.has_default_refresh_info_provider_adapter.get() {
            let adapter = self.refresh_info_provider_adapter();

            if has_default_refresh_visualizer {
                let size = Size::new(refresh_visualizer.width(), refresh_visualizer.height());
                if let Some(adapter) = &adapter {
                    adapter.update_visualizer_size(Some(size));
                }
            }

            if let Some(adapter) = &adapter {
                adapter.update_pull_direction(pull_direction);
            }
            self.raise_refresh_info_provider_adapter_changed(adapter);
        }
    }

    fn on_is_mouse_enabled_changed(&self) {
        if let Some(adapter) = self.refresh_info_provider_adapter() {
            adapter.update_is_mouse_enabled(self.is_mouse_enabled());
        }
    }

    /// Initiates an update of the content.
    pub fn request_refresh(&self) {
        let refresh_visualizer = self.refresh_visualizer.borrow().clone();
        if let Some(refresh_visualizer) = refresh_visualizer {
            refresh_visualizer.request_refresh();
        }
    }
}
