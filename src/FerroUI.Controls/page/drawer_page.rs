use super::safe_area_padding_extensions::SafeAreaPaddingExtensions;
use super::{
    DefaultPageDataTemplate, DrawerBehavior, DrawerClosingEventArgs, DrawerLayoutBehavior, DrawerPlacement,
    NavigatedFromEventArgs, NavigatedToEventArgs, NavigationPage, NavigationType, Page, PageImpl,
};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::documents::TextElement;
use crate::presenters::ContentPresenter;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, ToggleButton};
use crate::templates::IDataTemplate;
use crate::{
    Border, ContentControl, Control, ControlImpl, ControlImplExt, SplitView, SplitViewDisplayMode,
    SplitViewPanePlacement,
};
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyboardNavigationHandler,
    NavigationDirection, PointerPressedEventArgs, SwipeDirection, SwipeGestureEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{FlowDirection, IBrush};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate, BoxedValue,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, TypeInfo, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const EDGE_GESTURE_WIDTH: f64 = 20.0;

const PC_PLACEMENT_RIGHT: &str = ":placement-right";
const PC_PLACEMENT_TOP: &str = ":placement-top";
const PC_PLACEMENT_BOTTOM: &str = ":placement-bottom";
const PC_DETAIL_IS_NAV_PAGE: &str = ":detail-is-navpage";

/// A page that provides a drawer pattern.
#[repr(C)]
pub struct DrawerPage {
    base: Page,
    content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    drawer_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    drawer_header_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    drawer_footer_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    split_view: RefCell<Option<Ref<SplitView>>>,
    top_bar: RefCell<Option<Ref<Border>>>,
    pane_button: RefCell<Option<Ref<ToggleButton>>>,
    backdrop: RefCell<Option<Ref<Border>>>,
    /// The pointer pressed handler of the backdrop, while it is attached.
    backdrop_pointer_pressed: Cell<Option<RoutedEventHandlerToken>>,
    swipe_start_point: Cell<Point>,
    nav_bar_visible_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    suppress_drawer_events: Cell<bool>,
    has_had_first_page: Cell<bool>,
    hide_top_bar: Cell<bool>,
    swipe_recognizer: Ref<SwipeGestureRecognizer>,
    /// The swipe gesture handler, while attached to the visual tree.
    swipe_gesture_handler: Cell<Option<RoutedEventHandlerToken>>,
    /// The pointer pressed handler that records where a swipe starts, while
    /// attached to the visual tree.
    swipe_pointer_pressed_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(DrawerPage: Page);

ferro_class_info!(DrawerPage {
    new: DrawerPage::new,
    markup: {
        // XAML-SEAM: the content properties depend on their templates (markup assignment order).
        content: Content,
        property_attributes: [
            Drawer: [DependsOn("DrawerTemplate")],
            Content: [DependsOn("ContentTemplate")],
            DrawerHeader: [DependsOn("DrawerHeaderTemplate")],
            DrawerFooter: [DependsOn("DrawerFooterTemplate")],
        ],
        attributes: [
            TemplatePart("PART_DrawerPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_DrawerHeader", type(Ref<ContentPresenter>)),
            TemplatePart("PART_DrawerFooter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_ContentPresenter", type(Ref<ContentPresenter>)),
            TemplatePart("PART_SplitView", type(Ref<SplitView>)),
            TemplatePart("PART_TopBar", type(Ref<Border>)),
            TemplatePart("PART_PaneButton", type(Ref<ToggleButton>)),
            TemplatePart("PART_CompactPaneToggle", type(Ref<ToggleButton>)),
            TemplatePart("PART_Backdrop", type(Ref<Border>)),
            PseudoClasses(":placement-right", ":placement-top", ":placement-bottom", ":detail-is-navpage"),
        ],
    },
});

ferro_impl_classes!(DrawerPage: LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for DrawerPage {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.gesture_recognizers().add(this.swipe_recognizer.clone());
        this.update_swipe_recognizer_axes();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        let is_content = property == Self::content_property().as_property();

        if property == Self::drawer_property().as_property() || is_content {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<BoxedValue>>();

            if let Some(old_logical) = old_value.as_ref().and_then(Control::logical_from_boxed) {
                StyledElement::logical_children(this).remove(&old_logical);
            }
            if let Some(new_logical) = new_value.as_ref().and_then(Control::logical_from_boxed) {
                StyledElement::logical_children(this).add(new_logical);
            }
            this.update_active_page();

            if is_content {
                this.has_had_first_page.set(false);
                this.dispose_nav_bar_visible_sub();

                if let Some(old_nav) = navigation_page_of(&old_value) {
                    old_nav.set_drawer_page(None);
                }
                if let Some(new_nav) = navigation_page_of(&new_value) {
                    new_nav.set_drawer_page(Some(this));
                    let subscription = this.observe_nav_bar_visible(&new_nav);
                    *this.nav_bar_visible_sub.borrow_mut() = Some(subscription);
                }
                this.update_detail_nav_bar_visible_pseudo_class();
            }
        } else if property == Self::is_open_property().as_property()
            || property == Self::display_mode_property().as_property()
        {
            this.sync_current_page();
            this.update_backdrop_state();

            if property == Self::is_open_property().as_property() {
                this.update_drawer_focus();

                if !this.suppress_drawer_events.get() {
                    if change.get_new_value::<bool>() {
                        this.raise_event(&RoutedEventArgs::with_event(Self::opened_event()));
                    } else {
                        this.raise_event(&RoutedEventArgs::with_event(Self::closed_event()));
                    }
                }
            }
        } else if property == Self::drawer_background_property().as_property() {
            this.apply_drawer_background();
        } else if property == Self::drawer_behavior_property().as_property()
            || property == Self::drawer_layout_behavior_property().as_property()
            || property == Self::drawer_breakpoint_length_property().as_property()
        {
            this.update_split_view_display_mode();

            if property == Self::drawer_behavior_property().as_property() {
                if let Some(nav) = navigation_page_of(&this.content()) {
                    nav.set_drawer_page(Some(this));
                }
            }
        } else if property == Visual::bounds_property().as_property() && this.drawer_breakpoint_length() > 0.0 {
            this.update_split_view_display_mode();
        } else if property == Self::is_gesture_enabled_property().as_property() {
            this.swipe_recognizer.set_is_enabled(change.get_new_value::<bool>());
        } else if property == Self::drawer_placement_property().as_property() {
            this.update_swipe_recognizer_axes();
            this.update_pane_placement();
            this.update_content_safe_area_padding();
        } else if property == Self::backdrop_brush_property().as_property() {
            this.update_backdrop_state();
        } else if property == Visual::flow_direction_property().as_property() {
            this.update_content_safe_area_padding();
        } else if property == Self::drawer_header_foreground_property().as_property() {
            let presenter = this.drawer_header_presenter.borrow().clone();
            Self::apply_foreground(presenter.as_ref(), change.get_new_value::<Option<Rc<dyn IBrush>>>());
        } else if property == Self::drawer_footer_foreground_property().as_property() {
            let presenter = this.drawer_footer_presenter.borrow().clone();
            Self::apply_foreground(presenter.as_ref(), change.get_new_value::<Option<Rc<dyn IBrush>>>());
        }
    }
}

impl StyledElementImpl for DrawerPage {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <DrawerPage as StaticType>::TYPE
    }
}

impl VisualImpl for DrawerPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let weak = this.to_ref().downgrade();
        let token = this.add_handler(InputElement::swipe_gesture_event(), move |_, e: &SwipeGestureEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_swipe_gesture(e);
            }
        });
        if let Some(previous) = this.swipe_gesture_handler.replace(Some(token)) {
            this.remove_handler(InputElement::swipe_gesture_event(), previous);
        }

        let weak = this.to_ref().downgrade();
        let token = this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e: &PointerPressedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_swipe_pointer_pressed(e);
                }
            },
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            true,
        );
        if let Some(previous) = this.swipe_pointer_pressed_handler.replace(Some(token)) {
            this.remove_handler(InputElement::pointer_pressed_event(), previous);
        }

        this.attach_backdrop_pointer_pressed();
        this.restore_navigation_state();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(token) = this.swipe_gesture_handler.take() {
            this.remove_handler(InputElement::swipe_gesture_event(), token);
        }
        if let Some(token) = this.swipe_pointer_pressed_handler.take() {
            this.remove_handler(InputElement::pointer_pressed_event(), token);
        }

        this.detach_backdrop_pointer_pressed();

        this.clear_navigation_state();
    }
}

impl InputElementImpl for DrawerPage {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.key == Key::Escape
            && this.is_open()
            && this.drawer_behavior() != DrawerBehavior::Locked
            && this.is_overlay_display_mode()
        {
            this.set_current_value(Self::is_open_property(), false);
            e.set_handled(true);
        }
    }
}

impl ControlImpl for DrawerPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        if !this.has_had_first_page.get() {
            if let Some(current_page) = this.current_page() {
                this.has_had_first_page.set(true);
                current_page.send_navigated_to(&NavigatedToEventArgs::new(None, NavigationType::Push));
            }
        }
    }
}

impl TemplatedControlImpl for DrawerPage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        this.detach_backdrop_pointer_pressed();

        let name_scope = e.name_scope();
        *this.content_presenter.borrow_mut() = name_scope.find_as::<ContentPresenter>("PART_ContentPresenter");
        *this.drawer_presenter.borrow_mut() = name_scope.find_as::<ContentPresenter>("PART_DrawerPresenter");
        *this.drawer_header_presenter.borrow_mut() = name_scope.find_as::<ContentPresenter>("PART_DrawerHeader");
        *this.drawer_footer_presenter.borrow_mut() = name_scope.find_as::<ContentPresenter>("PART_DrawerFooter");
        *this.split_view.borrow_mut() = name_scope.find_as::<SplitView>("PART_SplitView");
        *this.top_bar.borrow_mut() = name_scope.find_as::<Border>("PART_TopBar");
        *this.pane_button.borrow_mut() = name_scope.find_as::<ToggleButton>("PART_PaneButton");
        *this.backdrop.borrow_mut() = name_scope.find_as::<Border>("PART_Backdrop");

        let backdrop = this.backdrop.borrow().clone();
        if let Some(backdrop) = backdrop {
            if this.is_attached_to_visual_tree() {
                this.attach_backdrop_pointer_pressed();
            }

            AutomationProperties::set_accessibility_view(&backdrop, AccessibilityView::Raw);
        }

        let drawer_header_presenter = this.drawer_header_presenter.borrow().clone();
        Self::apply_foreground(drawer_header_presenter.as_ref(), this.drawer_header_foreground());
        let drawer_footer_presenter = this.drawer_footer_presenter.borrow().clone();
        Self::apply_foreground(drawer_footer_presenter.as_ref(), this.drawer_footer_foreground());

        this.apply_drawer_background();
        this.update_split_view_display_mode();
        this.update_pane_placement();
        this.update_backdrop_state();
        this.update_content_safe_area_padding();
    }
}

impl PageImpl for DrawerPage {
    fn update_content_safe_area_padding(this: &Self) {
        let content_presenter = this.content_presenter.borrow().clone();
        let drawer_presenter = this.drawer_presenter.borrow().clone();
        let (Some(content_presenter), Some(drawer_presenter)) = (content_presenter, drawer_presenter) else {
            return;
        };

        let sa = this.safe_area_padding();
        let has_drawer_header = this.drawer_header().is_some();
        let has_drawer_footer = this.drawer_footer().is_some();
        let drawer_header_presenter = this.drawer_header_presenter.borrow().clone().filter(|_| has_drawer_header);
        let drawer_footer_presenter = this.drawer_footer_presenter.borrow().clone().filter(|_| has_drawer_footer);
        let placement = this.drawer_placement();

        if this.is_vertical_placement() {
            drawer_presenter.set_padding(if placement == DrawerPlacement::Top {
                Thickness::new(sa.left, if has_drawer_header { 0.0 } else { sa.top }, sa.right, 0.0)
            } else {
                Thickness::new(sa.left, 0.0, sa.right, if has_drawer_footer { 0.0 } else { sa.bottom })
            });

            if let Some(drawer_header_presenter) = drawer_header_presenter {
                drawer_header_presenter.set_padding(if placement == DrawerPlacement::Bottom {
                    Thickness::new(sa.left, 0.0, sa.right, 0.0)
                } else {
                    Thickness::new(sa.left, sa.top, sa.right, 0.0)
                });
            }

            if let Some(drawer_footer_presenter) = drawer_footer_presenter {
                drawer_footer_presenter.set_padding(if placement == DrawerPlacement::Bottom {
                    Thickness::new(sa.left, 0.0, sa.right, sa.bottom)
                } else {
                    Thickness::new(sa.left, 0.0, sa.right, 0.0)
                });
            }
        } else {
            let is_pane_on_right = this.is_pane_on_right();
            let top = if has_drawer_header { 0.0 } else { sa.top };
            let bottom = if has_drawer_footer { 0.0 } else { sa.bottom };

            drawer_presenter.set_padding(if is_pane_on_right {
                Thickness::new(0.0, top, sa.right, bottom)
            } else {
                Thickness::new(sa.left, top, 0.0, bottom)
            });

            if let Some(drawer_header_presenter) = drawer_header_presenter {
                drawer_header_presenter.set_padding(if is_pane_on_right {
                    Thickness::new(0.0, sa.top, sa.right, 0.0)
                } else {
                    Thickness::new(sa.left, sa.top, 0.0, 0.0)
                });
            }

            if let Some(drawer_footer_presenter) = drawer_footer_presenter {
                drawer_footer_presenter.set_padding(if is_pane_on_right {
                    Thickness::new(0.0, 0.0, sa.right, sa.bottom)
                } else {
                    Thickness::new(sa.left, 0.0, 0.0, sa.bottom)
                });
            }
        }

        let top_bar = this.top_bar.borrow().clone();
        if let Some(top_bar) = top_bar {
            top_bar.set_margin(Thickness::new(sa.left, sa.top, sa.right, 0.0));
        }

        content_presenter.set_padding(this.padding());

        if let Some(detail) = page_of(&this.content()) {
            let remaining_safe_area = this.padding().get_remaining_safe_area_padding(this.safe_area_padding());
            detail.set_safe_area_padding(Thickness::new(
                remaining_safe_area.left,
                if this.hide_top_bar.get() { remaining_safe_area.top } else { 0.0 },
                remaining_safe_area.right,
                remaining_safe_area.bottom,
            ));
        }
    }

    fn update_active_page(this: &Self) {
        let previous_page = this.current_page();

        this.set_current_value(Page::current_page_property(), this.resolve_current_page());

        if !same_page(&previous_page, &this.current_page()) && this.visual_root().is_some() {
            this.has_had_first_page.set(true);
            if let Some(previous_page) = &previous_page {
                previous_page
                    .send_navigated_from(&NavigatedFromEventArgs::new(this.current_page(), NavigationType::Replace));
            }
            if let Some(current_page) = this.current_page() {
                current_page.send_navigated_to(&NavigatedToEventArgs::new(previous_page, NavigationType::Replace));
            }
        }

        this.update_content_safe_area_padding();
    }
}

// AUTOMATION-SEAM: OnCreateAutomationPeer -> DrawerPageAutomationPeer (automation pass)

ferro_properties! {
    impl DrawerPage {
        /// Defines the `Drawer` property.
        pub fn drawer_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DrawerPage, _>("Drawer", None)
        }

        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DrawerPage, _>("Content", None)
        }

        /// Defines the `IsOpen` property.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<DrawerPage, _>(
                "IsOpen",
                StyledPropertyOptions::new(false).coerce(DrawerPage::coerce_is_open),
            )
        }

        /// Defines the `DrawerLength` property.
        pub fn drawer_length_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<DrawerPage, _>(
                "DrawerLength",
                StyledPropertyOptions::new(320.0).validate(DrawerPage::validate_length),
            )
        }

        /// Defines the `CompactDrawerLength` property.
        pub fn compact_drawer_length_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<DrawerPage, _>(
                "CompactDrawerLength",
                StyledPropertyOptions::new(48.0).validate(DrawerPage::validate_length),
            )
        }

        /// Defines the `DrawerBreakpointLength` property.
        pub fn drawer_breakpoint_length_property() -> StyledProperty<f64> {
            FerroProperty::register::<DrawerPage, _>("DrawerBreakpointLength", 0.0)
        }

        /// Defines the `IsGestureEnabled` property.
        pub fn is_gesture_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<DrawerPage, _>("IsGestureEnabled", true)
        }

        /// Defines the `DrawerBehavior` property.
        pub fn drawer_behavior_property() -> StyledProperty<DrawerBehavior> {
            FerroProperty::register::<DrawerPage, _>("DrawerBehavior", DrawerBehavior::Auto)
        }

        /// Defines the `DrawerLayoutBehavior` property.
        pub fn drawer_layout_behavior_property() -> StyledProperty<DrawerLayoutBehavior> {
            FerroProperty::register::<DrawerPage, _>("DrawerLayoutBehavior", DrawerLayoutBehavior::Overlay)
        }

        /// Defines the `DrawerPlacement` property.
        pub fn drawer_placement_property() -> StyledProperty<DrawerPlacement> {
            FerroProperty::register::<DrawerPage, _>("DrawerPlacement", DrawerPlacement::Left)
        }

        /// Defines the `DrawerHeader` property.
        pub fn drawer_header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DrawerPage, _>("DrawerHeader", None)
        }

        /// Defines the `DrawerFooter` property.
        pub fn drawer_footer_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DrawerPage, _>("DrawerFooter", None)
        }

        /// Defines the `DrawerHeaderTemplate` property.
        pub fn drawer_header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerHeaderTemplate", None)
        }

        /// Defines the `DrawerFooterTemplate` property.
        pub fn drawer_footer_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerFooterTemplate", None)
        }

        /// Defines the `DrawerIcon` property.
        pub fn drawer_icon_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<DrawerPage, _>("DrawerIcon", None)
        }

        /// Defines the `DrawerIconTemplate` property.
        pub fn drawer_icon_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerIconTemplate", None)
        }

        /// Defines the `DrawerTemplate` property.
        pub fn drawer_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerTemplate", None)
        }

        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            let default_template: Rc<dyn IDataTemplate> = DefaultPageDataTemplate::new();
            FerroProperty::register::<DrawerPage, _>("ContentTemplate", Some(default_template))
        }

        /// Defines the `DrawerBackground` property.
        pub fn drawer_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerBackground", None)
        }

        /// Defines the `DrawerHeaderBackground` property.
        pub fn drawer_header_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerHeaderBackground", None)
        }

        /// Defines the `DrawerHeaderForeground` property.
        pub fn drawer_header_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerHeaderForeground", None)
        }

        /// Defines the `DrawerFooterBackground` property.
        pub fn drawer_footer_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerFooterBackground", None)
        }

        /// Defines the `DrawerFooterForeground` property.
        pub fn drawer_footer_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawerPage, _>("DrawerFooterForeground", None)
        }

        /// Defines the `BackdropBrush` property.
        pub fn backdrop_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawerPage, _>("BackdropBrush", None)
        }

        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<DrawerPage>()
        }

        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<DrawerPage>()
        }

        /// Defines the `DisplayMode` property.
        pub fn display_mode_property() -> StyledProperty<SplitViewDisplayMode> {
            SplitView::display_mode_property().add_owner::<DrawerPage>()
        }
    }
}

impl DrawerPage {
    ferro_routed_event!(
        /// Defines the `Opened` routed event.
        pub fn opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<DrawerPage, _>("Opened", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Closing` routed event.
        pub fn closing_event() -> RoutedEvent<DrawerClosingEventArgs> {
            RoutedEvent::register::<DrawerPage, _>("Closing", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Closed` routed event.
        pub fn closed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<DrawerPage, _>("Closed", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        Page::page_navigation_system_back_button_pressed_event().add_class_handler::<DrawerPage>(
            |sender, event_args| {
                if event_args.handled() {
                    return;
                }

                if sender.is_open()
                    && sender.drawer_behavior() != DrawerBehavior::Locked
                    && sender.drawer_behavior() != DrawerBehavior::Disabled
                {
                    sender.set_is_open(false);
                    event_args.set_handled(true);
                } else {
                    let page_event =
                        RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
                    if let Some(current_page) = sender.current_page() {
                        current_page.raise_event(&page_event);
                    }

                    if page_event.handled() {
                        event_args.set_handled(true);
                    }
                }
            },
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Page::construct(),
            content_presenter: RefCell::new(None),
            drawer_presenter: RefCell::new(None),
            drawer_header_presenter: RefCell::new(None),
            drawer_footer_presenter: RefCell::new(None),
            split_view: RefCell::new(None),
            top_bar: RefCell::new(None),
            pane_button: RefCell::new(None),
            backdrop: RefCell::new(None),
            backdrop_pointer_pressed: Cell::new(None),
            swipe_start_point: Cell::new(Point::default()),
            nav_bar_visible_sub: RefCell::new(None),
            suppress_drawer_events: Cell::new(false),
            has_had_first_page: Cell::new(false),
            hide_top_bar: Cell::new(false),
            swipe_recognizer: SwipeGestureRecognizer::new(),
            swipe_gesture_handler: Cell::new(None),
            swipe_pointer_pressed_handler: Cell::new(None),
        }
    }

    /// Initializes a new instance of the [`DrawerPage`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn is_rtl(&self) -> bool {
        self.flow_direction() == FlowDirection::RightToLeft
    }

    fn is_vertical_placement(&self) -> bool {
        let placement = self.drawer_placement();
        placement == DrawerPlacement::Top || placement == DrawerPlacement::Bottom
    }

    fn is_pane_on_right(&self) -> bool {
        (self.drawer_placement() == DrawerPlacement::Right) != self.is_rtl()
    }

    fn is_overlay_display_mode(&self) -> bool {
        let display_mode = self.display_mode();
        display_mode == SplitViewDisplayMode::Overlay || display_mode == SplitViewDisplayMode::CompactOverlay
    }

    /// Occurs when `IsOpen` changes to true.
    pub fn opened(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::opened_event(), handler)
    }

    /// Occurs when the drawer is about to close.
    pub fn closing(
        &self,
        handler: impl Fn(&Interactive, &DrawerClosingEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::closing_event(), handler)
    }

    /// Occurs when `IsOpen` changes to false and closing is not cancelled.
    pub fn closed(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::closed_event(), handler)
    }

    fn validate_length(value: &f64) -> bool {
        !value.is_nan() && !value.is_infinite() && *value >= 0.0
    }

    fn coerce_is_open(instance: &FerroObject, value: bool) -> bool {
        let Some(drawer) = instance.downcast_ref::<DrawerPage>() else {
            return value;
        };
        if drawer.suppress_drawer_events.get() {
            return value;
        }

        if value && drawer.drawer_behavior() == DrawerBehavior::Disabled {
            return false;
        }

        if !value && drawer.is_open() && drawer.drawer_behavior() != DrawerBehavior::Disabled {
            let args = DrawerClosingEventArgs::with_event(Self::closing_event());
            drawer.raise_event(&args);
            if args.cancel() {
                return true;
            }
        }

        value
    }

    /// Gets or sets the drawer pane content.
    pub fn drawer(&self) -> Option<BoxedValue> {
        self.get_value(Self::drawer_property())
    }

    pub fn set_drawer(&self, value: Option<BoxedValue>) {
        self.set_value(Self::drawer_property(), value)
    }

    /// Gets or sets the main content.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// Gets or sets whether the drawer pane is currently open.
    ///
    /// Setting this property to true while the drawer behavior is
    /// [`DrawerBehavior::Disabled`] is a no-op; the value is coerced back
    /// to false. When closing programmatically, the `Closing` event is
    /// raised and can be cancelled; if cancelled, the property reverts to
    /// true. The `Closing` event is not raised when the drawer is forced
    /// closed because the drawer behavior is set to
    /// [`DrawerBehavior::Disabled`].
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value)
    }

    /// Gets or sets the width of the drawer pane.
    pub fn drawer_length(&self) -> f64 {
        self.get_value(Self::drawer_length_property())
    }

    pub fn set_drawer_length(&self, value: f64) {
        self.set_value(Self::drawer_length_property(), value)
    }

    /// Gets or sets the compact drawer width.
    pub fn compact_drawer_length(&self) -> f64 {
        self.get_value(Self::compact_drawer_length_property())
    }

    pub fn set_compact_drawer_length(&self, value: f64) {
        self.set_value(Self::compact_drawer_length_property(), value)
    }

    /// Gets or sets the size threshold for switching to overlay mode. Set
    /// to 0 to disable.
    pub fn drawer_breakpoint_length(&self) -> f64 {
        self.get_value(Self::drawer_breakpoint_length_property())
    }

    pub fn set_drawer_breakpoint_length(&self, value: f64) {
        self.set_value(Self::drawer_breakpoint_length_property(), value)
    }

    /// Gets or sets whether swipe gestures can open or close the drawer.
    pub fn is_gesture_enabled(&self) -> bool {
        self.get_value(Self::is_gesture_enabled_property())
    }

    pub fn set_is_gesture_enabled(&self, value: bool) {
        self.set_value(Self::is_gesture_enabled_property(), value)
    }

    /// Gets or sets the open/close behavior of the drawer pane.
    pub fn drawer_behavior(&self) -> DrawerBehavior {
        self.get_value(Self::drawer_behavior_property())
    }

    pub fn set_drawer_behavior(&self, value: DrawerBehavior) {
        self.set_value(Self::drawer_behavior_property(), value)
    }

    /// Gets or sets the layout behavior of the drawer.
    pub fn drawer_layout_behavior(&self) -> DrawerLayoutBehavior {
        self.get_value(Self::drawer_layout_behavior_property())
    }

    pub fn set_drawer_layout_behavior(&self, value: DrawerLayoutBehavior) {
        self.set_value(Self::drawer_layout_behavior_property(), value)
    }

    /// Gets or sets which edge of the control the drawer appears from.
    pub fn drawer_placement(&self) -> DrawerPlacement {
        self.get_value(Self::drawer_placement_property())
    }

    pub fn set_drawer_placement(&self, value: DrawerPlacement) {
        self.set_value(Self::drawer_placement_property(), value)
    }

    /// Gets or sets the header content displayed at the top of the drawer
    /// pane.
    pub fn drawer_header(&self) -> Option<BoxedValue> {
        self.get_value(Self::drawer_header_property())
    }

    pub fn set_drawer_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::drawer_header_property(), value)
    }

    /// Gets or sets the footer content displayed at the bottom of the
    /// drawer pane.
    pub fn drawer_footer(&self) -> Option<BoxedValue> {
        self.get_value(Self::drawer_footer_property())
    }

    pub fn set_drawer_footer(&self, value: Option<BoxedValue>) {
        self.set_value(Self::drawer_footer_property(), value)
    }

    /// Gets or sets the data template used to display the drawer header.
    pub fn drawer_header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::drawer_header_template_property())
    }

    pub fn set_drawer_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::drawer_header_template_property(), value)
    }

    /// Gets or sets the data template used to display the drawer footer.
    pub fn drawer_footer_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::drawer_footer_template_property())
    }

    pub fn set_drawer_footer_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::drawer_footer_template_property(), value)
    }

    /// Gets or sets the icon displayed in the drawer toggle button.
    pub fn drawer_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::drawer_icon_property())
    }

    pub fn set_drawer_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::drawer_icon_property(), value)
    }

    /// Gets or sets the data template used to display the drawer icon.
    pub fn drawer_icon_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::drawer_icon_template_property())
    }

    pub fn set_drawer_icon_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::drawer_icon_template_property(), value)
    }

    /// Gets or sets the data template used to display the drawer content.
    pub fn drawer_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::drawer_template_property())
    }

    pub fn set_drawer_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::drawer_template_property(), value)
    }

    /// Gets or sets the data template used to display the content.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }

    /// Gets or sets the background brush of the drawer pane.
    pub fn drawer_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::drawer_background_property())
    }

    pub fn set_drawer_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::drawer_background_property(), value)
    }

    /// Gets or sets the background brush of the drawer header area.
    pub fn drawer_header_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::drawer_header_background_property())
    }

    pub fn set_drawer_header_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::drawer_header_background_property(), value)
    }

    /// Gets or sets the foreground brush of the drawer header area.
    pub fn drawer_header_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::drawer_header_foreground_property())
    }

    pub fn set_drawer_header_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::drawer_header_foreground_property(), value)
    }

    /// Gets or sets the background brush of the drawer footer area.
    pub fn drawer_footer_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::drawer_footer_background_property())
    }

    pub fn set_drawer_footer_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::drawer_footer_background_property(), value)
    }

    /// Gets or sets the foreground brush of the drawer footer area.
    pub fn drawer_footer_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::drawer_footer_foreground_property())
    }

    pub fn set_drawer_footer_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::drawer_footer_foreground_property(), value)
    }

    /// Gets or sets the backdrop brush for overlay mode.
    pub fn backdrop_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::backdrop_brush_property())
    }

    pub fn set_backdrop_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::backdrop_brush_property(), value)
    }

    /// Gets or sets the horizontal alignment of the detail content.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// Gets or sets the vertical alignment of the detail content.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// Gets or sets the display mode of the split view.
    pub fn display_mode(&self) -> SplitViewDisplayMode {
        self.get_value(Self::display_mode_property())
    }

    pub fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.set_value(Self::display_mode_property(), value)
    }

    fn dispose_nav_bar_visible_sub(&self) {
        let subscription = self.nav_bar_visible_sub.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    /// Follows the effective visibility of the navigation bar of the
    /// navigation page.
    fn observe_nav_bar_visible(&self, nav: &NavigationPage) -> Rc<dyn IDisposable> {
        let weak = self.to_ref().downgrade();
        let object: &FerroObject = nav;
        FerroObjectExtensions::get_observable(object, NavigationPage::is_nav_bar_effectively_visible_property())
            .subscribe_fn(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.update_detail_nav_bar_visible_pseudo_class();
                }
            })
    }

    fn restore_navigation_state(&self) {
        let Some(nav) = navigation_page_of(&self.content()) else {
            return;
        };

        self.dispose_nav_bar_visible_sub();
        nav.set_drawer_page(Some(self));
        let subscription = self.observe_nav_bar_visible(&nav);
        *self.nav_bar_visible_sub.borrow_mut() = Some(subscription);
        self.update_detail_nav_bar_visible_pseudo_class();
    }

    fn clear_navigation_state(&self) {
        self.dispose_nav_bar_visible_sub();

        if let Some(nav) = navigation_page_of(&self.content()) {
            nav.set_drawer_page(None);
        }
    }

    fn attach_backdrop_pointer_pressed(&self) {
        let backdrop = self.backdrop.borrow().clone();
        if let Some(backdrop) = backdrop {
            let weak = self.to_ref().downgrade();
            let token =
                backdrop.add_handler(InputElement::pointer_pressed_event(), move |_, e: &PointerPressedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.on_backdrop_pressed(e);
                    }
                });
            if let Some(previous) = self.backdrop_pointer_pressed.replace(Some(token)) {
                backdrop.remove_handler(InputElement::pointer_pressed_event(), previous);
            }
        }
    }

    fn detach_backdrop_pointer_pressed(&self) {
        let backdrop = self.backdrop.borrow().clone();
        if let Some(backdrop) = backdrop {
            if let Some(token) = self.backdrop_pointer_pressed.take() {
                backdrop.remove_handler(InputElement::pointer_pressed_event(), token);
            }
        }
    }

    fn update_swipe_recognizer_axes(&self) {
        self.swipe_recognizer.set_can_vertically_swipe(self.is_vertical_placement());
        self.swipe_recognizer.set_can_horizontally_swipe(!self.is_vertical_placement());
    }

    fn on_swipe_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        self.swipe_start_point.set(e.get_position(Some(self)));
    }

    fn on_swipe_gesture(&self, e: &SwipeGestureEventArgs) {
        let display_mode = self.display_mode();
        let bounds = self.bounds();
        if !self.is_gesture_enabled()
            || self.drawer_behavior() == DrawerBehavior::Disabled
            || self.drawer_behavior() == DrawerBehavior::Locked
            || display_mode == SplitViewDisplayMode::Inline
            || bounds.width <= 0.0
            || bounds.height <= 0.0
        {
            return;
        }

        let swipe_direction = e.swipe_direction();
        let swipe_start_point = self.swipe_start_point.get();
        let open_gesture_edge = if display_mode == SplitViewDisplayMode::CompactOverlay
            || display_mode == SplitViewDisplayMode::CompactInline
        {
            self.compact_drawer_length()
        } else {
            EDGE_GESTURE_WIDTH
        };

        let (toward_pane, in_edge) = if self.is_vertical_placement() {
            if swipe_direction != SwipeDirection::Up && swipe_direction != SwipeDirection::Down {
                return;
            }

            if self.drawer_placement() == DrawerPlacement::Bottom {
                (swipe_direction == SwipeDirection::Up, swipe_start_point.y >= bounds.height - open_gesture_edge)
            } else {
                (swipe_direction == SwipeDirection::Down, swipe_start_point.y <= open_gesture_edge)
            }
        } else {
            if swipe_direction != SwipeDirection::Left && swipe_direction != SwipeDirection::Right {
                return;
            }

            if self.is_pane_on_right() {
                (swipe_direction == SwipeDirection::Left, swipe_start_point.x >= bounds.width - open_gesture_edge)
            } else {
                (swipe_direction == SwipeDirection::Right, swipe_start_point.x <= open_gesture_edge)
            }
        };

        if !self.is_open() {
            if toward_pane && in_edge {
                self.set_current_value(Self::is_open_property(), true);
                e.set_handled(true);
            }
        } else if !toward_pane {
            self.set_current_value(Self::is_open_property(), false);
            e.set_handled(true);
        }
    }

    fn sync_current_page(&self) {
        self.set_current_value(Page::current_page_property(), self.resolve_current_page());
    }

    fn resolve_current_page(&self) -> Option<Ref<Page>> {
        let drawer_is_overlay = self.is_open() && self.is_overlay_display_mode();
        let drawer_page = if drawer_is_overlay { page_of(&self.drawer()) } else { None };
        drawer_page.or_else(|| page_of(&self.content()))
    }

    fn update_pane_placement(&self) {
        let split_view = self.split_view.borrow().clone();
        let Some(split_view) = split_view else {
            return;
        };

        let placement = self.drawer_placement();
        split_view.set_pane_placement(match placement {
            DrawerPlacement::Right => SplitViewPanePlacement::Right,
            DrawerPlacement::Top => SplitViewPanePlacement::Top,
            DrawerPlacement::Bottom => SplitViewPanePlacement::Bottom,
            DrawerPlacement::Left => SplitViewPanePlacement::Left,
        });

        self.pseudo_classes().set(PC_PLACEMENT_RIGHT, self.drawer_placement() == DrawerPlacement::Right);
        self.pseudo_classes().set(PC_PLACEMENT_TOP, self.drawer_placement() == DrawerPlacement::Top);
        self.pseudo_classes().set(PC_PLACEMENT_BOTTOM, self.drawer_placement() == DrawerPlacement::Bottom);
    }

    /// Sets `IsOpen` without raising the drawer events.
    fn set_is_open_suppressing_drawer_events(&self, value: bool) {
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        self.suppress_drawer_events.set(true);
        let _reset = Reset(&self.suppress_drawer_events);
        self.set_current_value(Self::is_open_property(), value);
    }

    fn update_split_view_display_mode(&self) {
        let previous_mode = self.display_mode();

        let mode = self.resolve_split_view_display_mode();
        self.set_current_value(Self::display_mode_property(), mode);

        if self.drawer_behavior() == DrawerBehavior::Disabled {
            self.set_current_value(Self::is_open_property(), false);
            return;
        }

        if self.drawer_behavior() == DrawerBehavior::Locked {
            self.set_current_value(Self::is_open_property(), true);
            return;
        }

        if self.split_view.borrow().is_none() {
            return;
        }

        if self.drawer_breakpoint_length() > 0.0 && previous_mode != mode {
            if mode == SplitViewDisplayMode::Inline {
                self.set_is_open_suppressing_drawer_events(true);
            } else if mode == SplitViewDisplayMode::Overlay {
                self.set_is_open_suppressing_drawer_events(false);
            }
        }
    }

    fn resolve_split_view_display_mode(&self) -> SplitViewDisplayMode {
        match self.drawer_behavior() {
            DrawerBehavior::Locked => return SplitViewDisplayMode::Inline,
            DrawerBehavior::Disabled | DrawerBehavior::Flyout => return SplitViewDisplayMode::Overlay,
            DrawerBehavior::Auto => {}
        }

        let breakpoint = self.drawer_breakpoint_length();
        if breakpoint > 0.0 {
            let length = if self.is_vertical_placement() { self.bounds().height } else { self.bounds().width };
            if length > 0.0 && length < breakpoint {
                return SplitViewDisplayMode::Overlay;
            }
        }

        match self.drawer_layout_behavior() {
            DrawerLayoutBehavior::Split => SplitViewDisplayMode::Inline,
            DrawerLayoutBehavior::CompactOverlay => SplitViewDisplayMode::CompactOverlay,
            DrawerLayoutBehavior::CompactInline => SplitViewDisplayMode::CompactInline,
            DrawerLayoutBehavior::Overlay => SplitViewDisplayMode::Overlay,
        }
    }

    fn update_drawer_focus(&self) {
        if !self.is_open() {
            let pane_button = self.pane_button.borrow().clone();
            if let Some(pane_button) = pane_button {
                pane_button.focus();
            }
        } else if self.is_overlay_display_mode() {
            let drawer_presenter = self.drawer_presenter.borrow().clone();
            if let Some(drawer_presenter) = drawer_presenter {
                let first_focusable =
                    KeyboardNavigationHandler::get_next(&drawer_presenter.upcast(), NavigationDirection::Next);
                if let Some(first_focusable) = first_focusable {
                    first_focusable.focus();
                }
            }
        }
    }

    fn update_backdrop_state(&self) {
        let backdrop = self.backdrop.borrow().clone();
        let Some(backdrop) = backdrop else {
            return;
        };
        let show = self.is_open() && self.backdrop_brush().is_some() && self.is_overlay_display_mode();
        backdrop.set_is_visible(show);
        backdrop.set_is_hit_test_visible(show);
    }

    fn on_backdrop_pressed(&self, e: &PointerPressedEventArgs) {
        self.set_current_value(Self::is_open_property(), false);
        e.set_handled(true);
    }

    fn apply_drawer_background(&self) {
        let split_view = self.split_view.borrow().clone();
        let Some(split_view) = split_view else {
            return;
        };

        match self.drawer_background() {
            Some(drawer_background) => split_view.set_pane_background(Some(drawer_background)),
            None => split_view.clear_value(SplitView::pane_background_property()),
        }
    }

    fn apply_foreground(presenter: Option<&Ref<ContentPresenter>>, brush: Option<Rc<dyn IBrush>>) {
        let Some(presenter) = presenter else {
            return;
        };

        match brush {
            Some(brush) => TextElement::set_foreground_on(presenter, Some(brush)),
            None => presenter.clear_value(TextElement::foreground_property()),
        }
    }

    fn update_detail_nav_bar_visible_pseudo_class(&self) {
        self.hide_top_bar
            .set(navigation_page_of(&self.content()).is_some_and(|nav_page| nav_page.is_nav_bar_effectively_visible()));
        self.pseudo_classes().set(PC_DETAIL_IS_NAV_PAGE, self.hide_top_bar.get());
    }
}

/// `value as Page`.
fn page_of(value: &Option<BoxedValue>) -> Option<Ref<Page>> {
    value.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<Page>())
}

/// `value as NavigationPage`.
fn navigation_page_of(value: &Option<BoxedValue>) -> Option<Ref<NavigationPage>> {
    value.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<NavigationPage>())
}

/// `ReferenceEquals(a, b)`.
fn same_page(a: &Option<Ref<Page>>, b: &Option<Ref<Page>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.ptr_eq(b),
        _ => false,
    }
}
