//! The builders of elements: [`El`], the element of a control class, its
//! methods, and the conversions of what a setter takes ([`IntoProp`]).
//!
//! Every class gets a function that starts its element (`button()`) and a
//! trait with a setter per property and a handler per routed event
//! (`ButtonProps`), generated from the sources (`generated/builders.rs`). The
//! setters take what converts to the value of the property: the value, a
//! value that converts to it (`"text"` for an optional string, `8.0` for a
//! thickness), a [`Signal`] or a [`Memo`] (the property follows it), a
//! closure (computed reactively), a two-way signal, a binding, or an element
//! for a property that holds an object (`content`).
//!
//! What the upstream API writes with indexers and operators on properties is
//! here a method of the element:
//!
//! | upstream C#                                   | builder                                      |
//! |-----------------------------------------------|----------------------------------------------|
//! | `control[Grid.RowProperty] = 1`               | `.set(Grid::row_property(), 1)` or `.grid_row(1)` |
//! | `control[!TextBlock.TextProperty] = binding`  | `.bind(TextBlock::text_property(), binding)` |
//! | `control.Bind(Prop, observable)`              | `.text(signal)`, `.text(move \|\| ..)`        |
//! | `!!` / two-way binding                        | `.text(signal.two_way())`                    |
//! | `control.Classes.Add("accent")`               | `.class("accent")`                           |
//! | `control.Name = "x"`                          | `.name("x")`                                 |

use crate::element::{Attach, ControlElement, Handler, Element, ElementClass, IntoChild, IntoChildren, IntoElement, PropEntry, PropValue};
use crate::reactive::{Memo, Signal, TwoWay};
use ferroui_base::data::BindingBase;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent};
use ferroui_base::{AttachedProperty, BoxedValue, DirectProperty, FerroObject, FerroProperty, ObjectType, PropertyValue, Ref, StyledProperty, StyledValue, Upcast};
use ferroui_controls::Control;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

/// A class whose elements can be built: it makes its control.
pub trait ViewClass: ObjectType + Upcast<Control> {
    /// Creates a control of the class.
    fn create() -> Ref<Self>;
}

/// What an element name of `rsx!` resolves to: a control class (every
/// [`ViewClass`]) or a component (`#[component]` declares a type of the name
/// of its function). `Button { .. }` in `rsx!` is `<Button as Builder>::builder()`.
pub trait Builder {
    /// The builder of an element of the name.
    type Output;
    /// Starts an element.
    fn builder() -> Self::Output;

    #[doc(hidden)]
    fn with_editor_source(output: Self::Output, _location: crate::RsxSourceInfo) -> Self::Output {
        output
    }
}

impl<T: ViewClass> Builder for T {
    type Output = El<T>;
    fn builder() -> El<T> {
        El::new()
    }

    #[inline]
    fn with_editor_source(output: El<T>, location: crate::RsxSourceInfo) -> El<T> {
        if crate::hot::record_source_locations() {
            let location = location.with_resolver(crate::hot::source_location);
            return output.prop(crate::RsxSourceInfo::location_property(), Some(location));
        }
        output
    }
}

/// A property of a type known to the compiler: what the setters of an
/// element take.
pub trait TypedProperty: 'static {
    /// The type of the value of the property.
    type Value: StyledValue;
    /// The property, untyped.
    fn untyped(&'static self) -> &'static FerroProperty;
}

impl<T: StyledValue> TypedProperty for StyledProperty<T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        self.as_property()
    }
}

impl<T: StyledValue> TypedProperty for AttachedProperty<T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        let styled: &'static StyledProperty<T> = self;
        styled.as_property()
    }
}

impl<O: ObjectType, T: StyledValue> TypedProperty for DirectProperty<O, T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        let base: &'static ferroui_base::DirectPropertyBase<T> = self;
        base.as_property()
    }
}

/// A conversion of what a setter is given to the value of a property:
/// the value itself, and the conversions that make builders short (text to
/// an optional string, a number to a thickness, a color to a brush).
pub trait IntoValue<T> {
    /// The value of the property.
    fn into_value(self) -> T;
}

impl<T> IntoValue<T> for T {
    fn into_value(self) -> T {
        self
    }
}

/// A conversion of a value of a property back to the type of a signal, for
/// a two-way value: the reverse of [`IntoValue`].
pub trait FromValue<T>: Sized {
    /// The value of the signal, or `None` when the value of the property has
    /// none (the signal keeps its value).
    fn from_value(value: &T) -> Option<Self>;
}

impl<T: Clone> FromValue<T> for T {
    fn from_value(value: &T) -> Option<Self> {
        Some(value.clone())
    }
}

/// The markers that tell the conversions of [`IntoProp`] apart.
pub mod markers {
    /// A value, converted.
    pub struct Value;
    /// A signal.
    pub struct FromSignal<V>(std::marker::PhantomData<V>);
    /// A memo.
    pub struct FromMemo<V>(std::marker::PhantomData<V>);
    /// A reactive closure.
    pub struct FromClosure<V>(std::marker::PhantomData<V>);
    /// A two-way signal.
    pub struct FromTwoWay<V>(std::marker::PhantomData<V>);
    /// An element held by an object property.
    pub struct FromElement;
    /// An element held by a property of a control class.
    pub struct FromControlElement;
    /// A binding.
    pub struct FromBinding;
}

/// What a setter of a property whose value is `T` takes. `M` tells the
/// conversions apart and is inferred.
pub trait IntoProp<T, M> {
    /// The value of the element.
    fn into_prop(self) -> PropValue;
}

/// The values of one type whose boxes are kept, by the last use.
const KEPT_BOXES: usize = 8;

/// The largest value whose box is kept, in bytes.
const KEPT_BOX_SIZE: usize = 32;

/// The identifier of a type is a hash already.
#[derive(Default)]
struct TypeIdHasher(u64);

impl std::hash::Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(*byte);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 ^= value;
    }
}

type KeptBoxes = std::collections::HashMap<TypeId, Box<dyn Any>, std::hash::BuildHasherDefault<TypeIdHasher>>;

ferroui_base::ferro_thread_local! {
    /// For a type of plain values, the boxes of the values used last (`Vec<Rc<T>>`).
    static BOXES: RefCell<KeptBoxes> = RefCell::new(KeptBoxes::default());
}

/// The box of a value of a property.
///
/// A box cannot change and is compared by its value, so elements that give a property the same
/// value can hold the same box, and an application gives the elements of a list or a wall the
/// same few values (a width, an alignment, a margin, a font size): the boxes of the last
/// [`KEPT_BOXES`] values of a type are kept on the thread, and a value among them takes its box
/// from there, where every value of every element asked the allocator for one
/// (docs/porting/desktop-performance.md, section 9.100). Only for plain values of up to
/// [`KEPT_BOX_SIZE`] bytes: a value that owns something (a text, a brush) is as costly to
/// compare as to box, and usually its own.
fn boxed<T: PropertyValue>(value: T) -> BoxedValue {
    if std::mem::needs_drop::<T>() || std::mem::size_of::<T>() > KEPT_BOX_SIZE {
        return Rc::new(value);
    }
    // The table is not there any more while the thread is torn down: the value gets a box of
    // its own then.
    let mut value = Some(value);
    let shared = BOXES.try_with(|boxes| {
        let Some(value) = value.take() else { unreachable!("the value is taken once") };
        let mut boxes = boxes.borrow_mut();
        let kept = boxes.entry(TypeId::of::<T>());
        let kept = kept.or_insert_with(|| Box::new(Vec::<Rc<T>>::with_capacity(KEPT_BOXES)));
        let Some(kept) = kept.downcast_mut::<Vec<Rc<T>>>() else { unreachable!("the boxes are of their type") };
        if let Some(index) = kept.iter().position(|kept| **kept == value) {
            // Towards the front, so that the values in use stay.
            if index > 0 {
                kept.swap(index, index - 1);
            }
            return kept[index.saturating_sub(1)].clone() as BoxedValue;
        }
        // A new value takes the place of the one at the end, where the next new value finds
        // it: values that come once pass through one place and leave the others alone.
        let made = Rc::new(value);
        if kept.len() == KEPT_BOXES {
            kept.pop();
        }
        kept.push(made.clone());
        made as BoxedValue
    });
    match (shared, value) {
        (Ok(shared), _) => shared,
        (Err(_), Some(value)) => Rc::new(value),
        (Err(_), None) => unreachable!("the value is taken only where the table is"),
    }
}

/// The box of the value of a reactive property, kept between the runs of its effect
/// (docs/porting/desktop-performance.md, section 9.94). The property takes a copy of the value
/// and the effect lets the box go, so a run can write its value into the box of the run before,
/// where every run asked the allocator for a box and gave it back. A property that is computed
/// once keeps nothing: the box is kept from the second run on, and only while nothing else
/// holds it.
struct KeptBox<T> {
    kept: std::cell::RefCell<Option<Rc<T>>>,
    ran: std::cell::Cell<bool>,
}

impl<T: PropertyValue> KeptBox<T> {
    fn new() -> Self {
        Self { kept: std::cell::RefCell::new(None), ran: std::cell::Cell::new(false) }
    }

    fn boxed(&self, value: T) -> BoxedValue {
        if !self.ran.replace(true) {
            // The first run, and for most properties the only one.
            return boxed(value);
        }
        let mut kept = self.kept.borrow_mut();
        match kept.as_mut().and_then(Rc::get_mut) {
            Some(inside) => *inside = value,
            None => *kept = Some(Rc::new(value)),
        }
        match &*kept {
            Some(kept) => kept.clone(),
            None => unreachable!("the box was just kept"),
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The control and the property whose effect runs the closure of a reactive property
    /// (`compute_into`): a closure made by the builders takes it and sets the property itself.
    static TARGET: Cell<Option<(Ref<Control>, &'static FerroProperty)>> = const { Cell::new(None) };
}

/// Runs the closure of a reactive property (a value of [`PropValue::Dynamic`], of
/// [`PropValue::Follows`] or the `get` of [`PropValue::TwoWay`]) for the property of a control.
///
/// A closure that the builders made knows the type of its value: it sets the property with it,
/// through the typed setter, with no box and no copy of the value, and this returns `None`
/// (docs/porting/desktop-performance.md, the typed path of #804). The value of any other closure
/// is returned with the control, for the untyped setter. The typed path travels beside the
/// public representation of a property value: the closure finds its target on the thread, so
/// `PropValue` has neither a new variant nor a new field.
pub(crate) fn compute_into(
    control: Ref<Control>,
    property: &'static FerroProperty,
    compute: &dyn Fn() -> BoxedValue,
) -> Option<(Ref<Control>, BoxedValue)> {
    let previous = TARGET.replace(Some((control, property)));
    let value = compute();
    let left = TARGET.replace(previous);
    left.map(|(control, _)| (control, value))
}

/// The closure of a reactive property of type `T`, from what computes its value: run by the
/// effect of the property ([`compute_into`]) it sets the property with the value as it is; run
/// anywhere else it returns the value in the box of the run before ([`KeptBox`]).
fn typed_compute<T: StyledValue>(value: impl Fn() -> T + 'static) -> Rc<dyn Fn() -> BoxedValue> {
    let kept = KeptBox::<T>::new();
    Rc::new(move || {
        // Taken before the value is computed, so that a closure the computation runs (one of a
        // control it mounts) does not find it.
        let target = TARGET.try_with(Cell::take).ok().flatten();
        let value = value();
        match target {
            Some((control, property)) => {
                crate::reactive::untrack(|| {
                    if let Err(value) = control.try_set_value_typed(property, value) {
                        // Not the type of the property: the untyped setter converts it.
                        control.set_value_untyped(property, &value, ferroui_base::data::BindingPriority::LocalValue);
                    }
                });
                // A value that sets nothing, wherever it is given.
                ferroui_base::data::BindingOperations::do_nothing()
            }
            None => kept.boxed(value),
        }
    })
}

impl<T: StyledValue, V: IntoValue<T>> IntoProp<T, markers::Value> for V {
    fn into_prop(self) -> PropValue {
        PropValue::Value(boxed(self.into_value()))
    }
}

impl<T: StyledValue, V: IntoValue<T> + Clone + 'static> IntoProp<T, markers::FromSignal<V>> for Signal<V> {
    fn into_prop(self) -> PropValue {
        PropValue::Follows { source: self.source_id(), compute: typed_compute::<T>(move || self.get().into_value()) }
    }
}

impl<T: StyledValue, V: IntoValue<T> + Clone + 'static> IntoProp<T, markers::FromMemo<V>> for Memo<V> {
    fn into_prop(self) -> PropValue {
        PropValue::Follows { source: self.source_id(), compute: typed_compute::<T>(move || self.get().into_value()) }
    }
}

impl<T: StyledValue, V: IntoValue<T>, F: Fn() -> V + 'static> IntoProp<T, markers::FromClosure<V>> for F {
    fn into_prop(self) -> PropValue {
        PropValue::Dynamic(typed_compute::<T>(move || self().into_value()))
    }
}

impl<T: StyledValue, V: IntoValue<T> + FromValue<T> + Clone + PartialEq + 'static> IntoProp<T, markers::FromTwoWay<V>>
    for TwoWay<V>
{
    fn into_prop(self) -> PropValue {
        let signal = self.0;
        PropValue::TwoWay {
            source: Some(signal.source_id()),
            get: typed_compute::<T>(move || signal.get().into_value()),
            set: Rc::new(move |value: &dyn Any| {
                if let Some(value) = value.downcast_ref::<T>().and_then(V::from_value) {
                    if signal.is_alive() {
                        signal.set_if_changed(value);
                    }
                }
            }),
        }
    }
}

impl<E: IntoElement> IntoProp<Option<BoxedValue>, markers::FromElement> for E {
    fn into_prop(self) -> PropValue {
        PropValue::Element(Rc::new(RefCell::new(Some(self.into_element()))))
    }
}

impl<T: ObjectType + Upcast<Control>, E: IntoElement> IntoProp<Option<Ref<T>>, markers::FromControlElement> for E {
    fn into_prop(self) -> PropValue {
        fn wrap<T: ObjectType>(control: Option<Ref<Control>>) -> BoxedValue {
            Rc::new(control.and_then(|control| control.cast::<T>()))
        }
        PropValue::TypedElement { element: Rc::new(RefCell::new(Some(self.into_element()))), wrap: wrap::<T> }
    }
}

/// A binding of the property system given to a setter
/// (`.text(Bind(binding))`).
pub struct Bind<B: BindingBase + 'static>(pub B);

impl<T: StyledValue, B: BindingBase + 'static> IntoProp<T, markers::FromBinding> for Bind<B> {
    fn into_prop(self) -> PropValue {
        PropValue::Binding(Rc::new(self.0))
    }
}

/// The element of a control class `T`, built with chained setters. Turns
/// into an [`Element`].
pub struct El<T> {
    element: ControlElement,
    _marker: PhantomData<fn() -> T>,
}

impl<T: ViewClass> El<T> {
    /// An element of `T` with nothing set.
    pub fn new() -> Self {
        fn create<T: ViewClass>() -> Ref<Control> {
            T::create().upcast()
        }
        let class = ElementClass {
            name: std::any::type_name::<T>(),
            type_id: std::any::TypeId::of::<T>,
            create: create::<T>,
        };
        Self { element: ControlElement::new(class), _marker: PhantomData }
    }
}

impl<T: ViewClass> Default for El<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The values the list of an element has room for when it gets its first.
const PROPS_AT_FIRST: usize = 8;

/// The part of a builder the generated setters use.
pub trait ElementBuilder: Sized {
    /// The element being built.
    fn element_mut(&mut self) -> &mut ControlElement;

    /// Gives a property a value (the indexer `control[Property] = value`
    /// upstream).
    fn prop<P: TypedProperty + ?Sized, M>(mut self, property: &'static P, value: impl IntoProp<P::Value, M>) -> Self {
        let property = property.untyped();
        let value = value.into_prop();
        let props = &mut self.element_mut().props;
        if props.capacity() == 0 {
            // Room for the values an element usually has, taken once: a list that grows from
            // nothing asks for room three times on the way to eight values.
            props.reserve(PROPS_AT_FIRST);
        }
        props.retain(|entry| !std::ptr::eq(entry.property, property));
        props.push(PropEntry { property, value });
        self
    }

    /// Handles a routed event; the handler gets the control that handles it
    /// and the arguments.
    fn routed<A: IRoutedEventArgs + 'static>(
        mut self,
        event: &'static RoutedEvent<A>,
        handler: impl Fn(&Interactive, &A) + 'static,
    ) -> Self {
        type HandlerCell<A> = RefCell<Rc<dyn Fn(&Interactive, &A)>>;
        fn swap<A: 'static>(old: &dyn Any, new: &dyn Any) {
            if let (Some(old), Some(new)) = (old.downcast_ref::<HandlerCell<A>>(), new.downcast_ref::<HandlerCell<A>>()) {
                let closure = new.borrow().clone();
                *old.borrow_mut() = closure;
            }
        }
        let cell: Rc<HandlerCell<A>> = Rc::new(RefCell::new(Rc::new(handler)));
        let attached = cell.clone();
        let attach: Attach = Rc::new(move |control: &Ref<Control>| {
            // The trampoline calls the closure the cell holds now; it is
            // detached with its owner.
            let cell = attached.clone();
            let token = control.add_handler(event, move |sender, e| {
                let closure = cell.borrow().clone();
                closure(sender, e)
            });
            let weak = control.downgrade();
            crate::reactive::on_cleanup(move || {
                if let Some(control) = weak.upgrade() {
                    control.remove_handler(event, token);
                }
            });
        });
        let identity = event as *const RoutedEvent<A> as *const () as usize;
        self.element_mut().handlers.push(Handler { identity, attach, cell, swap: swap::<A> });
        self
    }
}

impl<T> ElementBuilder for El<T> {
    fn element_mut(&mut self) -> &mut ControlElement {
        &mut self.element
    }
}

impl<T> El<T> {
    /// The key of the element among its siblings.
    pub fn key(mut self, key: impl Into<crate::Key>) -> Self {
        self.element.key = Some(key.into());
        self
    }

    /// The `Name` of the control.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.element.name = Some(name.into());
        self
    }

    /// Adds style classes, separated by spaces (`"accent large"`).
    pub fn class(mut self, classes: &str) -> Self {
        self.element.classes.extend(classes.split_whitespace().map(str::to_string));
        self
    }

    /// Adds style classes, separated by spaces; the same as
    /// [`class`](Self::class).
    pub fn classes(self, classes: &str) -> Self {
        self.class(classes)
    }

    /// Gives a property a value; the same as the generated setter of the
    /// property, for any property (an attached one of another crate, one the
    /// generator left out).
    pub fn set<P: TypedProperty + ?Sized, M>(self, property: &'static P, value: impl IntoProp<P::Value, M>) -> Self {
        self.prop(property, value)
    }

    /// Binds a property with a binding of the property system (the indexer
    /// `control[!Property] = binding` upstream).
    pub fn bind<P: TypedProperty + ?Sized>(mut self, property: &'static P, binding: impl BindingBase + 'static) -> Self {
        let property = property.untyped();
        self.element.props.push(PropEntry { property, value: PropValue::Binding(Rc::new(binding)) });
        self
    }

    /// Handles a routed event with the control that handles it.
    pub fn on<A: IRoutedEventArgs + 'static>(
        self,
        event: &'static RoutedEvent<A>,
        handler: impl Fn(&Interactive, &A) + 'static,
    ) -> Self {
        self.routed(event, handler)
    }

    /// Calls `handler` with the new value whenever `property` changes on the
    /// control (the user typed, a slider moved, a binding set it).
    pub fn on_changed<P: TypedProperty + ?Sized>(mut self, property: &'static P, handler: impl Fn(&P::Value) + 'static) -> Self {
        type ChangedCell<V> = RefCell<Rc<dyn Fn(&V)>>;
        fn swap<V: 'static>(old: &dyn Any, new: &dyn Any) {
            if let (Some(old), Some(new)) = (old.downcast_ref::<ChangedCell<V>>(), new.downcast_ref::<ChangedCell<V>>()) {
                let closure = new.borrow().clone();
                *old.borrow_mut() = closure;
            }
        }
        let untyped = property.untyped();
        let cell: Rc<ChangedCell<P::Value>> = Rc::new(RefCell::new(Rc::new(handler)));
        let attached = cell.clone();
        let attach: Attach = Rc::new(move |control: &Ref<Control>| {
            let cell = attached.clone();
            let subscription = control.property_changed(move |e| {
                if std::ptr::eq(e.property(), untyped) {
                    if let Some(value) = e.new_value().downcast_ref::<P::Value>() {
                        let closure = cell.borrow().clone();
                        closure(value);
                    }
                }
            });
            crate::reactive::on_cleanup(move || subscription.dispose());
        });
        let identity = untyped as *const FerroProperty as usize;
        self.element.handlers.push(crate::element::Handler { identity, attach, cell, swap: swap::<P::Value> });
        self
    }

    /// Calls `f` with the control once it was made and given its values and
    /// children: what a builder cannot say (a control kept in a
    /// [`NodeRef`](crate::NodeRef), a call the class has no property for).
    pub fn with(mut self, f: impl Fn(&Ref<T>) + 'static) -> Self
    where
        T: ObjectType,
    {
        let attach: Attach = Rc::new(move |control: &Ref<Control>| {
            if let Some(control) = control.cast::<T>() {
                f(&control);
            }
        });
        self.element.on_mount.push(attach);
        self
    }

    /// Adds styles to the control (`Styles` of the control): they apply to
    /// it and to the controls below it. They are added when the control is
    /// made.
    pub fn styles(mut self, styles: impl IntoIterator<Item = crate::style::StyleBuilder>) -> Self {
        let styles: Vec<_> = styles.into_iter().map(crate::style::StyleBuilder::build).collect();
        let attach: Attach = Rc::new(move |control: &Ref<Control>| {
            let target = control.styles();
            for style in &styles {
                target.add(style.clone());
            }
        });
        self.element.on_mount.push(attach);
        self
    }

    /// Installs locally scoped visual state groups when this control is mounted.
    pub fn visual_states(mut self, groups: impl IntoIterator<Item = crate::ViewStateGroup>) -> Self {
        let groups: Vec<_> = groups.into_iter().collect();
        crate::visual_states::validate(&groups);
        self.element.on_mount.push(Rc::new(move |control| crate::visual_states::install(control, &groups)));
        self
    }

    /// Enter a state in the nearest enclosing visual state group when a
    /// registered routed event is raised on this control.
    pub fn state_trigger(mut self, event: &str, group: &str, state: &str) -> Self {
        let (event, group, state) = (event.to_owned(), group.to_owned(), state.to_owned());
        self.element.on_mount.push(Rc::new(move |control| {
            crate::visual_states::attach_state_trigger(control, &event, &group, &state);
        }));
        self
    }

    /// Keeps the control in `node_ref` once it is made.
    pub fn node_ref(self, node_ref: crate::NodeRef<T>) -> Self
    where
        T: ObjectType,
    {
        self.with(move |control| node_ref.set(control.clone()))
    }

    /// Adds a child: an element, a builder, text or a list. Children go into
    /// the content of the control: `Children` of a panel, `Items` of an items
    /// control, `Content` of a content control, `Child` of a decorator, the
    /// text of a `TextBlock`.
    pub fn child(mut self, child: impl IntoChild) -> Self {
        self.element.children.push(child.into_child());
        self
    }

    /// Adds children: a vector, an array or a tuple of them.
    pub fn children(mut self, children: impl IntoChildren) -> Self {
        self.element.children.extend(children.into_children());
        self
    }

    /// The element.
    pub fn build(self) -> Element {
        Element::Control(Box::new(self.element))
    }
}

impl<T> IntoElement for El<T> {
    fn into_element(self) -> Element {
        self.build()
    }
}

impl<T> From<El<T>> for Element {
    fn from(builder: El<T>) -> Self {
        builder.build()
    }
}

/// The object a property is set on, for the generated code.
#[doc(hidden)]
pub fn object_of(control: &Ref<Control>) -> &FerroObject {
    control
}
