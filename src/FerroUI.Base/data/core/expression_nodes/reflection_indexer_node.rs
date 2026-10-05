use super::{CollectionNode, CollectionNodeBase, ExpressionNode, ISettableNode, NodeState};
use crate::data::core::plugins::markup_members;
use crate::data::core::{ValueType, ValueTypes, INDEXER_NAME};
use crate::data::model::{IBindableIndexer, IBindableList, ModelType, ModelTypes};
use crate::data::BindingError;
use crate::metadata::{MarkupIndexer, MarkupValue};
use crate::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The indexer a source declared: a list indexed by position, an indexer
/// with its own parameters, or an indexer declared in the markup metadata of
/// the type of the source (or of one of its base types).
enum Indexer<'a> {
    List(&'a dyn IBindableList),
    Indexer(&'a dyn IBindableIndexer),
    Markup(&'static MarkupIndexer, &'a BoxedValue),
}

impl<'a> Indexer<'a> {
    fn find(model: Option<&ModelType>, source: &'a BoxedValue, argument_count: usize) -> Option<Self> {
        if let Some(model) = model {
            if let Some(indexer) = model.as_indexer(&**source) {
                return Some(Indexer::Indexer(indexer));
            }
            if let Some(list) = model.as_list(&**source) {
                return Some(Indexer::List(list));
            }
        }
        // An indexer without a getter is no indexer for a binding.
        markup_members::find_indexer(&**source, argument_count)
            .filter(|indexer| indexer.get.is_some())
            .map(|indexer| Indexer::Markup(indexer, source))
    }

    fn index_parameter_types(&self) -> Vec<ValueType> {
        match self {
            Indexer::List(_) => vec![ValueType::of::<i32>()],
            Indexer::Indexer(indexer) => indexer.index_parameter_types(),
            Indexer::Markup(indexer, _) => indexer.parameters.iter().map(|parameter| parameter()).collect(),
        }
    }

    fn value_type(&self) -> ValueType {
        match self {
            Indexer::List(list) => list.item_type(),
            Indexer::Indexer(indexer) => indexer.value_type(),
            Indexer::Markup(indexer, _) => (indexer.type_)(),
        }
    }

    fn can_set(&self) -> bool {
        match self {
            Indexer::List(_) => true,
            Indexer::Indexer(indexer) => indexer.can_set(),
            Indexer::Markup(indexer, _) => indexer.set.is_some(),
        }
    }

    /// The arguments of an invoker of a metadata indexer: the instance, then
    /// the indexes.
    fn markup_arguments(source: &BoxedValue, arguments: &[BoxedValue]) -> Vec<MarkupValue> {
        let mut all = Vec::with_capacity(arguments.len() + 2);
        all.push(Some(source.clone()));
        all.extend(arguments.iter().map(|argument| markup_members::untyped(Some(argument))));
        all
    }

    fn list_index(list: &dyn IBindableList, arguments: &[BoxedValue]) -> Result<usize, BindingError> {
        arguments
            .first()
            .and_then(|a| a.downcast_ref::<i32>())
            .and_then(|i| usize::try_from(*i).ok())
            .filter(|i| *i < list.count())
            .ok_or_else(|| {
                BindingError::message(
                    "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')",
                )
            })
    }

    fn get(&self, arguments: &[BoxedValue]) -> Result<Option<BoxedValue>, BindingError> {
        match self {
            Indexer::List(list) => Ok(list.get_item(Self::list_index(*list, arguments)?)),
            Indexer::Indexer(indexer) => indexer.get(arguments),
            Indexer::Markup(indexer, source) => match indexer.get {
                Some(get) => get(&Self::markup_arguments(source, arguments)).map_err(markup_members::invoke_error),
                None => Ok(None),
            },
        }
    }

    fn set(&self, arguments: &[BoxedValue], value: Option<&BoxedValue>) -> Result<bool, BindingError> {
        match self {
            Indexer::List(list) => {
                let index = Self::list_index(*list, arguments)?;
                Ok(value.is_some_and(|v| list.set_item(index, v)))
            }
            Indexer::Indexer(indexer) => indexer.set(arguments, value),
            Indexer::Markup(indexer, source) => {
                let Some(set) = indexer.set else { return Ok(false) };
                let mut all = Self::markup_arguments(source, arguments);
                all.push(markup_members::untyped(value));
                set(&all).map(|_| true).map_err(markup_members::invoke_error)
            }
        }
    }
}

/// A node that reads a value through the indexer the source's type declared
/// in its binding metadata or its markup metadata
/// (`[2]`, `[key]`, `[1,2]`), following the change notifications of the
/// source.
///
/// The arguments are the text of the binding path; they are converted to the
/// declared types of the index parameters each time the source changes.
pub struct ReflectionIndexerNode {
    this: Weak<ReflectionIndexerNode>,
    state: NodeState,
    base: CollectionNodeBase,
    arguments: Vec<String>,
    has_getter: Cell<bool>,
    has_setter: Cell<bool>,
    value_type: Cell<Option<ValueType>>,
    indexes: RefCell<Option<Vec<BoxedValue>>>,
}

impl ReflectionIndexerNode {
    pub fn new(arguments: Vec<String>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            base: CollectionNodeBase::new(),
            arguments,
            has_getter: Cell::new(false),
            has_setter: Cell::new(false),
            value_type: Cell::new(None),
            indexes: RefCell::new(None),
        })
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    fn convert_indexes(index_parameters: &[ValueType], arguments: &[String]) -> Result<Vec<BoxedValue>, String> {
        let mut result = Vec::with_capacity(index_parameters.len());
        for (i, type_) in index_parameters.iter().enumerate() {
            let argument: BoxedValue = Rc::new(arguments[i].clone());
            match ValueTypes::try_convert(Some(&argument), *type_) {
                Some(Some(value)) => result.push(value),
                _ => {
                    return Err(format!(
                        "Could not convert list index '{}' of type '{}' to '{}'.",
                        i, arguments[i], type_
                    ))
                }
            }
        }
        Ok(result)
    }
}

impl ExpressionNode for ReflectionIndexerNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push('[');
        for (i, argument) in self.arguments.iter().enumerate() {
            builder.push_str(argument);
            if i != self.arguments.len() - 1 {
                builder.push(',');
            }
        }
        builder.push(']');
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let Some(source_value) = source else { return };

        self.indexes.replace(None);
        self.has_getter.set(false);
        self.has_setter.set(false);
        self.value_type.set(None);

        let model = ModelTypes::find(&**source_value);
        let Some(indexer) = Indexer::find(model.as_deref(), source_value, self.arguments.len()) else {
            self.state.set_error(&format!(
                "Type '{}' does not have an indexer.",
                markup_members::type_name_of(&**source_value)
            ));
            return;
        };
        self.has_getter.set(true);
        self.has_setter.set(indexer.can_set());
        self.value_type.set(Some(indexer.value_type()));

        let parameters = indexer.index_parameter_types();
        if parameters.len() != self.arguments.len() {
            self.state.set_error(&format!(
                "Wrong number of arguments for indexer: expected {}, got {}.",
                parameters.len(),
                self.arguments.len()
            ));
            return;
        }

        match Self::convert_indexes(&parameters, &self.arguments) {
            Ok(indexes) => {
                self.indexes.replace(Some(indexes));
            }
            Err(message) => {
                self.state.set_error(&message);
                return;
            }
        }
        CollectionNodeBase::on_source_changed(&self.this, self, source);
    }

    fn unsubscribe(&self, old_source: &BoxedValue) {
        self.base.unsubscribe(old_source);
    }

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }
}

impl CollectionNode for ReflectionIndexerNode {
    fn collection_base(&self) -> &CollectionNodeBase {
        &self.base
    }

    fn should_update(&self, _source: &BoxedValue, property_name: &str) -> bool {
        // The notification names the indexer property itself. A name that
        // carries the arguments (`Item[key]`) is not the name of a property.
        property_name == INDEXER_NAME
    }

    fn try_get_first_argument_as_int(&self) -> Option<i32> {
        let argument: BoxedValue = Rc::new(self.arguments.first()?.clone());
        let value = ValueTypes::try_convert(Some(&argument), ValueType::of::<i32>())??;
        value.downcast_ref::<i32>().copied()
    }

    fn update_value(&self, source: &BoxedValue) -> Result<(), BindingError> {
        let indexes = self.indexes.borrow().clone();
        let model = ModelTypes::find(&**source);
        match (Indexer::find(model.as_deref(), source, self.arguments.len()), indexes) {
            (Some(indexer), Some(indexes)) if self.has_getter.get() => {
                let value = indexer.get(&indexes)?;
                self.state.set_value(value.and_then(ValueTypes::normalize), None);
            }
            _ => self.state.clear_value(),
        }
        Ok(())
    }
}

impl ISettableNode for ReflectionIndexerNode {
    fn value_type(&self) -> Option<ValueType> {
        self.value_type.get()
    }

    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        _nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        let Some(source) = self.state.source() else { return Ok(false) };
        if !self.has_setter.get() {
            return Ok(false);
        }
        let Some(indexes) = self.indexes.borrow().clone() else { return Ok(false) };
        let model = ModelTypes::find(&*source);
        match Indexer::find(model.as_deref(), &source, self.arguments.len()) {
            Some(indexer) => indexer.set(&indexes, value),
            None => Ok(false),
        }
    }
}

impl Drop for ReflectionIndexerNode {
    fn drop(&mut self) {
        if let Some(source) = self.state.source() {
            self.base.unsubscribe(&source);
        }
    }
}
