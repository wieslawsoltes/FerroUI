//! WinUI's `SemanticZoom` (`dxaml/xcp/dxaml/lib/SemanticZoom_Partial.cpp`,
//! `SemanticZoom_Partial.h` and the template of `dxaml/xcp/dxaml/themes/generic.xaml` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).
//!
//! The port keeps the programmatic changes of views (`IsZoomedInViewActive`, `ToggleActiveView`,
//! the zoom-out button, the keyboard, Ctrl+wheel, a tap on an item of the zoomed-out view or on
//! a group header of the zoomed-in view) with WinUI's order of calls to the two views. The pinch
//! of DirectManipulation (`NotifyStateChange`, the thresholds and the correction transforms) is
//! not ported; the zoom of a change is a composition animation of the two presenters
//! (`semantic_zoom_transition.rs`; docs/porting/winui-controls.md, section 10.20).

use super::semantic_zoom_information::{as_semantic_zoom_information, register_semantic_zoom_information};
use super::semantic_zoom_transition::SemanticZoomTransition;
use super::{SemanticZoomLocation, SemanticZoomViewChangedEventArgs};
use crate::automation::peers::{AutomationPeer, SemanticZoomAutomationPeer};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{Button, Control, ControlImpl, ListViewBase, ScrollViewer, ZoomMode};
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode,
    NavigationMethod, PointerEventArgs, PointerType, PointerWheelEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PART_ZOOMED_IN_PRESENTER: &str = "ZoomedInPresenter";
const PART_ZOOMED_OUT_PRESENTER: &str = "ZoomedOutPresenter";
const PART_ZOOM_OUT_BUTTON: &str = "ZoomOutButton";
/// Optional (not a declared part): a custom template with a ScrollViewer whose zoom is enabled
/// lets Ctrl+wheel change the views, as WinUI's template does.
const PART_SCROLL_VIEWER: &str = "ScrollViewer";

/// The `ZoomOutView` state of `SemanticZoomStates` (`ZoomInView` without it).
const PC_ZOOMED_OUT: &str = ":zoomed-out";
/// The `ZoomOutButtonVisible` state of `ZoomOutButtonStates` (`ZoomOutButtonHidden` without it).
const PC_ZOOM_OUT_BUTTON_VISIBLE: &str = ":zoom-out-button-visible";

/// The `BeginTime` of the transition from `ZoomOutButtonVisible` to `ZoomOutButtonHidden`.
pub(crate) const ZOOM_OUT_BUTTON_HIDE_DELAY: Duration = Duration::from_secs(3);

type ViewChangedHandler = dyn Fn(&Ref<SemanticZoom>, &SemanticZoomViewChangedEventArgs);

/// Why the views change (WinUI's `SemanticZoomPhase`; the phases of DirectManipulation are not
/// ported).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    /// An API call, the wheel, the keyboard or a click: until the zoom has ended.
    ApiSwitchingViews,
}

/// The input that changes the views, for the focus state of the destination (WinUI's
/// `FocusState` of `ToggleActiveViewWithFocusState`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ViewChangeInput {
    #[default]
    Programmatic,
    Keyboard,
    Pointer,
}

impl ViewChangeInput {
    /// The navigation method the destination view focuses its item with.
    pub(crate) fn navigation_method(self) -> NavigationMethod {
        match self {
            Self::Programmatic => NavigationMethod::Unspecified,
            Self::Keyboard => NavigationMethod::Directional,
            Self::Pointer => NavigationMethod::Pointer,
        }
    }
}

/// A control with two views of the same content, a zoomed-in view and a zoomed-out view to
/// navigate it (WinUI's `SemanticZoom`).
///
/// [`zoomed_in_view`](Self::zoomed_in_view) and [`zoomed_out_view`](Self::zoomed_out_view) are
/// controls that implement [`ISemanticZoomInformation`] (a `ListView` or a `GridView`, or a class
/// registered with [`register_semantic_zoom_information`]); the template shows them in the
/// presenters `ZoomedInPresenter` and `ZoomedOutPresenter`. Setting
/// [`is_zoomed_in_view_active`](Self::is_zoomed_in_view_active) changes the view: the source
/// view names the item it starts from, the destination view scrolls to the corresponding item,
/// and the two presenters zoom on the compositor. The pseudo-classes are `:zoomed-out` (WinUI's
/// `ZoomOutView` state) and `:zoom-out-button-visible`.
#[repr(C)]
pub struct SemanticZoom {
    base: TemplatedControl,
    zoomed_in_presenter: RefCell<Option<WeakRef<Control>>>,
    zoomed_out_presenter: RefCell<Option<WeakRef<Control>>>,
    zoom_out_button: RefCell<Option<(WeakRef<Button>, RoutedEventHandlerToken)>>,
    scroll_viewer: RefCell<Option<WeakRef<ScrollViewer>>>,
    /// From the constructor until the template is applied (WinUI's `m_isInitializing`).
    is_initializing: Cell<bool>,
    /// A change of `IsZoomedInViewActive` before the template (WinUI's `m_isPendingViewChange`).
    is_pending_view_change: Cell<bool>,
    input: Cell<ViewChangeInput>,
    is_cancelling_jump_list: Cell<bool>,
    /// Ctrl+wheel imitates a gesture (WinUI's `m_emulatingGesture`), centred on `zoom_point`.
    emulating_gesture: Cell<bool>,
    zoom_point: Cell<Point>,
    phase: Cell<Phase>,
    /// Whether the views were told `InitializeViewChange` and wait for `CompleteViewChange`.
    called_initialize_view_change: Cell<bool>,
    /// Set by `change_views`, cleared when `ViewChangeCompleted` is raised.
    is_processing_view_change: Cell<bool>,
    /// The arguments of the running change, for `ViewChangeCompleted` (WinUI's
    /// `m_tpCompletedArgs`).
    completed_args: RefCell<Option<SemanticZoomViewChangedEventArgs>>,
    transition: SemanticZoomTransition,
    zoom_out_button_hide: RefCell<Option<Subscription>>,
    view_change_started: HandlerList<ViewChangedHandler>,
    view_change_completed: HandlerList<ViewChangedHandler>,
}

ferro_class!(SemanticZoom: TemplatedControl);
ferro_class_info!(SemanticZoom {
    new: SemanticZoom::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            ViewChangeStarted(Option<BoxedValue>, SemanticZoomViewChangedEventArgs) =>
                |this: &Ref<SemanticZoom>, handler: MarkupDelegate| {
                    this.view_change_started(move |sender, args| {
                        handler.invoke(&[into_markup_value(Some(sender.clone())), into_markup_value(args.clone())]);
                    })
                },
            ViewChangeCompleted(Option<BoxedValue>, SemanticZoomViewChangedEventArgs) =>
                |this: &Ref<SemanticZoom>, handler: MarkupDelegate| {
                    this.view_change_completed(move |sender, args| {
                        handler.invoke(&[into_markup_value(Some(sender.clone())), into_markup_value(args.clone())]);
                    })
                },
        ],
        attributes: [
            TemplatePart("ZoomedInPresenter", type(Ref<Control>)),
            TemplatePart("ZoomedOutPresenter", type(Ref<Control>)),
            TemplatePart("ZoomOutButton", type(Ref<Button>)),
            PseudoClasses(":zoomed-out", ":zoom-out-button-visible"),
        ],
    },
});
ferro_impl_classes!(SemanticZoom: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

ferro_properties! { impl SemanticZoom {
    ferro_property!(
        /// The view of the content (WinUI's `ZoomedInView`).
        pub fn zoomed_in_view_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<SemanticZoom, _>("ZoomedInView", None)
        }
    );

    ferro_property!(
        /// The view that navigates the content (WinUI's `ZoomedOutView`).
        pub fn zoomed_out_view_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<SemanticZoom, _>("ZoomedOutView", None)
        }
    );

    ferro_property!(
        /// Whether the zoomed-in view is shown (WinUI's `IsZoomedInViewActive`).
        pub fn is_zoomed_in_view_active_property() -> StyledProperty<bool> {
            FerroProperty::register::<SemanticZoom, _>("IsZoomedInViewActive", true)
        }
    );

    ferro_property!(
        /// Whether the views can change (WinUI's `CanChangeViews`).
        pub fn can_change_views_property() -> StyledProperty<bool> {
            FerroProperty::register::<SemanticZoom, _>("CanChangeViews", true)
        }
    );

    ferro_property!(
        /// Whether the zoom-out button shows for a mouse over the zoomed-in view (WinUI's
        /// `IsZoomOutButtonEnabled`; WinUI's default style sets it to false).
        pub fn is_zoom_out_button_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<SemanticZoom, _>("IsZoomOutButtonEnabled", true)
        }
    );
} }

impl FerroObjectImpl for SemanticZoom {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::is_zoomed_in_view_active_property().as_property() {
            this.on_is_zoomed_in_view_active_changed();
        } else if property == Self::zoomed_in_view_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<Option<Ref<Control>>>();
            this.initialize_semantic_zoom_information(old.as_ref(), new.as_ref(), true);
        } else if property == Self::zoomed_out_view_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<Option<Ref<Control>>>();
            this.initialize_semantic_zoom_information(old.as_ref(), new.as_ref(), false);
        } else if property == Self::is_zoom_out_button_enabled_property().as_property()
            && !change.get_new_value::<bool>()
        {
            this.hide_zoom_out_button();
        }
    }
}

impl ControlImpl for SemanticZoom {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        SemanticZoomAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for SemanticZoom {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        this.transition.finish();
        this.detach_zoom_out_button();
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        *this.zoomed_in_presenter.borrow_mut() =
            scope.find_as::<Control>(PART_ZOOMED_IN_PRESENTER).map(|part| part.downgrade());
        *this.zoomed_out_presenter.borrow_mut() =
            scope.find_as::<Control>(PART_ZOOMED_OUT_PRESENTER).map(|part| part.downgrade());
        *this.scroll_viewer.borrow_mut() =
            scope.find_as::<ScrollViewer>(PART_SCROLL_VIEWER).map(|part| part.downgrade());
        if let Some(button) = scope.find_as::<Button>(PART_ZOOM_OUT_BUTTON) {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    // OnZoomOutButtonClick.
                    owner.toggle_active_view_with_input(ViewChangeInput::Pointer);
                }
            });
            *this.zoom_out_button.borrow_mut() = Some((button.downgrade(), token));
        }
        this.phase.set(Phase::Idle);
        this.is_initializing.set(false);
        if this.is_pending_view_change.get() {
            // The change waited for the template: no transition.
            this.update_visual_state();
            this.change_views();
        } else {
            this.update_visual_state();
            this.show_active_presenter();
        }
        this.hide_zoom_out_button();
        this.is_pending_view_change.set(false);
    }
}

impl InputElementImpl for SemanticZoom {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        this.input.set(ViewChangeInput::Keyboard);
        if !e.handled() && this.can_change_views() {
            match keyboard_zoom_direction(e.key_modifiers, e.key) {
                Some(ZoomDirection::Out) if this.is_zoomed_in_view_active() => {
                    e.set_handled(true);
                    this.set_is_zoomed_in_view_active(false);
                }
                Some(ZoomDirection::In) if !this.is_zoomed_in_view_active() => {
                    e.set_handled(true);
                    this.set_is_zoomed_in_view_active(true);
                }
                _ => {}
            }
        }
        this.input.set(ViewChangeInput::Programmatic);
        Self::parent_on_key_down(this, e);
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        this.input.set(ViewChangeInput::Pointer);
        Self::parent_on_pointer_wheel_changed(this, e);
        // Only a template ScrollViewer whose zoom is enabled lets Ctrl+wheel change the views (the
        // default template has none: WinUI's default style disables its zoom).
        let allows_mouse_zoom = this.scroll_viewer().is_some_and(|viewer| viewer.zoom_mode() != ZoomMode::Disabled);
        if !e.handled()
            && allows_mouse_zoom
            && this.can_change_views()
            && e.key_modifiers().contains(KeyModifiers::CONTROL)
        {
            let delta = e.delta().y;
            this.zoom_point.set(e.get_position(Some(this)));
            this.emulating_gesture.set(true);
            let active = this.is_zoomed_in_view_active();
            if !active && delta > 0.0 {
                e.set_handled(true);
                this.set_is_zoomed_in_view_active(true);
            } else if active && delta < 0.0 {
                e.set_handled(true);
                this.set_is_zoomed_in_view_active(false);
            }
        }
        this.emulating_gesture.set(false);
        this.input.set(ViewChangeInput::Programmatic);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        // The event keeps routing: it only shows the button again (and restarts its fade-out).
        if !this.is_processing_view_change.get()
            && this.is_zoomed_in_view_active()
            && this.is_zoom_out_button_enabled()
            && e.pointer().type_() != PointerType::Touch
        {
            this.show_zoom_out_button();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ZoomDirection {
    In,
    Out,
}

/// WinUI's `ScrollViewer::GetKeyboardMessageZoomAction`: Ctrl with minus zooms out, Ctrl with
/// plus zooms in (Shift is ignored).
fn keyboard_zoom_direction(modifiers: KeyModifiers, key: Key) -> Option<ZoomDirection> {
    if modifiers - KeyModifiers::SHIFT != KeyModifiers::CONTROL {
        return None;
    }
    match key {
        Key::Subtract | Key::OemMinus => Some(ZoomDirection::Out),
        Key::Add | Key::OemPlus => Some(ZoomDirection::In),
        _ => None,
    }
}

impl SemanticZoom {
    fn static_constructor() {
        // A ListView or GridView is a view of a semantic zoom (WinUI's ListViewBase implements
        // ISemanticZoomInformation); registered here, so an application without a semantic zoom
        // does not keep the implementation.
        register_semantic_zoom_information::<ListViewBase>();
        // The markup types of the events, with the control (not in the list of every application).
        MarkupType::register_all(&[
            <SemanticZoomLocation as MarkupTyped>::MARKUP,
            <SemanticZoomViewChangedEventArgs as MarkupTyped>::MARKUP,
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            zoomed_in_presenter: RefCell::new(None),
            zoomed_out_presenter: RefCell::new(None),
            zoom_out_button: RefCell::new(None),
            scroll_viewer: RefCell::new(None),
            is_initializing: Cell::new(true),
            is_pending_view_change: Cell::new(false),
            input: Cell::new(ViewChangeInput::Programmatic),
            is_cancelling_jump_list: Cell::new(false),
            emulating_gesture: Cell::new(false),
            zoom_point: Cell::new(Point::new(0.0, 0.0)),
            phase: Cell::new(Phase::Idle),
            called_initialize_view_change: Cell::new(false),
            is_processing_view_change: Cell::new(false),
            completed_args: RefCell::new(None),
            transition: SemanticZoomTransition::default(),
            zoom_out_button_hide: RefCell::new(None),
            view_change_started: HandlerList::new(),
            view_change_completed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn zoomed_in_view(&self) -> Option<Ref<Control>> {
        self.get_value(Self::zoomed_in_view_property())
    }

    pub fn set_zoomed_in_view(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::zoomed_in_view_property(), value);
    }

    pub fn zoomed_out_view(&self) -> Option<Ref<Control>> {
        self.get_value(Self::zoomed_out_view_property())
    }

    pub fn set_zoomed_out_view(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::zoomed_out_view_property(), value);
    }

    pub fn is_zoomed_in_view_active(&self) -> bool {
        self.get_value(Self::is_zoomed_in_view_active_property())
    }

    pub fn set_is_zoomed_in_view_active(&self, value: bool) {
        self.set_value(Self::is_zoomed_in_view_active_property(), value);
    }

    pub fn can_change_views(&self) -> bool {
        self.get_value(Self::can_change_views_property())
    }

    pub fn set_can_change_views(&self, value: bool) {
        self.set_value(Self::can_change_views_property(), value);
    }

    pub fn is_zoom_out_button_enabled(&self) -> bool {
        self.get_value(Self::is_zoom_out_button_enabled_property())
    }

    pub fn set_is_zoom_out_button_enabled(&self, value: bool) {
        self.set_value(Self::is_zoom_out_button_enabled_property(), value);
    }

    /// Adds a handler of `ViewChangeStarted`. Dropping the guard removes it.
    pub fn view_change_started(
        &self,
        handler: impl Fn(&Ref<SemanticZoom>, &SemanticZoomViewChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.view_change_started.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.view_change_started.remove(token);
            }
        }))
    }

    /// Adds a handler of `ViewChangeCompleted`. Dropping the guard removes it.
    pub fn view_change_completed(
        &self,
        handler: impl Fn(&Ref<SemanticZoom>, &SemanticZoomViewChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.view_change_completed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.view_change_completed.remove(token);
            }
        }))
    }

    /// Shows the other view (WinUI's `ToggleActiveView`). Does nothing when
    /// [`can_change_views`](Self::can_change_views) is false (WinUI fails the call).
    pub fn toggle_active_view(&self) {
        if !self.can_change_views() {
            return;
        }
        let zoomed_in = !self.is_zoomed_in_view_active();
        self.set_is_zoomed_in_view_active(zoomed_in);
        if let Some(peer) = self.get_automation_peer().and_then(|peer| peer.cast::<SemanticZoomAutomationPeer>()) {
            peer.raise_toggle_state_property_changed_event(zoomed_in);
        }
    }

    /// WinUI's `ToggleActiveViewWithFocusState`: the destination view focuses its item as the
    /// input does.
    pub(crate) fn toggle_active_view_with_input(&self, input: ViewChangeInput) {
        if self.can_change_views() {
            self.input.set(input);
            self.toggle_active_view();
        }
        self.input.set(ViewChangeInput::Programmatic);
    }

    /// The input that changes the views (WinUI's `GetIsProcessingKeyboardInput` and
    /// `GetIsProcessingPointerInput`).
    pub(crate) fn processing_input(&self) -> ViewChangeInput {
        self.input.get()
    }

    /// Whether the change goes back from the zoomed-out view without a destination (WinUI's
    /// `GetIsCancellingJumpList`, the back button of a phone).
    pub(crate) fn is_cancelling_jump_list(&self) -> bool {
        self.is_cancelling_jump_list.get()
    }

    /// Whether a zoom between the views runs.
    pub fn is_view_change_animating(&self) -> bool {
        self.transition.is_running()
    }

    fn scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        self.scroll_viewer.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn zoomed_in_presenter(&self) -> Option<Ref<Control>> {
        self.zoomed_in_presenter.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn zoomed_out_presenter(&self) -> Option<Ref<Control>> {
        self.zoomed_out_presenter.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn on_is_zoomed_in_view_active_changed(&self) {
        if !self.can_change_views() {
            return;
        }
        if self.is_initializing.get() {
            // Before the template: the change waits; a second change cancels it.
            self.is_pending_view_change.set(!self.is_pending_view_change.get());
            return;
        }
        // A zoom that still runs ends with its ViewChangeCompleted first.
        self.transition.finish();
        self.phase.set(Phase::ApiSwitchingViews);
        self.change_views();
    }

    /// WinUI's `InitializeSemanticZoomInformation`: the view knows its semantic zoom.
    fn initialize_semantic_zoom_information(
        &self,
        old: Option<&Ref<Control>>,
        new: Option<&Ref<Control>>,
        is_zoomed_in_view: bool,
    ) {
        if let Some(view) = old.and_then(|old| as_semantic_zoom_information(old)) {
            view.set_semantic_zoom_owner(None);
            view.set_is_active_view(false);
            view.set_is_zoomed_in_view(true);
        }
        if let Some(view) = new.and_then(|new| as_semantic_zoom_information(new)) {
            view.set_semantic_zoom_owner(Some(&self.to_ref()));
            view.set_is_zoomed_in_view(is_zoomed_in_view);
            view.set_is_active_view(is_zoomed_in_view == self.is_zoomed_in_view_active());
        }
    }

    /// WinUI's `ChangeVisualState`: `ZoomOutView` or `ZoomInView`.
    fn update_visual_state(&self) {
        self.pseudo_classes().set(PC_ZOOMED_OUT, !self.is_zoomed_in_view_active());
    }

    /// The presenters at rest: the active one shown, the other transparent, without hits and
    /// out of the tab order (WinUI's opacity of the fade-out state, `IsHitTestVisible` and
    /// `SetSkipFocusSubtree`).
    fn show_active_presenter(&self) {
        let zoomed_in_active = self.is_zoomed_in_view_active();
        for (presenter, active) in
            [(self.zoomed_in_presenter(), zoomed_in_active), (self.zoomed_out_presenter(), !zoomed_in_active)]
        {
            if let Some(presenter) = presenter {
                presenter.set_opacity(if active { 1.0 } else { 0.0 });
                Self::set_presenter_input(&presenter, active);
            }
        }
    }

    fn set_presenter_input(presenter: &Ref<Control>, active: bool) {
        presenter.set_is_hit_test_visible(active);
        if active {
            presenter.clear_value(KeyboardNavigation::tab_navigation_property());
        } else {
            KeyboardNavigation::set_tab_navigation(presenter, KeyboardNavigationMode::None);
        }
    }

    /// WinUI's `ChangeViews`, after `IsZoomedInViewActive` changed.
    fn change_views(&self) {
        if !self.can_change_views() {
            return;
        }
        self.is_processing_view_change.set(true);
        let changing_to_zoomed_out = !self.is_zoomed_in_view_active();
        let (source, destination) = if changing_to_zoomed_out {
            (self.zoomed_in_view(), self.zoomed_out_view())
        } else {
            (self.zoomed_out_view(), self.zoomed_in_view())
        };
        let (source_part, destination_part) = if changing_to_zoomed_out {
            (self.zoomed_in_presenter(), self.zoomed_out_presenter())
        } else {
            (self.zoomed_out_presenter(), self.zoomed_in_presenter())
        };
        let source_view = source.as_ref().and_then(|view| as_semantic_zoom_information(view));
        let destination_view = destination.as_ref().and_then(|view| as_semantic_zoom_information(view));
        let pending = self.is_pending_view_change.get();

        if self.phase.get() == Phase::ApiSwitchingViews && !self.called_initialize_view_change.get() {
            if let Some(view) = source_view {
                view.initialize_view_change();
            }
            if let Some(view) = destination_view {
                view.initialize_view_change();
            }
            self.called_initialize_view_change.set(true);
        }

        let source_item = SemanticZoomLocation::new();
        let destination_item = SemanticZoomLocation::new();
        if let (Some(zoomed_in), Some(zoomed_out), false, true) =
            (self.zoomed_in_view(), self.zoomed_out_view(), pending, self.emulating_gesture.get())
        {
            // Ctrl+wheel: the point under the pointer in the coordinates of each view.
            let point = self.zoom_point.get();
            let in_view = self.translate_point(point, &zoomed_in).unwrap_or(point);
            let out_view = self.translate_point(point, &zoomed_out).unwrap_or(point);
            let (source_point, destination_point) =
                if changing_to_zoomed_out { (in_view, out_view) } else { (out_view, in_view) };
            source_item.set_zoom_point(source_point);
            destination_item.set_zoom_point(destination_point);
        }

        if let Some(view) = source_view {
            view.start_view_change_from(&source_item, &destination_item);
        }
        if !pending {
            // The destination starts where the source item is.
            let bounds = source_item.bounds();
            let bounds = match (&source_part, &destination_part) {
                (Some(source_part), Some(destination_part)) => {
                    let corner = source_part.translate_point(Point::new(bounds.x, bounds.y), destination_part);
                    corner.map_or(bounds, |corner| Rect::new(corner.x, corner.y, bounds.width, bounds.height))
                }
                _ => bounds,
            };
            destination_item.set_bounds(bounds);
        }
        if let Some(view) = destination_view {
            view.start_view_change_to(&source_item, &destination_item);
        }

        let args = SemanticZoomViewChangedEventArgs::new(changing_to_zoomed_out, source_item, destination_item);
        self.raise_view_changed(&self.view_change_started, &args);

        if let Some(view) = source_view {
            view.set_is_active_view(false);
        }
        if let Some(view) = destination_view {
            view.set_is_active_view(true);
        }
        // The destination scrolls to its item as the zoom starts.
        let destination_item = args.destination_item();
        if let Some(part) = &destination_part {
            Self::set_presenter_input(part, true);
        }
        if let Some(view) = destination_view {
            view.make_visible(&destination_item);
        }
        if let Some(part) = &source_part {
            Self::set_presenter_input(part, false);
        }
        self.hide_zoom_out_button();
        self.update_visual_state();

        *self.completed_args.borrow_mut() = Some(args.clone());
        let (source_item, destination_item) = (args.source_item(), args.destination_item());
        if let Some(view) = source_view {
            view.complete_view_change_from(&source_item, &destination_item);
        }
        if let Some(view) = destination_view {
            view.complete_view_change_to(&source_item, &destination_item);
        }
        args.set_source_item(source_item);
        args.set_destination_item(destination_item);

        let animated = !pending
            && self.phase.get() == Phase::ApiSwitchingViews
            && match (&source_part, &destination_part) {
                (Some(source_part), Some(destination_part)) => {
                    let weak = self.to_ref().downgrade();
                    self.transition.start(source_part, destination_part, changing_to_zoomed_out, move || {
                        if let Some(owner) = weak.upgrade() {
                            owner.view_change_animation_finished();
                        }
                    })
                }
                _ => false,
            };
        if !animated {
            self.view_change_animation_finished();
        }
    }

    /// WinUI's `ViewChangeAnimationFinished` (and the end of `ChangeViews` without an animation):
    /// the presenters at rest, `ViewChangeCompleted`, and `CompleteViewChange` of both views.
    fn view_change_animation_finished(&self) {
        self.phase.set(Phase::Idle);
        self.show_active_presenter();
        let Some(args) = self.completed_args.borrow_mut().take() else { return };
        self.raise_view_changed(&self.view_change_completed, &args);
        self.is_processing_view_change.set(false);
        if self.called_initialize_view_change.replace(false) {
            let (source, destination) = if self.is_zoomed_in_view_active() {
                (self.zoomed_out_view(), self.zoomed_in_view())
            } else {
                (self.zoomed_in_view(), self.zoomed_out_view())
            };
            for view in [source, destination].iter().flatten() {
                if let Some(view) = as_semantic_zoom_information(view) {
                    view.complete_view_change();
                }
            }
        }
    }

    fn raise_view_changed(&self, handlers: &HandlerList<ViewChangedHandler>, args: &SemanticZoomViewChangedEventArgs) {
        if handlers.is_empty() {
            return;
        }
        let owner = self.to_ref();
        for (_, handler) in handlers.snapshot().iter() {
            handler(&owner, args);
        }
    }

    /// WinUI's `ShowZoomOutButton`: `ZoomOutButtonVisible`, then, after the delay of the
    /// transition, `ZoomOutButtonHidden`. Showing it again restarts the delay.
    fn show_zoom_out_button(&self) {
        self.pseudo_classes().set(PC_ZOOM_OUT_BUTTON_VISIBLE, true);
        let weak = self.to_ref().downgrade();
        let hide = DispatcherTimer::run_once(
            move || {
                if let Some(owner) = weak.upgrade() {
                    owner.pseudo_classes().set(PC_ZOOM_OUT_BUTTON_VISIBLE, false);
                }
            },
            ZOOM_OUT_BUTTON_HIDE_DELAY,
            DispatcherPriority::DEFAULT,
        );
        *self.zoom_out_button_hide.borrow_mut() = Some(hide);
    }

    /// WinUI's `HideZoomOutButton` without transitions.
    fn hide_zoom_out_button(&self) {
        self.zoom_out_button_hide.borrow_mut().take();
        self.pseudo_classes().set(PC_ZOOM_OUT_BUTTON_VISIBLE, false);
    }

    /// Whether the zoom-out button is in its `ZoomOutButtonVisible` state.
    pub fn is_zoom_out_button_visible(&self) -> bool {
        self.pseudo_classes().contains(PC_ZOOM_OUT_BUTTON_VISIBLE)
    }

    fn detach_zoom_out_button(&self) {
        if let Some((button, token)) = self.zoom_out_button.borrow_mut().take() {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }
}

impl Drop for SemanticZoom {
    fn drop(&mut self) {
        if let Some((button, token)) = self.zoom_out_button.get_mut().take() {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }
}
