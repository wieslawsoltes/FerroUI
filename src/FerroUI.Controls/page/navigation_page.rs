use super::{
    start_async, BarLayoutBehavior, INavigation, ModalPoppedEventArgs, ModalPushedEventArgs, MultiPage, MultiPageImpl,
    NavigatedFromEventArgs, NavigatedToEventArgs, NavigatingFromEventArgs, NavigationEventArgs, NavigationType, Page,
    PageImpl, PageInsertedEventArgs, PageList, PageRemovedEventArgs, SafeAreaPaddingExtensions,
};
use crate::automation::AutomationProperties;
use crate::presenters::ContentPresenter;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{Border, Button, Control, ControlImpl, Panel, PathIcon, SizeChangedEventArgs, ToolTip};
use ferroui_base::animation::IPageTransition;
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, PointerPressedEventArgs,
    SwipeDirection, SwipeGestureEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::{FlowDirection, Geometry, StreamGeometry};
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::threading::{CancellationTokenSource, DispatcherTask};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, AnyValue, AttachedProperty,
    BoxedValue, DirectProperty, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, IntoRef, Nullable, Point, Ref, Size, StaticType, StyledElement,
    StyledElementImpl, StyledProperty, Thickness, TypeInfo, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

const EDGE_GESTURE_WIDTH: f64 = 20.0;

const PC_NAV_BAR_INSET: &str = ":nav-bar-inset";
const PC_NAV_BAR_COMPACT: &str = ":nav-bar-compact";

const ALREADY_HOSTED: &str = "The page is already hosted by this NavigationPage.";

type Subscription = RefCell<Option<Rc<dyn IDisposable>>>;

/// A navigation page that supports simple stack-based navigation.
#[repr(C)]
pub struct NavigationPage {
    base: MultiPage,
    back_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    back_button_default_icon: RefCell<Option<Ref<Control>>>,
    back_button_content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    content_host: RefCell<Option<Ref<Panel>>>,
    page_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    page_back_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    current_transition: RefCell<Option<Rc<CancellationTokenSource>>>,
    /// `None` stands for a completed task.
    last_page_transition_task: RefCell<Option<DispatcherTask<()>>>,
    current_modal_transition: RefCell<Option<Rc<CancellationTokenSource>>>,
    nav_bar: RefCell<Option<(Ref<Border>, RoutedEventHandlerToken)>>,
    nav_bar_shadow: RefCell<Option<Ref<Border>>>,
    is_pop: Cell<bool>,
    has_had_first_page: Cell<bool>,
    effective_bar_layout_behavior: Cell<BarLayoutBehavior>,
    /// The navigation stack, as the reference stack enumerates: the visible
    /// (top) page first, the root page last. It is the value of the `Pages`
    /// property.
    navigation_stack: PageList,
    /// The modal stack: the oldest modal first, the topmost modal last.
    modal_stack: RefCell<Vec<Ref<Page>>>,
    cached_navigation_stack: RefCell<Option<Rc<Vec<Ref<Page>>>>>,
    cached_modal_stack: RefCell<Option<Rc<Vec<Ref<Page>>>>>,
    modal_back_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    modal_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    top_command_bar_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    has_navigation_bar_sub: Subscription,
    has_back_button_sub: Subscription,
    is_back_button_enabled_sub: Subscription,
    bar_layout_behavior_sub: Subscription,
    bar_height_sub: Subscription,
    back_button_content_sub: Subscription,
    restoring_pages_property: Cell<bool>,
    is_navigating: Cell<bool>,
    can_go_back: Cell<bool>,
    is_back_button_effectively_visible: Cell<bool>,
    is_nav_bar_effectively_visible: Cell<bool>,
    effective_bar_height: Cell<f64>,
    is_back_button_effectively_enabled: Cell<bool>,
    // DRAWER-SEAM: `private DrawerPage? _drawerPage` (a weak reference to the drawer page hosting this
    // navigation page, set by `SetDrawerPage`). See `drawer_page_allows_toggle`.
    override_transition: RefCell<Option<Rc<dyn IPageTransition>>>,
    swipe_start_point: Cell<Point>,
    last_swipe_gesture_id: Cell<i32>,
    has_override_transition: Cell<bool>,
    swipe_gesture_handler: Cell<Option<RoutedEventHandlerToken>>,
    pushed: HandlerList<dyn Fn(&NavigationEventArgs)>,
    popped: HandlerList<dyn Fn(&NavigationEventArgs)>,
    popped_to_root: HandlerList<dyn Fn(&NavigationEventArgs)>,
    page_inserted: HandlerList<dyn Fn(&PageInsertedEventArgs)>,
    page_removed: HandlerList<dyn Fn(&PageRemovedEventArgs)>,
    modal_pushed: HandlerList<dyn Fn(&ModalPushedEventArgs)>,
    modal_popped: HandlerList<dyn Fn(&ModalPoppedEventArgs)>,
}

ferro_class!(NavigationPage: MultiPage);

ferro_class_info!(NavigationPage {
    new: NavigationPage::new,
    interfaces: [Rc<dyn INavigation> => navigation_of],
    markup: {
        // XAML-SEAM: the `Content` property depends on `PageTemplate` (markup assignment order).
        content: Content,
        property_attributes: [Content: [DependsOn("PageTemplate")]],
        attributes: [
            TemplatePart("PART_NavigationBar", type(Ref<Border>)),
            TemplatePart("PART_BackButton", type(Ref<Button>)),
            TemplatePart("PART_ContentHost", type(Ref<Panel>)),
            TemplatePart("PART_PagePresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_PageBackPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_BackButtonDefaultIcon", type(Ref<Control>)),
            TemplatePart("PART_BackButtonContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_TopCommandBar", type(Ref<ContentPresenter>)),
            TemplatePart("PART_BottomCommandBar", type(Ref<ContentPresenter>)),
            TemplatePart("PART_ModalBackPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_ModalPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_NavBarShadow", type(Ref<Border>)),
            PseudoClasses(":nav-bar-inset", ":nav-bar-compact"),
        ],
    },
});

ferro_impl_classes!(NavigationPage: InteractiveImpl);

impl FerroObjectImpl for NavigationPage {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // The navigation stack is mutated by the navigation methods only, which maintain the logical
        // children and the active page themselves (the stack of the reference does not notify of changes).
        this.set_observes_pages(false);
        this.set_current_value(MultiPage::pages_property(), Some(this.navigation_stack.clone()));
        let recognizer = SwipeGestureRecognizer::new();
        recognizer.set_can_horizontally_swipe(true);
        recognizer.set_can_vertically_swipe(false);
        this.gesture_recognizers().add(recognizer);

        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e: &PointerPressedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_swipe_pointer_pressed(e);
                }
            },
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            true,
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == MultiPage::pages_property().as_property()
            && !this.restoring_pages_property.get()
            && change
                .get_new_value::<Option<PageList>>()
                .is_none_or(|new_value| !new_value.ptr_eq(&this.navigation_stack))
        {
            {
                struct Restoring<'a>(&'a Cell<bool>);
                impl Drop for Restoring<'_> {
                    fn drop(&mut self) {
                        self.0.set(false);
                    }
                }

                this.restoring_pages_property.set(true);
                let _restoring = Restoring(&this.restoring_pages_property);
                this.set_current_value(MultiPage::pages_property(), Some(this.navigation_stack.clone()));
            }

            panic!(
                "Direct assignment to NavigationPage.Pages is not supported. Use PushAsync, PopAsync, InsertPage, RemovePage, or ReplaceAsync to modify the navigation stack."
            );
        }

        if change.property() == Page::safe_area_padding_property().as_property() {
            this.update_effective_bar_height();
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for NavigationPage {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <NavigationPage as StaticType>::TYPE
    }
}

impl VisualImpl for NavigationPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.restore_navigation_state();

        let weak = this.to_ref().downgrade();
        let token = this.add_handler(InputElement::swipe_gesture_event(), move |_, e: &SwipeGestureEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_swipe_gesture(e);
            }
        });
        if let Some(previous) = this.swipe_gesture_handler.replace(Some(token)) {
            this.remove_handler(InputElement::swipe_gesture_event(), previous);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(token) = this.swipe_gesture_handler.take() {
            this.remove_handler(InputElement::swipe_gesture_event(), token);
        }

        let t = this.current_transition.borrow_mut().take();
        if let Some(t) = t {
            t.cancel();
        }

        let mt = this.current_modal_transition.borrow_mut().take();
        if let Some(mt) = mt {
            mt.cancel();
        }

        this.dispose_page_subscriptions();

        this.clear_navigation_state();
        this.invalidate_navigation_stack_cache();
    }
}

impl LayoutableImpl for NavigationPage {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let top_command_bar_presenter = this.top_command_bar_presenter.borrow().clone();
        if let Some(presenter) = top_command_bar_presenter {
            if presenter.content().is_some() && available_size.width > 0.0 && available_size.width.is_finite() {
                let max_width = (available_size.width * 0.5).floor();
                if presenter.max_width() != max_width {
                    presenter.set_max_width(max_width);
                }
            }
        }

        Self::parent_measure_override(this, available_size)
    }
}

impl InputElementImpl for NavigationPage {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.key == Key::Left && e.key_modifiers == KeyModifiers::ALT && this.stack_depth() > 1 {
            // A key handler cannot be asynchronous; fire-and-forget is intentional.
            drop(this.pop_async());
            e.set_handled(true);
        }
    }
}

impl TemplatedControlImpl for NavigationPage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        this.set_back_button(Some(e.name_scope().get_as::<Button>("PART_BackButton")));
        *this.back_button_default_icon.borrow_mut() = e.name_scope().find_as::<Control>("PART_BackButtonDefaultIcon");
        *this.back_button_content_presenter.borrow_mut() =
            e.name_scope().find_as::<ContentPresenter>("PART_BackButtonContentPresenter");

        *this.content_host.borrow_mut() = e.name_scope().find_as::<Panel>("PART_ContentHost");
        *this.page_presenter.borrow_mut() = e.name_scope().find_as::<ContentPresenter>("PART_PagePresenter");
        *this.page_back_presenter.borrow_mut() = e.name_scope().find_as::<ContentPresenter>("PART_PageBackPresenter");
        let previous_nav_bar = this.nav_bar.borrow_mut().take();
        if let Some((nav_bar, token)) = previous_nav_bar {
            nav_bar.remove_handler(Control::size_changed_event(), token);
        }
        let nav_bar = e.name_scope().get_as::<Border>("PART_NavigationBar");
        let weak = this.to_ref().downgrade();
        let token = nav_bar.size_changed(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.on_nav_bar_size_changed(e);
            }
        });
        *this.nav_bar.borrow_mut() = Some((nav_bar, token));
        *this.nav_bar_shadow.borrow_mut() = e.name_scope().find_as::<Border>("PART_NavBarShadow");
        *this.top_command_bar_presenter.borrow_mut() = e.name_scope().find_as::<ContentPresenter>("PART_TopCommandBar");
        this.update_top_command_bar_max_width();

        *this.modal_back_presenter.borrow_mut() = e.name_scope().find_as::<ContentPresenter>("PART_ModalBackPresenter");

        let modal_presenter = e.name_scope().find_as::<ContentPresenter>("PART_ModalPresenter");
        *this.modal_presenter.borrow_mut() = modal_presenter.clone();
        if let Some(modal_presenter) = modal_presenter {
            modal_presenter.set_content(this.modal_content());
            modal_presenter.set_is_visible(this.is_modal_visible());
        }

        this.restore_navigation_state();

        this.apply_nav_bar_visibility();
        this.apply_back_button_enabled(this.is_back_button_effectively_enabled());
        this.apply_has_shadow();
        this.update_active_page();
    }
}

impl PageImpl for NavigationPage {
    fn update_content_safe_area_padding(this: &Self) {
        let content_host = this.content_host.borrow().clone();
        let nav_bar = this.nav_bar();
        if let (Some(_content_host), Some(nav_bar)) = (content_host, nav_bar) {
            let page_safe_area_padding = this.safe_area_padding();
            let padding = this.padding();
            let is_nav_bar_effectively_visible = this.is_nav_bar_effectively_visible();
            let safe_area_padding = if is_nav_bar_effectively_visible {
                Thickness::new(
                    page_safe_area_padding.left,
                    0.0,
                    page_safe_area_padding.right,
                    page_safe_area_padding.bottom,
                )
            } else {
                page_safe_area_padding
            };
            nav_bar.set_padding(if is_nav_bar_effectively_visible {
                Thickness::new(page_safe_area_padding.left, page_safe_area_padding.top, page_safe_area_padding.right, 0.0)
            } else {
                Thickness::default()
            });

            let page_presenter = this.page_presenter.borrow().clone();
            if let Some(page_presenter) = page_presenter {
                page_presenter.set_padding(padding);
            }
            let page_back_presenter = this.page_back_presenter.borrow().clone();
            if let Some(page_back_presenter) = page_back_presenter {
                page_back_presenter.set_padding(padding);
            }

            if let Some(current_page) = this.current_page() {
                let remaining_safe_area = padding.get_remaining_safe_area_padding(safe_area_padding);
                current_page.set_safe_area_padding(Thickness::new(
                    remaining_safe_area.left,
                    remaining_safe_area.top,
                    remaining_safe_area.right,
                    remaining_safe_area.bottom,
                ));
            }

            for modal in this.modal_stack_top_first() {
                modal.set_safe_area_padding(this.safe_area_padding());
            }
        }
    }
}

impl MultiPageImpl for NavigationPage {
    // The navigation type is intentionally unused; lifecycle events are fired in each navigation method
    // directly and the transition direction is controlled by `is_pop`.
    fn update_active_page_with(this: &Self, _navigation_type: NavigationType) {
        let is_pop = this.is_pop.replace(false);

        let has_override = this.has_override_transition.replace(false);
        let override_transition = this.override_transition.borrow_mut().take();

        let page = this.navigation_stack.snapshot().first().cloned();
        let page_content = page.as_ref().map(|page| Control::boxed(page.clone()));

        let content_host = this.content_host.borrow().clone();
        let page_presenter = this.page_presenter.borrow().clone();
        let page_back_presenter = this.page_back_presenter.borrow().clone();
        if let (Some(_content_host), Some(page_presenter), Some(page_back_presenter)) =
            (content_host, page_presenter, page_back_presenter)
        {
            let resolved_transition = if !this.has_had_first_page.get() {
                None
            } else if has_override {
                override_transition
            } else {
                this.page_transition()
            };

            let current_transition = this.current_transition.borrow_mut().take();
            if let Some(current_transition) = current_transition {
                current_transition.cancel();
            }

            // A previous transition may have been canceled above before its teardown ran, leaving its
            // outgoing page in the back presenter; reconcile the logical tree.
            this.detach_orphaned_page_from_logical_tree(page_back_presenter.content().as_ref());
            page_back_presenter.set_is_visible(false);
            page_back_presenter.set_content(None);
            page_back_presenter.set_render_transform(None);
            page_back_presenter.set_opacity(1.0);

            let stray_cp =
                page.as_ref().and_then(|page| page.parent()).and_then(|parent| parent.cast::<ContentPresenter>());
            if let Some(stray_cp) = stray_cp {
                if !stray_cp.ptr_eq(&page_presenter) && !stray_cp.ptr_eq(&page_back_presenter) {
                    stray_cp.set_content(None);
                }
            }

            if let Some(resolved_transition) = resolved_transition {
                let cancel = Rc::new(CancellationTokenSource::new());
                *this.current_transition.borrow_mut() = Some(cancel.clone());

                let old_presenter = page_presenter;
                let new_presenter = page_back_presenter;

                new_presenter.set_content(page_content.clone());
                new_presenter.set_is_visible(true);

                if is_pop {
                    old_presenter.set_z_index(1);
                    new_presenter.set_z_index(0);
                } else {
                    new_presenter.set_z_index(1);
                    old_presenter.set_z_index(0);
                }

                let task = start_async(Self::run_page_transition_async(
                    this.to_ref(),
                    resolved_transition,
                    old_presenter.clone(),
                    new_presenter.clone(),
                    !is_pop,
                    cancel,
                ));
                *this.last_page_transition_task.borrow_mut() = Some(task);

                *this.page_presenter.borrow_mut() = Some(new_presenter);
                *this.page_back_presenter.borrow_mut() = Some(old_presenter);
            } else {
                let old_page_content = page_presenter.content();
                *this.last_page_transition_task.borrow_mut() = None;

                page_presenter.set_content(page_content.clone());
                page_presenter.set_is_visible(page.is_some());
                page_presenter.set_z_index(0);

                page_back_presenter.set_content(None);
                page_back_presenter.set_is_visible(false);
                page_back_presenter.set_z_index(0);

                this.detach_orphaned_page_from_logical_tree(old_page_content.as_ref());
            }

            if page.is_some() {
                this.has_had_first_page.set(true);
            }
        }

        this.set_current_value(Self::content_property(), page_content);

        this.set_current_value(Page::current_page_property(), page.clone());

        this.dispose_page_subscriptions();

        if let Some(page) = &page {
            let observe = |update: fn(&NavigationPage)| {
                let weak = this.to_ref().downgrade();
                move || {
                    if let Some(this) = weak.upgrade() {
                        update(&this);
                    }
                }
            };
            let object: &FerroObject = page;

            let update = observe(Self::update_is_nav_bar_effectively_visible);
            let subscription =
                FerroObjectExtensions::get_observable(object, &**Self::has_navigation_bar_property())
                    .subscribe_fn(move |_| update());
            *this.has_navigation_bar_sub.borrow_mut() = Some(subscription);

            let update = observe(Self::update_is_back_button_effectively_visible);
            let subscription = FerroObjectExtensions::get_observable(object, &**Self::has_back_button_property())
                .subscribe_fn(move |_| update());
            *this.has_back_button_sub.borrow_mut() = Some(subscription);

            let update = observe(Self::update_is_back_button_effectively_enabled);
            let subscription =
                FerroObjectExtensions::get_observable(object, &**Self::is_back_button_enabled_property())
                    .subscribe_fn(move |_| update());
            *this.is_back_button_enabled_sub.borrow_mut() = Some(subscription);

            let update = observe(Self::update_bar_layout_behavior_effective);
            let subscription =
                FerroObjectExtensions::get_observable(object, &**Self::bar_layout_behavior_property())
                    .subscribe_fn(move |_| update());
            *this.bar_layout_behavior_sub.borrow_mut() = Some(subscription);

            let update = observe(Self::update_effective_bar_height);
            let subscription =
                FerroObjectExtensions::get_observable(object, &**Self::bar_height_override_property())
                    .subscribe_fn(move |_| update());
            *this.bar_height_sub.borrow_mut() = Some(subscription);

            let update = observe(Self::update_back_button_content);
            let subscription =
                FerroObjectExtensions::get_observable(object, &**Self::back_button_content_property())
                    .subscribe_fn(move |_| update());
            *this.back_button_content_sub.borrow_mut() = Some(subscription);
        }

        this.update_is_nav_bar_effectively_visible();
        this.update_bar_layout_behavior_effective();
        this.update_effective_bar_height();

        this.invalidate_navigation_stack_cache();
        this.update_content_safe_area_padding();
        this.update_is_back_button_effectively_visible();
        this.update_is_back_button_effectively_enabled();
        this.update_back_button_content();
    }
}

impl ControlImpl for NavigationPage {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::NavigationPageAutomationPeer::new(this).upcast()
    }
}

/// The interface handle of a navigation page: what the pages it hosts hold
/// as their `Navigation`. It does not keep the navigation page alive (the
/// navigation page owns the pages).
struct NavigationPageNavigation {
    page: WeakRef<NavigationPage>,
    id: *const (),
}

impl NavigationPageNavigation {
    fn page(&self) -> Option<Ref<NavigationPage>> {
        self.page.upgrade()
    }
}

impl INavigation for NavigationPageNavigation {
    fn navigation_stack(&self) -> Rc<Vec<Ref<Page>>> {
        self.page().map(|page| page.navigation_stack()).unwrap_or_default()
    }

    fn modal_stack(&self) -> Rc<Vec<Ref<Page>>> {
        self.page().map(|page| page.modal_stack()).unwrap_or_default()
    }

    fn stack_depth(&self) -> i32 {
        self.page().map_or(0, |page| page.stack_depth())
    }

    fn can_go_back(&self) -> bool {
        self.page().is_some_and(|page| page.can_go_back())
    }

    fn push_async(&self, page: Ref<Page>) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.push_async(page),
            None => super::completed_task(),
        }
    }

    fn push_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.push_async_with_transition(page, transition),
            None => super::completed_task(),
        }
    }

    fn push_async_with_parameter(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.push_async_with_parameter(page, transition, parameter),
            None => super::completed_task(),
        }
    }

    fn pop_async(&self) -> DispatcherTask<Option<Ref<Page>>> {
        match self.page() {
            Some(this) => this.pop_async(),
            None => start_async(async { None }),
        }
    }

    fn pop_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>> {
        match self.page() {
            Some(this) => this.pop_async_with_transition(transition),
            None => start_async(async { None }),
        }
    }

    fn pop_to_root_async(&self) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.pop_to_root_async(),
            None => super::completed_task(),
        }
    }

    fn pop_to_root_async_with_transition(&self, transition: Option<Rc<dyn IPageTransition>>) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.pop_to_root_async_with_transition(transition),
            None => super::completed_task(),
        }
    }

    fn pop_to_page_async(&self, page: Ref<Page>) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.pop_to_page_async(page),
            None => super::completed_task(),
        }
    }

    fn pop_to_page_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.pop_to_page_async_with_transition(page, transition),
            None => super::completed_task(),
        }
    }

    fn replace_async(&self, page: Ref<Page>) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.replace_async(page),
            None => super::completed_task(),
        }
    }

    fn replace_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.replace_async_with_transition(page, transition),
            None => super::completed_task(),
        }
    }

    fn replace_async_with_parameter(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.replace_async_with_parameter(page, transition, parameter),
            None => super::completed_task(),
        }
    }

    fn push_modal_async(&self, page: Ref<Page>) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.push_modal_async(page),
            None => super::completed_task(),
        }
    }

    fn push_modal_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.push_modal_async_with_transition(page, transition),
            None => super::completed_task(),
        }
    }

    fn push_modal_async_with_parameter(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.push_modal_async_with_parameter(page, transition, parameter),
            None => super::completed_task(),
        }
    }

    fn pop_modal_async(&self) -> DispatcherTask<Option<Ref<Page>>> {
        match self.page() {
            Some(this) => this.pop_modal_async(),
            None => start_async(async { None }),
        }
    }

    fn pop_modal_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>> {
        match self.page() {
            Some(this) => this.pop_modal_async_with_transition(transition),
            None => start_async(async { None }),
        }
    }

    fn pop_all_modals_async(&self) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.pop_all_modals_async(),
            None => super::completed_task(),
        }
    }

    fn pop_all_modals_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        match self.page() {
            Some(this) => this.pop_all_modals_async_with_transition(transition),
            None => super::completed_task(),
        }
    }

    fn insert_page(&self, page: Ref<Page>, before: Ref<Page>) {
        if let Some(this) = self.page() {
            this.insert_page(page, before);
        }
    }

    fn remove_page(&self, page: Ref<Page>) {
        if let Some(this) = self.page() {
            this.remove_page(page);
        }
    }

    fn reference_id(&self) -> *const () {
        self.id
    }
}

fn navigation_of(page: Ref<NavigationPage>) -> Rc<dyn INavigation> {
    page.as_navigation()
}

/// How a page transition ended.
enum TransitionOutcome {
    Completed,
    Canceled,
    Faulted,
}

/// Waits for a page transition without re-raising its failure.
struct TransitionOutcomeFuture {
    task: DispatcherTask<()>,
}

impl Future for TransitionOutcomeFuture {
    type Output = TransitionOutcome;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<TransitionOutcome> {
        if self.task.is_faulted() {
            return Poll::Ready(TransitionOutcome::Faulted);
        }

        match Pin::new(&mut self.task).poll(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(TransitionOutcome::Completed),
            Poll::Ready(Err(_)) => Poll::Ready(TransitionOutcome::Canceled),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Clears the navigating flag when a navigation ends, however it ends (the
/// `finally` block of the navigation methods).
struct NavigatingScope(Ref<NavigationPage>);

impl Drop for NavigatingScope {
    fn drop(&mut self) {
        self.0.set_is_navigating(false);
    }
}

/// Clears the transition override when a navigation with an explicit
/// transition ends, however it ends.
struct OverrideTransitionScope(Ref<NavigationPage>);

impl Drop for OverrideTransitionScope {
    fn drop(&mut self) {
        self.0.has_override_transition.set(false);
        *self.0.override_transition.borrow_mut() = None;
    }
}

ferro_properties! {
    impl NavigationPage {
        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<NavigationPage, _>("Content", None)
        }

        pub(crate) fn modal_content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<NavigationPage, _>("ModalContent", None)
        }

        pub(crate) fn is_modal_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationPage, _>("IsModalVisible", false)
        }

        /// Defines the `PageTransition` property.
        pub fn page_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<NavigationPage, _>("PageTransition", None)
        }

        /// Defines the `ModalTransition` property.
        pub fn modal_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<NavigationPage, _>("ModalTransition", None)
        }

        /// Defines the `IsBackButtonEffectivelyVisible` property.
        pub fn is_back_button_effectively_visible_property() -> DirectProperty<NavigationPage, bool> {
            FerroProperty::register_direct::<NavigationPage, _>(
                "IsBackButtonEffectivelyVisible",
                |o| o.is_back_button_effectively_visible(),
                None,
                false,
            )
        }

        /// Defines the `IsNavBarEffectivelyVisible` property.
        pub(crate) fn is_nav_bar_effectively_visible_property() -> DirectProperty<NavigationPage, bool> {
            FerroProperty::register_direct::<NavigationPage, _>(
                "IsNavBarEffectivelyVisible",
                |o| o.is_nav_bar_effectively_visible(),
                None,
                false,
            )
        }

        /// Defines the `BarLayoutBehavior` attached property.
        pub fn bar_layout_behavior_property() -> AttachedProperty<Option<BarLayoutBehavior>> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("BarLayoutBehavior", None)
        }

        /// Defines the `HasShadow` property.
        pub fn has_shadow_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationPage, _>("HasShadow", false)
        }

        /// Defines the `BarHeight` property.
        pub fn bar_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<NavigationPage, _>("BarHeight", 48.0)
        }

        /// Defines the `BarHeightOverride` attached property.
        pub fn bar_height_override_property() -> AttachedProperty<Option<f64>> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("BarHeightOverride", None)
        }

        /// Defines the `EffectiveBarHeight` property.
        pub fn effective_bar_height_property() -> DirectProperty<NavigationPage, f64> {
            FerroProperty::register_direct::<NavigationPage, _>(
                "EffectiveBarHeight",
                |o| o.effective_bar_height(),
                None,
                0.0,
            )
        }

        /// Defines the `BackButtonContent` attached property.
        pub fn back_button_content_property() -> AttachedProperty<Option<BoxedValue>> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("BackButtonContent", None)
        }

        /// Defines the `HasBackButton` attached property.
        pub fn has_back_button_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("HasBackButton", true)
        }

        /// Defines the `IsBackButtonVisible` property.
        pub fn is_back_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationPage, _>("IsBackButtonVisible", true)
        }

        /// Defines the `TopCommandBar` attached property.
        pub fn top_command_bar_property() -> AttachedProperty<Option<Ref<Control>>> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("TopCommandBar", None)
        }

        /// Defines the `BottomCommandBar` attached property.
        pub fn bottom_command_bar_property() -> AttachedProperty<Option<Ref<Control>>> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("BottomCommandBar", None)
        }

        /// Defines the `HasNavigationBar` attached property.
        pub fn has_navigation_bar_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("HasNavigationBar", true)
        }

        /// Defines the `IsGestureEnabled` property.
        pub fn is_gesture_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<NavigationPage, _>("IsGestureEnabled", true)
        }

        /// Defines the `IsNavigating` property.
        pub fn is_navigating_property() -> DirectProperty<NavigationPage, bool> {
            FerroProperty::register_direct::<NavigationPage, _>("IsNavigating", |o| o.is_navigating.get(), None, false)
        }

        /// Defines the `CanGoBack` property.
        pub fn can_go_back_property() -> DirectProperty<NavigationPage, bool> {
            FerroProperty::register_direct::<NavigationPage, _>("CanGoBack", |o| o.can_go_back(), None, false)
        }

        /// Defines the `IsBackButtonEnabled` attached property.
        pub fn is_back_button_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<NavigationPage, Page, _>("IsBackButtonEnabled", true)
        }

        /// Defines the `IsBackButtonEffectivelyEnabled` property.
        fn is_back_button_effectively_enabled_property() -> DirectProperty<NavigationPage, bool> {
            FerroProperty::register_direct::<NavigationPage, _>(
                "IsBackButtonEffectivelyEnabled",
                |o| o.is_back_button_effectively_enabled(),
                None,
                false,
            )
        }
    }
}

impl NavigationPage {
    fn static_constructor() {
        Page::page_navigation_system_back_button_pressed_event().add_class_handler::<NavigationPage>(
            |sender, event_args| {
                if event_args.handled() {
                    return;
                }

                let top_modal = sender.modal_stack.borrow().last().cloned();
                if let Some(top_modal) = top_modal {
                    let forwarded =
                        RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
                    top_modal.raise_event(&forwarded);

                    event_args.set_handled(true);

                    if !forwarded.handled() {
                        drop(sender.pop_modal_async());
                    }

                    return;
                }

                if let Some(current_page) = sender.current_page() {
                    let forwarded =
                        RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
                    current_page.raise_event(&forwarded);

                    if forwarded.handled() {
                        event_args.set_handled(true);
                    }
                }

                if event_args.handled() {
                    return;
                }

                if sender.stack_depth() > 1 {
                    event_args.set_handled(true);
                    drop(sender.pop_async());
                }
            },
        );

        Self::modal_content_property().changed().add_class_handler::<NavigationPage>(|x, e| {
            let modal_presenter = x.modal_presenter.borrow().clone();
            if let Some(modal_presenter) = modal_presenter {
                modal_presenter.set_content(e.get_new_value::<Option<BoxedValue>>());
            }
        });

        Self::is_modal_visible_property().changed().add_class_handler::<NavigationPage>(|x, e| {
            let modal_presenter = x.modal_presenter.borrow().clone();
            if let Some(modal_presenter) = modal_presenter {
                modal_presenter.set_is_visible(e.get_new_value::<bool>());
            }
        });

        Self::is_back_button_visible_property()
            .changed()
            .add_class_handler::<NavigationPage>(|x, _| x.update_is_back_button_effectively_visible());

        Self::bar_height_property()
            .changed()
            .add_class_handler::<NavigationPage>(|x, _| x.update_effective_bar_height());

        Self::has_shadow_property().changed().add_class_handler::<NavigationPage>(|x, _| x.apply_has_shadow());

        Self::effective_bar_height_property()
            .changed()
            .add_class_handler::<NavigationPage>(|x, _| x.apply_has_shadow());

        Self::is_nav_bar_effectively_visible_property().changed().add_class_handler::<NavigationPage>(|x, _| {
            x.apply_nav_bar_visibility();
            x.apply_has_shadow();
        });

        Self::is_back_button_effectively_enabled_property()
            .changed()
            .add_class_handler::<NavigationPage>(|x, e| x.apply_back_button_enabled(e.get_new_value::<bool>()));

        Self::content_property().changed().add_class_handler::<NavigationPage>(|x, e| {
            let page = e
                .get_new_value::<Option<BoxedValue>>()
                .as_ref()
                .and_then(Control::from_boxed)
                .and_then(|control| control.cast::<Page>());
            let Some(page) = page else {
                return;
            };
            if x.stack_depth() > 0 {
                return;
            }
            // A property-changed handler cannot be asynchronous; fire-and-forget is intentional.
            drop(x.push_async(page));
        });
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: MultiPage::construct(),
            back_button: RefCell::new(None),
            back_button_default_icon: RefCell::new(None),
            back_button_content_presenter: RefCell::new(None),
            content_host: RefCell::new(None),
            page_presenter: RefCell::new(None),
            page_back_presenter: RefCell::new(None),
            current_transition: RefCell::new(None),
            last_page_transition_task: RefCell::new(None),
            current_modal_transition: RefCell::new(None),
            nav_bar: RefCell::new(None),
            nav_bar_shadow: RefCell::new(None),
            is_pop: Cell::new(false),
            has_had_first_page: Cell::new(false),
            effective_bar_layout_behavior: Cell::new(BarLayoutBehavior::Inset),
            navigation_stack: PageList::new(),
            modal_stack: RefCell::new(Vec::new()),
            cached_navigation_stack: RefCell::new(None),
            cached_modal_stack: RefCell::new(None),
            modal_back_presenter: RefCell::new(None),
            modal_presenter: RefCell::new(None),
            top_command_bar_presenter: RefCell::new(None),
            has_navigation_bar_sub: RefCell::new(None),
            has_back_button_sub: RefCell::new(None),
            is_back_button_enabled_sub: RefCell::new(None),
            bar_layout_behavior_sub: RefCell::new(None),
            bar_height_sub: RefCell::new(None),
            back_button_content_sub: RefCell::new(None),
            restoring_pages_property: Cell::new(false),
            is_navigating: Cell::new(false),
            can_go_back: Cell::new(false),
            is_back_button_effectively_visible: Cell::new(false),
            is_nav_bar_effectively_visible: Cell::new(false),
            effective_bar_height: Cell::new(0.0),
            is_back_button_effectively_enabled: Cell::new(false),
            override_transition: RefCell::new(None),
            swipe_start_point: Cell::new(Point::default()),
            last_swipe_gesture_id: Cell::new(0),
            has_override_transition: Cell::new(false),
            swipe_gesture_handler: Cell::new(None),
            pushed: HandlerList::new(),
            popped: HandlerList::new(),
            popped_to_root: HandlerList::new(),
            page_inserted: HandlerList::new(),
            page_removed: HandlerList::new(),
            modal_pushed: HandlerList::new(),
            modal_popped: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the navigation page.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The navigation page as the navigation service of the pages it hosts.
    pub fn as_navigation(&self) -> Rc<dyn INavigation> {
        let object: &FerroObject = self;
        Rc::new(NavigationPageNavigation {
            page: self.to_ref().downgrade(),
            id: object as *const FerroObject as *const (),
        })
    }

    fn is_rtl(&self) -> bool {
        self.flow_direction() == FlowDirection::RightToLeft
    }

    /// The transition used when pushing or popping pages.
    pub fn page_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::page_transition_property())
    }

    pub fn set_page_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::page_transition_property(), value)
    }

    /// The transition used when presenting or dismissing modal pages.
    pub fn modal_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::modal_transition_property())
    }

    pub fn set_modal_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::modal_transition_property(), value)
    }

    pub(crate) fn modal_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::modal_content_property())
    }

    #[allow(dead_code)]
    pub(crate) fn set_modal_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::modal_content_property(), value)
    }

    pub(crate) fn is_modal_visible(&self) -> bool {
        self.get_value(Self::is_modal_visible_property())
    }

    #[allow(dead_code)]
    pub(crate) fn set_is_modal_visible(&self, value: bool) {
        self.set_value(Self::is_modal_visible_property(), value)
    }

    /// The effective back-button visibility.
    pub fn is_back_button_effectively_visible(&self) -> bool {
        self.is_back_button_effectively_visible.get()
    }

    fn set_is_back_button_effectively_visible(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_back_button_effectively_visible_property(),
            &self.is_back_button_effectively_visible,
            value,
        );
    }

    /// The root page of the navigation stack.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// The effective navigation bar visibility.
    pub(crate) fn is_nav_bar_effectively_visible(&self) -> bool {
        self.is_nav_bar_effectively_visible.get()
    }

    fn set_is_nav_bar_effectively_visible(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_nav_bar_effectively_visible_property(),
            &self.is_nav_bar_effectively_visible,
            value,
        );
    }

    /// Whether the navigation bar has a shadow.
    pub fn has_shadow(&self) -> bool {
        self.get_value(Self::has_shadow_property())
    }

    pub fn set_has_shadow(&self, value: bool) {
        self.set_value(Self::has_shadow_property(), value)
    }

    /// The height of the navigation bar.
    pub fn bar_height(&self) -> f64 {
        self.get_value(Self::bar_height_property())
    }

    pub fn set_bar_height(&self, value: f64) {
        self.set_value(Self::bar_height_property(), value)
    }

    /// The effective navigation bar height.
    pub fn effective_bar_height(&self) -> f64 {
        self.effective_bar_height.get()
    }

    fn set_effective_bar_height(&self, value: f64) {
        self.set_and_raise_cell(Self::effective_bar_height_property(), &self.effective_bar_height, value);
    }

    /// Whether the back button is globally visible for this navigation
    /// page.
    pub fn is_back_button_visible(&self) -> bool {
        self.get_value(Self::is_back_button_visible_property())
    }

    pub fn set_is_back_button_visible(&self, value: bool) {
        self.set_value(Self::is_back_button_visible_property(), value)
    }

    /// Whether edge-swipe gestures can be used to navigate back.
    pub fn is_gesture_enabled(&self) -> bool {
        self.get_value(Self::is_gesture_enabled_property())
    }

    pub fn set_is_gesture_enabled(&self, value: bool) {
        self.set_value(Self::is_gesture_enabled_property(), value)
    }

    /// Whether a navigation operation is currently in progress.
    pub fn is_navigating(&self) -> bool {
        self.is_navigating.get()
    }

    fn set_is_navigating(&self, value: bool) {
        self.set_and_raise_cell(Self::is_navigating_property(), &self.is_navigating, value);
    }

    /// Whether the navigation stack has more than one entry.
    pub fn can_go_back(&self) -> bool {
        self.can_go_back.get()
    }

    fn is_back_button_effectively_enabled(&self) -> bool {
        self.is_back_button_effectively_enabled.get()
    }

    fn set_is_back_button_effectively_enabled(&self, value: bool) {
        self.set_and_raise_cell(
            Self::is_back_button_effectively_enabled_property(),
            &self.is_back_button_effectively_enabled,
            value,
        );
    }

    /// The current navigation stack as a read-only list: the root page
    /// first, the visible page last. The same list is returned until the
    /// stack changes.
    pub fn navigation_stack(&self) -> Rc<Vec<Ref<Page>>> {
        if let Some(cached) = self.cached_navigation_stack.borrow().as_ref() {
            return cached.clone();
        }

        let result: Rc<Vec<Ref<Page>>> = Rc::new(self.navigation_stack.snapshot().iter().rev().cloned().collect());
        *self.cached_navigation_stack.borrow_mut() = Some(result.clone());

        result
    }

    /// The current modal stack. Index 0 is the oldest (bottom-most) modal;
    /// the last index is the most recently pushed (topmost) modal.
    pub fn modal_stack(&self) -> Rc<Vec<Ref<Page>>> {
        if let Some(cached) = self.cached_modal_stack.borrow().as_ref() {
            return cached.clone();
        }

        let result = Rc::new(self.modal_stack.borrow().clone());
        *self.cached_modal_stack.borrow_mut() = Some(result.clone());
        result
    }

    /// The number of pages in the navigation stack.
    pub fn stack_depth(&self) -> i32 {
        self.navigation_stack.count() as i32
    }

    /// Gets the custom back-button content for the specified page.
    pub fn get_back_button_content(page: &Page) -> Option<BoxedValue> {
        page.get_value(&**Self::back_button_content_property())
    }

    /// Sets custom content for the back button on the specified page.
    pub fn set_back_button_content(page: &Page, content: Option<BoxedValue>) {
        page.set_value(&**Self::back_button_content_property(), content)
    }

    /// Gets whether the back button is visible for the specified page.
    pub fn get_has_back_button(page: &Page) -> bool {
        page.get_value(&**Self::has_back_button_property())
    }

    /// Sets whether the back button is visible for the specified page.
    pub fn set_has_back_button(page: &Page, value: bool) {
        page.set_value(&**Self::has_back_button_property(), value)
    }

    /// Gets the header of the specified page.
    pub fn get_header(page: &Page) -> Option<BoxedValue> {
        page.get_value(Page::header_property())
    }

    /// Sets the header of the specified page.
    pub fn set_header(page: &Page, header: Option<BoxedValue>) {
        page.set_value(Page::header_property(), header)
    }

    /// Gets the top command bar assigned to the specified page.
    pub fn get_top_command_bar(page: &Page) -> Option<Ref<Control>> {
        page.get_value(&**Self::top_command_bar_property())
    }

    /// Sets a top command bar for the specified page.
    pub fn set_top_command_bar(page: &Page, command_bar: impl Into<Nullable<Control>>) {
        page.set_value(&**Self::top_command_bar_property(), command_bar.into().0)
    }

    /// Gets the bottom command bar assigned to the specified page.
    pub fn get_bottom_command_bar(page: &Page) -> Option<Ref<Control>> {
        page.get_value(&**Self::bottom_command_bar_property())
    }

    /// Sets a bottom command bar for the specified page.
    pub fn set_bottom_command_bar(page: &Page, command_bar: impl Into<Nullable<Control>>) {
        page.set_value(&**Self::bottom_command_bar_property(), command_bar.into().0)
    }

    /// Gets whether the navigation bar is visible for the specified page.
    pub fn get_has_navigation_bar(page: &Page) -> bool {
        page.get_value(&**Self::has_navigation_bar_property())
    }

    /// Sets whether the navigation bar is visible for the specified page.
    pub fn set_has_navigation_bar(page: &Page, value: bool) {
        page.set_value(&**Self::has_navigation_bar_property(), value)
    }

    /// Gets the bar layout behavior for the specified page.
    pub fn get_bar_layout_behavior(page: &Page) -> Option<BarLayoutBehavior> {
        page.get_value(&**Self::bar_layout_behavior_property())
    }

    /// Sets the bar layout behavior for the specified page.
    pub fn set_bar_layout_behavior(page: &Page, value: Option<BarLayoutBehavior>) {
        page.set_value(&**Self::bar_layout_behavior_property(), value)
    }

    /// Gets the per-page navigation bar height override for the specified
    /// page.
    pub fn get_bar_height_override(page: &Page) -> Option<f64> {
        page.get_value(&**Self::bar_height_override_property())
    }

    /// Sets the per-page navigation bar height override.
    pub fn set_bar_height_override(page: &Page, value: Option<f64>) {
        page.set_value(&**Self::bar_height_override_property(), value)
    }

    /// Gets whether the back button is enabled for the specified page.
    pub fn get_is_back_button_enabled(page: &Page) -> bool {
        page.get_value(&**Self::is_back_button_enabled_property())
    }

    /// Sets whether the back button is enabled for the specified page.
    pub fn set_is_back_button_enabled(page: &Page, value: bool) {
        page.set_value(&**Self::is_back_button_enabled_property(), value)
    }

    fn subscribe<F: ?Sized + 'static>(
        &self,
        select: fn(&NavigationPage) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }

    /// Occurs when a page is pushed onto the navigation stack. Disposing
    /// the returned handle unsubscribes.
    pub fn pushed(&self, handler: impl Fn(&NavigationEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&NavigationEventArgs)>(|x| &x.pushed, Rc::new(handler))
    }

    /// Occurs when a page is popped from the navigation stack. Disposing
    /// the returned handle unsubscribes.
    pub fn popped(&self, handler: impl Fn(&NavigationEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&NavigationEventArgs)>(|x| &x.popped, Rc::new(handler))
    }

    /// Occurs when the stack is popped to root. Disposing the returned
    /// handle unsubscribes.
    ///
    /// The page of the args is the root page that is now current, not any
    /// of the pages that were popped. To observe each popped page
    /// individually, subscribe to [`popped`](Self::popped).
    pub fn popped_to_root(&self, handler: impl Fn(&NavigationEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&NavigationEventArgs)>(|x| &x.popped_to_root, Rc::new(handler))
    }

    /// Occurs when a page has been inserted into the navigation stack.
    /// Disposing the returned handle unsubscribes.
    pub fn page_inserted(&self, handler: impl Fn(&PageInsertedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&PageInsertedEventArgs)>(|x| &x.page_inserted, Rc::new(handler))
    }

    /// Occurs when a page has been removed from the navigation stack.
    /// Disposing the returned handle unsubscribes.
    pub fn page_removed(&self, handler: impl Fn(&PageRemovedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&PageRemovedEventArgs)>(|x| &x.page_removed, Rc::new(handler))
    }

    /// Occurs when a modal page is pushed. Disposing the returned handle
    /// unsubscribes.
    pub fn modal_pushed(&self, handler: impl Fn(&ModalPushedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&ModalPushedEventArgs)>(|x| &x.modal_pushed, Rc::new(handler))
    }

    /// Occurs when a modal page is popped. Disposing the returned handle
    /// unsubscribes.
    pub fn modal_popped(&self, handler: impl Fn(&ModalPoppedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe::<dyn Fn(&ModalPoppedEventArgs)>(|x| &x.modal_popped, Rc::new(handler))
    }

    fn raise<A>(handlers: &HandlerList<dyn Fn(&A)>, args: &A) {
        for (_, handler) in handlers.snapshot().iter() {
            handler(args);
        }
    }

    fn back_button(&self) -> Option<Ref<Button>> {
        self.back_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn set_back_button(&self, value: Option<Ref<Button>>) {
        let previous = self.back_button.borrow_mut().take();
        if let Some((button, token)) = previous {
            button.remove_handler(Button::click_event(), token);
        }
        if let Some(button) = value {
            let weak = self.to_ref().downgrade();
            let token = button.click(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.back_button_clicked(e);
                }
            });
            *self.back_button.borrow_mut() = Some((button, token));
        }
    }

    fn nav_bar(&self) -> Option<Ref<Border>> {
        self.nav_bar.borrow().as_ref().map(|(nav_bar, _)| nav_bar.clone())
    }

    /// The modal stack, the topmost modal first.
    fn modal_stack_top_first(&self) -> Vec<Ref<Page>> {
        self.modal_stack.borrow().iter().rev().cloned().collect()
    }

    fn top_modal(&self) -> Option<Ref<Page>> {
        self.modal_stack.borrow().last().cloned()
    }

    /// Whether the page is in the navigation stack (the page set of the
    /// reference, which always mirrors the stack).
    fn page_set_contains(&self, page: &Page) -> bool {
        self.navigation_stack.snapshot().iter().any(|p| std::ptr::eq::<Page>(&**p, page))
    }

    /// Removes and returns the top of the navigation stack.
    fn pop_navigation_stack(&self) -> Option<Ref<Page>> {
        let top = self.navigation_stack.try_get(0)?;
        self.navigation_stack.remove_at(0);
        Some(top)
    }

    fn is_top_of_navigation_stack(&self, page: &Page) -> bool {
        self.navigation_stack.snapshot().first().is_some_and(|top| std::ptr::eq::<Page>(&**top, page))
    }

    fn dispose_page_subscriptions(&self) {
        for subscription in [
            &self.has_navigation_bar_sub,
            &self.has_back_button_sub,
            &self.is_back_button_enabled_sub,
            &self.bar_layout_behavior_sub,
            &self.bar_height_sub,
            &self.back_button_content_sub,
        ] {
            let subscription = subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }

    fn on_nav_bar_size_changed(&self, _e: &SizeChangedEventArgs) {
        self.update_top_command_bar_max_width();
    }

    fn update_top_command_bar_max_width(&self) {
        let top_command_bar_presenter = self.top_command_bar_presenter.borrow().clone();
        let (Some(presenter), Some(nav_bar)) = (top_command_bar_presenter, self.nav_bar()) else {
            return;
        };
        if presenter.content().is_none() {
            return;
        }
        let nav_bar_width = nav_bar.bounds().width;
        if nav_bar_width <= 0.0 {
            return;
        }
        let max_width = (nav_bar_width * 0.5).floor();
        if presenter.max_width() != max_width {
            presenter.set_max_width(max_width);
        }
    }

    /// Whether a drawer page hosts this navigation page and lets the back
    /// button toggle its drawer.
    // DRAWER-SEAM: upstream evaluates, in every caller, `_drawerPage != null &&
    // _drawerPage.DrawerBehavior != DrawerBehavior.Locked && _drawerPage.DrawerBehavior !=
    // DrawerBehavior.Disabled`. Without the drawer page there is never one.
    fn drawer_page_allows_toggle(&self) -> bool {
        false
    }

    // DRAWER-SEAM: `internal void SetDrawerPage(DrawerPage? drawerPage)`: stores the drawer page in
    // `_drawerPage`, then calls `update_is_back_button_effectively_visible()` and
    // `update_back_button_content()`.

    fn back_button_clicked(&self, _event_args: &RoutedEventArgs) {
        if self.stack_depth() <= 1 && self.drawer_page_allows_toggle() {
            // DRAWER-SEAM: `_drawerPage.IsOpen = !_drawerPage.IsOpen;`
            return;
        }
        if self.can_go_back() {
            drop(self.pop_async());
        }
    }

    /// Returns the page that would become active after a pop.
    fn peek_destination_page(&self) -> Option<Ref<Page>> {
        let stack = self.navigation_stack.snapshot();
        if stack.len() < 2 {
            return None;
        }
        Some(stack[1].clone())
    }

    fn throw_if_page_is_already_present(&self, page: &Page) {
        if self.page_set_contains(page) {
            panic!("{ALREADY_HOSTED}");
        }

        if self.modal_stack.borrow().iter().any(|modal| std::ptr::eq::<Page>(&**modal, page)) {
            panic!("{ALREADY_HOSTED}");
        }
    }

    /// Performs the stack mutation and lifecycle events for a push. The
    /// visual transition runs subsequently via the active page update.
    fn execute_push_core(&self, page: &Ref<Page>, _previous_page: Option<&Ref<Page>>) {
        self.throw_if_page_is_already_present(page);

        self.navigation_stack.insert(0, page.clone());
        self.invalidate_navigation_stack_cache();

        StyledElement::logical_children(self).add(page.clone().upcast());

        page.set_navigation(Some(self.as_navigation()));
        page.set_in_navigation_page(true);

        self.update_active_page();
    }

    /// Performs the stack mutation for a pop. The visual transition runs
    /// subsequently via the active page update. Callers are responsible for
    /// firing lifecycle events via `send_pop_lifecycle_events` after
    /// awaiting the page transition where possible.
    fn execute_pop_core(&self) -> Option<Ref<Page>> {
        let old = self.pop_navigation_stack();

        // Remove the page from the logical tree only if it isn't in a presenter.
        // If it is, it's animating out and will be removed once the animation completes.
        if let Some(old) = &old {
            if !self.is_page_in_presenter(old) {
                StyledElement::logical_children(self).remove(&old.clone().upcast());
            }
        }

        self.invalidate_navigation_stack_cache();
        self.is_pop.set(true);
        self.update_active_page();

        if let Some(old) = &old {
            old.set_navigation(None);
            old.set_in_navigation_page(false);
            old.set_safe_area_padding(Thickness::default());
        }

        old
    }

    async fn push_async_private(this: Ref<Self>, page: Ref<Page>, parameter: Option<BoxedValue>) {
        if this.is_navigating.get() {
            return;
        }

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let previous_page = this.current_page();

        if let Some(previous_page) = &previous_page {
            let navigating_args =
                NavigatingFromEventArgs::with_parameter(Some(page.clone()), NavigationType::Push, parameter.clone());
            Page::send_navigating_async(previous_page.clone(), navigating_args.clone()).await;

            if navigating_args.cancel() {
                return;
            }
        }

        this.execute_push_core(&page, previous_page.as_ref());

        this.await_page_transition_async().await;

        if let Some(previous_page) = &previous_page {
            previous_page.send_navigated_from(&NavigatedFromEventArgs::with_parameter(
                Some(page.clone()),
                NavigationType::Push,
                parameter.clone(),
            ));
        }
        page.send_navigated_to(&NavigatedToEventArgs::with_parameter(
            previous_page,
            NavigationType::Push,
            parameter.clone(),
        ));
        Self::raise(&this.pushed, &NavigationEventArgs::with_parameter(page, NavigationType::Push, parameter));
    }

    fn begin_override_transition(&self, transition: Option<Rc<dyn IPageTransition>>) -> OverrideTransitionScope {
        *self.override_transition.borrow_mut() = transition;
        self.has_override_transition.set(true);
        OverrideTransitionScope(self.to_ref())
    }

    async fn push_async_private_with_transition(
        this: Ref<Self>,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) {
        if this.is_navigating.get() {
            return;
        }
        let _override = this.begin_override_transition(transition);
        Self::push_async_private(this.clone(), page, parameter).await;
    }

    /// Pushes `page` onto the navigation stack asynchronously using the
    /// page transition of the navigation page.
    ///
    /// If a navigation transition is already in progress, the call returns
    /// immediately without pushing the page and without raising any events.
    /// If a `Navigating` handler of the outgoing page cancels, the push is
    /// silently aborted: the stack is not modified, no events are raised,
    /// and nothing panics.
    pub fn push_async(&self, page: impl IntoRef<Page>) -> DispatcherTask<()> {
        start_async(Self::push_async_private(self.to_ref(), page.into_ref(), None))
    }

    /// Pushes `page` onto the navigation stack asynchronously using
    /// `transition`.
    pub fn push_async_with_transition(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        start_async(Self::push_async_private_with_transition(self.to_ref(), page.into_ref(), transition, None))
    }

    /// Pushes `page` onto the navigation stack asynchronously using
    /// `transition`, with an optional `parameter`.
    pub fn push_async_with_parameter(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        start_async(Self::push_async_private_with_transition(self.to_ref(), page.into_ref(), transition, parameter))
    }

    async fn pop_async_core(this: Ref<Self>) -> Option<Ref<Page>> {
        if this.stack_depth() <= 1 {
            return None;
        }
        if this.is_navigating.get() {
            return None;
        }

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let current_page = this.current_page();
        if let Some(current_page) = current_page {
            let destination = this.peek_destination_page();
            let navigating_args = NavigatingFromEventArgs::new(destination, NavigationType::Pop);
            Page::send_navigating_async(current_page, navigating_args.clone()).await;

            if navigating_args.cancel() {
                return None;
            }
        }

        let old = this.execute_pop_core();

        this.await_page_transition_async().await;

        if let Some(old) = &old {
            this.send_pop_lifecycle_events(old, NavigationType::Pop, None);
        }

        old
    }

    /// Pops the top page from the navigation stack asynchronously using the
    /// page transition of the navigation page.
    ///
    /// The result is `None` if the stack has only the root page or if a
    /// navigation transition is already in progress.
    pub fn pop_async(&self) -> DispatcherTask<Option<Ref<Page>>> {
        start_async(Self::pop_async_core(self.to_ref()))
    }

    /// Pops the top page from the navigation stack asynchronously using
    /// `transition`.
    pub fn pop_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>> {
        let this = self.to_ref();
        start_async(async move {
            if this.is_navigating.get() {
                return None;
            }
            let _override = this.begin_override_transition(transition);
            Self::pop_async_core(this.clone()).await
        })
    }

    async fn pop_to_root_async_core(this: Ref<Self>) {
        if this.stack_depth() <= 1 {
            return;
        }
        if this.is_navigating.get() {
            return;
        }

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let navigation_stack = this.navigation_stack();
        let root_page = navigation_stack.first().cloned();

        let current_page = this.current_page();
        if let Some(current_page) = &current_page {
            let navigating_args = NavigatingFromEventArgs::new(root_page.clone(), NavigationType::PopToRoot);
            Page::send_navigating_async(current_page.clone(), navigating_args.clone()).await;
            if navigating_args.cancel() {
                return;
            }
        }

        let mut popped_pages = Vec::new();

        loop {
            let popped = if this.navigation_stack.count() > 1 { this.pop_navigation_stack() } else { None };
            let Some(popped) = popped else {
                break;
            };
            this.tear_down_popped_page(&popped);
            popped_pages.push(popped);
        }

        this.invalidate_navigation_stack_cache();
        this.is_pop.set(true);
        this.update_active_page();

        this.await_page_transition_async().await;

        for popped in &popped_pages {
            popped.send_navigated_from(&NavigatedFromEventArgs::new(root_page.clone(), NavigationType::PopToRoot));
            Self::raise(&this.popped, &NavigationEventArgs::new(popped.clone(), NavigationType::PopToRoot));
        }

        let new_current_page = this.current_page();

        if let Some(new_current_page) = new_current_page {
            new_current_page.send_navigated_to(&NavigatedToEventArgs::new(current_page, NavigationType::PopToRoot));
            Self::raise(
                &this.popped_to_root,
                &NavigationEventArgs::new(new_current_page, NavigationType::PopToRoot),
            );
        }
    }

    /// Pops all pages to the root page using the page transition of the
    /// navigation page.
    ///
    /// If a navigation transition is already in progress, the call returns
    /// immediately without modifying the stack and without raising any
    /// events.
    pub fn pop_to_root_async(&self) -> DispatcherTask<()> {
        start_async(Self::pop_to_root_async_core(self.to_ref()))
    }

    fn tear_down_popped_page(&self, popped: &Ref<Page>) {
        // Remove the page from the logical tree only if it isn't in a presenter.
        // If it is, it will be removed once the animation is complete.
        if !self.is_page_in_presenter(popped) {
            StyledElement::logical_children(self).remove(&popped.clone().upcast());
        }

        popped.set_navigation(None);
        popped.set_in_navigation_page(false);
        popped.set_safe_area_padding(Thickness::default());
    }

    /// Pops all pages to the root page using `transition`.
    pub fn pop_to_root_async_with_transition(&self, transition: Option<Rc<dyn IPageTransition>>) -> DispatcherTask<()> {
        let this = self.to_ref();
        start_async(async move {
            if this.is_navigating.get() {
                return;
            }
            let _override = this.begin_override_transition(transition);
            Self::pop_to_root_async_core(this.clone()).await;
        })
    }

    async fn pop_to_page_async_core(this: Ref<Self>, page: Ref<Page>) {
        if !this.page_set_contains(&page) {
            panic!("Page is not in the navigation stack. (Parameter 'page')");
        }

        if this.is_top_of_navigation_stack(&page) {
            return;
        }

        if this.is_navigating.get() {
            return;
        }

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let current_page = this.current_page();
        if let Some(current_page) = &current_page {
            let navigating_args = NavigatingFromEventArgs::new(Some(page.clone()), NavigationType::Pop);
            Page::send_navigating_async(current_page.clone(), navigating_args.clone()).await;
            if navigating_args.cancel() {
                return;
            }
        }

        let mut popped_pages = Vec::new();

        loop {
            let popped = if this.navigation_stack.count() > 1 && !this.is_top_of_navigation_stack(&page) {
                this.pop_navigation_stack()
            } else {
                None
            };
            let Some(popped) = popped else {
                break;
            };
            this.tear_down_popped_page(&popped);
            popped_pages.push(popped);
        }

        this.invalidate_navigation_stack_cache();
        this.is_pop.set(true);
        this.update_active_page();

        this.await_page_transition_async().await;

        for popped in &popped_pages {
            popped.send_navigated_from(&NavigatedFromEventArgs::new(Some(page.clone()), NavigationType::Pop));
            Self::raise(&this.popped, &NavigationEventArgs::new(popped.clone(), NavigationType::Pop));
        }

        let new_current_page = this.current_page();
        if let Some(new_current_page) = new_current_page {
            new_current_page.send_navigated_to(&NavigatedToEventArgs::new(current_page, NavigationType::Pop));
        }
    }

    /// Pops to a specific page in the stack using the page transition of
    /// the navigation page.
    ///
    /// All pages above `page` are removed from the stack. Each removed page
    /// is navigated from with [`NavigationType::Pop`], and the popped event
    /// is raised for each one. The target page is navigated to with
    /// [`NavigationType::Pop`]. If `page` is already the top of the stack
    /// the method returns immediately without raising any events.
    ///
    /// If a navigation transition is already in progress, the call returns
    /// immediately without modifying the stack and without raising any
    /// events.
    ///
    /// The task fails when `page` is not in the navigation stack.
    pub fn pop_to_page_async(&self, page: impl IntoRef<Page>) -> DispatcherTask<()> {
        start_async(Self::pop_to_page_async_core(self.to_ref(), page.into_ref()))
    }

    /// Pops all pages above `page` using `transition`.
    pub fn pop_to_page_async_with_transition(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        let this = self.to_ref();
        let page = page.into_ref();
        start_async(async move {
            if this.is_navigating.get() {
                return;
            }
            let _override = this.begin_override_transition(transition);
            Self::pop_to_page_async_core(this.clone(), page).await;
        })
    }

    /// The modal transition of the navigation that is starting: the
    /// override if there is one, else the modal transition of the
    /// navigation page. Consumes the override.
    fn take_effective_modal_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        let override_transition = self.override_transition.borrow_mut().take();
        if self.has_override_transition.replace(false) {
            override_transition
        } else {
            self.modal_transition()
        }
    }

    /// Cancels the running modal transition and makes a new cancellation
    /// source the current one.
    fn begin_modal_transition(&self) -> Rc<CancellationTokenSource> {
        let previous = self.current_modal_transition.borrow_mut().take();
        if let Some(previous) = previous {
            previous.cancel();
        }
        let cts = Rc::new(CancellationTokenSource::new());
        *self.current_modal_transition.borrow_mut() = Some(cts.clone());
        cts
    }

    fn end_modal_transition(&self, cts: &Rc<CancellationTokenSource>) {
        let mut current = self.current_modal_transition.borrow_mut();
        if current.as_ref().is_some_and(|current| Rc::ptr_eq(current, cts)) {
            *current = None;
        }
    }

    /// Runs a modal transition to its end. A cancelled transition and a
    /// failed one both end normally: lifecycle events still fire afterwards.
    async fn run_modal_transition(
        this: &Ref<Self>,
        transition: &Rc<dyn IPageTransition>,
        from: Option<&Ref<ContentPresenter>>,
        to: Option<&Ref<ContentPresenter>>,
        forward: bool,
        cts: &Rc<CancellationTokenSource>,
    ) -> TransitionOutcome {
        let from: Option<Ref<Visual>> = from.map(|from| from.clone().upcast());
        let to: Option<Ref<Visual>> = to.map(|to| to.clone().upcast());
        let task = transition.start(from.as_ref(), to.as_ref(), forward, cts.token());
        let outcome = TransitionOutcomeFuture { task }.await;
        if let TransitionOutcome::Faulted = outcome {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::CONTROL) {
                logger.log(Some(this as &dyn Any), "Modal transition threw an unhandled exception.");
            }
        }
        outcome
    }

    async fn push_modal_async_private(this: Ref<Self>, page: Ref<Page>, parameter: Option<BoxedValue>) {
        if this.is_navigating.get() {
            return;
        }
        this.throw_if_page_is_already_present(&page);

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let previous_modal = this.top_modal();
        let covered_page = previous_modal.clone().or_else(|| this.current_page());

        if let Some(covered_page) = &covered_page {
            let navigating_args = NavigatingFromEventArgs::with_parameter(
                Some(page.clone()),
                NavigationType::PushModal,
                parameter.clone(),
            );
            Page::send_navigating_async(covered_page.clone(), navigating_args.clone()).await;

            if navigating_args.cancel() {
                return;
            }
        }

        this.modal_stack.borrow_mut().push(page.clone());
        *this.cached_modal_stack.borrow_mut() = None;
        page.set_navigation(Some(this.as_navigation()));
        page.set_in_navigation_page(true);
        page.set_safe_area_padding(this.safe_area_padding());

        let effective_modal_transition = this.take_effective_modal_transition();

        let modal_presenter = this.modal_presenter.borrow().clone();
        if let (Some(modal_presenter), Some(effective_modal_transition)) =
            (modal_presenter, effective_modal_transition)
        {
            let modal_back_presenter = this.modal_back_presenter.borrow().clone();
            if let (Some(previous_modal), Some(modal_back_presenter)) = (&previous_modal, modal_back_presenter) {
                modal_presenter.set_is_visible(false);

                this.set_current_value(Self::modal_content_property(), Some(Control::boxed(page.clone())));

                modal_back_presenter.set_content(Some(Control::boxed(previous_modal.clone())));
                modal_back_presenter.set_is_visible(true);
            } else {
                this.set_current_value(Self::modal_content_property(), Some(Control::boxed(page.clone())));
            }
            let modal_cts = this.begin_modal_transition();
            Self::run_modal_transition(
                &this,
                &effective_modal_transition,
                None,
                Some(&modal_presenter),
                true,
                &modal_cts,
            )
            .await;

            let modal_back_presenter = this.modal_back_presenter.borrow().clone();
            if let Some(modal_back_presenter) = modal_back_presenter {
                modal_back_presenter.set_is_visible(false);
                modal_back_presenter.set_content(None);
            }
            this.end_modal_transition(&modal_cts);
        } else {
            this.set_current_value(Self::modal_content_property(), Some(Control::boxed(page.clone())));
        }

        this.set_current_value(Self::is_modal_visible_property(), true);

        if let Some(covered_page) = &covered_page {
            covered_page.send_navigated_from(&NavigatedFromEventArgs::with_parameter(
                Some(page.clone()),
                NavigationType::PushModal,
                parameter.clone(),
            ));
        }
        page.send_navigated_to(&NavigatedToEventArgs::with_parameter(
            covered_page,
            NavigationType::PushModal,
            parameter,
        ));
        Self::raise(&this.modal_pushed, &ModalPushedEventArgs::new(page));
    }

    async fn push_modal_async_private_with_transition(
        this: Ref<Self>,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) {
        if this.is_navigating.get() {
            return;
        }
        let _override = this.begin_override_transition(transition);
        Self::push_modal_async_private(this.clone(), page, parameter).await;
    }

    /// Pushes a modal page using the modal transition of the navigation
    /// page.
    ///
    /// If a navigation transition is already in progress, the call returns
    /// immediately without pushing the page and without raising any events.
    pub fn push_modal_async(&self, page: impl IntoRef<Page>) -> DispatcherTask<()> {
        start_async(Self::push_modal_async_private(self.to_ref(), page.into_ref(), None))
    }

    /// Pushes `page` as a modal page using `transition`.
    pub fn push_modal_async_with_transition(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        start_async(Self::push_modal_async_private_with_transition(self.to_ref(), page.into_ref(), transition, None))
    }

    /// Pushes `page` as a modal page using `transition`, with an optional
    /// `parameter`.
    pub fn push_modal_async_with_parameter(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        start_async(Self::push_modal_async_private_with_transition(
            self.to_ref(),
            page.into_ref(),
            transition,
            parameter,
        ))
    }

    async fn pop_modal_async_core(this: Ref<Self>) -> Option<Ref<Page>> {
        if this.modal_stack.borrow().is_empty() {
            return None;
        }
        if this.is_navigating.get() {
            return None;
        }

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let modal = this.top_modal()?;
        let revealed_page = {
            let below_top = {
                let stack = this.modal_stack.borrow();
                if stack.len() < 2 {
                    None
                } else {
                    Some(stack[stack.len() - 2].clone())
                }
            };
            match below_top {
                Some(page) => Some(page),
                None => this.current_page(),
            }
        };

        let navigating_args = NavigatingFromEventArgs::new(revealed_page.clone(), NavigationType::PopModal);
        Page::send_navigating_async(modal, navigating_args.clone()).await;

        if navigating_args.cancel() {
            return None;
        }

        let modal = this.modal_stack.borrow_mut().pop()?;
        *this.cached_modal_stack.borrow_mut() = None;

        modal.set_navigation(None);
        modal.set_in_navigation_page(false);

        let effective_modal_transition = this.take_effective_modal_transition();

        let next = this.top_modal();
        let modal_presenter = this.modal_presenter.borrow().clone();
        if let Some(next) = next {
            let next_content = Some(Control::boxed(next));
            if let (Some(modal_presenter), Some(effective_modal_transition)) =
                (modal_presenter, effective_modal_transition)
            {
                let modal_back_presenter = this.modal_back_presenter.borrow().clone();
                if let Some(modal_back_presenter) = modal_back_presenter {
                    modal_back_presenter.set_content(next_content.clone());
                    modal_back_presenter.set_is_visible(true);
                }

                let pop_cts1 = this.begin_modal_transition();
                let outcome = Self::run_modal_transition(
                    &this,
                    &effective_modal_transition,
                    Some(&modal_presenter),
                    None,
                    false,
                    &pop_cts1,
                )
                .await;
                if let TransitionOutcome::Completed = outcome {
                    this.swap_modal_presenters();
                    let modal_back_presenter = this.modal_back_presenter.borrow().clone();
                    if let Some(modal_back_presenter) = modal_back_presenter {
                        modal_back_presenter.set_content(None);
                    }
                }

                this.set_current_value(Self::modal_content_property(), next_content);
                this.end_modal_transition(&pop_cts1);
            } else {
                this.set_current_value(Self::modal_content_property(), next_content);
            }
        } else if let (Some(modal_presenter), Some(effective_modal_transition)) =
            (modal_presenter, effective_modal_transition)
        {
            let pop_cts2 = this.begin_modal_transition();
            Self::run_modal_transition(
                &this,
                &effective_modal_transition,
                Some(&modal_presenter),
                None,
                false,
                &pop_cts2,
            )
            .await;

            this.set_current_value(Self::is_modal_visible_property(), false);
            this.set_current_value(Self::modal_content_property(), None);
            this.end_modal_transition(&pop_cts2);
        } else {
            this.set_current_value(Self::is_modal_visible_property(), false);
            this.set_current_value(Self::modal_content_property(), None);
        }

        modal.send_navigated_from(&NavigatedFromEventArgs::new(revealed_page.clone(), NavigationType::PopModal));
        if let Some(revealed_page) = revealed_page {
            revealed_page.send_navigated_to(&NavigatedToEventArgs::new(Some(modal.clone()), NavigationType::PopModal));
        }

        Self::raise(&this.modal_popped, &ModalPoppedEventArgs::new(modal.clone()));
        Some(modal)
    }

    /// Pops the top modal page using the modal transition of the
    /// navigation page.
    ///
    /// The result is `None` if there are no modal pages or if a navigation
    /// transition is already in progress.
    pub fn pop_modal_async(&self) -> DispatcherTask<Option<Ref<Page>>> {
        start_async(Self::pop_modal_async_core(self.to_ref()))
    }

    /// Pops the top modal page using `transition`.
    pub fn pop_modal_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>> {
        let this = self.to_ref();
        start_async(async move {
            if this.is_navigating.get() {
                return None;
            }
            let _override = this.begin_override_transition(transition);
            Self::pop_modal_async_core(this.clone()).await
        })
    }

    async fn pop_all_modals_async_core(this: Ref<Self>) {
        if this.modal_stack.borrow().is_empty() {
            return;
        }
        if this.is_navigating.get() {
            return;
        }

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let Some(top_modal) = this.top_modal() else {
            return;
        };
        let revealed_page = this.current_page();

        let navigating_args = NavigatingFromEventArgs::new(revealed_page.clone(), NavigationType::PopModal);
        Page::send_navigating_async(top_modal.clone(), navigating_args.clone()).await;

        if navigating_args.cancel() {
            return;
        }

        let effective_modal_transition = this.take_effective_modal_transition();

        let modal_presenter = this.modal_presenter.borrow().clone();
        if let (Some(modal_presenter), Some(effective_modal_transition)) =
            (modal_presenter, effective_modal_transition)
        {
            let all_modals_cts = this.begin_modal_transition();
            Self::run_modal_transition(
                &this,
                &effective_modal_transition,
                Some(&modal_presenter),
                None,
                false,
                &all_modals_cts,
            )
            .await;
            this.end_modal_transition(&all_modals_cts);
        }

        this.set_current_value(Self::modal_content_property(), None);
        this.set_current_value(Self::is_modal_visible_property(), false);

        loop {
            let modal = this.modal_stack.borrow_mut().pop();
            let Some(modal) = modal else {
                break;
            };
            let next_page = this.top_modal().or_else(|| this.current_page());
            modal.send_navigated_from(&NavigatedFromEventArgs::new(next_page, NavigationType::PopModal));
            modal.set_navigation(None);
            modal.set_in_navigation_page(false);
            Self::raise(&this.modal_popped, &ModalPoppedEventArgs::new(modal));
        }
        *this.cached_modal_stack.borrow_mut() = None;

        if let Some(revealed_page) = revealed_page {
            revealed_page.send_navigated_to(&NavigatedToEventArgs::new(Some(top_modal), NavigationType::PopModal));
        }
    }

    /// Pops all modal pages using the modal transition of the navigation
    /// page.
    ///
    /// All modals are dismissed in a single transition rather than
    /// one-by-one, so lifecycle events differ from popping the modals in a
    /// loop:
    ///
    /// - The `Navigating` event is consulted only on the topmost modal.
    ///   Intermediate modals do not receive a cancellation opportunity. If
    ///   the top modal cancels, the entire dismiss is aborted and no modals
    ///   are popped.
    /// - Every dismissed modal is navigated from, in LIFO order.
    /// - Only the current page is navigated to.
    pub fn pop_all_modals_async(&self) -> DispatcherTask<()> {
        start_async(Self::pop_all_modals_async_core(self.to_ref()))
    }

    /// Pops all modal pages using `transition`.
    pub fn pop_all_modals_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        let this = self.to_ref();
        start_async(async move {
            if this.is_navigating.get() {
                return;
            }
            let _override = this.begin_override_transition(transition);
            Self::pop_all_modals_async_core(this.clone()).await;
        })
    }

    /// Removes a page from the navigation stack without animation.
    ///
    /// If a navigation transition is already in progress, this call is a
    /// no-op: the page is not removed and no events are raised.
    pub fn remove_page(&self, page: impl IntoRef<Page>) {
        let page = page.into_ref();
        if self.is_navigating.get() {
            return;
        }

        if self.is_top_of_navigation_stack(&page) {
            let old = self.execute_pop_core();
            if let Some(old) = old {
                let new_current_page = self.current_page();
                old.send_navigated_from(&NavigatedFromEventArgs::new(new_current_page.clone(), NavigationType::Remove));
                if let Some(new_current_page) = new_current_page {
                    new_current_page.send_navigated_to(&NavigatedToEventArgs::new(Some(old), NavigationType::Remove));
                }
            }
            Self::raise(&self.page_removed, &PageRemovedEventArgs::new(page));
            return;
        }

        if !self.page_set_contains(&page) {
            return;
        }

        let index = self.navigation_stack.snapshot().iter().position(|p| p.ptr_eq(&page));
        if let Some(index) = index {
            self.navigation_stack.remove_at(index);
        }

        page.send_navigated_from(&NavigatedFromEventArgs::new(None, NavigationType::Remove));

        page.set_navigation(None);
        page.set_in_navigation_page(false);
        page.set_safe_area_padding(Thickness::default());

        StyledElement::logical_children(self).remove(&page.clone().upcast());

        self.invalidate_navigation_stack_cache();
        self.update_is_back_button_effectively_visible();

        Self::raise(&self.page_removed, &PageRemovedEventArgs::new(page));
    }

    /// Inserts a page into the stack before the specified page.
    ///
    /// If a navigation transition is already in progress, this call is a
    /// no-op: the page is not inserted and no events are raised.
    ///
    /// # Panics
    /// Panics when `before` is not in the navigation stack, or `page` is
    /// already hosted by this navigation page.
    pub fn insert_page(&self, page: impl IntoRef<Page>, before: impl IntoRef<Page>) {
        let page = page.into_ref();
        let before = before.into_ref();
        if self.is_navigating.get() {
            return;
        }

        self.throw_if_page_is_already_present(&page);

        let before_idx = self.navigation_stack.snapshot().iter().position(|p| p.ptr_eq(&before));
        let Some(before_idx) = before_idx else {
            panic!("The 'before' page is not in the navigation stack.");
        };

        // The page goes below `before`: right after it in the top-first order of the stack.
        self.navigation_stack.insert(before_idx + 1, page.clone());

        page.set_navigation(Some(self.as_navigation()));
        page.set_in_navigation_page(true);
        page.set_safe_area_padding(self.safe_area_padding());

        StyledElement::logical_children(self).add(page.clone().upcast());

        self.invalidate_navigation_stack_cache();
        self.update_is_back_button_effectively_visible();

        Self::raise(&self.page_inserted, &PageInsertedEventArgs::new(page, before));
    }

    async fn replace_async_private(this: Ref<Self>, page: Ref<Page>, parameter: Option<BoxedValue>) {
        if this.stack_depth() == 0 {
            Self::push_async_private_with_transition(this.clone(), page, None, parameter).await;
            return;
        }
        if this.current_page().is_some_and(|current| current.ptr_eq(&page)) {
            return;
        }
        if this.is_navigating.get() {
            return;
        }
        this.throw_if_page_is_already_present(&page);

        this.set_is_navigating(true);
        let _navigating = NavigatingScope(this.clone());

        let previous_page = this.current_page();

        if let Some(previous_page) = &previous_page {
            let navigating_args = NavigatingFromEventArgs::with_parameter(
                Some(page.clone()),
                NavigationType::Replace,
                parameter.clone(),
            );
            Page::send_navigating_async(previous_page.clone(), navigating_args.clone()).await;
            if navigating_args.cancel() {
                return;
            }
        }

        this.execute_replace_core(&page, previous_page.as_ref());

        this.await_page_transition_async().await;

        if let Some(previous_page) = &previous_page {
            previous_page.set_navigation(None);
            previous_page.set_in_navigation_page(false);
            previous_page.set_safe_area_padding(Thickness::default());
            previous_page.send_navigated_from(&NavigatedFromEventArgs::with_parameter(
                Some(page.clone()),
                NavigationType::Replace,
                parameter.clone(),
            ));
        }

        page.send_navigated_to(&NavigatedToEventArgs::with_parameter(
            previous_page,
            NavigationType::Replace,
            parameter,
        ));
    }

    async fn replace_async_private_with_transition(
        this: Ref<Self>,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) {
        if this.is_navigating.get() {
            return;
        }
        let _override = this.begin_override_transition(transition);
        Self::replace_async_private(this.clone(), page, parameter).await;
    }

    /// Replaces the top page with `page` using the page transition of the
    /// navigation page.
    ///
    /// If a navigation transition is already in progress, the call returns
    /// immediately without modifying the stack and without raising any
    /// events.
    pub fn replace_async(&self, page: impl IntoRef<Page>) -> DispatcherTask<()> {
        start_async(Self::replace_async_private(self.to_ref(), page.into_ref(), None))
    }

    /// Replaces the top page with `page` using `transition`.
    pub fn replace_async_with_transition(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        start_async(Self::replace_async_private_with_transition(self.to_ref(), page.into_ref(), transition, None))
    }

    /// Replaces the top page with `page` using `transition`, with an
    /// optional `parameter`.
    pub fn replace_async_with_parameter(
        &self,
        page: impl IntoRef<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        start_async(Self::replace_async_private_with_transition(
            self.to_ref(),
            page.into_ref(),
            transition,
            parameter,
        ))
    }

    async fn run_page_transition_async(
        this: Ref<Self>,
        transition: Rc<dyn IPageTransition>,
        from: Ref<ContentPresenter>,
        to: Ref<ContentPresenter>,
        forward: bool,
        ct: Rc<CancellationTokenSource>,
    ) {
        let from_visual: Ref<Visual> = from.clone().upcast();
        let to_visual: Ref<Visual> = to.upcast();
        let task = transition.start(Some(&from_visual), Some(&to_visual), forward, ct.token());
        match (TransitionOutcomeFuture { task }).await {
            TransitionOutcome::Canceled => return,
            TransitionOutcome::Faulted => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::CONTROL) {
                    logger.log(Some(&this as &dyn Any), "Page transition threw an unhandled exception.");
                }
            }
            TransitionOutcome::Completed => {}
        }

        if ct.is_cancellation_requested() {
            return;
        }

        let from_content = from.content();
        from.set_is_visible(false);
        from.set_content(None);
        from.set_render_transform(None);
        from.set_opacity(1.0);
        this.detach_orphaned_page_from_logical_tree(from_content.as_ref());
    }

    /// Awaits the page transition started by the last active page update.
    async fn await_page_transition_async(&self) {
        let task = self.last_page_transition_task.borrow_mut().take();
        if let Some(task) = task {
            // A transition whose task was dropped by a dispatcher shutdown ends the wait as well.
            let _ = task.await;
        }
    }

    /// Fires lifecycle events after a pop: the old page is navigated from,
    /// the new current page is navigated to, and the popped event is
    /// raised.
    fn send_pop_lifecycle_events(
        &self,
        old_page: &Ref<Page>,
        navigation_type: NavigationType,
        parameter: Option<BoxedValue>,
    ) {
        let new_current_page = self.current_page();
        old_page.send_navigated_from(&NavigatedFromEventArgs::with_parameter(
            new_current_page.clone(),
            navigation_type,
            parameter.clone(),
        ));
        if let Some(new_current_page) = new_current_page {
            new_current_page.send_navigated_to(&NavigatedToEventArgs::with_parameter(
                Some(old_page.clone()),
                navigation_type,
                parameter.clone(),
            ));
        }
        Self::raise(
            &self.popped,
            &NavigationEventArgs::with_parameter(old_page.clone(), navigation_type, parameter),
        );
    }

    /// Swaps the top of the navigation stack with `page`.
    fn execute_replace_core(&self, page: &Ref<Page>, _replaced_page: Option<&Ref<Page>>) {
        let removed = self.pop_navigation_stack();

        self.navigation_stack.insert(0, page.clone());
        self.invalidate_navigation_stack_cache();

        // Remove the replaced page from the logical tree only if it isn't in a presenter.
        // If it is, it's animating out and will be removed once the animation completes.
        if let Some(removed) = &removed {
            if !self.is_page_in_presenter(removed) {
                StyledElement::logical_children(self).remove(&removed.clone().upcast());
            }
        }
        StyledElement::logical_children(self).add(page.clone().upcast());

        page.set_navigation(Some(self.as_navigation()));
        page.set_in_navigation_page(true);

        self.update_active_page();
    }

    fn detach_orphaned_page_from_logical_tree(&self, content: Option<&BoxedValue>) {
        let page = content.and_then(Control::from_boxed).and_then(|control| control.cast::<Page>());
        let Some(page) = page else {
            return;
        };
        let this: &StyledElement = self;
        if !self.page_set_contains(&page) && page.parent().is_some_and(|parent| std::ptr::eq(&*parent, this)) {
            StyledElement::logical_children(self).remove(&page.upcast());
        }
    }

    fn is_page_in_presenter(&self, page: &Ref<Page>) -> bool {
        let holds_page = |presenter: &RefCell<Option<Ref<ContentPresenter>>>| {
            let presenter = presenter.borrow().clone();
            presenter
                .and_then(|presenter| presenter.content())
                .as_ref()
                .and_then(Control::from_boxed)
                .is_some_and(|content| content.ptr_eq(page))
        };
        holds_page(&self.page_presenter) || holds_page(&self.page_back_presenter)
    }

    fn swap_modal_presenters(&self) {
        let modal_presenter = self.modal_presenter.borrow().clone();
        let modal_back_presenter = self.modal_back_presenter.borrow().clone();
        let (Some(modal_presenter), Some(modal_back_presenter)) = (modal_presenter, modal_back_presenter) else {
            return;
        };

        let (z_index, back_z_index) = (modal_back_presenter.z_index(), modal_presenter.z_index());
        modal_presenter.set_z_index(z_index);
        modal_back_presenter.set_z_index(back_z_index);

        *self.modal_presenter.borrow_mut() = Some(modal_back_presenter);
        *self.modal_back_presenter.borrow_mut() = Some(modal_presenter);
    }

    fn invalidate_navigation_stack_cache(&self) {
        *self.cached_navigation_stack.borrow_mut() = None;
    }

    fn restore_navigation_state(&self) {
        for page in self.navigation_stack().iter() {
            page.set_navigation(Some(self.as_navigation()));
            page.set_in_navigation_page(true);
        }

        for modal in self.modal_stack_top_first() {
            modal.set_navigation(Some(self.as_navigation()));
            modal.set_in_navigation_page(true);
        }
    }

    fn clear_navigation_state(&self) {
        for modal in self.modal_stack_top_first() {
            modal.set_navigation(None);
            modal.set_in_navigation_page(false);
        }

        for page in self.navigation_stack().iter() {
            page.set_navigation(None);
            page.set_in_navigation_page(false);
        }
    }

    pub(crate) fn update_is_back_button_effectively_visible(&self) {
        let depth = self.stack_depth();

        let show_drawer_toggle = self.drawer_page_allows_toggle();

        let current_page = self.current_page();
        self.set_is_back_button_effectively_visible(
            self.is_back_button_visible()
                && (depth > 1 || show_drawer_toggle)
                && current_page.is_some_and(|current_page| Self::get_has_back_button(&current_page)),
        );

        self.set_and_raise_cell(Self::can_go_back_property(), &self.can_go_back, depth > 1);

        self.update_back_button_accessibility();
    }

    fn update_is_back_button_effectively_enabled(&self) {
        let current_page = self.current_page();
        self.set_is_back_button_effectively_enabled(
            current_page.is_none_or(|current_page| Self::get_is_back_button_enabled(&current_page)),
        );
    }

    fn update_back_button_accessibility(&self) {
        let Some(back_button) = self.back_button() else {
            return;
        };

        let is_drawer_toggle = self.stack_depth() <= 1 && self.drawer_page_allows_toggle();

        let label = if is_drawer_toggle { "Toggle navigation drawer" } else { "Go back" };

        AutomationProperties::set_name(&back_button, Some(label));
        let tip: BoxedValue = Rc::new(label.to_string());
        ToolTip::set_tip(&back_button, Some(tip));
    }

    fn update_back_button_content(&self) {
        let current_page = self.current_page();
        let mut content = current_page.as_ref().and_then(|current_page| Self::get_back_button_content(current_page));

        let show_toggle = current_page.is_some() && self.stack_depth() <= 1 && self.drawer_page_allows_toggle();

        if content.is_none() && show_toggle {
            let icon_data = self.try_find_resource(&ResourceKey::from("NavigationPageMenuIcon"), None).flatten();
            let geometry = icon_data.as_ref().and_then(stream_geometry_of);
            if let Some(geometry) = geometry {
                let icon = PathIcon::new();
                icon.set_data(geometry);
                content = Some(Control::boxed(icon));
            }
        }

        let back_button_default_icon = self.back_button_default_icon.borrow().clone();
        if let Some(back_button_default_icon) = back_button_default_icon {
            back_button_default_icon.set_is_visible(content.is_none());
        }

        let back_button_content_presenter = self.back_button_content_presenter.borrow().clone();
        if let Some(back_button_content_presenter) = back_button_content_presenter {
            back_button_content_presenter.set_content(content.clone());
            back_button_content_presenter.set_is_visible(content.is_some());
        }

        self.update_back_button_accessibility();
    }

    fn on_swipe_gesture(&self, e: &SwipeGestureEventArgs) {
        if !self.is_gesture_enabled()
            || self.stack_depth() <= 1
            || self.is_navigating.get()
            || !self.modal_stack.borrow().is_empty()
            || e.id() == self.last_swipe_gesture_id.get()
        {
            return;
        }

        let in_edge = if self.is_rtl() {
            self.swipe_start_point.get().x >= self.bounds().width - EDGE_GESTURE_WIDTH
        } else {
            self.swipe_start_point.get().x <= EDGE_GESTURE_WIDTH
        };
        if !in_edge {
            return;
        }

        let should_pop = if self.is_rtl() {
            e.swipe_direction() == SwipeDirection::Left
        } else {
            e.swipe_direction() == SwipeDirection::Right
        };
        if should_pop {
            e.set_handled(true);
            self.last_swipe_gesture_id.set(e.id());
            // A gesture handler cannot be asynchronous; fire-and-forget is intentional.
            drop(self.pop_async());
        }
    }

    fn on_swipe_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        self.swipe_start_point.set(e.get_position(Some(self)));
    }

    fn update_is_nav_bar_effectively_visible(&self) {
        let nav_bar_visible = match self.current_page() {
            Some(current_page) => Self::get_has_navigation_bar(&current_page),
            None => self.has_had_first_page.get(),
        };
        self.set_is_nav_bar_effectively_visible(nav_bar_visible);
        self.update_nav_bar_spacer();
    }

    fn update_bar_layout_behavior_effective(&self) {
        self.effective_bar_layout_behavior.set(
            self.current_page()
                .and_then(|current_page| Self::get_bar_layout_behavior(&current_page))
                .unwrap_or(BarLayoutBehavior::Inset),
        );
        self.update_nav_bar_spacer();
    }

    fn update_nav_bar_spacer(&self) {
        self.pseudo_classes().set(
            PC_NAV_BAR_INSET,
            self.is_nav_bar_effectively_visible()
                && self.effective_bar_layout_behavior.get() == BarLayoutBehavior::Inset,
        );
    }

    fn update_effective_bar_height(&self) {
        let content_bar_height = self
            .current_page()
            .and_then(|current_page| Self::get_bar_height_override(&current_page))
            .unwrap_or_else(|| self.bar_height());
        self.set_effective_bar_height(content_bar_height + self.safe_area_padding().top);
        self.pseudo_classes().set(PC_NAV_BAR_COMPACT, content_bar_height < 40.0);

        self.update_content_safe_area_padding();
    }

    fn apply_nav_bar_visibility(&self) {
        if let Some(nav_bar) = self.nav_bar() {
            nav_bar.set_is_visible(self.is_nav_bar_effectively_visible());
        }
    }

    fn apply_back_button_enabled(&self, enabled: bool) {
        if let Some(back_button) = self.back_button() {
            back_button.set_is_enabled(enabled);
        }
    }

    fn apply_has_shadow(&self) {
        let nav_bar_shadow = self.nav_bar_shadow.borrow().clone();
        let Some(nav_bar_shadow) = nav_bar_shadow else {
            return;
        };
        nav_bar_shadow.set_margin(Thickness::new(0.0, self.effective_bar_height(), 0.0, 0.0));
        nav_bar_shadow.set_is_visible(self.has_shadow() && self.is_nav_bar_effectively_visible());
    }
}

/// The stream geometry held by a resource value, if it holds one.
fn stream_geometry_of(value: &BoxedValue) -> Option<Ref<StreamGeometry>> {
    let value: &dyn AnyValue = &**value;
    if let Some(geometry) = value.downcast_ref::<Ref<StreamGeometry>>() {
        return Some(geometry.clone());
    }
    value.downcast_ref::<Ref<Geometry>>().and_then(|geometry| geometry.clone().cast::<StreamGeometry>())
}
