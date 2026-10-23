// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../../NOTICE.md.

use crate::RatingControl;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{
    IRangeValueProvider, IValueProvider, ProviderAdapter, ProviderError,
};
use crate::automation::{
    ElementNotEnabledException, RangeValuePatternIdentifiers, ValuePatternIdentifiers,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue,
    FerroObjectImpl, Ref,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Both WinUI Value and RangeValue patterns on the native automation tree.
#[repr(C)]
pub struct RatingControlAutomationPeer {
    base: ControlAutomationPeer,
    cached_description: RefCell<String>,
}
ferro_class!(RatingControlAutomationPeer: ControlAutomationPeer);
ferro_class_info!(RatingControlAutomationPeer {
    interfaces: [
        Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider,
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider,
    ]
});
ferro_impl_classes!(RatingControlAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);

impl AutomationPeerImpl for RatingControlAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Slider
    }
    fn get_class_name_core(_this: &Self) -> String {
        "RatingControl".to_owned()
    }
    fn get_localized_control_type_core(_this: &Self) -> String {
        "Rating Slider".to_owned()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        Self::parent_get_name_core(this)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                let caption = this.owner().caption();
                (!caption.is_empty()).then_some(caption)
            })
    }
}

impl RatingControlAutomationPeer {
    pub fn new(owner: &RatingControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            cached_description: RefCell::new(Self::describe_value(owner)),
        })
    }
    pub fn owner(&self) -> Ref<RatingControl> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("RatingControl peer owner")
    }
    fn value_description(&self) -> String {
        Self::describe_value(&self.owner())
    }
    fn describe_value(owner: &RatingControl) -> String {
        if owner.value() >= 0.0 {
            format!("Rating, {} of {}", owner.value(), owner.max_rating())
        } else if owner.placeholder_value() >= 0.0 {
            format!(
                "Community Rating, {} of {}",
                owner.placeholder_value(),
                owner.max_rating()
            )
        } else {
            "Rating Unset".to_owned()
        }
    }
    pub(crate) fn rating_changed(&self, old: f64, new: f64, placeholder: bool) {
        if !placeholder && old != new {
            self.raise_property_changed_event(
                RangeValuePatternIdentifiers::value_property(),
                Some(Rc::new(old.max(0.0)) as BoxedValue),
                Some(Rc::new(new.max(0.0)) as BoxedValue),
            );
        }
        self.description_changed();
    }
    pub(crate) fn description_changed(&self) {
        let new = self.value_description();
        let old = self.cached_description.replace(new.clone());
        if old != new {
            self.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                Some(Rc::new(old) as BoxedValue),
                Some(Rc::new(new) as BoxedValue),
            );
        }
    }
}

impl IRangeValueProvider for ProviderAdapter<RatingControlAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.owner().is_read_only()
    }
    fn minimum(&self) -> f64 {
        0.0
    }
    fn maximum(&self) -> f64 {
        self.0.owner().max_rating() as f64
    }
    fn small_change(&self) -> f64 {
        1.0
    }
    fn large_change(&self) -> f64 {
        1.0
    }
    fn value(&self) -> f64 {
        self.0.owner().value().max(0.0)
    }
    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        // The upstream provider setter delegates to Value even when the UI is
        // read-only; IsReadOnly describes interactive input, not assignment.
        if value.is_nan() {
            return Err(ElementNotEnabledException::with_message(
                "Rating cannot be NaN.",
            ));
        }
        self.0.owner().set_rating_value(value);
        Ok(())
    }
}
impl IValueProvider for ProviderAdapter<RatingControlAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.owner().is_read_only()
    }
    fn value(&self) -> Option<String> {
        Some(self.0.value_description())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        if let Some(value) = value
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| !v.is_nan())
        {
            self.0.owner().set_rating_value(value);
        }
        Ok(())
    }
}
