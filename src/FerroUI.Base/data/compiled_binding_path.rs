use crate::controls::NameScopeRef;
use crate::data::core::expression_nodes::{
    ArrayIndexerNode, CastTarget, ExpressionNode, FerroPropertyAccessorNode, FuncTransformNode,
    LogicalAncestorElementNode, LogicalNotNode, MethodCommandNode, NamedElementNode, PropertyAccessorNode,
    StreamNode, TemplatedParentNode, VisualAncestorElementNode,
};
use crate::data::core::plugins::{
    IPropertyAccessorPlugin, IStreamPlugin, ObservableStreamPlugin, PropertyAccessorFactory,
    PropertyInfoAccessorFactory, PropertyInfoAccessorPlugin, TaskStreamPlugin, UntypedAccessorPlugin,
    AsNotifyPropertyChanged, UntypedCanExecute, UntypedCreateDelegate, UntypedExecute, UntypedGetter,
    UntypedMember, UntypedSetter,
};
use crate::data::core::{
    ClrPropertyInfo, IPropertyInfo, PropertyKind, TypedBindingExpression, TypedClrPropertyInfo, Value, ValueType,
    ValueTypes, INDEXER_NAME,
};
use crate::data::{BindingError, BindingExpressionBase, BindingMode, BindingPriority};
use crate::data::model::{BindableDictionary, BindableList, INotifyPropertyChanged};
use crate::{AnyValue, BoxedValue, FerroProperty, ObjectType, PropertyValue, TypeInfo};
use std::cell::RefCell;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

type Execute = Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>)>;
type CanExecute = Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>) -> bool>;
type InpcLookup = Rc<dyn Fn(&dyn AnyValue) -> Option<&dyn INotifyPropertyChanged>>;
type Cast = Rc<dyn Fn(Option<&BoxedValue>) -> Option<BoxedValue>>;

/// An element of a compiled binding path.
#[derive(Clone)]
enum PathElement {
    Not,
    /// A registered property of an object of the class hierarchy.
    FerroProperty { property: &'static FerroProperty, accepts_null: bool },
    /// A plain property with typed accessors: eligible for the typed
    /// binding expression.
    TypedProperty {
        property: Rc<dyn IPropertyInfo>,
        accessor_factory: PropertyAccessorFactory,
        accepts_null: bool,
        typed: Rc<dyn TypedPropertyElement>,
    },
    /// A plain property, accessed through closures.
    Property { property: Rc<dyn IPropertyInfo>, accessor_factory: PropertyAccessorFactory, accepts_null: bool },
    /// A member accessed through closures over untyped values.
    Untyped {
        name: Box<str>,
        kind: CompiledBindingPathElementKind,
        property_type: Option<ValueType>,
        plugin: Rc<dyn IPropertyAccessorPlugin>,
        accepts_null: bool,
    },
    MethodAsCommand {
        method_name: Box<str>,
        execute: Execute,
        can_execute: Option<CanExecute>,
        depends_on_properties: Vec<Box<str>>,
        inpc: Option<InpcLookup>,
    },
    Stream(Rc<dyn IStreamPlugin>),
    SelfElement,
    Ancestor { ancestor_type: Option<&'static TypeInfo>, level: usize },
    VisualAncestor { ancestor_type: Option<&'static TypeInfo>, level: usize },
    // Deviation (DEVIATIONS.md, Bindings): the name scope is held weakly. The
    // managed original holds it, and its
    // collector frees a path that is held by what the scope names: the
    // binding of a setter of a style declared under a named element (or
    // under an element with a named ancestor) is held by that element, and
    // the scope holds the element. The node created from the path has
    // always held the scope weakly, as a reflection binding holds its
    // `NameScope`; a path whose scope is gone creates a node without one.
    ElementName { name_scope: std::rc::Weak<dyn crate::controls::INameScope>, name: Box<str> },
    TemplatedParent,
    ArrayElement { indices: Vec<i32> },
    TypeCast { type_name: &'static str, cast: Cast },
}

/// What an element of a compiled binding path is: the element classes of
/// the managed original.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompiledBindingPathElementKind {
    /// Negates the result of the path (`!`).
    Not,
    /// A property: a registered property, or a plain property (with typed
    /// or untyped accessors).
    Property,
    /// A method as a delegate value.
    MethodAsDelegate,
    /// A method as a command.
    MethodAsCommand,
    /// A stream (`^`).
    Stream,
    /// The binding target itself (`$self`).
    SelfElement,
    /// An ancestor in the logical tree.
    Ancestor,
    /// An ancestor in the visual tree.
    VisualAncestor,
    /// A named element.
    ElementName,
    /// The templated parent.
    TemplatedParent,
    /// An array element.
    ArrayElement,
    /// A type cast.
    TypeCast,
}

/// A read-only view of an element of a [`CompiledBindingPath`].
#[derive(Clone, Copy)]
pub struct CompiledBindingPathElement<'a>(&'a PathElement);

impl<'a> CompiledBindingPathElement<'a> {
    /// What the element is.
    pub fn kind(&self) -> CompiledBindingPathElementKind {
        use CompiledBindingPathElementKind as Kind;
        match self.0 {
            PathElement::Not => Kind::Not,
            PathElement::FerroProperty { .. } | PathElement::TypedProperty { .. } | PathElement::Property { .. } => {
                Kind::Property
            }
            PathElement::Untyped { kind, .. } => *kind,
            PathElement::MethodAsCommand { .. } => Kind::MethodAsCommand,
            PathElement::Stream(_) => Kind::Stream,
            PathElement::SelfElement => Kind::SelfElement,
            PathElement::Ancestor { .. } => Kind::Ancestor,
            PathElement::VisualAncestor { .. } => Kind::VisualAncestor,
            PathElement::ElementName { .. } => Kind::ElementName,
            PathElement::TemplatedParent => Kind::TemplatedParent,
            PathElement::ArrayElement { .. } => Kind::ArrayElement,
            PathElement::TypeCast { .. } => Kind::TypeCast,
        }
    }

    /// The name of the member a property or method element reads, of the
    /// element a named-element element selects.
    pub fn name(&self) -> Option<&'a str> {
        match self.0 {
            PathElement::FerroProperty { property, .. } => Some(property.name()),
            PathElement::TypedProperty { property, .. } | PathElement::Property { property, .. } => {
                Some(property.name())
            }
            PathElement::Untyped { name, .. } => Some(name),
            PathElement::MethodAsCommand { method_name, .. } => Some(method_name),
            PathElement::ElementName { name, .. } => Some(name),
            _ => None,
        }
    }

    /// The description of the property a property element reads (the
    /// `Property` of the managed original). `None` for other elements and
    /// for a property that only has untyped accessors (its name and type
    /// are [`name`](Self::name) and [`property_type`](Self::property_type)).
    pub fn property(&self) -> Option<Rc<dyn IPropertyInfo>> {
        match self.0 {
            PathElement::FerroProperty { property, .. } => Some(property.as_property_info()),
            PathElement::TypedProperty { property, .. } | PathElement::Property { property, .. } => {
                Some(property.clone())
            }
            _ => None,
        }
    }

    /// The type of the property a property element reads.
    pub fn property_type(&self) -> Option<ValueType> {
        match self.0 {
            PathElement::Untyped { property_type, .. } => *property_type,
            _ => self.property().map(|property| property.property_type()),
        }
    }

    /// Whether the element is a property with typed accessors: the one a
    /// path must consist of to be instantiated as a typed binding
    /// expression.
    pub fn is_typed_property(&self) -> bool {
        matches!(self.0, PathElement::TypedProperty { .. })
    }

    /// Whether a null-conditional operator applies to the access.
    pub fn accepts_null(&self) -> bool {
        match self.0 {
            PathElement::FerroProperty { accepts_null, .. }
            | PathElement::TypedProperty { accepts_null, .. }
            | PathElement::Property { accepts_null, .. }
            | PathElement::Untyped { accepts_null, .. } => *accepts_null,
            _ => false,
        }
    }

    /// The ancestor type and level of an ancestor element.
    pub fn ancestor(&self) -> Option<(Option<&'static TypeInfo>, usize)> {
        match self.0 {
            PathElement::Ancestor { ancestor_type, level } | PathElement::VisualAncestor { ancestor_type, level } => {
                Some((*ancestor_type, *level))
            }
            _ => None,
        }
    }

    /// The indexes of an array element.
    pub fn array_indices(&self) -> Option<&'a [i32]> {
        match self.0 {
            PathElement::ArrayElement { indices } => Some(indices),
            _ => None,
        }
    }
}

/// A path element that can create a typed binding expression.
pub(crate) trait TypedPropertyElement {
    fn value_type(&self) -> ValueType;
    fn can_set(&self) -> bool;
    fn create_expression(&self, mode: BindingMode, priority: BindingPriority) -> Rc<dyn BindingExpressionBase>;
}

impl<S: PartialEq + 'static, V: PropertyValue> TypedPropertyElement for Rc<TypedClrPropertyInfo<S, V>> {
    fn value_type(&self) -> ValueType {
        ValueType::of::<V>()
    }

    fn can_set(&self) -> bool {
        IPropertyInfo::can_set(&**self)
    }

    fn create_expression(&self, mode: BindingMode, priority: BindingPriority) -> Rc<dyn BindingExpressionBase> {
        TypedBindingExpression::new(self.clone(), mode, priority)
    }
}

/// The path of a compiled binding: a sequence of typed accessors, built
/// without any lookup by name.
///
/// A path is a reference object: clones refer to the same path and compare
/// by identity.
#[derive(Clone)]
pub struct CompiledBindingPath {
    elements: Rc<[PathElement]>,
}

impl Default for CompiledBindingPath {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for CompiledBindingPath {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.elements, &other.elements)
    }
}

impl CompiledBindingPath {
    /// An empty path: binds to the source itself.
    pub fn new() -> Self {
        Self { elements: Rc::new([]) }
    }

    /// Builds the expression nodes of the path. Returns whether the path is
    /// rooted: starts with an element that selects the binding source.
    pub(crate) fn build_expression(&self, result: &mut Vec<Rc<dyn ExpressionNode>>) -> bool {
        let mut negated = 0;
        let mut is_rooted = false;

        for element in self.elements.iter() {
            let node: Option<Rc<dyn ExpressionNode>> = match element {
                PathElement::Not => {
                    negated += 1;
                    None
                }
                PathElement::FerroProperty { property, accepts_null } => {
                    Some(FerroPropertyAccessorNode::new(property, *accepts_null))
                }
                PathElement::Property { property, accessor_factory, accepts_null }
                | PathElement::TypedProperty { property, accessor_factory, accepts_null, .. } => Some(PropertyAccessorNode::new(
                    property.name(),
                    Rc::new(PropertyInfoAccessorPlugin::new(property.clone(), accessor_factory.clone())),
                    *accepts_null,
                )),
                PathElement::MethodAsCommand { method_name, execute, can_execute, depends_on_properties, inpc } => {
                    let depends: Vec<&str> = depends_on_properties.iter().map(|s| &**s).collect();
                    Some(MethodCommandNode::new(
                        method_name,
                        execute.clone(),
                        can_execute.clone(),
                        &depends,
                        inpc.clone(),
                    ))
                }
                PathElement::Untyped { name, plugin, accepts_null, .. } => {
                    Some(PropertyAccessorNode::new(name, plugin.clone(), *accepts_null))
                }
                PathElement::ArrayElement { indices } => Some(ArrayIndexerNode::new(indices.clone())),
                PathElement::VisualAncestor { ancestor_type, level } => {
                    is_rooted = true;
                    Some(VisualAncestorElementNode::new(*ancestor_type, *level))
                }
                PathElement::Ancestor { ancestor_type, level } => {
                    is_rooted = true;
                    Some(LogicalAncestorElementNode::new(*ancestor_type, *level))
                }
                PathElement::SelfElement => {
                    is_rooted = true;
                    None
                }
                PathElement::ElementName { name_scope, name } => {
                    is_rooted = true;
                    let name_scope = name_scope.upgrade().map(NameScopeRef);
                    Some(NamedElementNode::new(name_scope.as_ref(), name))
                }
                PathElement::Stream(plugin) => Some(StreamNode::new(plugin.clone())),
                PathElement::TypeCast { cast, .. } => Some(FuncTransformNode::new(cast.clone())),
                PathElement::TemplatedParent => {
                    is_rooted = true;
                    Some(TemplatedParentNode::new())
                }
            };
            if let Some(node) = node {
                result.push(node);
            }
        }

        for _ in 0..negated {
            result.push(LogicalNotNode::new());
        }
        is_rooted
    }

    /// The typed element of the path, if the path consists of exactly one
    /// property with typed accessors.
    pub(crate) fn single_typed_element(&self) -> Option<&Rc<dyn TypedPropertyElement>> {
        match &*self.elements {
            [PathElement::TypedProperty { typed, .. }] => Some(typed),
            _ => None,
        }
    }

    /// The elements of the path, in order (the `Elements` of the managed
    /// original, as a read-only view).
    pub fn elements(&self) -> impl ExactSizeIterator<Item = CompiledBindingPathElement<'_>> + '_ {
        self.elements.iter().map(CompiledBindingPathElement)
    }

    /// The element at `index`.
    pub fn element(&self, index: usize) -> Option<CompiledBindingPathElement<'_>> {
        self.elements.get(index).map(CompiledBindingPathElement)
    }

    /// The number of elements in the path.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl fmt::Display for CompiledBindingPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for element in self.elements.iter() {
            let text = match element {
                PathElement::Not => "!".to_string(),
                PathElement::FerroProperty { property, .. } => {
                    format!("{}{}", if first { "" } else { "." }, property.name())
                }
                PathElement::Property { property, .. } | PathElement::TypedProperty { property, .. } => {
                    format!("{}{}", if first { "" } else { "." }, property.name())
                }
                PathElement::MethodAsCommand { method_name, .. } => {
                    format!("{}{}()", if first { "" } else { "." }, method_name)
                }
                PathElement::Untyped { name, kind, .. } => {
                    let is_method = *kind != CompiledBindingPathElementKind::Property;
                    format!("{}{}{}", if first { "" } else { "." }, name, if is_method { "()" } else { "" })
                }
                PathElement::Stream(_) => "^".to_string(),
                PathElement::SelfElement => "$self".to_string(),
                PathElement::Ancestor { ancestor_type, level } => {
                    format!("$parent[{}, {}]", ancestor_type.map_or("", |t| t.name()), level)
                }
                PathElement::VisualAncestor { ancestor_type, level } => {
                    format!("$visualParent[{}, {}]", ancestor_type.map_or("", |t| t.name()), level)
                }
                PathElement::ElementName { name, .. } => format!("#{name}"),
                PathElement::TemplatedParent => "$templatedParent".to_string(),
                PathElement::ArrayElement { indices } => {
                    format!("[{}]", indices.iter().map(i32::to_string).collect::<Vec<_>>().join(","))
                }
                PathElement::TypeCast { type_name, .. } => format!("({type_name})"),
            };
            f.write_str(&text)?;
            first = matches!(element, PathElement::Not);
        }
        Ok(())
    }
}

/// Builds a [`CompiledBindingPath`].
///
/// The builder is a reference object: a shared handle to the elements
/// collected so far. Every method adds an element and returns the builder
/// (another handle to it), so that calls can be chained, as well as made one
/// by one through any clone of the handle. Handles compare by identity.
#[derive(Clone, Default)]
pub struct CompiledBindingPathBuilder {
    elements: Rc<RefCell<Vec<PathElement>>>,
}

impl PartialEq for CompiledBindingPathBuilder {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.elements, &other.elements)
    }
}

impl CompiledBindingPathBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Negates the result of the path.
    pub fn not(&self) -> Self {
        self.elements.borrow_mut().push(PathElement::Not);
        self.clone()
    }

    /// A registered property of an object of the class hierarchy.
    pub fn ferro_property(&self, property: &'static FerroProperty) -> Self {
        self.elements.borrow_mut().push(PathElement::FerroProperty { property, accepts_null: false });
        self.clone()
    }

    /// A registered property of an object of the class hierarchy.
    /// `accepts_null` applies a null-conditional operator to the access: a
    /// null owner produces null instead of an error.
    pub fn ferro_property_with(&self, property: &'static FerroProperty, accepts_null: bool) -> Self {
        self.elements.borrow_mut().push(PathElement::FerroProperty { property, accepts_null });
        self.clone()
    }

    /// A plain property, from its description and the factory of its
    /// accessor.
    pub fn property(&self, info: Rc<dyn IPropertyInfo>, accessor_factory: PropertyAccessorFactory) -> Self {
        self.property_with(info, accessor_factory, false)
    }

    /// A plain property. `accepts_null` applies a null-conditional operator
    /// to the access: a null owner produces null instead of an error.
    pub fn property_with(
        &self,
        info: Rc<dyn IPropertyInfo>,
        accessor_factory: PropertyAccessorFactory,
        accepts_null: bool,
    ) -> Self {
        self.elements.borrow_mut().push(PathElement::Property { property: info, accessor_factory, accepts_null });
        self.clone()
    }

    /// A property with typed accessors on a model type that raises property
    /// change notifications. A binding whose path is this single element,
    /// bound to a target property of exactly type `V`, reads and writes
    /// values without boxing them.
    pub fn typed_property<S: INotifyPropertyChanged + PartialEq + 'static, V: PropertyValue>(
        &self,
        name: &str,
        get: impl Fn(&S) -> V + 'static,
        set: Option<Rc<dyn Fn(&S, V)>>,
    ) -> Self {
        ValueTypes::register_reference::<S>();
        let info = Rc::new(TypedClrPropertyInfo::<S, V>::new(name, Some(Rc::new(get)), set).notifying());
        self.elements.borrow_mut().push(PathElement::TypedProperty {
            property: info.clone(),
            accessor_factory: PropertyInfoAccessorFactory::create_inpc_property_accessor::<S>(),
            accepts_null: false,
            typed: Rc::new(info),
        });
        self.clone()
    }

    /// A read-write property of a model type that raises property change
    /// notifications, from typed accessors.
    pub fn notifying_property<O: INotifyPropertyChanged + PartialEq + 'static, K: PropertyKind>(
        &self,
        name: &str,
        get: impl Fn(&O) -> K::Typed + 'static,
        set: impl Fn(&O, K::Typed) + 'static,
    ) -> Self {
        ValueTypes::register_reference::<O>();
        self.property(
            Rc::new(ClrPropertyInfo::read_write::<O, K>(name, get, set)),
            PropertyInfoAccessorFactory::create_inpc_property_accessor::<O>(),
        )
    }

    /// A read-only property of a model type that raises property change
    /// notifications, from a typed getter.
    pub fn notifying_read_only_property<O: INotifyPropertyChanged + PartialEq + 'static, K: PropertyKind>(
        &self,
        name: &str,
        get: impl Fn(&O) -> K::Typed + 'static,
    ) -> Self {
        ValueTypes::register_reference::<O>();
        self.property(
            Rc::new(ClrPropertyInfo::read_only::<O, K>(name, get)),
            PropertyInfoAccessorFactory::create_inpc_property_accessor::<O>(),
        )
    }

    /// A method used as a command.
    pub fn command<O: 'static>(
        &self,
        method_name: &str,
        execute: impl Fn(&O, Option<&BoxedValue>) + 'static,
        can_execute: Option<Rc<dyn Fn(&O, Option<&BoxedValue>) -> bool>>,
        depends_on_properties: &[&str],
    ) -> Self {
        self.elements.borrow_mut().push(PathElement::MethodAsCommand {
            method_name: method_name.into(),
            execute: Rc::new(move |o, p| {
                if let Some(o) = o.downcast_ref::<O>() {
                    execute(o, p)
                }
            }),
            can_execute: can_execute.map(|f| {
                Rc::new(move |o: &dyn AnyValue, p: Option<&BoxedValue>| o.downcast_ref::<O>().is_some_and(|o| f(o, p)))
                    as CanExecute
            }),
            depends_on_properties: depends_on_properties.iter().map(|s| (*s).into()).collect(),
            inpc: None,
        });
        self.clone()
    }

    /// A method used as a command on an owner that raises property change
    /// notifications: the command's state is re-queried when one of
    /// `depends_on_properties` changes.
    pub fn notifying_command<O: INotifyPropertyChanged + 'static>(
        &self,
        method_name: &str,
        execute: impl Fn(&O, Option<&BoxedValue>) + 'static,
        can_execute: Option<Rc<dyn Fn(&O, Option<&BoxedValue>) -> bool>>,
        depends_on_properties: &[&str],
    ) -> Self {
        self.command::<O>(method_name, execute, can_execute, depends_on_properties);
        if let Some(PathElement::MethodAsCommand { inpc, .. }) = self.elements.borrow_mut().last_mut() {
            *inpc = Some(Rc::new(|v: &dyn AnyValue| v.downcast_ref::<O>().map(|o| o as &dyn INotifyPropertyChanged)));
        }
        self.clone()
    }

    /// Streams the outcome of a task (`^`).
    pub fn stream_task(&self) -> Self {
        self.elements.borrow_mut().push(PathElement::Stream(Rc::new(TaskStreamPlugin)));
        self.clone()
    }

    /// Streams the values of an observable (`^`).
    pub fn stream_observable(&self) -> Self {
        self.elements.borrow_mut().push(PathElement::Stream(Rc::new(ObservableStreamPlugin)));
        self.clone()
    }

    /// Streams values through a custom stream plugin.
    pub fn stream(&self, plugin: Rc<dyn IStreamPlugin>) -> Self {
        self.elements.borrow_mut().push(PathElement::Stream(plugin));
        self.clone()
    }

    /// Roots the path at the binding target itself (`$self`).
    pub fn self_(&self) -> Self {
        self.elements.borrow_mut().push(PathElement::SelfElement);
        self.clone()
    }

    /// Roots the path at an ancestor in the logical tree.
    pub fn ancestor(&self, ancestor_type: Option<&'static TypeInfo>, level: usize) -> Self {
        self.elements.borrow_mut().push(PathElement::Ancestor { ancestor_type, level });
        self.clone()
    }

    /// Roots the path at an ancestor in the visual tree.
    pub fn visual_ancestor(&self, ancestor_type: Option<&'static TypeInfo>, level: usize) -> Self {
        self.elements.borrow_mut().push(PathElement::VisualAncestor { ancestor_type, level });
        self.clone()
    }

    /// Roots the path at a named element.
    pub fn element_name(&self, name_scope: NameScopeRef, name: &str) -> Self {
        let name_scope = std::rc::Rc::downgrade(&name_scope.0);
        self.elements.borrow_mut().push(PathElement::ElementName { name_scope, name: name.into() });
        self.clone()
    }

    /// An indexer with one integer argument (`owner[index]`) on owner type
    /// `O`, from typed accessors that fail for a bad index. The value is
    /// re-read when the owner raises a collection change that affects
    /// `index`, or a property change for the indexer.
    pub fn indexer_property<O: 'static, K: PropertyKind>(
        &self,
        index: i32,
        get: impl Fn(&O, i32) -> Result<K::Typed, BindingError> + 'static,
        set: Option<Rc<dyn Fn(&O, i32, K::Typed) -> Result<(), BindingError>>>,
    ) -> Self {
        let info = match set {
            Some(set) => ClrPropertyInfo::read_write_fallible::<O, K>(
                INDEXER_NAME,
                move |o| get(o, index),
                move |o, v| set(o, index, v),
            ),
            None => ClrPropertyInfo::read_only_fallible::<O, K>(INDEXER_NAME, move |o| get(o, index)),
        };
        self.property(Rc::new(info), PropertyInfoAccessorFactory::create_indexer_property_accessor(index))
    }

    /// The item at `index` of a [`BindableList`] (`list[index]`).
    pub fn list_item<T: PropertyValue>(&self, index: i32) -> Self {
        fn position<T: PropertyValue>(list: &BindableList<T>, index: i32) -> Result<usize, BindingError> {
            usize::try_from(index).ok().filter(|i| *i < list.items().count()).ok_or_else(|| {
                BindingError::message(
                    "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')",
                )
            })
        }
        self.indexer_property::<BindableList<T>, Value<T>>(
            index,
            |list, index| {
                let position = position(list, index)?;
                list.items().try_get(position).ok_or_else(|| BindingError::message("Index was out of range."))
            },
            Some(Rc::new(|list, index, value| {
                let position = position(list, index)?;
                list.items().set(position, value);
                Ok(())
            })),
        )
    }

    /// The entry for `key` of a [`BindableDictionary`] (`dictionary[key]`).
    /// Reading a missing key is an error; writing adds the entry.
    pub fn dictionary_item<K: Eq + Hash + Clone + fmt::Display + 'static, V: PropertyValue>(&self, key: K) -> Self {
        // An integer key is an indexer with one integer argument.
        let index = (&key as &dyn std::any::Any).downcast_ref::<i32>().copied();
        let set_key = key.clone();
        let info = ClrPropertyInfo::read_write_fallible::<BindableDictionary<K, V>, Value<V>>(
            INDEXER_NAME,
            move |dictionary| {
                dictionary.items().try_get_value(&key).ok_or_else(|| {
                    BindingError::message(format!("The given key '{key}' was not present in the dictionary."))
                })
            },
            move |dictionary, value| {
                dictionary.items().set(set_key.clone(), value);
                Ok(())
            },
        );
        let accessor_factory = match index {
            Some(index) => PropertyInfoAccessorFactory::create_indexer_property_accessor(index),
            None => PropertyInfoAccessorFactory::create_inpc_property_accessor::<BindableDictionary<K, V>>(),
        };
        self.property(Rc::new(info), accessor_factory)
    }

    /// Indexes an array with integer indexes, one per dimension.
    pub fn array_element(&self, indices: &[i32]) -> Self {
        self.elements.borrow_mut().push(PathElement::ArrayElement { indices: indices.to_vec() });
        self.clone()
    }

    /// Casts to class `T`: passes an object on if it is a `T`, else null.
    pub fn type_cast<T: ObjectType>(&self) -> Self {
        self.elements.borrow_mut().push(PathElement::TypeCast {
            type_name: T::TYPE.name(),
            cast: Rc::new(|v| {
                v.filter(|v| ValueTypes::as_object(&***v).is_some_and(|o| o.is::<T>())).cloned()
            }),
        });
        self.clone()
    }

    /// Casts to a class, or to a value or model type: passes a value on if
    /// it is an instance of the type ([`CastTarget::is_instance`]), else
    /// null.
    pub fn type_cast_value(&self, target: CastTarget) -> Self {
        let type_name = match target {
            CastTarget::Class(t) => t.name(),
            CastTarget::Value(t) => t.name(),
        };
        let cast: Cast = Rc::new(move |v: Option<&BoxedValue>| v.filter(|v| target.is_instance(v)).cloned());
        self.elements.borrow_mut().push(PathElement::TypeCast { type_name, cast });
        self.clone()
    }

    /// Roots the path at the templated parent of the binding target.
    pub fn templated_parent(&self) -> Self {
        self.elements.borrow_mut().push(PathElement::TemplatedParent);
        self.clone()
    }

    /// A property whose accessors are closures over untyped values: `get`
    /// reads it from the handle of its owner, `set` (if the property can be
    /// written) writes it, and `notifier` (if the owner raises property
    /// change notifications) views the owner as a notifier. `property_type`
    /// is the type of the property's value.
    pub fn property_untyped(
        &self,
        name: &str,
        property_type: ValueType,
        get: UntypedGetter,
        set: Option<UntypedSetter>,
        notifier: Option<AsNotifyPropertyChanged>,
    ) -> Self {
        self.property_untyped_with(name, property_type, get, set, notifier, false)
    }

    /// As [`property_untyped`](Self::property_untyped). `accepts_null`
    /// applies a null-conditional operator to the access: a null owner
    /// produces null instead of an error.
    pub fn property_untyped_with(
        &self,
        name: &str,
        property_type: ValueType,
        get: UntypedGetter,
        set: Option<UntypedSetter>,
        notifier: Option<AsNotifyPropertyChanged>,
        accepts_null: bool,
    ) -> Self {
        self.untyped_member(name, UntypedMember::Property { property_type, get, set, notifier }, accepts_null)
    }

    /// A method used as a command, from closures over the handle of the
    /// owner. The command's state is re-queried when the owner (viewed as a
    /// notifier by `notifier`) reports a change of one of
    /// `depends_on_properties`.
    pub fn command_untyped(
        &self,
        method_name: &str,
        execute: UntypedExecute,
        can_execute: Option<UntypedCanExecute>,
        depends_on_properties: &[&str],
        notifier: Option<AsNotifyPropertyChanged>,
    ) -> Self {
        self.untyped_member(
            method_name,
            UntypedMember::Command {
                execute,
                can_execute,
                depends_on_properties: depends_on_properties.iter().map(|s| (*s).into()).collect(),
                notifier,
            },
            false,
        )
    }

    /// A method as a delegate value: the path element produces the value
    /// `create_delegate` makes for the handle of the owner, usually an
    /// untyped callback ([`MarkupDelegate`](crate::metadata::MarkupDelegate))
    /// that invokes the method on it. `accepts_null` applies a
    /// null-conditional operator to the access: a null owner produces null
    /// instead of an error.
    pub fn method_untyped(&self, name: &str, create_delegate: UntypedCreateDelegate, accepts_null: bool) -> Self {
        self.untyped_member(name, UntypedMember::Method { create_delegate }, accepts_null)
    }

    fn untyped_member(&self, name: &str, member: UntypedMember, accepts_null: bool) -> Self {
        let (kind, property_type) = match &member {
            UntypedMember::Property { property_type, .. } => {
                (CompiledBindingPathElementKind::Property, Some(*property_type))
            }
            UntypedMember::Command { .. } => (CompiledBindingPathElementKind::MethodAsCommand, None),
            UntypedMember::Method { .. } => (CompiledBindingPathElementKind::MethodAsDelegate, None),
        };
        self.elements.borrow_mut().push(PathElement::Untyped {
            name: name.into(),
            kind,
            property_type,
            plugin: Rc::new(UntypedAccessorPlugin::new(name, member)),
            accepts_null,
        });
        self.clone()
    }

    /// A property with typed accessors on a shared model type that raises
    /// NO property change notifications: as
    /// [`typed_property_info_with`](Self::typed_property_info_with), with
    /// the value read once per source (the typed property overload of the
    /// managed original does not require a notifying source).
    pub fn typed_plain_property_info_with<S: PartialEq + 'static, V: PropertyValue>(
        &self,
        info: TypedClrPropertyInfo<S, V>,
        accepts_null: bool,
    ) -> Self {
        ValueTypes::register_reference::<S>();
        let info = Rc::new(info);
        self.elements.borrow_mut().push(PathElement::TypedProperty {
            property: info.clone(),
            accessor_factory: PropertyInfoAccessorFactory::create_plain_property_accessor(),
            accepts_null,
            typed: Rc::new(info),
        });
        self.clone()
    }

    /// The path of the elements added so far.
    pub fn build(&self) -> CompiledBindingPath {
        CompiledBindingPath { elements: self.elements.borrow().clone().into() }
    }

    /// As [`typed_property`](Self::typed_property), from a property
    /// description (which may have a fallible getter).
    pub fn typed_property_info<S: INotifyPropertyChanged + PartialEq + 'static, V: PropertyValue>(
        &self,
        info: TypedClrPropertyInfo<S, V>,
    ) -> Self {
        self.typed_property_info_with(info, false)
    }

    /// As [`typed_property_info`](Self::typed_property_info). `accepts_null`
    /// applies a null-conditional operator to the access (the three
    /// argument `Property<TSource, TValue>` of the managed original).
    pub fn typed_property_info_with<S: INotifyPropertyChanged + PartialEq + 'static, V: PropertyValue>(
        &self,
        info: TypedClrPropertyInfo<S, V>,
        accepts_null: bool,
    ) -> Self {
        ValueTypes::register_reference::<S>();
        let info = Rc::new(info.notifying());
        self.elements.borrow_mut().push(PathElement::TypedProperty {
            property: info.clone(),
            accessor_factory: PropertyInfoAccessorFactory::create_inpc_property_accessor::<S>(),
            accepts_null,
            typed: Rc::new(info),
        });
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::model::Event;
    use crate::{StaticType, StyledElement};

    struct PathVm {
        text: RefCell<String>,
        property_changed: Event<str>,
    }

    impl PartialEq for PathVm {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    impl INotifyPropertyChanged for PathVm {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }

    #[test]
    fn elements_of_a_path_can_be_inspected() {
        use CompiledBindingPathElementKind as Kind;

        let path = CompiledBindingPathBuilder::new()
            .typed_property::<PathVm, String>("Text", |vm| vm.text.borrow().clone(), None)
            .build();
        assert_eq!(path.elements().len(), 1);
        let element = path.element(0).expect("the element");
        assert_eq!(element.kind(), Kind::Property);
        assert!(element.is_typed_property());
        assert_eq!(element.name(), Some("Text"));
        assert_eq!(element.property_type(), Some(ValueType::of::<String>()));
        let property = element.property().expect("the property");
        assert_eq!(property.name(), "Text");
        assert!(!property.can_set());
        assert!(path.element(1).is_none());

        let get: UntypedGetter = Rc::new(|_| Ok(None));
        let path = CompiledBindingPathBuilder::new()
            .not()
            .ancestor(Some(StyledElement::TYPE), 2)
            .ferro_property_with(StyledElement::data_context_property().as_property(), true)
            .notifying_property::<PathVm, Value<String>>("Text", |vm| vm.text.borrow().clone(), |vm, v| {
                vm.text.replace(v);
            })
            .property_untyped("Length", ValueType::of::<i32>(), get, None, None)
            .method_untyped("Run", Rc::new(|owner| owner.clone()), false)
            .command_untyped("Save", Rc::new(|_, _| {}), None, &[], None)
            .command::<PathVm>("Load", |_, _| {}, None, &[])
            .stream_task()
            .array_element(&[1, 2])
            .type_cast::<StyledElement>()
            .build();
        let kinds: Vec<Kind> = path.elements().map(|element| element.kind()).collect();
        assert_eq!(
            kinds,
            [
                Kind::Not,
                Kind::Ancestor,
                Kind::Property,
                Kind::Property,
                Kind::Property,
                Kind::MethodAsDelegate,
                Kind::MethodAsCommand,
                Kind::MethodAsCommand,
                Kind::Stream,
                Kind::ArrayElement,
                Kind::TypeCast,
            ]
        );
        let names: Vec<Option<&str>> = path.elements().map(|element| element.name()).collect();
        assert_eq!(
            names,
            [None, None, Some("DataContext"), Some("Text"), Some("Length"), Some("Run"), Some("Save"), Some("Load"), None, None, None]
        );
        let ancestor = path.element(1).unwrap().ancestor().expect("an ancestor");
        assert!(ancestor.0.is_some_and(|type_| std::ptr::eq(type_, StyledElement::TYPE)));
        assert_eq!(ancestor.1, 2);

        let registered = path.element(2).unwrap();
        assert!(registered.accepts_null() && !registered.is_typed_property());
        assert!(registered.property().and_then(|p| p.as_ferro_property()).is_some());

        let plain = path.element(3).unwrap();
        assert!(!plain.is_typed_property() && !plain.accepts_null());
        assert!(plain.property().is_some_and(|property| property.can_set()));

        let untyped = path.element(4).unwrap();
        assert!(untyped.property().is_none());
        assert_eq!(untyped.property_type(), Some(ValueType::of::<i32>()));
        assert_eq!(path.element(5).unwrap().property_type(), None);
        assert_eq!(path.element(9).unwrap().array_indices(), Some(&[1, 2][..]));
        assert_eq!(path.to_string(), "!$parent[StyledElement, 2].DataContext.Text.Length.Run().Save().Load()^[1,2](StyledElement)");
        assert_eq!(CompiledBindingPath::new().elements().len(), 0);
    }
}
