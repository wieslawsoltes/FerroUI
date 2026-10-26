use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::Event;
use ferroui_base::reactive::{IDisposable, IObservable, Observable};
use ferroui_base::BoxedValue;
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
}

impl PartialEq for TranslationBindingSource {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl TranslationBindingSource {
    pub fn new(service: TranslationService) -> Self {
        let source = Self(Rc::new(SourceInner {
            service: RefCell::new(service.clone()),
            subscription: RefCell::new(None),
            changed: Event::new(),
        }));
        source.listen(&service);
        source
    }

    fn listen(&self, service: &TranslationService) {
        let weak = Rc::downgrade(&self.0);
        let subscription = service.subscribe(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.changed.raise(&());
            }
        });
        self.0.subscription.replace(Some(subscription));
    }

    pub fn service(&self) -> TranslationService {
        self.0.service.borrow().clone()
    }

    pub fn set_service(&self, service: TranslationService) {
        self.0.subscription.borrow_mut().take();
        self.0.service.replace(service.clone());
        self.listen(&service);
        self.refresh();
    }

    pub fn culture(&self) -> Culture {
        self.service().cultures().current_culture()
    }

    pub fn set_culture(&self, culture: Culture) -> bool {
        self.service().cultures().set_culture(culture)
    }

    /// Explicitly re-evaluates consumers after application-owned catalog updates.
    pub fn refresh(&self) {
        self.service().refresh();
    }

    /// Number of live native consumers, useful for lifecycle diagnostics.
    pub fn subscriber_count(&self) -> usize {
        self.0.changed.handler_count()
    }

    /// Observes refreshes. A subscription never retains its source.
    pub fn subscribe(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
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
    /// empty invariant catalog; setting its service refreshes existing consumers.
    pub fn default_source() -> Self {
        ferroui_base::ferro_thread_local! {
            static SOURCE: TranslationBindingSource = TranslationBindingSource::new(
                TranslationService::new(
                    Rc::new(InMemoryTranslationProvider::new("Default")),
                    CultureService::new(Culture::invariant()),
                )
            );
        }
        SOURCE.with(Clone::clone)
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
    if let Some(v) = value.downcast_ref::<String>() {
        return Ok(v.clone().into());
    }
    if let Some(v) = value.downcast_ref::<bool>() {
        return Ok((*v).into());
    }
    macro_rules! signed { ($($t:ty),*) => { $(if let Some(v) = value.downcast_ref::<$t>() { return Ok(FormatArgument::Integer(*v as i64)); })* }; }
    macro_rules! unsigned { ($($t:ty),*) => { $(if let Some(v) = value.downcast_ref::<$t>() { return Ok(FormatArgument::Unsigned(*v as u64)); })* }; }
    signed!(i8, i16, i32, i64, isize);
    unsigned!(u8, u16, u32, u64, usize);
    if let Some(v) = value.downcast_ref::<f64>() {
        return Ok((*v).into());
    }
    if let Some(v) = value.downcast_ref::<f32>() {
        return Ok(FormatArgument::Number(*v as f64));
    }
    Err(
        "ProTranslate formatting requires text, a number, a boolean, an offset date/time, null or FormatArgument"
            .to_owned(),
    )
}

ferroui_base::ferro_markup_type!(class TranslationBindingSource {
    handles: [TranslationBindingSource, Option<TranslationBindingSource>],
    properties: [
        Culture: String { get: |source: &TranslationBindingSource| source.culture().name().to_owned() },
    ],
    methods: [fn Refresh() => TranslationBindingSource::refresh],
});
