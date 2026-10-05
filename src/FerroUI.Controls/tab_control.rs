use crate::generators::RecycleKey;
use crate::metadata::TemplatePartAttribute;
use crate::presenters::{register_content_presenter_host, ContentPresenter, IContentPresenterHost, ItemsPresenter};
use crate::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, SelectingItemsControlImplExt, TemplateAppliedEventArgs,
    TemplatedControlImpl,
};
use crate::templates::{FuncTemplate, IDataTemplate, ITemplateOf};
use crate::{
    ContentControl, Control, ControlImpl, Dock, ItemsControl, ItemsControlImpl, ItemsControlImplExt, Panel,
    SelectionMode, TabItem, WrapPanel,
};
use ferroui_base::animation::IPageTransition;
use ferroui_base::collections::FerroList;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, KeyboardNavigation, KeyboardNavigationMode,
    NavigationMethod, PointerEventArgs, PointerUpdateKind,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, LayoutableImplExt, VerticalAlignment};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::{CompositeDisposable, IDisposable, ObservableExt};
use ferroui_base::threading::{
    CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTask, FerroSynchronizationContext,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    Size, StaticType, StyledElement, StyledElementImpl, StyledProperty, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// A tab control that displays a tab strip along with the content of the
/// selected tab.
#[repr(C)]
pub struct TabControl {
    base: SelectingItemsControl,
    selected_content: RefCell<Option<BoxedValue>>,
    selected_content_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    selected_item_subscriptions: RefCell<Option<CompositeDisposable>>,
    items_presenter_part: RefCell<Option<Ref<ItemsPresenter>>>,
    content_part: RefCell<Option<Ref<ContentPresenter>>>,
    content_presenter2: RefCell<Option<Ref<ContentPresenter>>>,
    previous_selected_index: Cell<i32>,
    current_transition: RefCell<Option<Rc<CancellationTokenSource>>>,
    should_animate: Cell<bool>,
    pending_forward: Cell<bool>,
}

ferro_class! {
    TabControl: SelectingItemsControl, virtuals TabControlImpl: SelectingItemsControlImpl {
        /// Called when a [`ContentPresenter`] is registered with the
        /// control.
        fn register_content_presenter(this, presenter: &ContentPresenter) -> bool;
    }
}
ferroui_base::ferro_class_info!(TabControl { new: TabControl::new });

ferro_impl_classes!(TabControl: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for TabControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::tab_strip_placement_property().as_property() {
            this.refresh_containers();
        } else if change.property() == Self::indicator_template_property().as_property() {
            this.update_indicator_template();
        } else if change.property() == Self::content_template_property().as_property() {
            let new_template = change.get_new_value::<Option<Rc<dyn IDataTemplate>>>();
            if this.selected_content_template() != new_template {
                if let Some(container) = this.container_from_index(this.selected_index()) {
                    if container.get_value(ContentControl::content_template_property()).is_none() {
                        this.set_selected_content_template(new_template.clone());
                        if !this.should_animate.get() {
                            if let Some(content_part) = this.content_part() {
                                content_part.set_content_template(new_template);
                            }
                        }
                    }
                }
            }
        } else if change.property() == KeyboardNavigation::tab_once_active_element_property().as_property() {
            if let Some(panel) = this.items_presenter_part().and_then(|presenter| presenter.panel()) {
                KeyboardNavigation::set_tab_once_active_element(
                    &panel,
                    change.get_new_value::<Option<Ref<InputElement>>>(),
                );
            }
        }
    }
}

impl VisualImpl for TabControl {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if let Some(current_transition) = this.current_transition.take() {
            current_transition.cancel();
        }
        this.should_animate.set(false);
        Self::reset_transition_presenter(this.content_presenter2().as_ref());
    }
}

impl LayoutableImpl for TabControl {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);

        if this.should_animate.get() {
            this.should_animate.set(false);
            if let Some(current_transition) = this.current_transition.take() {
                current_transition.cancel();
            }
            let cancel = Rc::new(CancellationTokenSource::new());
            *this.current_transition.borrow_mut() = Some(cancel.clone());
            let from = this.content_part();
            let to = this.content_presenter2();
            if let (Some(from), Some(to), Some(page_transition)) = (from, to, this.page_transition()) {
                this.run_transition_async(page_transition, from, to, this.pending_forward.get(), cancel);
            }
        }

        result
    }
}

impl TemplatedControlImpl for TabControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let items_presenter_part = e.name_scope().find_as::<ItemsPresenter>("PART_ItemsPresenter");
        *this.items_presenter_part.borrow_mut() = items_presenter_part.clone();
        if let Some(items_presenter_part) = &items_presenter_part {
            items_presenter_part.apply_template();
        }

        this.update_tab_strip_placement();

        if let Some(panel) = items_presenter_part.and_then(|presenter| presenter.panel()) {
            if !panel.is_set(KeyboardNavigation::tab_navigation_property().as_property()) {
                panel.set_current_value(KeyboardNavigation::tab_navigation_property(), KeyboardNavigationMode::Once);
            }
            KeyboardNavigation::set_tab_once_active_element(
                &panel,
                KeyboardNavigation::get_tab_once_active_element(this),
            );
        }
    }
}

impl ItemsControlImpl for TabControl {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        TabItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<TabItem>(item)
    }

    fn prepare_container_for_item_override(
        this: &Self,
        element: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_prepare_container_for_item_override(this, element, item, index);

        if let Some(tab_item) = element.downcast_ref::<TabItem>() {
            tab_item.set_tab_strip_placement(Some(this.tab_strip_placement()));

            if let Some(tmpl) = this.indicator_template() {
                if !tab_item.is_set(TabItem::indicator_template_property().as_property()) {
                    tab_item.set_current_value(TabItem::indicator_template_property(), Some(tmpl));
                }
            }
        }

        if index == this.selected_index() {
            this.update_selected_content(Some(element.clone()));
        }
    }

    fn container_index_changed_override(this: &Self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        Self::parent_container_index_changed_override(this, container, old_index, new_index);

        let selected_index = this.selected_index();

        if selected_index == old_index || selected_index == new_index {
            this.update_selected_content(None);
        }
    }

    fn clear_container_for_item_override(this: &Self, element: &Ref<Control>) {
        Self::parent_clear_container_for_item_override(this, element);
        this.update_selected_content(None);
    }
}

impl SelectingItemsControlImpl for TabControl {
    fn should_trigger_selection(this: &Self, selectable: &Visual, event_args: &PointerEventArgs) -> bool {
        matches!(
            event_args.properties().pointer_update_kind,
            PointerUpdateKind::LeftButtonPressed | PointerUpdateKind::LeftButtonReleased
        ) && Self::parent_should_trigger_selection(this, selectable, event_args)
    }

    fn update_selection_from_event(this: &Self, container: &Ref<Control>, event_args: &dyn IRoutedEventArgs) -> bool {
        if let Some(focus) = event_args.downcast_ref::<FocusChangedEventArgs>() {
            if focus.navigation_method != NavigationMethod::Directional {
                return false;
            }
        }

        Self::parent_update_selection_from_event(this, container, event_args)
    }
}

impl TabControlImpl for TabControl {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        let name = presenter.name();

        if name.as_deref() == Some("PART_SelectedContentHost") {
            *this.content_part.borrow_mut() = Some(presenter.to_ref());
            presenter.set_content(this.selected_content());
            presenter.set_content_template(this.selected_content_template());
            return true;
        }

        if name.as_deref() == Some("PART_SelectedContentHost2") {
            *this.content_presenter2.borrow_mut() = Some(presenter.to_ref());
            presenter.set_is_visible(false);
            return true;
        }

        false
    }
}

impl IContentPresenterHost for TabControl {
    fn logical_children(&self) -> &FerroList<Ref<StyledElement>> {
        StyledElement::logical_children(self)
    }

    fn register_content_presenter(&self, presenter: &ContentPresenter) -> bool {
        TabControl::register_content_presenter(self, presenter)
    }
}

impl TabControl {
    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_ItemsPresenter", <ItemsPresenter as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SelectedContentHost", <ContentPresenter as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SelectedContentHost2", <ContentPresenter as StaticType>::TYPE),
    ];
}

ferroui_base::ferro_properties! { impl TabControl {
    ferro_property!(
        /// Defines the `TabStripPlacement` property.
        pub fn tab_strip_placement_property() -> StyledProperty<Dock> {
            FerroProperty::register::<TabControl, _>("TabStripPlacement", Dock::Top)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<TabControl>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<TabControl>()
        }
    );

    ferro_property!(
        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            ContentControl::content_template_property().add_owner::<TabControl>()
        }
    );

    ferro_property!(
        /// Defines the `SelectedContent` property.
        pub fn selected_content_property() -> DirectProperty<TabControl, Option<BoxedValue>> {
            FerroProperty::register_direct::<TabControl, _>("SelectedContent", |o| o.selected_content(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `SelectedContentTemplate` property.
        pub fn selected_content_template_property() -> DirectProperty<TabControl, Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register_direct::<TabControl, _>(
                "SelectedContentTemplate",
                |o| o.selected_content_template(),
                None,
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `PageTransition` property.
        pub fn page_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<TabControl, _>("PageTransition", None)
        }
    );

    ferro_property!(
        /// Defines the `IndicatorTemplate` property.
        pub fn indicator_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabControl, _>("IndicatorTemplate", None)
        }
    );
} }

impl TabControl {
    /// Initializes static members of the [`TabControl`] class.
    fn static_constructor() {
        register_content_presenter_host::<TabControl>();

        // The default value for the `ItemsPanel` property.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(WrapPanel::new().upcast::<Panel>()));

        SelectingItemsControl::selection_mode_property()
            .override_default_value::<TabControl>(SelectionMode::ALWAYS_SELECTED);
        ItemsControl::items_panel_property().override_default_value::<TabControl>(default_panel);
        Layoutable::affects_measure::<TabControl>(&[Self::tab_strip_placement_property().as_property()]);
        SelectingItemsControl::selected_item_property()
            .changed()
            .add_class_handler::<TabControl>(|x, _| x.update_selected_content(None));
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<TabControl>(Some(crate::automation::peers::AutomationControlType::Tab));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: SelectingItemsControl::construct(),
            selected_content: RefCell::new(None),
            selected_content_template: RefCell::new(None),
            selected_item_subscriptions: RefCell::new(None),
            items_presenter_part: RefCell::new(None),
            content_part: RefCell::new(None),
            content_presenter2: RefCell::new(None),
            previous_selected_index: Cell::new(-1),
            current_transition: RefCell::new(None),
            should_animate: Cell::new(false),
            pending_forward: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the horizontal alignment of the content within the
    /// control.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// Gets or sets the vertical alignment of the content within the
    /// control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// Gets or sets the tabstrip placement of the [`TabControl`].
    pub fn tab_strip_placement(&self) -> Dock {
        self.get_value(Self::tab_strip_placement_property())
    }

    pub fn set_tab_strip_placement(&self, value: Dock) {
        self.set_value(Self::tab_strip_placement_property(), value)
    }

    /// Gets or sets the default data template used to display the content
    /// of the selected tab.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }

    /// Gets the content of the selected tab.
    pub fn selected_content(&self) -> Option<BoxedValue> {
        self.selected_content.borrow().clone()
    }

    pub(crate) fn set_selected_content(&self, value: Option<BoxedValue>) {
        self.set_and_raise(Self::selected_content_property(), &self.selected_content, value);
    }

    /// Gets the content template for the selected tab.
    pub fn selected_content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.selected_content_template.borrow().clone()
    }

    pub(crate) fn set_selected_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_and_raise(Self::selected_content_template_property(), &self.selected_content_template, value);
    }

    /// Gets or sets the page transition to use when switching tabs.
    pub fn page_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::page_transition_property())
    }

    pub fn set_page_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::page_transition_property(), value)
    }

    /// Gets or sets the data template used to render the selection
    /// indicator on each tab item.
    pub fn indicator_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::indicator_template_property())
    }

    pub fn set_indicator_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::indicator_template_property(), value)
    }

    pub(crate) fn items_presenter_part(&self) -> Option<Ref<ItemsPresenter>> {
        self.items_presenter_part.borrow().clone()
    }

    pub(crate) fn content_part(&self) -> Option<Ref<ContentPresenter>> {
        self.content_part.borrow().clone()
    }

    fn content_presenter2(&self) -> Option<Ref<ContentPresenter>> {
        self.content_presenter2.borrow().clone()
    }

    /// Replaces the presenters that display the selected content. The
    /// reference tests set the two private fields through reflection.
    #[cfg(test)]
    pub(crate) fn set_content_presenters_for_test(
        &self,
        content_part: Option<Ref<ContentPresenter>>,
        content_presenter2: Option<Ref<ContentPresenter>>,
    ) {
        *self.content_part.borrow_mut() = content_part;
        *self.content_presenter2.borrow_mut() = content_presenter2;
    }

    fn select_content_template(
        &self,
        container_template: Option<Rc<dyn IDataTemplate>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        container_template.or_else(|| self.content_template())
    }

    fn update_selected_content(&self, container: Option<Ref<Control>>) {
        if let Some(subscriptions) = self.selected_item_subscriptions.take() {
            subscriptions.dispose();
        }

        if let Some(current_transition) = self.current_transition.take() {
            current_transition.cancel();
        }
        self.should_animate.set(false);

        Self::reset_transition_presenter(self.content_presenter2().as_ref());

        let old_index = self.previous_selected_index.get();
        self.previous_selected_index.set(self.selected_index());
        let forward = self.selected_index() >= old_index || old_index < 0;

        if self.selected_index() == -1 {
            self.set_selected_content_template(None);
            self.set_selected_content(None);
            if let Some(content_part) = self.content_part() {
                content_part.set_content(None);
                content_part.set_content_template(None);
                content_part.set_data_context(None);
            }
            return;
        }

        let container = container.or_else(|| self.container_from_index(self.selected_index()));
        let Some(container) = container else { return };

        if self.selected_content_template()
            != self.select_content_template(container.get_value(ContentControl::content_template_property()))
        {
            self.set_selected_content_template(None);
            if let Some(content_part) = self.content_part() {
                content_part.set_content_template(None);
            }
        }

        let should_transition = self.page_transition().is_some()
            && self.content_presenter2().is_some()
            && self.visual_root().is_some()
            && old_index >= 0
            && old_index != self.selected_index();
        let is_initial_fire = Rc::new(Cell::new(true));

        // The handlers are held by the container: both the tab control and
        // the container are referenced weakly.
        let weak = self.to_ref().downgrade();
        let weak_container = container.downgrade();
        let container_object: &FerroObject = &container;

        let content_subscription = {
            let (weak, weak_container) = (weak.clone(), weak_container.clone());
            let observable =
                FerroObjectExtensions::get_observable(container_object, ContentControl::content_property());
            observable.subscribe_fn(move |content| {
                let (Some(this), Some(container)) = (weak.upgrade(), weak_container.upgrade()) else { return };

                this.set_selected_content(content.clone());

                let template =
                    this.select_content_template(container.get_value(ContentControl::content_template_property()));
                let control = match &template {
                    None => content.as_ref().and_then(Control::from_boxed),
                    Some(_) => None,
                };

                if is_initial_fire.get() && should_transition {
                    this.set_selected_content_template(template.clone());

                    let content_presenter2 =
                        this.content_presenter2().expect("the transition presenter is registered");
                    content_presenter2.set_content_template(template);
                    content_presenter2.set_is_visible(true);

                    match (&content, &control) {
                        (Some(content), Some(control)) => {
                            this.set_control_content(&content_presenter2, content, control, container.data_context())
                        }
                        _ => content_presenter2.set_content(content),
                    }

                    this.pending_forward.set(forward);
                    this.should_animate.set(true);
                    this.invalidate_arrange();
                } else if let Some(content_part) = this.content_part() {
                    match (&content, &control) {
                        (Some(content), Some(control)) => {
                            this.set_control_content(&content_part, content, control, container.data_context())
                        }
                        _ => content_part.set_content(content),
                    }
                }

                is_initial_fire.set(false);
            })
        };

        let content_template_subscription = {
            let weak = weak.clone();
            let observable =
                FerroObjectExtensions::get_observable(container_object, ContentControl::content_template_property());
            observable.subscribe_fn(move |v| {
                let Some(this) = weak.upgrade() else { return };

                this.set_selected_content_template(this.select_content_template(v));
                if !this.should_animate.get() {
                    if let Some(content_part) = this.content_part() {
                        content_part.set_content_template(this.selected_content_template());
                    }
                }
            })
        };

        let data_context_subscription = {
            let observable =
                FerroObjectExtensions::get_observable(container_object, StyledElement::data_context_property());
            observable.subscribe_fn(move |dc| {
                let Some(this) = weak.upgrade() else { return };

                // During a transition, the content part holds the old tab's
                // content and the second content presenter holds the new
                // tab's content. Only update the presenter that is showing
                // this container's content. Only override the data context
                // when there's no content template; with a template, the
                // presenter's data context should be the content itself (so
                // the template can bind to it).
                let has_template_less_control_content = |presenter: &ContentPresenter| {
                    presenter.content().as_ref().and_then(Control::from_boxed).is_some()
                        && presenter.content_template().is_none()
                };

                match this.content_presenter2().filter(|presenter| presenter.is_visible()) {
                    Some(content_presenter2) => {
                        if has_template_less_control_content(&content_presenter2) {
                            content_presenter2.set_data_context(dc);
                        }
                    }
                    None => {
                        if let Some(content_part) = this.content_part() {
                            if has_template_less_control_content(&content_part) {
                                content_part.set_data_context(dc);
                            }
                        }
                    }
                }
            })
        };

        *self.selected_item_subscriptions.borrow_mut() = Some(CompositeDisposable::from_disposables([
            content_subscription,
            content_template_subscription,
            data_context_subscription,
        ]));
    }

    fn run_transition_async(
        &self,
        transition: Rc<dyn IPageTransition>,
        from: Ref<ContentPresenter>,
        to: Ref<ContentPresenter>,
        forward: bool,
        cts: Rc<CancellationTokenSource>,
    ) {
        let this = self.to_ref().downgrade();
        let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
            FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
        });

        // The transition is started before the first await, as part of the
        // call.
        let from_visual: Ref<Visual> = from.clone().upcast();
        let to_visual: Ref<Visual> = to.upcast();
        let task = transition.start(Some(&from_visual), Some(&to_visual), forward, cts.token());

        let _ = context.to_task_scheduler().start_local(async move {
            match (TransitionOutcomeFuture { task }).await {
                TransitionOutcome::Canceled => return,
                TransitionOutcome::Faulted => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::CONTROL) {
                        let source = this.upgrade();
                        logger.log(
                            source.as_ref().map(|source| source as &dyn Any),
                            "Tab transition threw an unhandled exception.",
                        );
                    }
                }
                TransitionOutcome::Completed => {}
            }

            if cts.is_cancellation_requested() {
                return;
            }

            from.set_is_visible(false);
            from.set_content(None);
            from.set_content_template(None);
            from.set_data_context(None);
            from.set_render_transform(None);
            from.set_opacity(1.0);

            if let Some(this) = this.upgrade() {
                let content_part = this.content_part.take();
                let content_presenter2 = this.content_presenter2.replace(content_part);
                *this.content_part.borrow_mut() = content_presenter2;
            }
        });
    }

    fn set_control_content(
        &self,
        presenter: &Ref<ContentPresenter>,
        content: &BoxedValue,
        control: &Ref<Control>,
        data_context: Option<BoxedValue>,
    ) {
        self.clear_owning_content_presenter(presenter, control);
        Self::clear_presenter_content_unless(self.content_part().as_ref(), presenter, control);
        Self::clear_presenter_content_unless(self.content_presenter2().as_ref(), presenter, control);
        presenter.set_content_with_data_context(Some(content.clone()), data_context);
    }

    fn clear_owning_content_presenter(&self, target_presenter: &Ref<ContentPresenter>, content: &Ref<Control>) {
        let parent = content.visual_parent().and_then(|parent| parent.cast::<ContentPresenter>());
        let Some(parent) = parent else { return };
        let this: Ref<FerroObject> = self.to_ref().upcast();

        if &parent != target_presenter && parent.host() == Some(this) && Self::has_content(&parent, content) {
            Self::clear_presenter_content(Some(&parent), content);
        }
    }

    fn clear_presenter_content_unless(
        presenter: Option<&Ref<ContentPresenter>>,
        target_presenter: &Ref<ContentPresenter>,
        content: &Ref<Control>,
    ) {
        if presenter != Some(target_presenter) {
            Self::clear_presenter_content(presenter, content);
        }
    }

    fn clear_presenter_content(presenter: Option<&Ref<ContentPresenter>>, content: &Ref<Control>) {
        if let Some(presenter) = presenter.filter(|presenter| Self::has_content(presenter, content)) {
            presenter.set_content(None);
            presenter.set_content_template(None);
            presenter.set_data_context(None);
            presenter.set_render_transform(None);
            presenter.set_opacity(1.0);
        }
    }

    /// Whether the content of the presenter is the given control.
    fn has_content(presenter: &ContentPresenter, content: &Ref<Control>) -> bool {
        presenter.content().as_ref().and_then(Control::from_boxed).as_ref() == Some(content)
    }

    fn reset_transition_presenter(presenter: Option<&Ref<ContentPresenter>>) {
        let Some(presenter) = presenter else { return };

        presenter.set_is_visible(false);
        presenter.set_content(None);
        presenter.set_content_template(None);
        presenter.set_data_context(None);
        presenter.set_render_transform(None);
        presenter.set_opacity(1.0);
    }

    fn update_tab_strip_placement(&self) {
        let Some(panel) = self.items_presenter_part().and_then(|presenter| presenter.panel()) else { return };

        for control in panel.children().snapshot().iter() {
            if let Some(tab_item) = control.downcast_ref::<TabItem>() {
                tab_item.set_tab_strip_placement(Some(self.tab_strip_placement()));
            }
        }
    }

    fn update_indicator_template(&self) {
        let Some(panel) = self.items_presenter_part().and_then(|presenter| presenter.panel()) else { return };

        let template = self.indicator_template();
        for control in panel.children().snapshot().iter() {
            if let Some(tab_item) = control.downcast_ref::<TabItem>() {
                if !tab_item.is_set(TabItem::indicator_template_property().as_property()) {
                    match &template {
                        Some(template) => tab_item
                            .set_current_value(TabItem::indicator_template_property(), Some(template.clone())),
                        None => tab_item.clear_value(TabItem::indicator_template_property()),
                    }
                }
            }
        }
    }
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
