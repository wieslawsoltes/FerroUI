use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IScrollProvider, ProviderAdapter, ScrollAmount};
use crate::automation::{ElementNotEnabledException, ScrollPatternIdentifiers};
use crate::ScrollViewer;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`ScrollViewer`].
#[repr(C)]
pub struct ScrollViewerAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ScrollViewerAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ScrollViewerAutomationPeer { interfaces: [Rc<dyn IScrollProvider> => ProviderAdapter::as_scroll_provider] });

impl FerroObjectImpl for ScrollViewerAutomationPeer {}
impl ControlAutomationPeerImpl for ScrollViewerAutomationPeer {}

impl AutomationPeerImpl for ScrollViewerAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ScrollViewer
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(this: &Self) -> bool {
        // Return false if the control is part of a control template.
        this.owner().templated_parent().is_none() && Self::parent_is_control_element_core(this)
    }
}

impl IScrollProvider for ProviderAdapter<ScrollViewerAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn horizontally_scrollable(&self) -> bool {
        self.0.horizontally_scrollable()
    }

    fn horizontal_scroll_percent(&self) -> f64 {
        self.0.horizontal_scroll_percent()
    }

    fn horizontal_view_size(&self) -> f64 {
        self.0.horizontal_view_size()
    }

    fn vertically_scrollable(&self) -> bool {
        self.0.vertically_scrollable()
    }

    fn vertical_scroll_percent(&self) -> f64 {
        self.0.vertical_scroll_percent()
    }

    fn vertical_view_size(&self) -> f64 {
        self.0.vertical_view_size()
    }

    fn scroll(
        &self,
        horizontal_amount: ScrollAmount,
        vertical_amount: ScrollAmount,
    ) -> Result<(), ElementNotEnabledException> {
        self.0.scroll(horizontal_amount, vertical_amount)
    }

    fn set_scroll_percent(
        &self,
        horizontal_percent: f64,
        vertical_percent: f64,
    ) -> Result<(), ElementNotEnabledException> {
        self.0.set_scroll_percent(horizontal_percent, vertical_percent)
    }
}

impl ScrollViewerAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ScrollViewer) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ScrollViewer) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning scroll viewer.
    pub fn owner(&self) -> Ref<ScrollViewer> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a scroll viewer.")
    }

    /// Gets the peer of the horizontal scroll bar of the scroll viewer, if
    /// it has one (not for use outside the framework).
    pub fn get_horizontal_scroll_bar_peer(&self) -> Option<Ref<AutomationPeer>> {
        self.owner().horizontal_scroll_bar().map(|scroll_bar| self.get_or_create(&scroll_bar))
    }

    /// Gets the peer of the vertical scroll bar of the scroll viewer, if it
    /// has one (not for use outside the framework).
    pub fn get_vertical_scroll_bar_peer(&self) -> Option<Ref<AutomationPeer>> {
        self.owner().vertical_scroll_bar().map(|scroll_bar| self.get_or_create(&scroll_bar))
    }

    /// Gets a value that indicates whether the control can scroll
    /// horizontally (the scroll provider contract).
    pub fn horizontally_scrollable(&self) -> bool {
        let owner = self.owner();
        MathUtilities::greater_than(owner.extent().width, owner.viewport().width)
    }

    /// Gets the current horizontal scroll position (the scroll provider
    /// contract).
    pub fn horizontal_scroll_percent(&self) -> f64 {
        if !self.horizontally_scrollable() {
            return ScrollPatternIdentifiers::NO_SCROLL;
        }
        let owner = self.owner();
        owner.offset().x * 100.0 / (owner.extent().width - owner.viewport().width)
    }

    /// Gets the current horizontal view size (the scroll provider
    /// contract).
    pub fn horizontal_view_size(&self) -> f64 {
        let owner = self.owner();
        if MathUtilities::is_zero(owner.extent().width) {
            return 100.0;
        }
        f64::min(100.0, owner.viewport().width * 100.0 / owner.extent().width)
    }

    /// Gets a value that indicates whether the control can scroll
    /// vertically (the scroll provider contract).
    pub fn vertically_scrollable(&self) -> bool {
        let owner = self.owner();
        MathUtilities::greater_than(owner.extent().height, owner.viewport().height)
    }

    /// Gets the current vertical scroll position (the scroll provider
    /// contract).
    pub fn vertical_scroll_percent(&self) -> f64 {
        if !self.vertically_scrollable() {
            return ScrollPatternIdentifiers::NO_SCROLL;
        }
        let owner = self.owner();
        owner.offset().y * 100.0 / (owner.extent().height - owner.viewport().height)
    }

    /// Gets the vertical view size (the scroll provider contract).
    pub fn vertical_view_size(&self) -> f64 {
        let owner = self.owner();
        if MathUtilities::is_zero(owner.extent().height) {
            return 100.0;
        }
        f64::min(100.0, owner.viewport().height * 100.0 / owner.extent().height)
    }

    /// Scrolls the visible region of the content area horizontally and
    /// vertically (the scroll provider contract).
    ///
    /// # Panics
    ///
    /// Panics if a direction that cannot scroll is asked to scroll.
    pub fn scroll(
        &self,
        horizontal_amount: ScrollAmount,
        vertical_amount: ScrollAmount,
    ) -> Result<(), ElementNotEnabledException> {
        if !self.is_enabled() {
            return Err(ElementNotEnabledException::new());
        }

        let scroll_horizontally = horizontal_amount != ScrollAmount::NoAmount;
        let scroll_vertically = vertical_amount != ScrollAmount::NoAmount;

        if scroll_horizontally && !self.horizontally_scrollable() || scroll_vertically && !self.vertically_scrollable()
        {
            panic!("Operation cannot be performed");
        }

        let owner = self.owner();

        match horizontal_amount {
            ScrollAmount::LargeDecrement => owner.page_left(),
            ScrollAmount::SmallDecrement => owner.line_left(),
            ScrollAmount::SmallIncrement => owner.line_right(),
            ScrollAmount::LargeIncrement => owner.page_right(),
            ScrollAmount::NoAmount => {}
        }

        match vertical_amount {
            ScrollAmount::LargeDecrement => owner.page_up(),
            ScrollAmount::SmallDecrement => owner.line_up(),
            ScrollAmount::SmallIncrement => owner.line_down(),
            ScrollAmount::LargeIncrement => owner.page_down(),
            ScrollAmount::NoAmount => {}
        }

        Ok(())
    }

    /// Sets the horizontal and vertical scroll position as a percentage of
    /// the total content area within the control (the scroll provider
    /// contract).
    ///
    /// # Panics
    ///
    /// Panics if a direction that cannot scroll is asked to scroll, or if
    /// a percentage is out of range.
    #[allow(clippy::precedence)]
    pub fn set_scroll_percent(
        &self,
        horizontal_percent: f64,
        vertical_percent: f64,
    ) -> Result<(), ElementNotEnabledException> {
        if !self.is_enabled() {
            return Err(ElementNotEnabledException::new());
        }

        let scroll_horizontally = horizontal_percent != ScrollPatternIdentifiers::NO_SCROLL;
        let scroll_vertically = vertical_percent != ScrollPatternIdentifiers::NO_SCROLL;

        if scroll_horizontally && !self.horizontally_scrollable() || scroll_vertically && !self.vertically_scrollable()
        {
            panic!("Operation cannot be performed");
        }

        // The grouping of the two range checks below is the one of the
        // reference: a percentage above 100 is out of range whether or not
        // the direction scrolls.
        if scroll_horizontally && (horizontal_percent < 0.0) || (horizontal_percent > 100.0) {
            panic!("Specified argument was out of the range of valid values. (Parameter 'horizontalPercent')");
        }

        if scroll_vertically && (vertical_percent < 0.0) || (vertical_percent > 100.0) {
            panic!("Specified argument was out of the range of valid values. (Parameter 'verticalPercent')");
        }

        let owner = self.owner();
        let mut offset = owner.offset();

        if scroll_horizontally {
            offset = offset.with_x((owner.extent().width - owner.viewport().width) * horizontal_percent * 0.01);
        }

        if scroll_vertically {
            offset = offset.with_y((owner.extent().height - owner.viewport().height) * vertical_percent * 0.01);
        }

        owner.set_offset(offset);

        Ok(())
    }
}
