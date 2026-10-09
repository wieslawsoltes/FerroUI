//! The build-time type system: [`IXamlTypeSystem`] over the type models of a
//! set of crates and the closed table of runtime library types. The
//! counterpart of the run-time type system of the loader
//! (`runtime::type_system::RuntimeTypeSystem`), which projects the same
//! declarations from the registries of the process.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use ferroui_base::metadata::attributes;
use ferroui_markup_xaml_loader::core_table::{
    self, attribute_type_name, CoreBody, CoreKind, CoreMember, CoreRef, CoreType, FERRO_DICTIONARY_DEFINITION, FERRO_LIST_CONVERTER_DEFINITION,
    FERRO_LIST_DEFINITION, KNOWN_ATTRIBUTES, LIST_DEFINITION, PROPERTY_NAMESPACE,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlAssembly, IXamlCustomAttribute, IXamlType, IXamlTypeSystem, XamlPseudoType, XamlTypeWellKnownTypes, XamlValue};

use crate::model::{
    AssemblyModel, AttributeModel, AttributeValueModel, MemberModel, PropertyModel, RegisteredKind, RegisteredModel, RegistrationModel, TypeKind,
    TypeModel,
};
use crate::model_set::ModelSet;

use super::types::{
    substitute, type_key, MemberSource, ModelAssembly, ModelConstructor, ModelCustomAttribute, ModelEvent, ModelField, ModelMembers, ModelMethod,
    ModelProperty, ModelType, ModelTypeKind, ModelTypeOrigin, ModelTypeSpec,
};

/// The position of a type in the set of models: its model and the type in it.
pub type Position = (usize, usize);

/// The type a declaration of the model is part of, when it is not a type of its own.
#[derive(Clone, Debug)]
enum Merged {
    /// Metadata of a runtime library type the type system defines itself is that type.
    Core(String),
    /// Metadata of the same name declared next to a type of the object model (the owner of
    /// attached properties that also is a type with values of its own) is that type.
    Class(Position),
}

/// What is looked up in the models, built once.
#[derive(Default)]
struct Index {
    /// The declaration that states a handle, by the canonical text of the handle.
    handle_owners: HashMap<String, Position>,
    /// The value type or enumeration whose nullable form a text is (`Option<T>`).
    nullable_owners: HashMap<String, Position>,
    /// The first declaration of a full name that is not an instantiation.
    names: HashMap<String, Position>,
    /// The declared instantiations of a generic definition, by the key of the definition.
    generic_index: HashMap<String, Vec<Position>>,
    merged: HashMap<Position, Merged>,
    /// The declarations merged into a runtime library type, by its full name.
    core_markup: HashMap<String, Vec<Position>>,
    /// The declarations merged into a type of the object model.
    companions: HashMap<Position, Vec<Position>>,
    /// The registered properties of a type of the object model: the ones whose
    /// registration names the type as their owner, whichever type has the accessor, each
    /// by the type it is listed under and its position there, in the order of the models.
    properties: HashMap<Position, Vec<(Position, usize)>>,
    /// The casts the crates register, by the texts of the two types.
    casts: HashSet<(String, String)>,
}

/// The build-time type system.
///
/// The shells of the types of the models are created with the type system, in the order
/// of the models; a type costs its name until one of its members is asked for.
pub struct ModelTypeSystem {
    this: Weak<ModelTypeSystem>,
    set: ModelSet,
    index: Index,
    /// The canonical path of the handle of the classes of the object model (`Ref`).
    ref_path: String,
    types: RefCell<HashMap<String, Rc<ModelType>>>,
    by_model: RefCell<HashMap<Position, Rc<ModelType>>>,
    by_handle: RefCell<HashMap<String, Rc<dyn IXamlType>>>,
    assemblies: RefCell<Vec<Rc<ModelAssembly>>>,
    /// The definitions whose instantiations need not be declared.
    instantiable: RefCell<HashSet<String>>,
    /// The instantiations whose type arguments are being resolved.
    creating: RefCell<HashSet<Position>>,
    well_known_types: OnceCell<Rc<XamlTypeWellKnownTypes>>,
}

fn full_name_of(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{namespace}.{name}")
    }
}

fn instantiation_key(definition: &Rc<ModelType>, arguments: &[Rc<dyn IXamlType>]) -> String {
    let arguments: Vec<String> = arguments.iter().map(type_key).collect();
    format!("{}<{}>", definition.key(), arguments.join(","))
}

fn kind_of(kind: TypeKind) -> ModelTypeKind {
    match kind {
        TypeKind::Class | TypeKind::Static => ModelTypeKind::Class,
        TypeKind::Struct => ModelTypeKind::Struct,
        TypeKind::Enum => ModelTypeKind::Enum,
        TypeKind::Interface => ModelTypeKind::Interface,
    }
}

impl ModelTypeSystem {
    /// The type system over `models`: the model of a crate and the models of the crates
    /// it is built on. Of two types with one full name the first is the type of the name.
    pub fn new(models: Vec<AssemblyModel>) -> Rc<Self> {
        let set = ModelSet::new(models);
        let ref_path = set.canonical_path("::ferroui_base::Ref");
        let core_names: HashSet<String> = core_table::core_types().iter().map(CoreType::full_name).collect();
        let mut index = Index::default();
        for (model_index, model) in set.models().iter().enumerate() {
            for (type_index, type_) in model.types.iter().enumerate() {
                let position = (model_index, type_index);
                let full_name = type_.full_name();
                if !type_.object_model && type_.generic.is_none() {
                    let class = model.types.iter().position(|other| {
                        other.object_model && !other.unregistered && other.name == type_.name && other.namespace == type_.namespace && other.module == type_.module
                    });
                    if type_.namespace.starts_with("System") && core_names.contains(&full_name) {
                        index.merged.insert(position, Merged::Core(full_name.clone()));
                        index.core_markup.entry(full_name.clone()).or_default().push(position);
                    } else if let Some(class) = class {
                        index.merged.insert(position, Merged::Class((model_index, class)));
                        index.companions.entry((model_index, class)).or_default().push(position);
                    }
                }
                for handle in handles_of(&set, &ref_path, type_) {
                    index.handle_owners.entry(handle).or_insert(position);
                }
                // The nullable form of a value type or an enumeration (`Option<T>`) is a
                // type of its own, the `Nullable<T>` of the managed original.
                if matches!(type_.kind, TypeKind::Struct | TypeKind::Enum) {
                    index.nullable_owners.entry(format!("Option<{}>", set.expanded(&type_.rust_path.text))).or_insert(position);
                }
                match &type_.generic {
                    Some(generic) => index.generic_index.entry(full_name_of(&type_.namespace, &generic.definition)).or_default().push(position),
                    // A class the crate does not register is not known by its name.
                    None if type_.unregistered => {}
                    None => {
                        index.names.entry(full_name).or_insert(position);
                    }
                }
                // The registry lists a property under the owner its registration names.
                for (registered_index, registered) in type_.registered.iter().enumerate() {
                    let named = registered.owner.as_ref().and_then(|owner| set.position_of_rust_type(&owner.text));
                    let owner = named.filter(|owner| set.type_at(*owner).object_model).unwrap_or(position);
                    index.properties.entry(owner).or_default().push((position, registered_index));
                }
            }
        }
        // The handles a crate registers for a type with markup metadata, after the ones the
        // declarations state: a handle that is already known keeps its type.
        for model in set.models() {
            for cast in model.casts.iter().filter(|cast| cast.from.is_resolved() && cast.to.is_resolved()) {
                index.casts.insert((set.expanded(&cast.from.text), set.expanded(&cast.to.text)));
            }
            for handle in &model.handles {
                if let Some(position) = set.position_of_rust_type(&handle.type_.text) {
                    index.handle_owners.entry(set.expanded(&handle.handle.text)).or_insert(position);
                }
            }
        }
        let system = Rc::new_cyclic(|this| ModelTypeSystem {
            this: this.clone(),
            set,
            index,
            ref_path,
            types: RefCell::new(HashMap::new()),
            by_model: RefCell::new(HashMap::new()),
            by_handle: RefCell::new(HashMap::new()),
            assemblies: RefCell::new(Vec::new()),
            instantiable: RefCell::new(HashSet::new()),
            creating: RefCell::new(HashSet::new()),
            well_known_types: OnceCell::new(),
        });
        system.define_assemblies();
        system.define_core_types();
        for model_index in 0..system.set.models().len() {
            for type_index in 0..system.set.models()[model_index].types.len() {
                let _ = system.model_type((model_index, type_index));
            }
        }
        system.define_property_types();
        system
    }

    /// This type system as a type system handle.
    pub fn as_type_system(&self) -> Rc<dyn IXamlTypeSystem> {
        match self.this.upgrade() {
            Some(this) => this,
            None => unreachable!("a type system is only used through its handle"),
        }
    }

    /// The models the type system is over.
    pub fn models(&self) -> &ModelSet {
        &self.set
    }

    /// The type named `full_name`, or the unknown pseudo type.
    pub fn get(&self, full_name: &str) -> Rc<dyn IXamlType> {
        match self.find_model_type(full_name) {
            Some(found) => found,
            None => XamlPseudoType::unknown(),
        }
    }

    /// The type the declaration at `position` of the models is (or is part of).
    pub fn type_of_model(&self, position: Position) -> Rc<dyn IXamlType> {
        self.model_type(position)
    }

    /// Every declaration of the model at `model`, with the type it is (or is part of).
    pub fn types_of_model(&self, model: usize) -> Vec<(&TypeModel, Rc<ModelType>)> {
        self.set.models()[model].types.iter().enumerate().map(|(type_index, declared)| (declared, self.model_type((model, type_index)))).collect()
    }

    // --- assemblies ---------------------------------------------------------

    fn define_assemblies(&self) {
        let mut assemblies = vec![Rc::new(ModelAssembly {
            system: self.this.clone(),
            name: "System.Runtime".to_string(),
            aliases: vec!["mscorlib", "netstandard", "System.Private.CoreLib", "System.Collections", "System.Collections.NonGeneric"],
            crate_name: None,
            model: None,
            attributes: RefCell::new(None),
        })];
        for (model_index, model) in self.set.models().iter().enumerate() {
            assemblies.push(Rc::new(ModelAssembly {
                system: self.this.clone(),
                // A crate that states no assembly is the assembly named after the crate.
                name: if model.name.is_empty() { model.crate_name.clone() } else { model.name.clone() },
                aliases: Vec::new(),
                crate_name: Some(model.crate_name.clone()),
                model: Some(model_index),
                attributes: RefCell::new(None),
            }));
        }
        *self.assemblies.borrow_mut() = assemblies;
    }

    pub(crate) fn core_assembly(&self) -> Rc<ModelAssembly> {
        self.assemblies.borrow()[0].clone()
    }

    fn assembly_of_model(&self, model: usize) -> Rc<ModelAssembly> {
        self.assemblies.borrow()[model + 1].clone()
    }

    /// The assembly the synthetic framework types (attribute and property definition
    /// types) belong to.
    fn framework_assembly(&self) -> Rc<ModelAssembly> {
        let found = self.assemblies.borrow().iter().find(|assembly| assembly.name == "FerroUI.Base").cloned();
        found.unwrap_or_else(|| self.core_assembly())
    }

    pub(crate) fn type_belongs_to(&self, type_: &ModelType, assembly: &ModelAssembly) -> bool {
        match type_.model_assembly() {
            Some(own) => std::ptr::eq(&*own, assembly) || (own.crate_name.is_some() && own.crate_name == assembly.crate_name),
            None => false,
        }
    }

    /// The custom attributes of an assembly: its xmlns definitions.
    pub(crate) fn assembly_attributes(&self, assembly: &ModelAssembly) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        let Some(model) = assembly.model else { return Vec::new() };
        let xmlns_definition = self.attribute_type("XmlnsDefinition");
        self.set.models()[model]
            .xmlns_definitions
            .iter()
            .map(|definition| {
                ModelCustomAttribute::new(
                    xmlns_definition.clone(),
                    vec![XamlValue::String(definition.xml_namespace.clone()), XamlValue::String(definition.namespace.clone())],
                    Vec::new(),
                )
            })
            .collect()
    }

    // --- type shells --------------------------------------------------------

    fn insert(&self, type_: &Rc<ModelType>, assembly: &Rc<ModelAssembly>) {
        type_.set_assembly(assembly);
        // The first type of a name (or the first declared instantiation) is the type.
        self.types.borrow_mut().entry(type_.key().to_string()).or_insert_with(|| type_.clone());
    }

    fn unique_key(&self, full_name: String, namespace: &str, module_path: &str, name: &str) -> String {
        if namespace.is_empty() || self.types.borrow().contains_key(&full_name) {
            format!("rust:{module_path}::{name}")
        } else {
            full_name
        }
    }

    fn map_handle(&self, handle: &str, type_: &Rc<dyn IXamlType>) {
        if let Some(model) = type_.as_any().downcast_ref::<ModelType>() {
            model.add_handle(handle);
        }
        self.by_handle.borrow_mut().insert(handle.to_string(), type_.clone());
    }

    /// Defines a type of the type system itself.
    fn define_synthetic(
        &self,
        assembly: &Rc<ModelAssembly>,
        namespace: &str,
        name: &str,
        kind: ModelTypeKind,
        generic_parameters: &[String],
        init: impl FnOnce(&mut MemberBuilder) + 'static,
    ) -> Rc<ModelType> {
        let key = full_name_of(namespace, name);
        let mut spec = ModelTypeSpec::new(key, namespace, name, kind, ModelTypeOrigin::Synthetic);
        spec.generic_parameter_names = generic_parameters.to_vec();
        let system = self.this.clone();
        spec.init = Some(Box::new(move |type_: &Rc<ModelType>| {
            let Some(system) = system.upgrade() else { return ModelMembers::default() };
            let mut builder = MemberBuilder { system, type_: type_.clone(), members: ModelMembers::default(), projected: false };
            if kind == ModelTypeKind::Class && type_.key() != "System.Object" {
                builder.members.base_type = builder.system.find_type("System.Object");
            }
            // One source: a runtime library type the framework declares metadata for
            // (`ISupportInitialize`, `IServiceProvider`, `Uri`, ...) has the members of
            // that metadata; the members of the table only fill in what it lacks.
            let type_namespace = type_.namespace().unwrap_or_default();
            if type_namespace.starts_with("System") && type_.generic_parameter_types().is_empty() {
                builder.project_metadata();
            }
            init(&mut builder);
            builder.members
        }));
        let type_ = ModelType::create(&self.this, spec);
        self.insert(&type_, assembly);
        if !generic_parameters.is_empty() {
            self.instantiable.borrow_mut().insert(type_.key().to_string());
        }
        type_
    }

    /// Defines the runtime library types from the table the run-time type system defines
    /// them from, and maps the Rust types the table lists to them.
    fn define_core_types(&self) {
        let core = self.core_assembly();
        for description in core_table::core_types() {
            let kind = match description.kind {
                CoreKind::Class => ModelTypeKind::Class,
                CoreKind::Interface => ModelTypeKind::Interface,
                CoreKind::Struct => ModelTypeKind::Struct,
            };
            let namespace = description.namespace.clone();
            let name = description.name.clone();
            let parameters = description.parameters.clone();
            self.define_synthetic(&core, &namespace, &name, kind, &parameters, move |builder| apply(builder, &description, None));
        }
        for (text, reference) in core_table::core_handles() {
            if let Some(type_) = self.core_reference(&reference) {
                self.map_handle(&self.set.expanded(&text), &type_);
            }
        }
    }

    /// The type a reference of the table names, outside the definition of a type.
    fn core_reference(&self, reference: &CoreRef) -> Option<Rc<dyn IXamlType>> {
        match reference {
            CoreRef::Type(full_name) => self.find_type(full_name),
            CoreRef::Generic(definition, arguments) => {
                let arguments: Option<Vec<Rc<dyn IXamlType>>> = arguments.iter().map(|argument| self.core_reference(argument)).collect();
                self.find_type(definition)?.make_generic_type(&arguments?).ok()
            }
            CoreRef::Array(element) => self.core_reference(element)?.make_array_type(1).ok(),
            CoreRef::Parameter(_) | CoreRef::Element => None,
        }
    }

    /// Defines the types of property definitions (`FerroUI.FerroProperty` and the generic
    /// `StyledProperty<T>`, `AttachedProperty<T>`, `DirectProperty<TOwner, T>`), unless
    /// types with these names are declared.
    fn define_property_types(&self) {
        let descriptions = core_table::property_types();
        if descriptions.iter().any(|description| description.name == "StyledProperty`1" && self.find_type(&description.full_name()).is_some()) {
            return;
        }
        let assembly = self.framework_assembly();
        for description in descriptions {
            if self.find_type(&description.full_name()).is_some() {
                continue;
            }
            let is_definition = description.parameters.is_empty();
            let namespace = description.namespace.clone();
            let name = description.name.clone();
            let parameters = description.parameters.clone();
            let type_: Rc<dyn IXamlType> =
                self.define_synthetic(&assembly, &namespace, &name, ModelTypeKind::Class, &parameters, move |builder| apply(builder, &description, None));
            if is_definition {
                for handle in core_table::PROPERTY_HANDLES {
                    self.map_handle(&self.set.expanded(handle), &type_);
                }
            }
        }
    }

    /// The type the declaration at `position` is, or is part of.
    fn model_type(&self, position: Position) -> Rc<ModelType> {
        if let Some(found) = self.by_model.borrow().get(&position) {
            return found.clone();
        }
        let declared = self.set.type_at(position);
        let target = match self.index.merged.get(&position) {
            Some(Merged::Core(full_name)) => self.types.borrow().get(full_name).cloned(),
            Some(Merged::Class(class)) => Some(self.model_type(*class)),
            None => None,
        };
        if let Some(target) = target {
            let as_type: Rc<dyn IXamlType> = target.clone();
            for handle in handles_of(&self.set, &self.ref_path, declared) {
                target.add_handle(&handle);
                self.by_handle.borrow_mut().insert(handle, as_type.clone());
            }
            self.by_model.borrow_mut().insert(position, target.clone());
            return target;
        }
        let kind = kind_of(declared.kind);
        let origin = ModelTypeOrigin::Model { model: position.0, type_: position.1 };
        let namespace = declared.namespace.as_str();
        let resolving = self.creating.borrow().contains(&position);
        let mut spec = match &declared.generic {
            Some(generic) if !resolving => {
                self.creating.borrow_mut().insert(position);
                let definition = self.definition_for(namespace, &generic.definition, kind, position.0);
                let arguments: Vec<Rc<dyn IXamlType>> = generic.arguments.iter().map(|argument| self.resolve(&argument.text)).collect();
                self.creating.borrow_mut().remove(&position);
                // A type argument that names the instantiation itself created it.
                if let Some(found) = self.by_model.borrow().get(&position) {
                    return found.clone();
                }
                let key = instantiation_key(&definition, &arguments);
                let mut spec = ModelTypeSpec::new(key, namespace, &generic.definition, kind, origin);
                spec.generic_definition = Some(definition);
                spec.generic_arguments = arguments;
                spec
            }
            // A class the crate does not register has its name and is not found by it.
            _ if declared.unregistered => {
                ModelTypeSpec::new(format!("rust:{}::{}", declared.module, declared.name), namespace, &declared.name, kind, origin)
            }
            _ => {
                let key = self.unique_key(declared.full_name(), namespace, &declared.module, &declared.name);
                ModelTypeSpec::new(key, namespace, &declared.name, kind, origin)
            }
        };
        spec.handles = handles_of(&self.set, &self.ref_path, declared);
        let type_ = ModelType::create(&self.this, spec);
        self.by_model.borrow_mut().insert(position, type_.clone());
        self.insert(&type_, &self.assembly_of_model(position.0));
        let as_type: Rc<dyn IXamlType> = type_.clone();
        for handle in type_.handles() {
            self.by_handle.borrow_mut().entry(handle).or_insert_with(|| as_type.clone());
        }
        type_
    }

    /// The type of a Rust type no model declares: assignable only to itself and to
    /// `System.Object`.
    fn opaque(&self, text: &str) -> Rc<dyn IXamlType> {
        let key = format!("rust:{text}");
        if let Some(found) = self.types.borrow().get(&key) {
            return found.clone();
        }
        let mut spec = ModelTypeSpec::new(key, "", text, ModelTypeKind::Class, ModelTypeOrigin::Opaque(text.to_string()));
        spec.handles = vec![text.to_string()];
        let type_ = ModelType::create(&self.this, spec);
        self.insert(&type_, &self.core_assembly());
        type_
    }

    /// The generic definition named `name` in `namespace`: a definition of the type
    /// system, or one synthesised for declared instantiations.
    fn definition_for(&self, namespace: &str, name: &str, kind: ModelTypeKind, model: usize) -> Rc<ModelType> {
        let key = full_name_of(namespace, name);
        if let Some(found) = self.types.borrow().get(&key) {
            return found.clone();
        }
        let arity = name.rsplit_once('`').and_then(|(_, arity)| arity.parse::<usize>().ok()).unwrap_or(1);
        let mut spec = ModelTypeSpec::new(key, namespace, name, kind, ModelTypeOrigin::Synthetic);
        spec.generic_parameter_names = (1..=arity).map(|index| format!("T{index}")).collect();
        let system = self.this.clone();
        spec.init = Some(Box::new(move |_| ModelMembers {
            base_type: match kind {
                ModelTypeKind::Class => system.upgrade().and_then(|system| system.find_type("System.Object")),
                _ => None,
            },
            ..ModelMembers::default()
        }));
        let type_ = ModelType::create(&self.this, spec);
        self.insert(&type_, &self.assembly_of_model(model));
        type_
    }

    /// A type the type system synthesizes from a description of the table: an
    /// instantiation of `definition` with the members the description states.
    fn synthesize(&self, key: String, definition: &Rc<ModelType>, arguments: &[Rc<dyn IXamlType>], description: CoreType) -> Rc<dyn IXamlType> {
        let mut spec = ModelTypeSpec::new(
            key,
            &definition.namespace().unwrap_or_default(),
            &definition.name(),
            ModelTypeKind::Class,
            ModelTypeOrigin::Synthetic,
        );
        spec.generic_definition = Some(definition.clone());
        spec.generic_arguments = arguments.to_vec();
        let system = self.this.clone();
        let element_type = arguments[0].clone();
        spec.init = Some(Box::new(move |type_: &Rc<ModelType>| {
            let Some(system) = system.upgrade() else { return ModelMembers::default() };
            let mut builder = MemberBuilder { system, type_: type_.clone(), members: ModelMembers::default(), projected: false };
            apply(&mut builder, &description, Some(&element_type));
            builder.members
        }));
        let type_ = ModelType::create(&self.this, spec);
        let assembly = definition.model_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        type_
    }

    /// `definition.MakeGenericType(arguments)`: the declared instantiation, or a synthetic
    /// one for a definition of the type system.
    pub(crate) fn instantiate(&self, definition: &Rc<ModelType>, arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlType>> {
        let key = instantiation_key(definition, arguments);
        if let Some(found) = self.types.borrow().get(&key) {
            return Ok(found.clone());
        }
        // The list converter is synthesized for every element type.
        if definition.key() == FERRO_LIST_CONVERTER_DEFINITION && arguments.len() == 1 {
            return Ok(self.synthesize(key, definition, arguments, core_table::list_converter_members()));
        }
        let candidates = self.index.generic_index.get(definition.key()).cloned().unwrap_or_default();
        for candidate in candidates {
            let _ = self.model_type(candidate);
        }
        if let Some(found) = self.types.borrow().get(&key) {
            return Ok(found.clone());
        }
        // The list of the runtime library is created by markup for every element type: an
        // instantiation no metadata declares has the members of the list markup creates.
        if definition.key() == LIST_DEFINITION && arguments.len() == 1 {
            return Ok(self.synthesize(key, definition, arguments, core_table::list_members(true)));
        }
        if !self.instantiable.borrow().contains(definition.key()) {
            return Err(XamlError::type_system_exception(format!(
                "The instantiation {}[{}] is not declared: the build-time type system only knows the instantiations of a generic type that declare markup metadata",
                definition.full_name(),
                arguments.iter().map(|argument| argument.full_name()).collect::<Vec<_>>().join(",")
            )));
        }
        let mut spec = ModelTypeSpec::new(
            key,
            &definition.namespace().unwrap_or_default(),
            &definition.name(),
            definition.kind(),
            ModelTypeOrigin::Instantiation,
        );
        spec.generic_definition = Some(definition.clone());
        spec.generic_arguments = arguments.to_vec();
        let type_ = ModelType::create(&self.this, spec);
        let assembly = definition.model_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        Ok(type_)
    }

    /// The open instantiation of a generic definition whose instantiations are otherwise
    /// the declared ones only: the definition applied to type parameters (of a generic
    /// method), deriving from `base`.
    fn instantiate_open(&self, definition: &Rc<ModelType>, arguments: &[Rc<dyn IXamlType>], base: Rc<dyn IXamlType>) -> Rc<dyn IXamlType> {
        let key = instantiation_key(definition, arguments);
        if let Some(found) = self.types.borrow().get(&key) {
            return found.clone();
        }
        let mut spec = ModelTypeSpec::new(
            key,
            &definition.namespace().unwrap_or_default(),
            &definition.name(),
            definition.kind(),
            ModelTypeOrigin::Synthetic,
        );
        spec.generic_definition = Some(definition.clone());
        spec.generic_arguments = arguments.to_vec();
        spec.init = Some(Box::new(move |_| ModelMembers { base_type: Some(base), ..ModelMembers::default() }));
        let type_ = ModelType::create(&self.this, spec);
        let assembly = definition.model_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        type_
    }

    pub(crate) fn create_array_type(&self, element: &Rc<ModelType>, dimensions: i32) -> Rc<ModelType> {
        let suffix = format!("[{}]", ",".repeat((dimensions - 1) as usize));
        let mut spec = ModelTypeSpec::new(
            format!("{}{suffix}", element.key()),
            &element.namespace().unwrap_or_default(),
            &format!("{}{suffix}", element.name()),
            ModelTypeKind::Class,
            ModelTypeOrigin::Array,
        );
        let element_type: Rc<dyn IXamlType> = element.clone();
        spec.array_element_type = Some(element_type.clone());
        let system = self.this.clone();
        spec.init = Some(Box::new(move |_| {
            let mut members = ModelMembers::default();
            let Some(system) = system.upgrade() else { return members };
            members.base_type = system.find_type("System.Array");
            if dimensions == 1 {
                // A vector implements the generic list interfaces of its element type.
                for name in ["System.Collections.Generic.IList`1", "System.Collections.Generic.IReadOnlyList`1"] {
                    if let Some(Ok(interface)) = system.find_type(name).map(|interface| interface.make_generic_type(std::slice::from_ref(&element_type))) {
                        members.interfaces.push(interface);
                    }
                }
            }
            members.interfaces.extend(system.find_type("System.Collections.IList"));
            members
        }));
        let type_ = ModelType::create(&self.this, spec);
        let assembly = element.model_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        type_
    }

    // --- lookups ------------------------------------------------------------

    /// The type that values of the Rust type `rust_type` (normalised type text, in any
    /// spelling of its paths and through any type alias of the models) are values of
    /// (docs/porting/xaml.md, 9.5.2): a runtime library type, a class (for its handle and
    /// its optional handle), a type with markup metadata (the optional form of a value
    /// type is `System.Nullable<T>`), or else an opaque type that carries the text.
    pub fn resolve(&self, rust_type: &str) -> Rc<dyn IXamlType> {
        let text = self.set.expanded(rust_type);
        if let Some(found) = self.by_handle.borrow().get(&text) {
            return found.clone();
        }
        let found: Option<Rc<dyn IXamlType>> = if let Some(owner) = self.index.handle_owners.get(&text) {
            Some(self.model_type(*owner))
        } else if let Some(owner) = self.index.nullable_owners.get(&text) {
            let inner: Rc<dyn IXamlType> = self.model_type(*owner);
            self.nullable_of(&inner).inspect(|nullable| {
                if let Some(model) = nullable.as_any().downcast_ref::<ModelType>() {
                    model.add_handle(&text);
                }
            })
        } else {
            self.resolve_optional_reference(&text)
        };
        match found {
            Some(found) => {
                self.by_handle.borrow_mut().insert(text, found.clone());
                found
            }
            None => self.opaque(&text),
        }
    }

    /// `Option<T>` of a reference type whose metadata lists only `T` among its handles: an
    /// optional reference is the type itself.
    fn resolve_optional_reference(&self, text: &str) -> Option<Rc<dyn IXamlType>> {
        let inner = text.strip_prefix("Option<")?.strip_suffix('>')?;
        let owner = *self.index.handle_owners.get(inner)?;
        let declared = self.set.type_at(owner);
        if declared.object_model || !matches!(declared.kind, TypeKind::Class | TypeKind::Interface) {
            return None;
        }
        let type_ = self.model_type(owner);
        type_.add_handle(text);
        Some(type_)
    }

    /// Whether a value of the Rust type `from` is a value of the Rust type `to`: the two
    /// are one type, or a crate registers the cast between them.
    pub fn is_cast(&self, from: &str, to: &str) -> bool {
        let (from, to) = (self.set.expanded(from), self.set.expanded(to));
        from == to || self.index.casts.contains(&(from, to))
    }

    /// `System.Nullable<T>` of a value type.
    pub fn nullable_of(&self, type_: &Rc<dyn IXamlType>) -> Option<Rc<dyn IXamlType>> {
        self.find_type("System.Nullable`1")?.make_generic_type(std::slice::from_ref(type_)).ok()
    }

    pub(crate) fn find_model_type(&self, full_name: &str) -> Option<Rc<ModelType>> {
        if let Some(found) = self.types.borrow().get(full_name) {
            return Some(found.clone());
        }
        if let Some(position) = self.index.names.get(full_name) {
            return Some(self.model_type(*position));
        }
        let name = full_name.rsplit_once('.').map_or(full_name, |(_, name)| name);
        if name.contains('`') {
            if let Some(first) = self.index.generic_index.get(full_name).and_then(|instantiations| instantiations.first()) {
                // The instantiation creates the definition it belongs to.
                return self.model_type(*first).definition().cloned();
            }
        }
        let attribute = name.strip_suffix("Attribute").filter(|attribute| KNOWN_ATTRIBUTES.contains(attribute) && attribute_type_name(attribute) == full_name)?;
        self.attribute_model_type(attribute)
    }

    fn attribute_model_type(&self, name: &str) -> Option<Rc<ModelType>> {
        let full_name = attribute_type_name(name);
        if let Some(found) = self.types.borrow().get(&full_name) {
            return Some(found.clone());
        }
        if let Some(position) = self.index.names.get(&full_name) {
            return Some(self.model_type(*position));
        }
        let (namespace, type_name) = full_name.rsplit_once('.').unwrap_or(("", &full_name));
        let assembly = if namespace.starts_with("System") { self.core_assembly() } else { self.framework_assembly() };
        Some(self.define_synthetic(&assembly, namespace, type_name, ModelTypeKind::Class, &[], |builder| builder.base("System.Attribute")))
    }

    /// The type of the attribute a metadata attribute named `name` is projected to;
    /// defined by the type system if no such type is declared.
    pub fn attribute_type(&self, name: &str) -> Rc<dyn IXamlType> {
        match self.attribute_model_type(name) {
            Some(found) => found,
            None => XamlPseudoType::unknown(),
        }
    }

    /// Projects an attribute of the model.
    pub fn project_attribute(&self, attribute: &AttributeModel) -> Rc<dyn IXamlCustomAttribute> {
        let value = |value: &AttributeValueModel| self.project_attribute_value(value);
        ModelCustomAttribute::new(
            self.attribute_type(&attribute.name),
            attribute.arguments.iter().map(value).collect(),
            attribute.properties.iter().map(|(name, argument)| (name.clone(), value(argument))).collect(),
        )
    }

    /// Projects an argument of an attribute: an array is an array-valued argument, as
    /// reflection reports `new[] { .. }`.
    fn project_attribute_value(&self, value: &AttributeValueModel) -> XamlValue {
        match value {
            AttributeValueModel::Null => XamlValue::Null,
            AttributeValueModel::Bool(value) => XamlValue::Boolean(*value),
            AttributeValueModel::Int(value) => match i32::try_from(*value) {
                Ok(value) => XamlValue::Int32(value),
                Err(_) => XamlValue::Int64(*value),
            },
            // The literal as written; a text that is not a number is not an argument the
            // declaration macros accept.
            AttributeValueModel::Float(text) => XamlValue::Double(text.replace('_', "").trim_end_matches("f64").trim_end_matches("f32").parse().unwrap_or(f64::NAN)),
            AttributeValueModel::Str(value) => XamlValue::String(value.clone()),
            AttributeValueModel::Type(type_) => XamlValue::Type(self.resolve(&type_.text)),
            AttributeValueModel::Array(items) => XamlValue::Array(items.iter().map(|item| self.project_attribute_value(item)).collect()),
        }
    }

    fn project_attributes(&self, attributes: &[AttributeModel]) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        attributes.iter().map(|attribute| self.project_attribute(attribute)).collect()
    }

    fn marker_attribute(&self, name: &str) -> Rc<dyn IXamlCustomAttribute> {
        ModelCustomAttribute::new(self.attribute_type(name), Vec::new(), Vec::new())
    }

    fn void(&self) -> Rc<dyn IXamlType> {
        self.get("System.Void")
    }

    fn method(
        &self,
        type_: &Rc<ModelType>,
        name: String,
        is_static: bool,
        return_type: Rc<dyn IXamlType>,
        parameters: Vec<Rc<dyn IXamlType>>,
        attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
        source: MemberSource,
    ) -> Rc<ModelMethod> {
        Rc::new(ModelMethod {
            name,
            declaring_type: Rc::downgrade(type_),
            is_static,
            return_type,
            parameters,
            attributes,
            generic_parameters: Vec::new(),
            generic_arguments: Vec::new(),
            is_abstract: false,
            source,
        })
    }

    fn resolve_all(&self, parameters: &[crate::model::ParameterModel]) -> Vec<Rc<dyn IXamlType>> {
        parameters.iter().map(|parameter| self.resolve(&parameter.type_.text)).collect()
    }

    // --- projection ---------------------------------------------------------

    pub(crate) fn project_members(&self, type_: &Rc<ModelType>) -> ModelMembers {
        let object = || self.find_type("System.Object");
        match type_.origin() {
            ModelTypeOrigin::Model { model, type_: type_index } => {
                let position = (*model, *type_index);
                if self.set.type_at(position).object_model {
                    self.project_class(type_, position)
                } else {
                    self.project_markup_type(type_, position)
                }
            }
            ModelTypeOrigin::Instantiation => self.project_instantiation(type_),
            ModelTypeOrigin::Opaque(_) => ModelMembers { base_type: object(), ..ModelMembers::default() },
            ModelTypeOrigin::Synthetic if type_.kind() == ModelTypeKind::Class => ModelMembers { base_type: object(), ..ModelMembers::default() },
            _ => ModelMembers::default(),
        }
    }

    fn project_instantiation(&self, type_: &Rc<ModelType>) -> ModelMembers {
        let Some(definition) = type_.definition().cloned() else { return ModelMembers::default() };
        let source = definition.members();
        let parameters = definition.generic_parameter_types();
        let arguments = type_.generic_argument_types();
        let weak = Rc::downgrade(type_);
        let sub = |type_: &Rc<dyn IXamlType>| substitute(type_, parameters, arguments);
        ModelMembers {
            base_type: source.base_type.as_ref().map(sub),
            interfaces: source.interfaces.iter().map(sub).collect(),
            properties: source
                .properties
                .iter()
                .map(|property| {
                    Rc::new(ModelProperty {
                        name: property.name.clone(),
                        declaring_type: weak.clone(),
                        property_type: sub(&property.property_type),
                        getter: property.getter.as_ref().map(|method| method.substituted(&weak, parameters, arguments)),
                        setter: property.setter.as_ref().map(|method| method.substituted(&weak, parameters, arguments)),
                        attributes: property.attributes.clone(),
                        indexer_parameters: property.indexer_parameters.iter().map(sub).collect(),
                        registered: property.registered.clone(),
                    })
                })
                .collect(),
            events: Vec::new(),
            fields: source
                .fields
                .iter()
                .map(|field| {
                    Rc::new(ModelField {
                        name: field.name.clone(),
                        declaring_type: weak.clone(),
                        field_type: sub(&field.field_type),
                        literal: field.literal.clone(),
                        attributes: field.attributes.clone(),
                        source: field.source.clone(),
                    })
                })
                .collect(),
            methods: source.methods.iter().map(|method| method.substituted(&weak, parameters, arguments)).collect(),
            constructors: source.constructors.iter().map(|constructor| constructor.substituted(&weak, parameters, arguments)).collect(),
            attributes: source.attributes.clone(),
            enum_underlying_type: None,
        }
    }

    /// Projects what the markup metadata of `declared` states into `members`.
    fn project_markup(&self, type_: &Rc<ModelType>, declared: &TypeModel, members: &mut ModelMembers) {
        let weak = Rc::downgrade(type_);
        let self_type: Rc<dyn IXamlType> = type_.clone();
        let type_path = declared.named_path().to_string();
        let source_of = |member: &MemberModel| MemberSource::Declared {
            type_path: type_path.clone(),
            callable: member.callable.clone(),
            typed_function: member.typed_function.clone(),
            fallible: member.fallible,
            call: member.call.clone(),
        };
        let accessor_source = |accessor: &crate::model::AccessorModel| MemberSource::Declared {
            type_path: type_path.clone(),
            callable: accessor.callable.clone(),
            typed_function: accessor.typed_function.clone(),
            fallible: accessor.fallible,
            call: accessor.call.clone(),
        };
        for interface in &declared.interfaces {
            let interface = self.resolve(&interface.text);
            if !members.interfaces.iter().any(|known| known.equals(&*interface)) {
                members.interfaces.push(interface);
            }
        }
        // The attributes of a constructor parameter are the ones its declaration states
        // (`(property: T [InheritDataTypeFrom(2)]) => ..`); a constructor declared in the
        // positional form states none.
        for constructor in &declared.constructors {
            let named = constructor.parameters.iter().any(|parameter| parameter.name.is_some());
            let parameter_attributes: Vec<Vec<Rc<dyn IXamlCustomAttribute>>> = constructor
                .parameters
                .iter()
                .map(|parameter| if named { self.project_attributes(&parameter.attributes) } else { Vec::new() })
                .collect();
            members.constructors.push(Rc::new(ModelConstructor {
                declaring_type: weak.clone(),
                parameters: self.resolve_all(&constructor.parameters),
                is_public: true,
                parameter_attributes,
                source: source_of(constructor),
            }));
        }
        for property in &declared.properties {
            let (getter, setter, property_type) = self.accessors(type_, property, false, &accessor_source);
            let mut attributes = self.project_attributes(&property.attributes);
            if declared.content_property.as_deref() == Some(property.name.as_str()) {
                attributes.push(self.marker_attribute(attributes::CONTENT));
            }
            members.methods.extend(getter.iter().cloned());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(ModelProperty {
                name: property.name.clone(),
                declaring_type: weak.clone(),
                property_type,
                getter,
                setter,
                attributes,
                indexer_parameters: Vec::new(),
                registered: None,
            }));
        }
        // An indexer is the default member of its type (`this[..]` compiles to the
        // property `Item` and `[DefaultMember("Item")]` on the type).
        for indexer in &declared.indexers {
            let value_type = self.resolve(&indexer.value_type.text);
            let parameters = self.resolve_all(&indexer.parameters);
            let getter = indexer.getter.as_ref().map(|accessor| {
                self.method(type_, "get_Item".to_string(), false, value_type.clone(), parameters.clone(), Vec::new(), accessor_source(accessor))
            });
            let setter = indexer.setter.as_ref().map(|accessor| {
                let mut setter_parameters = parameters.clone();
                setter_parameters.push(value_type.clone());
                self.method(type_, "set_Item".to_string(), false, self.void(), setter_parameters, Vec::new(), accessor_source(accessor))
            });
            members.methods.extend(getter.iter().cloned());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(ModelProperty {
                name: "Item".to_string(),
                declaring_type: weak.clone(),
                property_type: value_type,
                getter,
                setter,
                attributes: self.project_attributes(&indexer.attributes),
                indexer_parameters: parameters,
                registered: None,
            }));
        }
        if !declared.indexers.is_empty() {
            members.attributes.push(ModelCustomAttribute::new(
                self.attribute_type("DefaultMember"),
                vec![XamlValue::String("Item".to_string())],
                Vec::new(),
            ));
        }
        for method in &declared.methods {
            let return_type = match &method.return_type {
                Some(return_type) => self.resolve(&return_type.text),
                None => self.void(),
            };
            members.methods.push(self.method(
                type_,
                method.name.clone(),
                method.is_static,
                return_type,
                self.resolve_all(&method.parameters),
                self.project_attributes(&method.attributes),
                source_of(method),
            ));
        }
        if let Some(parse) = &declared.parse {
            let source = MemberSource::Declared {
                type_path: type_path.clone(),
                callable: parse.clone(),
                typed_function: Some("__markup_parse".to_string()),
                fallible: true,
                call: declared.parse_call.clone(),
            };
            members.methods.push(self.method(type_, "Parse".to_string(), true, self_type.clone(), vec![self.get("System.String")], Vec::new(), source));
        }
        for field in &declared.fields {
            let field_type = match &field.return_type {
                Some(field_type) => self.resolve(&field_type.text),
                None => XamlPseudoType::unknown(),
            };
            members.fields.push(Rc::new(ModelField {
                name: field.name.clone(),
                declaring_type: weak.clone(),
                field_type,
                literal: None,
                attributes: self.project_attributes(&field.attributes),
                source: source_of(field),
            }));
        }
        // Static properties: `static T Name { get; set; }` (static accessors, no field).
        for property in &declared.static_properties {
            let (getter, setter, property_type) = self.accessors(type_, property, true, &accessor_source);
            members.methods.extend(getter.iter().cloned());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(ModelProperty {
                name: property.name.clone(),
                declaring_type: weak.clone(),
                property_type,
                getter,
                setter,
                attributes: self.project_attributes(&property.attributes),
                indexer_parameters: Vec::new(),
                registered: None,
            }));
        }
        for event in &declared.events {
            let arguments = self.resolve_all(&event.parameters);
            let handler_type = match arguments.len() {
                0 => self.get("System.Action"),
                // `EventHandler<TArgs>`: the sender and the arguments.
                2 if arguments[0].is("System", "Object") => self
                    .find_type("System.EventHandler`1")
                    .and_then(|handler| handler.make_generic_type(&arguments[1..]).ok())
                    .unwrap_or_else(|| self.get("System.Delegate")),
                count => self
                    .find_type(&format!("System.Action`{count}"))
                    .and_then(|action| action.make_generic_type(&arguments).ok())
                    .unwrap_or_else(|| self.get("System.Delegate")),
            };
            let add = self.method(type_, format!("add_{}", event.name), false, self.void(), vec![handler_type], Vec::new(), source_of(event));
            members.methods.push(add.clone());
            members.events.push(Rc::new(ModelEvent { name: event.name.clone(), declaring_type: weak.clone(), add: Some(add) }));
        }
        if declared.kind == TypeKind::Enum {
            let wide = declared.enum_members.iter().any(|member| member.value.is_some_and(|value| i32::try_from(value).is_err()));
            members.enum_underlying_type = Some(self.get(if wide { "System.Int64" } else { "System.Int32" }));
            for member in &declared.enum_members {
                // A member whose value the scanner could not evaluate has no literal.
                let literal = member.value.map(|value| if wide { XamlValue::Int64(value) } else { XamlValue::Int32(value as i32) });
                members.fields.push(Rc::new(ModelField {
                    name: member.name.clone(),
                    declaring_type: weak.clone(),
                    field_type: self_type.clone(),
                    literal,
                    attributes: Vec::new(),
                    source: MemberSource::Declared {
                        type_path: type_path.clone(),
                        callable: crate::model::CallableModel { path: member.rust_variant.clone().or(member.rust_value.clone()), resolved: None, dereferenced: None },
                        typed_function: None,
                        fallible: false,
                        call: None,
                    },
                }));
            }
            if declared.is_flags {
                members.attributes.push(ModelCustomAttribute::new(self.get("System.FlagsAttribute"), Vec::new(), Vec::new()));
            }
        }
        members.attributes.extend(self.project_attributes(&declared.attributes));
        // A content property that an own property doesn't carry is stated on the type.
        if let Some(content) = &declared.content_property {
            if !members.properties.iter().any(|property| property.name == *content) {
                members.attributes.push(ModelCustomAttribute::new(
                    self.attribute_type(attributes::CONTENT),
                    Vec::new(),
                    vec![("Name".to_string(), XamlValue::String(content.clone()))],
                ));
            }
        }
    }

    /// The accessors of a plain property (`get_Name`, `set_Name`) and its type.
    fn accessors(
        &self,
        type_: &Rc<ModelType>,
        property: &PropertyModel,
        is_static: bool,
        source: &dyn Fn(&crate::model::AccessorModel) -> MemberSource,
    ) -> (Option<Rc<ModelMethod>>, Option<Rc<ModelMethod>>, Rc<dyn IXamlType>) {
        let property_type = self.resolve(&property.value_type.text);
        let getter = property
            .getter
            .as_ref()
            .map(|accessor| self.method(type_, format!("get_{}", property.name), is_static, property_type.clone(), Vec::new(), Vec::new(), source(accessor)));
        let setter = property.setter.as_ref().map(|accessor| {
            self.method(type_, format!("set_{}", property.name), is_static, self.void(), vec![property_type.clone()], Vec::new(), source(accessor))
        });
        (getter, setter, property_type)
    }

    fn project_markup_type(&self, type_: &Rc<ModelType>, position: Position) -> ModelMembers {
        let declared = self.set.type_at(position);
        let mut members = ModelMembers::default();
        // A collection that derives from an instantiation of the notifying list is that
        // list (and implements its list contracts) only when its values can be cast to it:
        // the crate registers the cast from the collection to the list. Without the cast
        // the collection is a type of its own.
        let declared_base = declared.base.as_ref().filter(|base| {
            let resolved = self.resolve(&base.text);
            let is_list = resolved.as_any().downcast_ref::<ModelType>().and_then(|base| base.definition().map(|definition| definition.key() == FERRO_LIST_DEFINITION));
            is_list != Some(true) || declared.handles.first().is_some_and(|handle| self.is_cast(&handle.text, &base.text))
        });
        members.base_type = match declared_base {
            Some(base) => Some(self.resolve(&base.text)),
            None => match declared.kind {
                TypeKind::Class | TypeKind::Static => self.find_type("System.Object"),
                TypeKind::Struct => self.find_type("System.ValueType"),
                TypeKind::Enum => self.find_type("System.Enum"),
                TypeKind::Interface => None,
            },
        };
        let instantiable = type_.definition().filter(|definition| self.instantiable.borrow().contains(definition.key())).cloned();
        // An instantiation of a definition of the type system has the base type and the
        // interfaces of the definition.
        if let Some(definition) = &instantiable {
            let source = definition.members();
            let parameters = definition.generic_parameter_types();
            let arguments = type_.generic_argument_types();
            if declared_base.is_none() {
                if let Some(base) = &source.base_type {
                    members.base_type = Some(substitute(base, parameters, arguments));
                }
            }
            members.interfaces = source.interfaces.iter().map(|interface| substitute(interface, parameters, arguments)).collect();
        }
        let definition_key = type_.definition().map(|definition| definition.key().to_string());
        let implement = |members: &mut ModelMembers, interface: Rc<dyn IXamlType>| {
            if !members.interfaces.iter().any(|known| known.equals(&*interface)) {
                members.interfaces.push(interface);
            }
        };
        // The dictionary of the framework implements the dictionary contract of its key and
        // value types.
        if definition_key.as_deref() == Some(FERRO_DICTIONARY_DEFINITION) {
            let interface = self.find_type("System.Collections.Generic.IDictionary`2");
            if let Some(Ok(interface)) = interface.map(|interface| interface.make_generic_type(type_.generic_argument_types())) {
                implement(&mut members, interface);
            }
        }
        // The notifying list of the framework implements the list contracts of its element
        // type, and with it every collection that derives from one of its instantiations.
        if definition_key.as_deref() == Some(FERRO_LIST_DEFINITION) {
            for name in ["System.Collections.Generic.IList`1", "System.Collections.Generic.IReadOnlyList`1"] {
                if let Some(Ok(interface)) = self.find_type(name).map(|interface| interface.make_generic_type(type_.generic_argument_types())) {
                    implement(&mut members, interface);
                }
            }
            for name in ["System.Collections.IList", "System.Collections.Specialized.INotifyCollectionChanged"] {
                if let Some(interface) = self.find_type(name) {
                    implement(&mut members, interface);
                }
            }
        }
        self.project_markup(type_, declared, &mut members);
        // A declared instantiation of a definition of the type system also has the members
        // of the definition its metadata does not declare (the indexer and `Count` of a
        // list): abstract members.
        if let Some(definition) = &instantiable {
            let source = definition.members();
            let parameters = definition.generic_parameter_types();
            let arguments = type_.generic_argument_types();
            let weak = Rc::downgrade(type_);
            let mut inherited: Vec<(Rc<ModelMethod>, Rc<ModelMethod>)> = Vec::new();
            for method in source.methods.iter().filter(|method| method.is_abstract) {
                let is_declared = members
                    .methods
                    .iter()
                    .any(|known| known.name == method.name && known.is_static == method.is_static && known.parameters.len() == method.parameters.len());
                if !is_declared {
                    inherited.push((method.clone(), method.substituted(&weak, parameters, arguments)));
                }
            }
            let accessor = |accessor: &Option<Rc<ModelMethod>>| {
                accessor.as_ref().and_then(|accessor| inherited.iter().find(|(source, _)| Rc::ptr_eq(source, accessor)).map(|(_, method)| method.clone()))
            };
            for property in &source.properties {
                if members.properties.iter().any(|known| known.name == property.name) {
                    continue;
                }
                let (getter, setter) = (accessor(&property.getter), accessor(&property.setter));
                if getter.is_none() && setter.is_none() {
                    continue;
                }
                members.properties.push(Rc::new(ModelProperty {
                    name: property.name.clone(),
                    declaring_type: weak.clone(),
                    property_type: substitute(&property.property_type, parameters, arguments),
                    getter,
                    setter,
                    attributes: property.attributes.clone(),
                    indexer_parameters: property.indexer_parameters.iter().map(|parameter| substitute(parameter, parameters, arguments)).collect(),
                    registered: None,
                }));
            }
            members.methods.extend(inherited.into_iter().map(|(_, method)| method));
            for attribute in &source.attributes {
                if !members.attributes.iter().any(|known| known.type_().equals(&*attribute.type_())) {
                    members.attributes.push(attribute.clone());
                }
            }
        }
        members
    }

    /// The class of the object model with the Rust type text `rust_path`.
    fn class_of(&self, rust_path: &str) -> Option<Rc<ModelType>> {
        let position = self.set.position_of_rust_type(rust_path)?;
        self.set.type_at(position).object_model.then(|| self.model_type(position))
    }

    fn project_class(&self, type_: &Rc<ModelType>, position: Position) -> ModelMembers {
        let declared = self.set.type_at(position);
        let mut members = ModelMembers::default();
        let weak = Rc::downgrade(type_);
        let type_path = declared.named_path().to_string();
        members.base_type = match declared.base.as_ref().and_then(|base| self.class_of(&base.text)) {
            Some(base) => Some(base as Rc<dyn IXamlType>),
            None => self.find_type("System.Object"),
        };
        if let Some(callable) = &declared.default_constructor {
            members.constructors.push(Rc::new(ModelConstructor {
                declaring_type: weak.clone(),
                parameters: Vec::new(),
                is_public: true,
                parameter_attributes: Vec::new(),
                source: MemberSource::DefaultConstructor { type_path: type_path.clone(), callable: callable.clone() },
            }));
        }
        // The metadata of the class: its own, and metadata of the same name declared next
        // to it.
        let mut markup: Vec<&TypeModel> = vec![declared];
        for companion in self.index.companions.get(&position).into_iter().flatten() {
            markup.push(self.set.type_at(*companion));
        }
        let content_property = markup.iter().find_map(|declared| declared.content_property.as_deref());
        let property_attributes = |name: &str| -> Vec<Rc<dyn IXamlCustomAttribute>> {
            let stated = markup.iter().find_map(|declared| declared.property_attributes.iter().find(|(property, _)| property == name));
            stated.map_or_else(Vec::new, |(_, attributes)| self.project_attributes(attributes))
        };
        let object_type: Rc<dyn IXamlType> = self.get("FerroUI.FerroObject");
        let has_attribute = |attributes: &[Rc<dyn IXamlCustomAttribute>], name: &str| attributes.iter().any(|attribute| attribute.type_().name() == name);
        // The attributes of declared accessors of attached properties, by accessor name.
        let mut accessor_attributes: Vec<(String, Vec<Rc<dyn IXamlCustomAttribute>>)> = Vec::new();
        for (listed, registered_index) in self.index.properties.get(&position).into_iter().flatten() {
            let listed = self.set.type_at(*listed);
            let registered = &listed.registered[*registered_index];
            // A second accessor of a property is no second property.
            if registered.registration == RegistrationModel::Alias {
                continue;
            }
            // The accessor is a function of the type it is listed under, unless the model
            // states another.
            let type_path = self.set.accessor_type_path(listed, registered).to_string();
            let Some(name) = self.set.name_of(registered).map(str::to_string) else { continue };
            let property_type = self.resolve(&registered.value_type.text);
            let declaration = self.set.declaration_of(registered);
            let assign_binding = registered.assign_binding || declaration.is_some_and(|declaration| declaration.assign_binding);
            // Only the type that registered an attached property declares it as one. A
            // class that adds itself as an owner declares an instance property.
            let attached = registered.kind == RegisteredKind::Attached && registered.registration == RegistrationModel::Declared;
            let source = MemberSource::Registered { type_path: type_path.clone(), accessor: registered.accessor.clone() };
            // The attributes metadata states for the registered property sit where the
            // attribute of the managed property sits: on the property, on the field with
            // its definition and on the accessors of an attached property.
            let declared_attributes = property_attributes(&name);
            members.fields.push(Rc::new(ModelField {
                name: format!("{name}Property"),
                declaring_type: weak.clone(),
                field_type: self.property_definition_type(registered, attached, &property_type, type_),
                literal: None,
                attributes: declared_attributes.clone(),
                source: source.clone(),
            }));
            if attached {
                // The accessors take the host type of the attached property.
                let object_type: Rc<dyn IXamlType> = match registered.host.as_ref().and_then(|host| self.class_of(&host.text)) {
                    Some(host) => host,
                    None => object_type.clone(),
                };
                // `[AssignBinding]` of the accessors is part of the definition of the property.
                let mut declared_attributes = declared_attributes;
                if assign_binding && !has_attribute(&declared_attributes, "AssignBindingAttribute") {
                    declared_attributes.push(self.marker_attribute(attributes::ASSIGN_BINDING));
                }
                // Accessors the metadata declares itself are the accessors of the managed
                // original; an accessor that is not declared is the plain one.
                let declares_any = |accessor: &str| markup.iter().any(|declared| declared.methods.iter().any(|method| method.is_static && method.name == accessor));
                // A declared accessor stands for the plain one only if it is the accessor
                // for the host type of the property.
                let declares = |accessor: &str| {
                    markup.iter().any(|declared| {
                        declared.methods.iter().any(|method| {
                            method.is_static
                                && method.name == accessor
                                && method.parameters.first().is_some_and(|host| self.resolve(&host.type_.text).equals(&*object_type))
                        })
                    })
                };
                let getter_name = format!("Get{name}");
                let setter_name = format!("Set{name}");
                if (declares_any(&getter_name) || declares_any(&setter_name)) && !declared_attributes.is_empty() {
                    accessor_attributes.push((getter_name.clone(), declared_attributes.clone()));
                    accessor_attributes.push((setter_name.clone(), declared_attributes.clone()));
                }
                if !declares(&getter_name) {
                    members.methods.push(self.method(
                        type_,
                        getter_name,
                        true,
                        property_type.clone(),
                        vec![object_type.clone()],
                        declared_attributes.clone(),
                        source.clone(),
                    ));
                }
                if !declares(&setter_name) {
                    members.methods.push(self.method(type_, setter_name, true, self.void(), vec![object_type.clone(), property_type], declared_attributes, source));
                }
                continue;
            }
            let getter = self.method(type_, format!("get_{name}"), false, property_type.clone(), Vec::new(), Vec::new(), source.clone());
            let setter = (!registered.read_only)
                .then(|| self.method(type_, format!("set_{name}"), false, self.void(), vec![property_type.clone()], Vec::new(), source.clone()));
            let mut attributes = declared_attributes;
            if assign_binding && !has_attribute(&attributes, "AssignBindingAttribute") {
                attributes.push(self.marker_attribute(attributes::ASSIGN_BINDING));
            }
            if content_property == Some(name.as_str()) {
                attributes.push(self.marker_attribute(attributes::CONTENT));
            }
            members.methods.push(getter.clone());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(ModelProperty {
                name,
                declaring_type: weak.clone(),
                property_type,
                getter: Some(getter),
                setter,
                attributes,
                indexer_parameters: Vec::new(),
                registered: Some((type_path.clone(), registered.accessor.clone())),
            }));
        }
        let first_declared = members.methods.len();
        for declared in &markup {
            self.project_markup(type_, declared, &mut members);
        }
        for method in &mut members.methods[first_declared..] {
            if !method.is_static {
                continue;
            }
            let Some((_, attributes)) = accessor_attributes.iter().find(|(name, _)| *name == method.name) else { continue };
            let mut merged = method.attributes.clone();
            for attribute in attributes {
                if !merged.iter().any(|known| known.type_().equals(&*attribute.type_())) {
                    merged.push(attribute.clone());
                }
            }
            *method = Rc::new(ModelMethod {
                name: method.name.clone(),
                declaring_type: method.declaring_type.clone(),
                is_static: method.is_static,
                return_type: method.return_type.clone(),
                parameters: method.parameters.clone(),
                attributes: merged,
                generic_parameters: method.generic_parameters.clone(),
                generic_arguments: method.generic_arguments.clone(),
                is_abstract: method.is_abstract,
                source: method.source.clone(),
            });
        }
        if members.constructors.is_empty() {
            // Every class of the managed original has a constructor; the one of a class
            // markup cannot create (an abstract class, a class without metadata for its
            // constructors) is not public.
            members.constructors.push(Rc::new(ModelConstructor {
                declaring_type: weak.clone(),
                parameters: Vec::new(),
                is_public: false,
                parameter_attributes: Vec::new(),
                source: MemberSource::TypeSystem,
            }));
        }
        match type_.key() {
            "FerroUI.FerroObject" => self.project_ferro_object(type_, &mut members),
            "FerroUI.Interactivity.Interactive" => self.project_interactive(type_, &mut members),
            _ => {}
        }
        members
    }

    /// The type of the definition of a registered property: `FerroUI.StyledProperty<T>`,
    /// `FerroUI.AttachedProperty<T>` or `FerroUI.DirectProperty<TOwner, T>`.
    fn property_definition_type(&self, registered: &RegisteredModel, attached: bool, property_type: &Rc<dyn IXamlType>, owner: &Rc<ModelType>) -> Rc<dyn IXamlType> {
        let (definition, arguments): (&str, Vec<Rc<dyn IXamlType>>) = if registered.kind == RegisteredKind::Direct {
            ("DirectProperty`2", vec![owner.clone() as Rc<dyn IXamlType>, property_type.clone()])
        } else if attached {
            ("AttachedProperty`1", vec![property_type.clone()])
        } else {
            ("StyledProperty`1", vec![property_type.clone()])
        };
        self.find_type(&format!("{PROPERTY_NAMESPACE}.{definition}"))
            .and_then(|definition| definition.make_generic_type(&arguments).ok())
            .unwrap_or_else(|| self.get(&format!("{PROPERTY_NAMESPACE}.FerroProperty")))
    }

    // --- the members of the object model no declaration states ---------------

    fn generic_parameter(&self, owner: &str, name: &str) -> Rc<ModelType> {
        ModelType::create(
            &self.this,
            ModelTypeSpec::new(format!("{owner}!!{name}"), "", name, ModelTypeKind::GenericParameter, ModelTypeOrigin::GenericParameter),
        )
    }

    fn generic_method(&self, type_: &Rc<ModelType>, name: &str, return_type: Rc<dyn IXamlType>, parameters: Vec<Rc<dyn IXamlType>>, parameter: Rc<ModelType>) -> Rc<ModelMethod> {
        Rc::new(ModelMethod {
            name: name.to_string(),
            declaring_type: Rc::downgrade(type_),
            is_static: false,
            return_type,
            parameters,
            attributes: Vec::new(),
            generic_parameters: vec![parameter],
            generic_arguments: Vec::new(),
            is_abstract: false,
            source: MemberSource::TypeSystem,
        })
    }

    /// The members of the root class the compiler resolves and metadata cannot declare:
    /// `SetValue<T>`, and the untyped `SetValue`, `GetValue` and `Bind` next to it.
    fn project_ferro_object(&self, type_: &Rc<ModelType>, members: &mut ModelMembers) {
        let name = |name: &str| format!("{PROPERTY_NAMESPACE}.{name}");
        let (Some(property), Some(styled), Some(priority)) =
            (self.find_type(&name("FerroProperty")), self.find_type(&name("StyledProperty`1")), self.find_type("FerroUI.Data.BindingPriority"))
        else {
            return;
        };
        let object_type = self.get("System.Object");
        let disposable = self.get("System.IDisposable");
        let method = |name: &str, return_type: Rc<dyn IXamlType>, parameters: Vec<Rc<dyn IXamlType>>| {
            self.method(type_, name.to_string(), false, return_type, parameters, Vec::new(), MemberSource::TypeSystem)
        };
        // IDisposable SetValue<T>(StyledProperty<T> property, T value, BindingPriority priority)
        if !is_declared(members, "SetValue", 3, |parameter| parameter.name() == "StyledProperty`1") {
            let t = self.generic_parameter("FerroUI.FerroObject.SetValue", "T");
            let t_type: Rc<dyn IXamlType> = t.clone();
            if let Ok(styled_of_t) = styled.make_generic_type(std::slice::from_ref(&t_type)) {
                members.methods.push(self.generic_method(type_, "SetValue", disposable.clone(), vec![styled_of_t, t_type, priority.clone()], t));
            }
        }
        // IDisposable SetValue(FerroProperty property, object value, BindingPriority priority)
        if !is_declared(members, "SetValue", 3, |parameter| parameter.equals(&*property)) {
            members.methods.push(method("SetValue", disposable, vec![property.clone(), object_type.clone(), priority]));
        }
        // object GetValue(FerroProperty property)
        if !is_declared(members, "GetValue", 1, |parameter| parameter.equals(&*property)) {
            members.methods.push(method("GetValue", object_type, vec![property.clone()]));
        }
        // BindingExpressionBase Bind(FerroProperty property, BindingBase binding)
        if let (Some(binding), Some(expression)) = (self.find_type("FerroUI.Data.BindingBase"), self.find_type("FerroUI.Data.BindingExpressionBase")) {
            if !is_declared(members, "Bind", 2, |parameter| parameter.equals(&*property)) {
                members.methods.push(method("Bind", expression, vec![property, binding]));
            }
        }
    }

    /// The members of the class of routed events the compiler resolves and metadata cannot
    /// declare: `AddHandler` and `AddHandler<TEventArgs>`.
    fn project_interactive(&self, type_: &Rc<ModelType>, members: &mut ModelMembers) {
        const NAMESPACE: &str = "FerroUI.Interactivity";
        let (Some(routed_event), Some(routes)) = (self.find_type(&format!("{NAMESPACE}.RoutedEvent")), self.find_type(&format!("{NAMESPACE}.RoutingStrategies")))
        else {
            return;
        };
        let delegate = self.get("System.Delegate");
        let boolean = self.get("System.Boolean");
        let void = self.void();
        // void AddHandler(RoutedEvent routedEvent, Delegate handler, RoutingStrategies routes, bool handledEventsToo)
        if !is_declared(members, "AddHandler", 4, |parameter| parameter.equals(&*routed_event)) {
            members.methods.push(self.method(
                type_,
                "AddHandler".to_string(),
                false,
                void.clone(),
                vec![routed_event.clone(), delegate, routes.clone(), boolean.clone()],
                Vec::new(),
                MemberSource::TypeSystem,
            ));
        }
        // void AddHandler<TEventArgs>(RoutedEvent<TEventArgs> routedEvent, EventHandler<TEventArgs> handler,
        //     RoutingStrategies routes, bool handledEventsToo)
        let generic_event = self.find_model_type(&format!("{NAMESPACE}.RoutedEvent`1"));
        let handler = self.find_type("System.EventHandler`1");
        if let (Some(generic_event), Some(handler)) = (generic_event, handler) {
            if !is_declared(members, "AddHandler", 4, |parameter| parameter.generic_arguments().len() == 1) {
                let t = self.generic_parameter("FerroUI.Interactivity.Interactive.AddHandler", "TEventArgs");
                let t_type: Rc<dyn IXamlType> = t.clone();
                let event_of_t = self.instantiate_open(&generic_event, std::slice::from_ref(&t_type), routed_event.clone());
                if let Ok(handler_of_t) = handler.make_generic_type(std::slice::from_ref(&t_type)) {
                    members.methods.push(self.generic_method(type_, "AddHandler", void, vec![event_of_t, handler_of_t, routes, boolean], t));
                }
            }
        }
    }
}

/// Whether `members` has an instance method `name` with `parameters` parameters whose
/// first one `first` accepts: a member metadata declares itself is not projected again.
fn is_declared(members: &ModelMembers, name: &str, parameters: usize, first: impl Fn(&Rc<dyn IXamlType>) -> bool) -> bool {
    members
        .methods
        .iter()
        .any(|method| method.name == name && !method.is_static && method.parameters.len() == parameters && method.parameters.first().is_some_and(&first))
}

/// The Rust types that hold a value of the type `declared`, as the one text of each
/// ([`ModelSet::expanded`]), the untyped form first: the handles its metadata states, the
/// handle and the optional handle of a class of the object model the crate registers, the
/// type itself for an enumeration.
fn handles_of(set: &ModelSet, ref_path: &str, declared: &TypeModel) -> Vec<String> {
    let mut handles: Vec<String> = Vec::new();
    let mut add = |handle: String| {
        if !handles.contains(&handle) {
            handles.push(handle);
        }
    };
    let path = set.expanded(&declared.rust_path.text);
    if declared.object_model && !declared.unregistered && declared.kind == TypeKind::Class {
        add(format!("{ref_path}<{path}>"));
        add(format!("Option<{ref_path}<{path}>>"));
    }
    if declared.kind == TypeKind::Enum {
        add(path);
    }
    for handle in &declared.handles {
        add(set.expanded(&handle.text));
    }
    handles
}

impl IXamlTypeSystem for ModelTypeSystem {
    fn assemblies(&self) -> Vec<Rc<dyn IXamlAssembly>> {
        self.assemblies.borrow().iter().map(|assembly| assembly.clone() as Rc<dyn IXamlAssembly>).collect()
    }
    fn well_known_types(&self) -> Rc<XamlTypeWellKnownTypes> {
        self.well_known_types
            .get_or_init(|| match XamlTypeWellKnownTypes::new(self) {
                Ok(types) => Rc::new(types),
                Err(error) => panic!("The table of runtime library types is incomplete: {error}"),
            })
            .clone()
    }
    /// The assembly whose name equals `name` without regard to case, as the run-time
    /// type system finds it.
    fn find_assembly(&self, substring: &str) -> Option<Rc<dyn IXamlAssembly>> {
        self.assemblies
            .borrow()
            .iter()
            .find(|assembly| assembly.name.to_lowercase() == substring.to_lowercase())
            .map(|assembly| assembly.clone() as Rc<dyn IXamlAssembly>)
    }
    fn find_type(&self, name: &str) -> Option<Rc<dyn IXamlType>> {
        self.find_model_type(name).map(|type_| type_ as Rc<dyn IXamlType>)
    }
    fn find_type_in_assembly(&self, name: &str, assembly: &str) -> Option<Rc<dyn IXamlType>> {
        let found = self.find_model_type(name)?;
        let assemblies = self.assemblies.borrow();
        assemblies.iter().filter(|candidate| candidate.has_name(assembly)).any(|candidate| self.type_belongs_to(&found, candidate)).then(|| found as Rc<dyn IXamlType>)
    }
}

/// Builds the members of a type the type system defines itself.
pub(crate) struct MemberBuilder {
    pub system: Rc<ModelTypeSystem>,
    pub type_: Rc<ModelType>,
    pub members: ModelMembers,
    /// The declared metadata of the type has been projected onto it.
    pub projected: bool,
}

impl MemberBuilder {
    /// The type named `full_name`.
    fn t(&self, full_name: &str) -> Rc<dyn IXamlType> {
        self.system.get(full_name)
    }

    /// The generic parameter `index` of the type being defined.
    fn parameter(&self, index: usize) -> Rc<dyn IXamlType> {
        match self.type_.generic_parameter_types().get(index) {
            Some(parameter) => parameter.clone(),
            None => XamlPseudoType::unknown(),
        }
    }

    /// `definition<arguments>`.
    fn generic(&self, definition: &str, arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
        self.t(definition).make_generic_type(arguments).unwrap_or_else(|_| XamlPseudoType::unknown())
    }

    /// Projects the members of the metadata a model declares for the type (a runtime
    /// library type under its name) onto this type. `false` (and nothing projected) if no
    /// model declares such metadata.
    fn project_metadata(&mut self) -> bool {
        if self.projected {
            return true;
        }
        let Some(position) = self.system.index.core_markup.get(self.type_.key()).and_then(|declarations| declarations.first()).copied() else {
            return false;
        };
        self.projected = true;
        let system = self.system.clone();
        system.project_markup(&self.type_, system.set.type_at(position), &mut self.members);
        true
    }

    fn base(&mut self, full_name: &str) {
        self.members.base_type = self.system.find_type(full_name);
    }

    fn method(&mut self, name: &str, is_static: bool, return_type: Rc<dyn IXamlType>, parameters: Vec<Rc<dyn IXamlType>>, is_abstract: bool) -> Rc<ModelMethod> {
        // A member the declared metadata of the type states is not declared a second time.
        if let Some(existing) =
            self.members.methods.iter().find(|method| method.name == name && method.is_static == is_static && method.parameters.len() == parameters.len())
        {
            return existing.clone();
        }
        let method = Rc::new(ModelMethod {
            name: name.to_string(),
            declaring_type: Rc::downgrade(&self.type_),
            is_static,
            return_type,
            parameters,
            attributes: Vec::new(),
            generic_parameters: Vec::new(),
            generic_arguments: Vec::new(),
            is_abstract,
            source: MemberSource::TypeSystem,
        });
        self.members.methods.push(method.clone());
        method
    }

    fn constructor(&mut self, parameters: Vec<Rc<dyn IXamlType>>) {
        if self.members.constructors.iter().any(|constructor| constructor.parameters.len() == parameters.len()) {
            return;
        }
        self.members.constructors.push(Rc::new(ModelConstructor {
            declaring_type: Rc::downgrade(&self.type_),
            parameters,
            is_public: true,
            parameter_attributes: Vec::new(),
            source: MemberSource::TypeSystem,
        }));
    }

    /// The indexer of the type: the property `Item` with the accessors `get_Item` and (when
    /// `writable`) `set_Item`, and `[DefaultMember("Item")]` on the type. The accessors are
    /// abstract unless a method of the name and arity is declared before.
    fn indexer(&mut self, parameters: Vec<Rc<dyn IXamlType>>, type_: Rc<dyn IXamlType>, writable: bool) {
        let getter = self.method("get_Item", false, type_.clone(), parameters.clone(), true);
        let setter = writable.then(|| {
            let mut setter_parameters = parameters.clone();
            setter_parameters.push(type_.clone());
            let void = self.system.void();
            self.method("set_Item", false, void, setter_parameters, true)
        });
        self.members.properties.push(Rc::new(ModelProperty {
            name: "Item".to_string(),
            declaring_type: Rc::downgrade(&self.type_),
            property_type: type_,
            getter: Some(getter),
            setter,
            attributes: Vec::new(),
            indexer_parameters: parameters,
            registered: None,
        }));
        self.members.attributes.push(ModelCustomAttribute::new(
            self.system.attribute_type("DefaultMember"),
            vec![XamlValue::String("Item".to_string())],
            Vec::new(),
        ));
    }

    /// A read-only property with the getter `get_<name>`.
    fn property(&mut self, name: &str, type_: Rc<dyn IXamlType>, is_static: bool, is_abstract: bool) {
        let getter = self.method(&format!("get_{name}"), is_static, type_.clone(), Vec::new(), is_abstract);
        self.members.properties.push(Rc::new(ModelProperty {
            name: name.to_string(),
            declaring_type: Rc::downgrade(&self.type_),
            property_type: type_,
            getter: Some(getter),
            setter: None,
            attributes: Vec::new(),
            indexer_parameters: Vec::new(),
            registered: None,
        }));
    }
}

/// The type a reference of the table names, for the type `builder` builds. `element` is
/// the element type of a list markup creates.
fn type_of(builder: &MemberBuilder, reference: &CoreRef, element: Option<&Rc<dyn IXamlType>>) -> Rc<dyn IXamlType> {
    match reference {
        CoreRef::Type(full_name) => builder.t(full_name),
        CoreRef::Parameter(index) => builder.parameter(*index),
        CoreRef::Element => match element {
            Some(element) => element.clone(),
            None => builder.t("System.Object"),
        },
        CoreRef::Generic(definition, arguments) => {
            let arguments: Vec<Rc<dyn IXamlType>> = arguments.iter().map(|argument| type_of(builder, argument, element)).collect();
            builder.generic(definition, &arguments)
        }
        CoreRef::Array(element_type) => type_of(builder, element_type, element).make_array_type(1).unwrap_or_else(|_| XamlPseudoType::unknown()),
    }
}

fn types_of(builder: &MemberBuilder, references: &[CoreRef], element: Option<&Rc<dyn IXamlType>>) -> Vec<Rc<dyn IXamlType>> {
    references.iter().map(|reference| type_of(builder, reference, element)).collect()
}

/// Defines what the description of a type of the table states on the type `builder`
/// builds, in the order the run-time type system defines it: the base, the interfaces,
/// and the members (none when the description only stands in for declared metadata).
fn apply(builder: &mut MemberBuilder, description: &CoreType, element: Option<&Rc<dyn IXamlType>>) {
    match &description.base {
        Some(CoreRef::Type(full_name)) => builder.base(full_name),
        Some(base) => {
            let base = type_of(builder, base, element);
            builder.members.base_type = Some(base);
        }
        None => {}
    }
    for interface in &description.interfaces {
        let interface = type_of(builder, interface, element);
        builder.members.interfaces.push(interface);
    }
    if description.stands_in && builder.project_metadata() {
        return;
    }
    for member in &description.members {
        match member {
            CoreMember::Constructor { parameters, .. } => {
                let parameters = types_of(builder, parameters, element);
                builder.constructor(parameters);
            }
            CoreMember::Method { name, is_static, return_type, parameters, body } => {
                let return_type = type_of(builder, return_type, element);
                let parameters = types_of(builder, parameters, element);
                builder.method(name, *is_static, return_type, parameters, *body == CoreBody::Virtual);
            }
            CoreMember::Property { name, type_, is_static, body } => {
                let type_ = type_of(builder, type_, element);
                builder.property(name, type_, *is_static, *body == CoreBody::Virtual);
            }
            CoreMember::Indexer { parameters, type_, writable } => {
                let parameters = types_of(builder, parameters, element);
                let type_ = type_of(builder, type_, element);
                builder.indexer(parameters, type_, *writable);
            }
        }
    }
}
