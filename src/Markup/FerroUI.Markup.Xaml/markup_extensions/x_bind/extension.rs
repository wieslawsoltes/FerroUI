//! The markup extension `{x:Bind}`.

use super::binding::XBindValueOptions;
use super::bindings::XBindScope;
use super::{XBinding, XBindings};
use crate::{IRootObjectProvider, ServiceProviderExtensions, XamlLoadError};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingMode, CompiledBindingPath, CompiledBindingPathElementKind, UpdateSourceTrigger};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `{x:Bind Path, Mode=..., UpdateSourceTrigger=..., BindBack=...}`: a binding whose path the
/// compiler resolved against the root of its document, or against the `x:DataType` of the
/// `DataTemplate` it is in.
///
/// Markup writes it as `x:Bind` (the XAML language namespace, as WinUI) or as `XBind` (the
/// framework namespace); the compiler maps the first onto the second. The compiler sets
/// [`data_type`](Self::data_type) for a binding inside a typed `DataTemplate`: the binding then
/// reads the data context of the template's root, which must be of that type; without it the
/// binding reads the root object of its document itself.
///
/// The path has the grammar of WinUI's compiled bindings: properties, attached properties
/// (`path.(Owner.Property)`), static members (`ns:Type.Member`), indexers (`Items[0]`,
/// `Map['key']`), casts (`(ns:Type)path`, `((ns:Type)path).Member`) and a function at its end
/// (`Format(Name, 'text', 3)`, `Model.Format(Name)`, `ns:Type.Format(Name)`), compiled into the
/// elements of a [`CompiledBindingPath`]. A `TwoWay` binding through a function writes back
/// through the method [`bind_back`](Self::bind_back) names, which the compiler resolves into
/// the function element of the path.
///
/// The value of the path goes through a [`converter`](Self::converter) (an `IValueConverter`,
/// with its [`converter_parameter`](Self::converter_parameter) and the culture
/// [`converter_language`](Self::converter_language) names; `ConvertBack` on the way back of a
/// `TwoWay` binding), and [`fallback_value`](Self::fallback_value) and
/// [`target_null_value`](Self::target_null_value) stand for a path that cannot be read (a null
/// in the middle of it, a failure) and for a null value (step X3).
///
/// On an event (`Click="{x:Bind OnClick}"`, `Click="{x:Bind Model.OnClick}"`,
/// `Click="{x:Bind local:Handlers.OnClick}"`) the path ends with the method the event calls,
/// which the compiler resolves into the event handler element of the path; the binding the
/// extension provides subscribes the handler while the bindings of the root are initialized
/// (step X4, [`XBinding::bind_routed_event`]).
///
/// Inside a `ControlTemplate` the compiler sets [`target_type`](Self::target_type), the
/// `TargetType` of the template: the binding reads the templated parent of the root of the
/// template instance, which must be of that type (step X5). Inside a typed `DataTemplate` the
/// compiler sets [`phase`](Self::phase) from the `x:Phase` of the element the binding is on: a
/// container reads the binding in that phase of the instance (`IDataTemplateComponent`).
///
/// The default mode is `OneTime`, as WinUI's; the compiler gives the binding the mode of the
/// nearest `x:DefaultBindMode` around it when the markup names none. Providing the value creates
/// an [`XBinding`] for the target and registers it with the [`XBindings`] of its root, which reads
/// it when it is initialized (on `Loading`, see [`XBindings`]) and on every `Update`.
pub struct XBindExtension {
    path: RefCell<Option<CompiledBindingPath>>,
    mode: Cell<BindingMode>,
    update_source_trigger: Cell<UpdateSourceTrigger>,
    data_type: Cell<Option<ValueType>>,
    target_type: Cell<Option<ValueType>>,
    phase: Cell<i32>,
    bind_back: RefCell<Option<String>>,
    converter: RefCell<Option<Rc<dyn IValueConverter>>>,
    converter_parameter: RefCell<Option<BoxedValue>>,
    converter_language: RefCell<Option<CultureInfo>>,
    fallback_value: RefCell<Option<BoxedValue>>,
    target_null_value: RefCell<Option<BoxedValue>>,
}

impl XBindExtension {
    /// `{x:Bind}`: a binding to the root source itself.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            path: RefCell::new(None),
            mode: Cell::new(BindingMode::OneTime),
            update_source_trigger: Cell::new(UpdateSourceTrigger::Default),
            data_type: Cell::new(None),
            target_type: Cell::new(None),
            phase: Cell::new(0),
            bind_back: RefCell::new(None),
            converter: RefCell::new(None),
            converter_parameter: RefCell::new(None),
            converter_language: RefCell::new(None),
            fallback_value: RefCell::new(Some(FerroProperty::unset_value())),
            target_null_value: RefCell::new(Some(FerroProperty::unset_value())),
        })
    }

    /// `{x:Bind Path}`.
    pub fn with_path(path: CompiledBindingPath) -> Rc<Self> {
        let this = Self::new();
        this.set_path(Some(path));
        this
    }

    /// The compiled path, read from the root source; `None` reads the root source itself.
    pub fn path(&self) -> Option<CompiledBindingPath> {
        self.path.borrow().clone()
    }

    pub fn set_path(&self, path: Option<CompiledBindingPath>) {
        *self.path.borrow_mut() = path;
    }

    /// `OneTime` (the default), `OneWay` or `TwoWay`.
    pub fn mode(&self) -> BindingMode {
        self.mode.get()
    }

    pub fn set_mode(&self, mode: BindingMode) {
        self.mode.set(mode);
    }

    /// When a `TwoWay` binding writes the target value back: `Default` (on `LostFocus` for
    /// `TextBox.Text`, else on every change), `PropertyChanged` or `LostFocus`.
    pub fn update_source_trigger(&self) -> UpdateSourceTrigger {
        self.update_source_trigger.get()
    }

    pub fn set_update_source_trigger(&self, trigger: UpdateSourceTrigger) {
        self.update_source_trigger.set(trigger);
    }

    /// The `x:DataType` of the `DataTemplate` the binding is in, which the compiler sets; `None`
    /// outside a template.
    pub fn data_type(&self) -> Option<ValueType> {
        self.data_type.get()
    }

    pub fn set_data_type(&self, data_type: Option<ValueType>) {
        self.data_type.set(data_type);
    }

    /// The `TargetType` of the `ControlTemplate` the binding is in, which the compiler sets: the
    /// binding then reads the templated parent of the root of the template instance. `None`
    /// outside a control template.
    pub fn target_type(&self) -> Option<ValueType> {
        self.target_type.get()
    }

    pub fn set_target_type(&self, target_type: Option<ValueType>) {
        self.target_type.set(target_type);
    }

    /// The `x:Phase` of the element the binding is on, which the compiler sets: `0` (the
    /// default) is read when a container prepares the template instance, a later phase when
    /// the container processes that phase.
    pub fn phase(&self) -> i32 {
        self.phase.get()
    }

    pub fn set_phase(&self, phase: i32) {
        self.phase.set(phase);
    }

    /// `BindBack`: the method a `TwoWay` binding through a function writes the value of its
    /// target with, as markup names it (`ParseName`, `Model.SetName`, `local:Helpers.Parse`).
    /// The compiler resolves it into the function element of the path; the extension keeps
    /// the text.
    pub fn bind_back(&self) -> Option<String> {
        self.bind_back.borrow().clone()
    }

    pub fn set_bind_back(&self, bind_back: Option<String>) {
        *self.bind_back.borrow_mut() = bind_back;
    }

    /// `Converter`: the `IValueConverter` the value of the path goes through (`Convert` to the
    /// target, `ConvertBack` to the source of a `TwoWay` binding). Markup gives it as a
    /// `{StaticResource}`.
    pub fn converter(&self) -> Option<Rc<dyn IValueConverter>> {
        self.converter.borrow().clone()
    }

    pub fn set_converter(&self, converter: Option<Rc<dyn IValueConverter>>) {
        *self.converter.borrow_mut() = converter;
    }

    /// `ConverterParameter`: the parameter the converter is called with.
    pub fn converter_parameter(&self) -> Option<BoxedValue> {
        self.converter_parameter.borrow().clone()
    }

    pub fn set_converter_parameter(&self, parameter: Option<BoxedValue>) {
        *self.converter_parameter.borrow_mut() = parameter;
    }

    /// `ConverterLanguage`: the culture the converter is called with, which markup names by its
    /// language tag (`fr-FR`); `None` is the current culture.
    pub fn converter_language(&self) -> Option<CultureInfo> {
        self.converter_language.borrow().clone()
    }

    pub fn set_converter_language(&self, language: Option<CultureInfo>) {
        *self.converter_language.borrow_mut() = language;
    }

    /// `FallbackValue`: the value of the target when the path cannot be read (a null in the
    /// middle of it, a failure); the unset marker (the default) for none, `None` for `x:Null`.
    pub fn fallback_value(&self) -> Option<BoxedValue> {
        self.fallback_value.borrow().clone()
    }

    pub fn set_fallback_value(&self, value: Option<BoxedValue>) {
        *self.fallback_value.borrow_mut() = value;
    }

    /// `TargetNullValue`: the value of the target when the value of the path is null; the
    /// unset marker (the default) for none.
    pub fn target_null_value(&self) -> Option<BoxedValue> {
        self.target_null_value.borrow().clone()
    }

    pub fn set_target_null_value(&self, value: Option<BoxedValue>) {
        *self.target_null_value.borrow_mut() = value;
    }

    /// Checks the mode, the update trigger and the converter as WinUI's compiler does
    /// (`BindAssignment.ValidateUpdateSourceTrigger`, `ValidateConverter`). The compiler reports
    /// the same errors with the position of the binding; this covers an extension made in code.
    pub fn validate(&self) -> Result<(), XamlLoadError> {
        let mode = self.mode();
        if !matches!(mode, BindingMode::OneTime | BindingMode::OneWay | BindingMode::TwoWay) {
            return Err(XamlLoadError::with_message(format!(
                "x:Bind: the mode {mode:?} is not supported; x:Bind is OneTime, OneWay or TwoWay"
            )));
        }
        if !(0..MAX_PHASE).contains(&self.phase()) {
            return Err(XamlLoadError::with_message(
                "x:Bind: Phase value must be a positive integer less than 25.",
            ));
        }
        if self.converter().is_none() {
            if self.converter_parameter().is_some() {
                return Err(XamlLoadError::with_message(
                    "x:Bind: ConverterParameter cannot be specified without an actual converter",
                ));
            }
            if self.converter_language().is_some() {
                return Err(XamlLoadError::with_message(
                    "x:Bind: ConverterLanguage cannot be specified without an actual converter",
                ));
            }
        } else {
            let ends_with_function = match self.path() {
                Some(path) => {
                    let last = path.len().checked_sub(1).and_then(|index| path.element(index)).map(|element| element.kind());
                    last == Some(CompiledBindingPathElementKind::Function)
                }
                None => false,
            };
            if ends_with_function {
                return Err(XamlLoadError::with_message(
                    "x:Bind: Using a converter with function binding is not supported",
                ));
            }
        }
        match self.update_source_trigger() {
            UpdateSourceTrigger::Explicit => Err(XamlLoadError::with_message(
                "x:Bind: 'Explicit' is not a supported value for 'UpdateSourceTrigger'",
            )),
            UpdateSourceTrigger::Default => Ok(()),
            _ if mode != BindingMode::TwoWay => Err(XamlLoadError::with_message(
                "x:Bind: 'UpdateSourceTrigger' may only be used with binding 'Mode=TwoWay'",
            )),
            _ => Ok(()),
        }
    }

    /// The binding of one target, registered with the [`XBindings`] of `root`: the root object
    /// of the document, or the root of the template instance for a binding with a data type or
    /// a target type. `name_scope` is the name scope of the place the binding is declared in; a
    /// template instance keeps it (see [`XBindings`]).
    pub fn create_binding(
        &self,
        root: &BoxedValue,
        name_scope: Option<&NameScopeRef>,
    ) -> Result<Rc<XBinding>, XamlLoadError> {
        self.validate()?;
        let scope = match (self.data_type(), self.target_type()) {
            (Some(data_type), _) => XBindScope::Template(data_type),
            (None, Some(target_type)) => XBindScope::ControlTemplate(target_type),
            (None, None) => XBindScope::Root,
        };
        let group = XBindings::for_root_value(root, scope, name_scope)?;
        let options = XBindValueOptions {
            converter: self.converter(),
            converter_parameter: self.converter_parameter(),
            converter_culture: self.converter_language(),
            fallback_value: self.fallback_value(),
            target_null_value: self.target_null_value(),
        };
        Ok(XBinding::new(group, self.path(), self.mode(), self.update_source_trigger(), self.phase(), options))
    }

    /// `ProvideValue`: the binding of the target the extension is provided for. Its root is the
    /// root object of the document (`IRootObjectProvider.RootObject`), or, inside a typed data
    /// template or a control template, the root of the template instance
    /// (`IntermediateRootObject`).
    pub fn provide_value(&self, provider: &Rc<dyn IServiceProvider>) -> Result<Rc<XBinding>, XamlLoadError> {
        let roots = provider
            .get_service_of::<Rc<dyn IRootObjectProvider>>()
            .ok_or_else(|| XamlLoadError::with_message("x:Bind requires IRootObjectProvider"))?;
        let root = match (self.data_type(), self.target_type()) {
            (None, None) => roots.root_object(),
            _ => roots.intermediate_root_object(),
        }
        .ok_or_else(|| XamlLoadError::with_message("x:Bind requires the root object of its document"))?;
        let name_scope = provider.get_name_scope().map(NameScopeRef);
        self.create_binding(&root, name_scope.as_ref())
    }
}

/// The phases of `x:Phase`: `0` to `24`, as WinUI's compiler accepts them
/// (`XamlCompiler_InvalidValueForPhase`).
pub(crate) const MAX_PHASE: i32 = 25;

crate::identity_eq!(XBindExtension);

type This = Rc<XBindExtension>;

ferro_markup_type!(class XBindExtension {
    this: Rc<XBindExtension>,
    handles: [Rc<XBindExtension>, Option<Rc<XBindExtension>>],
    constructors: [
        () => XBindExtension::new,
        (CompiledBindingPath) => XBindExtension::with_path,
    ],
    properties: [
        Path: Option<CompiledBindingPath> {
            get: |this: &This| this.path(),
            set: |this: &This, value: Option<CompiledBindingPath>| this.set_path(value)
        },
        Mode: BindingMode {
            get: |this: &This| this.mode(),
            set: |this: &This, value: BindingMode| this.set_mode(value)
        },
        UpdateSourceTrigger: UpdateSourceTrigger {
            get: |this: &This| this.update_source_trigger(),
            set: |this: &This, value: UpdateSourceTrigger| this.set_update_source_trigger(value)
        },
        DataType: Option<ValueType> {
            get: |this: &This| this.data_type(),
            set: |this: &This, value: Option<ValueType>| this.set_data_type(value)
        },
        TargetType: Option<ValueType> {
            get: |this: &This| this.target_type(),
            set: |this: &This, value: Option<ValueType>| this.set_target_type(value)
        },
        Phase: i32 {
            get: |this: &This| this.phase(),
            set: |this: &This, value: i32| this.set_phase(value)
        },
        BindBack: Option<String> {
            get: |this: &This| this.bind_back(),
            set: |this: &This, value: Option<String>| this.set_bind_back(value)
        },
        Converter: Option<Rc<dyn IValueConverter>> {
            get: |this: &This| this.converter(),
            set: |this: &This, value: Option<Rc<dyn IValueConverter>>| this.set_converter(value)
        },
        ConverterParameter: Option<BoxedValue> {
            get: |this: &This| this.converter_parameter(),
            set: |this: &This, value: Option<BoxedValue>| this.set_converter_parameter(value)
        },
        ConverterLanguage: Option<CultureInfo> {
            get: |this: &This| this.converter_language(),
            set: |this: &This, value: Option<CultureInfo>| this.set_converter_language(value)
        },
        FallbackValue: Option<BoxedValue> {
            get: |this: &This| this.fallback_value(),
            set: |this: &This, value: Option<BoxedValue>| this.set_fallback_value(value)
        },
        TargetNullValue: Option<BoxedValue> {
            get: |this: &This| this.target_null_value(),
            set: |this: &This, value: Option<BoxedValue>| this.set_target_null_value(value)
        },
    ],
    methods: [
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<XBinding> =>
            |this: &This, provider: Rc<dyn IServiceProvider>| this.provide_value(&provider),
    ],
});
