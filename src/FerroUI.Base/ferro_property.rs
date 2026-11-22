use crate::data::{BindingMode, BindingPriority, BindingValue};
use crate::ferro_property_metadata::Coercion;
use crate::property_store::{EffectiveValue, EffectiveValueDyn, IValueEntry};
use crate::reactive::{Disposable, IDisposable, IObservable, Subscription};
use crate::utilities::HandlerList;
use crate::type_system::{ChangeHandlerLayer, InheritedReaction};
use crate::{
    AttachedProperty, DirectProperty, DirectPropertyMetadata, FerroObject, FerroPropertyChangedEventArgs,
    FerroPropertyMetadata, FerroPropertyRegistry, ObjectType, StaticType, StyledProperty, StyledPropertyMetadata,
    StyledValue, TypeIdentity, TypeInfo,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

/// The bounds every property value type must satisfy.
pub trait PropertyValue: Clone + PartialEq + 'static {}

impl<T: Clone + PartialEq + 'static> PropertyValue for T {}

/// A value of any property value type, for untyped access to properties.
///
/// Implemented for every type that can be a property value. Unlike
/// [`Any`], it supports equality, so untyped values can themselves be
/// property values (for example a data context).
pub trait AnyValue: Any {
    /// The value as [`Any`]. Call [`as_any`](trait.AnyValue.html#method.as_any)
    /// on `dyn AnyValue` instead.
    ///
    /// The trait is implemented for every `PartialEq` type, which includes
    /// `Rc<dyn AnyValue>` itself. The trait methods therefore have names that
    /// cannot be reached by accident on a boxed value: the inherent methods
    /// of `dyn AnyValue` are the API, and they dereference through the box.
    #[doc(hidden)]
    fn any_value_as_any(&self) -> &dyn Any;

    /// Compares with another untyped value: equal if both hold the same type
    /// and the values are equal. Use `==` on `dyn AnyValue` instead.
    #[doc(hidden)]
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool;

    /// The Rust type name of the contained value. Use `type_name` on
    /// `dyn AnyValue` instead.
    #[doc(hidden)]
    fn any_value_type_name(&self) -> &'static str;
}

impl<T: PartialEq + 'static> AnyValue for T {
    #[inline]
    fn any_value_as_any(&self) -> &dyn Any {
        self
    }

    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        other.any_value_as_any().downcast_ref::<T>().is_some_and(|other| value_equals(self, other))
    }

    fn any_value_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

/// Deviation (DEVIATIONS.md, Property system): the comparer of the reference
/// in place of `PartialEq` alone.
///
/// Whether two property values are equal as the reference compares them
/// (`EqualityComparer<T>.Default.Equals`, and `object.Equals` for boxed
/// values): `==`, and a floating-point value that is not a number equals
/// another that is not a number, bare or nullable. With `==` alone a value
/// store whose value is NaN (a `Width` that is not set) reports a change
/// every time the value is set again.
#[inline]
pub fn value_equals<T: PartialEq + 'static>(a: &T, b: &T) -> bool {
    a == b || both_not_a_number(a, b)
}

#[inline]
fn both_not_a_number<T: 'static>(a: &T, b: &T) -> bool {
    let (a, b): (&dyn Any, &dyn Any) = (a, b);
    if let (Some(a), Some(b)) = (a.downcast_ref::<f64>(), b.downcast_ref::<f64>()) {
        return a.is_nan() && b.is_nan();
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<f32>(), b.downcast_ref::<f32>()) {
        return a.is_nan() && b.is_nan();
    }
    if let (Some(Some(a)), Some(Some(b))) = (a.downcast_ref::<Option<f64>>(), b.downcast_ref::<Option<f64>>()) {
        return a.is_nan() && b.is_nan();
    }
    if let (Some(Some(a)), Some(Some(b))) = (a.downcast_ref::<Option<f32>>(), b.downcast_ref::<Option<f32>>()) {
        return a.is_nan() && b.is_nan();
    }
    false
}

impl dyn AnyValue {
    /// The contained value as [`Any`], for downcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn Any {
        self.any_value_as_any()
    }

    /// The [`TypeId`] of the contained value.
    #[inline]
    pub fn value_type_id(&self) -> TypeId {
        self.any_value_as_any().type_id()
    }

    /// The Rust type name of the contained value.
    #[inline]
    pub fn type_name(&self) -> &'static str {
        self.any_value_type_name()
    }

    /// Whether the contained value is of type `T`.
    #[inline]
    pub fn is<T: 'static>(&self) -> bool {
        self.any_value_as_any().is::<T>()
    }

    /// The contained value, if it is of type `T`.
    #[inline]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.any_value_as_any().downcast_ref::<T>()
    }

    /// Compares with another untyped value: equal if both hold the same type
    /// and the values are equal.
    #[inline]
    pub fn value_eq(&self, other: &dyn AnyValue) -> bool {
        self.any_value_eq(other)
    }
}

impl PartialEq for dyn AnyValue {
    fn eq(&self, other: &Self) -> bool {
        self.any_value_eq(other)
    }
}

impl fmt::Debug for dyn AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyValue<{}>", self.any_value_type_name())
    }
}

/// An untyped, shared property value. The contained value is always exactly
/// of the property's value type (or one of the marker types
/// [`UnsetValueType`] and [`DoNothingType`]).
pub type BoxedValue = Rc<dyn AnyValue>;

/// The type of the marker value that represents an unset property value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsetValueType;

impl fmt::Display for UnsetValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(unset)")
    }
}

/// The type of the marker value that tells a binding target to do nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoNothingType;

impl fmt::Display for DoNothingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(do nothing)")
    }
}

/// A callback invoked before and after a property change notification is
/// raised on an object (`true` before, `false` after).
pub type NotifyingCallback = Box<dyn Fn(&FerroObject, bool)>;

// Installed only by an explicit inheritance registration. The property owns
// this capability; its metadata may be overridden without changing whether
// the property inherits or which Rust value type its entries contain.
pub(crate) type InheritedValueChanged = fn(
    &dyn EffectiveValueDyn,
    &FerroObject,
    Option<&dyn EffectiveValueDyn>,
    Option<&dyn EffectiveValueDyn>,
);

/// Only the payload copy depends on T. Keep implicit conversion lookup and
/// the converted box's ownership in one cold implementation (the `TryConvert`
/// of the property, see
/// [`ValueTypes::try_convert_implicit`](crate::data::core::ValueTypes::try_convert_implicit)).
pub(crate) fn convert_property_value<T: PropertyValue>(property: &FerroProperty, value: &dyn Any) -> Option<T> {
    let mut typed = None::<T>;
    copy_converted_property_value(property, value, &mut typed);
    typed
}

/// The typed destination of an untyped value: `Option<T>`, which takes a copy of
/// the payload when it is a `T`. The one typed operation of the untyped routes
/// of a property (conversion, binding values, notifications), shared by all of
/// them: one implementation per value type.
pub(crate) trait PayloadSlot {
    /// Stores a copy of `value` if it is of the type of the slot (and nothing
    /// otherwise); whether it was.
    fn copy_payload(&mut self, value: &dyn Any) -> bool;
}

impl<T: Clone + 'static> PayloadSlot for Option<T> {
    fn copy_payload(&mut self, value: &dyn Any) -> bool {
        *self = value.downcast_ref::<T>().cloned();
        self.is_some()
    }
}

/// The untyped destination of an untyped value: a box of its own, which takes
/// a copy of the payload when it is of the property's value type. The slot of
/// the type-erased binding observers, which keep their values boxed; the copy
/// is the [`route_box_value`](PropertyRoutes::route_box_value) of the
/// property.
pub(crate) struct BoxedPayloadSlot {
    routes: &'static dyn PropertyRoutes,
    value: Option<BoxedValue>,
}

impl BoxedPayloadSlot {
    #[inline]
    pub(crate) fn new(property: &FerroProperty) -> Self {
        Self { routes: property.routes(), value: None }
    }

    /// The copy of the payload, if there is one.
    #[inline]
    pub(crate) fn into_value(self) -> Option<BoxedValue> {
        self.value
    }
}

impl PayloadSlot for BoxedPayloadSlot {
    fn copy_payload(&mut self, value: &dyn Any) -> bool {
        self.value = self.routes.route_box_value(value);
        self.value.is_some()
    }
}

/// The value of `value` if it holds exactly a `T`: moved out of the box when
/// no one else holds it, else a clone. The typed end of the type-erased
/// binding observers, which hand a box they made themselves (the copy of a
/// converted or notified value) without a second copy.
pub(crate) fn take_boxed_value<T: PropertyValue>(value: BoxedValue) -> Option<T> {
    let value: Rc<dyn Any> = value;
    let value = value.downcast::<T>().ok()?;
    Some(Rc::try_unwrap(value).unwrap_or_else(|value| T::clone(&value)))
}

/// The parts of `value` if it is a `BindingValue<T>`, with its value copied
/// into a box of its own: the binding value case of an untyped binding, the
/// one that names `T` (see
/// [`route_unpack_binding_value`](PropertyRoutes::route_unpack_binding_value)).
pub(crate) fn unpack_binding_value<T: PropertyValue>(value: &dyn Any) -> Option<BindingValue<BoxedValue>> {
    let value = value.downcast_ref::<BindingValue<T>>()?;
    let payload = value.has_value().then(|| Rc::new(value.value().clone()) as BoxedValue);
    Some(BindingValue::from_notification_parts(value.value_type(), payload, value.error().cloned()))
}

#[inline(never)]
pub(crate) fn copy_converted_property_value(
    property: &FerroProperty,
    value: &dyn Any,
    copy_value: &mut dyn PayloadSlot,
) -> bool {
    let Some(converted) = crate::data::core::ValueTypes::try_convert_implicit(value, property.property_type()) else {
        return false;
    };
    let copied = copy_value.copy_payload(converted.as_any());
    // A converter's temporary may have an observable destructor. Release it
    // before validation, coercion or publication; it also drops if Clone panics.
    drop(converted);
    copied
}

/// Untyped operations that a property routes to its typed implementation.
pub(crate) trait PropertyRoutes {
    fn create_effective_value(&'static self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn>;
    fn route_clear_value(&'static self, o: &FerroObject);
    fn route_coerce_default_value(&'static self, o: &FerroObject);
    fn route_get_value(&'static self, o: &FerroObject) -> BoxedValue;
    fn route_get_base_value(&'static self, o: &FerroObject) -> BoxedValue;
    fn route_get_default_value(&'static self, type_: &'static TypeInfo) -> BoxedValue;
    fn route_set_value(&'static self, o: &FerroObject, value: &dyn Any, priority: BindingPriority)
        -> Option<Rc<dyn IDisposable>>;
    fn route_set_current_value(&'static self, o: &FerroObject, value: &dyn Any);
    fn route_set_direct_value_unchecked(&'static self, o: &FerroObject, entry: &dyn IValueEntry);
    fn route_bind(
        &'static self,
        o: &FerroObject,
        source: Rc<dyn IObservable<BoxedValue>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable>;
    fn route_get_metadata(&'static self, type_: &'static TypeInfo) -> FerroPropertyMetadata;
    fn route_unregister(&'static self, type_: &'static TypeInfo);
    fn route_is_valid_value(&'static self, value: &dyn Any) -> bool;
    /// The typed property behind the routes, for downcasting.
    fn as_any(&'static self) -> &'static dyn Any;
    /// Clones `value`, which must hold exactly the property's value type `T`,
    /// into `out`, which must be an `&mut Option<S>`: `S` is the type the value
    /// store keeps values of `T` as ([`StyledValue::Storage`](crate::StyledValue::Storage)),
    /// `T` itself for a direct property. Returns false if either has a
    /// different type.
    fn route_copy_value(&'static self, value: &dyn Any, out: &mut dyn Any) -> bool;
    /// Calls `f` with `old` and `new`, values of the type the value store
    /// keeps values of the property as, as values of the property's value
    /// type: the values of a change notification of a styled property whose
    /// value type is kept as a value of another type
    /// ([`StyledValue::SHARED_STORAGE`](crate::StyledValue::SHARED_STORAGE)).
    fn route_with_values(
        &'static self,
        old: Option<&dyn Any>,
        new: &dyn Any,
        f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any),
    ) {
        f(old, new)
    }
    /// Clones `value`, which must hold exactly the property's value type `T`,
    /// into a box of its own. `None` if it has a different type.
    fn route_box_value(&'static self, value: &dyn Any) -> Option<BoxedValue>;
    /// Whether the metadata of the property for the type of `owner` enables
    /// data validation; read without copying the metadata. The binding
    /// observers of untyped sources read it once, when they are created.
    fn route_enable_data_validation(&'static self, owner: &FerroObject) -> Option<bool>;
    /// The parts of `value` if it is a `BindingValue<T>` of the property's
    /// value type `T`, with the value in a box of its own; `None` if it is not.
    fn route_unpack_binding_value(&'static self, value: &dyn Any) -> Option<BindingValue<BoxedValue>>;
    /// Sets the local value of a styled property from a box that holds exactly
    /// the property's value type: the typed `set_local_value` of the store,
    /// for an untyped local binding. Only supported by styled properties.
    fn route_set_local_value(&'static self, _o: &FerroObject, _value: BoxedValue) {
        panic!("not supported for direct properties");
    }
    /// Sets a direct property from an untyped binding value whose value, if
    /// any, holds exactly the property's value type: the typed
    /// `set_direct_value_unchecked` of an untyped direct binding. Only
    /// supported by direct properties.
    fn route_set_direct_binding_value(&'static self, _o: &FerroObject, _value: BindingValue<BoxedValue>) {
        panic!("not supported for styled properties");
    }
}

type ChangedHandler = dyn Fn(&FerroPropertyChangedEventArgs<'_>);

/// The stream of change notifications of a property across all objects.
pub struct PropertyChangedObservable {
    handlers: HandlerList<ChangedHandler>,
    /// Who hears a change of the property that reaches an object from an ancestor: kept for a
    /// property that inherits only.
    hearing: Option<Box<Hearing>>,
}

/// Whether a change of an inherited property that reaches an object from an ancestor is heard
/// by an object of a class.
#[derive(Clone, Copy)]
enum Heard {
    /// No handler of the class reacts to the property and the property has no handler for it.
    No,
    Yes,
    /// Only while the question is `true` for the object (`InheritedReaction::While`, a class
    /// handler with a condition).
    One(Question),
    /// Only while one of several questions is: their place in `Hearing::asked`.
    While(u16),
}

/// A question to an object.
type Question = fn(&FerroObject) -> bool;

/// The questions kept for a class: more than these are answered with yes.
type Questions = [Option<Question>; 3];

/// What is known of who hears a change of an inherited property (FerroUI deviation,
/// desktop-performance.md section 9.101). The answers are for a class; they are made when a
/// class is first asked for and dropped when a handler of the property comes or goes.
struct Hearing {
    /// The class each handler of the property is for, with its token (`None` for a handler of
    /// every class), and the question it asks an object before it runs, where it has one.
    targets: RefCell<Vec<(u64, Option<&'static TypeIdentity>, Option<Question>)>>,
    /// The answers for the classes asked last: a walk over a tree asks for a handful of classes
    /// in turn. A class sits at the place made of a few bits of its address or at the next one,
    /// so that two classes the link happens to put at one place do not evict each other.
    recent: [Cell<(*const TypeInfo, Heard)>; RECENT],
    by_class: RefCell<HashMap<*const TypeInfo, Heard, BuildHasherDefault<ClassHasher>>>,
    /// The questions of the classes that have some.
    asked: RefCell<Vec<Questions>>,
    /// The questions that reached `by_class`.
    #[cfg(test)]
    past_recent: Cell<u32>,
}

/// The places of `Hearing::recent`.
const RECENT: usize = 8;

/// The first place of a class in `Hearing::recent`.
#[inline]
fn recent_place(class: *const TypeInfo) -> usize {
    let address = class as usize;
    ((address >> 4) ^ (address >> 8) ^ (address >> 12)) & (RECENT - 1)
}

/// The second place of a class whose first place is `place`.
#[inline]
fn second_recent_place(place: usize) -> usize {
    (place + 1) & (RECENT - 1)
}

/// The hash of the address of the type information of a class: the addresses of statics, which
/// differ above their alignment.
#[derive(Default)]
struct ClassHasher(u64);

impl Hasher for ClassHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 << 8) | u64::from(*byte);
        }
    }

    fn write_usize(&mut self, address: usize) {
        self.0 = (address as u64 >> 3).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

impl Hearing {
    fn new() -> Self {
        Self {
            targets: RefCell::new(Vec::new()),
            recent: std::array::from_fn(|_| Cell::new((std::ptr::null(), Heard::Yes))),
            by_class: RefCell::new(HashMap::default()),
            asked: RefCell::new(Vec::new()),
            #[cfg(test)]
            past_recent: Cell::new(0),
        }
    }

    fn forget(&self) {
        for recent in &self.recent {
            recent.set((std::ptr::null(), Heard::Yes));
        }
        self.by_class.borrow_mut().clear();
        self.asked.borrow_mut().clear();
    }

    fn handler_added(&self, token: u64, target: Option<&'static TypeIdentity>, question: Option<Question>) {
        self.targets.borrow_mut().push((token, target, question));
        self.forget();
    }

    fn handler_removed(&self, token: u64) {
        self.targets.borrow_mut().retain(|(candidate, _, _)| *candidate != token);
        self.forget();
    }

    /// The answer for a class: one comparison when it sits at its first place, a second one
    /// when it sits at the next. Both stay inline, since a walk over a tree whose classes share
    /// a place finds one of them at its second place at every step.
    #[inline]
    fn of_class(&self, class: &'static TypeInfo, property: &'static FerroProperty) -> Heard {
        let place = recent_place(class);
        let (recent, heard) = self.recent[place].get();
        if std::ptr::eq(recent, class) {
            return heard;
        }
        let (recent, heard) = self.recent[second_recent_place(place)].get();
        if std::ptr::eq(recent, class) {
            return heard;
        }
        self.past_recent(place, class, property)
    }

    #[cold]
    #[inline(never)]
    fn past_recent(&self, place: usize, class: &'static TypeInfo, property: &'static FerroProperty) -> Heard {
        #[cfg(test)]
        self.past_recent.set(self.past_recent.get() + 1);
        let known = self.by_class.borrow().get(&std::ptr::from_ref(class)).copied();
        let heard = known.unwrap_or_else(|| {
            let heard = self.find(class, property);
            self.by_class.borrow_mut().insert(class, heard);
            heard
        });
        self.remember(place, class, heard);
        heard
    }

    /// Keeps the answer for a class at an empty one of its two places, else at the first. What
    /// the first place held moves to the second when that is a place of its class too; a class
    /// visiting from the place before could not be found there, and is dropped.
    fn remember(&self, place: usize, class: *const TypeInfo, heard: Heard) {
        let first = &self.recent[place];
        let second = &self.recent[second_recent_place(place)];
        let held = first.get();
        if held.0.is_null() {
            first.set((class, heard));
        } else if second.get().0.is_null() {
            second.set((class, heard));
        } else {
            if recent_place(held.0) == place {
                second.set(held);
            }
            first.set((class, heard));
        }
    }

    #[cold]
    #[inline(never)]
    fn ask(&self, questions: u16, object: &FerroObject) -> bool {
        let questions = self.asked.borrow()[usize::from(questions)];
        questions.iter().flatten().any(|question| question(object))
    }

    fn find(&self, class: &'static TypeInfo, property: &'static FerroProperty) -> Heard {
        let mut questions: Questions = [None; 3];
        let mut asked = 0;
        let mut ask = |question: Question| {
            let kept = asked < questions.len();
            if kept {
                questions[asked] = Some(question);
                asked += 1;
            }
            kept
        };
        // The handlers of the property for the class, or for every class.
        for (_, target, question) in self.targets.borrow().iter() {
            if target.is_none_or(|target| class.derives_from(target)) {
                match question {
                    Some(question) if ask(*question) => {}
                    _ => return Heard::Yes,
                }
            }
        }
        // The change handlers of the class and of its base classes.
        let mut current = Some(class);
        while let Some(layer) = current {
            match layer.change_handler_layer() {
                ChangeHandlerLayer::Absent => {}
                ChangeHandlerLayer::Unknown => return Heard::Yes,
                ChangeHandlerLayer::Stated(reactions) => match reactions.reaction_to(property) {
                    InheritedReaction::Never => {}
                    InheritedReaction::Always => return Heard::Yes,
                    InheritedReaction::While(question) => {
                        if !ask(question) {
                            return Heard::Yes;
                        }
                    }
                },
            }
            current = layer.base_type();
        }
        match (asked, questions[0]) {
            (0, _) => return Heard::No,
            (1, Some(question)) => return Heard::One(question),
            _ => {}
        }
        let mut kept = self.asked.borrow_mut();
        let place = kept.iter().position(|candidate| *candidate == questions).unwrap_or_else(|| {
            kept.push(questions);
            kept.len() - 1
        });
        u16::try_from(place).map_or(Heard::Yes, Heard::While)
    }
}

impl PropertyChangedObservable {
    fn new(inherits: bool) -> Self {
        Self { handlers: HandlerList::new(), hearing: inherits.then(|| Box::new(Hearing::new())) }
    }

    /// Whether a change of the property that reaches `object` from an ancestor has anything to
    /// reach there but the listeners of the object: a handler of the property for the class of
    /// the object, or a change handler of the class that reacts to the property. `true` where
    /// that is not known.
    #[inline]
    pub(crate) fn hears_inherited(&self, object: &FerroObject, property: &'static FerroProperty) -> bool {
        let Some(hearing) = &self.hearing else {
            return true;
        };
        match hearing.of_class(object.get_type(), property) {
            Heard::Yes => true,
            Heard::No => false,
            Heard::One(question) => question(object),
            Heard::While(questions) => hearing.ask(questions, object),
        }
    }

    fn subscribe_for(
        &'static self,
        target: Option<&'static TypeIdentity>,
        question: Option<Question>,
        handler: Rc<ChangedHandler>,
    ) -> Subscription {
        let token = self.handlers.add(handler);
        if let Some(hearing) = &self.hearing {
            hearing.handler_added(token, target, question);
        }
        Subscription::new(Disposable::create(move || {
            self.handlers.remove(token);
            if let Some(hearing) = &self.hearing {
                hearing.handler_removed(token);
            }
        }))
    }

    /// Whether the property has class-wide observers. Controls with expensive
    /// computed values can avoid materializing notification values when no
    /// object or property observer exists.
    pub fn has_observers(&self) -> bool {
        !self.handlers.is_empty()
    }

    /// Subscribes to changes of the property on any object.
    ///
    /// Dropping the returned guard removes the handler; call
    /// [`detach`](Subscription::detach) to keep it for the life of the
    /// property.
    pub fn subscribe(&'static self, handler: impl Fn(&FerroPropertyChangedEventArgs<'_>) + 'static) -> Subscription {
        self.subscribe_for(None, None, Rc::new(handler))
    }

    /// Subscribes a handler that is only invoked for objects of class
    /// `TTarget` (or classes derived from it).
    ///
    /// Dropping the returned guard removes the handler; call
    /// [`detach`](Subscription::detach) to keep it for the life of the
    /// property, as a class handler registered once for its class does.
    pub fn add_class_handler<TTarget: ObjectType>(
        &'static self,
        handler: impl Fn(&TTarget, &FerroPropertyChangedEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.subscribe_for(
            Some(TTarget::IDENTITY),
            None,
            Rc::new(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if let Some(target) = e.sender().downcast_ref::<TTarget>() {
                    handler(target, e);
                }
            }),
        )
    }

    /// Subscribes a handler that is only invoked for the objects of class `TTarget` (or of
    /// classes derived from it) that `concerns` is `true` for when the change is raised: a class
    /// handler that looks at a state of the object first and leaves where it is not concerned
    /// states that as its condition. An inherited value that reaches an object the handler is
    /// not concerned with need not be raised there for it (FerroUI addition,
    /// desktop-performance.md section 9.101). `concerns` is only asked for objects of class
    /// `TTarget`.
    pub fn add_class_handler_where<TTarget: ObjectType>(
        &'static self,
        concerns: fn(&FerroObject) -> bool,
        handler: impl Fn(&TTarget, &FerroPropertyChangedEventArgs<'_>) + 'static,
    ) -> Subscription {
        self.subscribe_for(
            Some(TTarget::IDENTITY),
            Some(concerns),
            Rc::new(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if let Some(target) = e.sender().downcast_ref::<TTarget>() {
                    if concerns(e.sender()) {
                        handler(target, e);
                    }
                }
            }),
        )
    }

    #[inline]
    pub(crate) fn notify(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.handlers.is_empty() {
            return;
        }
        crate::perf_count!(PropertyChangesWithPropertyHandlers);
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(e);
        }
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

/// The state of a property that the properties added to other owners share with it.
struct SharedState {
    /// The value stores that hold an effective value of the property, when it inherits: while
    /// none does, every element inherits its default, and a lookup need not walk the ancestors.
    /// An addition (FerroUI deviation, desktop-performance.md section 9.21).
    holders: Cell<u32>,
    /// The classes whose `on_property_changed` reacts to a change of the property, a bit for
    /// each ([`PropertyChangeHandler`](crate::PropertyChangeHandler)). An addition (FerroUI
    /// deviation, desktop-performance.md section 9.59).
    change_handlers: Cell<u32>,
}

/// Base class for properties: the untyped identity and description of a
/// property that can be set on a [`FerroObject`].
pub struct FerroProperty {
    name: Box<str>,
    property_type: TypeId,
    property_type_name: &'static str,
    owner_type: &'static TypeInfo,
    id: u32,
    inherits: bool,
    inherited_value_changed: Cell<Option<InheritedValueChanged>>,
    /// What the property has in common with the properties added to other owners: they are the
    /// same property.
    shared: Rc<SharedState>,
    is_attached: bool,
    is_direct: bool,
    is_read_only: bool,
    assign_binding: Cell<bool>,
    host_type: Cell<Option<&'static TypeInfo>>,
    notifying: Option<Rc<dyn Fn(&FerroObject, bool)>>,
    changed: Rc<PropertyChangedObservable>,
    routes: OnceCell<&'static dyn PropertyRoutes>,
    /// The operations of a styled property that depend on its value type
    /// ([`StyledCodec`](crate::styled_property::StyledCodec)): what its routes and the store ask
    /// of the typed property. Empty for a direct property.
    styled_codec: OnceCell<&'static dyn crate::styled_property::StyledCodec>,
    /// The members of the value type when it is an enumeration the store keeps as an
    /// [`EnumValue`](crate::EnumValue) ([`StyledValue::MEMBERS`]): what a change notification
    /// hands out for a stored member. Empty otherwise.
    members: &'static [(i64, &'static dyn Any)],
    /// Whether the property validates its values (a styled property with a validation callback):
    /// [`is_valid_value`](Self::is_valid_value) asks its routes only then.
    validates: bool,
}

pub(crate) struct FerroPropertyInit {
    pub inherits: bool,
    pub is_attached: bool,
    pub is_direct: bool,
    pub is_read_only: bool,
}

impl FerroProperty {
    pub(crate) fn new<T: 'static>(
        name: &str,
        owner_type: &'static TypeInfo,
        init: FerroPropertyInit,
        notifying: Option<NotifyingCallback>,
    ) -> Self {
        Self::new_core(name, owner_type, init, notifying, TypeId::of::<T>(), std::any::type_name::<T>())
    }

    // Identity, allocation and notification setup are independent of the value
    // type. Keep one copy while preserving the original typed property facade.
    #[inline(never)]
    fn new_core(
        name: &str,
        owner_type: &'static TypeInfo,
        init: FerroPropertyInit,
        notifying: Option<NotifyingCallback>,
        property_type: TypeId,
        property_type_name: &'static str,
    ) -> Self {
        assert!(!name.contains('.'), "'name' may not contain periods.");
        Self {
            name: name.into(),
            property_type,
            property_type_name,
            owner_type,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            inherits: init.inherits,
            inherited_value_changed: Cell::new(None),
            shared: Rc::new(SharedState { holders: Cell::new(0), change_handlers: Cell::new(0) }),
            is_attached: init.is_attached,
            is_direct: init.is_direct,
            is_read_only: init.is_read_only,
            assign_binding: Cell::new(false),
            host_type: Cell::new(None),
            notifying: notifying.map(Rc::from),
            changed: Rc::new(PropertyChangedObservable::new(init.inherits)),
            routes: OnceCell::new(),
            styled_codec: OnceCell::new(),
            members: &[],
            validates: false,
        }
    }

    /// The property with the members of its value type ([`StyledValue::MEMBERS`]).
    pub(crate) fn with_members(mut self, members: &'static [(i64, &'static dyn Any)]) -> Self {
        self.members = members;
        self
    }

    /// The property with a validation callback or without.
    pub(crate) fn with_validation(mut self, validates: bool) -> Self {
        self.validates = validates;
        self
    }

    /// The members of the value type, when it is an enumeration kept as an
    /// [`EnumValue`](crate::EnumValue).
    #[inline]
    pub(crate) fn members(&self) -> &'static [(i64, &'static dyn Any)] {
        self.members
    }

    /// Creates the base of a property that shares its identity with `source`
    /// but has a different owner.
    pub(crate) fn for_added_owner(source: &FerroProperty, owner_type: &'static TypeInfo, is_read_only: bool) -> Self {
        Self {
            name: source.name.clone(),
            property_type: source.property_type,
            property_type_name: source.property_type_name,
            owner_type,
            id: source.id,
            inherits: source.inherits,
            inherited_value_changed: source.inherited_value_changed.clone(),
            shared: source.shared.clone(),
            is_attached: source.is_attached,
            is_direct: source.is_direct,
            is_read_only,
            assign_binding: source.assign_binding.clone(),
            host_type: source.host_type.clone(),
            notifying: source.notifying.clone(),
            changed: source.changed.clone(),
            routes: OnceCell::new(),
            styled_codec: OnceCell::new(),
            members: source.members,
            validates: source.validates,
        }
    }

    pub(crate) fn set_styled_codec(&self, codec: &'static dyn crate::styled_property::StyledCodec) {
        if self.styled_codec.set(codec).is_err() {
            panic!("the codec of the property is already set");
        }
    }

    /// The codec of a styled property ([`StyledCodec`](crate::styled_property::StyledCodec)).
    #[inline]
    /// The codec of a styled property, `None` for a direct property.
    #[inline]
    pub(crate) fn styled_codec_if_any(&self) -> Option<&'static dyn crate::styled_property::StyledCodec> {
        self.styled_codec.get().copied()
    }

    pub(crate) fn styled_codec(&self) -> &'static dyn crate::styled_property::StyledCodec {
        *self.styled_codec.get().expect("a styled property has a codec")
    }

    pub(crate) fn set_routes(&self, routes: &'static dyn PropertyRoutes) {
        if self.routes.set(routes).is_err() {
            panic!("property routes are already set");
        }
    }

    // Registration fills this after construction and before exposing the
    // property through the registry. Private inheriting constructors must do
    // the same explicitly; a universal typed constructor must not name the
    // algorithm and thereby retain it for non-inheriting value types.
    pub(crate) fn set_inherited_value_changed(&self, callback: Option<InheritedValueChanged>) {
        debug_assert_eq!(self.inherits, callback.is_some());
        debug_assert!(self.inherited_value_changed.get().is_none());
        self.inherited_value_changed.set(callback);
    }

    pub(crate) fn inherited_value_changed(&self) -> InheritedValueChanged {
        self.inherited_value_changed.get().expect("inheriting property has no change notifier")
    }

    #[inline]
    pub(crate) fn routes(&self) -> &'static dyn PropertyRoutes {
        *self.routes.get().expect("property is not fully registered")
    }

    /// The class an attached property is declared for: the `THost` of its
    /// registration, the type of the objects its accessors (`GetX`/`SetX`)
    /// take. `None` for properties that are not attached.
    #[inline]
    pub fn host_type(&self) -> Option<&'static TypeInfo> {
        self.host_type.get()
    }

    /// The name of the property.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The type of the property's value.
    #[inline]
    pub fn property_type(&self) -> TypeId {
        self.property_type
    }

    /// The Rust type name of the property's value type.
    #[inline]
    pub fn property_type_name(&self) -> &'static str {
        self.property_type_name
    }

    // The errors of the typed property routes. They only depend on the
    // property, so they are compiled once instead of once per value type.

    /// The message of a value that is not of the value type of the property.
    #[cold]
    pub(crate) fn invalid_value_message(&self) -> String {
        format!("Invalid value for property '{}': expected {}.", self.name(), self.property_type_name())
    }

    /// Panics for a value that is not of the value type of the property.
    #[cold]
    #[track_caller]
    pub(crate) fn invalid_value_type(&self) -> ! {
        panic!("Invalid value for property '{}': expected {}.", self.name(), self.property_type_name())
    }

    /// Panics for a value that the validation of the property rejects.
    #[cold]
    #[track_caller]
    pub(crate) fn value_not_valid(&self) -> ! {
        panic!("the value is not valid for property '{}'", self.name())
    }

    /// The class that registered the property.
    #[inline]
    pub fn owner_type(&self) -> &'static TypeInfo {
        self.owner_type
    }

    /// Whether the property inherits its value.
    #[inline]
    pub fn inherits(&self) -> bool {
        self.inherits
    }

    /// The value stores that hold an effective value of the property (inheriting properties).
    pub(crate) fn holders(&self) -> u32 {
        self.shared.holders.get()
    }

    pub(crate) fn add_holder(&self) {
        self.shared.holders.set(self.shared.holders.get() + 1);
    }

    pub(crate) fn remove_holder(&self) {
        self.shared.holders.set(self.shared.holders.get().saturating_sub(1));
    }

    /// The classes whose `on_property_changed` reacts to the property, a bit for each.
    #[inline]
    pub(crate) fn change_handlers(&self) -> u32 {
        self.shared.change_handlers.get()
    }

    pub(crate) fn add_change_handlers(&self, bits: u32) {
        self.shared.change_handlers.set(self.shared.change_handlers.get() | bits);
    }

    /// Whether the `on_property_changed` of a class said that it reacts to a change of this
    /// property ([`PropertyChangeHandler::reacts_to`](crate::PropertyChangeHandler::reacts_to)).
    #[inline]
    pub fn has_change_handler(&self, handler: crate::PropertyChangeHandler) -> bool {
        self.change_handlers() & handler.bit() != 0
    }

    /// Whether this is an attached property.
    #[inline]
    pub fn is_attached(&self) -> bool {
        self.is_attached
    }

    /// Whether this is a direct property.
    #[inline]
    pub fn is_direct(&self) -> bool {
        self.is_direct
    }

    /// Whether this is a readonly property.
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Whether a binding given for this property in markup is assigned to
    /// the property as its value rather than bound to it (the "assign
    /// binding" attribute of the managed property). The property's value
    /// type is then a binding type. A fact about the definition: it is the
    /// same for every owner and is not part of the metadata.
    #[inline]
    pub fn assign_binding(&self) -> bool {
        self.assign_binding.get()
    }

    /// Marks the property as one that bindings are assigned to. Part of the
    /// definition of the property: call it where the property is registered
    /// (styled and attached properties state it through
    /// [`StyledPropertyOptions::assign_binding`]).
    pub fn set_assign_binding(&self, value: bool) {
        self.assign_binding.set(value);
    }

    /// An integer ID that uniquely identifies the property; shared by all
    /// owners added with `add_owner`.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Notifications of changes to this property on any object.
    ///
    /// The handlers are invoked after the object's own change handling.
    #[inline]
    pub fn changed(&'static self) -> &'static PropertyChangedObservable {
        // The observable is shared between a property and its added owners.
        &self.changed
    }

    #[inline]
    pub(crate) fn notify_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.changed.notify(e)
    }

    #[inline]
    pub(crate) fn notifying(&self) -> Option<&dyn Fn(&FerroObject, bool)> {
        self.notifying.as_deref()
    }

    /// Gets the base metadata of the property on the specified type.
    pub fn get_metadata(&self, type_: &'static TypeInfo) -> FerroPropertyMetadata {
        self.routes().route_get_metadata(type_)
    }

    /// Gets the base metadata of the property on the type of `owner`.
    pub fn get_metadata_for(&self, owner: &FerroObject) -> FerroPropertyMetadata {
        self.routes().route_get_metadata(owner.get_type())
    }

    /// The default value of the property for `type_`, untyped: the default of
    /// the metadata of a styled or attached property (upstream's internal
    /// `IStyledPropertyAccessor.GetDefaultValue(Type)`), the unset value of a
    /// direct property. For tests only: the snapshot of the property defaults
    /// (`property_defaults_tests` of `ferroui-controls`) reads it.
    #[cfg(feature = "testing")]
    #[doc(hidden)]
    pub fn default_value_boxed(&self, type_: &'static TypeInfo) -> BoxedValue {
        self.routes().route_get_default_value(type_)
    }

    /// Removes the metadata registered for the property on the specified
    /// type.
    pub fn unregister(&self, type_: &'static TypeInfo) {
        self.routes().route_unregister(type_)
    }

    /// Checks whether `value` is valid for the property: of the property's
    /// value type and accepted by its validation callback.
    pub fn is_valid_value(&self, value: &dyn Any) -> bool {
        // Without a validation callback a value is valid when it is of the value type, which is
        // what the routes would answer.
        if !self.validates {
            return value.type_id() == self.property_type;
        }
        self.routes().route_is_valid_value(value)
    }

    /// The property as a styled (or attached) property with value type `T`:
    /// the typed view needed to read, set and bind it without boxing.
    /// `None` for direct properties and when `T` is not the property's value
    /// type.
    pub fn as_styled<T: crate::PropertyValue>(&'static self) -> Option<&'static StyledProperty<T>> {
        // From the codec directly: one call, as from the typed routes before the routes were shared.
        // A typed write of the declarative layer asks this for every value it sets.
        self.styled_codec.get().and_then(|codec| codec.typed().downcast_ref::<StyledProperty<T>>())
    }

    /// The property as a direct property with value type `T`: the typed view needed to set it
    /// without boxing. `None` for styled properties and when `T` is not the property's value
    /// type.
    pub fn as_direct<T: crate::PropertyValue>(&'static self) -> Option<&'static crate::DirectPropertyBase<T>> {
        self.routes.get().and_then(|routes| routes.as_any().downcast_ref::<crate::DirectPropertyBase<T>>())
    }

    /// The marker value that represents an unset property value.
    pub fn unset_value() -> BoxedValue {
        crate::ferro_thread_local! {
            static UNSET: BoxedValue = Rc::new(UnsetValueType);
        }
        UNSET.with(Rc::clone)
    }

    /// Registers a styled property with a default value.
    pub fn register<TOwner: ObjectType, TValue: StyledValue>(
        name: &str,
        default_value: TValue,
    ) -> &'static StyledProperty<TValue> {
        Self::register_with::<TOwner, TValue>(name, StyledPropertyOptions::new(default_value))
    }

    /// Registers a styled property.
    #[inline]
    pub fn register_with<TOwner: ObjectType, TValue: StyledValue>(
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static StyledProperty<TValue> {
        Self::register_with_core(TOwner::TYPE, name, options)
    }

    // Ownership is descriptor data. Only the value's metadata, validation and
    // coercion specialize this installation path; every owner of that value
    // type shares it without changing the typed read/write implementation.
    #[inline(never)]
    fn register_with_core<TValue: StyledValue>(
        owner_type: &'static TypeInfo,
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static StyledProperty<TValue> {
        let assign_binding = options.assign_binding;
        let inherited_value_changed = options.inherited_value_changed;
        let (metadata, inherits, validate, notifying) = options.into_parts();
        let result = StyledProperty::create(name, owner_type, owner_type, metadata, inherits, validate, notifying, false);
        result.set_inherited_value_changed(inherited_value_changed);
        result.set_assign_binding(assign_binding);
        FerroPropertyRegistry::instance().register(owner_type, result);
        result
    }

    /// Registers an attached property with a default value.
    ///
    /// `TOwner` is the class that registers the property and `THost` the class
    /// of objects the property can be set on.
    pub fn register_attached<TOwner: StaticType, THost: ObjectType, TValue: StyledValue>(
        name: &str,
        default_value: TValue,
    ) -> &'static AttachedProperty<TValue> {
        Self::register_attached_with::<TOwner, THost, TValue>(name, StyledPropertyOptions::new(default_value))
    }

    /// Registers an attached property.
    #[inline]
    pub fn register_attached_with<TOwner: StaticType, THost: ObjectType, TValue: StyledValue>(
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static AttachedProperty<TValue> {
        Self::register_attached_with_core(TOwner::TYPE, THost::TYPE, name, options)
    }

    #[inline(never)]
    fn register_attached_with_core<TValue: StyledValue>(
        owner_type: &'static TypeInfo,
        host_type: &'static TypeInfo,
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static AttachedProperty<TValue> {
        let assign_binding = options.assign_binding;
        let inherited_value_changed = options.inherited_value_changed;
        let (metadata, inherits, validate, notifying) = options.into_parts();
        let result = AttachedProperty::create(name, owner_type, host_type, metadata, inherits, validate, notifying);
        result.set_inherited_value_changed(inherited_value_changed);
        result.set_assign_binding(assign_binding);
        result.host_type.set(Some(host_type));
        let registry = FerroPropertyRegistry::instance();
        registry.register(owner_type, result);
        registry.register_attached(host_type, result);
        result
    }

    /// Registers a direct property: a property whose value is stored in a
    /// field of the owner and accessed through a getter and setter.
    pub fn register_direct<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        getter: fn(&TOwner) -> TValue,
        setter: Option<fn(&TOwner, TValue)>,
        unset_value: TValue,
    ) -> &'static DirectProperty<TOwner, TValue> {
        Self::register_direct_with(name, getter, setter, DirectPropertyMetadata::new(Some(unset_value)))
    }

    /// Registers a direct property with explicit metadata.
    pub fn register_direct_with<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        getter: fn(&TOwner) -> TValue,
        setter: Option<fn(&TOwner, TValue)>,
        metadata: DirectPropertyMetadata<TValue>,
    ) -> &'static DirectProperty<TOwner, TValue> {
        let result = DirectProperty::create(name, getter, setter, metadata);
        FerroPropertyRegistry::instance().register(TOwner::TYPE, result);
        result
    }
}

impl PartialEq for FerroProperty {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FerroProperty {}

impl Hash for FerroProperty {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl fmt::Debug for FerroProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl fmt::Display for FerroProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// Options for registering a styled or attached property.
pub struct StyledPropertyOptions<T> {
    default_value: T,
    inherits: bool,
    inherited_value_changed: Option<InheritedValueChanged>,
    default_binding_mode: BindingMode,
    validate: Option<Box<dyn Fn(&T) -> bool>>,
    coerce: Option<Coercion<T>>,
    enable_data_validation: bool,
    assign_binding: bool,
    notifying: Option<NotifyingCallback>,
}

impl<T: StyledValue> StyledPropertyOptions<T> {
    pub fn new(default_value: T) -> Self {
        Self {
            default_value,
            inherits: false,
            inherited_value_changed: None,
            default_binding_mode: BindingMode::OneWay,
            validate: None,
            coerce: None,
            enable_data_validation: false,
            assign_binding: false,
            notifying: None,
        }
    }

    /// Whether the property inherits its value from the parent object.
    pub fn inherits(mut self, inherits: bool) -> Self {
        self.inherits = inherits;
        self.inherited_value_changed = if inherits {
            Some(EffectiveValue::<T::Storage>::notify_inherited_value_changed)
        } else {
            None
        };
        self
    }

    /// The default binding mode for the property.
    pub fn default_binding_mode(mut self, mode: BindingMode) -> Self {
        self.default_binding_mode = mode;
        self
    }

    /// A value validation callback. Setting a value for which it returns
    /// false is rejected.
    pub fn validate(mut self, validate: impl Fn(&T) -> bool + 'static) -> Self {
        self.validate = Some(Box::new(validate));
        self
    }

    /// A value coercion callback.
    pub fn coerce(mut self, coerce: impl Fn(&FerroObject, T) -> T + 'static) -> Self {
        self.coerce = Some(Coercion::new(Rc::new(coerce)));
        self
    }

    /// Whether the property is interested in data validation.
    pub fn enable_data_validation(mut self, enable: bool) -> Self {
        self.enable_data_validation = enable;
        self
    }

    /// Whether a binding given for the property in markup is assigned to the
    /// property instead of bound to it; see [`FerroProperty::assign_binding`].
    pub fn assign_binding(mut self, assign_binding: bool) -> Self {
        self.assign_binding = assign_binding;
        self
    }

    /// A callback invoked before and after change notifications are raised.
    pub fn notifying(mut self, notifying: impl Fn(&FerroObject, bool) + 'static) -> Self {
        self.notifying = Some(Box::new(notifying));
        self
    }

    #[allow(clippy::type_complexity)]
    fn into_parts(self) -> (StyledPropertyMetadata<T>, bool, Option<Box<dyn Fn(&T) -> bool>>, Option<NotifyingCallback>) {
        let mut metadata = StyledPropertyMetadata::new(Some(self.default_value))
            .with_default_binding_mode(self.default_binding_mode)
            .with_enable_data_validation(self.enable_data_validation);
        if let Some(coerce) = self.coerce {
            // Transfer the already registered callback and algorithm. Naming
            // the algorithm here would root it for every options value type,
            // including properties that never supplied a coercer.
            metadata = metadata.with_registered_coercion(coerce);
        }
        (metadata, self.inherits, self.validate, self.notifying)
    }
}

#[cfg(test)]
mod hearing_tests {
    use super::*;
    use crate::StyledElement;

    /// Two classes at one first place: of nine addresses, two share one of the eight places.
    fn two_classes_at_one_place() -> (&'static TypeInfo, &'static TypeInfo) {
        let mut seen: [Option<&'static TypeInfo>; RECENT] = [None; RECENT];
        loop {
            let class: &'static TypeInfo = Box::leak(Box::new(TypeInfo::new("AtOnePlace", None)));
            let place = recent_place(class);
            if let Some(other) = seen[place] {
                return (other, class);
            }
            seen[place] = Some(class);
        }
    }

    #[test]
    fn two_classes_at_one_place_stay_known_across_alternating_questions() {
        let property: &'static FerroProperty = StyledElement::data_context_property();
        let (one, other) = two_classes_at_one_place();
        assert_eq!(recent_place(one), recent_place(other));
        let hearing = Hearing::new();
        hearing.of_class(one, property);
        hearing.of_class(other, property);
        assert_eq!(hearing.past_recent.get(), 2);
        for _ in 0..100 {
            hearing.of_class(one, property);
            hearing.of_class(other, property);
        }
        assert_eq!(hearing.past_recent.get(), 2, "neither class left its two places");
    }

    #[test]
    fn a_class_moved_to_its_second_place_is_found_there_with_both_places_taken() {
        let property: &'static FerroProperty = StyledElement::data_context_property();
        let (one, other) = two_classes_at_one_place();
        let hearing = Hearing::new();
        let place = recent_place(one);
        let stranger: &'static TypeInfo = Box::leak(Box::new(TypeInfo::new("Stranger", None)));
        hearing.recent[place].set((one, Heard::No));
        hearing.recent[second_recent_place(place)].set((stranger, Heard::No));
        hearing.of_class(other, property);
        assert_eq!(hearing.past_recent.get(), 1);
        assert!(matches!(hearing.of_class(one, property), Heard::No), "moved with its answer");
        hearing.of_class(other, property);
        assert_eq!(hearing.past_recent.get(), 1, "both are at their places");
    }

    #[test]
    fn forgetting_empties_both_places() {
        let property: &'static FerroProperty = StyledElement::data_context_property();
        let (one, other) = two_classes_at_one_place();
        let hearing = Hearing::new();
        hearing.of_class(one, property);
        hearing.of_class(other, property);
        hearing.forget();
        hearing.of_class(one, property);
        hearing.of_class(other, property);
        assert_eq!(hearing.past_recent.get(), 4);
    }
}

#[cfg(test)]
mod styled_property_options_tests {
    use super::*;

    #[test]
    fn registration_transfers_the_original_coercer() {
        let options = StyledPropertyOptions::new(0_i32).coerce(|_, n| n.max(0));
        let registered = options.coerce.as_ref().unwrap().callback().clone();
        let (metadata, _, _, _) = options.into_parts();
        assert!(Rc::ptr_eq(&registered, metadata.coerce_value().unwrap()));
    }
}

/// Records the accessor `$name` of `$owner` returning `$property`
/// (`metadata::record_property_accessor`) with the `compiler-metadata`
/// feature of this crate; expands to nothing without it.
#[cfg(feature = "compiler-metadata")]
#[doc(hidden)]
#[macro_export]
macro_rules! __record_property_accessor {
    ($owner:ident, $vis:vis, $name:ident, $property:expr) => {
        $crate::metadata::record_property_accessor(
            <$owner as $crate::StaticType>::TYPE,
            ::std::any::TypeId::of::<Self>(),
            ::std::stringify!($name),
            ::std::stringify!($vis),
            $crate::Registrable::as_registered_property($property),
        )
    };
}

/// Records the accessor `$name` of `$owner` returning `$property` with the
/// `compiler-metadata` feature of this crate; expands to nothing without it.
#[cfg(not(feature = "compiler-metadata"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __record_property_accessor {
    ($owner:ident, $vis:vis, $name:ident, $property:expr) => {};
}

/// Declares the accessor of a single property definition.
///
/// ```ignore
/// impl Border {
///     ferro_property!(pub fn background_property() -> StyledProperty<Option<Ref<Brush>>> {
///         FerroProperty::register::<Border, _>("Background", None)
///     });
/// }
/// ```
///
/// The property is registered the first time the accessor is called on a
/// thread. Property definitions, like the objects that use them, belong to
/// the thread that created them.
///
/// On its own the macro registers nothing until the accessor is called, so a
/// lookup by name can miss the property. Declare the properties of a type in
/// a [`ferro_properties!`] block instead, which also registers them when the
/// type is initialised.
///
/// Where an accessor cannot be written inside the block (it is generated by
/// a local macro, or declared between other items), name the type whose
/// initialisation it belongs to and list it in the block of that type:
///
/// ```ignore
/// ferro_property!(for XYFocus; pub fn left_property() -> AttachedProperty<..> { .. });
///
/// ferro_properties! { impl XYFocus, also [XYFocus::left_property] {} }
/// ```
#[macro_export]
macro_rules! ferro_property {
    // An accessor that belongs to the static initialisation of `$owner`: the
    // first call on a thread first initialises the owner (as reading a
    // static field of a class runs its static initialisation), which
    // registers the properties of the owner in declaration order, this one
    // included.
    (for $owner:ident; $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block) => {
        $(#[$meta])*
        #[inline]
        $vis fn $name() -> &'static $ty {
            $crate::ferro_thread_local! {
                static CELL: ::std::cell::Cell<::std::option::Option<&'static $ty>> =
                    const { ::std::cell::Cell::new(::std::option::Option::None) };
            }
            match CELL.get() {
                ::std::option::Option::Some(property) => property,
                ::std::option::Option::None => {
                    <$owner as $crate::StaticType>::TYPE.ensure_class_init();
                    match CELL.get() {
                        ::std::option::Option::Some(property) => property,
                        // Reached while the owner is being initialised (this
                        // is the registration in declaration order), or for
                        // an owner whose runtime type has no initialisation.
                        ::std::option::Option::None => {
                            let property: &'static $ty = $body;
                            CELL.set(::std::option::Option::Some(property));
                            $crate::__register_value_markup!(property);
                            $crate::__record_property_accessor!($owner, $vis, $name, property);
                            property
                        }
                    }
                }
            }
        }
    };

    ($(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block) => {
        $(#[$meta])*
        #[inline]
        $vis fn $name() -> &'static $ty {
            $crate::ferro_thread_local! {
                static CELL: ::std::cell::Cell<::std::option::Option<&'static $ty>> =
                    const { ::std::cell::Cell::new(::std::option::Option::None) };
            }
            match CELL.get() {
                ::std::option::Option::Some(property) => property,
                ::std::option::Option::None => {
                    let property: &'static $ty = $body;
                    CELL.set(::std::option::Option::Some(property));
                    $crate::__register_value_markup!(property);
                    property
                }
            }
        }
    };
}

/// Registers the markup metadata of the value type of a property that is registered
/// ([`MarkupType::register_used`](crate::metadata::MarkupType::register_used)), when the value
/// type has metadata ([`MarkupTyped`](crate::metadata::MarkupTyped)); nothing otherwise. The
/// choice is made where the property is declared, by the method a reference to a reference to
/// the probe resolves to: the implementation for a value type with metadata (on `&Probe`) takes
/// that receiver as it is and is found first; the one of every type (on `Probe`) is found after
/// one dereference.
///
/// In the same way it makes the value type a used type of the untyped value conversions
/// ([`ValueTypes::register_used`](crate::data::core::ValueTypes::register_used)) when the type
/// (`T`, `Option<T>`, `Rc<T>`, `Option<Rc<T>>`) has registrations
/// ([`ValueTypeRegistration`](crate::data::core::ValueTypeRegistration), #566): its nullable
/// form and its casts are registered with its first property.
#[doc(hidden)]
#[macro_export]
macro_rules! __register_value_markup {
    ($property:expr) => {{
        #[allow(unused_imports)]
        use $crate::{__NoPropertyValueType as _, __NoValueMarkup as _, __RegisterPropertyValueType as _, __RegisterValueMarkup as _};
        let probe = $crate::__ValueMarkupProbe($property);
        (&&probe).__register_value_markup();
        (&&probe).__register_property_value_type();
    }};
}

#[doc(hidden)]
pub struct __ValueMarkupProbe<'a, P: ?Sized>(pub &'a P);

#[doc(hidden)]
pub trait __RegisterValueMarkup {
    fn __register_value_markup(&self);
}

impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::StyledProperty<T>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used_of::<T>();
    }
}

impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<T>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used_of::<T>();
    }
}

impl<O: crate::ObjectType, T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, T>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used_of::<T>();
    }
}

// A nullable value type (`Option<T>`) is found by its metadata too (the nullable form of the type).
impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::StyledProperty<Option<T>>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used_of::<T>();
    }
}

impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<Option<T>>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used_of::<T>();
    }
}

impl<O: crate::ObjectType, T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, Option<T>>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used_of::<T>();
    }
}

#[doc(hidden)]
pub trait __NoValueMarkup {
    #[inline(always)]
    fn __register_value_markup(&self) {}
}

impl<P: ?Sized> __NoValueMarkup for __ValueMarkupProbe<'_, P> {}

/// The registration of the value type of a property with the untyped value conversions
/// ([`__register_value_markup!`](crate::__register_value_markup)): for a property of `T`,
/// `Option<T>`, `Rc<T>` or `Option<Rc<T>>` of a type `T` with registrations.
#[doc(hidden)]
pub trait __RegisterPropertyValueType {
    fn __register_property_value_type(&self);
}

impl<T: PropertyValue + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::StyledProperty<T>>
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: PropertyValue + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::StyledProperty<Option<T>>>
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: ?Sized + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::StyledProperty<Rc<T>>>
where
    Rc<T>: PropertyValue,
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: ?Sized + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::StyledProperty<Option<Rc<T>>>>
where
    Rc<T>: PropertyValue,
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: PropertyValue + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<T>>
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: PropertyValue + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<Option<T>>>
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: ?Sized + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<Rc<T>>>
where
    Rc<T>: PropertyValue,
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<T: ?Sized + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<Option<Rc<T>>>>
where
    Rc<T>: PropertyValue,
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<O: crate::ObjectType, T: PropertyValue + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, T>>
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<O: crate::ObjectType, T: PropertyValue + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, Option<T>>>
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<O: crate::ObjectType, T: ?Sized + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, Rc<T>>>
where
    Rc<T>: PropertyValue,
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

impl<O: crate::ObjectType, T: ?Sized + crate::data::core::ValueTypeRegistration> __RegisterPropertyValueType
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, Option<Rc<T>>>>
where
    Rc<T>: PropertyValue,
{
    #[inline(always)]
    fn __register_property_value_type(&self) {
        crate::data::core::ValueTypes::register_used::<T>();
    }
}

#[doc(hidden)]
pub trait __NoPropertyValueType {
    #[inline(always)]
    fn __register_property_value_type(&self) {}
}

impl<P: ?Sized> __NoPropertyValueType for __ValueMarkupProbe<'_, P> {}

/// Declares the property definitions of a type: the accessors, as
/// [`ferro_property!`] declares them one by one, and their registration as
/// part of the static initialisation of the type.
///
/// ```ignore
/// ferro_properties! {
///     impl Border {
///         /// Defines the `Background` property.
///         pub fn background_property() -> StyledProperty<Option<Ref<Brush>>> {
///             FerroProperty::register::<Border, _>("Background", None)
///         }
///
///         /// Defines the `Child` property (declared by another class).
///         pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
///             Decorator::child_property().add_owner::<Border>()
///         }
///     }
/// }
/// ```
///
/// All the properties are registered, in declaration order (the order of the
/// static field initialisers of the managed class), when the type is
/// initialised on a thread: when the first instance of the class or of a
/// class deriving from it is created, when one of the accessors is first
/// called, or when the property registry is asked about the type, whichever
/// comes first. A lookup by name therefore finds every property, whether or
/// not its accessor was called and whether or not an instance exists. After
/// that an accessor is a thread-local read, exactly as with
/// [`ferro_property!`].
///
/// The owner is a class ([`ferro_class!`](crate::ferro_class)) or a type
/// declared with [`ferro_static_type!`](crate::ferro_static_type).
///
/// Existing `ferro_property!(..);` invocations can be wrapped in the block
/// unchanged:
///
/// ```ignore
/// ferro_properties! { impl Border {
///     ferro_property!(pub fn background_property() -> StyledProperty<Option<Ref<Brush>>> {
///         FerroProperty::register::<Border, _>("Background", None)
///     });
/// } }
/// ```
///
/// A type has one such block. Properties of the type that are declared
/// elsewhere are named by it: `impl Owner, also [path, ..] { .. }` registers
/// the properties of the block first and then calls the listed functions in
/// order. A listed function is either
///
/// * the registration function of another block of accessors, declared with
///   `ferro_properties! { impl Owner, fn name { .. } }` (an upstream partial
///   class in a second file): `also [InputElement::register_gesture_properties]`;
/// * or a single accessor declared with `ferro_property!(for Owner; ..)`
///   (accessors generated by a local macro or interleaved with other items):
///   `also [XYFocus::left_property, XYFocus::right_property]`.
#[macro_export]
macro_rules! ferro_properties {
    (@accessors $owner:ident { $( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block )* }) => {
        $(
            $crate::ferro_property!(for $owner; $(#[$meta])* $vis fn $name() -> $ty $body);
        )*
    };

    // The main block, wrapping `ferro_property!(..);` invocations. The
    // accessors are declared through the macro name written at the call
    // site, so the block works with the import the file already has.
    (
        impl $owner:ident $(, also [$($more:path),* $(,)?])? {
            $( $macro_:ident ! ( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block ) ; )+
        }
    ) => {
        impl $owner {
            $( $macro_!(for $owner; $(#[$meta])* $vis fn $name() -> $ty $body); )+

            /// Registers the properties of the type in declaration order.
            /// Called by the static initialisation of the type.
            #[doc(hidden)]
            pub fn __register_properties() {
                $( let _ = <$owner>::$name(); )+
                $($( $more(); )*)?
            }
        }
    };

    // A part of the properties of a type, wrapping `ferro_property!(..);`.
    (
        impl $owner:ident, fn $part:ident {
            $( $macro_:ident ! ( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block ) ; )+
        }
    ) => {
        impl $owner {
            $( $macro_!(for $owner; $(#[$meta])* $vis fn $name() -> $ty $body); )+

            /// Registers a part of the properties of the type; named by its
            /// main `ferro_properties!` block.
            #[doc(hidden)]
            pub fn $part() {
                $( let _ = <$owner>::$name(); )+
            }
        }
    };

    // A part of the properties of a type.
    (
        impl $owner:ident, fn $part:ident {
            $( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block )*
        }
    ) => {
        impl $owner {
            $crate::ferro_properties!(@accessors $owner { $( $(#[$meta])* $vis fn $name() -> $ty $body )* });

            /// Registers a part of the properties of the type; named by its
            /// main `ferro_properties!` block.
            #[doc(hidden)]
            pub fn $part() {
                $( let _ = <$owner>::$name(); )*
            }
        }
    };

    // The main block.
    (
        impl $owner:ident $(, also [$($more:path),* $(,)?])? {
            $( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block )*
        }
    ) => {
        impl $owner {
            $crate::ferro_properties!(@accessors $owner { $( $(#[$meta])* $vis fn $name() -> $ty $body )* });

            /// Registers the properties of the type in declaration order.
            /// Called by the static initialisation of the type.
            #[doc(hidden)]
            pub fn __register_properties() {
                $( let _ = <$owner>::$name(); )*
                $($( $more(); )*)?
            }
        }
    };
}
