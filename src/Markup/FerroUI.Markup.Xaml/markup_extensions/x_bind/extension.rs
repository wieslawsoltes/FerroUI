//! The markup extension `{x:Bind}`.

use super::bindings::XBindScope;
use super::{XBinding, XBindings};
use crate::{IRootObjectProvider, ServiceProviderExtensions, XamlLoadError};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingMode, CompiledBindingPath, UpdateSourceTrigger};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `{x:Bind Path, Mode=..., UpdateSourceTrigger=...}`: a binding whose path the compiler resolved
/// against the root of its document, or against the `x:DataType` of the `DataTemplate` it is in.
///
/// Markup writes it as `x:Bind` (the XAML language namespace, as WinUI) or as `XBind` (the
/// framework namespace); the compiler maps the first onto the second. The compiler sets
/// [`data_type`](Self::data_type) for a binding inside a typed `DataTemplate`: the binding then
/// reads the data context of the template's root, which must be of that type; without it the
/// binding reads the root object of its document itself.
///
/// The default mode is `OneTime`, as WinUI's. Providing the value creates an [`XBinding`] for
/// the target and registers it with the [`XBindings`] of its root, which reads it when it is
/// initialized (on `Loading`, see [`XBindings`]) and on every `Update`.
pub struct XBindExtension {
    path: RefCell<Option<CompiledBindingPath>>,
    mode: Cell<BindingMode>,
    update_source_trigger: Cell<UpdateSourceTrigger>,
    data_type: Cell<Option<ValueType>>,
}

impl XBindExtension {
    /// `{x:Bind}`: a binding to the root source itself.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            path: RefCell::new(None),
            mode: Cell::new(BindingMode::OneTime),
            update_source_trigger: Cell::new(UpdateSourceTrigger::Default),
            data_type: Cell::new(None),
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

    /// Checks the mode and the update trigger as WinUI's compiler does
    /// (`BindAssignment.ValidateUpdateSourceTrigger`). The compiler reports the same errors with
    /// the position of the binding; this covers an extension made in code.
    pub fn validate(&self) -> Result<(), XamlLoadError> {
        let mode = self.mode();
        if !matches!(mode, BindingMode::OneTime | BindingMode::OneWay | BindingMode::TwoWay) {
            return Err(XamlLoadError::with_message(format!(
                "x:Bind: the mode {mode:?} is not supported; x:Bind is OneTime, OneWay or TwoWay"
            )));
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
    /// of the document, or the root of the template instance for a binding with a data type.
    /// `name_scope` is the name scope of the place the binding is declared in; a template
    /// instance keeps it (see [`XBindings`]).
    pub fn create_binding(
        &self,
        root: &BoxedValue,
        name_scope: Option<&NameScopeRef>,
    ) -> Result<Rc<XBinding>, XamlLoadError> {
        self.validate()?;
        let scope = match self.data_type() {
            Some(data_type) => XBindScope::Template(data_type),
            None => XBindScope::Root,
        };
        let group = XBindings::for_root_value(root, scope, name_scope)?;
        Ok(XBinding::new(group, self.path(), self.mode(), self.update_source_trigger()))
    }

    /// `ProvideValue`: the binding of the target the extension is provided for. Its root is the
    /// root object of the document (`IRootObjectProvider.RootObject`), or, inside a typed
    /// template, the root of the template instance (`IntermediateRootObject`).
    pub fn provide_value(&self, provider: &Rc<dyn IServiceProvider>) -> Result<Rc<XBinding>, XamlLoadError> {
        let roots = provider
            .get_service_of::<Rc<dyn IRootObjectProvider>>()
            .ok_or_else(|| XamlLoadError::with_message("x:Bind requires IRootObjectProvider"))?;
        let root = match self.data_type() {
            Some(_) => roots.intermediate_root_object(),
            None => roots.root_object(),
        }
        .ok_or_else(|| XamlLoadError::with_message("x:Bind requires the root object of its document"))?;
        let name_scope = provider.get_name_scope().map(NameScopeRef);
        self.create_binding(&root, name_scope.as_ref())
    }
}

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
    ],
    methods: [
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<XBinding> =>
            |this: &This, provider: Rc<dyn IServiceProvider>| this.provide_value(&provider),
    ],
});
