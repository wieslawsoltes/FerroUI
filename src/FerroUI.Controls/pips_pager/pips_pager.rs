use super::{PipsPagerSelectedIndexChangedEventArgs, PipsPagerTemplateSettings};
use crate::automation::AutomationProperties;
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{Button, Control, ControlImpl, ItemsSource, ListBox, ScrollViewer, StackPanel};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingMode;
use ferroui_base::input::{InputElementImpl, InputElementImplExt, Key, KeyEventArgs};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::{CancellationToken, CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
    DirectProperty, DirectPropertyMetadata, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable,
    Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const PART_PREVIOUS_BUTTON: &str = "PART_PreviousButton";
const PART_NEXT_BUTTON: &str = "PART_NextButton";
const PART_PIPS_PAGER_LIST: &str = "PART_PipsPagerList";

const PC_FIRST_PAGE: &str = ":first-page";
const PC_LAST_PAGE: &str = ":last-page";
const PC_VERTICAL: &str = ":vertical";
const PC_HORIZONTAL: &str = ":horizontal";

/// Represents a control that lets the user navigate through a paginated
/// collection using a set of pips.
#[repr(C)]
pub struct PipsPager {
    base: TemplatedControl,
    previous_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    next_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    pips_pager_list: RefCell<Option<(Ref<ListBox>, RoutedEventHandlerToken)>>,
    /// The `ContainerPrepared` / `ContainerIndexChanged` subscriptions on the pips list.
    pips_pager_list_container_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    scroll_pending: Cell<bool>,
    updating_pager_size: Cell<bool>,
    is_initial_load: Cell<bool>,
    last_selected_page_index: Cell<i32>,
    scroll_animation_cts: RefCell<Option<CancellationTokenSource>>,
    template_settings: RefCell<Ref<PipsPagerTemplateSettings>>,
}

ferro_class!(PipsPager: TemplatedControl);

ferro_class_info!(PipsPager {
    new: PipsPager::new,
    markup: {
        attributes: [
            TemplatePart("PART_PreviousButton", type(Ref<Button>)),
            TemplatePart("PART_NextButton", type(Ref<Button>)),
            TemplatePart("PART_PipsPagerList", type(Ref<ListBox>)),
            PseudoClasses(":first-page", ":last-page", ":vertical", ":horizontal"),
        ],
    },
});

ferro_impl_classes!(PipsPager: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for PipsPager {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes();
    }
}

impl TemplatedControlImpl for PipsPager {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if let Some(cts) = this.scroll_animation_cts.take() {
            cts.cancel();
        }
        this.is_initial_load.set(true);

        // Unsubscribe from previous button events
        if let Some((button, token)) = this.previous_button.take() {
            button.remove_handler(Button::click_event(), token);
        }

        if let Some((button, token)) = this.next_button.take() {
            button.remove_handler(Button::click_event(), token);
        }

        // Unsubscribe from previous list events
        if let Some((list, token)) = this.pips_pager_list.take() {
            list.remove_handler(Control::size_changed_event(), token);
            for subscription in this.pips_pager_list_container_subscriptions.take() {
                subscription.dispose();
            }
        }

        // Get template parts
        let previous_button = e.name_scope().find_as::<Button>(PART_PREVIOUS_BUTTON);
        let next_button = e.name_scope().find_as::<Button>(PART_NEXT_BUTTON);
        let pips_pager_list = e.name_scope().find_as::<ListBox>(PART_PIPS_PAGER_LIST);

        let weak = this.to_ref().downgrade();

        // Set up previous button
        if let Some(button) = previous_button {
            let weak = weak.clone();
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.previous_button_click();
                }
            });
            AutomationProperties::set_name(&button, Some("Previous page"));
            *this.previous_button.borrow_mut() = Some((button, token));
        }

        // Set up next button
        if let Some(button) = next_button {
            let weak = weak.clone();
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.next_button_click();
                }
            });
            AutomationProperties::set_name(&button, Some("Next page"));
            *this.next_button.borrow_mut() = Some((button, token));
        }

        // Set up pips list
        if let Some(list) = pips_pager_list {
            let size_changed_weak = weak.clone();
            let token = list.size_changed(move |_, _| {
                if let Some(this) = size_changed_weak.upgrade() {
                    this.on_pips_pager_list_size_changed();
                }
            });
            let prepared_weak = weak.clone();
            let prepared = list.container_prepared(move |e| {
                if let Some(this) = prepared_weak.upgrade() {
                    this.update_container_automation_properties(e.container(), e.index());
                }
            });
            let index_changed = list.container_index_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.update_container_automation_properties(e.container(), e.new_index());
                }
            });
            *this.pips_pager_list_container_subscriptions.borrow_mut() = vec![prepared, index_changed];
            *this.pips_pager_list.borrow_mut() = Some((list, token));
        }

        this.update_buttons_state();
        this.update_pseudo_classes();
        this.update_pager_size();
    }

    // AUTOMATION-SEAM: OnCreateAutomationPeer -> PipsPagerAutomationPeer (automation pass)
}

impl InputElementImpl for PipsPager {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.handled() {
            return;
        }

        let is_horizontal = this.orientation() == Orientation::Horizontal;

        match e.key {
            Key::Left | Key::Up if (e.key == Key::Left) == is_horizontal => {
                if this.selected_page_index() > 0 {
                    this.set_current_value(Self::selected_page_index_property(), this.selected_page_index() - 1);
                    e.set_handled(true);
                }
            }
            Key::Right | Key::Down if (e.key == Key::Right) == is_horizontal => {
                if this.selected_page_index() < this.number_of_pages() - 1 {
                    this.set_current_value(Self::selected_page_index_property(), this.selected_page_index() + 1);
                    e.set_handled(true);
                }
            }
            Key::Home => {
                this.set_current_value(Self::selected_page_index_property(), 0);
                e.set_handled(true);
            }
            Key::End => {
                if this.number_of_pages() > 0 {
                    this.set_current_value(Self::selected_page_index_property(), this.number_of_pages() - 1);
                    e.set_handled(true);
                }
            }
            _ => {}
        }
    }
}

ferro_properties! {
    impl PipsPager {
        /// Defines the `MaxVisiblePips` property.
        pub fn max_visible_pips_property() -> StyledProperty<i32> {
            FerroProperty::register::<PipsPager, _>("MaxVisiblePips", 5)
        }

        /// Defines the `IsNextButtonVisible` property.
        pub fn is_next_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<PipsPager, _>("IsNextButtonVisible", true)
        }

        /// Defines the `NumberOfPages` property.
        pub fn number_of_pages_property() -> StyledProperty<i32> {
            FerroProperty::register::<PipsPager, _>("NumberOfPages", 0)
        }

        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<PipsPager, _>("Orientation", Orientation::Horizontal)
        }

        /// Defines the `IsPreviousButtonVisible` property.
        pub fn is_previous_button_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<PipsPager, _>("IsPreviousButtonVisible", true)
        }

        /// Defines the `SelectedPageIndex` property.
        pub fn selected_page_index_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<PipsPager, _>(
                "SelectedPageIndex",
                StyledPropertyOptions::new(0).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `TemplateSettings` property.
        pub fn template_settings_property() -> DirectProperty<PipsPager, Ref<PipsPagerTemplateSettings>> {
            FerroProperty::register_direct_with::<PipsPager, _>(
                "TemplateSettings",
                |x| x.template_settings(),
                None,
                DirectPropertyMetadata::new(None),
            )
        }

        /// Defines the `PreviousButtonTheme` property.
        pub fn previous_button_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<PipsPager, _>("PreviousButtonTheme", None)
        }

        /// Defines the `NextButtonTheme` property.
        pub fn next_button_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<PipsPager, _>("NextButtonTheme", None)
        }
    }
}

impl PipsPager {
    ferro_routed_event!(
        /// Defines the `SelectedIndexChanged` event.
        pub fn selected_index_changed_event() -> RoutedEvent<PipsPagerSelectedIndexChangedEventArgs> {
            RoutedEvent::register::<PipsPager, _>("SelectedIndexChanged", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        // XAML-SEAM: not in the reference class. A binding that delivers the pips of the template
        // settings to an items source property (the binding of the control theme) needs the
        // conversion of the list handle; upstream gets it from the type system.
        ValueTypes::register_conversion::<FerroList<i32>, ItemsSource>(|pips| Some(Rc::new(pips.clone()).into()));
        ValueTypes::register_conversion::<FerroList<i32>, Option<ItemsSource>>(|pips| {
            Some(Some(Rc::new(pips.clone()).into()))
        });

        Self::selected_page_index_property()
            .changed()
            .add_class_handler::<PipsPager>(|x, e| x.on_selected_page_index_changed(e));
        Self::number_of_pages_property().changed().add_class_handler::<PipsPager>(|x, e| x.on_number_of_pages_changed(e));
        Self::is_previous_button_visible_property()
            .changed()
            .add_class_handler::<PipsPager>(|x, _| x.on_is_previous_button_visible_changed());
        Self::is_next_button_visible_property()
            .changed()
            .add_class_handler::<PipsPager>(|x, _| x.on_is_next_button_visible_changed());
        Self::orientation_property().changed().add_class_handler::<PipsPager>(|x, _| x.on_orientation_changed());
        Self::max_visible_pips_property()
            .changed()
            .add_class_handler::<PipsPager>(|x, e| x.on_max_visible_pips_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            previous_button: RefCell::new(None),
            next_button: RefCell::new(None),
            pips_pager_list: RefCell::new(None),
            pips_pager_list_container_subscriptions: RefCell::new(Vec::new()),
            scroll_pending: Cell::new(false),
            updating_pager_size: Cell::new(false),
            is_initial_load: Cell::new(false),
            last_selected_page_index: Cell::new(0),
            scroll_animation_cts: RefCell::new(None),
            template_settings: RefCell::new(PipsPagerTemplateSettings::new()),
        }
    }

    /// Initializes a new instance of the pips pager.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs when the selected index has changed.
    pub fn selected_index_changed(
        &self,
        handler: impl Fn(&Interactive, &PipsPagerSelectedIndexChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::selected_index_changed_event(), handler)
    }

    /// Gets or sets the maximum number of visible pips.
    pub fn max_visible_pips(&self) -> i32 {
        self.get_value(Self::max_visible_pips_property())
    }

    pub fn set_max_visible_pips(&self, value: i32) {
        self.set_value(Self::max_visible_pips_property(), value)
    }

    /// Gets or sets the visibility of the next button.
    pub fn is_next_button_visible(&self) -> bool {
        self.get_value(Self::is_next_button_visible_property())
    }

    pub fn set_is_next_button_visible(&self, value: bool) {
        self.set_value(Self::is_next_button_visible_property(), value)
    }

    fn update_container_automation_properties(&self, container: &Ref<Control>, index: i32) {
        AutomationProperties::set_name(container, Some(&format!("Page {}", index + 1)));
        AutomationProperties::set_position_in_set(container, index + 1);
        AutomationProperties::set_size_of_set(container, self.number_of_pages());
    }

    /// Gets or sets the number of pages.
    pub fn number_of_pages(&self) -> i32 {
        self.get_value(Self::number_of_pages_property())
    }

    pub fn set_number_of_pages(&self, value: i32) {
        self.set_value(Self::number_of_pages_property(), value)
    }

    /// Gets or sets the orientation of the pips.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// Gets or sets the visibility of the previous button.
    pub fn is_previous_button_visible(&self) -> bool {
        self.get_value(Self::is_previous_button_visible_property())
    }

    pub fn set_is_previous_button_visible(&self, value: bool) {
        self.set_value(Self::is_previous_button_visible_property(), value)
    }

    /// Gets or sets the current selected page index.
    pub fn selected_page_index(&self) -> i32 {
        self.get_value(Self::selected_page_index_property())
    }

    pub fn set_selected_page_index(&self, value: i32) {
        self.set_value(Self::selected_page_index_property(), value)
    }

    /// Gets the template settings.
    pub fn template_settings(&self) -> Ref<PipsPagerTemplateSettings> {
        self.template_settings.borrow().clone()
    }

    // The upstream private setter of `TemplateSettings` is never called: the
    // settings created with the control are kept for its lifetime.

    /// Gets or sets the theme for the previous button.
    pub fn previous_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::previous_button_theme_property())
    }

    pub fn set_previous_button_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::previous_button_theme_property(), value.into().0)
    }

    /// Gets or sets the theme for the next button.
    pub fn next_button_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::next_button_theme_property())
    }

    pub fn set_next_button_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::next_button_theme_property(), value.into().0)
    }

    fn previous_button_part(&self) -> Option<Ref<Button>> {
        self.previous_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn next_button_part(&self) -> Option<Ref<Button>> {
        self.next_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn pips_pager_list_part(&self) -> Option<Ref<ListBox>> {
        self.pips_pager_list.borrow().as_ref().map(|(list, _)| list.clone())
    }

    fn on_selected_page_index_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_index, new_index) = e.get_old_and_new_value::<i32>();

        if new_index < 0 {
            self.set_current_value(Self::selected_page_index_property(), 0);
            return;
        }

        if self.number_of_pages() > 0 {
            if new_index >= self.number_of_pages() {
                self.set_current_value(Self::selected_page_index_property(), self.number_of_pages() - 1);
                return;
            }
        } else if new_index > 0 {
            self.set_current_value(Self::selected_page_index_property(), 0);
            return;
        }

        self.last_selected_page_index.set(old_index);

        self.update_buttons_state();
        self.update_pseudo_classes();
        self.request_scroll_to_selected_pip();

        self.raise_event(&PipsPagerSelectedIndexChangedEventArgs::new(old_index, new_index));
    }

    fn on_number_of_pages_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<i32>();

        if new_value < 0 {
            self.set_current_value(Self::number_of_pages_property(), 0);
            return;
        }

        let pips = self.template_settings().pips();
        let count = pips.count() as i32;

        if count < new_value {
            let start = count + 1;
            pips.add_range(start..=new_value);
        } else if count > new_value {
            pips.remove_range(new_value as usize, (count - new_value) as usize);
        }

        let mut index_clamped = false;

        if new_value > 0 && self.selected_page_index() >= new_value {
            self.set_current_value(Self::selected_page_index_property(), new_value - 1);
            index_clamped = true;
        } else if new_value == 0 && self.selected_page_index() > 0 {
            self.set_current_value(Self::selected_page_index_property(), 0);
            index_clamped = true;
        }

        if !index_clamped {
            self.update_buttons_state();
            self.update_pseudo_classes();
        }

        self.update_pager_size();
    }

    fn on_is_previous_button_visible_changed(&self) {
        self.update_buttons_state();
    }

    fn on_is_next_button_visible_changed(&self) {
        self.update_buttons_state();
    }

    fn on_orientation_changed(&self) {
        self.update_pseudo_classes();
        self.update_pager_size();
    }

    fn on_max_visible_pips_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<i32>();

        if new_value < 1 {
            self.set_current_value(Self::max_visible_pips_property(), 1);
            return;
        }

        self.update_pager_size();
    }

    fn previous_button_click(&self) {
        if self.selected_page_index() > 0 {
            self.set_current_value(Self::selected_page_index_property(), self.selected_page_index() - 1);
        }
    }

    fn next_button_click(&self) {
        if self.selected_page_index() < self.number_of_pages() - 1 {
            self.set_current_value(Self::selected_page_index_property(), self.selected_page_index() + 1);
        }
    }

    fn update_buttons_state(&self) {
        if let Some(previous_button) = self.previous_button_part() {
            previous_button.set_is_enabled(self.selected_page_index() > 0);
        }

        if let Some(next_button) = self.next_button_part() {
            next_button.set_is_enabled(self.selected_page_index() < self.number_of_pages() - 1);
        }
    }

    fn update_pseudo_classes(&self) {
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(PC_FIRST_PAGE, self.selected_page_index() == 0);
        pseudo_classes
            .set(PC_LAST_PAGE, self.number_of_pages() > 0 && self.selected_page_index() >= self.number_of_pages() - 1);
        pseudo_classes.set(PC_VERTICAL, self.orientation() == Orientation::Vertical);
        pseudo_classes.set(PC_HORIZONTAL, self.orientation() == Orientation::Horizontal);
    }

    fn on_pips_pager_list_size_changed(&self) {
        if !self.updating_pager_size.get() {
            self.update_pager_size();
        }
    }

    fn request_scroll_to_selected_pip(&self) {
        if self.scroll_pending.get() {
            return;
        }

        self.scroll_pending.set(true);
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                this.scroll_pending.set(false);
                this.scroll_to_selected_pip();
            },
            DispatcherPriority::INPUT,
        );
    }

    fn scroll_to_selected_pip(&self) {
        let Some(list) = self.pips_pager_list_part() else {
            return;
        };

        if self.number_of_pages() <= self.max_visible_pips() {
            return;
        }

        let Some(scroll_viewer) = list.scroll().and_then(|scroll| scroll.cast::<ScrollViewer>()) else {
            return;
        };

        let Some(container) = list.container_from_index(self.selected_page_index()) else {
            return;
        };

        let is_horizontal = self.orientation() == Orientation::Horizontal;
        let margin = container.margin();
        let pip_size = if is_horizontal {
            container.bounds().width + margin.left + margin.right
        } else {
            container.bounds().height + margin.top + margin.bottom
        };

        if pip_size <= 0.0 {
            return;
        }

        let max_visible_pips = self.max_visible_pips();
        let even_offset =
            if max_visible_pips % 2 == 0 && self.selected_page_index() > self.last_selected_page_index.get() {
                1
            } else {
                0
            };
        let offset_elements = self.selected_page_index() + even_offset - max_visible_pips / 2;
        let mut target_offset = f64::max(0.0, f64::from(offset_elements) * pip_size);
        let max_offset = if is_horizontal {
            scroll_viewer.extent().width - scroll_viewer.viewport().width
        } else {
            scroll_viewer.extent().height - scroll_viewer.viewport().height
        };
        target_offset = f64::min(target_offset, f64::max(0.0, max_offset));

        if self.is_initial_load.get() {
            self.is_initial_load.set(false);
            Self::set_scroll_offset(&scroll_viewer, target_offset, is_horizontal);
            return;
        }

        self.animate_scroll_offset(&scroll_viewer, target_offset, is_horizontal);
    }

    fn set_scroll_offset(scroll_viewer: &Ref<ScrollViewer>, offset: f64, is_horizontal: bool) {
        let current = scroll_viewer.offset();
        scroll_viewer.set_offset(if is_horizontal {
            Vector::new(offset, current.y)
        } else {
            Vector::new(current.x, offset)
        });
    }

    fn animate_scroll_offset(&self, scroll_viewer: &Ref<ScrollViewer>, target_offset: f64, is_horizontal: bool) {
        if let Some(cts) = self.scroll_animation_cts.take() {
            cts.cancel();
        }
        let cts = CancellationTokenSource::new();
        let token = cts.token();
        *self.scroll_animation_cts.borrow_mut() = Some(cts);

        let start_offset = if is_horizontal { scroll_viewer.offset().x } else { scroll_viewer.offset().y };
        let delta = target_offset - start_offset;

        if delta.abs() < 0.5 {
            Self::set_scroll_offset(scroll_viewer, target_offset, is_horizontal);
            return;
        }

        const DURATION_MS: i32 = 200;
        const FRAME_MS: u64 = 16;
        // The tick count of upstream is the clock of the dispatcher here.
        let start_time = Dispatcher::ui_thread().now();

        let scroll_viewer = scroll_viewer.clone();
        DispatcherTimer::run_once(
            move || {
                Self::animate_step(&scroll_viewer, start_offset, delta, start_time, DURATION_MS, is_horizontal, &token)
            },
            Duration::from_millis(FRAME_MS),
            DispatcherPriority::RENDER,
        );
    }

    fn animate_step(
        scroll_viewer: &Ref<ScrollViewer>,
        start_offset: f64,
        delta: f64,
        start_time: i64,
        duration_ms: i32,
        is_horizontal: bool,
        token: &CancellationToken,
    ) {
        if token.is_cancellation_requested() {
            return;
        }

        let elapsed = Dispatcher::ui_thread().now() - start_time;
        let t = f64::min(1.0, elapsed as f64 / f64::from(duration_ms));
        let eased = 1.0 - (1.0 - t).powi(3);
        let current = start_offset + (delta * eased);

        Self::set_scroll_offset(scroll_viewer, current, is_horizontal);

        if t < 1.0 {
            let scroll_viewer = scroll_viewer.clone();
            let token = token.clone();
            DispatcherTimer::run_once(
                move || {
                    Self::animate_step(
                        &scroll_viewer,
                        start_offset,
                        delta,
                        start_time,
                        duration_ms,
                        is_horizontal,
                        &token,
                    )
                },
                Duration::from_millis(16),
                DispatcherPriority::RENDER,
            );
        }
    }

    fn update_pager_size(&self) {
        let Some(list) = self.pips_pager_list_part() else {
            return;
        };

        // `try`/`finally` of the reference: the flag is reset on every path.
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        self.updating_pager_size.set(true);
        let _reset = Reset(&self.updating_pager_size);
        self.update_pager_size_core(&list);
    }

    fn update_pager_size_core(&self, list: &Ref<ListBox>) {
        let mut pip_size = 12.0;

        let mut container = list.container_from_index(self.selected_page_index());

        if container.is_none() && list.items().count() > 0 {
            container = list.container_from_index(0);
        }

        if let Some(container) = container {
            let margin = container.margin();
            let size = if self.orientation() == Orientation::Horizontal {
                container.bounds().width + margin.left + margin.right
            } else {
                container.bounds().height + margin.top + margin.bottom
            };

            if size > 0.0 {
                pip_size = size;
            }
        }

        let mut spacing = 0.0;

        if let Some(items_panel) = list.items_panel_root().and_then(|panel| panel.cast::<StackPanel>()) {
            spacing = items_panel.spacing();
        }

        let visible_count = i32::min(self.number_of_pages(), self.max_visible_pips());

        if visible_count <= 0 {
            return;
        }

        let extent = (f64::from(visible_count) * pip_size) + (f64::from(visible_count - 1) * spacing);

        if self.orientation() == Orientation::Horizontal {
            list.set_width(extent);
            list.set_height(f64::NAN);
        } else {
            list.set_height(extent);
            list.set_width(f64::NAN);
        }

        self.request_scroll_to_selected_pip();
    }
}
