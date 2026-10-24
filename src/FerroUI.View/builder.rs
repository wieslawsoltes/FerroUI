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
use ferroui_base::{AttachedProperty, BoxedValue, DirectProperty, FerroObject, FerroProperty, ObjectType, PropertyValue, Ref, StyledProperty, Upcast};
use ferroui_controls::Control;
use std::any::Any;
use std::cell::RefCell;
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
}

impl<T: ViewClass> Builder for T {
    type Output = El<T>;
    fn builder() -> El<T> {
        El::new()
    }
}

/// A property of a type known to the compiler: what the setters of an
/// element take.
pub trait TypedProperty: 'static {
    /// The type of the value of the property.
    type Value: PropertyValue;
    /// The property, untyped.
    fn untyped(&'static self) -> &'static FerroProperty;
}

impl<T: PropertyValue> TypedProperty for StyledProperty<T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        self.as_property()
    }
}

impl<T: PropertyValue> TypedProperty for AttachedProperty<T> {
    type Value = T;
    fn untyped(&'static self) -> &'static FerroProperty {
        let styled: &'static StyledProperty<T> = self;
        styled.as_property()
    }
}

impl<O: ObjectType, T: PropertyValue> TypedProperty for DirectProperty<O, T> {
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

fn boxed<T: PropertyValue>(value: T) -> BoxedValue {
    Rc::new(value)
}

impl<T: PropertyValue, V: IntoValue<T>> IntoProp<T, markers::Value> for V {
    fn into_prop(self) -> PropValue {
        PropValue::Value(boxed(self.into_value()))
    }
}

impl<T: PropertyValue, V: IntoValue<T> + Clone + 'static> IntoProp<T, markers::FromSignal<V>> for Signal<V> {
    fn into_prop(self) -> PropValue {
        PropValue::Follows { source: self.source_id(), compute: Rc::new(move || boxed(self.get().into_value())) }
    }
}

impl<T: PropertyValue, V: IntoValue<T> + Clone + 'static> IntoProp<T, markers::FromMemo<V>> for Memo<V> {
    fn into_prop(self) -> PropValue {
        PropValue::Follows { source: self.source_id(), compute: Rc::new(move || boxed(self.get().into_value())) }
    }
}

impl<T: PropertyValue, V: IntoValue<T>, F: Fn() -> V + 'static> IntoProp<T, markers::FromClosure<V>> for F {
    fn into_prop(self) -> PropValue {
        PropValue::Dynamic(Rc::new(move || boxed(self().into_value())))
    }
}

impl<T: PropertyValue, V: IntoValue<T> + FromValue<T> + Clone + PartialEq + 'static> IntoProp<T, markers::FromTwoWay<V>>
    for TwoWay<V>
{
    fn into_prop(self) -> PropValue {
        let signal = self.0;
        PropValue::TwoWay {
            source: Some(signal.source_id()),
            get: Rc::new(move || boxed(signal.get().into_value())),
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

impl<T: PropertyValue, B: BindingBase + 'static> IntoProp<T, markers::FromBinding> for Bind<B> {
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
