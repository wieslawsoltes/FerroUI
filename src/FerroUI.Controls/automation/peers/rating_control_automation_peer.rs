//! The automation peer of WinUI's `RatingControl`
//! (`controls/dev/RatingControl/RatingControlAutomationPeer.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), with the strings of its English resources (BasicRatingString, CommunityRatingString,
//! RatingUnset, RatingLocalizedControlType).

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::RatingControl;
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

/// An automation peer which represents a [`RatingControl`]: a slider with the value pattern (the
/// text of the rating) and the range value pattern (0 to `MaxRating`; an unset rating is 0),
/// named by the caption when it has no name.
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
    /// Initializes a new peer of `owner`.
    pub fn new(owner: &RatingControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            cached_description: RefCell::new(Self::describe_value(owner)),
        })
    }
    /// Gets the owning rating control.
    pub fn owner(&self) -> Ref<RatingControl> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a rating control.")
    }
    fn value_description(&self) -> String {
        Self::describe_value(&self.owner())
    }
    /// IValueProvider_Value: "Rating, 3 of 5", "Community Rating, 3.5 of 5" or "Rating Unset".
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
    /// The changes of the value of the two patterns after a change of the rating (WinUI's
    /// `RaisePropertyChangedEvent`; an unset rating is 0).
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
    /// The change of the text of the value pattern.
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
        // RatingControlAutomationPeer::SetValue sets Value even when the control is read-only:
        // IsReadOnly is about the input of the user.
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
