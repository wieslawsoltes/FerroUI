use crate::binding_source::boxed_argument;
use crate::source_observation::SourceObservation;
use crate::{Translation, TranslationBindingSource};
use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::{UntypedObservableBindingExpression, ValueType, ValueTypes};
use ferroui_base::data::{
    BindingBase, BindingError, BindingErrorType, BindingExpressionBase, BindingNotification,
    BindingOperations, BindingPriority, MultiBinding,
};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::reactive::Observable;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, FerroProperty, Ref, WeakRef};
use ferroui_markup_xaml::ServiceProviderExtensions;
use ferroui_protranslate_core::FormatArgument;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// `{pt:Translate Key}`: a native one-way binding that follows culture and service changes.
pub struct TranslateExtension {
    key: RefCell<String>,
    fallback: RefCell<Option<BoxedValue>>,
    string_format: RefCell<Option<String>>,
    source: RefCell<Option<TranslationBindingSource>>,
    priority: Cell<BindingPriority>,
}

impl TranslateExtension {
    pub fn new() -> Rc<Self> {
        Self::with_key("")
    }
    pub fn with_key(key: &str) -> Rc<Self> {
        Rc::new(Self {
            key: RefCell::new(key.to_owned()),
            fallback: RefCell::new(None),
            string_format: RefCell::new(None),
            source: RefCell::new(None),
            priority: Cell::new(BindingPriority::LocalValue),
        })
    }
    pub fn key(&self) -> String {
        self.key.borrow().clone()
    }
    pub fn set_key(&self, value: String) {
        self.key.replace(value);
    }
    pub fn fallback_value(&self) -> Option<BoxedValue> {
        self.fallback.borrow().clone()
    }
    pub fn set_fallback_value(&self, value: Option<BoxedValue>) {
        self.fallback.replace(value);
    }
    /// Convenience setter for a textual fallback. Object fallbacks retain identity.
    pub fn set_fallback_text(&self, value: Option<String>) {
        self.set_fallback_value(value.map(|value| Rc::new(value) as BoxedValue));
    }
    pub fn string_format(&self) -> Option<String> {
        self.string_format.borrow().clone()
    }
    pub fn set_string_format(&self, value: Option<String>) {
        self.string_format.replace(value);
    }
    pub fn set_source(&self, value: Option<TranslationBindingSource>) {
        self.source.replace(value);
    }
    pub fn source(&self) -> Option<TranslationBindingSource> {
        self.source.borrow().clone()
    }
    pub fn provide_value(
        self: &Rc<Self>,
        service_provider: &Rc<dyn IServiceProvider>,
    ) -> Rc<dyn BindingBase> {
        if service_provider.is_in_control_template() {
            self.priority.set(BindingPriority::Template);
        }
        self.clone()
    }
    fn options(&self, target: &FerroObject) -> TranslationOptions {
        TranslationOptions {
            source: self.source.borrow().clone(),
            target: target.to_weak(),
            key: self.key(),
            fallback: self.fallback_value(),
            string_format: self.string_format(),
        }
    }
}

impl PartialEq for TranslateExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[derive(Clone)]
struct TranslationOptions {
    source: Option<TranslationBindingSource>,
    target: WeakRef<FerroObject>,
    key: String,
    fallback: Option<BoxedValue>,
    string_format: Option<String>,
}

impl TranslationOptions {
    fn source(&self) -> TranslationBindingSource {
        self.source
            .clone()
            .or_else(|| {
                self.target
                    .upgrade()
                    .map(|target| Translation::get_source(&target))
            })
            .unwrap_or_else(TranslationBindingSource::default_source)
    }
    fn render(&self, arguments: &[FormatArgument]) -> Result<BoxedValue, BindingError> {
        let service = self.source().service();
        let localized = service.get_string(&self.key).map_err(BindingError::new)?;
        if localized.resource_not_found {
            // FallbackValue is the final binding fallback, not a replacement
            // format template. Attached text retains its separate text semantics.
            return Ok(self
                .fallback
                .clone()
                .unwrap_or_else(FerroProperty::unset_value));
        }
        let culture = service.cultures().current_culture();
        let value = if arguments.is_empty() {
            localized.value
        } else {
            service
                .format_localized(localized, arguments, &culture)
                .map_err(BindingError::new)?
        };
        let value = match self.string_format.as_deref() {
            Some(format) => service
                .format_value(&self.key, format, &[value.into()], &culture)
                .map_err(BindingError::new)?,
            None => value,
        };
        Ok(Rc::new(value))
    }
    fn boxed(
        &self,
        arguments: &[FormatArgument],
        target_type: Option<ValueType>,
    ) -> Option<BoxedValue> {
        let convert = |value: Option<BoxedValue>| {
            if BindingOperations::is_unset(value.as_ref()) {
                return Ok(value);
            }
            match target_type {
                Some(target_type) => ValueTypes::try_convert(value.as_ref(), target_type)
                    .ok_or_else(|| {
                        BindingError::message(format!(
                            "Cannot convert translation '{}' to {}",
                            self.key,
                            target_type.name()
                        ))
                    }),
                None => Ok(value),
            }
        };
        Some(
            match self
                .render(arguments)
                .and_then(|value| convert(Some(value)))
            {
                Ok(value) => return value,
                Err(error) => Rc::new(BindingNotification::with_error_and_fallback(
                    error,
                    BindingErrorType::Error,
                    self.fallback.as_ref().map_or_else(
                        || Some(FerroProperty::unset_value()),
                        |value| {
                            convert(Some(value.clone()))
                                .unwrap_or_else(|_| Some(FerroProperty::unset_value()))
                        },
                    ),
                )),
            },
        )
    }
}

impl BindingBase for TranslateExtension {
    fn create_instance(
        &self,
        target: &FerroObject,
        property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let options = self.options(target);
        let target_type = property.map(|property| {
            ValueType::new(property.property_type(), property.property_type_name())
        });
        let observable = Observable::create(move |observer| {
            let evaluation = options.clone();
            let update = Rc::new(move || observer.on_next(evaluation.boxed(&[], target_type)));
            let observation =
                SourceObservation::start(options.target.clone(), options.source.clone(), update);
            observation.refresh();
            observation
        });
        UntypedObservableBindingExpression::new(observable, self.priority.get())
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// `{pt:Format Key, Value={Binding Amount}}`: combines culture refreshes with
/// changes of the bound argument through FerroUI's native multi-binding engine.
pub struct FormatExtension {
    base: Rc<TranslateExtension>,
    value: RefCell<Option<BoxedValue>>,
}
impl Deref for FormatExtension {
    type Target = TranslateExtension;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl PartialEq for FormatExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl FormatExtension {
    pub fn new() -> Rc<Self> {
        Self::with_key("")
    }
    pub fn with_key(key: &str) -> Rc<Self> {
        Rc::new(Self {
            base: TranslateExtension::with_key(key),
            value: RefCell::new(None),
        })
    }
    pub fn base(&self) -> Rc<TranslateExtension> {
        self.base.clone()
    }
    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }
    pub fn set_value(&self, value: Option<BoxedValue>) {
        self.value.replace(value);
    }
    pub fn provide_value(
        self: &Rc<Self>,
        services: &Rc<dyn IServiceProvider>,
    ) -> Rc<dyn BindingBase> {
        if services.is_in_control_template() {
            self.priority.set(BindingPriority::Template);
        }
        self.clone()
    }
}

impl BindingBase for FormatExtension {
    fn create_instance(
        &self,
        target: &FerroObject,
        property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let Some(value) = self.value() else {
            return self.base.create_instance(target, property, anchor);
        };
        let options = self.options(target);
        let pulse_options = options.clone();
        let pulse = Observable::create(move |observer| {
            // A fresh local revision changes even if a replacement source happens
            // to have the same revision or produces an identical template string.
            let revision = Cell::new(0_u64);
            let update = Rc::new(move || {
                revision.set(revision.get().wrapping_add(1));
                observer.on_next(revision.get());
            });
            let observation = SourceObservation::start(
                pulse_options.target.clone(),
                pulse_options.source.clone(),
                update,
            );
            observation.refresh();
            observation
        });
        let value_binding =
            ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn BindingBase>>())
                .flatten()
                .and_then(|value| value.downcast_ref::<Rc<dyn BindingBase>>().cloned())
                .unwrap_or_else(|| ferroui_base::to_binding(Observable::single_value(Some(value))));
        let fallback = options.fallback.clone();
        let binding = MultiBinding::new()
            .with_bindings(vec![ferroui_base::to_binding(pulse), value_binding])
            .with_converter(Rc::new(FormatConverter(options)))
            .with_priority(self.priority.get());
        if fallback.is_some() {
            binding.set_fallback_value(fallback);
        }
        binding.create_instance(target, property, anchor)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

struct FormatConverter(TranslationOptions);
impl IMultiValueConverter for FormatConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let values: Vec<_> = values
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .and_then(|value| ValueTypes::normalize(value.clone()))
            })
            .collect();
        if values.iter().any(|value| {
            BindingOperations::is_unset(value.as_ref())
                || BindingOperations::is_do_nothing(value.as_ref())
        }) {
            return Ok(Some(FerroProperty::unset_value()));
        }
        let argument = boxed_argument(values.get(1).and_then(Option::as_ref))
            .map_err(BindingError::message)?;
        self.0.render(&[argument]).map(Some)
    }
}

/// Concise alias of [`TranslateExtension`].
pub struct TExtension {
    base: Rc<TranslateExtension>,
}
impl TExtension {
    pub fn new() -> Rc<Self> {
        Self::with_key("")
    }
    pub fn with_key(key: &str) -> Rc<Self> {
        Rc::new(Self {
            base: TranslateExtension::with_key(key),
        })
    }
    pub fn base(&self) -> Rc<TranslateExtension> {
        self.base.clone()
    }
    pub fn provide_value(&self, services: &Rc<dyn IServiceProvider>) -> Rc<dyn BindingBase> {
        self.base.provide_value(services)
    }
}
impl Deref for TExtension {
    type Target = TranslateExtension;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl PartialEq for TExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Concise alias of [`FormatExtension`].
pub struct FExtension {
    base: Rc<FormatExtension>,
}
impl FExtension {
    pub fn new() -> Rc<Self> {
        Self::with_key("")
    }
    pub fn with_key(key: &str) -> Rc<Self> {
        Rc::new(Self {
            base: FormatExtension::with_key(key),
        })
    }
    pub fn base(&self) -> Rc<FormatExtension> {
        self.base.clone()
    }
    pub fn provide_value(&self, services: &Rc<dyn IServiceProvider>) -> Rc<dyn BindingBase> {
        self.base.provide_value(services)
    }
}
impl Deref for FExtension {
    type Target = FormatExtension;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl PartialEq for FExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class TranslateExtension {
    this: Rc<TranslateExtension>,
    handles: [Rc<TranslateExtension>, Option<Rc<TranslateExtension>>],
    constructors: [() => TranslateExtension::new, (String) => |key: String| TranslateExtension::with_key(&key)],
    properties: [
        Key: String { get: TranslateExtension::key, set: TranslateExtension::set_key } [ConstructorArgument("key")],
        Path: String { get: TranslateExtension::key, set: TranslateExtension::set_key },
        FallbackValue: Option<BoxedValue> { get: TranslateExtension::fallback_value, set: TranslateExtension::set_fallback_value },
        StringFormat: Option<String> { get: TranslateExtension::string_format, set: TranslateExtension::set_string_format },
        Source: Option<TranslationBindingSource> { get: TranslateExtension::source, set: TranslateExtension::set_source },
    ],
    methods: [fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<dyn BindingBase> => |this: &Rc<TranslateExtension>, services: Rc<dyn IServiceProvider>| this.provide_value(&services)],
});
ferro_markup_type!(class FormatExtension {
    this: Rc<FormatExtension>, handles: [Rc<FormatExtension>, Option<Rc<FormatExtension>>], base: Rc<TranslateExtension>,
    constructors: [() => FormatExtension::new, (String) => |key: String| FormatExtension::with_key(&key)],
    properties: [Value: Option<BoxedValue> { get: FormatExtension::value, set: FormatExtension::set_value }],
    methods: [fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<dyn BindingBase> => |this: &Rc<FormatExtension>, services: Rc<dyn IServiceProvider>| this.provide_value(&services)],
});
ferro_markup_type!(class TExtension {
    this: Rc<TExtension>, handles: [Rc<TExtension>, Option<Rc<TExtension>>], base: Rc<TranslateExtension>,
    constructors: [() => TExtension::new, (String) => |key: String| TExtension::with_key(&key)],
    methods: [fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<dyn BindingBase> => |this: &Rc<TExtension>, services: Rc<dyn IServiceProvider>| this.provide_value(&services)],
});
ferro_markup_type!(class FExtension {
    this: Rc<FExtension>, handles: [Rc<FExtension>, Option<Rc<FExtension>>], base: Rc<FormatExtension>,
    constructors: [() => FExtension::new, (String) => |key: String| FExtension::with_key(&key)],
    methods: [fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<dyn BindingBase> => |this: &Rc<FExtension>, services: Rc<dyn IServiceProvider>| this.provide_value(&services)],
});
