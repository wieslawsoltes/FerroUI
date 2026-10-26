use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, Observable};
use ferroui_base::{BoxedValue, FerroProperty};
use ferroui_protranslate_core::{
    Culture, CultureService, FormatArgument, InMemoryTranslationProvider, Subscription,
    TranslationError, TranslationService,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A replaceable translation service shared by native bindings and attached targets.
///
/// Cloning shares the source, while [`set_service`](Self::set_service) atomically
/// replaces its service and refreshes existing bindings. The source owns no UI
/// target, and subscription handles unsubscribe both on disposal and on drop.
#[derive(Clone)]
pub struct TranslationBindingSource(pub(crate) Rc<SourceInner>);

pub(crate) struct SourceInner {
    service: RefCell<TranslationService>,
    subscription: RefCell<Option<Subscription>>,
    changed: Event<()>,
    property_changed: Event<str>,
    disposed: Cell<bool>,
}

impl PartialEq for TranslationBindingSource {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for TranslationBindingSource {}

impl INotifyPropertyChanged for TranslationBindingSource {
    fn property_changed(&self) -> &Event<str> {
        &self.0.property_changed
    }
}

impl TranslationBindingSource {
    pub fn new(service: TranslationService) -> Self {
        let source = Self(Rc::new(SourceInner {
            service: RefCell::new(service.clone()),
            subscription: RefCell::new(None),
            changed: Event::new(),
            property_changed: Event::new(),
            disposed: Cell::new(false),
        }));
        source.listen(&service);
        source
    }

    fn listen(&self, service: &TranslationService) {
        let weak = Rc::downgrade(&self.0);
        let subscription = service.subscribe(move |_| {
            if let Some(inner) = weak.upgrade() {
                if !inner.disposed.get() {
                    // The native all-properties contract covers both typed getters
                    // and metadata indexers, with no borrowed service during callbacks.
                    inner.property_changed.raise("");
                    inner.changed.raise(&());
                }
            }
        });
        self.0.subscription.replace(Some(subscription));
    }

    pub fn service(&self) -> TranslationService {
        self.0.service.borrow().clone()
    }

    pub fn set_service(&self, service: TranslationService) {
        assert!(!self.is_disposed(), "TranslationBindingSource is disposed");
        self.0.subscription.borrow_mut().take();
        self.0.service.replace(service.clone());
        self.listen(&service);
        self.refresh();
    }

    /// Culture used to select resources and resolve text direction.
    pub fn culture(&self) -> Culture {
        self.service().cultures().current_ui_culture()
    }

    /// Culture used for numeric, date and measurement formatting.
    pub fn formatting_culture(&self) -> Culture {
        self.service().cultures().current_culture()
    }

    pub fn set_culture(&self, culture: Culture) -> bool {
        self.service().cultures().set_culture(culture)
    }

    pub fn set_cultures(&self, culture: Culture, ui_culture: Culture) -> bool {
        self.service().cultures().set_cultures(culture, ui_culture)
    }

    /// Direct translation preserves the core missing-key and formatting policies.
    pub fn translate(
        &self,
        key: &str,
        arguments: &[FormatArgument],
    ) -> Result<String, TranslationError> {
        let service = self.service();
        if arguments.is_empty() {
            service.get_string(key).map(|localized| localized.value)
        } else {
            service.format(key, arguments)
        }
    }

    /// The native binding indexer yields UnsetValue for a missing resource so the
    /// binding engine can apply its fallback or the target property's default.
    pub fn get_value(&self, key: &str) -> Result<Option<BoxedValue>, TranslationError> {
        let localized = self.service().get_string(key)?;
        Ok(Some(if localized.resource_not_found {
            FerroProperty::unset_value()
        } else {
            Rc::new(localized.value)
        }))
    }

    pub fn is_disposed(&self) -> bool {
        self.0.disposed.get()
    }

    /// Disconnects this source from its service. Dropping its final clone also
    /// releases the service subscription without requiring explicit disposal.
    pub fn dispose(&self) {
        IDisposable::dispose(self);
    }

    /// Explicitly re-evaluates consumers after application-owned catalog updates.
    pub fn refresh(&self) {
        self.service().refresh();
    }

    /// Number of live native consumers, useful for lifecycle diagnostics.
    pub fn subscriber_count(&self) -> usize {
        self.0.changed.handler_count() + self.0.property_changed.handler_count()
    }

    /// Observes refreshes. A subscription never retains its source.
    pub fn subscribe(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        if self.is_disposed() {
            return Disposable::empty();
        }
        let token = self.0.changed.add(Rc::new(move |_| handler()));
        Rc::new(SourceSubscription {
            source: Rc::downgrade(&self.0),
            token: Cell::new(Some(token)),
        })
    }

    /// A native observable suitable for FerroUI's typed property bindings.
    pub fn observe(
        &self,
        key: impl Into<String>,
        arguments: Vec<FormatArgument>,
    ) -> Rc<dyn IObservable<String>> {
        let source = self.clone();
        let key = key.into();
        Observable::create(move |observer| {
            let subscription = Rc::new_cyclic(|weak: &Weak<ObservedTranslation>| {
                let weak = weak.clone();
                let update_source = source.clone();
                let key = key.clone();
                let arguments = arguments.clone();
                ObservedTranslation {
                    subscription: RefCell::new(None),
                    disposed: Cell::new(false),
                    update: Rc::new(move || {
                        let Some(this) = weak.upgrade() else { return };
                        if this.disposed.get() {
                            return;
                        }
                        match update_source.render(&key, &arguments, None, None, None) {
                            Ok(value) => observer.on_next(value),
                            Err(error) => {
                                // IObservable errors terminate the subscription; remove
                                // the handler before invoking reentrant consumer code.
                                this.dispose();
                                observer.on_error(Rc::new(error));
                            }
                        }
                    }),
                }
            });
            // Register before the first delivery, so a reentrant culture change is observed.
            let weak = Rc::downgrade(&subscription);
            subscription
                .subscription
                .replace(Some(source.subscribe(move || {
                    if let Some(this) = weak.upgrade() {
                        (this.update)();
                    }
                })));
            (subscription.update)();
            subscription
        })
    }

    pub(crate) fn render(
        &self,
        key: &str,
        arguments: &[FormatArgument],
        culture: Option<&Culture>,
        fallback: Option<&BoxedValue>,
        string_format: Option<&str>,
    ) -> Result<String, TranslationError> {
        self.render_value(key, arguments, culture, fallback, string_format)
            .map(|value| ValueTypes::to_display_string(Some(&value)))
    }

    pub(crate) fn render_value(
        &self,
        key: &str,
        arguments: &[FormatArgument],
        culture: Option<&Culture>,
        fallback: Option<&BoxedValue>,
        string_format: Option<&str>,
    ) -> Result<BoxedValue, TranslationError> {
        let service = self.service();
        let localized = match culture {
            Some(culture) => service.get_string_for(key, culture)?,
            None => service.get_string(key)?,
        };
        let culture = culture
            .cloned()
            .unwrap_or_else(|| service.cultures().current_culture());
        let mut value: BoxedValue = if localized.resource_not_found {
            fallback
                .cloned()
                .unwrap_or_else(|| Rc::new(localized.value))
        } else {
            Rc::new(localized.value)
        };
        if !arguments.is_empty() {
            let template = ValueTypes::to_display_string(Some(&value));
            value = Rc::new(service.format_value(key, &template, arguments, &culture)?);
        }
        if let Some(format) = string_format {
            let argument = boxed_argument(Some(&value))
                .unwrap_or_else(|_| ValueTypes::to_display_string(Some(&value)).into());
            value = Rc::new(service.format_value(key, format, &[argument], &culture)?);
        }
        Ok(value)
    }

    /// The explicitly configured, UI-thread-local default source. Initially an
    /// empty catalog using the framework's current UI culture.
    pub fn default_source() -> Self {
        default_context().source.borrow().clone()
    }

    /// Replaces the ambient source and reconnects existing unscoped consumers.
    /// Independently scoped users of the old source keep their own source alive.
    pub fn use_default_source(source: Self) {
        assert!(
            !source.is_disposed(),
            "TranslationBindingSource is disposed"
        );
        let context = default_context();
        let unchanged = *context.source.borrow() == source;
        if unchanged {
            source.refresh();
            return;
        }
        let previous = context.source.replace(source.clone());
        // No RefCell borrow is retained while arbitrary binding callbacks run.
        source.refresh();
        context.changed.raise(&());
        drop(previous);
    }

    pub(crate) fn subscribe_default_changed(handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let context = default_context();
        let token = context.changed.add(Rc::new(move |_| handler()));
        Rc::new(DefaultSourceSubscription {
            context: Rc::downgrade(&context),
            token: Cell::new(Some(token)),
        })
    }
}

struct DefaultSourceContext {
    source: RefCell<TranslationBindingSource>,
    changed: Event<()>,
}

fn default_context() -> Rc<DefaultSourceContext> {
    ferroui_base::ferro_thread_local! {
        static CONTEXT: Rc<DefaultSourceContext> = Rc::new(DefaultSourceContext {
            source: RefCell::new(TranslationBindingSource::new(TranslationService::new(
                Rc::new(InMemoryTranslationProvider::new("Default")),
                CultureService::new(Culture::parse(ferroui_base::utilities::CultureInfo::current_ui_culture().name()).unwrap_or_else(|_| Culture::invariant())),
            ))),
            changed: Event::new(),
        });
    }
    CONTEXT.with(Rc::clone)
}

struct DefaultSourceSubscription {
    context: Weak<DefaultSourceContext>,
    token: Cell<Option<u64>>,
}

impl IDisposable for DefaultSourceSubscription {
    fn dispose(&self) {
        if let (Some(context), Some(token)) = (self.context.upgrade(), self.token.take()) {
            context.changed.remove(token);
        }
    }
}

impl Drop for DefaultSourceSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl IDisposable for TranslationBindingSource {
    fn dispose(&self) {
        if self.0.disposed.replace(true) {
            return;
        }
        self.0.subscription.borrow_mut().take();
        for (token, _) in self.0.changed.snapshot().iter() {
            self.0.changed.remove(*token);
        }
        for (token, _) in self.0.property_changed.snapshot().iter() {
            self.0.property_changed.remove(*token);
        }
    }
}

struct SourceSubscription {
    source: Weak<SourceInner>,
    token: Cell<Option<u64>>,
}

impl IDisposable for SourceSubscription {
    fn dispose(&self) {
        if let (Some(source), Some(token)) = (self.source.upgrade(), self.token.take()) {
            source.changed.remove(token);
        }
    }
}

impl Drop for SourceSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}

struct ObservedTranslation {
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    update: Rc<dyn Fn()>,
    disposed: Cell<bool>,
}

impl IDisposable for ObservedTranslation {
    fn dispose(&self) {
        if !self.disposed.replace(true) {
            let subscription = self.subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }
}

impl Drop for ObservedTranslation {
    fn drop(&mut self) {
        self.dispose();
    }
}

pub(crate) fn boxed_argument(value: Option<&BoxedValue>) -> Result<FormatArgument, String> {
    let Some(value) = value else {
        return Ok(FormatArgument::Null);
    };
    if let Some(v) = value.downcast_ref::<FormatArgument>() {
        return Ok(v.clone());
    }
    if let Some(value) = value.downcast_ref::<rust_decimal::Decimal>() {
        return FormatArgument::decimal(&value.to_string()).map_err(|error| error.to_string());
    }
    if let Some(value) = value.downcast_ref::<ferroui_protranslate_core::LocalizedDateTime>() {
        return Ok(FormatArgument::DateTime(*value));
    }
    if let Some(value) = value.downcast_ref::<time::OffsetDateTime>() {
        return Ok((*value).into());
    }
    if let Some(value) = value.downcast_ref::<ferroui_protranslate_core::LocalizedNaiveDateTime>() {
        return Ok((*value).into());
    }
    if let Some(value) = value.downcast_ref::<time::PrimitiveDateTime>() {
        return Ok((*value).into());
    }
    if let Some(v) = value.downcast_ref::<String>() {
        return Ok(v.clone().into());
    }
    if let Some(v) = value.downcast_ref::<bool>() {
        return Ok((*v).into());
    }
    macro_rules! signed { ($($t:ty),*) => { $(if let Some(v) = value.downcast_ref::<$t>() { return Ok(FormatArgument::from(*v)); })* }; }
    macro_rules! unsigned { ($($t:ty),*) => { $(if let Some(v) = value.downcast_ref::<$t>() { return Ok(FormatArgument::from(*v)); })* }; }
    signed!(i8, i16, i32, i64, isize);
    unsigned!(u8, u16, u32, u64, usize);
    if let Some(v) = value.downcast_ref::<f64>() {
        return Ok((*v).into());
    }
    if let Some(v) = value.downcast_ref::<f32>() {
        return Ok(FormatArgument::from(*v));
    }
    // Objects without a dedicated format representation retain native display
    // conversion, matching the general object argument behavior of string formats.
    Ok(FormatArgument::String(ValueTypes::to_display_string(Some(
        value,
    ))))
}

ferroui_base::ferro_markup_type!(class TranslationBindingSource {
    handles: [TranslationBindingSource, Option<TranslationBindingSource>],
    properties: [
        Culture: String {
            get: |source: &TranslationBindingSource| source.culture().name().to_owned(),
            try_set: |source: &TranslationBindingSource, name: String| Culture::parse(&name).map(|culture| { source.set_culture(culture); })
        },
        FormattingCulture: String { get: |source: &TranslationBindingSource| source.formatting_culture().name().to_owned() },
    ],
    indexers: [(String) -> Option<BoxedValue> { try_get: |source: &TranslationBindingSource, key: String| source.get_value(&key) }],
    methods: [
        fn Refresh() => TranslationBindingSource::refresh,
        try fn Translate(String) -> String => |source: &TranslationBindingSource, key: String| source.translate(&key, &[]),
    ],
    notify_property_changed: TranslationBindingSource,
});
