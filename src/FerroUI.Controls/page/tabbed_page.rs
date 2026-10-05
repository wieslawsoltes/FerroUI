use super::{
    MultiPage, MultiPageImpl, NavigationType, Page, PageImpl, PageImplExt, SelectingMultiPage, SelectingMultiPageImpl,
    TabPlacement,
};
use crate::primitives::{SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{
    ContainerClearingEventArgs, ContainerPreparedEventArgs, Control, ControlImpl, Dock, ItemsSource, TabControl,
    TabItem,
};
use ferroui_base::animation::IPageTransition;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
    SwipeDirection, SwipeGestureEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElement,
    StyledElementImpl, StyledProperty, Thickness, TypeInfo, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const IS_MOBILE_PLATFORM: bool = cfg!(target_os = "android") || cfg!(target_os = "ios");

/// The page shown by a tab container, and the subscription to the property
/// changes of the page.
struct ContainerPage {
    tab_item: Ref<TabItem>,
    page: Ref<Page>,
    page_property_changed: Rc<dyn IDisposable>,
}

/// The tab control of the template and the handlers added to it.
struct TabControlPart {
    tab_control: Ref<TabControl>,
    selection_changed: Option<RoutedEventHandlerToken>,
    container_prepared: Rc<dyn IDisposable>,
    container_clearing: Rc<dyn IDisposable>,
}

fn tab_item_key(tab_item: &TabItem) -> usize {
    tab_item as *const TabItem as usize
}

fn page_key(page: &Page) -> usize {
    page as *const Page as usize
}

/// A page that displays its child pages through a tab strip.
#[repr(C)]
pub struct TabbedPage {
    base: SelectingMultiPage,
    tab_control: RefCell<Option<TabControlPart>>,
    ignoring_disabled_selection: Cell<bool>,
    /// The pages of the tab containers, by the identity of the container.
    container_page_map: RefCell<HashMap<usize, ContainerPage>>,
    /// The tab containers of the pages, by the identity of the page.
    page_container_map: RefCell<HashMap<usize, Ref<TabItem>>>,
    /// The pages created by the page template, by their identity.
    template_created_pages: RefCell<HashMap<usize, Ref<Page>>>,
    last_swipe_gesture_id: Cell<i32>,
    swipe_recognizer: Ref<SwipeGestureRecognizer>,
    swipe_gesture_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(TabbedPage: SelectingMultiPage);

ferro_class_info!(TabbedPage {
    new: TabbedPage::new,
    markup: {
        attributes: [
            TemplatePart("PART_TabControl", type(Ref<TabControl>)),
        ],
    },
});

ferro_impl_classes!(TabbedPage: LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for TabbedPage {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(MultiPage::pages_property(), Some(super::PageList::new()));
        this.gesture_recognizers().add(this.swipe_recognizer.clone());
        this.update_swipe_recognizer_axes();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let tab_control = this.tab_control();
        if change.property() == Self::tab_placement_property().as_property() {
            this.apply_tab_placement();
            this.update_swipe_recognizer_axes();
        } else if change.property() == Self::page_transition_property().as_property() && tab_control.is_some() {
            if let Some(tab_control) = tab_control {
                tab_control.set_page_transition(change.get_new_value::<Option<Rc<dyn IPageTransition>>>());
            }
        } else if change.property() == Self::indicator_template_property().as_property() {
            this.apply_indicator_template();
        } else if change.property() == Self::is_gesture_enabled_property().as_property() {
            this.swipe_recognizer.set_is_enabled(change.get_new_value::<bool>());
        } else if change.property() == MultiPage::items_source_property().as_property() && tab_control.is_some() {
            if let Some(tab_control) = tab_control {
                tab_control.set_items_source(
                    change.get_new_value::<Option<ItemsSource>>().or_else(|| this.pages_items_source()),
                );
            }
        } else if change.property() == MultiPage::pages_property().as_property()
            && this.items_source().is_none()
            && tab_control.is_some()
        {
            if let Some(tab_control) = tab_control {
                tab_control.set_items_source(
                    change.get_new_value::<Option<super::PageList>>().as_ref().map(MultiPage::items_source_of),
                );
            }
        } else if change.property() == MultiPage::page_template_property().as_property()
            && this.items_source().is_some()
            && tab_control.is_some()
        {
            this.rebuild_template_created_pages();
        }
    }
}

impl StyledElementImpl for TabbedPage {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TabbedPage as StaticType>::TYPE
    }
}

impl VisualImpl for TabbedPage {
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
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(token) = this.swipe_gesture_handler.take() {
            this.remove_handler(InputElement::swipe_gesture_event(), token);
        }
    }
}

impl InputElementImpl for TabbedPage {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if !this.is_keyboard_navigation_enabled() {
            return;
        }
        let Some(tab_control) = this.tab_control() else {
            return;
        };

        let resolved = this.resolve_tab_placement();
        let is_horizontal = resolved == TabPlacement::Top || resolved == TabPlacement::Bottom;
        let is_rtl = this.flow_direction() == FlowDirection::RightToLeft;

        let mut next = if is_horizontal {
            if is_rtl {
                e.key == Key::Left
            } else {
                e.key == Key::Right
            }
        } else {
            e.key == Key::Down
        };
        let mut prev = if is_horizontal {
            if is_rtl {
                e.key == Key::Right
            } else {
                e.key == Key::Left
            }
        } else {
            e.key == Key::Up
        };

        if e.key_modifiers.contains(KeyModifiers::CONTROL) && e.key == Key::Tab {
            next = !e.key_modifiers.contains(KeyModifiers::SHIFT);
            prev = e.key_modifiers.contains(KeyModifiers::SHIFT);
        }

        if next {
            let target = this.find_next_enabled_tab(tab_control.selected_index() + 1, 1);
            if target >= 0 {
                tab_control.set_selected_index(target);
                this.focus_selected_tab_item();
                e.set_handled(true);
            }
        } else if prev {
            let target = this.find_next_enabled_tab(tab_control.selected_index() - 1, -1);
            if target >= 0 {
                tab_control.set_selected_index(target);
                this.focus_selected_tab_item();
                e.set_handled(true);
            }
        }
    }
}

impl TemplatedControlImpl for TabbedPage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let requested_index = this.selected_index();

        let previous = this.tab_control.borrow_mut().take();
        if let Some(previous) = previous {
            if let Some(token) = previous.selection_changed {
                previous.tab_control.remove_handler(SelectingItemsControl::selection_changed_event(), token);
            }
            previous.container_prepared.dispose();
            previous.container_clearing.dispose();
        }

        this.clear_container_pages();

        let tab_control = e.name_scope().find_as::<TabControl>("PART_TabControl");

        if let Some(tab_control) = tab_control {
            let weak = this.to_ref().downgrade();
            let container_prepared = tab_control.container_prepared(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_container_prepared(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let container_clearing = tab_control.container_clearing(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_container_clearing(e);
                }
            });
            *this.tab_control.borrow_mut() = Some(TabControlPart {
                tab_control: tab_control.clone(),
                selection_changed: None,
                container_prepared,
                container_clearing,
            });
            tab_control.set_items_source(this.items_source().or_else(|| this.pages_items_source()));

            if requested_index >= 0 {
                tab_control.set_selected_index(requested_index);
            }

            let weak = this.to_ref().downgrade();
            let token = tab_control.selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.tab_control_selection_changed();
                }
            });
            if let Some(part) = this.tab_control.borrow_mut().as_mut() {
                part.selection_changed = Some(token);
            }

            if let Some(page_transition) = this.page_transition() {
                tab_control.set_page_transition(Some(page_transition));
            }

            this.apply_tab_placement();
            this.apply_indicator_template();
            this.update_active_page();

            let weak = this.to_ref().downgrade();
            let captured_tab_control = tab_control;
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.sync_all_tab_enabled_states(&captured_tab_control);
                    }
                },
                DispatcherPriority::LOADED,
            );
        }
    }
}

impl PageImpl for TabbedPage {
    fn update_content_safe_area_padding(this: &Self) {
        Self::parent_update_content_safe_area_padding(this);

        if let Some(tab_control) = this.tab_control() {
            let sa = this.safe_area_padding();

            let (bar_margin, content_safe_area) = match this.resolve_tab_placement() {
                TabPlacement::Bottom => {
                    (Thickness::new(sa.left, 0.0, sa.right, sa.bottom), Thickness::new(sa.left, sa.top, sa.right, 0.0))
                }
                TabPlacement::Left => {
                    (Thickness::new(sa.left, sa.top, 0.0, sa.bottom), Thickness::new(0.0, sa.top, sa.right, sa.bottom))
                }
                TabPlacement::Right => {
                    (Thickness::new(0.0, sa.top, sa.right, sa.bottom), Thickness::new(sa.left, sa.top, 0.0, sa.bottom))
                }
                _ => (Thickness::new(sa.left, sa.top, sa.right, 0.0), Thickness::new(sa.left, 0.0, sa.right, sa.bottom)),
            };

            tab_control.set_margin(bar_margin);

            if let Some(current_page) = this.current_page() {
                current_page.set_safe_area_padding(content_safe_area);
            }
        }
    }
}

impl MultiPageImpl for TabbedPage {
    fn update_active_page_with(this: &Self, navigation_type: NavigationType) {
        if let Some(tab_control) = this.tab_control() {
            let index = tab_control.selected_index();
            this.commit_selection_if_resolved(index, navigation_type);
            this.update_content_safe_area_padding();
        }
    }
}

impl SelectingMultiPageImpl for TabbedPage {
    fn apply_selected_index(this: &Self, index: i32) {
        if let Some(tab_control) = this.tab_control() {
            tab_control.set_selected_index(index);
        } else {
            this.store_selected_index(index);
        }
    }
}

impl ControlImpl for TabbedPage {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::TabbedPageAutomationPeer::new(this).upcast()
    }
}

ferro_properties! {
    impl TabbedPage {
        /// Defines the `TabPlacement` property.
        pub fn tab_placement_property() -> StyledProperty<TabPlacement> {
            FerroProperty::register::<TabbedPage, _>("TabPlacement", TabPlacement::Auto)
        }

        /// Defines the `IsKeyboardNavigationEnabled` property.
        pub fn is_keyboard_navigation_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabbedPage, _>("IsKeyboardNavigationEnabled", true)
        }

        /// Defines the `IsGestureEnabled` property.
        pub fn is_gesture_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<TabbedPage, _>("IsGestureEnabled", false)
        }

        /// Defines the `PageTransition` property.
        pub fn page_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<TabbedPage, _>("PageTransition", None)
        }

        /// Defines the `IndicatorTemplate` property.
        pub fn indicator_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabbedPage, _>("IndicatorTemplate", None)
        }

        /// Defines the `IsTabEnabled` attached property.
        pub fn is_tab_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<TabbedPage, Page, _>("IsTabEnabled", true)
        }
    }
}

impl TabbedPage {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<TabbedPage>(true);
        Page::page_navigation_system_back_button_pressed_event().add_class_handler::<TabbedPage>(
            |sender, event_args| {
                if event_args.handled() {
                    return;
                }

                let forwarded = RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
                if let Some(current_page) = sender.current_page() {
                    current_page.raise_event(&forwarded);
                }

                if forwarded.handled() {
                    event_args.set_handled(true);
                }
            },
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        let swipe_recognizer = SwipeGestureRecognizer::new();
        swipe_recognizer.set_is_enabled(false);
        Self {
            base: SelectingMultiPage::construct(),
            tab_control: RefCell::new(None),
            ignoring_disabled_selection: Cell::new(false),
            container_page_map: RefCell::new(HashMap::new()),
            page_container_map: RefCell::new(HashMap::new()),
            template_created_pages: RefCell::new(HashMap::new()),
            last_swipe_gesture_id: Cell::new(0),
            swipe_recognizer,
            swipe_gesture_handler: Cell::new(None),
        }
    }

    /// Initializes a new instance of the [`TabbedPage`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the value of the `IsTabEnabled` attached property for a page:
    /// true if the tab is enabled.
    pub fn get_is_tab_enabled(page: &Page) -> bool {
        page.get_value(Self::is_tab_enabled_property())
    }

    /// Sets the value of the `IsTabEnabled` attached property for a page.
    /// Disabled tabs are skipped during keyboard and swipe navigation.
    pub fn set_is_tab_enabled(page: &Page, value: bool) {
        page.set_value(Self::is_tab_enabled_property(), value)
    }

    /// Gets or sets the tab bar placement.
    pub fn tab_placement(&self) -> TabPlacement {
        self.get_value(Self::tab_placement_property())
    }

    pub fn set_tab_placement(&self, value: TabPlacement) {
        self.set_value(Self::tab_placement_property(), value)
    }

    /// Gets or sets whether keyboard navigation can switch tabs.
    pub fn is_keyboard_navigation_enabled(&self) -> bool {
        self.get_value(Self::is_keyboard_navigation_enabled_property())
    }

    pub fn set_is_keyboard_navigation_enabled(&self, value: bool) {
        self.set_value(Self::is_keyboard_navigation_enabled_property(), value)
    }

    /// Gets or sets whether swipe gestures can be used to navigate between
    /// tabs.
    ///
    /// Defaults to false because tab strips do not respond to swipe
    /// gestures on most platforms (iOS, desktop). Enable this only when the
    /// host platform and the content inside each tab page do not conflict
    /// with horizontal swipe input.
    pub fn is_gesture_enabled(&self) -> bool {
        self.get_value(Self::is_gesture_enabled_property())
    }

    pub fn set_is_gesture_enabled(&self, value: bool) {
        self.set_value(Self::is_gesture_enabled_property(), value)
    }

    /// Gets or sets the page transition to use when switching tabs.
    pub fn page_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::page_transition_property())
    }

    pub fn set_page_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::page_transition_property(), value)
    }

    /// Gets or sets the data template used to render the selection
    /// indicator on each tab.
    pub fn indicator_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::indicator_template_property())
    }

    pub fn set_indicator_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::indicator_template_property(), value)
    }

    fn tab_control(&self) -> Option<Ref<TabControl>> {
        self.tab_control.borrow().as_ref().map(|part| part.tab_control.clone())
    }

    fn tab_item_from_index(tab_control: &TabControl, index: i32) -> Option<Ref<TabItem>> {
        tab_control.container_from_index(index).and_then(|container| container.cast::<TabItem>())
    }

    fn container_page(&self, tab_item: &TabItem) -> Option<Ref<Page>> {
        self.container_page_map.borrow().get(&tab_item_key(tab_item)).map(|entry| entry.page.clone())
    }

    fn page_container(&self, page: &Page) -> Option<Ref<TabItem>> {
        self.page_container_map.borrow().get(&page_key(page)).cloned()
    }

    fn resolve_tab_placement(&self) -> TabPlacement {
        if self.tab_placement() != TabPlacement::Auto {
            return self.tab_placement();
        }

        if IS_MOBILE_PLATFORM {
            TabPlacement::Bottom
        } else {
            TabPlacement::Top
        }
    }

    fn apply_tab_placement(&self) {
        let Some(tab_control) = self.tab_control() else {
            return;
        };

        tab_control.set_tab_strip_placement(match self.resolve_tab_placement() {
            TabPlacement::Bottom => Dock::Bottom,
            TabPlacement::Left => Dock::Left,
            TabPlacement::Right => Dock::Right,
            _ => Dock::Top,
        });
    }

    fn update_swipe_recognizer_axes(&self) {
        let placement = self.resolve_tab_placement();
        let is_horizontal = placement == TabPlacement::Top || placement == TabPlacement::Bottom;
        self.swipe_recognizer.set_can_horizontally_swipe(is_horizontal);
        self.swipe_recognizer.set_can_vertically_swipe(!is_horizontal);
    }

    fn apply_indicator_template(&self) {
        let Some(tab_control) = self.tab_control() else {
            return;
        };

        tab_control.set_indicator_template(self.indicator_template());
    }

    fn tab_control_selection_changed(&self) {
        if self.ignoring_disabled_selection.get() {
            return;
        }

        let Some(tab_control) = self.tab_control() else {
            return;
        };
        let new_index = tab_control.selected_index();
        let new_page = self.resolve_tab_page_at_index(new_index);

        if let Some(new_page) = new_page {
            if !Self::get_is_tab_enabled(&new_page) {
                let target = self.find_nearest_enabled_tab(new_index);
                if target < 0 {
                    self.select_ignoring_disabled_selection(&tab_control, self.selected_index());
                    return;
                }

                self.select_ignoring_disabled_selection(&tab_control, target);

                if target == self.selected_index() {
                    return;
                }

                self.commit_selection_if_resolved(target, NavigationType::Replace);
                self.update_content_safe_area_padding();
                return;
            }
        }

        self.commit_selection_if_resolved(new_index, NavigationType::Replace);
        self.update_content_safe_area_padding();
    }

    /// Selects `index` in the tab control without reacting to the selection
    /// change it raises.
    fn select_ignoring_disabled_selection(&self, tab_control: &TabControl, index: i32) {
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        self.ignoring_disabled_selection.set(true);
        let _reset = Reset(&self.ignoring_disabled_selection);
        tab_control.set_selected_index(index);
    }

    fn sync_tab_item_with_page(tab_item: &TabItem, page: &Page) {
        tab_item.set_is_enabled(Self::get_is_tab_enabled(page));
        tab_item.set_header(page.header());
        tab_item.set_icon(page.icon());
        tab_item.set_icon_template(page.icon_template());
    }

    fn on_container_prepared(&self, e: &ContainerPreparedEventArgs) {
        let Some(tab_item) = e.container().clone().cast::<TabItem>() else {
            return;
        };

        let Some(page) = self.prepare_page_for_container(&tab_item, e.index()) else {
            return;
        };

        Self::sync_tab_item_with_page(&tab_item, &page);

        if e.index() == self.tab_control().map_or(-1, |tab_control| tab_control.selected_index()) {
            self.update_active_page();
        }
    }

    fn on_container_clearing(&self, e: &ContainerClearingEventArgs) {
        if let Some(tab_item) = e.container().clone().cast::<TabItem>() {
            self.clear_container_page(&tab_item);
        }
    }

    fn rebuild_template_created_pages(&self) {
        let Some(tab_control) = self.tab_control() else {
            return;
        };
        if self.items_source().is_none() {
            return;
        }

        for i in 0..tab_control.item_count() {
            let Some(tab_item) = Self::tab_item_from_index(&tab_control, i) else {
                continue;
            };

            let Some(page) = self.prepare_page_for_container(&tab_item, i) else {
                continue;
            };

            Self::sync_tab_item_with_page(&tab_item, &page);
        }

        self.update_active_page();
    }

    fn on_page_property_changed(&self, page: &Ref<Page>, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.tab_control().is_none() {
            return;
        }

        if e.property() == Page::icon_property().as_property() {
            if let Some(tab_item) = self.page_container(page) {
                tab_item.set_icon(page.icon());
            }
        } else if e.property() == Page::icon_template_property().as_property() {
            if let Some(tab_item) = self.page_container(page) {
                tab_item.set_icon_template(page.icon_template());
            }
        } else if e.property() == Page::header_property().as_property() {
            if let Some(tab_item) = self.page_container(page) {
                tab_item.set_header(page.header());
            }
        } else if e.property() == Self::is_tab_enabled_property().as_property() {
            self.sync_tab_enabled_state(page);
        }
    }

    fn find_nearest_enabled_tab(&self, disabled_index: i32) -> i32 {
        let count = self.get_tab_count();
        for dist in 1..count {
            let p = disabled_index - dist;
            if p >= 0 && self.is_tab_enabled_at_index(p) {
                return p;
            }
            let n = disabled_index + dist;
            if n < count && self.is_tab_enabled_at_index(n) {
                return n;
            }
        }

        -1
    }

    pub(crate) fn find_next_enabled_tab(&self, start: i32, direction: i32) -> i32 {
        let count = self.get_tab_count();
        let mut i = start;
        while i >= 0 && i < count {
            if self.is_tab_enabled_at_index(i) {
                return i;
            }
            i += direction;
        }

        -1
    }

    pub(crate) fn get_tab_count(&self) -> i32 {
        if let Some(tab_control) = self.tab_control() {
            return tab_control.item_count();
        }

        if let Some(source) = self.items_source() {
            return source.count() as i32;
        }

        self.pages().map_or(0, |pages| pages.count() as i32)
    }

    fn is_tab_enabled_at_index(&self, index: i32) -> bool {
        if let Some(page) = self
            .tab_control()
            .and_then(|tab_control| Self::tab_item_from_index(&tab_control, index))
            .and_then(|tab_item| self.container_page(&tab_item))
        {
            return Self::get_is_tab_enabled(&page);
        }
        if let Some(page) = self.resolve_tab_page_at_index(index) {
            return Self::get_is_tab_enabled(&page);
        }
        true
    }

    /// The page of the tab at `index`: the page of its container, else the
    /// item of the tab control if it is a page, else the page at the index
    /// of the `Pages` collection. (The reference hides the member of the
    /// base class of the same name with this one.)
    fn resolve_tab_page_at_index(&self, index: i32) -> Option<Ref<Page>> {
        let tab_control = self.tab_control();
        let tab_item = tab_control.as_ref().and_then(|tab_control| Self::tab_item_from_index(tab_control, index));

        if let Some(page) = tab_item.as_ref().and_then(|tab_item| self.container_page(tab_item)) {
            return Some(page);
        }

        if let Some(tab_control) = &tab_control {
            let items_view = tab_control.items_view();
            if index >= 0 && (index as usize) < items_view.count() {
                return items_view
                    .get_at(index as usize)
                    .as_ref()
                    .and_then(Control::from_boxed)
                    .and_then(|control| control.cast::<Page>());
            }
        }

        if let Some(page) = tab_item
            .and_then(|tab_item| tab_item.content())
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|control| control.cast::<Page>())
        {
            return Some(page);
        }

        self.resolve_page_at_index(index)
    }

    fn sync_tab_enabled_state(&self, page: &Page) {
        let Some(tab_control) = self.tab_control() else {
            return;
        };
        let Some(tab_item) = self.page_container(page) else {
            return;
        };

        tab_item.set_is_enabled(Self::get_is_tab_enabled(page));

        let selected_container = Self::tab_item_from_index(&tab_control, tab_control.selected_index());
        if !Self::get_is_tab_enabled(page) && selected_container.is_some_and(|selected| selected.ptr_eq(&tab_item)) {
            let i = tab_control.selected_index();
            let target = self.find_nearest_enabled_tab(i);
            if target >= 0 && target != i {
                tab_control.set_selected_index(target);
            }
        }
    }

    fn sync_all_tab_enabled_states(&self, tab_control: &TabControl) {
        for i in 0..tab_control.item_count() {
            if let Some(tab_item) = Self::tab_item_from_index(tab_control, i) {
                if let Some(page) = self.container_page(&tab_item) {
                    tab_item.set_is_enabled(Self::get_is_tab_enabled(&page));
                }
            }
        }
    }

    fn prepare_page_for_container(&self, tab_item: &Ref<TabItem>, index: i32) -> Option<Ref<Page>> {
        self.clear_container_page(tab_item);

        let page;

        if self.items_source().is_some() {
            let item = self.try_get_item_at_index(index)?;

            page = self
                .page_template()
                .and_then(|page_template| page_template.build(&item))
                .and_then(|control| control.cast::<Page>());
            tab_item.set_content(page.as_ref().map(|page| Control::boxed(page.clone())));

            let page = page?;

            self.template_created_pages.borrow_mut().insert(page_key(&page), page.clone());
            StyledElement::logical_children(self).add(page.clone().upcast());

            self.map_container_to_page(tab_item, &page);
            return Some(page);
        }

        let item = self.try_get_item_at_index(index)?;

        page = item.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<Page>());
        let page = page?;

        tab_item.set_content(Some(Control::boxed(page.clone())));

        self.map_container_to_page(tab_item, &page);
        Some(page)
    }

    fn map_container_to_page(&self, tab_item: &Ref<TabItem>, page: &Ref<Page>) {
        let weak = self.to_ref().downgrade();
        let weak_page = page.downgrade();
        let page_property_changed = page.property_changed(move |e| {
            if let (Some(this), Some(page)) = (weak.upgrade(), weak_page.upgrade()) {
                this.on_page_property_changed(&page, e);
            }
        });

        let replaced = self.container_page_map.borrow_mut().insert(
            tab_item_key(tab_item),
            ContainerPage { tab_item: tab_item.clone(), page: page.clone(), page_property_changed },
        );
        if let Some(replaced) = replaced {
            replaced.page_property_changed.dispose();
        }
        self.page_container_map.borrow_mut().insert(page_key(page), tab_item.clone());
    }

    fn clear_container_pages(&self) {
        let entries: Vec<ContainerPage> = self.container_page_map.borrow_mut().drain().map(|(_, entry)| entry).collect();
        for entry in &entries {
            entry.page_property_changed.dispose();
        }

        let template_created_pages: Vec<Ref<Page>> =
            self.template_created_pages.borrow_mut().drain().map(|(_, page)| page).collect();
        let logical_children = StyledElement::logical_children(self);
        for page in template_created_pages {
            logical_children.remove(&page.upcast());
        }

        self.page_container_map.borrow_mut().clear();
    }

    fn clear_container_page(&self, tab_item: &TabItem) {
        let entry = self.container_page_map.borrow_mut().remove(&tab_item_key(tab_item));
        let Some(entry) = entry else {
            return;
        };
        debug_assert!(std::ptr::eq::<TabItem>(&*entry.tab_item, tab_item));

        self.page_container_map.borrow_mut().remove(&page_key(&entry.page));
        entry.page_property_changed.dispose();

        let template_created = self.template_created_pages.borrow_mut().remove(&page_key(&entry.page));
        if let Some(page) = template_created {
            StyledElement::logical_children(self).remove(&page.upcast());
        }
    }

    /// The item at `index` of the items of the tab control, else of the
    /// items source or the pages. `None` when there is no such item.
    fn try_get_item_at_index(&self, index: i32) -> Option<Option<BoxedValue>> {
        if index < 0 {
            return None;
        }
        let index = index as usize;

        if let Some(tab_control) = self.tab_control() {
            let items_view = tab_control.items_view();
            if index < items_view.count() {
                return Some(items_view.get_at(index));
            }
        }

        if let Some(source) = self.items_source() {
            return (index < source.count()).then(|| source.get_at(index));
        }

        self.pages().and_then(|pages| pages.try_get(index)).map(|page| Some(Control::boxed(page)))
    }

    fn commit_selection_if_resolved(&self, index: i32, navigation_type: NavigationType) {
        let page = self.resolve_tab_page_at_index(index);

        if page.is_none()
            && self.items_source().is_some()
            && index >= 0
            && self.tab_control().and_then(|tab_control| tab_control.container_from_index(index)).is_none()
        {
            self.store_selected_index(index);
            return;
        }

        self.commit_selection(index, page, navigation_type);
    }

    fn on_swipe_gesture(&self, e: &SwipeGestureEventArgs) {
        if !self.is_gesture_enabled() {
            return;
        }
        let Some(tab_control) = self.tab_control() else {
            return;
        };
        if e.id() == self.last_swipe_gesture_id.get() {
            return;
        }

        let placement = self.resolve_tab_placement();
        let is_horizontal = placement == TabPlacement::Top || placement == TabPlacement::Bottom;
        let is_rtl = self.flow_direction() == FlowDirection::RightToLeft;

        let delta = match (e.swipe_direction(), is_horizontal, is_rtl) {
            (SwipeDirection::Left, true, false) => 1,
            (SwipeDirection::Right, true, false) => -1,
            (SwipeDirection::Left, true, true) => -1,
            (SwipeDirection::Right, true, true) => 1,
            (SwipeDirection::Up, false, _) => -1,
            (SwipeDirection::Down, false, _) => 1,
            _ => 0,
        };

        if delta == 0 {
            return;
        }

        let next = self.find_next_enabled_tab(tab_control.selected_index() + delta, delta);
        if next >= 0 {
            tab_control.set_selected_index(next);
            e.set_handled(true);
            self.last_swipe_gesture_id.set(e.id());
        }
    }

    fn focus_selected_tab_item(&self) {
        let Some(tab_control) = self.tab_control() else {
            return;
        };

        if let Some(container) = tab_control.container_from_index(tab_control.selected_index()) {
            container.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
        }
    }
}
