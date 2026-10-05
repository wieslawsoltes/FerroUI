use super::{AutomationControlType, AutomationLandmarkType, AutomationPeer, AutomationPeerImpl};
use crate::automation::{
    AccessibilityView, AutomationElementIdentifiers, AutomationLiveSetting, AutomationProperties, IsOffscreenBehavior,
};
use crate::{Control, DataValidationErrors, ToolTip};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, instantiate, AnyValue, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, PixelRect, Rect, Ref, Visual, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An automation peer which represents a [`Control`].
///
/// The peer does not keep its control alive (the control owns the peer); see
/// [`owner`](Self::owner).
#[repr(C)]
pub struct ControlAutomationPeer {
    base: AutomationPeer,
    owner: WeakRef<Control>,
    children: RefCell<Option<Rc<Vec<Ref<AutomationPeer>>>>>,
    children_valid: Cell<bool>,
    parent: RefCell<Option<WeakRef<AutomationPeer>>>,
    parent_valid: Cell<bool>,
}

ferro_class! {
    ControlAutomationPeer: AutomationPeer, virtuals ControlAutomationPeerImpl: AutomationPeerImpl {
        /// Gets the child automation peers of the peer, or `None` if it has
        /// none. The default implementation returns the peers of the visible
        /// visual children of the owner.
        fn get_children_core(this) -> Option<Vec<Ref<AutomationPeer>>>;
        /// Gets the visual the peer of which is the parent of this peer
        /// (not for use outside the framework).
        fn get_visual_parent(this) -> Option<Ref<Visual>>;
    }
}
ferroui_base::ferro_class_info!(ControlAutomationPeer {});

impl FerroObjectImpl for ControlAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.initialize();
    }
}

impl ControlAutomationPeerImpl for ControlAutomationPeer {
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let owner = this.owner();
        let children = owner.visual_children_snapshot()?;

        if children.is_empty() {
            return None;
        }

        let mut result = Vec::new();

        for child in children.iter() {
            if let Some(c) = child.cast::<Control>() {
                let peer = this.get_or_create(&c);
                if c.is_visible() {
                    result.push(peer);
                }
            }
        }

        Some(result)
    }

    fn get_visual_parent(this: &Self) -> Option<Ref<Visual>> {
        this.owner().get_visual_parent()
    }
}

impl AutomationPeerImpl for ControlAutomationPeer {
    fn bring_into_view_core(this: &Self) {
        this.owner().bring_into_view();
    }

    fn get_or_create_children_core(this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        let children = this.children.borrow().clone().unwrap_or_default();

        if this.children_valid.get() {
            return children;
        }

        let new_children = this.get_children_core().unwrap_or_default();

        for peer in children.iter().filter(|peer| !new_children.contains(peer)) {
            peer.try_set_parent(None);
        }

        let this_peer: Ref<AutomationPeer> = this.to_ref().upcast();
        for peer in &new_children {
            peer.try_set_parent(Some(this_peer.clone()));
        }

        this.children_valid.set(true);
        let new_children = Rc::new(new_children);
        *this.children.borrow_mut() = Some(new_children.clone());
        new_children
    }

    fn get_labeled_by_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        let label = AutomationProperties::get_labeled_by(&this.owner());
        label.map(|c| this.get_or_create(&c))
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let mut result = AutomationProperties::get_name(&this.owner());

        if is_null_or_white_space(&result) {
            if let Some(labeled_by) = this.get_labeled_by() {
                result = Some(labeled_by.get_name());
            }
        }

        result
    }

    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let mut result = AutomationProperties::get_help_text(&owner);

        if is_null_or_white_space(&result) {
            let errors = DataValidationErrors::get_errors(&owner);
            result = errors.map(|errors| {
                let errors_string_list: Vec<String> =
                    errors.iter().map(|x| ValueTypes::to_display_string(Some(x))).collect();
                errors_string_list.join(NEW_LINE)
            });
        }

        if is_null_or_white_space(&result) {
            result = ToolTip::get_tip(&owner).and_then(|tip| {
                let tip: &dyn AnyValue = &*tip;
                tip.downcast_ref::<String>().cloned()
            });
        }

        // Windows uses HelpText for placeholder text; macOS uses a separate property.
        if is_null_or_white_space(&result) {
            result = this.get_placeholder_text_core();
        }

        result
    }

    fn get_landmark_type_core(this: &Self) -> Option<AutomationLandmarkType> {
        AutomationProperties::get_landmark_type(&this.owner())
    }

    fn get_heading_level_core(this: &Self) -> i32 {
        AutomationProperties::get_heading_level(&this.owner())
    }

    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.ensure_connected();
        this.parent.borrow().as_ref().and_then(|parent| parent.upgrade())
    }

    fn get_visual_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        let owner = this.try_owner()?;
        let focus_root = owner.presentation_source()?.input_root().focus_root();
        let c = focus_root.cast::<Control>()?;
        Some(Self::create_peer_for_element(&c))
    }

    fn to_screen_core(this: &Self, rect: Rect) -> Option<Rect> {
        let root = this.try_owner()?.presentation_source()?.root_visual()?;
        Some(
            PixelRect::from_points(root.point_to_screen(rect.top_left()), root.point_to_screen(rect.bottom_right()))
                .to_rect(1.0),
        )
    }

    fn show_context_menu_core(this: &Self) -> bool {
        let mut c = Some(this.owner());

        while let Some(control) = c {
            if let Some(context_menu) = control.context_menu() {
                context_menu.open_at(Some(&control));
                return true;
            }

            c = control.parent().and_then(|parent| parent.cast::<Control>());
        }

        false
    }

    fn get_live_setting_core(this: &Self) -> AutomationLiveSetting {
        AutomationProperties::get_live_setting(&this.owner())
    }

    fn try_set_parent(this: &Self, parent: Option<Ref<AutomationPeer>>) -> bool {
        *this.parent.borrow_mut() = parent.map(|parent| parent.downgrade());
        true
    }

    fn get_accelerator_key_core(this: &Self) -> Option<String> {
        AutomationProperties::get_accelerator_key(&this.owner())
    }

    fn get_access_key_core(this: &Self) -> Option<String> {
        AutomationProperties::get_access_key(&this.owner())
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }

    fn get_automation_id_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        AutomationProperties::get_automation_id(&owner).or_else(|| owner.name())
    }

    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        Self::get_bounds(&this.owner())
    }

    fn get_class_name_core(this: &Self) -> String {
        this.owner().get_type().name().to_owned()
    }

    fn get_item_status_core(this: &Self) -> Option<String> {
        AutomationProperties::get_item_status(&this.owner())
    }

    fn get_item_type_core(this: &Self) -> Option<String> {
        AutomationProperties::get_item_type(&this.owner())
    }

    fn has_keyboard_focus_core(this: &Self) -> bool {
        this.owner().is_focused()
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }

    fn is_enabled_core(this: &Self) -> bool {
        this.owner().is_effectively_enabled()
    }

    fn is_keyboard_focusable_core(this: &Self) -> bool {
        this.owner().focusable()
    }

    fn set_focus_core(this: &Self) {
        this.owner().focus();
    }

    fn get_control_type_override_core(this: &Self) -> AutomationControlType {
        AutomationProperties::get_control_type_override(&this.owner())
            .unwrap_or_else(|| this.get_automation_control_type_core())
    }

    fn get_class_name_override_core(this: &Self) -> String {
        AutomationProperties::get_class_name_override(&this.owner()).unwrap_or_else(|| this.get_class_name_core())
    }

    fn is_content_element_override_core(this: &Self) -> bool {
        let view = AutomationProperties::get_accessibility_view(&this.owner());
        if view == AccessibilityView::Default {
            this.is_content_element_core()
        } else {
            view as i32 >= AccessibilityView::Content as i32
        }
    }

    fn is_control_element_override_core(this: &Self) -> bool {
        let owner = this.owner();
        if let Some(is_control_element) = AutomationProperties::get_is_control_element_override(&owner) {
            return is_control_element;
        }

        let view = AutomationProperties::get_accessibility_view(&owner);
        if view == AccessibilityView::Default {
            this.is_control_element_core()
        } else {
            view as i32 >= AccessibilityView::Control as i32
        }
    }

    fn is_offscreen_core(this: &Self) -> bool {
        let owner = this.owner();
        match AutomationProperties::get_is_offscreen_behavior(&owner) {
            IsOffscreenBehavior::Onscreen => false,
            IsOffscreenBehavior::Offscreen => true,
            IsOffscreenBehavior::FromClip => match owner.get_transformed_bounds() {
                Some(bounds) => MathUtilities::is_zero(bounds.clip.width) || MathUtilities::is_zero(bounds.clip.height),
                None => true,
            },
            IsOffscreenBehavior::Default => {
                !owner.is_effectively_visible()
                    || !owner.visual_root().map(|root| root.is_effectively_visible()).unwrap_or(false)
            }
        }
    }
}

/// The line separator of the platform.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

fn is_null_or_white_space(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|value| value.chars().all(char::is_whitespace))
}

impl ControlAutomationPeer {
    /// Creates the class data of a peer of `owner`; see
    /// [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Control) -> Self {
        Self {
            base: AutomationPeer::construct(),
            owner: owner.to_ref().downgrade(),
            children: RefCell::new(None),
            children_valid: Cell::new(false),
            parent: RefCell::new(None),
            parent_valid: Cell::new(false),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning control.
    ///
    /// A control owns its peer, so the peer holds the control weakly.
    ///
    /// # Panics
    ///
    /// Panics if the control no longer exists: a peer must not be used
    /// after its control has been released (code that keeps a peer beyond
    /// the life of the tree keeps the control with it, or asks
    /// [`try_owner`](Self::try_owner)).
    pub fn owner(&self) -> Ref<Control> {
        self.owner.upgrade().expect("The control of the automation peer no longer exists.")
    }

    /// Gets the owning control, or `None` if it no longer exists.
    pub fn try_owner(&self) -> Option<Ref<Control>> {
        self.owner.upgrade()
    }

    /// Gets or creates an automation peer for a control: this peer for its
    /// owner, the peer of `element` otherwise.
    pub fn get_or_create(&self, element: &Control) -> Ref<AutomationPeer> {
        if self.owner.upgrade().is_some_and(|owner| std::ptr::eq::<Control>(&*owner, element)) {
            return self.to_ref().upcast();
        }
        Self::create_peer_for_element(element)
    }

    /// Gets an existing automation peer of a control, or creates a new
    /// one if it does not yet exist.
    ///
    /// Despite the name (which comes from the analogous WPF API), this
    /// method does not create a new peer if one already exists: instead it
    /// returns the existing peer.
    pub fn create_peer_for_element(element: &Control) -> Ref<AutomationPeer> {
        element.get_or_create_automation_peer()
    }

    /// Gets an existing automation peer of a control, or `None` if it does
    /// not yet exist.
    pub fn from_element(element: &Control) -> Option<Ref<AutomationPeer>> {
        element.get_automation_peer()
    }

    /// Invalidates the peer's children and causes a re-read from
    /// [`get_children_core`](Self::get_children_core).
    pub fn invalidate_children(&self) {
        self.children_valid.set(false);
        self.raise_children_changed_event();
    }

    /// Invalidates the peer's parent.
    pub fn invalidate_parent(&self) {
        *self.parent.borrow_mut() = None;
        self.parent_valid.set(false);
    }

    fn get_bounds(control: &Control) -> Rect {
        let Some(root_visual) = control.visual_root() else {
            return Rect::default();
        };

        let Some(transform) = control.transform_to_visual(&root_visual) else {
            return Rect::default();
        };

        Rect::from_size(control.bounds().size()).transform_to_aabb(transform)
    }

    fn initialize(&self) {
        let Some(owner) = self.try_owner() else { return };

        // The owner holds the handlers and the peer; the handlers hold the
        // peer weakly.
        let weak = self.to_ref().downgrade();
        owner.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_property_changed(e);
            }
        });

        let weak = self.to_ref().downgrade();
        owner.visual_children().add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.invalidate_children();
            }
        }));
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();

        if property == Visual::is_visible_property().as_property() {
            let parent = self.get_visual_parent();
            if let Some(c) = parent.and_then(|parent| parent.cast::<Control>()) {
                if let Some(peer) = self.get_or_create(&c).cast::<ControlAutomationPeer>() {
                    peer.invalidate_children();
                }
            }
        } else if property == Visual::bounds_property().as_property()
            || property == Visual::render_transform_property().as_property()
            || property == Visual::render_transform_origin_property().as_property()
        {
            let Some(owner) = self.try_owner() else { return };
            self.raise_property_changed_event(
                AutomationElementIdentifiers::bounding_rectangle_property(),
                None,
                Some(Rc::new(Self::get_bounds(&owner)) as BoxedValue),
            );
        } else if property == Visual::visual_parent_property().as_property() {
            self.invalidate_parent();
        } else if property == AutomationProperties::item_status_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<String>>();
            self.raise_property_changed_event(
                AutomationElementIdentifiers::item_status_property(),
                old_value.map(|v| Rc::new(v) as BoxedValue),
                new_value.map(|v| Rc::new(v) as BoxedValue),
            );
        } else if property == AutomationProperties::automation_id_property().as_property() {
            self.raise_property_changed_event(
                AutomationElementIdentifiers::automation_id_property(),
                None,
                self.get_automation_id().map(|v| Rc::new(v) as BoxedValue),
            );
        }
    }

    fn ensure_connected(&self) {
        if !self.parent_valid.get() {
            let mut parent = self.get_visual_parent();

            while let Some(p) = parent {
                if let Some(c) = p.cast::<Control>() {
                    let parent_peer = self.get_or_create(&c);
                    parent_peer.get_children();

                    if let Some(control_peer) = parent_peer.cast::<ControlAutomationPeer>() {
                        parent = control_peer.get_visual_parent();
                        continue;
                    }
                }

                parent = p.get_visual_parent();
            }

            self.parent_valid.set(true);
        }
    }
}
