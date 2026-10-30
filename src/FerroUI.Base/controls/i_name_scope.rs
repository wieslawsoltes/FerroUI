use crate::utilities::{SynchronousCompletionAsyncResult, SynchronousCompletionAsyncResultSource};
use crate::data::core::ValueTypes;
use crate::{BoxedValue, FerroObject, Ref};
use std::ops::Deref;
use std::rc::Rc;

/// Why an element could not be registered in a name scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameScopeError {
    /// The name scope is completed: no further registrations are allowed.
    Completed,
    /// Another element is registered with the name.
    DuplicateName(String),
}

impl std::fmt::Display for NameScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => f.write_str("NameScope is completed, no further registrations are allowed"),
            Self::DuplicateName(name) => write!(f, "Control with the name '{name}' already registered."),
        }
    }
}

impl std::error::Error for NameScopeError {}

/// Why a value could not be registered in a name scope
/// ([`INameScope::try_register_value`]): the errors of [`NameScopeError`] and
/// the ones only a value can have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameScopeValueError {
    /// The name scope is completed: no further registrations are allowed.
    Completed,
    /// Another value is registered with the name.
    DuplicateName(String),
    /// A custom scope that only implements the registration of objects of the
    /// object model was given another value.
    UnsupportedValue,
    /// The value is null (a boxed nullable that holds no value).
    NullValue,
}

impl std::fmt::Display for NameScopeValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => NameScopeError::Completed.fmt(f),
            Self::DuplicateName(name) => NameScopeError::DuplicateName(name.clone()).fmt(f),
            Self::NullValue => f.write_str("Value cannot be null. (Parameter 'element')"),
            Self::UnsupportedValue => f.write_str("This custom name scope only supports native objects."),
        }
    }
}

impl std::error::Error for NameScopeValueError {}

impl From<NameScopeError> for NameScopeValueError {
    fn from(error: NameScopeError) -> Self {
        match error {
            NameScopeError::Completed => Self::Completed,
            NameScopeError::DuplicateName(name) => Self::DuplicateName(name),
        }
    }
}

/// Defines a name scope.
pub trait INameScope {
    /// Registers any non-null object, retaining its identity. The value form
    /// corresponds to the managed `object` parameter. Existing native-only
    /// custom scopes can override this method to accept plain objects too.
    fn try_register_value(&self, name: &str, value: BoxedValue) -> Result<(), NameScopeValueError> {
        if self.is_completed() {
            return Err(NameScopeValueError::Completed);
        }
        let value = ValueTypes::normalize(value).ok_or(NameScopeValueError::NullValue)?;
        let element = ValueTypes::as_object(&*value).ok_or(NameScopeValueError::UnsupportedValue)?;
        Ok(self.try_register(name, element)?)
    }

    /// Registers any object; panics on the same errors as `try_register_value`.
    fn register_value(&self, name: &str, value: BoxedValue) {
        if let Err(error) = self.try_register_value(name, value) {
            panic!("{error}");
        }
    }

    /// Finds any named object immediately, without traversing incomplete scopes.
    fn find_value(&self, name: &str) -> Option<BoxedValue> {
        self.find(name).map(|element| Rc::new(element) as BoxedValue)
    }

    /// Finds any named object, completing synchronously on registration or completion.
    fn find_value_async(&self, name: &str) -> SynchronousCompletionAsyncResult<Option<BoxedValue>> {
        map_result(self.find_async(name), |element| element.map(|element| Rc::new(element) as BoxedValue))
    }

    /// Registers an element in the name scope. Panics if the scope is
    /// completed or another element is registered with the name: see
    /// [`try_register`](Self::try_register) for the form that reports these
    /// as errors.
    fn register(&self, name: &str, element: Ref<FerroObject>) {
        if let Err(error) = self.try_register(name, element) {
            panic!("{error}");
        }
    }

    /// Registers an element in the name scope, reporting a completed scope
    /// and a name that another element is registered with as errors.
    /// Registering an element again with its name does nothing.
    fn try_register(&self, name: &str, element: Ref<FerroObject>) -> Result<(), NameScopeError>;

    /// Finds a named element in the name scope, waits for the scope to be
    /// completely populated before returning `None`. Returned results always
    /// complete synchronously.
    fn find_async(&self, name: &str) -> SynchronousCompletionAsyncResult<Option<Ref<FerroObject>>>;

    /// Finds a named element in the name scope, returns immediately, doesn't
    /// traverse the name scope stack.
    fn find(&self, name: &str) -> Option<Ref<FerroObject>>;

    /// Marks the name scope as completed, no further registrations will be
    /// allowed.
    fn complete(&self);

    /// Whether the name scope is completed, no further registrations are
    /// allowed.
    fn is_completed(&self) -> bool;

    /// The scope became the value of the attached `NameScope` property of
    /// `owner`, which holds it from now on. Not a member of the managed
    /// original: there the scope and the element hold each other and the
    /// collector frees both; here the scope holds this one element weakly
    /// (it finds it as long as it is alive). A scope that holds no elements
    /// itself has nothing to do.
    fn attached_to(&self, _owner: &Ref<FerroObject>) {}

    /// The scope is no longer the value of the attached `NameScope`
    /// property of `owner`: it holds the element as every other again.
    fn detached_from(&self, _owner: &Ref<FerroObject>) {}
}

/// Projects a result without holding a scope borrow across synchronous callbacks.
pub(crate) fn map_result<T: Clone + 'static, U: Clone + 'static>(
    result: SynchronousCompletionAsyncResult<T>,
    map: impl FnOnce(T) -> U + 'static,
) -> SynchronousCompletionAsyncResult<U> {
    if result.is_completed() {
        return SynchronousCompletionAsyncResult::new(map(result.get_result()));
    }
    let source = SynchronousCompletionAsyncResultSource::new();
    let output = source.async_result();
    let pending = result.clone();
    result.on_completed(move || source.set_result(map(pending.get_result())));
    output
}

/// A shared handle to a name scope, compared by identity so that it can be a
/// property value.
#[derive(Clone)]
pub struct NameScopeRef(pub Rc<dyn INameScope>);

impl NameScopeRef {
    pub fn new(scope: impl INameScope + 'static) -> Self {
        NameScopeRef(Rc::new(scope))
    }
}

impl Deref for NameScopeRef {
    type Target = dyn INameScope;

    fn deref(&self) -> &(dyn INameScope + 'static) {
        &*self.0
    }
}

impl PartialEq for NameScopeRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl From<Rc<dyn INameScope>> for NameScopeRef {
    fn from(value: Rc<dyn INameScope>) -> Self {
        NameScopeRef(value)
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn INameScope {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
