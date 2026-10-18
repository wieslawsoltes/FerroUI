//! `TreeDataGridBindingAccessor.cs`: reads and writes the value a binding of a declarative column
//! selects from a model, by binding the two-way property of a probe element and setting the
//! model as its data context, and finds the objects on the binding's path whose changes the
//! column observes (the links).
//!
//! The reference reads the elements of a compiled binding's path through a private member and
//! resolves a reflection binding's path on the model's static type. The port reads a compiled
//! binding's path through the framework's public path elements, and resolves a reflection
//! binding's path on the models at run time through the property metadata of their types
//! (`ModelTypes`), the stand-in of reflection.

use crate::TreeDataGridBindingProbe;
use ferroui_base::data::core::{IPropertyInfo, ValueType, ValueTypes};
use ferroui_base::data::model::ModelTypes;
use ferroui_base::data::{
    BindingBase, BindingExpressionBase, BindingMode, BindingNotification, BindingOperations, CompiledBinding,
    CompiledBindingPath, CompiledBindingPathElementKind, MultiBinding, ReflectionBinding,
};
use ferroui_markup_xaml::markup_extensions::{CompiledBindingExtension, ReflectionBindingExtension};
use ferroui_base::{BoxedValue, Ref};
use std::cell::Cell;
use std::rc::Rc;

/// An object below a model whose changes a column observes (`Func<object, object>`; the
/// `ObjectLink` of the declarative view columns).
pub type ObjectLink = Rc<dyn Fn(&BoxedValue) -> Option<BoxedValue>>;

/// How a link finds the owner on the path below a model.
enum PathStep {
    // A property of a compiled binding's path.
    Property(Rc<dyn IPropertyInfo>),
    // A member of a reflection binding's path, looked up by name on the owner's type.
    Member(String),
}

fn evaluate(model: &BoxedValue, steps: &[PathStep], length: usize) -> Option<BoxedValue> {
    let mut current = Some(model.clone());
    for step in steps.iter().take(length) {
        let owner = current?;
        current = match step {
            PathStep::Property(property) => property.get(&*owner),
            PathStep::Member(name) => ModelTypes::find(&*owner).and_then(|t| t.find_property(name)).and_then(|p| p.get(&*owner)),
        };
        current = current.and_then(ValueTypes::normalize);
    }
    current
}

struct ReflectionInfo {
    path: String,
    has_own_source: bool,
    source: Option<BoxedValue>,
}

fn self_links() -> Vec<ObjectLink> {
    vec![Rc::new(|model: &BoxedValue| Some(model.clone())) as ObjectLink]
}

fn create_links(steps: Vec<PathStep>) -> Vec<ObjectLink> {
    if steps.is_empty() {
        return self_links();
    }
    let steps = Rc::new(steps);
    let mut result = self_links();
    for i in 1..steps.len() {
        let steps = steps.clone();
        result.push(Rc::new(move |model: &BoxedValue| evaluate(model, &steps, i)) as ObjectLink);
    }
    result
}

/// The accessor of the value a binding selects from a model (`TreeDataGridBindingAccessor`).
pub struct TreeDataGridBindingAccessor {
    probe: Ref<TreeDataGridBindingProbe>,
    expression: Rc<dyn BindingExpressionBase>,
    links: Vec<ObjectLink>,
    mode: BindingMode,
    declared_type: Option<ValueType>,
    inferred_type: Cell<Option<ValueType>>,
    sample_value_was_null: bool,
}

impl TreeDataGridBindingAccessor {
    /// The accessor of `binding`; `sample_model` is a model of the items, whose value tells
    /// whether the value can be null and, for a reflection binding, the type of the value.
    pub fn new(binding: &Rc<dyn BindingBase>, sample_model: Option<&BoxedValue>) -> Rc<Self> {
        let probe = TreeDataGridBindingProbe::new();
        let expression = probe.bind_binding(TreeDataGridBindingProbe::value_property().as_property(), &**binding);
        let mode = Self::get_mode(&**binding);
        let steps = Self::try_create_steps(&**binding);
        let declared_type = Self::try_get_value_type(&**binding, sample_model);
        let links = steps.map(create_links).unwrap_or_else(self_links);
        let mut this = Self {
            probe,
            expression,
            links,
            mode,
            declared_type,
            inferred_type: Cell::new(None),
            sample_value_was_null: false,
        };
        if let Some(sample) = sample_model {
            let sample_value = this.read(sample);
            this.sample_value_was_null = sample_value.is_none();
        }
        Rc::new(this)
    }

    /// Whether the binding writes values back to the model (`CanWrite`).
    pub fn can_write(&self) -> bool {
        matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource)
    }

    /// The objects on the binding's path whose changes the column observes (`Links`): the model,
    /// then the owner of each member read below it.
    pub fn links(&self) -> Vec<ObjectLink> {
        self.links.clone()
    }

    /// Whether the value of the sample model was null (`SampleValueWasNull`).
    pub fn sample_value_was_null(&self) -> bool {
        self.sample_value_was_null
    }

    /// The type of the value: the type the binding's path declares, or the type of the first value
    /// read (`ValueType`).
    pub fn value_type(&self) -> Option<ValueType> {
        self.declared_type.or(self.inferred_type.get())
    }

    /// The selector of the text `binding` selects from a model, if there is a binding
    /// (`TryCreateTextSelector`).
    pub fn try_create_text_selector(binding: Option<&Rc<dyn BindingBase>>) -> Option<Rc<dyn Fn(&BoxedValue) -> Option<String>>> {
        let accessor = Self::new(binding?, None);
        Some(Rc::new(move |model: &BoxedValue| accessor.read_as_string(model)))
    }

    /// The value the binding selects from `model` (`Read`).
    pub fn read(&self, model: &BoxedValue) -> Option<BoxedValue> {
        self.set_model(model);
        self.expression.update_target();
        let value = Self::normalize_value(self.probe.value());

        if let Some(value) = &value {
            if self.inferred_type.get().is_none() {
                self.inferred_type.set(Some(ValueType::of_value(&**value)));
            }
        }

        value
    }

    /// The text of the value (`ReadAsString`: `ToString()` of the value).
    pub fn read_as_string(&self, model: &BoxedValue) -> Option<String> {
        self.read(model).map(|value| ValueTypes::to_display_string(Some(&value)))
    }

    /// The value as a boolean, false for null (`ReadAsBoolean`).
    pub fn read_as_boolean(&self, model: &BoxedValue) -> bool {
        Self::convert_to_nullable_boolean(self.read(model)).unwrap_or(false)
    }

    /// The value as a nullable boolean (`ReadAsNullableBoolean`).
    pub fn read_as_nullable_boolean(&self, model: &BoxedValue) -> Option<bool> {
        Self::convert_to_nullable_boolean(self.read(model))
    }

    /// Writes `value` to `model` through the binding, converted to the type of the value
    /// (`Write`); nothing when the binding does not write back.
    pub fn write(&self, model: &BoxedValue, value: Option<BoxedValue>) {
        if !self.can_write() {
            return;
        }

        self.set_model(model);
        self.probe.set_current_value(TreeDataGridBindingProbe::value_property(), self.convert_for_source(value));
        self.expression.update_source();
        self.expression.update_target();

        let written_value = Self::normalize_value(self.probe.value());

        if let Some(value) = &written_value {
            if self.inferred_type.get().is_none() {
                self.inferred_type.set(Some(ValueType::of_value(&**value)));
            }
        }
    }

    /// Ends the binding (`Dispose`).
    pub fn dispose(&self) {
        self.expression.dispose();
    }

    fn get_mode(binding: &dyn BindingBase) -> BindingMode {
        let any = binding.as_any();
        let mode = any
            .and_then(|a| a.downcast_ref::<ReflectionBinding>())
            .map(|b| b.mode())
            .or_else(|| any.and_then(|a| a.downcast_ref::<CompiledBinding>()).map(|b| b.mode()))
            .or_else(|| any.and_then(|a| a.downcast_ref::<MultiBinding>()).map(|b| b.mode()))
            .or_else(|| any.and_then(|a| a.downcast_ref::<ReflectionBindingExtension>()).map(|b| b.base().mode()))
            .or_else(|| any.and_then(|a| a.downcast_ref::<CompiledBindingExtension>()).map(|b| b.base().mode()));
        match mode {
            Some(mode) => Self::normalize_mode(mode),
            None => BindingMode::TwoWay,
        }
    }

    fn normalize_mode(mode: BindingMode) -> BindingMode {
        if mode == BindingMode::Default {
            BindingMode::TwoWay
        } else {
            mode
        }
    }

    fn normalize_value(value: Option<BoxedValue>) -> Option<BoxedValue> {
        let value = BindingNotification::extract_value(value.as_ref());
        if BindingOperations::is_unset(value.as_ref()) {
            return None;
        }
        value.and_then(ValueTypes::normalize)
    }

    fn convert_to_nullable_boolean(value: Option<BoxedValue>) -> Option<bool> {
        let value = Self::normalize_value(value)?;
        if let Some(value) = value.downcast_ref::<bool>() {
            return Some(*value);
        }
        ValueTypes::try_convert(Some(&value), ValueType::of::<bool>())
            .flatten()
            .and_then(|converted| converted.downcast_ref::<bool>().copied())
    }

    fn convert_for_source(&self, value: Option<BoxedValue>) -> Option<BoxedValue> {
        let value = Self::normalize_value(value)?;

        let Some(target_type) = self.value_type() else {
            return Some(value);
        };

        let underlying_type = ValueTypes::nullable_inner(target_type).unwrap_or(target_type);

        if ValueTypes::is_assignable(ValueType::of_value(&*value), underlying_type) {
            return Some(value);
        }

        // `Enum.Parse`, `Enum.ToObject` and `Convert.ChangeType`: the conversions of the
        // framework's values, which parse enumerations from text and numbers.
        match ValueTypes::try_convert(Some(&value), underlying_type) {
            Some(Some(converted)) => Some(converted),
            _ => Some(value),
        }
    }

    fn set_model(&self, model: &BoxedValue) {
        let same = self.probe.data_context().is_some_and(|current| std::ptr::addr_eq(Rc::as_ptr(&current), Rc::as_ptr(model)));
        if !same {
            self.probe.set_data_context(Some(model.clone()));
        }
    }

    // The steps of the path of a compiled binding (`TryCreateCompiledBindingLinks`), or of a
    // reflection binding without a source of its own (`TryCreateReflectionBindingLinks`).
    fn try_create_steps(binding: &dyn BindingBase) -> Option<Vec<PathStep>> {
        let any = binding.as_any()?;
        if let Some(path) = Self::compiled_path(any) {
            return Self::try_get_compiled_binding_steps(&path?);
        }
        if let Some(reflection) = Self::reflection_info(any) {
            if reflection.has_own_source {
                return None;
            }
            return Self::try_resolve_reflection_path(&reflection.path);
        }
        None
    }

    // The path of a compiled binding (given as itself or as its markup extension); the outer
    // `None` when the binding is not a compiled binding.
    fn compiled_path(any: &dyn std::any::Any) -> Option<Option<CompiledBindingPath>> {
        if let Some(binding) = any.downcast_ref::<CompiledBindingExtension>() {
            return Some(binding.base().path());
        }
        any.downcast_ref::<CompiledBinding>().map(|binding| binding.path())
    }

    // What the links of a reflection binding read: its path, and whether it has a source of its
    // own (a source, a relative source or an element name); `None` when the binding is not a
    // reflection binding.
    fn reflection_info(any: &dyn std::any::Any) -> Option<ReflectionInfo> {
        let binding: &ReflectionBinding = match any.downcast_ref::<ReflectionBindingExtension>() {
            Some(extension) => &**extension.base(),
            None => any.downcast_ref::<ReflectionBinding>()?,
        };
        let source = binding.source().filter(|source| !BindingOperations::is_unset(Some(source)));
        Some(ReflectionInfo {
            path: binding.path(),
            has_own_source: source.is_some()
                || binding.relative_source().is_some()
                || binding.element_name().is_some_and(|name| !name.is_empty()),
            source,
        })
    }

    fn try_get_compiled_binding_steps(path: &CompiledBindingPath) -> Option<Vec<PathStep>> {
        let mut result = Vec::new();
        for element in path.elements() {
            match element.kind() {
                CompiledBindingPathElementKind::Property => result.push(PathStep::Property(element.property()?)),
                CompiledBindingPathElementKind::SelfElement | CompiledBindingPathElementKind::TypeCast => {}
                _ => return None,
            }
        }
        Some(result)
    }

    fn try_resolve_reflection_path(path: &str) -> Option<Vec<PathStep>> {
        if path.trim().is_empty() || path == "." {
            return Some(Vec::new());
        }

        let mut result = Vec::new();
        for raw_part in path.split('.') {
            let part = raw_part.trim();
            if part.is_empty() || part.contains(['[', '(', '!', '#', '$', '/']) {
                return None;
            }
            result.push(PathStep::Member(part.to_string()));
        }
        Some(result)
    }

    // `TryGetValueType`: the type of the last property of a compiled binding's path, or of the
    // member a reflection binding's path ends with on the sample model (or the source's).
    fn try_get_value_type(binding: &dyn BindingBase, sample_model: Option<&BoxedValue>) -> Option<ValueType> {
        let any = binding.as_any()?;
        if let Some(path) = Self::compiled_path(any) {
            let path = path?;
            let last = path.elements().filter(|e| matches!(e.kind(), CompiledBindingPathElementKind::Property)).last()?;
            return last.property_type().or_else(|| last.property().map(|p| p.property_type()));
        }
        if let Some(reflection) = Self::reflection_info(any) {
            let root = reflection.source.or_else(|| sample_model.cloned())?;
            let steps = Self::try_resolve_reflection_path(&reflection.path)?;
            if steps.is_empty() {
                return Some(ValueType::of_value(&*root));
            }
            let owner = evaluate(&root, &steps, steps.len() - 1)?;
            let PathStep::Member(name) = steps.last()? else {
                return None;
            };
            return ModelTypes::find(&*owner).and_then(|t| t.find_property(name)).map(|p| p.property_type());
        }
        None
    }
}

impl Drop for TreeDataGridBindingAccessor {
    fn drop(&mut self) {
        self.expression.dispose();
    }
}
