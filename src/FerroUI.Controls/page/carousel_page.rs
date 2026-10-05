use super::selecting_multi_page::same_page;
use super::{
    MultiPage, MultiPageImpl, NavigationType, Page, PageImpl, PageList, SelectingMultiPage, SelectingMultiPageImpl,
};
use crate::automation::AutomationProperties;
use crate::presenters::ContentPresenter;
use crate::primitives::{SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::{FuncTemplate, IDataTemplate, ITemplateOf};
use crate::{
    Carousel, ContainerPreparedEventArgs, ContentControl, Control, ControlImpl, ItemsControl, ItemsSource, Panel,
    VirtualizingCarouselPanel,
};
use ferroui_base::animation::IPageTransition;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl,
    StyledProperty, TypeInfo, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The carousel of the template and the handlers added to it.
struct CarouselPart {
    carousel: Ref<Carousel>,
    selection_changed: Option<RoutedEventHandlerToken>,
    container_prepared: Rc<dyn IDisposable>,
}

/// A page that displays its child pages in a horizontally scrollable
/// carousel, with optional animated page transitions.
#[repr(C)]
pub struct CarouselPage {
    base: SelectingMultiPage,
    carousel: RefCell<Option<CarouselPart>>,
    pointer_wheel_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(CarouselPage: SelectingMultiPage);

ferro_class_info!(CarouselPage {
    new: CarouselPage::new,
    markup: {
        attributes: [
            TemplatePart("PART_Carousel", type(Ref<Carousel>)),
        ],
    },
});

ferro_impl_classes!(CarouselPage: LayoutableImpl, InteractiveImpl, PageImpl);

impl FerroObjectImpl for CarouselPage {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(MultiPage::pages_property(), Some(PageList::new()));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let Some(carousel) = this.carousel() else {
            return;
        };

        if change.property() == Self::page_transition_property().as_property() {
            carousel.set_page_transition(change.get_new_value::<Option<Rc<dyn IPageTransition>>>());
        } else if change.property() == Self::items_panel_property().as_property() {
            carousel.set_items_panel(change.get_new_value::<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>>());
        } else if change.property() == MultiPage::page_template_property().as_property() {
            carousel.set_item_template(change.get_new_value::<Option<Rc<dyn IDataTemplate>>>());
            if this.items_source().is_some() {
                this.update_active_page();
            }
        } else if change.property() == Self::is_gesture_enabled_property().as_property() {
            carousel.set_is_swipe_enabled(change.get_new_value::<bool>());
        } else if change.property() == MultiPage::items_source_property().as_property() {
            carousel
                .set_items_source(change.get_new_value::<Option<ItemsSource>>().or_else(|| this.pages_items_source()));
            this.update_active_page();
        } else if change.property() == MultiPage::pages_property().as_property() && this.items_source().is_none() {
            carousel.set_items_source(
                change.get_new_value::<Option<PageList>>().as_ref().map(MultiPage::items_source_of),
            );
        }
    }
}

impl StyledElementImpl for CarouselPage {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <CarouselPage as StaticType>::TYPE
    }
}

impl VisualImpl for CarouselPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let weak = this.to_ref().downgrade();
        let token = this.add_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, e: &PointerWheelEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_pointer_wheel_changed(e);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        if let Some(previous) = this.pointer_wheel_handler.replace(Some(token)) {
            this.remove_handler(InputElement::pointer_wheel_changed_event(), previous);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(token) = this.pointer_wheel_handler.take() {
            this.remove_handler(InputElement::pointer_wheel_changed_event(), token);
        }
    }
}

impl InputElementImpl for CarouselPage {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() || !this.is_keyboard_navigation_enabled() {
            return;
        }

        let is_rtl = this.is_right_to_left();
        let next = if is_rtl { e.key == Key::Left } else { e.key == Key::Right };
        let prev = if is_rtl { e.key == Key::Right } else { e.key == Key::Left };

        let page_count = this.get_page_count();
        if next || e.key == Key::Down {
            if this.selected_index() < page_count - 1 {
                this.apply_selected_index(this.selected_index() + 1);
                e.set_handled(true);
            }
        } else if prev || e.key == Key::Up {
            if this.selected_index() > 0 {
                this.apply_selected_index(this.selected_index() - 1);
                e.set_handled(true);
            }
        } else if e.key == Key::Home {
            if page_count > 0 && this.selected_index() != 0 {
                this.apply_selected_index(0);
                e.set_handled(true);
            }
        } else if e.key == Key::End && page_count > 0 && this.selected_index() != page_count - 1 {
            this.apply_selected_index(page_count - 1);
            e.set_handled(true);
        }
    }
}

impl TemplatedControlImpl for CarouselPage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let requested_index = this.selected_index();

        let previous = this.carousel.borrow_mut().take();
        if let Some(previous) = previous {
            if let Some(token) = previous.selection_changed {
                previous.carousel.remove_handler(SelectingItemsControl::selection_changed_event(), token);
            }
            previous.container_prepared.dispose();
        }

        let carousel = e.name_scope().find_as::<Carousel>("PART_Carousel");

        if let Some(carousel) = carousel {
            let weak = this.to_ref().downgrade();
            let container_prepared = carousel.container_prepared(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_carousel_container_prepared(e);
                }
            });
            *this.carousel.borrow_mut() =
                Some(CarouselPart { carousel: carousel.clone(), selection_changed: None, container_prepared });
            carousel.set_page_transition(this.page_transition());
            carousel.set_items_panel(this.items_panel());
            carousel.set_item_template(this.page_template());
            carousel.set_is_swipe_enabled(this.is_gesture_enabled());
            carousel.set_items_source(this.items_source().or_else(|| this.pages_items_source()));

            if requested_index >= 0 {
                carousel.set_selected_index(requested_index);
            }

            let weak = this.to_ref().downgrade();
            let token = carousel.selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_carousel_selection_changed();
                }
            });
            if let Some(part) = this.carousel.borrow_mut().as_mut() {
                part.selection_changed = Some(token);
            }

            this.update_active_page();
        }
    }
}

impl MultiPageImpl for CarouselPage {
    fn update_active_page_with(this: &Self, navigation_type: NavigationType) {
        if let Some(carousel) = this.carousel() {
            let index = carousel.selected_index();
            if index >= 0 {
                this.update_selection(index, navigation_type);
            } else if this.get_page_count() > 0 {
                this.apply_selected_index(0);
            }
        } else if this.get_page_count() > 0 {
            let index = this.coerce_pre_template_selected_index(this.selected_index());
            if this.items_source().is_some() {
                this.store_selected_index(index);
            } else {
                let page = this.resolve_page_at_index(index);

                if index != this.selected_index() || !same_page(&this.selected_page(), &page) {
                    this.commit_selection(index, page, navigation_type);
                }
            }
        }
    }
}

impl SelectingMultiPageImpl for CarouselPage {
    fn apply_selected_index(this: &Self, index: i32) {
        if let Some(carousel) = this.carousel() {
            carousel.set_selected_index(index);
        } else {
            let page_count = this.get_page_count();

            if page_count > 0 {
                let coerced_index = this.coerce_pre_template_selected_index(index);
                if this.items_source().is_some() {
                    this.store_selected_index(coerced_index);
                } else {
                    let new_page = this.resolve_page_at_index(coerced_index);
                    this.commit_selection(coerced_index, new_page, NavigationType::Replace);
                }
            } else {
                // Preserve preselection until pages exist.
                this.store_selected_index(index);
            }
        }
    }
}

impl ControlImpl for CarouselPage {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::CarouselPageAutomationPeer::new(this).upcast()
    }
}

ferro_properties! {
    impl CarouselPage {
        /// Defines the `ItemsPanel` property.
        pub fn items_panel_property() -> StyledProperty<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>> {
            ItemsControl::items_panel_property().add_owner::<CarouselPage>()
        }

        /// Defines the `PageTransition` property.
        pub fn page_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<CarouselPage, _>("PageTransition", None)
        }

        /// Defines the `IsGestureEnabled` property.
        pub fn is_gesture_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<CarouselPage, _>("IsGestureEnabled", true)
        }

        /// Defines the `IsKeyboardNavigationEnabled` property.
        pub fn is_keyboard_navigation_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<CarouselPage, _>("IsKeyboardNavigationEnabled", true)
        }
    }
}

impl CarouselPage {
    fn static_constructor() {
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(VirtualizingCarouselPanel::new().upcast::<Panel>()));

        Self::items_panel_property().override_default_value::<CarouselPage>(default_panel);
        InputElement::focusable_property().override_default_value::<CarouselPage>(true);
        Page::page_navigation_system_back_button_pressed_event().add_class_handler::<CarouselPage>(
            |sender, event_args| {
                if event_args.handled() {
                    return;
                }

                let page_event = RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
                if let Some(current_page) = sender.current_page() {
                    current_page.raise_event(&page_event);
                }

                if page_event.handled() {
                    event_args.set_handled(true);
                }
            },
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: SelectingMultiPage::construct(),
            carousel: RefCell::new(None),
            pointer_wheel_handler: Cell::new(None),
        }
    }

    /// Initializes a new instance of the [`CarouselPage`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the items panel template used to arrange page items.
    pub fn items_panel(&self) -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
        self.get_value(Self::items_panel_property())
    }

    pub fn set_items_panel(&self, value: Rc<dyn ITemplateOf<Option<Ref<Panel>>>>) {
        self.set_value(Self::items_panel_property(), value)
    }

    /// Gets or sets the animated page transition used when the selected
    /// page changes.
    pub fn page_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::page_transition_property())
    }

    pub fn set_page_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::page_transition_property(), value)
    }

    /// Gets or sets whether swipe and scroll gestures can be used to
    /// navigate between pages.
    pub fn is_gesture_enabled(&self) -> bool {
        self.get_value(Self::is_gesture_enabled_property())
    }

    pub fn set_is_gesture_enabled(&self, value: bool) {
        self.set_value(Self::is_gesture_enabled_property(), value)
    }

    /// Gets or sets whether keyboard shortcuts (arrow keys, Home/End) can
    /// be used to navigate between pages.
    pub fn is_keyboard_navigation_enabled(&self) -> bool {
        self.get_value(Self::is_keyboard_navigation_enabled_property())
    }

    pub fn set_is_keyboard_navigation_enabled(&self, value: bool) {
        self.set_value(Self::is_keyboard_navigation_enabled_property(), value)
    }

    fn carousel(&self) -> Option<Ref<Carousel>> {
        self.carousel.borrow().as_ref().map(|part| part.carousel.clone())
    }

    fn on_carousel_selection_changed(&self) {
        let Some(carousel) = self.carousel() else {
            return;
        };

        let new_index = carousel.selected_index();
        let new_page = self.resolve_displayed_page_at_index(new_index);
        if new_index == self.selected_index() && same_page(&new_page, &self.selected_page()) {
            return;
        }

        self.update_selection(new_index, NavigationType::Replace);
    }

    fn on_carousel_container_prepared(&self, e: &ContainerPreparedEventArgs) {
        let Some(carousel) = self.carousel() else {
            return;
        };
        if e.index() != carousel.selected_index() {
            return;
        }

        let weak = self.to_ref().downgrade();
        let index = e.index();
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if this.carousel().is_some_and(|carousel| carousel.selected_index() == index) {
                    this.update_selection(index, NavigationType::Replace);
                }
            },
            DispatcherPriority::LOADED,
        );
    }

    fn on_pointer_wheel_changed(&self, e: &PointerWheelEventArgs) {
        if !self.is_gesture_enabled() {
            return;
        }

        let is_rtl = self.is_right_to_left();
        let page_count = self.get_page_count();
        let delta = e.delta();
        let go_next = delta.y < 0.0 || if is_rtl { delta.x < 0.0 } else { delta.x > 0.0 };
        let go_prev = delta.y > 0.0 || if is_rtl { delta.x > 0.0 } else { delta.x < 0.0 };

        if go_next && self.selected_index() < page_count - 1 {
            self.apply_selected_index(self.selected_index() + 1);
            e.set_handled(true);
        } else if go_prev && self.selected_index() > 0 {
            self.apply_selected_index(self.selected_index() - 1);
            e.set_handled(true);
        }
    }

    fn update_accessibility_name(&self, index: i32, page_count: i32, page: Option<&Ref<Page>>) {
        let header =
            page.and_then(|page| page.header()).map(|header| ValueTypes::to_display_string(Some(&header)));
        let position = if page_count > 0 { format!("Page {} of {}", index + 1, page_count) } else { String::new() };
        let name = match header {
            None => position,
            Some(header) if header.is_empty() => position,
            Some(header) if position.is_empty() => header,
            Some(header) => format!("{position}: {header}"),
        };
        // CarouselPageAutomationPeer::get_name_core reads this via the base implementation, which returns
        // the automation name when set. Position and header are encoded here rather than in the
        // peer so that the name stays current without requiring the peer to re-query the carousel state.
        AutomationProperties::set_name(self, Some(&name));
    }

    fn is_right_to_left(&self) -> bool {
        self.flow_direction() == FlowDirection::RightToLeft
    }

    fn get_page_count(&self) -> i32 {
        if let Some(carousel) = self.carousel() {
            return carousel.item_count();
        }

        if let Some(source) = self.items_source() {
            return source.count() as i32;
        }

        self.pages().map_or(0, |pages| pages.count() as i32)
    }

    fn coerce_pre_template_selected_index(&self, index: i32) -> i32 {
        let page_count = self.get_page_count();
        if page_count <= 0 {
            return index;
        }

        if index >= 0 && index < page_count {
            index
        } else {
            0
        }
    }

    fn update_selection(&self, index: i32, navigation_type: NavigationType) {
        let page = self.resolve_displayed_page_at_index(index);
        let page_count = self.get_page_count();

        if page.is_none() && self.items_source().is_some() {
            self.store_selected_index(index);
            self.update_accessibility_name(index, page_count, None);
            return;
        }

        self.commit_selection(index, page.clone(), navigation_type);
        self.update_accessibility_name(index, page_count, page.as_ref());
    }

    fn resolve_displayed_page_at_index(&self, index: i32) -> Option<Ref<Page>> {
        if index < 0 {
            return None;
        }

        if let Some(container) = self.carousel().and_then(|carousel| carousel.container_from_index(index)) {
            return Self::try_get_page_from_container(&container);
        }

        if self.items_source().is_none() {
            self.resolve_page_at_index(index)
        } else {
            None
        }
    }

    fn try_get_page_from_container(container: &Ref<Control>) -> Option<Ref<Page>> {
        if let Some(page) = container.clone().cast::<Page>() {
            return Some(page);
        }

        if let Some(presenter) = container.clone().cast::<ContentPresenter>() {
            if presenter.child().is_none() {
                presenter.update_child();
            }

            return presenter.child().and_then(|child| child.cast::<Page>());
        }

        if let Some(content_control) = container.clone().cast::<ContentControl>() {
            return content_control
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .and_then(|content| content.cast::<Page>());
        }

        None
    }
}
