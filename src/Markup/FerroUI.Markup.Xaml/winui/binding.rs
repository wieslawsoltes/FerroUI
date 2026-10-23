//! A dedicated x:Bind adapter. No ordinary Binding or CompiledBinding defaults are changed.

use super::path_node::{PathNode, RootGate};
use super::WinUIBindings;
use crate::{IRootObjectProvider, ServiceProviderExtensions, XamlLoadException};
use ferroui_base::controls::{INameScope, NameScopeRef};
use ferroui_base::data::core::expression_nodes::ExpressionNode;
use ferroui_base::data::core::{
    BindingExpression, BindingExpressionOptions, TargetTypeConverter, ValueType, ValueTypes,
};
use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingMode, UpdateSourceTrigger};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{
    ferro_markup_enum, ferro_markup_type, FerroObject, FerroProperty, Ref, StyledElement,
};
use ferroui_controls::TextBox;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The source scope captured by the explicit WinUI migration pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum WinUIBindingScope {
    /// The declaring document root, with named-element fields resolved in its name scope.
    #[default]
    Root = 0,
    /// The typed DataContext of this template instance's root.
    Template = 1,
}

/// Markup syntax for the supported property-path subset of WinUI x:Bind.
///
/// It creates a dormant binding. Initialize the root's [`WinUIBindings`] after code-behind
/// construction. Ordinary bindings keep their own source and lifecycle. This extension
/// evaluates registered Rust property/model metadata; it does not generate C#/C++ code and
/// does not support function calls, event handlers, casts, indexers or phased loading.
pub struct WinUIBindExtension {
    path: RefCell<String>,
    mode: Cell<BindingMode>,
    scope: Cell<WinUIBindingScope>,
    data_type: Cell<Option<ValueType>>,
    update_source_trigger: Cell<UpdateSourceTrigger>,
    source_line: Cell<i32>,
    source_column: Cell<i32>,
}

impl WinUIBindExtension {
    pub fn new() -> Rc<Self> {
        Self::with_path("")
    }

    pub fn with_path(path: &str) -> Rc<Self> {
        Rc::new(Self {
            path: RefCell::new(path.into()),
            mode: Cell::new(BindingMode::OneTime),
            scope: Cell::new(WinUIBindingScope::Root),
            data_type: Cell::new(None),
            update_source_trigger: Cell::new(UpdateSourceTrigger::Default),
            source_line: Cell::new(0),
            source_column: Cell::new(0),
        })
    }

    pub fn path(&self) -> String {
        self.path.borrow().clone()
    }
    pub fn set_path(&self, path: String) {
        *self.path.borrow_mut() = path;
    }
    pub fn mode(&self) -> BindingMode {
        self.mode.get()
    }
    pub fn set_mode(&self, mode: BindingMode) {
        self.mode.set(mode);
    }
    pub fn scope(&self) -> WinUIBindingScope {
        self.scope.get()
    }
    pub fn set_scope(&self, scope: WinUIBindingScope) {
        self.scope.set(scope);
    }
    pub fn data_type(&self) -> Option<ValueType> {
        self.data_type.get()
    }
    pub fn set_data_type(&self, data_type: Option<ValueType>) {
        self.data_type.set(data_type);
    }
    pub fn update_source_trigger(&self) -> UpdateSourceTrigger {
        self.update_source_trigger.get()
    }
    pub fn set_update_source_trigger(&self, trigger: UpdateSourceTrigger) {
        self.update_source_trigger.set(trigger);
    }
    pub fn source_line(&self) -> i32 {
        self.source_line.get()
    }
    pub fn set_source_line(&self, value: i32) {
        self.source_line.set(value);
    }
    pub fn source_column(&self) -> i32 {
        self.source_column.get()
    }
    pub fn set_source_column(&self, value: i32) {
        self.source_column.set(value);
    }

    /// Captures one document/template instance's context. The returned binding and its
    /// expressions hold the root weakly; fields remain owned by the declaring root/item.
    pub fn create_binding(
        &self,
        root: &FerroObject,
        name_scope: Option<&NameScopeRef>,
    ) -> Result<Rc<WinUIBinding>, XamlLoadException> {
        let path = self.path();
        validate_path(&path)?;
        if !matches!(
            self.mode(),
            BindingMode::OneTime | BindingMode::OneWay | BindingMode::TwoWay
        ) {
            return Err(XamlLoadException::with_message(
                "WinUIBind Mode must be OneTime, OneWay or TwoWay",
            ));
        }
        if self.mode() == BindingMode::TwoWay && path.is_empty() {
            return Err(XamlLoadException::with_message(
                "A TwoWay x:Bind needs a writable property path",
            ));
        }
        if self.update_source_trigger() == UpdateSourceTrigger::Explicit
            || (self.update_source_trigger() != UpdateSourceTrigger::Default
                && self.mode() != BindingMode::TwoWay)
        {
            return Err(XamlLoadException::with_message(
                "WinUIBind UpdateSourceTrigger accepts PropertyChanged or LostFocus only with TwoWay"));
        }
        if self.scope() == WinUIBindingScope::Template {
            if self.data_type().is_none() || root.to_ref().cast::<StyledElement>().is_none() {
                return Err(XamlLoadException::with_message(
                    "A template x:Bind needs a StyledElement root and a declared x:DataType",
                ));
            }
        }
        Ok(Rc::new(WinUIBinding {
            group: WinUIBindings::for_root(root),
            path,
            mode: self.mode(),
            scope: self.scope(),
            data_type: self.data_type(),
            trigger: self.update_source_trigger(),
            name_scope: name_scope.map(|scope| Rc::downgrade(&scope.0)),
            line: self.source_line(),
            column: self.source_column(),
        }))
    }

    pub fn provide_value(
        &self,
        provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Rc<WinUIBinding>, XamlLoadException> {
        let root_provider = provider
            .get_service_of::<Rc<dyn IRootObjectProvider>>()
            .ok_or_else(|| {
                XamlLoadException::with_message("WinUIBind requires IRootObjectProvider")
            })?;
        let root = match self.scope() {
            WinUIBindingScope::Root => root_provider.root_object(),
            WinUIBindingScope::Template => root_provider.intermediate_root_object(),
        }
        .and_then(|root| ValueTypes::as_object(&*root))
        .ok_or_else(|| {
            XamlLoadException::with_message("WinUIBind requires a declaring object root")
        })?;
        let names = provider.get_name_scope().map(NameScopeRef);
        self.create_binding(&root, names.as_ref())
    }
}

/// An immutable WinUI binding description captured for one root or template instance.
/// Its actual source is selected only when the owning group is initialized.
pub struct WinUIBinding {
    group: Rc<WinUIBindings>,
    path: String,
    mode: BindingMode,
    scope: WinUIBindingScope,
    data_type: Option<ValueType>,
    trigger: UpdateSourceTrigger,
    name_scope: Option<Weak<dyn INameScope>>,
    line: i32,
    column: i32,
}

impl WinUIBinding {
    pub fn try_create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
    ) -> Result<Rc<BindingExpression>, XamlLoadException> {
        let target_property = target_property.ok_or_else(|| XamlLoadException::with_message(
            "The WinUI x:Bind property-path adapter requires a registered target property; event bindings are unsupported"))?;
        let trigger = match self.trigger {
            // BindAssignment.NeedsLostFocusForTwoWay in the pinned WinUI compiler.
            UpdateSourceTrigger::Default
                if target.to_ref().cast::<TextBox>().is_some()
                    && target_property.id() == TextBox::text_property().id() =>
            {
                UpdateSourceTrigger::LostFocus
            }
            UpdateSourceTrigger::Default => UpdateSourceTrigger::PropertyChanged,
            trigger => trigger,
        };
        let gate = RootGate::new(
            &self.group,
            self.scope,
            self.data_type,
            self.path.clone(),
            self.line,
            self.column,
        );
        let segments: Vec<_> = if self.path.is_empty() {
            Vec::new()
        } else {
            self.path.split('.').collect()
        };
        let mut nodes: Vec<Rc<dyn ExpressionNode>> = vec![gate.clone()];
        for (index, segment) in segments.iter().enumerate() {
            nodes.push(PathNode::new(
                &gate,
                segment,
                self.mode,
                (index == 0).then_some(self.scope),
                self.name_scope.clone(),
                index + 1 == segments.len(),
            ));
        }
        gate.set_nodes(&nodes[1..]);
        let expression = BindingExpression::new(
            Some(Rc::new(target.to_ref())),
            nodes,
            BindingExpressionOptions {
                mode: self.mode,
                update_source_trigger: trigger,
                target_property: Some(target_property),
                target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
                ..Default::default()
            },
        );
        self.group
            .register(&gate, self.scope == WinUIBindingScope::Template);
        Ok(expression)
    }
}

impl BindingBase for WinUIBinding {
    fn create_instance(
        &self,
        target: &FerroObject,
        property: Option<&'static FerroProperty>,
        _: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let expression = crate::throw(self.try_create_instance(target, property));
        expression
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// This strict grammar intentionally excludes operators, functions, casts and indexers.
pub(super) fn validate_path(path: &str) -> Result<(), XamlLoadException> {
    if path.is_empty()
        || path.split('.').all(|segment| {
            let mut chars = segment.chars();
            chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
                && chars.all(|c| c == '_' || c.is_alphanumeric())
        })
    {
        Ok(())
    } else {
        Err(XamlLoadException::with_message(format!(
            "Unsupported x:Bind path '{path}'; only dotted property/field names are implemented"
        )))
    }
}

crate::identity_eq!(WinUIBindExtension, WinUIBinding);

ferro_markup_enum!(WinUIBindingScope { Root, Template });

ferro_markup_type!(class WinUIBindExtension {
    namespace: "FerroUI.Markup.Xaml.WinUI",
    this: Rc<WinUIBindExtension>,
    handles: [Rc<WinUIBindExtension>, Option<Rc<WinUIBindExtension>>],
    constructors: [
        () => WinUIBindExtension::new,
        (String) => |path: String| WinUIBindExtension::with_path(&path),
    ],
    properties: [
        Path: String { get: WinUIBindExtension::path, set: WinUIBindExtension::set_path },
        Mode: BindingMode { get: WinUIBindExtension::mode, set: WinUIBindExtension::set_mode },
        Scope: WinUIBindingScope { get: WinUIBindExtension::scope, set: WinUIBindExtension::set_scope },
        DataType: Option<ValueType> { get: WinUIBindExtension::data_type, set: WinUIBindExtension::set_data_type },
        UpdateSourceTrigger: UpdateSourceTrigger { get: WinUIBindExtension::update_source_trigger, set: WinUIBindExtension::set_update_source_trigger },
        SourceLine: i32 { get: WinUIBindExtension::source_line, set: WinUIBindExtension::set_source_line },
        SourceColumn: i32 { get: WinUIBindExtension::source_column, set: WinUIBindExtension::set_source_column },
    ],
    methods: [
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<WinUIBinding> =>
            |this: &Rc<WinUIBindExtension>, provider: Rc<dyn IServiceProvider>| this.provide_value(&provider),
    ],
});

ferro_markup_type!(class WinUIBinding {
    namespace: "FerroUI.Markup.Xaml.WinUI",
    this: Rc<WinUIBinding>,
    handles: [Rc<WinUIBinding>, Option<Rc<WinUIBinding>>],
    base: Rc<dyn BindingBase>,
});
