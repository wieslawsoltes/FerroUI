//! Port of `InteractionTriggerBehavior.cs` (`InteractionTriggerBehavior<TInput, TOutput>`).

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_reactiveui::{IInteractionContext, Interaction as ReactiveInteraction};
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTriggerOfT,
};
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// A ReactiveUI interaction ([`ferroui_reactiveui::Interaction`]) of any input and output type,
/// as an [`InteractionTriggerBehavior`] holds it.
///
/// Deviation (DEVIATIONS.md, Behaviors): the classes of the object model are not generic, so the
/// type arguments `TInput` and `TOutput` of the reference travel with the interaction: this
/// value wraps an `Interaction<I, O>` and registers handlers that receive its input as an untyped
/// value and set the default output (`context.SetOutput(default!)`; the output type implements
/// [`Default`]). Clones are the same interaction; two values are equal when they wrap the same
/// interaction.
#[derive(Clone)]
pub struct AnyInteraction(Rc<dyn ErasedInteraction>);

/// The operations of an interaction whose types are erased.
trait ErasedInteraction {
    /// Registers a handler that receives the input of each question, and answers the question
    /// with the default output.
    fn register_handler(&self, handler: Rc<dyn Fn(Option<BoxedValue>)>) -> Rc<dyn IDisposable>;

    /// The typed interaction (a `TypedInteraction<I, O>`), for comparisons and downcasts.
    fn as_any(&self) -> &dyn Any;

    /// Whether `other` wraps the same interaction.
    fn same_as(&self, other: &dyn ErasedInteraction) -> bool;
}

/// An interaction with its types.
struct TypedInteraction<I, O>(ReactiveInteraction<I, O>);

impl<I: Clone + PartialEq + 'static, O: Clone + Default + 'static> ErasedInteraction for TypedInteraction<I, O> {
    fn register_handler(&self, handler: Rc<dyn Fn(Option<BoxedValue>)>) -> Rc<dyn IDisposable> {
        self.0.register_handler(move |context: Rc<dyn IInteractionContext<I, O>>| {
            handler(box_input(context.input()));
            context.set_output(O::default());
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn same_as(&self, other: &dyn ErasedInteraction) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| other.0 == self.0)
    }
}

/// The input of a question as the parameter of the actions: the untyped value itself when the
/// input is one, else the input boxed.
fn box_input<I: Clone + PartialEq + 'static>(input: &I) -> Option<BoxedValue> {
    let any: &dyn Any = input;
    if let Some(value) = any.downcast_ref::<Option<BoxedValue>>() {
        return value.clone();
    }
    if let Some(value) = any.downcast_ref::<BoxedValue>() {
        return Some(value.clone());
    }
    Some(Rc::new(input.clone()) as BoxedValue)
}

impl AnyInteraction {
    /// Wraps `interaction`.
    pub fn new<I: Clone + PartialEq + 'static, O: Clone + Default + 'static>(interaction: ReactiveInteraction<I, O>) -> Self {
        Self(Rc::new(TypedInteraction(interaction)))
    }

    /// Registers a handler that receives the input of each question (as an untyped value) and
    /// answers it with the default output; disposing the result unregisters it.
    pub fn register_handler(&self, handler: impl Fn(Option<BoxedValue>) + 'static) -> Rc<dyn IDisposable> {
        self.0.register_handler(Rc::new(handler))
    }

    /// The wrapped interaction, if its types are `I` and `O`.
    pub fn downcast<I: 'static, O: 'static>(&self) -> Option<ReactiveInteraction<I, O>> {
        self.0.as_any().downcast_ref::<TypedInteraction<I, O>>().map(|typed| typed.0.clone())
    }
}

impl<I: Clone + PartialEq + 'static, O: Clone + Default + 'static> From<ReactiveInteraction<I, O>> for AnyInteraction {
    fn from(interaction: ReactiveInteraction<I, O>) -> Self {
        Self::new(interaction)
    }
}

impl PartialEq for AnyInteraction {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.same_as(&*other.0)
    }
}

impl fmt::Debug for AnyInteraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyInteraction").finish_non_exhaustive()
    }
}

/// A behavior that registers a handler for a ReactiveUI interaction
/// ([`ferroui_reactiveui::Interaction`]) and executes its actions when the interaction is
/// triggered.
///
/// The handler is registered while the associated object is in the visual tree; it executes the
/// actions with the associated object as the sender and the input of the interaction as the
/// parameter, and answers the interaction with the default output.
///
/// Deviation (DEVIATIONS.md, Behaviors): the class is not generic; the type arguments of the
/// reference travel with the interaction ([`AnyInteraction`]). The associated object is checked
/// to be a [`Visual`] on attach, as `StyledElementTrigger<Visual>` does.
#[repr(C)]
pub struct InteractionTriggerBehavior {
    base: StyledElementTriggerOfT,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(InteractionTriggerBehavior: StyledElementTriggerOfT);
ferro_class_info!(InteractionTriggerBehavior { new: InteractionTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(InteractionTriggerBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl InteractionTriggerBehavior {
        /// Identifies the `Interaction` property.
        pub fn interaction_property() -> StyledProperty<Option<AnyInteraction>> {
            FerroProperty::register::<InteractionTriggerBehavior, _>("Interaction", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for InteractionTriggerBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let Some(interaction) = this.interaction() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let disposable = interaction.register_handler(move |input| {
                let Some(behavior) = behavior.upgrade() else {
                    return;
                };
                let sender = behavior.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
                Interaction::execute_actions(sender.as_ref(), behavior.actions().as_ref(), input.as_ref());
            });
            *this.disposable.borrow_mut() = Some(disposable);
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            let disposable = this.disposable.borrow().clone();
            if let Some(disposable) = disposable {
                disposable.dispose();
            }
        }
    }
}

impl InteractionTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Visual as StaticType>::TYPE), disposable: RefCell::new(None) }
    }

    /// Creates the trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the interaction to register the handler for. This is a property of the
    /// object model.
    pub fn interaction(&self) -> Option<AnyInteraction> {
        self.get_value(Self::interaction_property())
    }

    pub fn set_interaction(&self, value: Option<AnyInteraction>) {
        self.set_value(Self::interaction_property(), value)
    }

    /// The interaction, if its types are `I` and `O` (the typed `Interaction` of the reference).
    pub fn interaction_of<I: 'static, O: 'static>(&self) -> Option<ReactiveInteraction<I, O>> {
        self.interaction().and_then(|interaction| interaction.downcast::<I, O>())
    }

    /// Sets the interaction from a typed one.
    pub fn set_interaction_of<I: Clone + PartialEq + 'static, O: Clone + Default + 'static>(
        &self,
        value: Option<ReactiveInteraction<I, O>>,
    ) {
        self.set_interaction(value.map(AnyInteraction::new))
    }
}
