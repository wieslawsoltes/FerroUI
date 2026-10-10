//! The wrapper of an automation peer: what an accessibility element of
//! UIKit says of the peer (label, value, hint, identifier, traits, frame),
//! the accessibility container over the children of the peer, and the
//! actions of assistive technology (activate, increment, decrement, scroll).
//!
//! The wrapper is plain: it keeps what the element answers with and has
//! the logic of the reference, so the tests of the crate run it on the
//! development machine. The object of UIKit (the class
//! `FerroAutomationPeerElement`, in the module `uikit`) answers every
//! question of the accessibility protocol from its wrapper.

use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Ref};
use ferroui_controls::automation::peers::{AutomationControlType, AutomationPeer};
use ferroui_controls::automation::provider::{
    IInvokeProvider, IRangeValueProvider, IScrollProvider, ISelectionItemProvider, IToggleProvider, IValueProvider,
    ScrollAmount,
};
use ferroui_controls::automation::{
    AutomationElementIdentifiers, AutomationProperty, RangeValuePatternIdentifiers, SelectionItemPatternIdentifiers,
    ValuePatternIdentifiers,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

bitflags::bitflags! {
    /// The accessibility traits of UIKit the wrapper sets, with the
    /// values of the constants of the system (`UIAccessibilityTrait...`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AccessibilityTraits: u64 {
        const BUTTON = 1;
        const LINK = 2;
        const IMAGE = 4;
        const SELECTED = 8;
        const NOT_ENABLED = 256;
        const ADJUSTABLE = 4096;
        const HEADER = 65536;
    }
}

/// The container type of an accessibility container, with the values of
/// `UIAccessibilityContainerType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum AccessibilityContainerType {
    None = 0,
    DataTable = 1,
    List = 2,
    Landmark = 3,
    SemanticGroup = 4,
}

/// The direction of a scroll of assistive technology, with the values of
/// `UIAccessibilityScrollDirection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum AccessibilityScrollDirection {
    Right = 1,
    Left = 2,
    Up = 3,
    Down = 4,
    Next = 5,
    Previous = 6,
}

impl AccessibilityScrollDirection {
    /// The direction of a value of the system.
    pub fn from_native(value: isize) -> Option<Self> {
        match value {
            1 => Some(Self::Right),
            2 => Some(Self::Left),
            3 => Some(Self::Up),
            4 => Some(Self::Down),
            5 => Some(Self::Next),
            6 => Some(Self::Previous),
            _ => None,
        }
    }
}

/// The index an accessibility container answers with for an element it
/// does not have (`NSNotFound`).
pub const NOT_FOUND: isize = isize::MAX;

/// Whether a peer of the control type is a container of other elements
/// and not an element of its own.
pub(crate) fn is_container_type(control_type: AutomationControlType) -> bool {
    matches!(
        control_type,
        AutomationControlType::Calendar
            | AutomationControlType::ComboBoxItem
            | AutomationControlType::Custom
            | AutomationControlType::DataGrid
            | AutomationControlType::DataItem
            | AutomationControlType::Document
            | AutomationControlType::Expander
            | AutomationControlType::Group
            | AutomationControlType::List
            | AutomationControlType::ListItem
            | AutomationControlType::Menu
            | AutomationControlType::MenuBar
            | AutomationControlType::MenuItem
            | AutomationControlType::Pane
            | AutomationControlType::ScrollViewer
            | AutomationControlType::SplitButton
            | AutomationControlType::Tab
            | AutomationControlType::TabItem
            | AutomationControlType::Table
            | AutomationControlType::TitleBar
            | AutomationControlType::ToolBar
            | AutomationControlType::Tree
            | AutomationControlType::TreeItem
            | AutomationControlType::Window
    )
}

/// The traits of a peer of the control type that is enabled or not.
pub(crate) fn traits_of(control_type: AutomationControlType, is_enabled: bool) -> AccessibilityTraits {
    let mut traits = AccessibilityTraits::empty();

    match control_type {
        AutomationControlType::Button => traits |= AccessibilityTraits::BUTTON,
        AutomationControlType::Header => traits |= AccessibilityTraits::HEADER,
        AutomationControlType::Hyperlink => traits |= AccessibilityTraits::LINK,
        AutomationControlType::Image => traits |= AccessibilityTraits::IMAGE,
        _ => {}
    }

    if !is_enabled {
        traits |= AccessibilityTraits::NOT_ENABLED;
    }

    traits
}

/// The frame of an element: the rectangle between the two corners of its
/// bounds on the screen.
pub(crate) fn frame_of(top_left: PixelPoint, bottom_right: PixelPoint) -> Rect {
    let screen_rect = PixelRect::from_points(top_left, bottom_right);
    Rect::new(
        f64::from(screen_rect.x),
        f64::from(screen_rect.y),
        f64::from(screen_rect.width),
        f64::from(screen_rect.height),
    )
}

/// The value of a range as the element says it: at most two decimals,
/// none that are zero (the format `0.##` of the reference).
pub(crate) fn format_range_value(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    // A value that lies exactly between two values of two decimals is
    // rounded away from zero by the reference; such a value is a multiple
    // of an eighth that is no multiple of a quarter.
    let eighths = value * 8.0;
    let rounded = if eighths.fract() == 0.0 && (value * 4.0).fract() != 0.0 && eighths.abs() < 1e15 {
        (value * 100.0).round() / 100.0
    } else {
        value
    };

    let text = format!("{rounded:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.to_string()
}

/// The amounts (horizontal, vertical) of a scroll in a direction, and
/// whether the direction is one the wrapper scrolls in.
pub(crate) fn scroll_amounts(direction: Option<AccessibilityScrollDirection>) -> (ScrollAmount, ScrollAmount, bool) {
    match direction {
        Some(AccessibilityScrollDirection::Up) => (ScrollAmount::NoAmount, ScrollAmount::LargeIncrement, true),
        Some(AccessibilityScrollDirection::Down) => (ScrollAmount::NoAmount, ScrollAmount::LargeDecrement, true),
        Some(AccessibilityScrollDirection::Left) => (ScrollAmount::LargeIncrement, ScrollAmount::NoAmount, true),
        Some(AccessibilityScrollDirection::Right) => (ScrollAmount::LargeDecrement, ScrollAmount::NoAmount, true),
        _ => (ScrollAmount::NoAmount, ScrollAmount::NoAmount, false),
    }
}

/// What a wrapper asks of the view it belongs to.
pub(crate) trait IWrapperView {
    /// A point of the root element of the top-level of the view in the
    /// coordinates of the screen, or `None` while the top-level has no
    /// input root.
    fn point_to_screen(&self, point: Point) -> Option<PixelPoint>;

    /// Tells assistive technology that a page was scrolled.
    fn post_page_scrolled(&self);

    /// The view, as the accessibility container of the root wrapper.
    #[cfg(target_os = "ios")]
    fn native_view(&self) -> Option<objc2::rc::Retained<objc2_ui_kit::UIView>>;
}

/// The setters of the properties of an element, in the order the
/// reference declares them for the automation properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PropertySetter {
    AutomationId,
    Name,
    HelpText,
    BoundingRectangle,
    IsReadOnly,
    Value,
    Selected,
}

/// The automation properties a wrapper follows, each with its setter.
fn property_setters() -> [(&'static AutomationProperty, PropertySetter); 9] {
    [
        (AutomationElementIdentifiers::automation_id_property(), PropertySetter::AutomationId),
        (AutomationElementIdentifiers::name_property(), PropertySetter::Name),
        (AutomationElementIdentifiers::help_text_property(), PropertySetter::HelpText),
        (AutomationElementIdentifiers::bounding_rectangle_property(), PropertySetter::BoundingRectangle),
        (RangeValuePatternIdentifiers::is_read_only_property(), PropertySetter::IsReadOnly),
        (RangeValuePatternIdentifiers::value_property(), PropertySetter::Value),
        (ValuePatternIdentifiers::is_read_only_property(), PropertySetter::IsReadOnly),
        (ValuePatternIdentifiers::value_property(), PropertySetter::Value),
        (SelectionItemPatternIdentifiers::is_selected_property(), PropertySetter::Selected),
    ]
}

fn peer_key(peer: &AutomationPeer) -> usize {
    peer as *const AutomationPeer as usize
}

/// Wraps an automation peer for the accessibility of UIKit.
pub(crate) struct AutomationPeerWrapper {
    this: Weak<AutomationPeerWrapper>,
    view: Rc<dyn IWrapperView>,
    peer: Ref<AutomationPeer>,
    parent: Option<Weak<AutomationPeerWrapper>>,
    children_list: RefCell<Vec<Ref<AutomationPeer>>>,
    /// The wrapper of each child, by the address of its peer, which the
    /// entry keeps alive.
    children_map: RefCell<HashMap<usize, (Ref<AutomationPeer>, Rc<AutomationPeerWrapper>)>>,
    is_container: bool,
    /// The handlers on the events of the peer.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,

    // What the element answers with.
    accessibility_identifier: RefCell<Option<String>>,
    accessibility_label: RefCell<Option<String>>,
    accessibility_hint: RefCell<Option<String>>,
    accessibility_value: RefCell<Option<String>>,
    accessibility_frame: Cell<Rect>,
    accessibility_traits: Cell<AccessibilityTraits>,
    accessibility_responds_to_user_interaction: Cell<bool>,
    is_accessibility_element: Cell<bool>,
    accessibility_container_type: Cell<AccessibilityContainerType>,

    /// The object of UIKit, made when UIKit first needs it.
    #[cfg(target_os = "ios")]
    element: std::cell::OnceCell<objc2::rc::Retained<uikit::AutomationPeerElement>>,
}

impl AutomationPeerWrapper {
    /// The wrapper of a child of `parent`.
    fn new_child(
        parent: &Rc<AutomationPeerWrapper>,
        view: Rc<dyn IWrapperView>,
        peer: Ref<AutomationPeer>,
    ) -> Rc<Self> {
        Self::create(Some(Rc::downgrade(parent)), view, peer)
    }

    /// The wrapper of the root peer of a view.
    pub(crate) fn new(view: Rc<dyn IWrapperView>, peer: Ref<AutomationPeer>) -> Rc<Self> {
        Self::create(None, view, peer)
    }

    fn create(parent: Option<Weak<AutomationPeerWrapper>>, view: Rc<dyn IWrapperView>, peer: Ref<AutomationPeer>) -> Rc<Self> {
        let control_type = peer.get_automation_control_type();
        let is_container = is_container_type(control_type);

        let this = Rc::new_cyclic(|this: &Weak<Self>| Self {
            this: this.clone(),
            view,
            peer: peer.clone(),
            parent,
            children_list: RefCell::new(Vec::new()),
            children_map: RefCell::new(HashMap::new()),
            is_container,
            subscriptions: RefCell::new(Vec::new()),
            accessibility_identifier: RefCell::new(None),
            accessibility_label: RefCell::new(None),
            accessibility_hint: RefCell::new(None),
            accessibility_value: RefCell::new(None),
            accessibility_frame: Cell::new(Rect::default()),
            accessibility_traits: Cell::new(AccessibilityTraits::empty()),
            // What an accessibility element of UIKit says before it is
            // told otherwise.
            accessibility_responds_to_user_interaction: Cell::new(true),
            is_accessibility_element: Cell::new(!is_container),
            accessibility_container_type: Cell::new(if is_container {
                AccessibilityContainerType::SemanticGroup
            } else {
                AccessibilityContainerType::None
            }),
            #[cfg(target_os = "ios")]
            element: std::cell::OnceCell::new(),
        });

        let weak = Rc::downgrade(&this);
        let children_changed = peer.children_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_children();
            }
        });
        let weak = Rc::downgrade(&this);
        let property_changed = peer.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.update_properties(&[e.property()]);
            }
        });
        *this.subscriptions.borrow_mut() = vec![children_changed, property_changed];

        this
    }

    pub(crate) fn accessibility_identifier(&self) -> Option<String> {
        self.accessibility_identifier.borrow().clone()
    }

    pub(crate) fn accessibility_label(&self) -> Option<String> {
        self.accessibility_label.borrow().clone()
    }

    pub(crate) fn accessibility_hint(&self) -> Option<String> {
        self.accessibility_hint.borrow().clone()
    }

    pub(crate) fn accessibility_value(&self) -> Option<String> {
        self.accessibility_value.borrow().clone()
    }

    pub(crate) fn accessibility_frame(&self) -> Rect {
        self.accessibility_frame.get()
    }

    pub(crate) fn accessibility_traits(&self) -> AccessibilityTraits {
        self.accessibility_traits.get()
    }

    pub(crate) fn accessibility_responds_to_user_interaction(&self) -> bool {
        self.accessibility_responds_to_user_interaction.get()
    }

    pub(crate) fn is_accessibility_element(&self) -> bool {
        self.is_accessibility_element.get()
    }

    pub(crate) fn accessibility_container_type(&self) -> AccessibilityContainerType {
        self.accessibility_container_type.get()
    }

    /// The number of the accessibility elements of the container, which
    /// are brought up to date first.
    pub(crate) fn accessibility_element_count(&self) -> isize {
        self.update_children();
        self.children_list.borrow().len() as isize
    }

    /// The wrapper of the accessibility element at an index. The reference
    /// fails for an index outside of the list, which would end the
    /// process from a method UIKit calls; the port answers with nothing.
    pub(crate) fn get_accessibility_element_at(&self, index: isize) -> Option<Rc<AutomationPeerWrapper>> {
        let children_list = self.children_list.borrow();
        let child = children_list.get(usize::try_from(index).ok()?)?;
        self.children_map.borrow().get(&peer_key(child)).map(|(_, wrapper)| wrapper.clone())
    }

    /// The index of the accessibility element of a wrapper, or
    /// [`NOT_FOUND`]. `None` stands for an element that is no wrapper.
    pub(crate) fn get_index_of_accessibility_element(&self, element: Option<&AutomationPeerWrapper>) -> isize {
        let Some(wrapper) = element else {
            return NOT_FOUND;
        };

        match self.children_list.borrow().iter().position(|child| *child == wrapper.peer) {
            Some(index_of) => index_of as isize,
            None => NOT_FOUND,
        }
    }

    fn update_children(&self) {
        self.update_all_properties();
        self.update_traits();

        let Some(this) = self.this.upgrade() else {
            return;
        };

        let mut children = Vec::new();
        let mut retained_children = HashSet::new();
        for child in self.peer.get_children().iter() {
            if child.is_offscreen() {
                continue;
            }

            let key = peer_key(child);
            let existing = self.children_map.borrow().get(&key).map(|(_, wrapper)| wrapper.clone());
            let wrapper = match existing {
                Some(wrapper) => wrapper,
                None => {
                    let wrapper = Self::new_child(&this, self.view.clone(), child.clone());
                    self.children_map.borrow_mut().insert(key, (child.clone(), wrapper.clone()));
                    wrapper
                }
            };

            children.push(child.clone());
            retained_children.insert(key);
            wrapper.update_all_properties();
            wrapper.update_traits();
        }

        let removed: Vec<Rc<AutomationPeerWrapper>> = {
            let mut children_map = self.children_map.borrow_mut();
            let keys: Vec<usize> = children_map.keys().copied().filter(|key| !retained_children.contains(key)).collect();
            keys.into_iter().filter_map(|key| children_map.remove(&key)).map(|(_, wrapper)| wrapper).collect()
        };
        for wrapper in removed {
            wrapper.dispose();
        }

        *self.children_list.borrow_mut() = children;
    }

    fn update_automation_id(&self) {
        *self.accessibility_identifier.borrow_mut() = self.peer.get_automation_id();
    }

    fn update_name(&self) {
        *self.accessibility_label.borrow_mut() = Some(self.peer.get_name());
    }

    fn update_help_text(&self) {
        *self.accessibility_hint.borrow_mut() = Some(self.peer.get_help_text());
    }

    fn update_bounding_rectangle(&self) {
        let bounds = self.peer.get_bounding_rectangle();
        let native_rect = frame_of(
            self.view.point_to_screen(bounds.top_left()).unwrap_or_default(),
            self.view.point_to_screen(bounds.bottom_right()).unwrap_or_default(),
        );
        if self.accessibility_frame.get() != native_rect {
            self.accessibility_frame.set(native_rect);
        }
    }

    fn update_is_read_only(&self) {
        let peer = &self.peer;
        let range_value_is_writable =
            peer.get_provider::<dyn IRangeValueProvider>().is_some_and(|provider| !provider.is_read_only());
        self.accessibility_responds_to_user_interaction.set(
            peer.is_enabled()
                && (peer.get_provider::<dyn IValueProvider>().is_some_and(|provider| !provider.is_read_only())
                    || range_value_is_writable
                    || self.get_selection_item_provider().is_some()
                    || peer.get_provider::<dyn IToggleProvider>().is_some()
                    || peer.get_provider::<dyn IInvokeProvider>().is_some()
                    || peer.get_provider::<dyn IScrollProvider>().is_some()),
        );

        let mut traits = self.accessibility_traits.get();
        traits.remove(AccessibilityTraits::ADJUSTABLE);
        if range_value_is_writable {
            traits.insert(AccessibilityTraits::ADJUSTABLE);
        }
        self.accessibility_traits.set(traits);
    }

    fn update_value(&self) {
        let new_value = match self.peer.get_provider::<dyn IRangeValueProvider>() {
            Some(provider) => Some(format_range_value(provider.value())),
            None => self.peer.get_provider::<dyn IValueProvider>().and_then(|provider| provider.value()),
        };
        if *self.accessibility_value.borrow() != new_value {
            *self.accessibility_value.borrow_mut() = new_value;
        }
    }

    fn update_selected(&self) {
        let mut traits = self.accessibility_traits.get();
        traits.remove(AccessibilityTraits::SELECTED);
        if self.get_selection_item_provider().is_some_and(|provider| provider.is_selected()) {
            traits.insert(AccessibilityTraits::SELECTED);
        }
        self.accessibility_traits.set(traits);
    }

    fn update_properties(&self, properties: &[&'static AutomationProperty]) {
        let setters = property_setters();
        let mut called_setters: Vec<PropertySetter> = Vec::new();
        for property in properties {
            let setter = setters.iter().find(|(key, _)| key == property).map(|(_, setter)| *setter);
            if let Some(setter) = setter {
                if !called_setters.contains(&setter) {
                    called_setters.push(setter);
                    match setter {
                        PropertySetter::AutomationId => self.update_automation_id(),
                        PropertySetter::Name => self.update_name(),
                        PropertySetter::HelpText => self.update_help_text(),
                        PropertySetter::BoundingRectangle => self.update_bounding_rectangle(),
                        PropertySetter::IsReadOnly => self.update_is_read_only(),
                        PropertySetter::Value => self.update_value(),
                        PropertySetter::Selected => self.update_selected(),
                    }
                }
            }
        }
    }

    /// Reads every property of the element from the peer.
    pub(crate) fn update_all_properties(&self) {
        let keys: Vec<&'static AutomationProperty> = property_setters().iter().map(|(key, _)| *key).collect();
        self.update_properties(&keys);
        let is_accessibility_element = !self.peer.is_offscreen() && self.peer.is_control_element();
        if self.is_container {
            let is_named_selection_item = self.peer.get_provider::<dyn ISelectionItemProvider>().is_some()
                && !self.peer.get_name().chars().all(char::is_whitespace);
            self.accessibility_container_type.set(if is_named_selection_item {
                AccessibilityContainerType::None
            } else {
                AccessibilityContainerType::SemanticGroup
            });
            self.is_accessibility_element.set(is_named_selection_item && is_accessibility_element);
        } else {
            self.is_accessibility_element.set(is_accessibility_element);
        }
    }

    /// Sets the traits of the element from the control type and the
    /// state of the peer.
    pub(crate) fn update_traits(&self) {
        let traits = traits_of(self.peer.get_automation_control_type(), self.peer.is_enabled());

        self.accessibility_traits.set(traits);
        self.update_selected();
    }

    /// The default action of the element. A provider that refuses (its
    /// element is not enabled) makes the answer false, where the failure
    /// of the reference would leave a method UIKit called.
    pub(crate) fn accessibility_activate(&self) -> bool {
        let selection_item_provider = self.peer.get_provider::<dyn ISelectionItemProvider>();
        let toggle_provider = self.peer.get_provider::<dyn IToggleProvider>();
        let invoke_provider = self.peer.get_provider::<dyn IInvokeProvider>();
        if let Some(selection_item_provider) = selection_item_provider {
            let selected = selection_item_provider.select().is_ok();
            self.update_traits();
            selected
        } else if let Some(toggle_provider) = toggle_provider {
            toggle_provider.toggle().is_ok()
        } else if let Some(invoke_provider) = invoke_provider {
            invoke_provider.invoke().is_ok()
        } else if let Some(parent_selection_item_provider) =
            self.parent().and_then(|parent| parent.get_selection_item_provider())
        {
            let selected = parent_selection_item_provider.select().is_ok();
            self.update_traits();
            selected
        } else {
            false
        }
    }

    pub(crate) fn accessibility_element_is_focused(&self) -> bool {
        self.peer.has_keyboard_focus()
    }

    pub(crate) fn accessibility_element_did_become_focused(&self) {
        self.peer.bring_into_view();
    }

    pub(crate) fn accessibility_decrement(&self) {
        if let Some(provider) = self.peer.get_provider::<dyn IRangeValueProvider>() {
            let value = provider.value();
            let _ = provider.set_value(value - provider.small_change());
        }
    }

    pub(crate) fn accessibility_increment(&self) {
        if let Some(provider) = self.peer.get_provider::<dyn IRangeValueProvider>() {
            let value = provider.value();
            let _ = provider.set_value(value + provider.small_change());
        }
    }

    pub(crate) fn accessibility_scroll(&self, direction: Option<AccessibilityScrollDirection>) -> bool {
        if let Some(scroll_provider) = self.peer.get_provider::<dyn IScrollProvider>() {
            let (horizontal_amount, vertical_amount, did_scroll) = scroll_amounts(direction);

            if scroll_provider.scroll(horizontal_amount, vertical_amount).is_ok() && did_scroll {
                self.view.post_page_scrolled();
                return true;
            }
        }
        false
    }

    fn parent(&self) -> Option<Rc<AutomationPeerWrapper>> {
        self.parent.as_ref().and_then(Weak::upgrade)
    }

    fn get_selection_item_provider(&self) -> Option<Rc<dyn ISelectionItemProvider>> {
        if let Some(provider) = self.peer.get_provider::<dyn ISelectionItemProvider>() {
            return Some(provider);
        }

        let mut wrapper = self.parent();
        while let Some(current) = wrapper {
            if let Some(provider) = current.peer.get_provider::<dyn ISelectionItemProvider>() {
                return Some(provider);
            }
            wrapper = current.parent();
        }

        None
    }

    /// Removes the handlers of the wrapper and of its children from
    /// their peers.
    pub(crate) fn dispose(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }

        let children: Vec<Rc<AutomationPeerWrapper>> =
            self.children_map.borrow_mut().drain().map(|(_, (_, wrapper))| wrapper).collect();
        for child in children {
            child.dispose();
        }

        self.children_list.borrow_mut().clear();
    }
}

#[cfg(target_os = "ios")]
pub(crate) use uikit::{element_object_of, wrapper_of, ViewWrapperHost};

#[cfg(target_os = "ios")]
mod uikit {
    use super::{AccessibilityScrollDirection, AutomationPeerWrapper, IWrapperView, NOT_FOUND};
    use crate::ferro_view::FerroView;
    use ferroui_base::{PixelPoint, Point};
    use objc2::rc::{Retained, Weak as ObjcWeak};
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_foundation::{NSInteger, NSObjectProtocol, NSString};
    use objc2_ui_kit::{
        UIAccessibilityElement, UIAccessibilityPageScrolledNotification, UIAccessibilityPostNotification,
        UIAccessibilityScrollDirection, UIView,
    };
    use std::rc::{Rc, Weak};

    /// What a wrapper asks of the view of the port.
    pub(crate) struct ViewWrapperHost {
        view: ObjcWeak<FerroView>,
    }

    impl ViewWrapperHost {
        pub(crate) fn new(view: &Retained<FerroView>) -> Rc<Self> {
            Rc::new(Self { view: ObjcWeak::from_retained(view) })
        }
    }

    impl IWrapperView for ViewWrapperHost {
        fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
            let root = self.view.load()?.try_top_level()?.get_input_root()?.try_root_element()?;
            Some(root.point_to_screen(point))
        }

        fn post_page_scrolled(&self) {
            // SAFETY: the notification of a scrolled page takes an
            // optional description of the new position; none is given.
            unsafe { UIAccessibilityPostNotification(UIAccessibilityPageScrolledNotification, None) };
        }

        fn native_view(&self) -> Option<Retained<UIView>> {
            self.view.load().map(Retained::into_super)
        }
    }

    /// The instance variables of an element: its wrapper, which owns the
    /// element.
    pub(crate) struct AutomationPeerElementIvars {
        wrapper: Weak<AutomationPeerWrapper>,
    }

    define_class!(
        // SAFETY: `UIAccessibilityElement` is made to be subclassed; the
        // overrides are the getters and the actions of the accessibility
        // protocols, with the types the protocols declare; the class does
        // not implement `Drop`.
        #[unsafe(super(UIAccessibilityElement))]
        #[thread_kind = MainThreadOnly]
        #[name = "FerroAutomationPeerElement"]
        #[ivars = AutomationPeerElementIvars]
        pub(crate) struct AutomationPeerElement;

        impl AutomationPeerElement {
            #[unsafe(method(accessibilityElementCount))]
            fn accessibility_element_count(&self) -> NSInteger {
                self.wrapper().map_or(0, |wrapper| wrapper.accessibility_element_count())
            }

            #[unsafe(method_id(accessibilityElementAtIndex:))]
            fn get_accessibility_element_at(&self, index: NSInteger) -> Option<Retained<AnyObject>> {
                let child = self.wrapper().and_then(|wrapper| wrapper.get_accessibility_element_at(index));
                child.and_then(|child| element_object_of(&child))
            }

            #[unsafe(method(indexOfAccessibilityElement:))]
            fn get_index_of_accessibility_element(&self, element: &AnyObject) -> NSInteger {
                let Some(wrapper) = self.wrapper() else {
                    return NOT_FOUND;
                };
                wrapper.get_index_of_accessibility_element(wrapper_of(element).as_deref())
            }

            #[unsafe(method(accessibilityContainerType))]
            fn accessibility_container_type(&self) -> NSInteger {
                self.wrapper().map_or(0, |wrapper| wrapper.accessibility_container_type() as NSInteger)
            }

            #[unsafe(method(isAccessibilityElement))]
            fn is_accessibility_element(&self) -> bool {
                self.wrapper().is_some_and(|wrapper| wrapper.is_accessibility_element())
            }

            #[unsafe(method_id(accessibilityIdentifier))]
            fn accessibility_identifier(&self) -> Option<Retained<NSString>> {
                let text = self.wrapper().and_then(|wrapper| wrapper.accessibility_identifier());
                text.map(|text| NSString::from_str(&text))
            }

            #[unsafe(method_id(accessibilityLabel))]
            fn accessibility_label(&self) -> Option<Retained<NSString>> {
                let text = self.wrapper().and_then(|wrapper| wrapper.accessibility_label());
                text.map(|text| NSString::from_str(&text))
            }

            #[unsafe(method_id(accessibilityHint))]
            fn accessibility_hint(&self) -> Option<Retained<NSString>> {
                let text = self.wrapper().and_then(|wrapper| wrapper.accessibility_hint());
                text.map(|text| NSString::from_str(&text))
            }

            #[unsafe(method_id(accessibilityValue))]
            fn accessibility_value(&self) -> Option<Retained<NSString>> {
                let text = self.wrapper().and_then(|wrapper| wrapper.accessibility_value());
                text.map(|text| NSString::from_str(&text))
            }

            #[unsafe(method(accessibilityFrame))]
            fn accessibility_frame(&self) -> CGRect {
                let frame = self.wrapper().map(|wrapper| wrapper.accessibility_frame()).unwrap_or_default();
                CGRect::new(CGPoint::new(frame.x, frame.y), CGSize::new(frame.width, frame.height))
            }

            #[unsafe(method(accessibilityTraits))]
            fn accessibility_traits(&self) -> u64 {
                self.wrapper().map_or(0, |wrapper| wrapper.accessibility_traits().bits())
            }

            #[unsafe(method(accessibilityRespondsToUserInteraction))]
            fn accessibility_responds_to_user_interaction(&self) -> bool {
                self.wrapper().is_some_and(|wrapper| wrapper.accessibility_responds_to_user_interaction())
            }

            #[unsafe(method(accessibilityActivate))]
            fn accessibility_activate(&self) -> bool {
                self.wrapper().is_some_and(|wrapper| wrapper.accessibility_activate())
            }

            #[unsafe(method(accessibilityElementIsFocused))]
            fn accessibility_element_is_focused(&self) -> bool {
                // SAFETY: the method of the superclass this one overrides.
                let _: bool = unsafe { msg_send![super(self), accessibilityElementIsFocused] };
                self.wrapper().is_some_and(|wrapper| wrapper.accessibility_element_is_focused())
            }

            #[unsafe(method(accessibilityElementDidBecomeFocused))]
            fn accessibility_element_did_become_focused(&self) {
                // SAFETY: the method of the superclass this one overrides.
                let _: () = unsafe { msg_send![super(self), accessibilityElementDidBecomeFocused] };
                if let Some(wrapper) = self.wrapper() {
                    wrapper.accessibility_element_did_become_focused();
                }
            }

            #[unsafe(method(accessibilityDecrement))]
            fn accessibility_decrement(&self) {
                // SAFETY: the method of the superclass this one overrides.
                let _: () = unsafe { msg_send![super(self), accessibilityDecrement] };
                if let Some(wrapper) = self.wrapper() {
                    wrapper.accessibility_decrement();
                }
            }

            #[unsafe(method(accessibilityIncrement))]
            fn accessibility_increment(&self) {
                // SAFETY: the method of the superclass this one overrides.
                let _: () = unsafe { msg_send![super(self), accessibilityIncrement] };
                if let Some(wrapper) = self.wrapper() {
                    wrapper.accessibility_increment();
                }
            }

            #[unsafe(method(accessibilityScroll:))]
            fn accessibility_scroll(&self, direction: UIAccessibilityScrollDirection) -> bool {
                // SAFETY: the method of the superclass this one overrides,
                // with its argument.
                let _: bool = unsafe { msg_send![super(self), accessibilityScroll: direction] };
                self.wrapper().is_some_and(|wrapper| {
                    wrapper.accessibility_scroll(AccessibilityScrollDirection::from_native(direction.0))
                })
            }
        }

        unsafe impl NSObjectProtocol for AutomationPeerElement {}
    );

    impl AutomationPeerElement {
        fn new(wrapper: &Rc<AutomationPeerWrapper>, container: &AnyObject, mtm: MainThreadMarker) -> Retained<Self> {
            let this = mtm.alloc::<Self>().set_ivars(AutomationPeerElementIvars { wrapper: Rc::downgrade(wrapper) });
            // SAFETY: the designated initializer of the superclass, on
            // the object that was just allocated and whose instance
            // variables are set; the container is an object of UIKit (the
            // view, or the element of the parent), which the element
            // refers to weakly.
            unsafe { msg_send![super(this), initWithAccessibilityContainer: container] }
        }

        /// The wrapper of the element, while its parent has it.
        fn wrapper(&self) -> Option<Rc<AutomationPeerWrapper>> {
            self.ivars().wrapper.upgrade()
        }
    }

    /// The object of UIKit of a wrapper: made once, with the element of
    /// the parent, or the view for the root, as its accessibility
    /// container.
    fn element_of(wrapper: &Rc<AutomationPeerWrapper>) -> Option<Retained<AutomationPeerElement>> {
        if let Some(element) = wrapper.element.get() {
            return Some(element.clone());
        }

        let mtm = MainThreadMarker::new()?;
        let container: Retained<AnyObject> = match wrapper.parent() {
            Some(parent) => element_object_of(&parent)?,
            None => Retained::into_super(Retained::into_super(wrapper.view.native_view()?)).into(),
        };
        let element = AutomationPeerElement::new(wrapper, &container, mtm);
        Some(wrapper.element.get_or_init(|| element).clone())
    }

    /// The object of UIKit of a wrapper, as UIKit is given it.
    pub(crate) fn element_object_of(wrapper: &Rc<AutomationPeerWrapper>) -> Option<Retained<AnyObject>> {
        let element = element_of(wrapper)?;
        Some(Retained::into_super(Retained::into_super(Retained::into_super(element))).into())
    }

    /// The wrapper of an accessibility element, when the element is one
    /// of the port whose wrapper is alive.
    pub(crate) fn wrapper_of(element: &AnyObject) -> Option<Rc<AutomationPeerWrapper>> {
        element.downcast_ref::<AutomationPeerElement>()?.wrapper()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::StyledElement;
    use ferroui_controls::automation::peers::ControlAutomationPeer;
    use ferroui_controls::automation::{AutomationProperties, IsOffscreenBehavior};
    use ferroui_controls::primitives::RangeBase;
    use ferroui_controls::{Button, CheckBox, Control, Slider, StackPanel, TextBox};

    /// A view whose root is at an offset on a screen of a scaling.
    struct TestView {
        offset: (f64, f64),
        scaling: f64,
        page_scrolled: Cell<u32>,
    }

    impl TestView {
        fn new() -> Rc<Self> {
            Rc::new(Self { offset: (0.0, 0.0), scaling: 1.0, page_scrolled: Cell::new(0) })
        }
    }

    impl IWrapperView for TestView {
        fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
            Some(PixelPoint::new(
                ((point.x + self.offset.0) * self.scaling) as i32,
                ((point.y + self.offset.1) * self.scaling) as i32,
            ))
        }

        fn post_page_scrolled(&self) {
            self.page_scrolled.set(self.page_scrolled.get() + 1);
        }
    }

    /// A control outside of a tree counts as off the screen; the tests
    /// say that theirs are on it.
    fn mark_onscreen(element: &StyledElement, name: &str) {
        AutomationProperties::set_is_offscreen_behavior(element, IsOffscreenBehavior::Onscreen);
        if !name.is_empty() {
            AutomationProperties::set_name(element, Some(name));
        }
    }

    macro_rules! onscreen {
        ($control:expr, $name:expr) => {{
            let control = $control;
            mark_onscreen(&control, $name);
            control
        }};
    }

    fn wrapper_of_control(control: &Control) -> (Rc<AutomationPeerWrapper>, Rc<TestView>) {
        let view = TestView::new();
        let wrapper = AutomationPeerWrapper::new(view.clone(), ControlAutomationPeer::create_peer_for_element(control));
        (wrapper, view)
    }

    #[test]
    fn the_traits_follow_the_control_type_and_the_enabled_state() {
        assert_eq!(AccessibilityTraits::BUTTON, traits_of(AutomationControlType::Button, true));
        assert_eq!(AccessibilityTraits::HEADER, traits_of(AutomationControlType::Header, true));
        assert_eq!(AccessibilityTraits::LINK, traits_of(AutomationControlType::Hyperlink, true));
        assert_eq!(AccessibilityTraits::IMAGE, traits_of(AutomationControlType::Image, true));
        assert_eq!(AccessibilityTraits::empty(), traits_of(AutomationControlType::Slider, true));
        assert_eq!(AccessibilityTraits::empty(), traits_of(AutomationControlType::CheckBox, true));
        assert_eq!(AccessibilityTraits::empty(), traits_of(AutomationControlType::Edit, true));
        assert_eq!(
            AccessibilityTraits::BUTTON | AccessibilityTraits::NOT_ENABLED,
            traits_of(AutomationControlType::Button, false)
        );
        assert_eq!(AccessibilityTraits::NOT_ENABLED, traits_of(AutomationControlType::Text, false));
    }

    #[test]
    fn the_container_types_are_the_ones_of_the_reference() {
        for control_type in [
            AutomationControlType::Custom,
            AutomationControlType::Group,
            AutomationControlType::List,
            AutomationControlType::ListItem,
            AutomationControlType::Pane,
            AutomationControlType::ScrollViewer,
            AutomationControlType::TabItem,
            AutomationControlType::Window,
        ] {
            assert!(is_container_type(control_type), "{control_type:?}");
        }
        // Unlike on Android, a peer without a control type is no container.
        for control_type in [
            AutomationControlType::None,
            AutomationControlType::Button,
            AutomationControlType::CheckBox,
            AutomationControlType::Edit,
            AutomationControlType::Slider,
            AutomationControlType::Text,
            AutomationControlType::ComboBox,
        ] {
            assert!(!is_container_type(control_type), "{control_type:?}");
        }
    }

    #[test]
    fn the_frame_is_the_rectangle_between_the_corners_on_the_screen() {
        assert_eq!(Rect::new(10.0, 20.0, 30.0, 40.0), frame_of(PixelPoint::new(10, 20), PixelPoint::new(40, 60)));
        assert_eq!(Rect::new(0.0, 0.0, 0.0, 0.0), frame_of(PixelPoint::default(), PixelPoint::default()));
    }

    #[test]
    fn a_range_value_has_at_most_two_decimals() {
        assert_eq!("0", format_range_value(0.0));
        assert_eq!("5", format_range_value(5.0));
        assert_eq!("5.5", format_range_value(5.5));
        assert_eq!("5.25", format_range_value(5.25));
        assert_eq!("0.33", format_range_value(1.0 / 3.0));
        assert_eq!("0.67", format_range_value(2.0 / 3.0));
        assert_eq!("0.13", format_range_value(0.125));
        assert_eq!("-0.13", format_range_value(-0.125));
        assert_eq!("0.38", format_range_value(0.375));
        assert_eq!("1", format_range_value(1.005));
        assert_eq!("100", format_range_value(99.999));
        assert_eq!("12.3", format_range_value(12.3));
    }

    #[test]
    fn a_scroll_is_a_page_in_the_direction() {
        use AccessibilityScrollDirection::*;
        assert_eq!((ScrollAmount::NoAmount, ScrollAmount::LargeIncrement, true), scroll_amounts(Some(Up)));
        assert_eq!((ScrollAmount::NoAmount, ScrollAmount::LargeDecrement, true), scroll_amounts(Some(Down)));
        assert_eq!((ScrollAmount::LargeIncrement, ScrollAmount::NoAmount, true), scroll_amounts(Some(Left)));
        assert_eq!((ScrollAmount::LargeDecrement, ScrollAmount::NoAmount, true), scroll_amounts(Some(Right)));
        assert_eq!((ScrollAmount::NoAmount, ScrollAmount::NoAmount, false), scroll_amounts(Some(Next)));
        assert_eq!((ScrollAmount::NoAmount, ScrollAmount::NoAmount, false), scroll_amounts(None));
        assert_eq!(Some(Up), AccessibilityScrollDirection::from_native(3));
        assert_eq!(None, AccessibilityScrollDirection::from_native(0));
    }

    #[test]
    fn the_children_are_the_children_of_the_peer_in_their_order() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let button = onscreen!(Button::new(), "Press");
        let text_box = onscreen!(TextBox::new(), "Name");
        let check_box = onscreen!(CheckBox::new(), "Agree");
        let slider = onscreen!(Slider::new(), "Volume");
        panel.children().add(button.clone());
        panel.children().add(text_box.clone());
        panel.children().add(check_box.clone());
        panel.children().add(slider.clone());

        let (wrapper, _view) = wrapper_of_control(&panel);
        assert_eq!(4, wrapper.accessibility_element_count());

        let labels: Vec<Option<String>> = (0..4)
            .map(|index| wrapper.get_accessibility_element_at(index).and_then(|child| child.accessibility_label()))
            .collect();
        assert_eq!(
            vec![
                Some("Press".to_string()),
                Some("Name".to_string()),
                Some("Agree".to_string()),
                Some("Volume".to_string())
            ],
            labels
        );

        // The index of an element is its place in the list; an element of
        // another container, and one that is no wrapper, are not found.
        let third = wrapper.get_accessibility_element_at(2).unwrap();
        assert_eq!(2, wrapper.get_index_of_accessibility_element(Some(&third)));
        assert_eq!(NOT_FOUND, third.get_index_of_accessibility_element(Some(&wrapper)));
        assert_eq!(NOT_FOUND, wrapper.get_index_of_accessibility_element(None));
        assert!(wrapper.get_accessibility_element_at(4).is_none());
        assert!(wrapper.get_accessibility_element_at(-1).is_none());

        // The same wrapper for the same peer, as long as the peer is a child.
        let again = wrapper.get_accessibility_element_at(2).unwrap();
        assert!(Rc::ptr_eq(&third, &again));

        // A child that leaves is not an element any more; the others keep
        // their wrappers.
        let second = wrapper.get_accessibility_element_at(1).unwrap();
        panel.children().remove(text_box.clone());
        assert_eq!(3, wrapper.accessibility_element_count());
        assert!(Rc::ptr_eq(&third, &wrapper.get_accessibility_element_at(1).unwrap()));
        assert_eq!(NOT_FOUND, wrapper.get_index_of_accessibility_element(Some(&second)));

        // A child that is off the screen is passed over.
        AutomationProperties::set_is_offscreen_behavior(&check_box, IsOffscreenBehavior::Offscreen);
        assert_eq!(2, wrapper.accessibility_element_count());
    }

    #[test]
    fn a_button_is_an_element_with_the_button_trait_that_activates() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let button = onscreen!(Button::new(), "Press");
        panel.children().add(button.clone());
        let clicks = Rc::new(Cell::new(0));
        let counter = clicks.clone();
        button.click(move |_, _| counter.set(counter.get() + 1));

        let (wrapper, _view) = wrapper_of_control(&panel);
        assert_eq!(1, wrapper.accessibility_element_count());
        // A panel has a peer without a control type: no element of its
        // own, and no semantic group either; its children are reached
        // through it.
        assert!(!wrapper.is_accessibility_element());
        assert_eq!(AccessibilityContainerType::None, wrapper.accessibility_container_type());
        assert!(!wrapper.accessibility_activate());

        let element = wrapper.get_accessibility_element_at(0).unwrap();
        assert!(element.is_accessibility_element());
        assert_eq!(AccessibilityContainerType::None, element.accessibility_container_type());
        assert_eq!(AccessibilityTraits::BUTTON, element.accessibility_traits());
        assert!(element.accessibility_responds_to_user_interaction());
        assert_eq!(None, element.accessibility_value());

        assert!(element.accessibility_activate());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, clicks.get());

        // Not enabled: the trait says so, the element does not respond,
        // and activating it is refused.
        button.set_is_enabled(false);
        wrapper.accessibility_element_count();
        assert_eq!(AccessibilityTraits::BUTTON | AccessibilityTraits::NOT_ENABLED, element.accessibility_traits());
        assert!(!element.accessibility_responds_to_user_interaction());
        assert!(!element.accessibility_activate());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, clicks.get());
    }

    #[test]
    fn a_check_box_toggles_and_a_text_box_has_its_text_as_the_value() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let check_box = onscreen!(CheckBox::new(), "Agree");
        let text_box = onscreen!(TextBox::new(), "Name");
        text_box.set_text(Some("abc"));
        AutomationProperties::set_help_text(&text_box, Some("Your name"));
        AutomationProperties::set_automation_id(&text_box, Some("name-box"));
        panel.children().add(check_box.clone());
        panel.children().add(text_box.clone());

        let (wrapper, _view) = wrapper_of_control(&panel);
        assert_eq!(2, wrapper.accessibility_element_count());
        let check = wrapper.get_accessibility_element_at(0).unwrap();
        let text = wrapper.get_accessibility_element_at(1).unwrap();

        assert_eq!(Some(false), check_box.is_checked());
        assert!(check.accessibility_activate());
        assert_eq!(Some(true), check_box.is_checked());

        assert_eq!(Some("abc".to_string()), text.accessibility_value());
        assert_eq!(Some("Your name".to_string()), text.accessibility_hint());
        assert_eq!(Some("name-box".to_string()), text.accessibility_identifier());
        assert!(text.accessibility_responds_to_user_interaction());
        // A text box has no default action.
        assert!(!text.accessibility_activate());

        // A change of the value of the peer reaches the element without
        // a question of the container.
        text_box.set_text(Some("abcd"));
        assert_eq!(Some("abcd".to_string()), text.accessibility_value());
    }

    #[test]
    fn a_slider_has_its_value_and_is_incremented_by_its_small_change() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let slider = onscreen!(Slider::new(), "Volume");
        let range: &RangeBase = &slider;
        range.set_minimum(0.0);
        range.set_maximum(10.0);
        range.set_small_change(0.5);
        range.set_range_value(4.0);
        panel.children().add(slider.clone());

        let (wrapper, _view) = wrapper_of_control(&panel);
        assert_eq!(1, wrapper.accessibility_element_count());
        let element = wrapper.get_accessibility_element_at(0).unwrap();
        assert_eq!(Some("4".to_string()), element.accessibility_value());
        assert!(element.accessibility_responds_to_user_interaction());

        element.accessibility_increment();
        assert_eq!(4.5, range.value());
        assert_eq!(Some("4.5".to_string()), element.accessibility_value());
        element.accessibility_decrement();
        element.accessibility_decrement();
        assert_eq!(3.5, range.value());
        assert_eq!(Some("3.5".to_string()), element.accessibility_value());

        // The reference sets the adjustable trait with the properties
        // and then sets the traits anew from the control type, so the
        // trait is gone after every update of a container. Kept as it is
        // (docs/porting/ios-platform.md, section 10c).
        assert_eq!(AccessibilityTraits::empty(), element.accessibility_traits());
        element.update_all_properties();
        assert_eq!(AccessibilityTraits::ADJUSTABLE, element.accessibility_traits());
        element.update_traits();
        assert_eq!(AccessibilityTraits::empty(), element.accessibility_traits());
    }

    #[test]
    fn the_frame_of_an_element_is_its_bounds_on_the_screen() {
        let _scope = Dispatcher::unit_test_scope();
        let button = onscreen!(Button::new(), "Press");
        let view = Rc::new(TestView { offset: (5.0, 7.0), scaling: 2.0, page_scrolled: Cell::new(0) });
        let wrapper =
            AutomationPeerWrapper::new(view.clone(), ControlAutomationPeer::create_peer_for_element(&button));
        wrapper.update_all_properties();
        // A control outside of a tree has empty bounds at the origin of
        // the root, which the view puts on the screen.
        assert_eq!(Rect::new(10.0, 14.0, 0.0, 0.0), wrapper.accessibility_frame());
        assert_eq!(0, view.page_scrolled.get());
    }

    #[test]
    fn a_disposed_wrapper_does_not_follow_its_peer() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let text_box = onscreen!(TextBox::new(), "Name");
        text_box.set_text(Some("abc"));
        panel.children().add(text_box.clone());

        let (wrapper, _view) = wrapper_of_control(&panel);
        assert_eq!(1, wrapper.accessibility_element_count());
        let text = wrapper.get_accessibility_element_at(0).unwrap();
        assert_eq!(Some("abc".to_string()), text.accessibility_value());

        wrapper.dispose();
        assert!(wrapper.get_accessibility_element_at(0).is_none());
        text_box.set_text(Some("abcd"));
        assert_eq!(Some("abc".to_string()), text.accessibility_value());
    }
}
