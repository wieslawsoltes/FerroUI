//! WinUI's `Pivot` (`dxaml/phone/lib/Pivot_Partial.cpp`, `Pivot_Partial.h` and the metadata of
//! `dxaml/xcp/tools/XCPTypesAutoGen/Modules/Phone/Phone.cs` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md): the control, its selection, keyboard and lock handling and the callbacks of its
//! state machine (docs/porting/winui-calculator-gaps.md, section 7.1, PV-1), the headers, the
//! navigation buttons and the animations (`PivotAnimator`, `Pivot_SlideInEffect_Partial.cpp`, PV-2).

use super::pivot_common::{positive_mod, PivotAnimationDirection, PIVOT_PANEL_MULTIPLIER};
use super::pivot_header_manager::PivotHeaderManager;
use super::pivot_panel::{ITEMS_PRESENTER_NAME, LAYOUT_ELEMENT_NAME};
use super::pivot_state_machine::{PivotCollectionChange, PivotStateMachine, PivotStateMachineCallbacks};
use super::{PivotHeaderItem, PivotHeaderPanel, PivotItem, PivotItemEventArgs, PivotPanel};
use crate::generators::RecycleKey;
use crate::hosting::ElementCompositionPreview;
use crate::presenters::ItemsPresenter;
use crate::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, SnapPointsType, TemplateAppliedEventArgs, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{FuncTemplate, IDataTemplate, ITemplateOf};
use crate::{
    Button, ContentControl, Control, ControlImpl, Grid, ItemsChangedEventArgs, ItemsControl, ItemsControlImpl,
    ItemsControlImplExt, Panel, ScrollViewer, SelectionMode, TopLevel, VisualStateManager,
};
use ferroui_base::animation::easings::IEasing;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::gesture_recognizers::ScrollGestureRecognizer;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, PointerEventArgs,
    PointerType, ScrollGestureEndedEventArgs, ScrollGestureEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::numerics::Vector2;
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::rendering::composition::animations::{CompositionEasingFunctionMode, ExponentialEasingFunction};
use ferroui_base::rendering::composition::{
    AsCompositionObject, CompositionBatchTypes, CompositionPropertySet, CompositionVisual, Compositor,
    ElementComposition, ICompositionObjectAnimations,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_property, instantiate, AttachedProperty, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl,
    StyledProperty, Vector3D, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Where the focus visual of the headers is drawn (WinUI's `PivotHeaderFocusVisualPlacement`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PivotHeaderFocusVisualPlacement {
    /// Around all the headers (WinUI's default).
    #[default]
    ItemHeaders = 0,
    /// Around the header of the selected item.
    SelectedItemHeader = 1,
}

ferro_markup_enum!(
    PivotHeaderFocusVisualPlacement { ItemHeaders, SelectedItemHeader },
    { namespace: "FerroUI.Controls" }
);

/// The group of the slide-in animation of an element of a pivot item (WinUI's
/// `PivotSlideInAnimationGroup`): the groups slide in one after the other when an item is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PivotSlideInAnimationGroup {
    #[default]
    Default = 0,
    GroupOne = 1,
    GroupTwo = 2,
    GroupThree = 3,
}

ferro_markup_enum!(
    PivotSlideInAnimationGroup { Default, GroupOne, GroupTwo, GroupThree },
    { namespace: "FerroUI.Controls" }
);

type PivotItemHandler = dyn Fn(&Ref<Pivot>, &PivotItemEventArgs);

/// The template parts.
#[derive(Default)]
struct Parts {
    items_presenter: Option<Ref<ItemsPresenter>>,
    title_control: Option<Ref<ContentControl>>,
    scroll_viewer: Option<Ref<ScrollViewer>>,
    panel: Option<Ref<PivotPanel>>,
    layout_element: Option<Ref<Control>>,
    header_clipper: Option<Ref<ContentControl>>,
    left_header_presenter: Option<Ref<Control>>,
    right_header_presenter: Option<Ref<Control>>,
    header_panel: Option<Ref<PivotHeaderPanel>>,
    static_header_panel: Option<Ref<PivotHeaderPanel>>,
    previous_button: Option<Ref<Button>>,
    next_button: Option<Ref<Button>>,
    focus_follower: Option<Ref<Control>>,
}

/// The run-time state of the pivot (the members of WinUI's `Pivot` outside the machine).
struct Runtime {
    index_change_guard: bool,
    previous_selected_index: i32,
    current_pivot_item: Option<WeakRef<PivotItem>>,
    /// Between the first scroll gesture of a manipulation and its end.
    in_manipulation: bool,
    /// WinUI's `m_isDirectManipulationInProgress`: the neighbours are shown while it is set.
    is_direct_manipulation_in_progress: bool,
    pivot_drag_direction: PivotAnimationDirection,
    keyboard_focus_to_next_pivot_item_pending: bool,
    /// The section offset of the items (WinUI's `ItemsPresenterTranslateTransform.X`).
    section_offset: f64,
    viewport_enabled: bool,
    using_static_headers: bool,
    is_pointer_over_headers: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            index_change_guard: false,
            previous_selected_index: -1,
            current_pivot_item: None,
            in_manipulation: false,
            is_direct_manipulation_in_progress: false,
            pivot_drag_direction: PivotAnimationDirection::Center,
            keyboard_focus_to_next_pivot_item_pending: false,
            section_offset: 0.0,
            viewport_enabled: true,
            using_static_headers: false,
            is_pointer_over_headers: false,
        }
    }
}

/// The composition side of the layout: a property set with the offsets the expressions of the
/// layout element and of the items presenter read.
struct Composition {
    properties: Rc<CompositionPropertySet>,
    layout_visual: Option<Rc<CompositionVisual>>,
    items_visual: Option<Rc<CompositionVisual>>,
    /// The parent of the header panels (WinUI's `HeaderOffsetTranslateTransform`).
    header_offset_visual: Option<Rc<CompositionVisual>>,
}

/// A control that shows one of its items at a time, under a row of headers that the user taps,
/// pans or flicks through (WinUI's `Pivot`).
///
/// The items are [`PivotItem`]s (other items get one as their container). The pivot keeps one
/// item selected while it has items ([`SelectionMode::ALWAYS_SELECTED`]); setting
/// [`selected_index`](SelectingItemsControl::selected_index) flies the current item out and the
/// new one in, a pan of the content moves to the neighbour past a quarter of the width. With
/// [`is_header_items_carousel_enabled`](Self::is_header_items_carousel_enabled) (the default) the
/// headers and the items wrap around. [`is_locked`](Self::is_locked) keeps the selected item.
///
/// The structure is WinUI's (decision D8): the items are in a `ScrollViewer` (template part
/// `ScrollViewer`) whose content is a [`PivotPanel`] (`Panel`) a thousand sections wide, kept
/// near its middle by the pivot's state machine; the panel holds the `PivotLayoutElement` with the
/// headers and the `PivotItemPresenter`. During a pan the layout element stays in the viewport and
/// the items follow the content through expression animations of their `Translation`, on the
/// compositor (WinUI's secondary content relationships).
#[repr(C)]
pub struct Pivot {
    base: SelectingItemsControl,
    machine: RefCell<PivotStateMachine>,
    parts: RefCell<Parts>,
    runtime: RefCell<Runtime>,
    composition: RefCell<Option<Composition>>,
    headers: RefCell<PivotHeaderManager>,
    /// The completion of the running fly animation.
    fly_batch: RefCell<Option<Subscription>>,
    template_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    animation_generation: Cell<u64>,
    pivot_item_loading: HandlerList<PivotItemHandler>,
    pivot_item_loaded: HandlerList<PivotItemHandler>,
    pivot_item_unloading: HandlerList<PivotItemHandler>,
    pivot_item_unloaded: HandlerList<PivotItemHandler>,
}

ferro_class!(Pivot: SelectingItemsControl);
ferroui_base::ferro_class_info!(Pivot {
    new: Pivot::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [
            TemplatePart("ScrollViewer", type(Ref<ScrollViewer>)),
            TemplatePart("Panel", type(Ref<PivotPanel>)),
            TemplatePart("PivotLayoutElement", type(Ref<Grid>)),
            TemplatePart("PivotItemPresenter", type(Ref<ItemsPresenter>)),
            TemplatePart("TitleContentControl", type(Ref<ContentControl>)),
            TemplatePart("HeaderClipper", type(Ref<ContentControl>)),
            TemplatePart("Header", type(Ref<PivotHeaderPanel>)),
            TemplatePart("StaticHeader", type(Ref<PivotHeaderPanel>)),
            TemplatePart("PreviousButton", type(Ref<Button>)),
            TemplatePart("NextButton", type(Ref<Button>)),
        ],
    },
});
ferro_impl_classes!(Pivot: StyledElementImpl, InteractiveImpl);

ferroui_base::ferro_properties! { impl Pivot {
    ferro_property!(
        /// Defines the `Title` property: the content shown above the headers.
        pub fn title_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Pivot, _>("Title", None)
        }
    );

    ferro_property!(
        /// Defines the `TitleTemplate` property.
        pub fn title_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Pivot, _>("TitleTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `LeftHeader` property: the content left of the headers.
        pub fn left_header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Pivot, _>("LeftHeader", None)
        }
    );

    ferro_property!(
        /// Defines the `LeftHeaderTemplate` property.
        pub fn left_header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Pivot, _>("LeftHeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `RightHeader` property: the content right of the headers.
        pub fn right_header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Pivot, _>("RightHeader", None)
        }
    );

    ferro_property!(
        /// Defines the `RightHeaderTemplate` property.
        pub fn right_header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Pivot, _>("RightHeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `HeaderTemplate` property: the template of the headers of the items.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<Pivot, _>("HeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `IsLocked` property: whether the selected item is kept (no pan, no header
        /// taps, the other headers hidden).
        pub fn is_locked_property() -> StyledProperty<bool> {
            FerroProperty::register::<Pivot, _>("IsLocked", false)
        }
    );

    ferro_property!(
        /// Defines the `IsHeaderItemsCarouselEnabled` property: whether the headers and the items
        /// wrap around (WinUI's default `true`).
        pub fn is_header_items_carousel_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Pivot, _>("IsHeaderItemsCarouselEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `HeaderFocusVisualPlacement` property.
        pub fn header_focus_visual_placement_property() -> StyledProperty<PivotHeaderFocusVisualPlacement> {
            FerroProperty::register::<Pivot, _>(
                "HeaderFocusVisualPlacement",
                PivotHeaderFocusVisualPlacement::ItemHeaders,
            )
        }
    );

    ferro_property!(
        /// Defines the attached `SlideInAnimationGroup` property of the elements of a pivot item.
        pub fn slide_in_animation_group_property() -> AttachedProperty<PivotSlideInAnimationGroup> {
            FerroProperty::register_attached::<Pivot, Control, _>(
                "SlideInAnimationGroup",
                PivotSlideInAnimationGroup::Default,
            )
        }
    );
} }

impl FerroObjectImpl for Pivot {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.ensure_gesture_handlers();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == SelectingItemsControl::selected_index_property().as_property() {
            let old = change.get_old_value::<i32>().unwrap_or(-1);
            let new = change.get_new_value::<i32>();
            this.on_selected_index_changed(old, new);
        } else if property == Self::is_locked_property().as_property() {
            let locked = change.get_new_value::<bool>();
            this.headers.borrow_mut().is_locked_changed_event(locked);
            this.with_machine(move |machine, cb| machine.is_locked_changed_event(cb, locked));
            this.update_visual_states();
        } else if property == Self::title_property().as_property()
            || property == Self::title_template_property().as_property()
        {
            this.update_title_control_visibility();
        } else if property == Self::header_template_property().as_property() {
            this.headers.borrow().header_template_changed_event(this.header_template());
        } else if property == Self::header_focus_visual_placement_property().as_property() {
            this.update_header_manager_focus_state();
        } else if property == Self::is_header_items_carousel_enabled_property().as_property() {
            this.with_machine(|machine, cb| machine.is_header_items_carousel_enabled_changed_event(cb));
            if let Some(panel) = this.parts.borrow().panel.clone() {
                panel.invalidate_measure();
            }
        }
    }
}

impl VisualImpl for Pivot {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.invalidate_measure();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        // WinUI's `OnUnloaded`: a running animation completes, the composition is set up again on
        // the next attach.
        this.complete_animations();
        *this.composition.borrow_mut() = None;
        this.headers.borrow_mut().unloaded_event();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl LayoutableImpl for Pivot {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut available = available_size;
        if available.width.is_infinite() {
            // WinUI measures with the width of the screen; here the client width of the window.
            let visual: &Visual = this;
            available.width =
                TopLevel::get_top_level(Some(visual)).map_or(0.0, |top_level| top_level.client_size().width);
        }
        this.with_machine(move |machine, cb| machine.measure_event(cb, available));
        Self::parent_measure_override(this, available)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut size = final_size;
        if size.width.is_infinite() {
            let visual: &Visual = this;
            size.width = TopLevel::get_top_level(Some(visual)).map_or(0.0, |top_level| top_level.client_size().width);
        }
        let result = Self::parent_arrange_override(this, size);
        this.with_machine(move |machine, cb| machine.arrange_event(cb, size));
        this.ensure_composition();
        this.update_header_state();
        result
    }
}

impl TemplatedControlImpl for Pivot {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        for subscription in this.template_subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(panel) = this.parts.borrow().panel.clone() {
            panel.set_parent_pivot(None);
        }
        *this.composition.borrow_mut() = None;

        Self::parent_on_apply_template(this, e);

        let scope = e.name_scope();
        {
            let mut parts = this.parts.borrow_mut();
            parts.items_presenter = scope.find_as::<ItemsPresenter>(ITEMS_PRESENTER_NAME);
            parts.title_control = scope.find_as::<ContentControl>("TitleContentControl");
            parts.scroll_viewer = scope.find_as::<ScrollViewer>("ScrollViewer");
            parts.panel = scope.find_as::<PivotPanel>("Panel");
            parts.layout_element = scope.find_as::<Control>(LAYOUT_ELEMENT_NAME);
            parts.header_clipper = scope.find_as::<ContentControl>("HeaderClipper");
            parts.left_header_presenter = scope.find_as::<Control>("LeftHeaderPresenter");
            parts.right_header_presenter = scope.find_as::<Control>("RightHeaderPresenter");
            parts.header_panel = scope.find_as::<PivotHeaderPanel>("Header");
            parts.static_header_panel = scope.find_as::<PivotHeaderPanel>("StaticHeader");
            parts.previous_button = scope.find_as::<Button>("PreviousButton");
            parts.next_button = scope.find_as::<Button>("NextButton");
            parts.focus_follower = scope.find_as::<Control>("FocusFollower");
        }

        if let Some(scroll_viewer) = this.parts.borrow().scroll_viewer.clone() {
            // The view changes of a manipulation go to the machine (WinUI's `ViewChanging`).
            let weak = this.to_ref().downgrade();
            let subscription = scroll_viewer.property_changed(move |e| {
                if e.property() == ScrollViewer::offset_property().as_property() {
                    if let Some(pivot) = weak.upgrade() {
                        pivot.on_view_changing();
                    }
                }
            });
            this.template_subscriptions.borrow_mut().push(subscription);
            // WinUI ignores the wheel and the bring into view requests of its scroll viewer.
            ScrollViewer::set_bring_into_view_on_focus_change_on(&scroll_viewer, false);
        }

        if let Some(panel) = this.parts.borrow().panel.clone() {
            panel.set_parent_pivot(Some(this));
        }

        this.attach_header_handlers();
        this.validate_item_index();
        this.update_title_control_visibility();
        let valid = this.has_valid_template();
        this.with_machine(move |machine, cb| machine.apply_template_event(cb, valid));
        let (header_panel, static_header_panel) = {
            let parts = this.parts.borrow();
            (parts.header_panel.clone(), parts.static_header_panel.clone())
        };
        this.headers.borrow_mut().apply_template_event(this, header_panel, static_header_panel);
        this.invalidate_measure();
    }
}

impl InputElementImpl for Pivot {
    /// WinUI's `OnKeyDown`: Ctrl+PageDown and Ctrl+Tab move to the next item, Ctrl+PageUp and
    /// Ctrl+Shift+Tab to the previous one, wrapping while the headers wrap.
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() || !this.is_effectively_enabled() {
            return;
        }
        let control = e.key_modifiers.contains(KeyModifiers::CONTROL);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);
        let wrap = this.should_wrap();
        let changing = match e.key {
            Key::PageDown if control => {
                this.move_to_next_item(wrap);
                true
            }
            Key::PageUp if control => {
                this.move_to_previous_item(wrap);
                true
            }
            Key::Tab if control => {
                if shift {
                    this.move_to_previous_item(wrap);
                } else {
                    this.move_to_next_item(wrap);
                }
                true
            }
            _ => false,
        };
        if changing {
            e.set_handled(true);
            let current = this.runtime.borrow().current_pivot_item.as_ref().and_then(WeakRef::upgrade);
            if let Some(current) = current {
                if current.is_keyboard_focus_within() {
                    this.runtime.borrow_mut().keyboard_focus_to_next_pivot_item_pending = true;
                }
            }
        }
    }
}

impl ControlImpl for Pivot {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::PivotAutomationPeer::new(this).upcast()
    }
}

impl ItemsControlImpl for Pivot {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        PivotItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<PivotItem>(item)
    }

    /// WinUI's `PrepareContainerForItemOverride`: the item starts collapsed and transparent.
    fn prepare_container_for_item_override(this: &Self, element: &Ref<Control>, item: &Option<BoxedValue>, index: i32) {
        Self::parent_prepare_container_for_item_override(this, element, item, index);
        if let Some(pivot_item) = element.downcast_ref::<PivotItem>() {
            pivot_item.set_parent_pivot(Some(this));
            pivot_item.set_content_visibility(false);
            pivot_item.set_opacity(0.0);
            pivot_item.set_is_hit_test_visible(false);
        }
    }

    fn container_for_item_prepared_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_container_for_item_prepared_override(this, container, item, index);
        // The selected item gets its container after the machine asked for it: show it now.
        if index == this.selected_index()
            && this.machine.borrow().state() >= super::pivot_state_machine::PivotState::Idle
        {
            this.update_visible_content(index, PivotAnimationDirection::Reset);
        }
    }

    /// WinUI's `OnItemsChanged`.
    fn on_items_view_collection_changed(this: &Self, e: &ItemsChangedEventArgs<'_>) {
        // The selection of the base follows the change; the machine decides the index.
        let guard = std::mem::replace(&mut this.runtime.borrow_mut().index_change_guard, true);
        Self::parent_on_items_view_collection_changed(this, e);
        this.runtime.borrow_mut().index_change_guard = guard;

        let (change, index) = match e.action {
            NotifyCollectionChangedAction::Add => (PivotCollectionChange::ItemInserted, e.new_starting_index),
            NotifyCollectionChangedAction::Remove => (PivotCollectionChange::ItemRemoved, e.old_starting_index),
            NotifyCollectionChangedAction::Replace => (PivotCollectionChange::ItemChanged, e.new_starting_index),
            _ => (PivotCollectionChange::Reset, 0),
        };
        let count = this.item_count();
        // Several items added or removed at once are told one by one, as WinUI's vector does.
        let changed = match e.action {
            NotifyCollectionChangedAction::Add => e.new_items.len() as i32,
            NotifyCollectionChangedAction::Remove => e.old_items.len() as i32,
            _ => 1,
        }
        .max(1);
        let base_count = match change {
            PivotCollectionChange::ItemInserted => count - changed,
            PivotCollectionChange::ItemRemoved => count + changed,
            _ => count,
        };
        for step in 0..changed {
            let (item_count, at) = match change {
                PivotCollectionChange::ItemInserted => (base_count + step + 1, index + step),
                PivotCollectionChange::ItemRemoved => (base_count - step - 1, index),
                _ => (count, index),
            };
            this.headers.borrow_mut().items_collection_changed_event(this, item_count, at, change);
            this.with_machine(move |machine, cb| machine.items_collection_changed_event(cb, item_count, at, change));
        }
        this.invalidate_arrange();
        this.update_visual_states();
        if !this.is_header_items_carousel_enabled() {
            if let Some(panel) = this.parts.borrow().panel.clone() {
                panel.invalidate_measure();
            }
        }
    }
}

impl SelectingItemsControlImpl for Pivot {
    /// The items select through the pivot only (taps on the headers, pans, the keyboard).
    fn update_selection_from_event(
        _this: &Self,
        _container: &Ref<Control>,
        _event_args: &dyn ferroui_base::interactivity::IRoutedEventArgs,
    ) -> bool {
        false
    }
}

impl Pivot {
    fn static_constructor() {
        // WinUI's default items panel is a grid: the items lie on each other.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(Grid::new().upcast::<Panel>()));
        ItemsControl::items_panel_property().override_default_value::<Pivot>(default_panel);
        SelectingItemsControl::selection_mode_property()
            .override_default_value::<Pivot>(SelectionMode::ALWAYS_SELECTED);
        InputElement::focusable_property().override_default_value::<Pivot>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        let mut machine = PivotStateMachine::new();
        machine.initialize(0, 0, false);
        Self {
            base: SelectingItemsControl::construct(),
            machine: RefCell::new(machine),
            parts: RefCell::new(Parts::default()),
            runtime: RefCell::new(Runtime::default()),
            composition: RefCell::new(None),
            headers: RefCell::new(PivotHeaderManager::default()),
            fly_batch: RefCell::new(None),
            template_subscriptions: RefCell::new(Vec::new()),
            animation_generation: Cell::new(0),
            pivot_item_loading: HandlerList::new(),
            pivot_item_loaded: HandlerList::new(),
            pivot_item_unloading: HandlerList::new(),
            pivot_item_unloaded: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn title(&self) -> Option<BoxedValue> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<BoxedValue>) {
        self.set_value(Self::title_property(), value)
    }

    pub fn title_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::title_template_property())
    }

    pub fn set_title_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::title_template_property(), value)
    }

    pub fn left_header(&self) -> Option<BoxedValue> {
        self.get_value(Self::left_header_property())
    }

    pub fn set_left_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::left_header_property(), value)
    }

    pub fn left_header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::left_header_template_property())
    }

    pub fn set_left_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::left_header_template_property(), value)
    }

    pub fn right_header(&self) -> Option<BoxedValue> {
        self.get_value(Self::right_header_property())
    }

    pub fn set_right_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::right_header_property(), value)
    }

    pub fn right_header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::right_header_template_property())
    }

    pub fn set_right_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::right_header_template_property(), value)
    }

    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    pub fn is_locked(&self) -> bool {
        self.get_value(Self::is_locked_property())
    }

    pub fn set_is_locked(&self, value: bool) {
        self.set_value(Self::is_locked_property(), value)
    }

    pub fn is_header_items_carousel_enabled(&self) -> bool {
        self.get_value(Self::is_header_items_carousel_enabled_property())
    }

    pub fn set_is_header_items_carousel_enabled(&self, value: bool) {
        self.set_value(Self::is_header_items_carousel_enabled_property(), value)
    }

    pub fn header_focus_visual_placement(&self) -> PivotHeaderFocusVisualPlacement {
        self.get_value(Self::header_focus_visual_placement_property())
    }

    pub fn set_header_focus_visual_placement(&self, value: PivotHeaderFocusVisualPlacement) {
        self.set_value(Self::header_focus_visual_placement_property(), value)
    }

    pub fn get_slide_in_animation_group(element: &Control) -> PivotSlideInAnimationGroup {
        element.get_value(Self::slide_in_animation_group_property())
    }

    pub fn set_slide_in_animation_group(element: &Control, value: PivotSlideInAnimationGroup) {
        element.set_value(Self::slide_in_animation_group_property(), value)
    }

    /// Adds a handler of `PivotItemLoading`, raised before an item is shown. Dropping the guard
    /// removes it.
    pub fn pivot_item_loading(&self, handler: impl Fn(&Ref<Pivot>, &PivotItemEventArgs) + 'static) -> Subscription {
        Self::subscribe(self, |pivot| &pivot.pivot_item_loading, Rc::new(handler))
    }

    /// Adds a handler of `PivotItemLoaded`, raised when an item is shown.
    pub fn pivot_item_loaded(&self, handler: impl Fn(&Ref<Pivot>, &PivotItemEventArgs) + 'static) -> Subscription {
        Self::subscribe(self, |pivot| &pivot.pivot_item_loaded, Rc::new(handler))
    }

    /// Adds a handler of `PivotItemUnloading`, raised before an item is hidden.
    pub fn pivot_item_unloading(&self, handler: impl Fn(&Ref<Pivot>, &PivotItemEventArgs) + 'static) -> Subscription {
        Self::subscribe(self, |pivot| &pivot.pivot_item_unloading, Rc::new(handler))
    }

    /// Adds a handler of `PivotItemUnloaded`, raised when an item is hidden.
    pub fn pivot_item_unloaded(&self, handler: impl Fn(&Ref<Pivot>, &PivotItemEventArgs) + 'static) -> Subscription {
        Self::subscribe(self, |pivot| &pivot.pivot_item_unloaded, Rc::new(handler))
    }

    fn subscribe(
        &self,
        list: fn(&Pivot) -> &HandlerList<PivotItemHandler>,
        handler: Rc<PivotItemHandler>,
    ) -> Subscription {
        let token = list(self).add(handler);
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                list(&this).remove(token);
            }
        }))
    }

    fn raise(&self, list: &HandlerList<PivotItemHandler>, item: &Ref<PivotItem>) {
        let this = self.to_ref();
        let args = PivotItemEventArgs::new(item.clone());
        for (_, handler) in list.snapshot().iter() {
            handler(&this, &args);
        }
    }

    /// WinUI's `RaiseOnPivotItemLoaded`: `PivotItemLoading`, then `PivotItemLoaded`.
    fn raise_pivot_item_loaded(&self, item: &Ref<PivotItem>) {
        self.raise(&self.pivot_item_loading, item);
        self.raise(&self.pivot_item_loaded, item);
    }

    /// WinUI's `RaiseOnPivotItemUnloaded`: `PivotItemUnloading`, then `PivotItemUnloaded`.
    fn raise_pivot_item_unloaded(&self, item: &Ref<PivotItem>) {
        self.raise(&self.pivot_item_unloading, item);
        self.raise(&self.pivot_item_unloaded, item);
    }

    /// The panel of the template, for the tests.
    #[cfg(test)]
    pub(crate) fn panel_part(&self) -> Option<Ref<PivotPanel>> {
        self.parts.borrow().panel.clone()
    }

    /// The headers, for the tests.
    #[cfg(test)]
    pub(crate) fn header_items(&self) -> Vec<Ref<PivotHeaderItem>> {
        self.headers.borrow().header_items().to_vec()
    }

    /// Whether the static headers are in use, for the tests.
    #[cfg(test)]
    pub(crate) fn uses_static_headers(&self) -> bool {
        self.runtime.borrow().using_static_headers
    }

    #[cfg(test)]
    pub(crate) fn should_wrap_for_tests(&self) -> bool {
        self.should_wrap()
    }

    #[cfg(test)]
    pub(crate) fn on_header_key_down_for_tests(&self, e: &KeyEventArgs) {
        self.on_header_key_down(e);
    }

    /// The state of the machine, for the tests.
    #[cfg(test)]
    pub(crate) fn machine_state(&self) -> super::pivot_state_machine::PivotState {
        self.machine.borrow().state()
    }

    /// Runs `f` on the machine with the pivot as its callbacks. A call from within a callback
    /// (the application changes the selection in a handler the machine raised) runs after it.
    fn with_machine(&self, f: impl FnOnce(&mut PivotStateMachine, &dyn PivotStateMachineCallbacks) + 'static) {
        match self.machine.try_borrow_mut() {
            Ok(mut machine) => {
                let (widths, _) = self.headers.borrow().header_widths();
                machine.header_widths_changed_event(widths);
                f(&mut machine, self);
                drop(machine);
                self.sync_layout_offset();
            }
            Err(_) => {
                let weak = self.to_ref().downgrade();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.with_machine(f);
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
        }
    }

    /// The layout element is arranged at the section the viewport rests on.
    fn sync_layout_offset(&self) {
        let offset = self.machine.borrow().static_viewport_offset();
        if let Some(panel) = self.parts.borrow().panel.clone() {
            panel.set_section_offset(offset);
        }
    }

    fn has_valid_template(&self) -> bool {
        let parts = self.parts.borrow();
        parts.items_presenter.is_some()
            && parts.scroll_viewer.is_some()
            && parts.panel.is_some()
            && parts.layout_element.is_some()
    }

    /// WinUI's `GetPivotPanelMultiplierImpl`: one section per item without the carousel.
    pub(crate) fn pivot_panel_multiplier_impl(&self) -> u32 {
        if self.is_header_items_carousel_enabled() {
            PIVOT_PANEL_MULTIPLIER
        } else {
            self.item_count().max(1) as u32
        }
    }

    /// WinUI's `ShouldWrap`: the headers wrap unless they are static (PV-2).
    fn should_wrap(&self) -> bool {
        !self.runtime.borrow().using_static_headers
    }

    /// WinUI's `ValidateItemIndex`: the selection is coerced into the items.
    fn validate_item_index(&self) {
        let count = self.item_count();
        let index = self.selected_index();
        if count > 0 && !(0..count).contains(&index) {
            self.set_selected_index(0);
        }
    }

    fn update_title_control_visibility(&self) {
        let title_control = self.parts.borrow().title_control.clone();
        if let Some(title_control) = title_control {
            title_control.set_is_visible(self.title().is_some() || self.title_template().is_some());
        }
    }

    /// WinUI's `OnSelectedIndexChanged`.
    fn on_selected_index_changed(&self, old: i32, new: i32) {
        if !self.runtime.borrow().index_change_guard {
            self.runtime.borrow_mut().index_change_guard = true;
            let count = self.item_count();
            if count == 0 || (0..count).contains(&new) {
                let skip_animation = !self.is_attached_to_visual_tree();
                self.with_machine(move |machine, cb| {
                    machine.selected_index_changed_event(cb, new, false, skip_animation)
                });
            } else {
                // WinUI rejects an index out of the items.
                self.set_selected_index(old);
            }
            self.runtime.borrow_mut().index_change_guard = false;
        }
        self.runtime.borrow_mut().previous_selected_index = old;
    }

    /// Moves to the item before the selected one (WinUI's `MoveToPreviousItem`); whether the
    /// selection moved.
    pub fn move_to_previous_item(&self, should_wrap: bool) -> bool {
        let count = self.item_count();
        if self.is_locked() || count == 0 {
            return false;
        }
        let old = self.selected_index();
        if old > 0 || should_wrap {
            let new = if old == 0 { count - 1 } else { old - 1 };
            return self.move_to_item(new);
        }
        false
    }

    /// Moves to the item after the selected one (WinUI's `MoveToNextItem`).
    pub fn move_to_next_item(&self, should_wrap: bool) -> bool {
        let count = self.item_count();
        if self.is_locked() || count == 0 {
            return false;
        }
        let old = self.selected_index();
        if old < count - 1 || should_wrap {
            let new = if old == count - 1 { 0 } else { old + 1 };
            return self.move_to_item(new);
        }
        false
    }

    /// WinUI's `MoveToItem`: through the machine, which animates the move.
    fn move_to_item(&self, index: i32) -> bool {
        if !(0..self.item_count()).contains(&index) {
            return false;
        }
        self.with_machine(move |machine, cb| machine.selected_index_changed_event(cb, index, false, false));
        true
    }

    /// A header was tapped (WinUI's `OnHeaderItemTapped`).
    pub(crate) fn on_header_item_tapped(&self, item: &Ref<PivotHeaderItem>) {
        let index = self.headers.borrow().index_of(item);
        if let Some(index) = index {
            self.on_header_item_tapped_at(index as i32);
        }
    }

    /// The header of `index` was tapped.
    pub(crate) fn on_header_item_tapped_at(&self, index: i32) {
        if !self.is_locked() {
            self.with_machine(move |machine, cb| machine.selected_index_changed_event(cb, index, true, false));
        }
    }

    /// The scroll viewer moved (WinUI's `OnViewChanging`): while the user pans, the view goes to
    /// the machine; programmatic moves are the machine's own.
    fn on_view_changing(&self) {
        if !self.runtime.borrow().in_manipulation {
            return;
        }
        let offset = self.scroll_offset();
        let section_offset = self.runtime.borrow().section_offset;
        self.with_machine(move |machine, cb| machine.view_changed_event(cb, false, offset, offset, section_offset));
    }

    fn scroll_offset(&self) -> f64 {
        self.parts.borrow().scroll_viewer.as_ref().map_or(0.0, |scroll_viewer| scroll_viewer.offset().x)
    }

    /// The first scroll gesture of a manipulation (WinUI's `DirectManipulationStarted`).
    fn on_scroll_gesture(&self, _e: &ScrollGestureEventArgs) {
        if self.runtime.borrow().in_manipulation {
            return;
        }
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime.in_manipulation = true;
            runtime.is_direct_manipulation_in_progress = true;
        }
        let direction = self.runtime.borrow().pivot_drag_direction;
        self.update_visible_content(self.selected_index(), direction);
    }

    /// The manipulation and its inertia ended, the scroll viewer snapped to a section: the view
    /// is final (WinUI's last `ViewChanging` of the inertia, then `ViewChanged` and
    /// `DirectManipulationCompleted`).
    fn on_scroll_gesture_ended(&self, _e: &ScrollGestureEndedEventArgs) {
        if !self.runtime.borrow().in_manipulation {
            return;
        }
        self.runtime.borrow_mut().in_manipulation = false;
        let offset = self.scroll_offset();
        let section_offset = self.runtime.borrow().section_offset;
        self.with_machine(move |machine, cb| {
            machine.view_changed_event(cb, true, offset, offset, section_offset);
            machine.final_view_event(cb);
        });
        self.runtime.borrow_mut().is_direct_manipulation_in_progress = false;
        let direction = self.runtime.borrow().pivot_drag_direction;
        self.update_visible_content(self.selected_index(), direction);
    }

    /// WinUI's `UpdateVisibleContent`: the selected item is shown; while a manipulation runs its
    /// neighbours too, one section to the left and to the right (their `Translation`), so that
    /// the items presenter works as one content presenter with some content revealed on the sides.
    fn update_visible_content(&self, index: i32, animation_hint: PivotAnimationDirection) {
        if self.parts.borrow().items_presenter.is_none() && self.items_panel_root().is_none() {
            return;
        }
        let count = self.item_count();
        let current = self.runtime.borrow().current_pivot_item.as_ref().and_then(WeakRef::upgrade);
        if let Some(current) = current {
            let new_container = self.container_from_index(index).and_then(|container| container.cast::<PivotItem>());
            if new_container.as_ref().is_none_or(|new| !Ref::ptr_eq(new, &current)) {
                self.raise_pivot_item_unloaded(&current);
                self.runtime.borrow_mut().current_pivot_item = None;
            }
        }

        let manipulating = self.runtime.borrow().is_direct_manipulation_in_progress;
        let width = self.parts.borrow().items_presenter.as_ref().map_or(0.0, |presenter| presenter.bounds().width);
        for i in 0..count {
            let Some(container) = self.container_from_index(i).and_then(|container| container.cast::<PivotItem>())
            else {
                continue;
            };
            let is_current = i == index;
            let is_neighbor = i == positive_mod(index - 1, count) || i == positive_mod(index + 1, count);
            // The neighbours are collapsed again after the manipulation, or the narrator would
            // read the items the user does not see.
            let to_visible = is_current || (is_neighbor && manipulating);
            self.update_item_visibility(&container, to_visible);
            if to_visible {
                let x_offset = if count == 1 || is_current {
                    0.0
                } else if count == 2 {
                    if animation_hint == PivotAnimationDirection::Left {
                        -width
                    } else {
                        width
                    }
                } else if i == positive_mod(index + 1, count) {
                    width
                } else {
                    -width
                };
                container.set_translation(Vector3D::new(x_offset, 0.0, 0.0));
            }
            if is_current && animation_hint != PivotAnimationDirection::Reset {
                apply_slide_in_animation(&container, animation_hint);
            }
        }
    }

    /// WinUI's `UpdateItemVisibility`.
    fn update_item_visibility(&self, container: &Ref<PivotItem>, to_visible: bool) {
        container.set_is_hit_test_visible(to_visible);
        let current_opacity = container.opacity();
        container.set_opacity(if to_visible { 1.0 } else { 0.0 });
        let content_visible = container.content_visibility();
        let mut first_expand = false;
        if to_visible && !content_visible {
            container.set_content_visibility(true);
            // The content is not loaded yet: the focus waits for its measure.
            first_expand = true;
        } else if !to_visible && content_visible {
            // Collapsed, so that tabbing skips the item.
            container.set_content_visibility(false);
        }
        if to_visible && current_opacity == 0.0 {
            // The item templates are realized before the loaded events.
            container.realize_content();
            self.raise_pivot_item_loaded(container);
            self.runtime.borrow_mut().current_pivot_item = Some(container.downgrade());
        }
        let focus_pending = std::mem::take(&mut self.runtime.borrow_mut().keyboard_focus_to_next_pivot_item_pending);
        if to_visible && focus_pending {
            container.set_keyboard_focus(first_expand);
        } else if focus_pending {
            self.runtime.borrow_mut().keyboard_focus_to_next_pivot_item_pending = true;
        }
    }

    /// Sets up the expressions of the layout element and of the items presenter (WinUI's static
    /// layout relationship and its inverse): during a pan the layout element stays in the
    /// viewport and the items follow the content; at rest both are where they are arranged.
    fn ensure_composition(&self) {
        let (scroll_viewer, layout_element, items_presenter) = {
            let parts = self.parts.borrow();
            (parts.scroll_viewer.clone(), parts.layout_element.clone(), parts.items_presenter.clone())
        };
        let (Some(scroll_viewer), Some(layout_element), Some(items_presenter)) =
            (scroll_viewer, layout_element, items_presenter)
        else {
            return;
        };
        let layout_visual = ElementComposition::get_element_visual(&layout_element);
        let items_visual = ElementComposition::get_element_visual(&items_presenter);
        let header_offset_visual = {
            let parts = self.parts.borrow();
            parts
                .header_panel
                .clone()
                .or_else(|| parts.static_header_panel.clone())
                .and_then(|panel| panel.visual_parent())
                .and_then(|parent| ElementComposition::get_element_visual(&parent))
        };
        let current = {
            let composition = self.composition.borrow();
            composition.as_ref().is_some_and(|composition| {
                same_visual(&composition.layout_visual, &layout_visual)
                    && same_visual(&composition.items_visual, &items_visual)
                    && same_visual(&composition.header_offset_visual, &header_offset_visual)
            })
        };
        if !current {
            let Some(visual) = layout_visual.clone().or_else(|| items_visual.clone()) else { return };
            let Some(scroller) = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroll_viewer)
            else {
                return;
            };
            let compositor = visual.compositor().clone();
            let properties = CompositionPropertySet::new(&compositor);
            properties.insert_scalar("LayoutOffset", 0.0);
            properties.insert_scalar("SectionOffset", 0.0);
            properties.insert_scalar("ItemsOpacity", 1.0);
            properties.insert_scalar("HeaderOffset", 0.0);
            let scroller_reference: Rc<dyn AsCompositionObject> = scroller;
            let properties_reference: Rc<dyn AsCompositionObject> = properties.clone();
            if let Some(layout_visual) = &layout_visual {
                let animation = compositor
                    .create_expression_animation_with("Vector3(-scroller.Translation.X - pivot.LayoutOffset, 0, 0)");
                animation.set_reference_parameter("scroller", scroller_reference.clone());
                animation.set_reference_parameter("pivot", properties_reference.clone());
                layout_visual.start_animation("Translation", &*animation);
            }
            if let Some(items_visual) = &items_visual {
                let animation = compositor
                    .create_expression_animation_with("Vector3(pivot.SectionOffset + scroller.Translation.X, 0, 0)");
                animation.set_reference_parameter("scroller", scroller_reference);
                animation.set_reference_parameter("pivot", properties_reference.clone());
                items_visual.start_animation("Translation", &*animation);
                // The fade of the fly animations (WinUI animates the opacity of the presenter).
                let opacity = compositor.create_expression_animation_with("pivot.ItemsOpacity");
                opacity.set_reference_parameter("pivot", properties_reference.clone());
                items_visual.start_animation("Opacity", &*opacity);
            }
            if let Some(header_offset_visual) = &header_offset_visual {
                let animation = compositor.create_expression_animation_with("Vector3(pivot.HeaderOffset, 0, 0)");
                animation.set_reference_parameter("pivot", properties_reference);
                header_offset_visual.start_animation("Translation", &*animation);
            }
            *self.composition.borrow_mut() =
                Some(Composition { properties, layout_visual, items_visual, header_offset_visual });
        }
        self.write_composition_offsets();
    }

    /// Writes the offsets the expressions read: where the layout element is arranged and the
    /// section offset of the items.
    fn write_composition_offsets(&self) {
        let composition = self.composition.borrow();
        let Some(composition) = composition.as_ref() else { return };
        let layout_offset = self.parts.borrow().panel.as_ref().map_or(0.0, |panel| panel.section_offset());
        composition.properties.insert_scalar("LayoutOffset", layout_offset as f32);
        composition.properties.insert_scalar("SectionOffset", self.runtime.borrow().section_offset as f32);
    }

    /// Completes the running fly animations (WinUI's `PivotAnimator::Complete`).
    /// Stops the fly animations (WinUI's `PivotAnimator::Stop`): the header offset and the opacity
    /// of the items go back to their base values; a pending completion is dropped.
    fn complete_animations(&self) {
        self.animation_generation.set(self.animation_generation.get() + 1);
        self.fly_batch.borrow_mut().take();
        let composition = self.composition.borrow();
        if let Some(composition) = composition.as_ref() {
            for (name, value) in [("SectionOffset", None), ("ItemsOpacity", Some(1.0)), ("HeaderOffset", Some(0.0))] {
                composition.properties.stop_animation(name);
                if let Some(value) = value {
                    composition.properties.insert_scalar(name, value);
                }
            }
            composition.properties.insert_scalar("SectionOffset", self.runtime.borrow().section_offset as f32);
        }
    }

    /// Whether the platform animates (WinUI's `UISettings.AnimationsEnabled`): without, the fly
    /// animations jump to their end.
    fn animations_enabled(&self) -> bool {
        self.get_platform_settings().is_none_or(|settings| settings.animations_enabled())
    }

    fn fly_duration(&self, milliseconds: u64) -> Duration {
        Duration::from_millis(if self.animations_enabled() { milliseconds } else { 1 })
    }

    /// WinUI's `PivotAnimator::AnimateOut`: the items move 7 pixels and fade out (83 and 67 ms,
    /// linear); the headers start to shift by `header_offset` (850 ms, WinUI's spline), until the
    /// fly-in stops them.
    fn animate_out(&self, start: f64, header_offset: f64, to_left: bool) {
        self.complete_animations();
        let composition = self.composition.borrow();
        let Some(composition) = composition.as_ref() else {
            drop(composition);
            self.complete_on_next_turn(None);
            return;
        };
        let properties = composition.properties.clone();
        let compositor = properties.compositor().clone();
        if header_offset > 0.0 {
            let animation = compositor.create_scalar_key_frame_animation();
            animation.insert_key_frame(0.0, 0.0);
            animation.insert_key_frame_with_easing(
                1.0,
                (if to_left { -header_offset } else { header_offset }) as f32,
                fly_spline(&compositor),
            );
            animation.set_duration(self.fly_duration(FLY_OUT_TRANSLATION_MS + FLY_IN_TRANSLATION_MS));
            properties.start_animation("HeaderOffset", &*animation);
        }
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let translation = compositor.create_scalar_key_frame_animation();
        translation.insert_key_frame(0.0, start as f32);
        let distance = if to_left { -FLY_OUT_TRANSLATION_DISTANCE } else { FLY_OUT_TRANSLATION_DISTANCE };
        translation.insert_key_frame_with_easing(1.0, (start + distance) as f32, linear(&compositor));
        translation.set_duration(self.fly_duration(FLY_OUT_TRANSLATION_MS));
        properties.start_animation("SectionOffset", &*translation);
        let opacity = compositor.create_scalar_key_frame_animation();
        opacity.insert_key_frame(0.0, 1.0);
        opacity.insert_key_frame_with_easing(1.0, 0.0, linear(&compositor));
        opacity.set_duration(self.fly_duration(FLY_OUT_OPACITY_MS));
        properties.start_animation("ItemsOpacity", &*opacity);
        self.complete_with(&batch);
        batch.end();
    }

    /// WinUI's `PivotAnimator::AnimateIn`: the items come from 20 pixels away and fade in (767 and
    /// 333 ms, WinUI's spline).
    fn animate_in(&self, to: f64, from_left: bool) {
        self.complete_animations();
        self.runtime.borrow_mut().section_offset = to;
        let composition = self.composition.borrow();
        let Some(composition) = composition.as_ref() else {
            drop(composition);
            self.complete_on_next_turn(Some(to));
            return;
        };
        let properties = composition.properties.clone();
        let compositor = properties.compositor().clone();
        let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
        let translation = compositor.create_scalar_key_frame_animation();
        let distance = if from_left { -FLY_IN_TRANSLATION_DISTANCE } else { FLY_IN_TRANSLATION_DISTANCE };
        translation.insert_key_frame(0.0, (to + distance) as f32);
        translation.insert_key_frame_with_easing(1.0, to as f32, fly_spline(&compositor));
        translation.set_duration(self.fly_duration(FLY_IN_TRANSLATION_MS));
        properties.start_animation("SectionOffset", &*translation);
        let opacity = compositor.create_scalar_key_frame_animation();
        opacity.insert_key_frame(0.0, 0.0);
        opacity.insert_key_frame_with_easing(1.0, 1.0, fly_spline(&compositor));
        opacity.set_duration(self.fly_duration(FLY_IN_OPACITY_MS));
        properties.start_animation("ItemsOpacity", &*opacity);
        self.complete_with(&batch);
        batch.end();
    }

    /// The machine moves on when the animations of `batch` complete (WinUI's `OnCompleted`).
    fn complete_with(&self, batch: &Rc<ferroui_base::rendering::composition::CompositionScopedBatch>) {
        let generation = self.animation_generation.get();
        let weak = self.to_ref().downgrade();
        let subscription = batch.completed(move |_| {
            if let Some(this) = weak.upgrade() {
                if this.animation_generation.get() == generation {
                    this.fly_batch.borrow_mut().take();
                    this.with_machine(|machine, cb| machine.animation_complete_event(cb));
                }
            }
        });
        *self.fly_batch.borrow_mut() = Some(subscription);
    }

    /// Without a compositor the animations complete on the next turn of the dispatcher.
    fn complete_on_next_turn(&self, final_section_offset: Option<f64>) {
        let generation = self.animation_generation.get();
        if let Some(offset) = final_section_offset {
            self.runtime.borrow_mut().section_offset = offset;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    if this.animation_generation.get() == generation {
                        this.with_machine(|machine, cb| machine.animation_complete_event(cb));
                    }
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    /// WinUI's `ShouldUseStaticHeaders`: static headers without the carousel, or while they fit.
    fn should_use_static_headers(&self) -> bool {
        let (header_panel, static_header_panel) = {
            let parts = self.parts.borrow();
            (parts.header_panel.clone(), parts.static_header_panel.clone())
        };
        let using_static = self.runtime.borrow().using_static_headers;
        match (header_panel, static_header_panel) {
            (Some(header_panel), Some(static_header_panel)) => {
                !self.is_header_items_carousel_enabled()
                    || (using_static && !static_header_panel.is_content_clipped())
                    || (!using_static && !header_panel.is_content_clipped())
            }
            (Some(_), None) => false,
            _ => true,
        }
    }

    /// WinUI's `UpdateHeaderState`: the headers in use (dynamic or static), the state of the
    /// template, the header manager and the machine.
    fn update_header_state(&self) {
        let previous = self.runtime.borrow().using_static_headers;
        let using_static = self.should_use_static_headers();
        self.runtime.borrow_mut().using_static_headers = using_static;
        let (header_panel, static_header_panel, panel) = {
            let parts = self.parts.borrow();
            (parts.header_panel.clone(), parts.static_header_panel.clone(), parts.panel.clone())
        };
        if (using_static && static_header_panel.is_some()) || (!using_static && header_panel.is_some()) {
            self.pseudo_classes().set(PC_HEADER_STATIC, using_static);
            VisualStateManager::go_to_state(self, if using_static { "HeaderStatic" } else { "HeaderDynamic" }, true);
            if let (Some(header_panel), Some(static_header_panel)) = (&header_panel, &static_header_panel) {
                header_panel.set_is_visible(!using_static);
                static_header_panel.set_is_visible(using_static);
            }
            self.headers.borrow_mut().header_state_changed_event(using_static);
            self.with_machine(move |machine, cb| machine.header_state_changed_event(cb, using_static));
            if previous != using_static {
                if let Some(panel) = panel {
                    panel.invalidate_measure();
                }
            }
        }
        self.update_visual_states();
    }

    /// WinUI's `UpdateVisualStates`: the previous and next buttons show while a mouse or a pen is
    /// over the headers, and the pivot is not locked; with static headers only while they are
    /// clipped and there is an item on that side.
    fn update_visual_states(&self) {
        let state = if self.runtime.borrow().is_pointer_over_headers && !self.is_locked() {
            let (previous, next) = if self.runtime.borrow().using_static_headers {
                let clipped =
                    self.parts.borrow().static_header_panel.as_ref().is_some_and(|panel| panel.is_content_clipped());
                let index = self.selected_index();
                if clipped && self.item_count() > 1 && index >= 0 {
                    (index > 0, index < self.item_count() - 1)
                } else {
                    (false, false)
                }
            } else {
                (true, true)
            };
            match (previous, next) {
                (true, true) => "NavigationButtonsVisible",
                (true, false) => "PreviousButtonVisible",
                (false, true) => "NextButtonVisible",
                (false, false) => "NavigationButtonsHidden",
            }
        } else {
            "NavigationButtonsHidden"
        };
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes
            .set(PC_PREVIOUS_BUTTON_VISIBLE, matches!(state, "NavigationButtonsVisible" | "PreviousButtonVisible"));
        pseudo_classes.set(PC_NEXT_BUTTON_VISIBLE, matches!(state, "NavigationButtonsVisible" | "NextButtonVisible"));
        if !VisualStateManager::go_to_state(self, state, true)
            && matches!(state, "PreviousButtonVisible" | "NextButtonVisible")
        {
            VisualStateManager::go_to_state(self, "NavigationButtonsVisible", true);
        }
    }

    /// The handlers of the parts of the headers (WinUI's `OnApplyTemplate`): the previous and next
    /// buttons, the pointer over the headers, the keys of the headers and their focus.
    fn attach_header_handlers(&self) {
        let (header_clipper, previous_button, next_button) = {
            let parts = self.parts.borrow();
            (parts.header_clipper.clone(), parts.previous_button.clone(), parts.next_button.clone())
        };
        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();
        let hover_targets: Vec<Ref<InputElement>> = [
            header_clipper.clone().map(|clipper| clipper.upcast()),
            previous_button.clone().map(|button| button.upcast()),
            next_button.clone().map(|button| button.upcast()),
        ]
        .into_iter()
        .flatten()
        .collect();
        for target in hover_targets {
            let weak = self.to_ref().downgrade();
            subscriptions.push(
                target
                    .add_disposable_handler(InputElement::pointer_entered_event(), move |_, e: &PointerEventArgs| {
                        if !matches!(e.pointer().type_(), PointerType::Mouse | PointerType::Pen) {
                            return;
                        }
                        if let Some(pivot) = weak.upgrade() {
                            pivot.runtime.borrow_mut().is_pointer_over_headers = true;
                            pivot.update_visual_states();
                        }
                    })
                    .detach(),
            );
            let weak = self.to_ref().downgrade();
            subscriptions.push(
                target
                    .add_disposable_handler(InputElement::pointer_exited_event(), move |_, _: &PointerEventArgs| {
                        if let Some(pivot) = weak.upgrade() {
                            // WinUI's `OnPointerExitedHeader`: still over the headers when the
                            // pointer moves to a navigation button.
                            let over = {
                                let parts = pivot.parts.borrow();
                                [
                                    parts.header_clipper.clone().map(|c| c.upcast::<InputElement>()),
                                    parts.previous_button.clone().map(|b| b.upcast::<InputElement>()),
                                    parts.next_button.clone().map(|b| b.upcast::<InputElement>()),
                                ]
                                .into_iter()
                                .flatten()
                                .any(|element| element.is_pointer_over())
                            };
                            pivot.runtime.borrow_mut().is_pointer_over_headers = over;
                            pivot.update_visual_states();
                        }
                    })
                    .detach(),
            );
        }
        for (button, next) in [(previous_button, false), (next_button, true)] {
            if let Some(button) = button {
                let weak = self.to_ref().downgrade();
                subscriptions.push(
                    button
                        .add_disposable_handler(Button::click_event(), move |_, _| {
                            if let Some(pivot) = weak.upgrade() {
                                let wrap = pivot.should_wrap();
                                if next {
                                    pivot.move_to_next_item(wrap);
                                } else {
                                    pivot.move_to_previous_item(wrap);
                                }
                            }
                        })
                        .detach(),
                );
            }
        }
        if let Some(header_clipper) = header_clipper {
            let weak = self.to_ref().downgrade();
            subscriptions.push(
                header_clipper
                    .add_disposable_handler(InputElement::key_down_event(), move |_, e: &KeyEventArgs| {
                        if let Some(pivot) = weak.upgrade() {
                            pivot.on_header_key_down(e);
                        }
                    })
                    .detach(),
            );
            for event in [InputElement::got_focus_event(), InputElement::lost_focus_event()] {
                let weak = self.to_ref().downgrade();
                subscriptions.push(
                    header_clipper
                        .add_disposable_handler(event, move |_, _| {
                            if let Some(pivot) = weak.upgrade() {
                                pivot.update_header_manager_focus_state();
                            }
                        })
                        .detach(),
                );
            }
        }
        self.template_subscriptions.borrow_mut().extend(subscriptions);
    }

    /// WinUI's `OnHeaderKeyDown`: Left and Right move through the items (wrapping with the
    /// carousel), Home and End to the first and the last item; right to left mirrors the arrows.
    fn on_header_key_down(&self, e: &KeyEventArgs) {
        if e.handled() {
            return;
        }
        let rtl = self.flow_direction() == ferroui_base::media::FlowDirection::RightToLeft;
        let key = match e.key {
            Key::Left if rtl => Key::Right,
            Key::Right if rtl => Key::Left,
            key => key,
        };
        let wrap = self.should_wrap();
        let moved = match key {
            Key::Left => self.move_to_previous_item(wrap),
            Key::Right => self.move_to_next_item(wrap),
            Key::Home => self.move_to_first_item(),
            Key::End => self.move_to_last_item(),
            _ => return,
        };
        e.set_handled(moved);
    }

    /// Moves to the first item (WinUI's `MoveToFirstItem`).
    pub fn move_to_first_item(&self) -> bool {
        if self.is_locked() || self.item_count() == 0 || self.selected_index() <= 0 {
            return false;
        }
        self.move_to_item(0)
    }

    /// Moves to the last item (WinUI's `MoveToLastItem`).
    pub fn move_to_last_item(&self) -> bool {
        let count = self.item_count();
        if self.is_locked() || count == 0 || self.selected_index() >= count - 1 {
            return false;
        }
        self.move_to_item(count - 1)
    }

    /// WinUI's `InvalidateHeaderSecondaryContentRelationships`: the header panels are arranged
    /// again, which applies their parallax.
    pub(crate) fn invalidate_header_secondary_content_relationships(&self) {
        let parts = self.parts.borrow();
        for panel in [&parts.header_panel, &parts.static_header_panel].into_iter().flatten() {
            panel.invalidate_arrange();
        }
    }

    /// The header panel was arranged (WinUI's `HeaderPanelMeasureEvent`).
    pub(crate) fn on_header_panel_measure(&self, viewport_size: f32) {
        self.with_machine(move |machine, cb| machine.header_measure_event(cb, viewport_size));
    }

    /// The header panel placed its headers (WinUI's `HeaderPanelSetLteOffsetEvent`).
    pub(crate) fn on_header_panel_ghost_offset(
        &self,
        panel: &PivotHeaderPanel,
        ghost_offset: f64,
        vertical_offset: f64,
    ) {
        if let Ok(mut headers) = self.headers.try_borrow_mut() {
            headers.update_ghost(panel, ghost_offset, vertical_offset);
        }
    }

    /// WinUI's `UpdateHeaderManagerFocusState`: the selected header shows the focus when the
    /// headers have it and the placement is the selected header.
    fn update_header_manager_focus_state(&self) {
        let focused =
            self.parts.borrow().header_clipper.as_ref().is_some_and(|clipper| clipper.is_keyboard_focus_within());
        let show =
            focused && self.header_focus_visual_placement() == PivotHeaderFocusVisualPlacement::SelectedItemHeader;
        if let Ok(mut headers) = self.headers.try_borrow_mut() {
            headers.focus_state_changed_event(show);
        }
        self.update_focus_follower_impl(self.selected_index());
    }

    /// WinUI's `UpdateFocusFollowerImpl`: with the focus on the selected header, the focus follower
    /// of the template takes the place and the size of that header.
    fn update_focus_follower_impl(&self, index: i32) {
        if self.header_focus_visual_placement() != PivotHeaderFocusVisualPlacement::SelectedItemHeader {
            return;
        }
        let (focus_follower, header_clipper) = {
            let parts = self.parts.borrow();
            (parts.focus_follower.clone(), parts.header_clipper.clone())
        };
        let (Some(focus_follower), Some(header_clipper)) = (focus_follower, header_clipper) else { return };
        let header =
            usize::try_from(index).ok().and_then(|index| self.headers.borrow().header_items().get(index).cloned());
        let (position, size) = match &header {
            Some(header) => {
                let position = header.translate_point(ferroui_base::Point::new(0.0, 0.0), &header_clipper);
                (position.unwrap_or_default(), header.bounds().size())
            }
            None => (ferroui_base::Point::default(), Size::default()),
        };
        focus_follower.set_width(size.width);
        focus_follower.set_height(size.height);
        focus_follower.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
        focus_follower.set_translation(Vector3D::new(position.x, 0.0, 0.0));
    }

    /// The scroll gesture recognizer of the scroll viewer.
    fn scroll_gesture_recognizer(&self) -> Option<Ref<ScrollGestureRecognizer>> {
        let scroll_viewer = self.parts.borrow().scroll_viewer.clone()?;
        let presenter = scroll_viewer.scroll_content_presenter()?;
        presenter
            .gesture_recognizers()
            .to_vec()
            .into_iter()
            .find_map(|recognizer| recognizer.cast::<ScrollGestureRecognizer>())
    }

    /// Adds the handlers of the scroll gestures, once per pivot.
    fn ensure_gesture_handlers(&self) {
        let weak = self.to_ref().downgrade();
        self.add_handler_with(
            InputElement::scroll_gesture_event(),
            move |_, e: &ScrollGestureEventArgs| {
                if let Some(pivot) = weak.upgrade() {
                    pivot.on_scroll_gesture(e);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        let weak = self.to_ref().downgrade();
        self.add_handler_with(
            InputElement::scroll_gesture_ended_event(),
            move |_, e: &ScrollGestureEndedEventArgs| {
                if let Some(pivot) = weak.upgrade() {
                    pivot.on_scroll_gesture_ended(e);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
    }
}

fn same_visual(a: &Option<Rc<CompositionVisual>>, b: &Option<Rc<CompositionVisual>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

impl PivotStateMachineCallbacks for Pivot {
    fn is_header_items_carousel_enabled(&self) -> bool {
        Pivot::is_header_items_carousel_enabled(self)
    }

    fn pivot_panel_multiplier(&self) -> u32 {
        self.pivot_panel_multiplier_impl()
    }

    /// WinUI's `SetPivotSectionOffset`: stops the fly animations and moves the items.
    fn set_pivot_section_offset(&self, offset: f64) {
        self.complete_animations();
        self.runtime.borrow_mut().section_offset = offset;
        self.write_composition_offsets();
    }

    /// WinUI's `SetViewportOffset`: the snap points are turned off for the jump, which the scroll
    /// viewer could otherwise move while its viewport is resized.
    fn set_viewport_offset(&self, offset: f64, animate: bool) -> bool {
        let scroll_viewer = self.parts.borrow().scroll_viewer.clone();
        let Some(scroll_viewer) = scroll_viewer else { return false };
        let snap_points = scroll_viewer.horizontal_snap_points_type();
        scroll_viewer.set_horizontal_snap_points_type(SnapPointsType::None);
        let moved = scroll_viewer.change_view_with(Some(offset), None, None, !animate);
        scroll_viewer.set_horizontal_snap_points_type(snap_points);
        // WinUI's `SyncParallax`: the header panels apply their relationships again.
        self.invalidate_header_secondary_content_relationships();
        moved
    }

    fn set_pivot_section_width(&self, width: f32) {
        if let Some(panel) = self.parts.borrow().panel.clone() {
            panel.set_section_width(width as f64);
        }
    }

    /// WinUI's `SetSelectedIndex`.
    fn set_selected_index_with(
        &self,
        index: i32,
        update_visual: bool,
        update_index: bool,
        _update_item: bool,
        animation_hint: PivotAnimationDirection,
    ) {
        if update_index {
            let guarded = std::mem::replace(&mut self.runtime.borrow_mut().index_change_guard, true);
            if !guarded {
                // The selected item follows the index in the base, which raises the selection
                // changed event when the item changes (WinUI's `SyncItemToIndex` and
                // `RaiseOnSelectionChanged`).
                SelectingItemsControl::set_selected_index(self, index);
                self.runtime.borrow_mut().index_change_guard = false;
            }
        }
        // The handlers of the application may have moved the selection.
        let index = self.selected_index();
        if update_visual {
            let previous = self.runtime.borrow().previous_selected_index;
            if previous >= 0 {
                // A focused element of the item that is hidden gives the focus to the headers.
                if let (Some(_), Some(previous_item)) = (
                    self.container_from_index(index),
                    self.container_from_index(previous).and_then(|container| container.cast::<PivotItem>()),
                ) {
                    if previous_item.is_keyboard_focus_within() {
                        if let Some(header_clipper) = self.parts.borrow().header_clipper.clone() {
                            header_clipper.focus();
                        }
                    }
                }
            }
            self.update_visible_content(index, animation_hint);
            self.headers.borrow_mut().set_selected_index(index, animation_hint);
            self.runtime.borrow_mut().previous_selected_index = -1;
        } else if let Some(container) =
            self.container_from_index(index).and_then(|container| container.cast::<PivotItem>())
        {
            // The new item joins the tree hidden, so that the narrator finds an item for the
            // selected index; it is shown when the fly-out completes.
            container.set_is_hit_test_visible(false);
            container.set_opacity(0.0);
            container.set_content_visibility(true);
        }
    }

    /// WinUI's `SetParallaxRelationship`: the parallax of the headers in use, and the static
    /// relationships of the layout element and of the items (composition expressions).
    fn set_parallax_relationship(&self, section_offset: f64, section_width: f64, viewport_size: f32) {
        self.ensure_composition();
        let scroll_viewer = self.parts.borrow().scroll_viewer.clone();
        let Some(scroll_viewer) = scroll_viewer else { return };
        let scroller = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroll_viewer)
            .map(|set| set as Rc<dyn AsCompositionObject>);
        let is_dynamic = !self.runtime.borrow().using_static_headers;
        let multiplier = self.pivot_panel_multiplier_impl();
        self.headers.borrow_mut().apply_parallax(
            &scroll_viewer,
            scroller,
            section_offset,
            section_width,
            is_dynamic,
            viewport_size,
            multiplier,
        );
    }

    fn set_snapping_behavior(&self, mandatory_single: bool) {
        if let Some(scroll_viewer) = self.parts.borrow().scroll_viewer.clone() {
            scroll_viewer.set_horizontal_snap_points_type(if mandatory_single {
                SnapPointsType::MandatorySingle
            } else {
                SnapPointsType::Mandatory
            });
        }
    }

    /// WinUI's `SetViewportEnabled` (the scroll mode of the scroll viewer): the scroll gestures of
    /// the scroll viewer are turned on or off.
    fn set_viewport_enabled(&self, enabled: bool) {
        self.runtime.borrow_mut().viewport_enabled = enabled;
        if let Some(recognizer) = self.scroll_gesture_recognizer() {
            recognizer.set_can_horizontally_scroll(enabled);
        }
    }

    /// WinUI's `StartFlyOutAnimation`.
    fn start_fly_out_animation(&self, from: f64, header_offset: f64, to_left: bool) {
        self.animate_out(from, header_offset, to_left);
    }

    /// WinUI's `StartFlyInAnimation`: the items end at `to`.
    fn start_fly_in_animation(&self, to: f64, from_left: bool) {
        self.animate_in(to, from_left);
    }

    fn is_in_manipulation_animation(&self) -> bool {
        self.runtime.borrow().in_manipulation
    }

    fn cancel_manipulation_animations(&self) {
        // FerroUI's scroll gestures cannot be cancelled from outside: the manipulation ends with
        // its gesture, and the deferred offset is set then.
    }

    /// WinUI's `UpdateScrollViewerDragDirection`: with two items the other one moves to the side
    /// the user drags to.
    fn update_scroll_viewer_drag_direction(&self, direction: PivotAnimationDirection) {
        if self.item_count() == 2 && self.runtime.borrow().pivot_drag_direction != direction {
            self.runtime.borrow_mut().pivot_drag_direction = direction;
            self.update_visible_content(self.selected_index(), direction);
        }
    }

    fn update_focus_follower(&self) {
        self.update_focus_follower_impl(self.selected_index());
    }
}

/// The pseudo-classes of the header and navigation button states (WinUI's `HeaderStates` and
/// `NavigationButtonsVisibility` groups).
const PC_HEADER_STATIC: &str = ":header-static";
const PC_PREVIOUS_BUTTON_VISIBLE: &str = ":previous-button-visible";
const PC_NEXT_BUTTON_VISIBLE: &str = ":next-button-visible";

/// WinUI's `PivotAnimator` constants.
const FLY_OUT_TRANSLATION_MS: u64 = 83;
const FLY_OUT_TRANSLATION_DISTANCE: f64 = 7.0;
const FLY_OUT_OPACITY_MS: u64 = 67;
const FLY_IN_TRANSLATION_MS: u64 = 767;
const FLY_IN_TRANSLATION_DISTANCE: f64 = 20.0;
const FLY_IN_OPACITY_MS: u64 = 333;

/// WinUI's spline key frame of the pivot animations (control points 0.1,0.9 and 0.2,1.0).
fn fly_spline(compositor: &Compositor) -> Rc<dyn IEasing> {
    compositor.create_cubic_bezier_easing_function(Vector2::new(0.1, 0.9), Vector2::new(0.2, 1.0))
}

fn linear(compositor: &Compositor) -> Rc<dyn IEasing> {
    compositor.create_linear_easing_function()
}

/// WinUI's `PivotSlideInManager::ApplySlideInAnimation` with its `PivotSlideInThemeAnimation`: the
/// elements of `item` with a `SlideInAnimationGroup` slide in from 40 pixels times their group, on
/// the side the pivot moves from, over 700 ms (linear to 28 % of the distance at half time, then an
/// exponential ease in), on the compositor.
fn apply_slide_in_animation(item: &PivotItem, direction: PivotAnimationDirection) {
    const SLIDE_IN_GAP_BETWEEN_GROUPS: f64 = 40.0;
    const DURATION_MS: u64 = 700;
    const LINEAR_PORTION: f32 = 0.5;
    const LINEAR_TO_EXP_VALUE_FACTOR: f64 = 0.283_350_521_296_607_65;
    const EASE_EXPONENT: f32 = -2.302_585_1;
    let side = if direction == PivotAnimationDirection::Left { -1.0 } else { 1.0 };
    let mut stack: Vec<Ref<Visual>> = item.visual_children().snapshot().iter().cloned().collect();
    while let Some(visual) = stack.pop() {
        stack.extend(visual.visual_children().snapshot().iter().cloned());
        let Some(control) = visual.cast::<Control>() else { continue };
        let group = Pivot::get_slide_in_animation_group(&control);
        if group == PivotSlideInAnimationGroup::Default {
            continue;
        }
        let Some(composition_visual) = ElementComposition::get_element_visual(&control) else { continue };
        let offset = SLIDE_IN_GAP_BETWEEN_GROUPS * group as i32 as f64 * side;
        let compositor = composition_visual.compositor().clone();
        let animation = compositor.create_scalar_key_frame_animation();
        animation.insert_key_frame(0.0, offset as f32);
        animation.insert_key_frame_with_easing(
            LINEAR_PORTION,
            (offset * LINEAR_TO_EXP_VALUE_FACTOR) as f32,
            linear(&compositor),
        );
        animation.insert_key_frame_with_easing(
            1.0,
            0.0,
            Rc::new(ExponentialEasingFunction::new(CompositionEasingFunctionMode::In, EASE_EXPONENT)),
        );
        animation.set_duration(Duration::from_millis(DURATION_MS));
        composition_visual.start_animation("Translation.X", &*animation);
    }
}
