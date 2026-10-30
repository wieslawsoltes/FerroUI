// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::animation_manager::{
    AnimationManager, TransitionTimeline, DEFAULT_DURATION_MS, QUICK_DURATION_MS,
};
use super::item_collection_transition::{
    ItemCollectionTransition, ItemCollectionTransitionOperation as Operation,
    ItemCollectionTransitionTriggers as Triggers,
};
use super::item_collection_transition_provider::{
    ItemCollectionTransitionProvider, ItemCollectionTransitionProviderImpl,
};
use ferroui_controls::Control;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Point, Ref};
use std::rc::Rc;

/// Lined-flow add, remove and move transitions with the original scaling,
/// slide-in, line changes, easing curves and delete/move/add sequencing.
#[repr(C)]
pub struct LinedFlowLayoutItemCollectionTransitionProvider {
    base: ItemCollectionTransitionProvider,
    animations: Rc<AnimationManager>,
}
ferro_class!(LinedFlowLayoutItemCollectionTransitionProvider: ItemCollectionTransitionProvider);
ferro_class_info!(LinedFlowLayoutItemCollectionTransitionProvider {
    new: LinedFlowLayoutItemCollectionTransitionProvider::new
});
impl FerroObjectImpl for LinedFlowLayoutItemCollectionTransitionProvider {}
impl LinedFlowLayoutItemCollectionTransitionProvider {
    pub fn construct() -> Self {
        Self {
            base: ItemCollectionTransitionProvider::construct(),
            animations: AnimationManager::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn active_animation_count(&self) -> usize {
        self.animations.active_count()
    }
}
impl ItemCollectionTransitionProviderImpl for LinedFlowLayoutItemCollectionTransitionProvider {
    fn should_animate_core(_this: &Self, _transition: Rc<ItemCollectionTransition>) -> bool {
        true
    }
    fn cancel_transitions_core(this: &Self, element: Ref<Control>) {
        this.animations.cancel_for_element(&element);
    }
    fn start_transitions_core(this: &Self, transitions: Vec<Rc<ItemCollectionTransition>>) {
        let has_add = transitions
            .iter()
            .any(|transition| transition.operation() == Operation::Add);
        let has_remove = transitions
            .iter()
            .any(|transition| transition.operation() == Operation::Remove);
        let has_move = transitions
            .iter()
            .any(|transition| transition.operation() == Operation::Move);
        for transition in transitions {
            if transition.is_cancelled() {
                continue;
            }
            let timeline = match transition.operation() {
                Operation::Add => {
                    if transition
                        .triggers()
                        .contains(Triggers::COLLECTION_CHANGE_ADD)
                    {
                        TransitionTimeline::ScaleIn {
                            delay: if has_move {
                                DEFAULT_DURATION_MS * 2.0
                            } else {
                                0.0
                            } + if has_remove { QUICK_DURATION_MS } else { 0.0 },
                        }
                    } else {
                        TransitionTimeline::SlideIn
                    }
                }
                Operation::Remove => {
                    if has_add
                        && transition
                            .triggers()
                            .contains(Triggers::COLLECTION_CHANGE_RESET)
                    {
                        continue;
                    }
                    TransitionTimeline::ScaleOut
                }
                Operation::Move => {
                    if transition.triggers().contains(Triggers::LAYOUT_TRANSITION) {
                        continue;
                    }
                    let old = transition.old_bounds();
                    let new = transition.new_bounds();
                    let delay = if has_remove { QUICK_DURATION_MS } else { 0.0 };
                    let delta = Point::new(old.x - new.x, old.y - new.y);
                    if (old.y - new.y).abs() < 0.001 {
                        TransitionTimeline::Translate { delay, delta }
                    } else {
                        TransitionTimeline::ChangeLine { delay, delta }
                    }
                }
            };
            this.animations.start(transition, timeline);
        }
    }
}
