//! WinUI's `VisualTransition` (`Microsoft.UI.Xaml.VisualTransition`; the engine
//! `dxaml/xcp/components/vsm/VisualStateManagerActuator.cpp`,
//! `DynamicTransitionStoryboardGenerator.cpp` and the transition lookup of
//! `CVisualStateManager2.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::animation::{EasingFunctionBase, Storyboard};
use ferroui_base::animation::SignedDuration;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};
use std::cell::RefCell;

/// How a [`VisualStateGroup`](super::VisualStateGroup) changes from one state to another when
/// `GoToState` is called with transitions (WinUI's `VisualTransition`).
///
/// A transition applies from the state named [`from`](Self::from) (empty: any) to the state
/// named [`to`](Self::to) (empty: any). It runs its [`storyboard`](Self::storyboard) and, over
/// [`generated_duration`](Self::generated_duration) eased by
/// [`generated_easing_function`](Self::generated_easing_function), animations it generates
/// from the values of the old state to those of the new one; the setters and the storyboard of
/// the new state apply when they have completed.
#[repr(C)]
pub struct VisualTransition {
    base: FerroObject,
    from: RefCell<String>,
    to: RefCell<String>,
    generated_duration: RefCell<SignedDuration>,
    generated_easing_function: RefCell<Option<Ref<EasingFunctionBase>>>,
    storyboard: RefCell<Option<Ref<Storyboard>>>,
}

ferro_class!(VisualTransition: FerroObject);
ferro_class_info!(VisualTransition {
    new: VisualTransition::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Storyboard,
        properties: [
            From: String {
                get: VisualTransition::from,
                set: |this: &Ref<VisualTransition>, value: String| this.set_from(value)
            },
            To: String {
                get: VisualTransition::to,
                set: |this: &Ref<VisualTransition>, value: String| this.set_to(value)
            },
            GeneratedDuration: SignedDuration {
                get: VisualTransition::generated_duration,
                set: |this: &Ref<VisualTransition>, value: SignedDuration| this.set_generated_duration(value)
            },
            GeneratedEasingFunction: Option<Ref<EasingFunctionBase>> {
                get: VisualTransition::generated_easing_function,
                set: |this: &Ref<VisualTransition>, value: Option<Ref<EasingFunctionBase>>| {
                    this.set_generated_easing_function(value)
                }
            },
            Storyboard: Option<Ref<Storyboard>> {
                get: VisualTransition::storyboard,
                set: |this: &Ref<VisualTransition>, value: Option<Ref<Storyboard>>| this.set_storyboard(value)
            },
        ],
    },
});
ferro_impl_classes!(VisualTransition: FerroObjectImpl);

impl VisualTransition {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            from: RefCell::new(String::new()),
            to: RefCell::new(String::new()),
            generated_duration: RefCell::new(SignedDuration::ZERO),
            generated_easing_function: RefCell::new(None),
            storyboard: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The name of the state the transition applies from; empty for any state.
    pub fn from(&self) -> String {
        self.from.borrow().clone()
    }

    pub fn set_from(&self, value: impl Into<String>) {
        *self.from.borrow_mut() = value.into();
    }

    /// The name of the state the transition applies to; empty for any state.
    pub fn to(&self) -> String {
        self.to.borrow().clone()
    }

    pub fn set_to(&self, value: impl Into<String>) {
        *self.to.borrow_mut() = value.into();
    }

    /// The length of the generated animations. Default zero (none are generated).
    pub fn generated_duration(&self) -> SignedDuration {
        *self.generated_duration.borrow()
    }

    pub fn set_generated_duration(&self, value: SignedDuration) {
        *self.generated_duration.borrow_mut() = value;
    }

    /// The easing of the generated animations that last the full duration. Default `None`.
    pub fn generated_easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.generated_easing_function.borrow().clone()
    }

    pub fn set_generated_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        *self.generated_easing_function.borrow_mut() = value;
    }

    /// The storyboard the transition runs. Default `None`.
    pub fn storyboard(&self) -> Option<Ref<Storyboard>> {
        self.storyboard.borrow().clone()
    }

    pub fn set_storyboard(&self, value: Option<Ref<Storyboard>>) {
        *self.storyboard.borrow_mut() = value;
    }

    /// WinUI's `IsZeroDuration`: no generated duration and no storyboard of any length.
    pub(crate) fn is_zero_duration(&self) -> bool {
        let generated = self.generated_duration().to_duration().unwrap_or_default();
        let storyboard = self
            .storyboard()
            .map(|storyboard| storyboard.active_duration().is_none_or(|duration| !duration.is_zero()))
            .unwrap_or(false);
        generated.is_zero() && !storyboard
    }
}
