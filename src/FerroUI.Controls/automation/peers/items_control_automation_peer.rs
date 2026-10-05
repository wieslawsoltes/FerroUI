use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IScrollProvider, ProviderAdapter, ScrollAmount};
use crate::automation::ElementNotEnabledException;
use crate::{Control, ItemsControl, ListBox};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An automation peer which represents an [`ItemsControl`].
#[repr(C)]
pub struct ItemsControlAutomationPeer {
    base: ControlAutomationPeer,
    searched_for_scrollable: Cell<bool>,
    scroller: RefCell<Option<Rc<dyn IScrollProvider>>>,
}

ferro_class! {
    ItemsControlAutomationPeer: ControlAutomationPeer, virtuals ItemsControlAutomationPeerImpl: ControlAutomationPeerImpl {
        /// Gets the scroll provider of the element that scrolls the items
        /// of the owner, searching for it on the first call.
        fn scroller(this) -> Option<Rc<dyn IScrollProvider>>;
    }
}
ferroui_base::ferro_class_info!(ItemsControlAutomationPeer { interfaces: [Rc<dyn IScrollProvider> => ProviderAdapter::as_scroll_provider] });

impl FerroObjectImpl for ItemsControlAutomationPeer {}
impl ControlAutomationPeerImpl for ItemsControlAutomationPeer {}

impl ItemsControlAutomationPeerImpl for ItemsControlAutomationPeer {
    fn scroller(this: &Self) -> Option<Rc<dyn IScrollProvider>> {
        if !this.searched_for_scrollable.get() {
            let scrollable = this
                .owner()
                .get_direct_value(ListBox::scroll_property())
                .and_then(|scrollable| scrollable.cast::<Control>());
            if let Some(scrollable) = scrollable {
                let scroller = this.get_or_create(&scrollable).get_provider::<dyn IScrollProvider>();
                *this.scroller.borrow_mut() = scroller;
            }
            this.searched_for_scrollable.set(true);
        }

        this.scroller.borrow().clone()
    }
}

impl AutomationPeerImpl for ItemsControlAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::List
    }
}

impl IScrollProvider for ProviderAdapter<ItemsControlAutomationPeer> {
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

impl ItemsControlAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ItemsControl) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
            searched_for_scrollable: Cell::new(false),
            scroller: RefCell::new(None),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ItemsControl) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning items control.
    pub fn owner(&self) -> Ref<ItemsControl> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is an items control.")
    }

    // The members of the scroll provider contract read the scroll provider
    // that was found, as the reference does: they do not search for it.
    fn found_scroller(&self) -> Option<Rc<dyn IScrollProvider>> {
        self.scroller.borrow().clone()
    }

    /// Gets a value that indicates whether the control can scroll
    /// horizontally (the scroll provider contract).
    pub fn horizontally_scrollable(&self) -> bool {
        self.found_scroller().map(|scroller| scroller.horizontally_scrollable()).unwrap_or(false)
    }

    /// Gets the current horizontal scroll position (the scroll provider
    /// contract).
    pub fn horizontal_scroll_percent(&self) -> f64 {
        self.found_scroller().map(|scroller| scroller.horizontal_scroll_percent()).unwrap_or(-1.0)
    }

    /// Gets the current horizontal view size (the scroll provider
    /// contract).
    pub fn horizontal_view_size(&self) -> f64 {
        self.found_scroller().map(|scroller| scroller.horizontal_view_size()).unwrap_or(0.0)
    }

    /// Gets a value that indicates whether the control can scroll
    /// vertically (the scroll provider contract).
    pub fn vertically_scrollable(&self) -> bool {
        self.found_scroller().map(|scroller| scroller.vertically_scrollable()).unwrap_or(false)
    }

    /// Gets the current vertical scroll position (the scroll provider
    /// contract).
    pub fn vertical_scroll_percent(&self) -> f64 {
        self.found_scroller().map(|scroller| scroller.vertical_scroll_percent()).unwrap_or(-1.0)
    }

    /// Gets the vertical view size (the scroll provider contract).
    pub fn vertical_view_size(&self) -> f64 {
        self.found_scroller().map(|scroller| scroller.vertical_view_size()).unwrap_or(0.0)
    }

    /// Scrolls the visible region of the content area horizontally and
    /// vertically (the scroll provider contract).
    pub fn scroll(
        &self,
        horizontal_amount: ScrollAmount,
        vertical_amount: ScrollAmount,
    ) -> Result<(), ElementNotEnabledException> {
        match self.found_scroller() {
            Some(scroller) => scroller.scroll(horizontal_amount, vertical_amount),
            None => Ok(()),
        }
    }

    /// Sets the horizontal and vertical scroll position as a percentage of
    /// the total content area within the control (the scroll provider
    /// contract).
    pub fn set_scroll_percent(
        &self,
        horizontal_percent: f64,
        vertical_percent: f64,
    ) -> Result<(), ElementNotEnabledException> {
        match self.found_scroller() {
            Some(scroller) => scroller.set_scroll_percent(horizontal_percent, vertical_percent),
            None => Ok(()),
        }
    }
}
