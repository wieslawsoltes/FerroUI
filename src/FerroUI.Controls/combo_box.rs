use crate::assigned_binding::AssignedBinding;
use crate::generators::RecycleKey;
use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::primitives::{
    ItemSelectionEventTriggers, Popup, SelectingItemsControl, SelectingItemsControlImpl, SelectingItemsControlImplExt,
    TemplateAppliedEventArgs, TemplatedControlImpl, TextSearch,
};
use crate::shapes::Rectangle;
use crate::templates::{FuncTemplate, IDataTemplate, ITemplateOf};
use crate::utils::BindingEvaluator;
use crate::{
    ComboBoxItem, ContentControl, Control, ControlImpl, ItemsControl, ItemsControlImpl, Panel, TextBlock, TextBox,
    VirtualizingStackPanel,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingMode;
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation,
    NavigationMethod, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerUpdateKind,
    PointerWheelEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{AlignmentX, FlowDirection, IBrush, Stretch, VisualBrush};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable, ObservableExt};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StaticType, StyledElement, StyledElementImpl, StyledProperty, StyledPropertyMetadata, StyledPropertyOptions, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Resets a flag when dropped.
struct Reset<'a>(&'a Cell<bool>);

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A drop-down list control.
#[repr(C)]
pub struct ComboBox {
    base: SelectingItemsControl,
    popup: RefCell<Option<Ref<Popup>>>,
    // The subscriptions to the opened and closed events of the popup.
    popup_subscriptions: RefCell<Option<(Rc<dyn IDisposable>, Rc<dyn IDisposable>)>>,
    selection_box_item: RefCell<Option<BoxedValue>>,
    subscriptions_on_open: CompositeDisposable,
    input_text_box: RefCell<Option<Ref<TextBox>>>,
    text_value_binding_evaluator: RefCell<Option<Ref<BindingEvaluator>>>,
    skip_next_text_changed: Cell<bool>,
    drop_down_closed: HandlerList<dyn Fn()>,
    drop_down_opened: HandlerList<dyn Fn()>,
}

ferro_class!(ComboBox: SelectingItemsControl);
ferroui_base::ferro_class_info!(ComboBox { new: ComboBox::new });
ferro_impl_classes!(ComboBox: StyledElementImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for ComboBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ComboBoxAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for ComboBox {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();

        if property == SelectingItemsControl::selected_item_property().as_property() {
            let new_value = change.get_new_value::<Option<BoxedValue>>();
            this.update_selection_box_item(new_value.clone());
            this.try_focus_selected_item();
            this.update_input_text_from_selection(&new_value);
        } else if property == Self::is_drop_down_open_property().as_property() {
            this.pseudo_classes().set(Self::PC_DROPDOWN_OPEN, change.get_new_value::<bool>());
        } else if property == ItemsControl::item_template_property().as_property() {
            this.coerce_value(Self::selection_box_item_template_property().as_property());
        } else if property == Self::selection_box_item_template_property().as_property() {
            this.update_selection_box_item(this.selected_item());
        } else if property == Self::is_editable_property().as_property() && change.get_new_value::<bool>() {
            this.update_input_text_from_selection(&this.selected_item());
        } else if property == Self::text_property().as_property() {
            this.text_changed(change.get_new_value::<Option<String>>());
        } else if property == ItemsControl::items_source_property().as_property() {
            // The base handler deselects the current item (and resets the
            // text) so we want to run the base first, then try to match by
            // text.
            let text = this.text();
            Self::parent_on_property_changed(this, change);
            this.set_current_value(Self::text_property(), text);
            return;
        } else if property == ItemsControl::display_member_binding_property().as_property() {
            this.handle_text_value_binding_value_changed(None, Some(change));
            // The base handler invalidates the cached template: run it
            // before coercing.
            Self::parent_on_property_changed(this, change);
            this.coerce_value(Self::selection_box_item_template_property().as_property());
            return;
        } else if property == TextSearch::text_binding_property().as_property() {
            this.handle_text_value_binding_value_changed(Some(change), None);
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl VisualImpl for ComboBox {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.update_selection_box_item(this.selected_item());
    }

    fn invalidate_mirror_transform(this: &Self) {
        Self::parent_invalidate_mirror_transform(this);
        this.update_flow_direction();
    }
}

impl InputElementImpl for ComboBox {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.handled() {
            return;
        }

        let alt = e.key_modifiers.contains(KeyModifiers::ALT);

        if (e.key == Key::F4 && !alt) || ((e.key == Key::Down || e.key == Key::Up) && alt) {
            this.set_current_value(Self::is_drop_down_open_property(), !this.is_drop_down_open());
            e.set_handled(true);
        } else if this.is_drop_down_open() && e.key == Key::Escape {
            this.set_current_value(Self::is_drop_down_open_property(), false);
            e.set_handled(true);
        } else if !this.is_drop_down_open() && !this.is_editable() && (e.key == Key::Enter || e.key == Key::Space) {
            this.set_current_value(Self::is_drop_down_open_property(), true);
            e.set_handled(true);
        } else if this.is_drop_down_open() && e.key == Key::Tab {
            this.set_current_value(Self::is_drop_down_open_property(), false);
        }
        // Ignore key buttons, if they are used for XY focus.
        else if !this.is_drop_down_open() && !XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type)) {
            if e.key == Key::Down {
                e.set_handled(this.select_next());
            } else if e.key == Key::Up {
                e.set_handled(this.select_previous());
            }
        }
        // This part of code is needed just to acquire initial focus,
        // subsequent focus navigation will be done by the items control.
        else if this.is_drop_down_open()
            && this.selected_index() < 0
            && this.item_count() > 0
            && (e.key == Key::Up || e.key == Key::Down)
            && this.is_focused()
        {
            let first_child = this
                .presenter()
                .and_then(|presenter| presenter.panel())
                .and_then(|panel| panel.children().snapshot().iter().find(|c| Self::can_focus(c)).cloned());
            if let Some(first_child) = first_child {
                e.set_handled(first_child.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
            }
        }
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);

        if !e.handled() {
            if !this.is_drop_down_open() {
                if this.is_focused() {
                    e.set_handled(if e.delta().y < 0.0 { this.select_next() } else { this.select_previous() });
                }
            } else {
                e.set_handled(true);
            }
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !e.handled() {
            if let Some(source) = e.source().and_then(|source| source.cast::<Visual>()) {
                if this.is_inside_popup(&source) {
                    e.set_handled(true);
                    return;
                }
            }
        }

        if this.is_drop_down_open() {
            // When a drop-down is open with the pass through of the overlay
            // dismiss event enabled and the control is pressed, close the
            // drop-down.
            this.set_current_value(Self::is_drop_down_open_property(), false);
            e.set_handled(true);
        } else {
            this.pseudo_classes().set(Self::PC_PRESSED, true);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        // If the user clicked in the input text we don't want to open the
        // dropdown.
        let input_text_box = this.input_text_box.borrow().clone();
        if let Some(input_text_box) = input_text_box {
            if !e.handled() {
                let styled_source = e.source().and_then(|source| source.cast::<StyledElement>());
                if let Some(styled_source) = styled_source {
                    if styled_source.templated_parent().is_some_and(|parent| parent.ptr_eq(&input_text_box)) {
                        return;
                    }
                }
            }
        }

        if !e.handled() {
            if let Some(source) = e.source().and_then(|source| source.cast::<Visual>()) {
                if !this.is_inside_popup(&source) && this.pseudo_classes().contains(Self::PC_PRESSED) {
                    this.set_current_value(Self::is_drop_down_open_property(), !this.is_drop_down_open());
                    crate::platform::PlatformFeedbackExtensions::perform_feedback(
                        &**this as &InputElement,
                        crate::platform::FeedbackAction::click(),
                    );
                    e.set_handled(true);
                }
            }
        }

        this.pseudo_classes().set(Self::PC_PRESSED, false);
        Self::parent_on_pointer_released(this, e);
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        if this.is_editable() {
            let input_text_box = this.input_text_box.borrow().clone();
            if let Some(input_text_box) = input_text_box {
                input_text_box.focus();
                input_text_box.select_all();
            }
        }

        Self::parent_on_got_focus(this, e);
    }
}

impl TemplatedControlImpl for ComboBox {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        if let Some((opened, closed)) = this.popup_subscriptions.take() {
            opened.dispose();
            closed.dispose();
        }

        let popup = e.name_scope().get_as::<Popup>("PART_Popup");

        let weak = this.to_ref().downgrade();
        let opened = popup.opened(move || {
            if let Some(this) = weak.upgrade() {
                this.popup_opened();
            }
        });
        let weak = this.to_ref().downgrade();
        let closed = popup.closed(move || {
            if let Some(this) = weak.upgrade() {
                this.popup_closed();
            }
        });

        *this.popup_subscriptions.borrow_mut() = Some((opened, closed));
        *this.popup.borrow_mut() = Some(popup);

        let input_text_box = e.name_scope().find_as::<TextBox>("PART_EditableTextBox");
        *this.input_text_box.borrow_mut() = input_text_box;
    }
}

impl ItemsControlImpl for ComboBox {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        ComboBoxItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<ComboBoxItem>(item)
    }
}

impl SelectingItemsControlImpl for ComboBox {
    fn update_selection_from_event(this: &Self, container: &Ref<Control>, event_args: &dyn IRoutedEventArgs) -> bool {
        if Self::parent_update_selection_from_event(this, container, event_args) {
            let popup = this.popup.borrow().clone();
            if let Some(popup) = popup {
                popup.close();
            }
            return true;
        }

        false
    }

    fn should_trigger_selection(_this: &Self, selectable: &Visual, event_args: &PointerEventArgs) -> bool {
        ItemSelectionEventTriggers::is_pointer_event_within_bounds(selectable, event_args)
            && matches!(
                event_args.properties().pointer_update_kind,
                PointerUpdateKind::LeftButtonReleased | PointerUpdateKind::RightButtonReleased
            )
            && event_args.routed_event().is_some_and(|event| event == *InputElement::pointer_released_event())
    }
}

impl ComboBox {
    pub(crate) const PC_DROPDOWN_OPEN: &'static str = ":dropdownopen";
    pub(crate) const PC_PRESSED: &'static str = ":pressed";

    /// The named parts expected in the control template.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::required("PART_Popup", <Popup as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_EditableTextBox", <TextBox as StaticType>::TYPE),
    ];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[Self::PC_DROPDOWN_OPEN, Self::PC_PRESSED]);
}

ferroui_base::ferro_properties! { impl ComboBox {
    ferro_property!(
        /// Defines the `IsDropDownOpen` property.
        pub fn is_drop_down_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<ComboBox, _>("IsDropDownOpen", false)
        }
    );

    ferro_property!(
        /// Defines the `IsEditable` property.
        pub fn is_editable_property() -> StyledProperty<bool> {
            FerroProperty::register::<ComboBox, _>("IsEditable", false)
        }
    );

    ferro_property!(
        /// Defines the `MaxDropDownHeight` property.
        pub fn max_drop_down_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<ComboBox, _>("MaxDropDownHeight", 200.0)
        }
    );

    ferro_property!(
        /// Defines the `SelectionBoxItem` property.
        pub fn selection_box_item_property() -> DirectProperty<ComboBox, Option<BoxedValue>> {
            FerroProperty::register_direct::<ComboBox, _>("SelectionBoxItem", |o| o.selection_box_item(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `PlaceholderText` property.
        pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
            TextBox::placeholder_text_property().add_owner::<ComboBox>()
        }
    );

    ferro_property!(
        /// Defines the `PlaceholderForeground` property.
        pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextBox::placeholder_foreground_property().add_owner::<ComboBox>()
        }
    );

    ferro_property!(
        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<ComboBox>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<ComboBox>()
        }
    );

    ferro_property!(
        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            TextBlock::text_property().add_owner_with::<ComboBox>(
                StyledPropertyMetadata::new(Some(Some(String::new())))
                    .with_default_binding_mode(BindingMode::TwoWay)
                    .with_enable_data_validation(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectionBoxItemTemplate` property.
        pub fn selection_box_item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register_with::<ComboBox, _>(
                "SelectionBoxItemTemplate",
                StyledPropertyOptions::new(None)
                    .default_binding_mode(BindingMode::TwoWay)
                    .coerce(Self::coerce_selection_box_item_template),
            )
        }
    );
} }

impl ComboBox {
    fn coerce_selection_box_item_template(
        obj: &FerroObject,
        template: Option<Rc<dyn IDataTemplate>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        if template.is_some() {
            return template;
        }
        if let Some(combo_box) = obj.downcast_ref::<ComboBox>() {
            return combo_box.get_effective_item_template();
        }
        template
    }

    fn static_constructor() {
        // The default value for the `ItemsPanel` property.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>()));
        ItemsControl::items_panel_property().override_default_value::<ComboBox>(default_panel);
        InputElement::focusable_property().override_default_value::<ComboBox>(true);
        SelectingItemsControl::is_text_search_enabled_property().override_default_value::<ComboBox>(true);
        crate::platform::PlatformFeedback::feedback_type_property()
            .override_default_value::<ComboBox>(crate::platform::FeedbackType::Auto);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: SelectingItemsControl::construct(),
            popup: RefCell::new(None),
            popup_subscriptions: RefCell::new(None),
            selection_box_item: RefCell::new(None),
            input_text_box: RefCell::new(None),
            subscriptions_on_open: CompositeDisposable::new(),
            text_value_binding_evaluator: RefCell::new(None),
            skip_next_text_changed: Cell::new(false),
            drop_down_closed: HandlerList::new(),
            drop_down_opened: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs after the drop-down (popup) list of the [`ComboBox`] closes.
    pub fn drop_down_closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.drop_down_closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.drop_down_closed.remove(token);
            }
        })
    }

    /// Occurs after the drop-down (popup) list of the [`ComboBox`] opens.
    pub fn drop_down_opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.drop_down_opened.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.drop_down_opened.remove(token);
            }
        })
    }

    /// Gets or sets a value indicating whether the dropdown is currently
    /// open.
    pub fn is_drop_down_open(&self) -> bool {
        self.get_value(Self::is_drop_down_open_property())
    }

    pub fn set_is_drop_down_open(&self, value: bool) {
        self.set_value(Self::is_drop_down_open_property(), value)
    }

    /// Gets or sets a value indicating whether the control is editable.
    pub fn is_editable(&self) -> bool {
        self.get_value(Self::is_editable_property())
    }

    pub fn set_is_editable(&self, value: bool) {
        self.set_value(Self::is_editable_property(), value)
    }

    /// Gets or sets the maximum height for the dropdown list.
    pub fn max_drop_down_height(&self) -> f64 {
        self.get_value(Self::max_drop_down_height_property())
    }

    pub fn set_max_drop_down_height(&self, value: f64) {
        self.set_value(Self::max_drop_down_height_property(), value)
    }

    /// Gets the item to display as the control's content.
    pub fn selection_box_item(&self) -> Option<BoxedValue> {
        self.selection_box_item.borrow().clone()
    }

    /// Sets the item to display as the control's content.
    pub fn set_selection_box_item(&self, value: Option<BoxedValue>) {
        self.set_and_raise(Self::selection_box_item_property(), &self.selection_box_item, value);
    }

    /// Gets or sets the placeholder text.
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }

    pub fn set_placeholder_text(&self, value: Option<String>) {
        self.set_value(Self::placeholder_text_property(), value)
    }

    /// Gets or sets the brush that renders the placeholder text.
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }

    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value)
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

    /// Gets or sets the data template used to display the selected item.
    /// This has a higher priority than the item template if set.
    pub fn selection_box_item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::selection_box_item_template_property())
    }

    pub fn set_selection_box_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::selection_box_item_template_property(), value)
    }

    /// Gets or sets the text used when the control is editable. Does
    /// nothing if it is not editable.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value)
    }

    pub(crate) fn item_focused(&self, drop_down_item: &ComboBoxItem) {
        if self.is_drop_down_open() && drop_down_item.is_focused() && drop_down_item.is_arrange_valid() {
            drop_down_item.bring_into_view();
        }
    }

    /// Whether the popup of the template is open and contains `visual`.
    fn is_inside_popup(&self, visual: &Visual) -> bool {
        let popup = self.popup.borrow().clone();
        popup.is_some_and(|popup| popup.is_inside_popup(visual))
    }

    fn popup_closed(&self) {
        self.subscriptions_on_open.clear();

        if self.is_editable() && Self::can_focus(self) {
            self.focus();
        }

        for (_, handler) in self.drop_down_closed.snapshot().iter() {
            handler();
        }
    }

    fn popup_opened(&self) {
        self.try_focus_selected_item();

        self.subscriptions_on_open.clear();

        let weak = self.to_ref().downgrade();
        let is_visible_changed = Rc::new(move |is_visible: bool| {
            if let Some(this) = weak.upgrade() {
                this.is_visible_changed(is_visible);
            }
        });

        let subscribe = |object: &FerroObject| {
            let handler = is_visible_changed.clone();
            let subscription = FerroObjectExtensions::get_observable(object, Visual::is_visible_property())
                .subscribe_fn(move |is_visible| handler(is_visible));
            self.subscriptions_on_open.add(subscription);
        };

        subscribe(self);

        for parent in self.get_visual_ancestors().filter_map(|ancestor| ancestor.cast::<Control>()) {
            subscribe(&parent);
        }

        self.update_flow_direction();

        for (_, handler) in self.drop_down_opened.snapshot().iter() {
            handler();
        }
    }

    fn is_visible_changed(&self, is_visible: bool) {
        if !is_visible && self.is_drop_down_open() {
            self.set_current_value(Self::is_drop_down_open_property(), false);
        }
    }

    fn try_focus_selected_item(&self) {
        let selected_index = self.selected_index();
        if self.is_drop_down_open() && selected_index != -1 {
            self.scroll_into_view(selected_index);
            let container = self.container_from_index(selected_index);

            if let Some(container) = container {
                if Self::can_focus(&container) {
                    container.focus();
                }
            }
        }
    }

    fn can_focus(control: &Control) -> bool {
        control.focusable() && control.is_effectively_enabled() && control.is_visible()
    }

    fn update_selection_box_item(&self, item: Option<BoxedValue>) {
        let mut item = item;

        let content_control =
            item.as_ref().and_then(Control::from_boxed).and_then(|control| control.cast::<ContentControl>());
        if let Some(content_control) = content_control {
            item = content_control.content();
        }

        let Some(item) = item else {
            self.set_selection_box_item(None);
            return;
        };

        if self.selection_box_item_template().is_some() {
            self.set_selection_box_item(Some(item));
        } else if let Some(control) = Control::from_boxed(&item) {
            if self.visual_root().is_some() {
                control.measure(Size::INFINITY);

                let fill = VisualBrush::new();
                fill.set_visual(&control);
                fill.set_stretch(Stretch::None);
                fill.set_alignment_x(AlignmentX::Left);

                let rectangle = Rectangle::new();
                rectangle.set_width(control.desired_size().width);
                rectangle.set_height(control.desired_size().height);
                rectangle.set_fill(Some(fill.into()));

                self.set_selection_box_item(Some(Control::boxed(rectangle)));
            }

            self.update_flow_direction();
        } else {
            self.set_selection_box_item(Some(item));
        }
    }

    fn update_flow_direction(&self) {
        let rectangle = self
            .selection_box_item()
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|control| control.cast::<Rectangle>());
        if let Some(rectangle) = rectangle {
            let content = rectangle
                .fill()
                .and_then(|fill| fill.as_object().and_then(|object| object.downcast_ref::<VisualBrush>()?.visual()));
            if let Some(content) = content {
                let flow_direction =
                    content.visual_parent().map_or(FlowDirection::LeftToRight, |parent| parent.flow_direction());
                rectangle.set_flow_direction(flow_direction);
            }
        }
    }

    fn update_input_text_from_selection(&self, item: &Option<BoxedValue>) {
        // If we are modifying the text box which has deselected a value we
        // don't want to update the text box value.
        if self.skip_next_text_changed.get() {
            return;
        }
        self.set_current_value(Self::text_property(), Some(self.get_item_text_value(item)));
    }

    fn select_next(&self) -> bool {
        self.move_selection_by(self.selected_index(), 1, self.wrap_selection())
    }

    fn select_previous(&self) -> bool {
        self.move_selection_by(self.selected_index(), -1, self.wrap_selection())
    }

    fn move_selection_by(&self, start_index: i32, step: i32, wrap: bool) -> bool {
        fn is_selectable(o: Option<&FerroObject>) -> bool {
            o.is_none_or(|o| o.get_value(InputElement::is_enabled_property()))
        }

        let count = self.item_count();

        let mut i = start_index + step;
        while i != start_index {
            if i < 0 || i >= count {
                if wrap {
                    if i < 0 {
                        i += count;
                    } else if i >= count {
                        i %= count;
                    }
                } else {
                    return false;
                }
            }

            let item = self.items_view().get_at(i as usize);
            let container = self.container_from_index(i);

            let item_object = item.as_ref().and_then(|value| ValueTypes::as_object(&**value));
            if is_selectable(item_object.as_deref())
                && is_selectable(container.as_deref().map(|control| -> &FerroObject { control }))
            {
                self.set_selected_index(i);
                return true;
            }

            i += step;
        }

        false
    }

    /// Clears the selection.
    pub fn clear(&self) {
        self.set_selected_item(None);
        self.set_selected_index(-1);
    }

    fn handle_text_value_binding_value_changed(
        &self,
        text_search_prop_change: Option<&FerroPropertyChangedEventArgs<'_>>,
        display_member_prop_change: Option<&FerroPropertyChangedEventArgs<'_>>,
    ) {
        let new_binding = |change: Option<&FerroPropertyChangedEventArgs<'_>>| {
            change.and_then(|change| change.get_new_value::<Option<AssignedBinding>>())
        };

        // Prioritise using the text binding of the text search if possible.
        let text_search_binding =
            if text_search_prop_change.is_none() { TextSearch::get_text_binding(self) } else { None };
        let text_value_binding = text_search_binding
            .or_else(|| new_binding(text_search_prop_change))
            .or_else(|| new_binding(display_member_prop_change));

        let evaluator = self.text_value_binding_evaluator.borrow().clone();
        let evaluator = match (evaluator, &text_value_binding) {
            (None, binding) => BindingEvaluator::try_create(binding.as_ref()),
            (Some(_), None) => None,
            (Some(evaluator), Some(binding)) => {
                evaluator.update_binding(binding);
                Some(evaluator)
            }
        };
        *self.text_value_binding_evaluator.borrow_mut() = evaluator.clone();

        // If the binding is set we want to set the initial value for the
        // selected item so the text box has the correct value.
        if let Some(evaluator) = evaluator {
            let text: BoxedValue = Rc::new(self.get_item_text_value(&self.selected_value()));
            evaluator.set_value(BindingEvaluator::value_property(), Some(text));
        }
    }

    fn text_changed(&self, new_value: Option<String>) {
        if !self.is_editable() || self.skip_next_text_changed.get() {
            return;
        }

        let mut selected_idx = -1;
        let mut selected_item = None;
        let count = self.items().count();
        for i in 0..count {
            let item = self.items().get_at(i);
            let item_text = self.get_item_text_value(&item);
            if new_value.as_deref().is_some_and(|new_value| equals_ignore_case(new_value, &item_text)) {
                selected_idx = i as i32;
                selected_item = item;
                break;
            }
        }

        {
            self.skip_next_text_changed.set(true);
            let _reset = Reset(&self.skip_next_text_changed);
            self.set_selected_index(selected_idx);
            self.set_selected_item(selected_item);
        }

        // When changing the selected index it will set the tab once active
        // element of this control to a combo box item; this will then break
        // tab navigation back into this combo box as it will try to focus
        // the combo box item rather than the combo box or editable text
        // box, so we need to set the tab once active element to null.
        KeyboardNavigation::set_tab_once_active_element(self, None);
    }

    fn get_item_text_value(&self, item: &Option<BoxedValue>) -> String {
        let evaluator = self.text_value_binding_evaluator.borrow().clone();
        TextSearch::get_effective_text(item, evaluator.as_ref())
    }
}

/// Whether two texts are equal, comparing without regard to case.
fn equals_ignore_case(a: &str, b: &str) -> bool {
    a.chars().flat_map(char::to_uppercase).eq(b.chars().flat_map(char::to_uppercase))
}
