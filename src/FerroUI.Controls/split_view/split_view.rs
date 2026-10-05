use super::{SplitViewDisplayMode, SplitViewPanePlacement, SplitViewTemplateSettings};
use crate::presenters::ContentPresenter;
use crate::primitives::{PopupRoot, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{
    ContentControl, ContentControlImpl, ContentControlImplExt, Control, ControlImpl, GridLength, GridUnitType, Panel, TopLevel,
};
use ferroui_base::input::{InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::{
    CancelRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate, BoxedValue,
    DirectProperty, DirectPropertyMetadata, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_OPEN: &str = ":open";
const PC_CLOSED: &str = ":closed";
const PC_COMPACT_OVERLAY: &str = ":compactoverlay";
const PC_COMPACT_INLINE: &str = ":compactinline";
const PC_OVERLAY: &str = ":overlay";
const PC_INLINE: &str = ":inline";
const PC_LEFT: &str = ":left";
const PC_RIGHT: &str = ":right";
const PC_TOP: &str = ":top";
const PC_BOTTOM: &str = ":bottom";
const PC_LIGHT_DISMISS: &str = ":lightDismiss";

/// A control with two views: a collapsible pane and an area for content.
#[repr(C)]
pub struct SplitView {
    base: ContentControl,
    pane: RefCell<Option<Ref<Panel>>>,
    pointer_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    template_settings: RefCell<Ref<SplitViewTemplateSettings>>,
    last_display_mode_pseudoclass: Cell<Option<&'static str>>,
    last_placement_pseudoclass: Cell<Option<&'static str>>,
}

ferro_class! {
    SplitView: ContentControl, virtuals SplitViewImpl: ContentControlImpl {
        /// Raises the `PaneOpening` event.
        fn on_pane_opening(this, args: &CancelRoutedEventArgs);
        /// Raises the `PaneOpened` event.
        fn on_pane_opened(this, args: &RoutedEventArgs);
        /// Raises the `PaneClosing` event.
        fn on_pane_closing(this, args: &CancelRoutedEventArgs);
        /// Raises the `PaneClosed` event.
        fn on_pane_closed(this, args: &RoutedEventArgs);
        /// Called when the `IsPaneOpen` property has to be coerced.
        fn on_coerce_is_pane_open(this, value: bool) -> bool;
    }
}

ferro_class_info!(SplitView {
    new: SplitView::new,
    markup: {
        property_attributes: [Pane: [DependsOn("PaneTemplate")]],
        attributes: [
            TemplatePart("PART_PaneRoot", type(Ref<Panel>)),
            PseudoClasses(":open", ":closed"),
            PseudoClasses(":compactoverlay", ":compactinline", ":overlay", ":inline"),
            PseudoClasses(":left", ":right", ":top", ":bottom"),
            PseudoClasses(":lightDismiss"),
        ],
    },
});

ferro_impl_classes!(SplitView: StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for SplitView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::compact_pane_length_property().as_property() {
            this.update_visual_state_for_compact_pane_length(change.get_new_value::<f64>());
        } else if change.property() == Self::display_mode_property().as_property() {
            this.update_visual_state_for_display_mode(change.get_new_value::<SplitViewDisplayMode>());
        } else if change.property() == Self::is_pane_open_property().as_property() {
            let is_pane_open = change.get_new_value::<bool>();
            this.update_pane_state_pseudo_class(is_pane_open);
            if is_pane_open {
                this.on_pane_opened(&RoutedEventArgs::with_event_and_source(Self::pane_opened_event(), this.to_ref()));
            } else {
                this.on_pane_closed(&RoutedEventArgs::with_event_and_source(Self::pane_closed_event(), this.to_ref()));
            }
        } else if change.property() == Self::pane_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<BoxedValue>>();

            if let Some(old_child) = old_value.as_ref().and_then(Control::logical_from_boxed) {
                StyledElement::logical_children(this).remove(&old_child);
            }

            if let Some(new_child) = new_value.as_ref().and_then(Control::logical_from_boxed) {
                StyledElement::logical_children(this).add(new_child);
            }
        } else if change.property() == Self::pane_placement_property().as_property() {
            this.update_visual_state_for_pane_placement_property(change.get_new_value::<SplitViewPanePlacement>());
        } else if change.property() == Self::use_light_dismiss_overlay_mode_property().as_property() {
            let mode = change.get_new_value::<bool>();
            this.pseudo_classes().set(PC_LIGHT_DISMISS, mode);
        }
    }
}

impl VisualImpl for SplitView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        // The :left and :right style triggers contain the template so we
        // need to do this as soon as we're attached so the template applies.
        // The other visual states can be updated after the template applies.
        this.update_visual_state_for_pane_placement_property(this.pane_placement());
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.dispose_pointer_disposable();
    }
}

impl InputElementImpl for SplitView {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if !e.handled() && e.key == Key::Escape && this.is_pane_open() && this.is_in_overlay_mode() {
            this.set_current_value(Self::is_pane_open_property(), false);
            e.set_handled(true);
        }

        Self::parent_on_key_down(this, e);
    }
}

impl TemplatedControlImpl for SplitView {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let pane = e.name_scope().find_as::<Panel>("PART_PaneRoot");
        *this.pane.borrow_mut() = pane;

        this.update_visual_state_for_display_mode(this.display_mode());
        this.update_pane_state_pseudo_class(this.is_pane_open());
    }
}

impl ContentControlImpl for SplitView {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        let result = Self::parent_register_content_presenter(this, presenter);

        if presenter.name().as_deref() == Some("PART_PanePresenter") {
            return true;
        }

        result
    }
}

impl SplitViewImpl for SplitView {
    fn on_pane_opening(this: &Self, args: &CancelRoutedEventArgs) {
        this.raise_event(args);
    }

    fn on_pane_opened(this: &Self, args: &RoutedEventArgs) {
        this.invalidate_light_dismiss_subscription();
        this.raise_event(args);
    }

    fn on_pane_closing(this: &Self, args: &CancelRoutedEventArgs) {
        this.raise_event(args);
    }

    fn on_pane_closed(this: &Self, args: &RoutedEventArgs) {
        this.dispose_pointer_disposable();
        this.raise_event(args);
    }

    fn on_coerce_is_pane_open(this: &Self, value: bool) -> bool {
        let event_args = if value {
            let event_args = CancelRoutedEventArgs::with_event_and_source(Self::pane_opening_event(), this.to_ref());
            this.on_pane_opening(&event_args);
            event_args
        } else {
            let event_args = CancelRoutedEventArgs::with_event_and_source(Self::pane_closing_event(), this.to_ref());
            this.on_pane_closing(&event_args);
            event_args
        };

        if event_args.cancel() {
            return !value;
        }

        value
    }
}

ferro_properties! {
    impl SplitView {
        /// Defines the `CompactPaneLength` property.
        pub fn compact_pane_length_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitView, _>("CompactPaneLength", 48.0)
        }

        /// Defines the `DisplayMode` property.
        pub fn display_mode_property() -> StyledProperty<SplitViewDisplayMode> {
            FerroProperty::register::<SplitView, _>("DisplayMode", SplitViewDisplayMode::Overlay)
        }

        /// Defines the `IsPaneOpen` property.
        pub fn is_pane_open_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<SplitView, _>(
                "IsPaneOpen",
                StyledPropertyOptions::new(false).coerce(SplitView::coerce_is_pane_open),
            )
        }

        /// Defines the `OpenPaneLength` property.
        pub fn open_pane_length_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitView, _>("OpenPaneLength", 320.0)
        }

        /// Defines the `PaneBackground` property.
        pub fn pane_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<SplitView, _>("PaneBackground", None)
        }

        /// Defines the `PanePlacement` property.
        pub fn pane_placement_property() -> StyledProperty<SplitViewPanePlacement> {
            FerroProperty::register::<SplitView, _>("PanePlacement", SplitViewPanePlacement::Left)
        }

        /// Defines the `Pane` property.
        pub fn pane_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<SplitView, _>("Pane", None)
        }

        /// Defines the `PaneTemplate` property.
        pub fn pane_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<SplitView, _>("PaneTemplate", None)
        }

        /// Defines the `UseLightDismissOverlayMode` property.
        pub fn use_light_dismiss_overlay_mode_property() -> StyledProperty<bool> {
            FerroProperty::register::<SplitView, _>("UseLightDismissOverlayMode", false)
        }

        /// Defines the `TemplateSettings` property.
        pub fn template_settings_property() -> DirectProperty<SplitView, Ref<SplitViewTemplateSettings>> {
            FerroProperty::register_direct_with::<SplitView, _>(
                "TemplateSettings",
                |x| x.template_settings(),
                None,
                DirectPropertyMetadata::new(None),
            )
        }
    }
}

impl SplitView {
    ferro_routed_event!(
        /// Defines the `PaneClosed` event.
        pub fn pane_closed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<SplitView, _>("PaneClosed", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PaneClosing` event.
        pub fn pane_closing_event() -> RoutedEvent<CancelRoutedEventArgs> {
            RoutedEvent::register::<SplitView, _>("PaneClosing", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PaneOpened` event.
        pub fn pane_opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<SplitView, _>("PaneOpened", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PaneOpening` event.
        pub fn pane_opening_event() -> RoutedEvent<CancelRoutedEventArgs> {
            RoutedEvent::register::<SplitView, _>("PaneOpening", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            pane: RefCell::new(None),
            pointer_disposable: RefCell::new(None),
            template_settings: RefCell::new(SplitViewTemplateSettings::new()),
            last_display_mode_pseudoclass: Cell::new(None),
            last_placement_pseudoclass: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The length of the pane when in the compact overlay or the compact
    /// inline mode.
    pub fn compact_pane_length(&self) -> f64 {
        self.get_value(Self::compact_pane_length_property())
    }

    pub fn set_compact_pane_length(&self, value: f64) {
        self.set_value(Self::compact_pane_length_property(), value)
    }

    /// The display mode of the split view.
    pub fn display_mode(&self) -> SplitViewDisplayMode {
        self.get_value(Self::display_mode_property())
    }

    pub fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.set_value(Self::display_mode_property(), value)
    }

    /// Whether the pane is open or closed.
    pub fn is_pane_open(&self) -> bool {
        self.get_value(Self::is_pane_open_property())
    }

    pub fn set_is_pane_open(&self, value: bool) {
        self.set_value(Self::is_pane_open_property(), value)
    }

    /// The length of the pane when open.
    pub fn open_pane_length(&self) -> f64 {
        self.get_value(Self::open_pane_length_property())
    }

    pub fn set_open_pane_length(&self, value: f64) {
        self.set_value(Self::open_pane_length_property(), value)
    }

    /// The background of the pane.
    pub fn pane_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::pane_background_property())
    }

    pub fn set_pane_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::pane_background_property(), value)
    }

    /// The placement of the pane.
    pub fn pane_placement(&self) -> SplitViewPanePlacement {
        self.get_value(Self::pane_placement_property())
    }

    pub fn set_pane_placement(&self, value: SplitViewPanePlacement) {
        self.set_value(Self::pane_placement_property(), value)
    }

    /// The pane of the split view.
    pub fn pane(&self) -> Option<BoxedValue> {
        self.get_value(Self::pane_property())
    }

    pub fn set_pane(&self, value: Option<BoxedValue>) {
        self.set_value(Self::pane_property(), value)
    }

    /// The data template used to display the pane content of the control.
    pub fn pane_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::pane_template_property())
    }

    pub fn set_pane_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::pane_template_property(), value)
    }

    /// Whether the light dismiss overlay mode is enabled.
    ///
    /// When enabled, and the pane is open in the overlay or the compact
    /// overlay mode, the contents of the split view are darkened to visually
    /// separate the open pane and the rest of the split view.
    pub fn use_light_dismiss_overlay_mode(&self) -> bool {
        self.get_value(Self::use_light_dismiss_overlay_mode_property())
    }

    pub fn set_use_light_dismiss_overlay_mode(&self, value: bool) {
        self.set_value(Self::use_light_dismiss_overlay_mode_property(), value)
    }

    /// The template settings of the split view.
    pub fn template_settings(&self) -> Ref<SplitViewTemplateSettings> {
        self.template_settings.borrow().clone()
    }

    /// Fired when the pane is closed.
    pub fn pane_closed(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::pane_closed_event(), handler)
    }

    /// Fired when the pane is closing.
    ///
    /// The `cancel` property of the event args may be set to true to cancel
    /// the event and keep the pane open.
    pub fn pane_closing(
        &self,
        handler: impl Fn(&Interactive, &CancelRoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::pane_closing_event(), handler)
    }

    /// Fired when the pane is opened.
    pub fn pane_opened(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::pane_opened_event(), handler)
    }

    /// Fired when the pane is opening.
    ///
    /// The `cancel` property of the event args may be set to true to cancel
    /// the event and keep the pane closed.
    pub fn pane_opening(
        &self,
        handler: impl Fn(&Interactive, &CancelRoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::pane_opening_event(), handler)
    }

    fn dispose_pointer_disposable(&self) {
        let disposable = self.pointer_disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }

    fn pointer_released_outside(&self, e: &PointerReleasedEventArgs) {
        let Some(pane) = self.pane.borrow().clone().map(Ref::upcast::<Visual>) else {
            return;
        };
        if !self.is_pane_open() {
            return;
        }

        let mut close_pane = true;
        let mut src = e.source().and_then(|source| source.cast::<Visual>());
        while let Some(current) = src {
            // Make assumption that if a popup is in the visual tree, the
            // owning control is within the pane. This works because if the
            // pane is triggered to close when clicked anywhere else in the
            // window, the pane would close before the popup is opened.
            if current.ptr_eq(&pane) || current.is::<PopupRoot>() {
                close_pane = false;
                break;
            }

            src = current.visual_parent();
        }

        if close_pane {
            self.set_current_value(Self::is_pane_open_property(), false);
            e.set_handled(true);
        }
    }

    fn is_in_overlay_mode(&self) -> bool {
        let display_mode = self.display_mode();
        display_mode == SplitViewDisplayMode::CompactOverlay || display_mode == SplitViewDisplayMode::Overlay
    }

    /// The pseudoclass for the given display mode.
    fn display_mode_pseudo_class(mode: SplitViewDisplayMode) -> &'static str {
        match mode {
            SplitViewDisplayMode::Inline => PC_INLINE,
            SplitViewDisplayMode::CompactInline => PC_COMPACT_INLINE,
            SplitViewDisplayMode::Overlay => PC_OVERLAY,
            SplitViewDisplayMode::CompactOverlay => PC_COMPACT_OVERLAY,
        }
    }

    /// The pseudoclass for the given pane placement.
    fn placement_pseudo_class(placement: SplitViewPanePlacement) -> &'static str {
        match placement {
            SplitViewPanePlacement::Left => PC_LEFT,
            SplitViewPanePlacement::Right => PC_RIGHT,
            SplitViewPanePlacement::Top => PC_TOP,
            SplitViewPanePlacement::Bottom => PC_BOTTOM,
        }
    }

    fn update_visual_state_for_compact_pane_length(&self, new_len: f64) {
        let display_mode = self.display_mode();
        let template_settings = self.template_settings();
        if display_mode == SplitViewDisplayMode::CompactInline {
            template_settings.set_closed_pane_width(new_len);
            template_settings.set_closed_pane_height(new_len);
        } else if display_mode == SplitViewDisplayMode::CompactOverlay {
            template_settings.set_closed_pane_width(new_len);
            template_settings.set_pane_column_grid_length(GridLength::new(new_len, GridUnitType::Pixel));
            template_settings.set_closed_pane_height(new_len);
            template_settings.set_pane_row_grid_length(GridLength::new(new_len, GridUnitType::Pixel));
        }
    }

    fn update_visual_state_for_display_mode(&self, new_value: SplitViewDisplayMode) {
        if let Some(last) = self.last_display_mode_pseudoclass.get() {
            self.pseudo_classes().remove_pseudo(last);
        }

        let pseudo_class = Self::display_mode_pseudo_class(new_value);
        self.last_display_mode_pseudoclass.set(Some(pseudo_class));
        self.pseudo_classes().add_pseudo(pseudo_class);

        let template_settings = self.template_settings();

        let (closed_pane_width, pane_column_grid_length) = match new_value {
            SplitViewDisplayMode::Overlay => (0.0, GridLength::new(0.0, GridUnitType::Pixel)),
            SplitViewDisplayMode::CompactOverlay => {
                (self.compact_pane_length(), GridLength::new(self.compact_pane_length(), GridUnitType::Pixel))
            }
            SplitViewDisplayMode::Inline => (0.0, GridLength::new(0.0, GridUnitType::Auto)),
            SplitViewDisplayMode::CompactInline => {
                (self.compact_pane_length(), GridLength::new(0.0, GridUnitType::Auto))
            }
        };
        template_settings.set_closed_pane_width(closed_pane_width);
        template_settings.set_pane_column_grid_length(pane_column_grid_length);

        let (closed_pane_height, pane_row_grid_length) = match new_value {
            SplitViewDisplayMode::Overlay => (0.0, GridLength::new(0.0, GridUnitType::Pixel)),
            SplitViewDisplayMode::CompactOverlay => {
                (self.compact_pane_length(), GridLength::new(self.compact_pane_length(), GridUnitType::Pixel))
            }
            SplitViewDisplayMode::Inline => (0.0, GridLength::new(0.0, GridUnitType::Auto)),
            SplitViewDisplayMode::CompactInline => {
                (self.compact_pane_length(), GridLength::new(0.0, GridUnitType::Auto))
            }
        };
        template_settings.set_closed_pane_height(closed_pane_height);
        template_settings.set_pane_row_grid_length(pane_row_grid_length);

        self.invalidate_light_dismiss_subscription();
    }

    fn update_visual_state_for_pane_placement_property(&self, new_value: SplitViewPanePlacement) {
        if let Some(last) = self.last_placement_pseudoclass.get() {
            self.pseudo_classes().remove_pseudo(last);
        }

        let pseudo_class = Self::placement_pseudo_class(new_value);
        self.last_placement_pseudoclass.set(Some(pseudo_class));
        self.pseudo_classes().add_pseudo(pseudo_class);
    }

    fn invalidate_light_dismiss_subscription(&self) {
        if self.pane.borrow().is_none() {
            return;
        }

        // If this returns false, we're not in the overlay or the compact
        // overlay display mode and don't need the light dismiss behavior.
        if !self.is_in_overlay_mode() {
            self.dispose_pointer_disposable();
            return;
        }

        if self.pointer_disposable.borrow().is_some() {
            return;
        }

        if let Some(top_level) = TopLevel::get_top_level(Some(self)) {
            // The handlers refer to the split view weakly: the top level
            // must not keep the split view alive through them.
            let weak = self.to_ref().downgrade();
            let pointer_released = top_level.add_handler(InputElement::pointer_released_event(), {
                let weak = weak.clone();
                move |_, e: &PointerReleasedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.pointer_released_outside(e);
                    }
                }
            });
            let back_requested = top_level.back_requested(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.top_level_back_requested(e);
                }
            });

            let top_level = top_level.downgrade();
            let disposable = Disposable::create(move || {
                if let Some(top_level) = top_level.upgrade() {
                    top_level.remove_handler(InputElement::pointer_released_event(), pointer_released);
                    top_level.remove_handler(TopLevel::back_requested_event(), back_requested);
                }
            });
            *self.pointer_disposable.borrow_mut() = Some(disposable);
        }
    }

    fn top_level_back_requested(&self, e: &RoutedEventArgs) {
        if !self.is_in_overlay_mode() {
            return;
        }

        if !self.is_pane_open() {
            return;
        }

        self.set_current_value(Self::is_pane_open_property(), false);
        e.set_handled(true);
    }

    fn update_pane_state_pseudo_class(&self, is_pane_open: bool) {
        if is_pane_open {
            self.pseudo_classes().add_pseudo(PC_OPEN);
            self.pseudo_classes().remove_pseudo(PC_CLOSED);
        } else {
            self.pseudo_classes().add_pseudo(PC_CLOSED);
            self.pseudo_classes().remove_pseudo(PC_OPEN);
        }
    }

    /// Coerces/validates the `IsPaneOpen` property value.
    fn coerce_is_pane_open(instance: &FerroObject, value: bool) -> bool {
        if let Some(split_view) = instance.downcast_ref::<SplitView>() {
            return split_view.on_coerce_is_pane_open(value);
        }

        value
    }
}
