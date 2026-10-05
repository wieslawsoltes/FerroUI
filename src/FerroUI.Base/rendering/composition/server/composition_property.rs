use super::IServerObject;
use crate::rendering::composition::expressions::ExpressionVariant;
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicI32, Ordering};

static NEXT_ID: AtomicI32 = AtomicI32::new(1);

/// The identity of a property of a server-side composition object: what
/// animations, expressions and change notifications refer to.
///
/// Properties are registered once per process and are plain data (ids,
/// names and function pointers), so the UI thread and the render thread
/// share them.
pub struct CompositionProperty {
    id: i32,
    name: &'static str,
    owner: TypeId,
    owner_name: &'static str,
    get_variant: Option<fn(&dyn IServerObject) -> ExpressionVariant>,
    set_variant: Option<fn(&dyn IServerObject, ExpressionVariant)>,
}

impl CompositionProperty {
    fn new(
        id: i32,
        name: &'static str,
        owner: TypeId,
        owner_name: &'static str,
        get_variant: Option<fn(&dyn IServerObject) -> ExpressionVariant>,
        set_variant: Option<fn(&dyn IServerObject, ExpressionVariant)>,
    ) -> Self {
        Self { id, name, owner, owner_name, get_variant, set_variant }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The type that declares the property: the generated property block
    /// of the server class.
    pub fn owner(&self) -> TypeId {
        self.owner
    }

    /// The name of the server class that declares the property.
    pub fn owner_name(&self) -> &'static str {
        self.owner_name
    }

    /// Reads the property of an object as an expression variant; `None` for
    /// a property whose type has no variant form.
    pub fn get_variant(&self) -> Option<fn(&dyn IServerObject) -> ExpressionVariant> {
        self.get_variant
    }

    /// Stores an expression variant into the field of the property, without
    /// any notification; `None` for a property that cannot be animated.
    ///
    /// This is the type-erased form of `set_field` an animation uses to
    /// write the value it evaluated.
    pub fn set_variant(&self) -> Option<fn(&dyn IServerObject, ExpressionVariant)> {
        self.set_variant
    }

    /// Registers a property of the server class whose generated property
    /// block is `TOwner`.
    pub fn register<TOwner: Any, TField>(
        name: &'static str,
        owner_name: &'static str,
        get_field: fn(&dyn IServerObject) -> TField,
        set_field: fn(&dyn IServerObject, TField),
        get_variant: Option<fn(&dyn IServerObject) -> ExpressionVariant>,
        set_variant: Option<fn(&dyn IServerObject, ExpressionVariant)>,
    ) -> CompositionPropertyOf<TField> {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        CompositionPropertyOf {
            base: CompositionProperty::new(id, name, TypeId::of::<TOwner>(), owner_name, get_variant, set_variant),
            get_field,
            set_field,
        }
    }
}

impl PartialEq for CompositionProperty {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CompositionProperty {}

impl std::hash::Hash for CompositionProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl std::fmt::Debug for CompositionProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}#{}", self.owner_name, self.name, self.id)
    }
}

/// A [`CompositionProperty`] with typed access to its field (upstream
/// `CompositionProperty<T>`).
pub struct CompositionPropertyOf<T> {
    base: CompositionProperty,
    get_field: fn(&dyn IServerObject) -> T,
    set_field: fn(&dyn IServerObject, T),
}

impl<T> CompositionPropertyOf<T> {
    /// The untyped property.
    pub fn base(&self) -> &CompositionProperty {
        &self.base
    }

    /// Reads the field of the property.
    pub fn get_field(&self, obj: &dyn IServerObject) -> T {
        (self.get_field)(obj)
    }

    /// Writes the field of the property, without any notification.
    pub fn set_field(&self, obj: &dyn IServerObject, value: T) {
        (self.set_field)(obj, value)
    }
}

impl<T> std::ops::Deref for CompositionPropertyOf<T> {
    type Target = CompositionProperty;

    fn deref(&self) -> &CompositionProperty {
        &self.base
    }
}

/// The generated property block `P` embedded in a server object.
///
/// Panics if the object does not embed one: properties are only used with
/// objects of the class that registered them.
pub fn props_of<P: Any>(obj: &dyn IServerObject) -> &P {
    match obj.get_props(TypeId::of::<P>()).and_then(|props| props.downcast_ref::<P>()) {
        Some(props) => props,
        None => panic!("the server object does not have the properties of {}", std::any::type_name::<P>()),
    }
}
