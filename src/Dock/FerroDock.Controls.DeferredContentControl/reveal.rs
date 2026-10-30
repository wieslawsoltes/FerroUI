// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::animation::easings::{CubicEaseOut, Easing};
use ferroui_base::animation::{DoubleTransition, TimeSpan, TransitionBaseImpl, Transitions};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, FerroObjectImpl, Ref, Visual,
};
use ferroui_controls::Control;
#[repr(C)]
struct DeferredRevealTransition {
    base: DoubleTransition,
}
ferro_class!(DeferredRevealTransition:DoubleTransition);
ferro_class_info!(DeferredRevealTransition {});
ferro_impl_classes!(DeferredRevealTransition:FerroObjectImpl,TransitionBaseImpl);
fn create(duration: TimeSpan) -> Ref<DeferredRevealTransition> {
    let transition = ferroui_base::instantiate(DeferredRevealTransition {
        base: DoubleTransition::construct(),
    });
    transition.set_property(Some(Visual::opacity_property().as_property()));
    transition.set_duration(duration);
    transition.set_easing(Easing::from(CubicEaseOut::new()));
    transition
}
pub(crate) fn apply(control: &Control, duration: TimeSpan, should_reveal: bool) {
    if !should_reveal || duration <= TimeSpan::ZERO {
        control.set_opacity(1.0);
        return;
    }
    ensure(control, duration);
    control.set_opacity(0.85);
    let control = control.to_ref();
    Dispatcher::ui_thread().post_local(
        move || control.set_current_value(Visual::opacity_property(), 1.0),
        DispatcherPriority::BACKGROUND,
    );
}
fn ensure(control: &Control, duration: TimeSpan) {
    if let Some(transitions) = control.transitions() {
        for index in 0..transitions.count() {
            let transition = transitions.get(index);
            if let Some(object) = transition.as_object() {
                if let Some(reveal) = object.cast::<DeferredRevealTransition>() {
                    reveal.set_duration(duration);
                    reveal.set_easing(Easing::from(CubicEaseOut::new()));
                    return;
                }
                if object.cast::<DoubleTransition>().is_some()
                    && transition.property() == Visual::opacity_property().as_property()
                {
                    return;
                }
            }
        }
        transitions.add(create(duration).into());
    } else {
        control.set_transitions(Some(Transitions::from_items([create(duration).into()])));
    }
}
