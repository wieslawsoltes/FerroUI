use crate::automation::provider::IRootProvider;
use crate::automation::{
    AutomationLiveSetting, AutomationProperty, AutomationPropertyChangedEventArgs, ElementNotEnabledException,
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_class, AnyValue, BoxedValue, FerroObject, FerroObjectImpl, Rect, Ref};
use std::rc::Rc;

/// The control type an automation peer reports for its element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AutomationControlType {
    None,
    Button,
    Calendar,
    CheckBox,
    ComboBox,
    ComboBoxItem,
    Edit,
    Hyperlink,
    Image,
    ListItem,
    List,
    Menu,
    MenuBar,
    MenuItem,
    ProgressBar,
    RadioButton,
    ScrollBar,
    Slider,
    Spinner,
    StatusBar,
    Tab,
    TabItem,
    Text,
    ToolBar,
    ToolTip,
    Tree,
    TreeItem,
    Custom,
    Group,
    Thumb,
    DataGrid,
    DataItem,
    Document,
    SplitButton,
    Window,
    Pane,
    Header,
    HeaderItem,
    Table,
    TitleBar,
    Separator,
    Expander,
    ScrollViewer,
}

/// The landmark type an automation peer reports for its element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AutomationLandmarkType {
    Banner,
    Complementary,
    ContentInfo,
    Region,
    Form,
    Main,
    Navigation,
    Search,
}

/// Provides a base class that exposes an element to UI Automation.
///
/// The members named `*_core` are the overridable ones; the members without
/// the suffix are the public surface that automation clients call. The
/// members the reference declares abstract panic in this class: a peer class
/// overrides every one of them (deriving from `ControlAutomationPeer` does).
#[repr(C)]
pub struct AutomationPeer {
    base: FerroObject,
    children_changed: HandlerList<dyn Fn()>,
    property_changed: HandlerList<dyn Fn(&AutomationPropertyChangedEventArgs)>,
}

ferro_class! {
    AutomationPeer: FerroObject, virtuals AutomationPeerImpl: FerroObjectImpl {
        /// Gets a human-readable localized string that represents the type of the control
        /// that is associated with this automation peer.
        fn get_localized_control_type_core(this) -> String;
        fn bring_into_view_core(this);
        fn get_accelerator_key_core(this) -> Option<String>;
        fn get_access_key_core(this) -> Option<String>;
        fn get_automation_control_type_core(this) -> AutomationControlType;
        fn get_automation_id_core(this) -> Option<String>;
        fn get_bounding_rectangle_core(this) -> Rect;
        fn get_or_create_children_core(this) -> Rc<Vec<Ref<AutomationPeer>>>;
        fn get_class_name_core(this) -> String;
        fn get_labeled_by_core(this) -> Option<Ref<AutomationPeer>>;
        fn get_name_core(this) -> Option<String>;
        fn get_help_text_core(this) -> Option<String>;
        fn get_placeholder_text_core(this) -> Option<String>;
        fn get_landmark_type_core(this) -> Option<AutomationLandmarkType>;
        fn get_heading_level_core(this) -> i32;
        fn get_item_type_core(this) -> Option<String>;
        fn get_item_status_core(this) -> Option<String>;
        fn get_parent_core(this) -> Option<Ref<AutomationPeer>>;
        fn has_keyboard_focus_core(this) -> bool;
        fn is_content_element_core(this) -> bool;
        fn is_control_element_core(this) -> bool;
        fn is_enabled_core(this) -> bool;
        fn is_keyboard_focusable_core(this) -> bool;
        fn is_offscreen_core(this) -> bool;
        fn set_focus_core(this);
        fn show_context_menu_core(this) -> bool;
        fn get_live_setting_core(this) -> AutomationLiveSetting;
        fn get_control_type_override_core(this) -> AutomationControlType;
        fn get_class_name_override_core(this) -> String;
        /// Gets the root of the automation tree of the peer (not for use
        /// outside the framework).
        fn get_automation_root_core(this) -> Option<Ref<AutomationPeer>>;
        /// Gets the peer of the visual root of the element (not for use
        /// outside the framework).
        fn get_visual_root_core(this) -> Option<Ref<AutomationPeer>>;
        /// Converts a rectangle in the coordinates of the visual root to
        /// screen coordinates (not for use outside the framework).
        fn to_screen_core(this, rect: Rect) -> Option<Rect>;
        fn is_content_element_override_core(this) -> bool;
        fn is_control_element_override_core(this) -> bool;
        /// Gets the handle of the provider contract `provider_type` (the
        /// type of the handle, for example `Rc<dyn IInvokeProvider>`) if the
        /// peer provides it.
        fn get_provider_core(this, provider_type: ValueType) -> Option<BoxedValue>;
        /// Sets the parent of the peer; returns whether the peer accepted
        /// it.
        fn try_set_parent(this, parent: Option<Ref<AutomationPeer>>) -> bool;
    }
}
ferroui_base::ferro_class_info!(AutomationPeer {});

fn abstract_member(name: &str) -> ! {
    panic!("The automation peer class does not implement the abstract member `{name}`.")
}

impl FerroObjectImpl for AutomationPeer {}

impl AutomationPeerImpl for AutomationPeer {
    fn get_localized_control_type_core(this: &Self) -> String {
        let control_type = this.get_automation_control_type();

        match control_type {
            AutomationControlType::CheckBox => "check box".to_owned(),
            AutomationControlType::ComboBox => "combo box".to_owned(),
            AutomationControlType::ListItem => "list item".to_owned(),
            AutomationControlType::MenuBar => "menu bar".to_owned(),
            AutomationControlType::MenuItem => "menu item".to_owned(),
            AutomationControlType::ProgressBar => "progress bar".to_owned(),
            AutomationControlType::RadioButton => "radio button".to_owned(),
            AutomationControlType::ScrollBar => "scroll bar".to_owned(),
            AutomationControlType::StatusBar => "status bar".to_owned(),
            AutomationControlType::TabItem => "tab item".to_owned(),
            AutomationControlType::ToolBar => "toolbar".to_owned(),
            AutomationControlType::ToolTip => "tooltip".to_owned(),
            AutomationControlType::TreeItem => "tree item".to_owned(),
            AutomationControlType::Custom => "custom".to_owned(),
            AutomationControlType::DataGrid => "data grid".to_owned(),
            AutomationControlType::DataItem => "data item".to_owned(),
            AutomationControlType::SplitButton => "split button".to_owned(),
            AutomationControlType::HeaderItem => "header item".to_owned(),
            AutomationControlType::TitleBar => "title bar".to_owned(),
            AutomationControlType::Expander => "group".to_owned(),
            AutomationControlType::ScrollViewer => "scroll viewer".to_owned(),
            AutomationControlType::None => match this.get_landmark_type() {
                Some(landmark) => format!("{landmark:?}").to_lowercase(),
                None => format!("{control_type:?}").to_lowercase(),
            },
            _ => format!("{control_type:?}").to_lowercase(),
        }
    }

    fn bring_into_view_core(_this: &Self) {
        abstract_member("bring_into_view_core")
    }

    fn get_accelerator_key_core(_this: &Self) -> Option<String> {
        abstract_member("get_accelerator_key_core")
    }

    fn get_access_key_core(_this: &Self) -> Option<String> {
        abstract_member("get_access_key_core")
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        abstract_member("get_automation_control_type_core")
    }

    fn get_automation_id_core(_this: &Self) -> Option<String> {
        abstract_member("get_automation_id_core")
    }

    fn get_bounding_rectangle_core(_this: &Self) -> Rect {
        abstract_member("get_bounding_rectangle_core")
    }

    fn get_or_create_children_core(_this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        abstract_member("get_or_create_children_core")
    }

    fn get_class_name_core(_this: &Self) -> String {
        abstract_member("get_class_name_core")
    }

    fn get_labeled_by_core(_this: &Self) -> Option<Ref<AutomationPeer>> {
        abstract_member("get_labeled_by_core")
    }

    fn get_name_core(_this: &Self) -> Option<String> {
        abstract_member("get_name_core")
    }

    fn get_help_text_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_placeholder_text_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_landmark_type_core(_this: &Self) -> Option<AutomationLandmarkType> {
        None
    }

    fn get_heading_level_core(_this: &Self) -> i32 {
        0
    }

    fn get_item_type_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_item_status_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_parent_core(_this: &Self) -> Option<Ref<AutomationPeer>> {
        abstract_member("get_parent_core")
    }

    fn has_keyboard_focus_core(_this: &Self) -> bool {
        abstract_member("has_keyboard_focus_core")
    }

    fn is_content_element_core(_this: &Self) -> bool {
        abstract_member("is_content_element_core")
    }

    fn is_control_element_core(_this: &Self) -> bool {
        abstract_member("is_control_element_core")
    }

    fn is_enabled_core(_this: &Self) -> bool {
        abstract_member("is_enabled_core")
    }

    fn is_keyboard_focusable_core(_this: &Self) -> bool {
        abstract_member("is_keyboard_focusable_core")
    }

    fn is_offscreen_core(_this: &Self) -> bool {
        false
    }

    fn set_focus_core(_this: &Self) {
        abstract_member("set_focus_core")
    }

    fn show_context_menu_core(_this: &Self) -> bool {
        abstract_member("show_context_menu_core")
    }

    fn get_live_setting_core(_this: &Self) -> AutomationLiveSetting {
        AutomationLiveSetting::Off
    }

    fn get_control_type_override_core(this: &Self) -> AutomationControlType {
        this.get_automation_control_type_core()
    }

    fn get_class_name_override_core(this: &Self) -> String {
        this.get_class_name_core()
    }

    fn get_automation_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        let mut peer = this.to_ref();
        let mut parent = peer.get_parent();

        while peer.get_provider::<dyn IRootProvider>().is_none() {
            let Some(p) = parent else { break };
            peer = p;
            parent = peer.get_parent();
        }

        Some(peer)
    }

    fn get_visual_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.get_automation_root_core()
    }

    fn to_screen_core(_this: &Self, _rect: Rect) -> Option<Rect> {
        None
    }

    fn is_content_element_override_core(this: &Self) -> bool {
        this.is_control_element() && this.is_content_element_core()
    }

    fn is_control_element_override_core(this: &Self) -> bool {
        this.is_control_element_core()
    }

    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        // The reference tests whether the provider type is assignable from
        // the class of the peer; here a class declares the provider handles
        // it converts to (the `interfaces` of its class information).
        let object: BoxedValue = Rc::new(this.to_ref().upcast::<FerroObject>());
        ValueTypes::try_cast(&object, provider_type)
    }

    fn try_set_parent(_this: &Self, _parent: Option<Ref<AutomationPeer>>) -> bool {
        abstract_member("try_set_parent")
    }
}

impl AutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), children_changed: HandlerList::new(), property_changed: HandlerList::new() }
    }

    /// Attempts to bring the element associated with the automation peer into view.
    pub fn bring_into_view(&self) {
        self.bring_into_view_core()
    }

    /// Gets the accelerator key combinations for the element that is associated with the UI
    /// Automation peer.
    pub fn get_accelerator_key(&self) -> Option<String> {
        self.get_accelerator_key_core()
    }

    /// Gets the access key for the element that is associated with the automation peer.
    pub fn get_access_key(&self) -> Option<String> {
        self.get_access_key_core()
    }

    /// Gets the control type for the element that is associated with the UI Automation peer.
    pub fn get_automation_control_type(&self) -> AutomationControlType {
        self.get_control_type_override_core()
    }

    /// Gets the automation ID of the element that is associated with the automation peer.
    pub fn get_automation_id(&self) -> Option<String> {
        self.get_automation_id_core()
    }

    /// Gets the bounding rectangle of the element that is associated with the automation peer
    /// in top-level coordinates.
    pub fn get_bounding_rectangle(&self) -> Rect {
        self.get_bounding_rectangle_core()
    }

    /// Gets the child automation peers.
    pub fn get_children(&self) -> Rc<Vec<Ref<AutomationPeer>>> {
        self.get_or_create_children_core()
    }

    /// Gets a string that describes the class of the element.
    pub fn get_class_name(&self) -> String {
        self.get_class_name_override_core()
    }

    /// Gets the automation peer for the label that is targeted to the element.
    pub fn get_labeled_by(&self) -> Option<Ref<AutomationPeer>> {
        self.get_labeled_by_core()
    }

    /// Gets a human-readable localized string that represents the type of the control that is
    /// associated with this automation peer.
    pub fn get_localized_control_type(&self) -> String {
        self.get_localized_control_type_core()
    }

    /// Gets text that describes the element that is associated with this automation peer.
    pub fn get_name(&self) -> String {
        self.get_name_core().unwrap_or_default()
    }

    /// Gets text that provides help for the element that is associated with this automation peer.
    pub fn get_help_text(&self) -> String {
        self.get_help_text_core().unwrap_or_default()
    }

    /// Gets the placeholder text for the element that is associated with this automation peer.
    pub fn get_placeholder_text(&self) -> String {
        self.get_placeholder_text_core().unwrap_or_default()
    }

    /// Gets the control type for the element that is associated with the UI Automation peer.
    pub fn get_landmark_type(&self) -> Option<AutomationLandmarkType> {
        self.get_landmark_type_core()
    }

    /// Gets the heading level that is associated with this automation peer.
    pub fn get_heading_level(&self) -> i32 {
        self.get_heading_level_core()
    }

    /// Gets the item type that is associated with this automation peer.
    pub fn get_item_type(&self) -> Option<String> {
        self.get_item_type_core()
    }

    /// Gets the item status that is associated with this automation peer.
    pub fn get_item_status(&self) -> Option<String> {
        self.get_item_status_core()
    }

    /// Gets the automation peer that is the parent of this automation peer.
    pub fn get_parent(&self) -> Option<Ref<AutomationPeer>> {
        self.get_parent_core()
    }

    /// Gets the automation peer that is the root of this automation peer's
    /// visual tree (not for use outside the framework).
    pub fn get_visual_root(&self) -> Option<Ref<AutomationPeer>> {
        self.get_visual_root_core()
    }

    /// Gets the automation peer that is the root of this automation peer's
    /// automation tree (not for use outside the framework).
    pub fn get_automation_root(&self) -> Option<Ref<AutomationPeer>> {
        self.get_automation_root_core()
    }

    /// Converts a rectangle in the coordinates of the visual root to screen
    /// coordinates.
    pub fn to_screen(&self, rect: Rect) -> Option<Rect> {
        self.to_screen_core(rect)
    }

    /// Gets a value that indicates whether the element that is associated with this automation
    /// peer currently has keyboard focus.
    pub fn has_keyboard_focus(&self) -> bool {
        self.has_keyboard_focus_core()
    }

    /// Gets a value that indicates whether the element that is associated with this automation
    /// peer contains data that is presented to the user.
    pub fn is_content_element(&self) -> bool {
        self.is_content_element_override_core()
    }

    /// Gets a value that indicates whether the element is understood by the user as
    /// interactive or as contributing to the logical structure of the control in the GUI.
    pub fn is_control_element(&self) -> bool {
        self.is_control_element_override_core()
    }

    /// Gets a value indicating whether the control is enabled for user interaction.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled_core()
    }

    /// Gets a value that indicates whether the element can accept keyboard focus.
    pub fn is_keyboard_focusable(&self) -> bool {
        self.is_keyboard_focusable_core()
    }

    /// Gets a value that indicates whether an element is off the screen.
    pub fn is_offscreen(&self) -> bool {
        self.is_offscreen_core()
    }

    /// Sets the keyboard focus on the element that is associated with this automation peer.
    pub fn set_focus(&self) {
        self.set_focus_core()
    }

    /// Shows the context menu for the element that is associated with this automation peer.
    /// Returns true if a context menu is present for the element; otherwise false.
    pub fn show_context_menu(&self) -> bool {
        self.show_context_menu_core()
    }

    /// Gets the current live setting that is associated with this automation peer.
    pub fn get_live_setting(&self) -> AutomationLiveSetting {
        self.get_live_setting_core()
    }

    /// Tries to get a provider of the specified contract from the peer:
    /// `peer.get_provider::<dyn IInvokeProvider>()`.
    ///
    /// Returns the provider, or `None` if the peer does not provide it.
    pub fn get_provider<I: ?Sized + 'static>(&self) -> Option<Rc<I>> {
        let provider = self.get_provider_core(ValueType::of::<Rc<I>>())?;
        let provider: &dyn AnyValue = &*provider;
        provider.downcast_ref::<Rc<I>>().cloned()
    }

    /// Occurs when the children of the automation peer have changed.
    pub fn children_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.children_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.children_changed.remove(token);
            }
        })
    }

    /// Occurs when a property value of the automation peer has changed.
    pub fn property_changed(
        &self,
        handler: impl Fn(&AutomationPropertyChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.property_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.property_changed.remove(token);
            }
        })
    }

    /// Raises an event to notify the automation client the peer's children have changed.
    pub fn raise_children_changed_event(&self) {
        for (_, handler) in self.children_changed.snapshot().iter() {
            handler();
        }
    }

    /// Raises an event to notify the automation client of a changed property value.
    pub fn raise_property_changed_event(
        &self,
        property: &'static AutomationProperty,
        old_value: Option<BoxedValue>,
        new_value: Option<BoxedValue>,
    ) {
        if self.property_changed.is_empty() {
            return;
        }
        let e = AutomationPropertyChangedEventArgs::new(property, old_value, new_value);
        for (_, handler) in self.property_changed.snapshot().iter() {
            handler(&e);
        }
    }

    /// Returns the error of an element that is not enabled unless the
    /// element of the peer is enabled.
    pub fn ensure_enabled(&self) -> Result<(), ElementNotEnabledException> {
        if !self.is_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        Ok(())
    }
}
