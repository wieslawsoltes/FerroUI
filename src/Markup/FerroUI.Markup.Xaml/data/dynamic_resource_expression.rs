//! Port of `Data/DynamicResourceExpression.cs`.

use crate::converters::ColorToBrushConverter;
use ferroui_base::controls::{IResourceProvider, ResourceHostRef, ResourceKey, ResourcesChangedEventArgs, WeakResourceHost};
use ferroui_base::data::core::{Publish, UntypedBindingExpression, UntypedBindingExpressionBase, ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, DoNothingType, FerroProperty, Ref, StyledElement, UnsetValueType, WeakRef};
use ferroui_controls::Application;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// What a dynamic resource is looked up from when its target does not host
/// resources itself: the nearest element, resource provider or resource
/// host above the place the resource is used.
#[derive(Clone)]
pub(crate) enum DynamicResourceAnchor {
    Element(Ref<StyledElement>),
    Provider(Rc<dyn IResourceProvider>),
    Host(ResourceHostRef),
}

/// The anchor as the expression keeps it. The managed original holds the
/// element and the host themselves, which the collector makes harmless; here
/// an element owns the expressions of its values, and an expression that held
/// the element (or an element above it) would keep both alive, so they are
/// held weakly. A resource provider is no element and stays held.
enum HeldAnchor {
    Element(WeakRef<StyledElement>),
    Provider(Rc<dyn IResourceProvider>),
    Host(WeakResourceHost),
}

impl From<DynamicResourceAnchor> for HeldAnchor {
    fn from(anchor: DynamicResourceAnchor) -> Self {
        match anchor {
            DynamicResourceAnchor::Element(element) => HeldAnchor::Element(element.downgrade()),
            DynamicResourceAnchor::Provider(provider) => HeldAnchor::Provider(provider),
            DynamicResourceAnchor::Host(host) => HeldAnchor::Host(host.downgrade()),
        }
    }
}

/// The binding expression of a dynamic resource: publishes the resource and
/// follows the resources and the theme variant of its host.
pub(crate) struct DynamicResourceExpression {
    this: Weak<DynamicResourceExpression>,
    base: UntypedBindingExpressionBase,
    resource_key: ResourceKey,
    anchor: Option<HeldAnchor>,
    /// Held weakly, as the anchor is: the host is most often the element
    /// that owns this expression.
    host: RefCell<Option<WeakResourceHost>>,
    provider: RefCell<Option<Rc<dyn IResourceProvider>>>,
    override_theme_variant: Cell<bool>,
    target_type_is_brush: Cell<bool>,
    theme_variant: RefCell<Option<ThemeVariant>>,
    owner_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    host_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl DynamicResourceExpression {
    pub(crate) fn new(
        resource_key: ResourceKey,
        anchor: Option<DynamicResourceAnchor>,
        theme_variant: Option<ThemeVariant>,
        priority: BindingPriority,
    ) -> Rc<Self> {
        ferroui_base::perf_count!(DynamicResourceExpressionsCreated);
        Rc::new_cyclic(|this: &Weak<DynamicResourceExpression>| Self {
            this: this.clone(),
            base: UntypedBindingExpressionBase::new(this.clone(), priority, None, false),
            resource_key,
            anchor: anchor.map(HeldAnchor::from),
            host: RefCell::new(None),
            provider: RefCell::new(None),
            override_theme_variant: Cell::new(false),
            target_type_is_brush: Cell::new(false),
            theme_variant: RefCell::new(theme_variant),
            owner_changed_subscription: RefCell::new(None),
            host_subscriptions: RefCell::new(Vec::new()),
        })
    }

    // Read by diagnostics in the managed original; here by the tests.
    #[allow(dead_code)]
    pub(crate) fn resource_key(&self) -> &ResourceKey {
        &self.resource_key
    }

    fn on_start(&self) {
        let host = self.try_get_resource_host();
        if host.is_none() {
            // The target is not an IResourceHost, so we need to find one from the anchor.
            if let Some(HeldAnchor::Provider(provider)) = &self.anchor {
                self.set_host(provider.owner());
                *self.provider.borrow_mut() = Some(provider.clone());
                self.override_theme_variant.set(self.theme_variant.borrow().is_some());
            }
        } else {
            self.set_host(host);
        }

        // If we wouldn't find a host or provider then log an error: we can't do anything.
        if self.host().is_none() && self.provider.borrow().is_none() {
            self.log_error(
                &format!(
                    "Unable to find IResourceHost or IResourceProvider from which to lookup DynamicResource {}.",
                    self.resource_key
                ),
                LogEventLevel::Error,
            );
            return;
        }

        // Hook up events.
        let provider = self.provider.borrow().clone();
        if let Some(provider) = provider {
            let weak = self.this.clone();
            let subscription = provider.owner_changed(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_resource_provider_owner_changed();
                }
            }));
            *self.owner_changed_subscription.borrow_mut() = Some(subscription);
        }
        let host = self.host();
        self.subscribe(host.as_ref());

        // And publish the initial value.
        let target_type = self.base.target_type();
        self.target_type_is_brush
            .set(target_type.is::<Rc<dyn IBrush>>() || target_type.is::<Option<Rc<dyn IBrush>>>());
        self.publish_value();
    }

    fn on_stop(&self) {
        let subscription = self.owner_changed_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.unsubscribe();
        *self.host.borrow_mut() = None;
        *self.provider.borrow_mut() = None;
    }

    fn on_resource_provider_owner_changed(&self) {
        self.unsubscribe();
        let host = self.provider.borrow().as_ref().and_then(|provider| provider.owner());
        self.set_host(host.clone());
        self.subscribe(host.as_ref());
        self.publish_value();
    }

    fn actual_theme_variant_changed(&self) {
        if !self.base.is_running() {
            return;
        }

        let theme_variant =
            self.host().as_ref().and_then(|host| host.as_theme_variant_host().and_then(|host| host.actual_theme_variant()));
        *self.theme_variant.borrow_mut() = theme_variant;
        self.publish_value();
    }

    fn publish_value(&self) {
        let value = match self.host() {
            Some(host) => {
                let theme = self.theme_variant.borrow().clone();
                let value =
                    host.find_resource_for_theme(theme.as_ref(), &self.resource_key).or_else(|| Some(FerroProperty::unset_value()));
                if self.target_type_is_brush.get() {
                    ColorToBrushConverter::convert_to(value, Some(ValueType::of::<Rc<dyn IBrush>>()))
                } else {
                    value
                }
            }
            None => Some(FerroProperty::unset_value()),
        };
        let value = self.convert_for_target(value);
        self.base.publish_value(Publish::Value(value), None, false);
    }

    /// An untyped value published to a property must hold exactly the value
    /// type of the property (where the managed runtime assigns any
    /// compatible object): the value is cast or wrapped to it here. A value
    /// that cannot be converted is published as it is, and rejected by the
    /// property like any other invalid value.
    fn convert_for_target(&self, value: Option<BoxedValue>) -> Option<BoxedValue> {
        if self.base.target_property().is_none() {
            return value;
        }
        if value.as_ref().is_some_and(|v| v.is::<UnsetValueType>() || v.is::<DoNothingType>()) {
            return value;
        }
        // What is assignable to the type of the property is cast to it (a
        // handle of a class to a contract it implements, a value to its
        // nullable form, and the two together), before the conversions.
        let target_type = self.base.target_type();
        if let Some(cast) = value.as_ref().and_then(|v| ValueTypes::try_cast(v, target_type)) {
            return Some(cast);
        }
        // Formatting a value as text is not an implicit conversion: a resource
        // of another type is no value for a text property.
        if target_type.is::<String>() || target_type.is::<Option<String>>() {
            return value;
        }
        match ValueTypes::try_convert(value.as_ref(), target_type) {
            Some(converted) => converted,
            None => value,
        }
    }

    fn try_get_resource_host(&self) -> Option<ResourceHostRef> {
        if let Some(target) = self.base.try_get_target() {
            if let Some(element) = target.cast::<StyledElement>() {
                return Some(ResourceHostRef::Element(element));
            }
            if let Some(application) = target.cast::<Application>() {
                return Some(ResourceHostRef::Other(application.as_resource_host()));
            }
        }

        match &self.anchor {
            Some(HeldAnchor::Element(element)) => element.upgrade().map(ResourceHostRef::Element),
            Some(HeldAnchor::Host(host)) => host.upgrade(),
            _ => None,
        }
    }

    /// The host the resource is looked up from, while it is alive.
    fn host(&self) -> Option<ResourceHostRef> {
        self.host.borrow().as_ref().and_then(WeakResourceHost::upgrade)
    }

    fn set_host(&self, host: Option<ResourceHostRef>) {
        *self.host.borrow_mut() = host.as_ref().map(ResourceHostRef::downgrade);
    }

    fn subscribe(&self, host: Option<&ResourceHostRef>) {
        let Some(host) = host else { return };
        let mut subscriptions = Vec::with_capacity(2);

        let weak = self.this.clone();
        subscriptions.push(host.resources_changed(Rc::new(move |_: &ResourcesChangedEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.publish_value();
            }
        })));

        if !self.override_theme_variant.get() {
            if let Some(theme_variant_host) = host.as_theme_variant_host() {
                *self.theme_variant.borrow_mut() = theme_variant_host.actual_theme_variant();
                let weak = self.this.clone();
                subscriptions.push(theme_variant_host.actual_theme_variant_changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.actual_theme_variant_changed();
                    }
                })));
            }
        }

        *self.host_subscriptions.borrow_mut() = subscriptions;
    }

    fn unsubscribe(&self) {
        let subscriptions = std::mem::take(&mut *self.host_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

impl UntypedBindingExpression for DynamicResourceExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        format!("DynamicResource {}", self.resource_key)
    }

    fn start_core(&self) {
        self.on_start();
    }

    fn stop_core(&self) {
        self.on_stop();
    }
}

ferroui_base::impl_untyped_binding_expression!(DynamicResourceExpression);
