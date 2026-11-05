//! The routing of the pointer input of an [`InputElement`] to its interactive effect (#455, E7;
//! not from upstream; docs/porting/custom-effects.md, section 3.5), as Effector's
//! `SkiaEffectInputManager` routes it.
//!
//! One class handler of the changes of `Visual.Effect` (registered with the class handlers of
//! [`InputElement`]) attaches the effect of an element to it when the effect is an interactive
//! custom effect class ([`Effect::construct_interactive`]), and detaches the effect it attached
//! when the effect changes. The attachment holds the guards of the handlers it added to the
//! element's pointer events and of its subscription to the element's bounds, so detaching it (or
//! dropping the element) removes them: no handler outlives the attachment, and nothing of the
//! effect is reached from an element it is no longer the effect of.
//!
//! The handlers are added to the direct and the bubble routes with `handled_events_too`, and
//! deliver a handled event only to an effect that asks for handled events
//! ([`IEffectInputHandler::handled_events_too`]); the order is that of the routed events: after
//! the tunnel route and the class handlers of the element, and after the handlers of its
//! descendants on the bubble route.

use super::{
    InputElement, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerWheelEventArgs,
};
use crate::interactivity::Interactive;
use crate::media::effects::{Effect, EffectHostContext, IEffect, IEffectInputHandler};
use crate::reactive::Subscription;
use crate::{FerroProperty, Ref, Upcast, Visual};

/// An interactive effect attached to the element whose effect it is, with the guards of the
/// handlers that route the element's input to it.
pub(crate) struct EffectInputAttachment {
    effect: Ref<Effect>,
    _subscriptions: Vec<Subscription>,
}

/// Delivers an event of `sender` to its attached interactive effect: nothing when the element has
/// none, or when the event is `handled` and the effect does not take handled events. No borrow of
/// the element is held while the effect runs, so the effect may capture the pointer (which raises
/// events on the element) or change the effect of the element.
fn route_effect_input(
    sender: &Interactive,
    handled: bool,
    deliver: impl FnOnce(&dyn IEffectInputHandler, &EffectHostContext<'_>),
) {
    let Some(host) = sender.downcast_ref::<InputElement>() else { return };
    let Some(effect) = host.effect_input.borrow().as_ref().map(|attachment| attachment.effect.clone()) else {
        return;
    };
    let Some(handler) = effect.input_handler() else { return };
    if handled && !handler.handled_events_too() {
        return;
    }
    deliver(handler, &EffectHostContext::new(&effect, host));
}

/// Whether a change of `property` is a change of the bounds of the host for its effect.
fn is_host_bounds_property(property: &'static FerroProperty) -> bool {
    property == Visual::bounds_property().as_property()
        || property == Visual::render_transform_property().as_property()
        || property == Visual::render_transform_origin_property().as_property()
}

impl InputElement {
    /// Registers the class handler of the changes of `Visual.Effect` that attaches and detaches
    /// interactive effects.
    pub(super) fn register_effect_input() {
        Visual::effect_property()
            .changed()
            .subscribe(|e| {
                if let Some(host) = e.sender().downcast_ref::<InputElement>() {
                    host.update_effect_input();
                }
            })
            .detach();
    }

    /// The interactive effect class behind `effect`, when it is one.
    fn interactive_effect_of(effect: &dyn IEffect) -> Option<Ref<Effect>> {
        let effect = effect.as_object()?.downcast_ref::<Effect>()?;
        if effect.is_interactive() {
            Some(effect.to_ref())
        } else {
            None
        }
    }

    /// The interactive effect the element routes its pointer input to (#455, E7, not from
    /// upstream): its `Effect` when that is an interactive custom effect class
    /// ([`Effect::construct_interactive`]).
    pub fn interactive_effect(&self) -> Option<Ref<Effect>> {
        self.effect_input.borrow().as_ref().map(|attachment| attachment.effect.clone())
    }

    /// Whether `effect` is the interactive effect attached to the element.
    pub(crate) fn has_effect_input_attached(&self, effect: &Effect) -> bool {
        self.effect_input.borrow().as_ref().is_some_and(|attachment| std::ptr::eq(&*attachment.effect, effect))
    }

    /// Detaches the interactive effect attached to the element, if it is no longer its effect,
    /// and attaches its effect, if that is an interactive one.
    fn update_effect_input(&self) {
        let effect = self.effect();
        let new = effect.as_deref().and_then(Self::interactive_effect_of);
        drop(effect);
        if self.interactive_effect() == new {
            return;
        }

        let old = self.effect_input.borrow_mut().take();
        if let Some(old) = old {
            let effect = old.effect.clone();
            // The handlers are removed before the effect hears of it.
            drop(old);
            if let Some(handler) = effect.input_handler() {
                handler.on_detached(&EffectHostContext::new(&effect, self));
            }
        }

        if let Some(effect) = new {
            let subscriptions = self.effect_input_subscriptions();
            let previous =
                self.effect_input.replace(Some(EffectInputAttachment { effect: effect.clone(), _subscriptions: subscriptions }));
            drop(previous);
            if let Some(handler) = effect.input_handler() {
                let context = EffectHostContext::new(&effect, self);
                handler.on_attached(&context);
                // `on_attached` may have replaced the effect of the element.
                if self.has_effect_input_attached(&effect) {
                    handler.on_host_bounds_changed(&context);
                }
            }
        }
    }

    /// Adds the handlers that route the pointer input and the bounds of the element to its
    /// interactive effect, and returns their guards.
    fn effect_input_subscriptions(&self) -> Vec<Subscription> {
        let routes = Interactive::DEFAULT_ROUTES;
        let mut subscriptions = Vec::with_capacity(8);
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_entered_event(),
            |sender, e: &PointerEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_entered(context, e))
            },
            routes,
            true,
        ));
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_exited_event(),
            |sender, e: &PointerEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_exited(context, e))
            },
            routes,
            true,
        ));
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_moved_event(),
            |sender, e: &PointerEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_moved(context, e))
            },
            routes,
            true,
        ));
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_pressed_event(),
            |sender, e: &PointerPressedEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_pressed(context, e))
            },
            routes,
            true,
        ));
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_released_event(),
            |sender, e: &PointerReleasedEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_released(context, e))
            },
            routes,
            true,
        ));
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_capture_lost_event(),
            |sender, e: &PointerCaptureLostEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_capture_lost(context, e))
            },
            routes,
            true,
        ));
        subscriptions.push(self.add_disposable_handler_with(
            Self::pointer_wheel_changed_event(),
            |sender, e: &PointerWheelEventArgs| {
                route_effect_input(sender, e.handled(), |handler, context| handler.on_pointer_wheel_changed(context, e))
            },
            routes,
            true,
        ));
        let bounds = self.property_changed(|e| {
            if !is_host_bounds_property(e.property()) {
                return;
            }
            if let Some(host) = e.sender().downcast_ref::<InputElement>() {
                let sender: &Interactive = host.upcast();
                route_effect_input(sender, false, |handler, context| handler.on_host_bounds_changed(context));
            }
        });
        subscriptions.push(Subscription::new(bounds));
        subscriptions
    }
}
