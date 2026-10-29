//! Explicit ownership of literal, static-resource and binding assignments on preserved roots.
use crate::{PreparedReload, ReloadError};
use ferroui_base::{
    data::BindingPriority, diagnostics::FerroObjectDiagnosticExtensions, AnyValue, FerroObject,
    PropertyValue, Ref, StyledProperty,
};
use ferroui_markup_xaml_loader::binding_capture::BindingCaptureScope;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(crate) trait OwnedRootProperty {
    fn name(&self) -> &str;
    fn binding_property(&self) -> Option<&'static ferroui_base::FerroProperty> {
        None
    }
    fn prepare(
        &self,
        root: &Ref<FerroObject>,
        candidate: &Ref<FerroObject>,
        before: &str,
        after: &str,
        bindings: &BindingCaptureScope,
    ) -> Result<Option<Box<dyn PreparedReload>>, ReloadError>;
}
struct Property<T: PropertyValue> {
    property: &'static StyledProperty<T>,
    static_resources: bool,
    expected: Rc<RefCell<Result<Option<T>, ReloadError>>>,
    version: Rc<Cell<u64>>,
}
fn snapshot<T: PropertyValue>(
    root: &FerroObject,
    property: &'static StyledProperty<T>,
) -> Result<Option<T>, ReloadError> {
    let diagnostic = root.get_diagnostic(property);
    if root.has_local_value_binding(property)
        || diagnostic.priority() < BindingPriority::LocalValue
        || diagnostic.is_overridden_current_value()
    {
        return Err(ReloadError::RestartRequired(format!(
            "Root {} has a binding, animation or current-value override",
            property.name()
        )));
    }
    Ok((diagnostic.priority() == BindingPriority::LocalValue).then(|| root.get_value(property)))
}
fn equal<T: PropertyValue>(a: &Option<T>, b: &Option<T>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => (a as &dyn AnyValue).value_eq(b),
        (None, None) => true,
        _ => false,
    }
}
pub(crate) fn own<T: PropertyValue>(
    root: &FerroObject,
    property: &'static StyledProperty<T>,
) -> Box<dyn OwnedRootProperty> {
    own_with_policy(root, property, false)
}
pub(crate) fn own_static_resource<T: PropertyValue>(
    root: &FerroObject,
    property: &'static StyledProperty<T>,
) -> Box<dyn OwnedRootProperty> {
    own_with_policy(root, property, true)
}
fn own_with_policy<T: PropertyValue>(
    root: &FerroObject,
    property: &'static StyledProperty<T>,
    static_resources: bool,
) -> Box<dyn OwnedRootProperty> {
    Box::new(Property {
        property,
        static_resources,
        version: Rc::new(Cell::new(root.local_write_version(property))),
        expected: Rc::new(RefCell::new(snapshot(root, property))),
    })
}
fn is_static_resource(value: &str) -> bool {
    value
        .strip_prefix("{StaticResource")
        .is_some_and(|rest| rest.starts_with(char::is_whitespace) && rest.ends_with('}'))
}
fn attribute(source: &str, name: &str) -> Result<Option<String>, ReloadError> {
    let xml = roxmltree::Document::parse(source)
        .map_err(|error| ReloadError::Failed(error.to_string()))?;
    Ok(xml.root_element().attribute(name).map(str::to_owned))
}
impl<T: PropertyValue> OwnedRootProperty for Property<T> {
    fn name(&self) -> &str {
        self.property.name()
    }
    fn prepare(
        &self,
        root: &Ref<FerroObject>,
        candidate: &Ref<FerroObject>,
        before: &str,
        after: &str,
        _bindings: &BindingCaptureScope,
    ) -> Result<Option<Box<dyn PreparedReload>>, ReloadError> {
        let old = attribute(before, self.name())?;
        let new = attribute(after, self.name())?;
        if old == new && (!self.static_resources || old.is_none()) {
            return Ok(None);
        }
        if matches!(
            self.name(),
            "Content" | "DataContext" | "Name" | "Resources" | "Styles"
        ) || (!self.static_resources && matches!(self.name(), "Theme" | "Template"))
        {
            return Err(ReloadError::RestartRequired(format!(
                "Root {} requires its lifecycle adapter",
                self.name()
            )));
        }
        if old.iter().chain(new.iter()).any(|value| {
            let value = value.trim();
            value.starts_with('{') && !(self.static_resources && is_static_resource(value))
        }) {
            return Err(ReloadError::RestartRequired(format!(
                "Root {} markup extensions require a binding/resource adapter",
                self.name()
            )));
        }
        let current = snapshot(root, self.property)?;
        let expected = self.expected.borrow().clone()?;
        if root.local_write_version(self.property) != self.version.get()
            || !equal(&current, &expected)
            || (old.is_none() && current.is_some())
        {
            return Err(ReloadError::RestartRequired(format!(
                "Application changed owned root {}",
                self.name()
            )));
        }
        let value = if new.is_some() {
            snapshot(candidate, self.property)?
                .ok_or_else(|| {
                    let message = format!("Root {} did not produce a local value", self.name());
                    if self.static_resources {
                        ReloadError::Failed(message)
                    } else {
                        ReloadError::RestartRequired(message)
                    }
                })?
                .into()
        } else {
            None
        };
        if old == new && equal(&current, &value) {
            return Ok(None);
        }
        Ok(Some(Box::new(Change {
            root: root.clone(),
            property: self.property,
            before: current,
            after: value,
            expected: self.expected.clone(),
            version: self.version.clone(),
            committed_version: 0,
            started: false,
        })))
    }
}

#[derive(Clone)]
struct BindingOwner {
    expression: Option<std::rc::Weak<dyn ferroui_base::data::BindingExpressionBase>>,
    assignment: u64,
}
fn binding_owner(
    root: &FerroObject,
    property: &'static ferroui_base::FerroProperty,
) -> Result<BindingOwner, ReloadError> {
    use ferroui_base::data::BindingOperations;
    let expression = if root.has_local_value_binding(property) {
        let expression = BindingOperations::get_binding_expression_base(root, property)
            .filter(|expression| expression.priority() == BindingPriority::LocalValue)
            .ok_or_else(|| {
                ReloadError::RestartRequired(format!(
                    "Unsupported root {} binding",
                    property.name()
                ))
            })?;
        Some(Rc::downgrade(&expression))
    } else {
        None
    };
    Ok(BindingOwner {
        expression,
        assignment: root.local_assignment_version(property),
    })
}
fn same_owner(a: &BindingOwner, b: &BindingOwner) -> bool {
    a.assignment == b.assignment
        && match (&a.expression, &b.expression) {
            (None, None) => true,
            (Some(a), Some(b)) => std::rc::Weak::ptr_eq(a, b),
            _ => false,
        }
}
struct BindingProperty<T: PropertyValue> {
    property: &'static StyledProperty<T>,
    owner: Rc<RefCell<Result<BindingOwner, ReloadError>>>,
}
pub(crate) fn own_binding<T: PropertyValue>(
    root: &FerroObject,
    property: &'static StyledProperty<T>,
) -> Box<dyn OwnedRootProperty> {
    Box::new(BindingProperty {
        property,
        owner: Rc::new(RefCell::new(binding_owner(root, property))),
    })
}

fn binding_for_root(
    binding: &Rc<dyn ferroui_base::data::BindingBase>,
    root: &Ref<FerroObject>,
    candidate: &Ref<FerroObject>,
) -> Result<Rc<dyn ferroui_base::data::BindingBase>, ReloadError> {
    use ferroui_base::{
        data::{CompiledBinding, ReflectionBinding},
        BoxedValue,
    };
    use ferroui_markup_xaml::FromXamlObject;
    let remap = |value: Option<BoxedValue>| {
        if value
            .as_ref()
            .and_then(Ref::<FerroObject>::from_xaml_object)
            .is_some_and(|object| object.ptr_eq(candidate))
        {
            Some(Rc::new(root.clone()) as BoxedValue)
        } else {
            value
        }
    };
    macro_rules! copy {
        ($binding:expr) => {{
            // These options are retained strongly by the expression. Replacing
            // a temporary-root reference with the live root would create
            // root -> binding -> root ownership; preserving it would retain the
            // detached candidate. Such custom options need their own adapter.
            if [
                $binding.converter_parameter(),
                $binding.fallback_value(),
                $binding.target_null_value(),
            ]
            .into_iter()
            .flatten()
            .any(|value| {
                Ref::<FerroObject>::from_xaml_object(&value)
                    .is_some_and(|object| object.ptr_eq(candidate))
            }) {
                return Err(ReloadError::RestartRequired(
                    "Root references in retained binding options require an adapter".into(),
                ));
            }
            let binding = $binding.snapshot_for_reload();
            binding.set_source(remap(binding.source()));
            binding.set_default_anchor(Some(root.downgrade()));
            Ok(binding as Rc<dyn ferroui_base::data::BindingBase>)
        }};
    }
    if let Some(binding) = binding
        .as_any()
        .and_then(|value| value.downcast_ref::<ReflectionBinding>())
    {
        copy!(binding)
    } else if let Some(binding) = binding
        .as_any()
        .and_then(|value| value.downcast_ref::<CompiledBinding>())
    {
        copy!(binding)
    } else {
        Err(ReloadError::RestartRequired(
            "Custom root binding requires its own adapter".into(),
        ))
    }
}

#[cfg(test)]
mod binding_reference_tests {
    use super::*;
    use ferroui_base::{data::ReflectionBinding, layout::Layoutable, threading::Dispatcher};
    use ferroui_controls::ContentControl;

    #[test]
    fn source_rebases_to_live_root_but_strong_self_options_require_an_adapter() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        let root = ContentControl::new();
        root.set_height(42.0);
        let candidate = ContentControl::new();
        candidate.set_height(13.0);
        let live: Ref<FerroObject> = root.clone().upcast();
        let temporary: Ref<FerroObject> = candidate.clone().upcast();
        let description =
            ReflectionBinding::new("Height").with_source(Some(Rc::new(temporary.clone())));
        let binding: Rc<dyn ferroui_base::data::BindingBase> = description.clone();
        let adapted = binding_for_root(&binding, &live, &temporary).unwrap();
        let expression = root.bind_binding(Layoutable::width_property(), &*adapted);
        assert_eq!(root.width(), 42.0);
        assert!(
            description
                .source()
                .unwrap()
                .downcast_ref::<Ref<FerroObject>>()
                .unwrap()
                .ptr_eq(&temporary),
            "adapting a shared description mutated its original source"
        );
        for option in 0..3 {
            let binding = ReflectionBinding::new("Height");
            let value = Some(Rc::new(temporary.clone()) as ferroui_base::BoxedValue);
            match option {
                0 => binding.set_converter_parameter(value),
                1 => binding.set_fallback_value(value),
                _ => binding.set_target_null_value(value),
            }
            assert!(matches!(
                binding_for_root(
                    &(binding as Rc<dyn ferroui_base::data::BindingBase>),
                    &live,
                    &temporary
                ),
                Err(ReloadError::RestartRequired(_))
            ));
        }
        expression.dispose();
    }
}

// These bindings depend only on the preserved data context and ordinary
// properties. Keeping them avoids discarding uncommitted input on an unrelated
// child edit. Named elements, rooted paths and resource-dependent options must
// instead be recreated against the candidate context.
fn context_only_binding(binding: &Rc<dyn ferroui_base::data::BindingBase>) -> bool {
    use ferroui_base::data::{CompiledBinding, CompiledBindingPathElementKind, ReflectionBinding};
    let unset = |value: Option<ferroui_base::BoxedValue>| {
        value.is_some_and(|value| value.is::<ferroui_base::UnsetValueType>())
    };
    macro_rules! plain {
        ($binding:expr) => {
            unset($binding.source())
                && $binding.converter().is_none()
                && $binding.converter_parameter().is_none()
                && $binding.string_format().is_none()
                && unset($binding.fallback_value())
                && unset($binding.target_null_value())
        };
    }
    if let Some(binding) = binding
        .as_any()
        .and_then(|value| value.downcast_ref::<ReflectionBinding>())
    {
        plain!(binding)
            && binding.element_name().is_none()
            && binding.relative_source().is_none()
            && (binding.path().is_empty()
                || binding.path().split('.').all(|part| {
                    !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                }))
    } else if let Some(binding) = binding
        .as_any()
        .and_then(|value| value.downcast_ref::<CompiledBinding>())
    {
        plain!(binding)
            && binding.path().is_none_or(|path| {
                path.elements()
                    .all(|element| element.kind() == CompiledBindingPathElementKind::Property)
            })
    } else {
        false
    }
}

impl<T: PropertyValue> OwnedRootProperty for BindingProperty<T> {
    fn name(&self) -> &str {
        self.property.name()
    }
    fn binding_property(&self) -> Option<&'static ferroui_base::FerroProperty> {
        Some(self.property)
    }
    fn prepare(
        &self,
        root: &Ref<FerroObject>,
        candidate: &Ref<FerroObject>,
        before: &str,
        after: &str,
        bindings: &BindingCaptureScope,
    ) -> Result<Option<Box<dyn PreparedReload>>, ReloadError> {
        use ferroui_base::data::LocalBindingReload;
        let old = attribute(before, self.name())?;
        let new = attribute(after, self.name())?;
        let captured = bindings.binding(candidate, self.property);
        if old == new && captured.is_none() {
            return Ok(None);
        }
        if matches!(
            self.name(),
            "Content"
                | "Children"
                | "DataContext"
                | "Name"
                | "Resources"
                | "Styles"
                | "Theme"
                | "Template"
        ) {
            return Err(ReloadError::RestartRequired(format!(
                "Root {} requires its lifecycle adapter",
                self.name()
            )));
        }
        let owner = self.owner.borrow().clone()?;
        if !same_owner(&owner, &binding_owner(root, self.property)?) {
            return Err(ReloadError::RestartRequired(format!(
                "Application changed owned root {}",
                self.name()
            )));
        }
        if old == new
            // Even a plain resolved path/mode can have come from a resource.
            // Its authored nested extension must be evaluated on each revision.
            && new.as_ref().is_some_and(|source| source.matches('{').count() == 1)
            && owner
                .expression
                .as_ref()
                .and_then(std::rc::Weak::upgrade)
                .is_some_and(|expression| expression.can_pause_for_reload())
            && captured.as_ref().is_some_and(context_only_binding)
        {
            return Ok(None);
        }
        // An absent authored attribute cannot own an application's pre-existing
        // local contribution, even if registration happened after that assignment.
        if old.is_none() && (owner.expression.is_some() || snapshot(root, self.property)?.is_some())
        {
            return Err(ReloadError::RestartRequired(format!(
                "Root {} was not XAML-owned",
                self.name()
            )));
        }
        let change = match captured {
            Some(binding) => {
                let binding = binding_for_root(&binding, root, candidate)?;
                LocalBindingReload::prepare(root, self.property, Some(&*binding), Some(root))
            }
            None => {
                if new
                    .as_ref()
                    .is_some_and(|value| value.trim_start().starts_with('{'))
                {
                    return Err(ReloadError::RestartRequired(
                        "Root markup extension needs its own adapter".into(),
                    ));
                }
                let value = if new.is_some() {
                    Some(snapshot(candidate, self.property)?.ok_or_else(|| {
                        ReloadError::Failed(format!(
                            "Root {} did not produce a local value",
                            self.name()
                        ))
                    })?)
                } else {
                    None
                };
                LocalBindingReload::prepare_value(root, self.property, value)
            }
        }
        .map_err(ReloadError::RestartRequired)?;
        Ok(Some(Box::new(BindingChange {
            change,
            root: root.clone(),
            property: self.property,
            owner: self.owner.clone(),
            committed: None,
            entered: false,
        })))
    }
}

struct BindingChange<T: PropertyValue> {
    change: ferroui_base::data::LocalBindingReload<T>,
    root: Ref<FerroObject>,
    property: &'static StyledProperty<T>,
    owner: Rc<RefCell<Result<BindingOwner, ReloadError>>>,
    committed: Option<BindingOwner>,
    entered: bool,
}
impl<T: PropertyValue> PreparedReload for BindingChange<T> {
    fn commit(&mut self) -> Result<(), ReloadError> {
        let owner = self.owner.borrow().clone()?;
        if !same_owner(&owner, &binding_owner(&self.root, self.property)?) {
            return Err(ReloadError::RestartRequired(format!(
                "Application changed owned root {} after preparation",
                self.property.name()
            )));
        }
        self.entered = true;
        self.change.commit().map_err(ReloadError::RestartRequired)?;
        self.committed = Some(binding_owner(&self.root, self.property)?);
        Ok(())
    }
    fn rollback(&mut self) {
        if !self.entered {
            return;
        }
        *self.owner.borrow_mut() = self
            .change
            .rollback()
            .map_err(ReloadError::RestartRequired)
            .and_then(|_| binding_owner(&self.root, self.property));
    }
    fn finish(&mut self) {
        *self.owner.borrow_mut() =
            self.change
                .finish()
                .map_err(ReloadError::Failed)
                .and_then(|_| {
                    self.committed
                        .take()
                        .ok_or_else(|| ReloadError::Failed("Missing binding commit".into()))
                });
    }
}
struct Change<T: PropertyValue> {
    root: Ref<FerroObject>,
    property: &'static StyledProperty<T>,
    before: Option<T>,
    after: Option<T>,
    expected: Rc<RefCell<Result<Option<T>, ReloadError>>>,
    version: Rc<Cell<u64>>,
    started: bool,
    committed_version: u64,
}
impl<T: PropertyValue> Change<T> {
    fn assign(&self, value: &Option<T>) {
        match value {
            Some(value) => self.root.set_value(self.property, value.clone()),
            None => self.root.clear_value(self.property),
        }
    }
}
impl<T: PropertyValue> PreparedReload for Change<T> {
    fn commit(&mut self) -> Result<(), ReloadError> {
        if self.root.local_write_version(self.property) != self.version.get()
            || !equal(&snapshot(&self.root, self.property)?, &self.before)
        {
            return Err(ReloadError::RestartRequired(format!(
                "Root {} changed after preparation",
                self.property.name()
            )));
        }
        self.started = true;
        self.assign(&self.after);
        self.committed_version = self.root.local_write_version(self.property);
        Ok(())
    }
    fn rollback(&mut self) {
        if self.started {
            self.assign(&self.before);
            self.version
                .set(self.root.local_write_version(self.property));
        }
    }
    fn finish(&mut self) {
        *self.expected.borrow_mut() = Ok(self.after.clone());
        self.version.set(self.committed_version);
    }
}
