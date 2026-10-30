//! Live toolbar orchestration over FerroUI controls and the portable ink presenter.
use super::ink_toolbar_buttons::strings;
use super::*;
use ferroui_controls::primitives::{PopupFlyoutBase, ToggleButton};
use ferroui_controls::utils::BorderRenderHelper;
use ferroui_controls::{
    Border, Button, Control, ControlImpl, Controls, Decorator, PanelImpl, PanelImplExt,
    PlacementMode, StackPanel, StackPanelImpl,
};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyboardNavigation,
    KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{
    LayoutHelper, Layoutable, LayoutableImpl, LayoutableImplExt, Orientation,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::{BackgroundSizing, BoxShadows, DrawingContext, IBrush};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{EventArgs, HandlerList};
use ferroui_base::BoxedValue;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, CornerRadius,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty, Thickness,
    Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ToolbarHandlers = Rc<HandlerList<dyn Fn(&InkToolbar)>>;

#[derive(Clone, PartialEq)]
pub struct InkToolbarIsStencilButtonCheckedChangedEventArgs {
    pub stencil_button: Ref<InkToolbarStencilButton>,
    pub stencil_kind: InkToolbarStencilKind,
}

/// A functional inking toolbar using FerroUI's panel layout, control templates and input routing.
#[repr(C)]
pub struct InkToolbar {
    base: StackPanel,
    children: Controls,
    defaults: RefCell<Vec<Ref<Control>>>,
    subscriptions: RefCell<Vec<(WeakRef<Control>, Rc<dyn IDisposable>)>>,
    selection_sync: Cell<bool>,
    recent_tool: RefCell<Option<WeakRef<InkToolbarToolButton>>>,
    current_presenter: RefCell<Option<Rc<InkPresenter>>>,
    presenter_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    drawing_attributes: Cell<Option<InkDrawingAttributes>>,
    active_tool_changed: ToolbarHandlers,
    ink_drawing_attributes_changed: ToolbarHandlers,
    erase_all_clicked: ToolbarHandlers,
    stencil_changed: Rc<HandlerList<dyn Fn(&InkToolbarIsStencilButtonCheckedChangedEventArgs)>>,
    border_renderer: RefCell<BorderRenderHelper>,
}
ferro_class!(InkToolbar: StackPanel);
ferro_class_info!(InkToolbar { new: InkToolbar::new, markup: {
    content: Children,
    methods: [
        fn GetToolButton(InkToolbarTool) -> Option<Ref<InkToolbarToolButton>> => InkToolbar::get_tool_button,
        fn GetToggleButton(InkToolbarToggle) -> Option<Ref<InkToolbarToggleButton>> => InkToolbar::get_toggle_button,
        fn GetMenuButton(InkToolbarMenuKind) -> Option<Ref<InkToolbarMenuButton>> => InkToolbar::get_menu_button,
    ],
    events: [
        ActiveToolChanged(Option<BoxedValue>, EventArgs) => |this: &Ref<InkToolbar>, handler: MarkupDelegate| {
            let sender=this.downgrade();
            this.active_tool_changed(move |_| { handler.invoke(&[into_markup_value(sender.upgrade()),into_markup_value(EventArgs::EMPTY)]); })
        },
        InkDrawingAttributesChanged(Option<BoxedValue>, EventArgs) => |this: &Ref<InkToolbar>, handler: MarkupDelegate| {
            let sender=this.downgrade();
            this.ink_drawing_attributes_changed(move |_| { handler.invoke(&[into_markup_value(sender.upgrade()),into_markup_value(EventArgs::EMPTY)]); })
        },
        EraseAllClicked(Option<BoxedValue>, EventArgs) => |this: &Ref<InkToolbar>, handler: MarkupDelegate| {
            let sender=this.downgrade();
            this.erase_all_clicked(move |_| { handler.invoke(&[into_markup_value(sender.upgrade()),into_markup_value(EventArgs::EMPTY)]); })
        },
        IsStencilButtonCheckedChanged(Option<BoxedValue>, InkToolbarIsStencilButtonCheckedChangedEventArgs) => |this: &Ref<InkToolbar>, handler: MarkupDelegate| {
            let sender=this.downgrade();
            this.is_stencil_button_checked_changed(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()),into_markup_value(args.clone())]); })
        },
    ],
} });
ferro_impl_classes!(InkToolbar:StyledElementImpl,InteractiveImpl);
impl LayoutableImpl for InkToolbar {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if this.active_tool().is_none() {
            let first_pen = this
                .registered_children()
                .snapshot()
                .iter()
                .find_map(|child| child.cast::<InkToolbarPenButton>())
                .map(|pen| pen.upcast::<InkToolbarToolButton>());
            if first_pen.is_some() {
                this.set_active_tool(first_pen);
            }
        }
        let insets = this.layout_insets();
        Self::parent_measure_override(this, available_size.deflate(insets)).inflate(insets)
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        Self::parent_arrange_override(this, final_size.deflate(this.layout_insets()));
        final_size
    }
}
impl StackPanelImpl for InkToolbar {
    fn arrange_child(
        this: &Self,
        child: &Ref<Control>,
        rect: Rect,
        _panel_size: Size,
        _orientation: Orientation,
    ) {
        let insets = this.layout_insets();
        child.arrange(
            rect.with_x(rect.x + insets.left)
                .with_y(rect.y + insets.top),
        );
    }
}
impl VisualImpl for InkToolbar {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.border_renderer.borrow_mut().render(
            context,
            this.bounds().size(),
            round_panel_thickness(this, this.border_thickness()),
            this.corner_radius(),
            this.background_sizing(),
            this.background().as_ref(),
            this.border_brush().as_ref(),
            &BoxShadows::default(),
        );
    }
    fn clip_to_bounds_radius(this: &Self) -> Option<CornerRadius> {
        Some(this.corner_radius())
    }
}
impl InputElementImpl for InkToolbar {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if event.handled() {
            return;
        }
        let backward = match event.key {
            Key::Left | Key::Up => true,
            Key::Right | Key::Down => false,
            _ => return,
        };
        let children = this.registered_children().snapshot();
        if children.len() <= 1 {
            return;
        }
        let Some(current) = children
            .iter()
            .position(|child| child.is_keyboard_focus_within())
        else {
            return;
        };
        for offset in 1..children.len() {
            let index = if backward {
                (current + children.len() - offset) % children.len()
            } else {
                (current + offset) % children.len()
            };
            let child = &children[index];
            if child.is_visible()
                && child.is_effectively_enabled()
                && child.focus_with(NavigationMethod::Directional, event.key_modifiers)
            {
                KeyboardNavigation::set_tab_once_active_element(
                    this,
                    Some(child.clone().upcast::<InputElement>()),
                );
                break;
            }
        }
        event.set_handled(true);
    }
}
impl ControlImpl for InkToolbar {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::InkToolbarAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for InkToolbar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_orientation(Orientation::Horizontal);
        let weak = this.to_ref().downgrade();
        this.children.add_collection_changed(Rc::new(move |event| {
            if let Some(toolbar) = weak.upgrade() {
                toolbar.public_children_changed(event);
            }
        }));
        this.rebuild_default_buttons();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::initial_controls_property().as_property() {
            this.rebuild_default_buttons();
        } else if property == Self::active_tool_property().as_property() {
            this.apply_active_tool(true);
        } else if property == Self::target_ink_canvas_property().as_property()
            || property == Self::target_ink_presenter_property().as_property()
        {
            this.update_target();
        } else if property == Self::is_ruler_button_checked_property().as_property() {
            this.apply_stencil();
        } else if property == Self::is_stencil_button_checked_property().as_property() {
            this.apply_stencil();
            // As WinUI (`OnIsStencilButtonCheckedChanged`), the event is raised for a change of
            // the property alone, not whenever the stencils are applied again.
            this.raise_stencil_changed();
        } else if property == Self::button_flyout_placement_property().as_property()
            || property == StackPanel::orientation_property().as_property()
        {
            this.update_flyout_placement();
        }
    }
}
impl PanelImpl for InkToolbar {
    fn children_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        Self::parent_children_changed(this, e);
        // A visual reorder retains registration, subscriptions and flyouts.
        if e.action == NotifyCollectionChangedAction::Move {
            return;
        }
        let removed: Vec<_> = {
            let mut slots = this.subscriptions.borrow_mut();
            let mut removed = Vec::new();
            slots.retain(|(weak, subscription)| {
                let remove = weak
                    .upgrade()
                    .is_none_or(|child| e.old_items.contains(&child));
                if remove {
                    removed.push(subscription.clone());
                }
                !remove
            });
            removed
        };
        for subscription in removed {
            subscription.dispose();
        }
        for child in e.new_items.iter() {
            this.wire_child(child);
        }
        if let Some(active) = this.active_tool() {
            let active_control: Ref<Control> = active.clone().upcast();
            if e.old_items.contains(&active_control)
                && !this
                    .registered_children()
                    .snapshot()
                    .contains(&active_control)
            {
                this.set_active_tool(None);
            }
        }
    }
}
ferro_properties! {impl InkToolbar{
    pub fn children_property()->DirectProperty<InkToolbar,Controls>{FerroProperty::register_direct::<InkToolbar,_>("Children",InkToolbar::children,None,Controls::new())}
    pub fn initial_controls_property()->StyledProperty<InkToolbarInitialControls>{FerroProperty::register::<InkToolbar,_>("InitialControls",InkToolbarInitialControls::All)}
    pub fn active_tool_property()->StyledProperty<Option<Ref<InkToolbarToolButton>>>{FerroProperty::register::<InkToolbar,_>("ActiveTool",None)}
    pub fn ink_drawing_attributes_property()->DirectProperty<InkToolbar,Option<InkDrawingAttributes>>{FerroProperty::register_direct::<InkToolbar,_>("InkDrawingAttributes",InkToolbar::ink_drawing_attributes,None,None)}
    pub fn is_ruler_button_checked_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbar,_>("IsRulerButtonChecked",false)}
    pub fn target_ink_canvas_property()->StyledProperty<Option<Ref<InkCanvas>>>{FerroProperty::register::<InkToolbar,_>("TargetInkCanvas",None)}
    pub fn is_stencil_button_checked_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbar,_>("IsStencilButtonChecked",false)}
    pub fn button_flyout_placement_property()->StyledProperty<InkToolbarButtonFlyoutPlacement>{FerroProperty::register::<InkToolbar,_>("ButtonFlyoutPlacement",InkToolbarButtonFlyoutPlacement::Auto)}
    pub fn orientation_property()->StyledProperty<Orientation>{StackPanel::orientation_property().add_owner::<InkToolbar>()}
    pub fn target_ink_presenter_property()->StyledProperty<Option<Rc<InkPresenter>>>{FerroProperty::register::<InkToolbar,_>("TargetInkPresenter",None)}
    pub fn border_brush_property()->StyledProperty<Option<Rc<dyn IBrush>>>{Border::border_brush_property().add_owner::<InkToolbar>()}
    pub fn border_thickness_property()->StyledProperty<Thickness>{Border::border_thickness_property().add_owner::<InkToolbar>()}
    pub fn corner_radius_property()->StyledProperty<CornerRadius>{Border::corner_radius_property().add_owner::<InkToolbar>()}
    pub fn background_sizing_property()->StyledProperty<BackgroundSizing>{Border::background_sizing_property().add_owner::<InkToolbar>()}
    pub fn padding_property()->StyledProperty<Thickness>{Decorator::padding_property().add_owner::<InkToolbar>()}
}}
impl InkToolbar {
    fn static_constructor() {
        StackPanel::spacing_property().override_default_value::<InkToolbar>(4.0);
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<InkToolbar>(KeyboardNavigationMode::Once);
        InputElement::is_tab_stop_property().override_default_value::<InkToolbar>(false);
        Visual::affects_render::<InkToolbar>(&[
            Self::border_brush_property().as_property(),
            Self::border_thickness_property().as_property(),
            Self::corner_radius_property().as_property(),
            Self::background_sizing_property().as_property(),
        ]);
        Layoutable::affects_measure::<InkToolbar>(&[
            Self::padding_property().as_property(),
            Self::border_thickness_property().as_property(),
        ]);
    }
    pub fn construct() -> Self {
        Self {
            base: StackPanel::construct(),
            children: Controls::new(),
            defaults: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
            selection_sync: Cell::new(false),
            recent_tool: RefCell::new(None),
            current_presenter: RefCell::new(None),
            presenter_subscription: RefCell::new(None),
            drawing_attributes: Cell::new(None),
            active_tool_changed: Rc::new(HandlerList::new()),
            ink_drawing_attributes_changed: Rc::new(HandlerList::new()),
            erase_all_clicked: Rc::new(HandlerList::new()),
            stencil_changed: Rc::new(HandlerList::new()),
            border_renderer: RefCell::new(BorderRenderHelper::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Public content, including entries ignored by toolbar registration.
    /// Only supported, nonduplicate registrations enter the visual panel.
    pub fn children(&self) -> Controls {
        self.children.clone()
    }
    fn registered_children(&self) -> Controls {
        self.base.children()
    }
    fn public_children_changed(&self, event: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        let registered = self.registered_children();
        if event.action != NotifyCollectionChangedAction::Move {
            for old in event.old_items {
                registered.remove(old);
            }
            for child in event.new_items {
                // A prior removal can notify application code and reenter the
                // public collection. Never register an entry since removed.
                if !self.children.contains(child) {
                    continue;
                }
                // WinUI fills in only the kinds the application has not supplied: a button the
                // application declares replaces the built-in button of its kind.
                let replaced_active = self.displace_default_of_kind(child);
                if !self.children.contains(child) {
                    continue;
                }
                if !self.can_register_child(child) {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL)
                    {
                        logger.log(
                            None,
                            "InkToolbar ignored a duplicate or unsupported child registration.",
                        );
                    }
                    continue;
                }
                let index = self
                    .children
                    .snapshot()
                    .iter()
                    .take_while(|item| *item != child)
                    .filter_map(|item| registered.index_of(item))
                    .last()
                    .map_or(0, |index| index + 1)
                    .min(registered.count());
                registered.insert(index, child);
                if replaced_active {
                    if let Some(tool) = child.cast::<InkToolbarToolButton>() {
                        self.set_active_tool(Some(tool));
                    }
                }
            }
        } else {
            let mut ordered = Vec::new();
            for child in self.children.snapshot().iter() {
                if registered.contains(child) && !ordered.contains(child) {
                    ordered.push(child.clone());
                }
            }
            for (index, child) in ordered.iter().enumerate() {
                if let Some(previous) = registered.index_of(child) {
                    if previous != index && index < registered.count() {
                        registered.move_item(previous, index);
                    }
                }
            }
        }
    }
    /// Removes the built-in button of the same kind as the application's `child`, if there is
    /// one. Returns whether that button was the active tool.
    fn displace_default_of_kind(&self, child: &Ref<Control>) -> bool {
        let Some(kind) = button_kind(child) else {
            return false;
        };
        if self.defaults.borrow().contains(child) {
            return false;
        }
        let displaced = self
            .defaults
            .borrow()
            .iter()
            .find(|default| button_kind(default) == Some(kind))
            .cloned();
        let Some(displaced) = displaced else {
            return false;
        };
        self.defaults.borrow_mut().retain(|default| default != &displaced);
        let was_active = self
            .active_tool()
            .is_some_and(|active| active.upcast::<Control>() == displaced);
        self.children.remove(&displaced);
        was_active
    }
    fn can_register_child(&self, child: &Ref<Control>) -> bool {
        let registered = self.registered_children().snapshot();
        if registered.contains(child) {
            return false;
        }
        if let Some(tool) = child.cast::<InkToolbarToolButton>() {
            return matches!(
                tool.tool_kind(),
                InkToolbarTool::CustomPen | InkToolbarTool::CustomTool
            ) || !registered
                .iter()
                .filter_map(|item| item.cast::<InkToolbarToolButton>())
                .any(|item| item.tool_kind() == tool.tool_kind());
        }
        if let Some(toggle) = child.cast::<InkToolbarToggleButton>() {
            return toggle.toggle_kind() == InkToolbarToggle::Custom
                || !registered
                    .iter()
                    .filter_map(|item| item.cast::<InkToolbarToggleButton>())
                    .any(|item| item.toggle_kind() == toggle.toggle_kind());
        }
        if let Some(menu) = child.cast::<InkToolbarMenuButton>() {
            return !registered
                .iter()
                .filter_map(|item| item.cast::<InkToolbarMenuButton>())
                .any(|item| item.menu_kind() == menu.menu_kind());
        }
        false
    }
    pub fn border_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::border_brush_property())
    }
    pub fn set_border_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::border_brush_property(), value);
    }
    pub fn border_thickness(&self) -> Thickness {
        self.get_value(Self::border_thickness_property())
    }
    pub fn set_border_thickness(&self, value: Thickness) {
        self.set_value(Self::border_thickness_property(), value);
    }
    pub fn corner_radius(&self) -> CornerRadius {
        self.get_value(Self::corner_radius_property())
    }
    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::corner_radius_property(), value);
    }
    pub fn background_sizing(&self) -> BackgroundSizing {
        self.get_value(Self::background_sizing_property())
    }
    pub fn set_background_sizing(&self, value: BackgroundSizing) {
        self.set_value(Self::background_sizing_property(), value);
    }
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }
    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value);
    }
    fn layout_insets(&self) -> Thickness {
        round_panel_thickness(self, self.padding())
            + round_panel_thickness(self, self.border_thickness())
    }
    pub fn initial_controls(&self) -> InkToolbarInitialControls {
        self.get_value(Self::initial_controls_property())
    }
    pub fn set_initial_controls(&self, value: InkToolbarInitialControls) {
        self.set_value(Self::initial_controls_property(), value);
    }
    pub fn active_tool(&self) -> Option<Ref<InkToolbarToolButton>> {
        self.get_value(Self::active_tool_property())
    }
    pub fn set_active_tool(&self, value: Option<Ref<InkToolbarToolButton>>) {
        self.set_value(Self::active_tool_property(), value);
    }
    pub fn ink_drawing_attributes(&self) -> Option<InkDrawingAttributes> {
        self.drawing_attributes.get()
    }
    pub fn is_ruler_button_checked(&self) -> bool {
        self.get_value(Self::is_ruler_button_checked_property())
    }
    pub fn set_is_ruler_button_checked(&self, value: bool) {
        self.set_value(Self::is_ruler_button_checked_property(), value);
    }
    pub fn target_ink_canvas(&self) -> Option<Ref<InkCanvas>> {
        self.get_value(Self::target_ink_canvas_property())
    }
    pub fn set_target_ink_canvas(&self, value: Option<Ref<InkCanvas>>) {
        self.set_value(Self::target_ink_canvas_property(), value);
    }
    pub fn is_stencil_button_checked(&self) -> bool {
        self.get_value(Self::is_stencil_button_checked_property())
    }
    pub fn set_is_stencil_button_checked(&self, value: bool) {
        self.set_value(Self::is_stencil_button_checked_property(), value);
    }
    pub fn button_flyout_placement(&self) -> InkToolbarButtonFlyoutPlacement {
        self.get_value(Self::button_flyout_placement_property())
    }
    pub fn set_button_flyout_placement(&self, value: InkToolbarButtonFlyoutPlacement) {
        self.set_value(Self::button_flyout_placement_property(), value);
    }
    pub fn target_ink_presenter(&self) -> Option<Rc<InkPresenter>> {
        self.get_value(Self::target_ink_presenter_property())
    }
    pub fn set_target_ink_presenter(&self, value: Option<Rc<InkPresenter>>) {
        self.set_value(Self::target_ink_presenter_property(), value);
    }
    pub fn effective_ink_presenter(&self) -> Option<Rc<InkPresenter>> {
        self.target_ink_presenter().or_else(|| {
            self.target_ink_canvas()
                .map(|canvas| canvas.ink_presenter())
        })
    }
    pub fn get_tool_button(&self, tool: InkToolbarTool) -> Option<Ref<InkToolbarToolButton>> {
        self.registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarToolButton>())
            .find(|b| b.tool_kind() == tool)
    }
    pub fn get_toggle_button(&self, kind: InkToolbarToggle) -> Option<Ref<InkToolbarToggleButton>> {
        self.registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarToggleButton>())
            .find(|b| b.toggle_kind() == kind)
    }
    pub fn get_menu_button(&self, kind: InkToolbarMenuKind) -> Option<Ref<InkToolbarMenuButton>> {
        self.registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarMenuButton>())
            .find(|b| b.menu_kind() == kind)
    }
    pub fn active_tool_changed(
        &self,
        handler: impl Fn(&InkToolbar) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.active_tool_changed, handler)
    }
    pub fn ink_drawing_attributes_changed(
        &self,
        handler: impl Fn(&InkToolbar) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.ink_drawing_attributes_changed, handler)
    }
    pub fn erase_all_clicked(
        &self,
        handler: impl Fn(&InkToolbar) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.erase_all_clicked, handler)
    }
    pub fn is_stencil_button_checked_changed(
        &self,
        handler: impl Fn(&InkToolbarIsStencilButtonCheckedChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.stencil_changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.stencil_changed);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.remove(token);
            }
        })
    }
    /// As WinUI: selects the most recent tool that is not the eraser, raises EraseAllClicked
    /// (handlers can capture an undo snapshot), then clears the strokes.
    pub fn erase_all(&self) {
        let recent = self
            .recent_tool
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        let children = self.registered_children().snapshot();
        let restore = recent
            .filter(|tool| children.contains(&tool.clone().upcast::<Control>()))
            .or_else(|| {
                children
                    .iter()
                    .find_map(|child| child.cast::<InkToolbarPenButton>())
                    .map(|pen| pen.upcast())
            });
        if let Some(restore) = restore {
            self.set_active_tool(Some(restore));
        }
        let target = self.effective_ink_presenter();
        for (_, handler) in self.erase_all_clicked.snapshot().iter() {
            handler(self);
        }
        if let Some(target) = target {
            target.stroke_container().clear();
        }
    }
    fn rebuild_default_buttons(&self) {
        let previous = self.defaults.replace(Vec::new());
        for child in previous {
            self.children().remove(child);
        }
        let mut defaults: Vec<Ref<Control>> = Vec::new();
        match self.initial_controls() {
            InkToolbarInitialControls::All | InkToolbarInitialControls::PensOnly => {
                defaults.push(InkToolbarBallpointPenButton::new().upcast());
                defaults.push(InkToolbarPencilButton::new().upcast());
                defaults.push(InkToolbarHighlighterButton::new().upcast());
            }
            _ => {}
        }
        if matches!(
            self.initial_controls(),
            InkToolbarInitialControls::All | InkToolbarInitialControls::AllExceptPens
        ) {
            defaults.push(InkToolbarEraserButton::new().upcast());
            defaults.push(InkToolbarStencilButton::new().upcast());
        }
        // Kinds the application supplied itself are not filled in again.
        let supplied: Vec<ButtonKind> = self
            .children
            .snapshot()
            .iter()
            .filter_map(button_kind)
            .collect();
        defaults.retain(|default| button_kind(default).is_none_or(|kind| !supplied.contains(&kind)));
        *self.defaults.borrow_mut() = defaults.clone();
        for (index, child) in defaults.into_iter().enumerate() {
            self.children().insert(index, child);
        }
        if self.active_tool().is_none() {
            let first = self
                .registered_children()
                .snapshot()
                .iter()
                .filter_map(|c| c.cast::<InkToolbarToolButton>())
                .find(|c| c.tool_kind() != InkToolbarTool::Eraser);
            self.set_active_tool(first);
        }
    }
    fn wire_child(&self, child: &Ref<Control>) {
        if self
            .subscriptions
            .borrow()
            .iter()
            .any(|(weak, _)| weak.upgrade().as_ref() == Some(child))
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        let weak_child = child.downgrade();
        let subscription = child.property_changed(move |change| {
            let (Some(toolbar), Some(child)) = (weak.upgrade(), weak_child.upgrade()) else {
                return;
            };
            if toolbar.selection_sync.get() {
                return;
            }
            if let Some(tool) = child.cast::<InkToolbarToolButton>() {
                if change.property() == ToggleButton::is_checked_property().as_property()
                    && tool.is_checked() == Some(true)
                {
                    toolbar.set_active_tool(Some(tool.clone()));
                }
                if toolbar.active_tool().as_ref() == Some(&tool)
                    && super::ink_toolbar_buttons::affects_pen_attributes(change.property())
                {
                    toolbar.update_drawing_attributes();
                }
            } else if let Some(stencil) = child.cast::<InkToolbarStencilButton>() {
                if change.property() == ToggleButton::is_checked_property().as_property() {
                    toolbar.set_is_stencil_button_checked(stencil.is_checked() == Some(true));
                }
                if change.property()
                    == InkToolbarStencilButton::selected_stencil_property().as_property()
                {
                    toolbar.apply_stencil();
                }
            } else if let Some(toggle) = child.cast::<InkToolbarToggleButton>() {
                if toggle.toggle_kind() == InkToolbarToggle::Ruler
                    && change.property() == ToggleButton::is_checked_property().as_property()
                {
                    toolbar.set_is_ruler_button_checked(toggle.is_checked() == Some(true));
                }
            }
        });
        self.subscriptions
            .borrow_mut()
            .push((child.downgrade(), subscription));
        if let Some(pen) = child.cast::<InkToolbarPenButton>() {
            let flyout = super::ink_toolbar_buttons::toolbar_flyout(child);
            let weak_pen = pen.downgrade();
            let weak_flyout = flyout.downgrade();
            flyout.opening(move |_| {
                let (Some(pen), Some(flyout)) = (weak_pen.upgrade(), weak_flyout.upgrade()) else {
                    return;
                };
                if let Some(custom) = pen.cast::<InkToolbarCustomPenButton>() {
                    flyout.set_content(
                        custom
                            .configuration_content()
                            .map(|content| Rc::new(content) as BoxedValue),
                    );
                } else if flyout.content().is_none() {
                    let content =
                        InkToolbarPenConfigurationControl::for_pen(&pen).upcast::<Control>();
                    flyout.set_content(Some(Rc::new(content)));
                }
            });
            pen.set_flyout(flyout);
        } else if let Some(eraser) = child.cast::<InkToolbarEraserButton>() {
            let button = InkToolbarFlyoutItem::new();
            strings::bind_content(&button, "InkToolbarClearAllLabel");
            button.bind_indexer(
                &ferroui_base::Visual::is_visible_property().bind(),
                &eraser.indexer(&InkToolbarEraserButton::is_clear_all_visible_property().bind()),
            );
            let weak = self.to_ref().downgrade();
            button.add_handler(Button::click_event(), move |_, _| {
                if let Some(toolbar) = weak.upgrade() {
                    toolbar.erase_all();
                }
            });
            let flyout = super::ink_toolbar_buttons::toolbar_flyout(child);
            flyout.set_content(Some(Rc::new(button.upcast::<Control>())));
            eraser.set_flyout(flyout);
        } else if let Some(tool) = child.cast::<InkToolbarCustomToolButton>() {
            let flyout = super::ink_toolbar_buttons::toolbar_flyout(child);
            let weak_tool = tool.downgrade();
            let weak_flyout = flyout.downgrade();
            flyout.opening(move |_| {
                if let (Some(tool), Some(flyout)) = (weak_tool.upgrade(), weak_flyout.upgrade()) {
                    flyout.set_content(
                        tool.configuration_content()
                            .map(|content| Rc::new(content) as ferroui_base::BoxedValue),
                    );
                }
            });
            tool.set_flyout(flyout);
        } else if let Some(stencil) = child.cast::<InkToolbarStencilButton>() {
            stencil.set_presenter(self.effective_ink_presenter());
        }
        self.update_flyout_placement();
    }
    fn apply_active_tool(&self, notify: bool) {
        if self.selection_sync.replace(true) {
            return;
        }
        let active = self.active_tool();
        if notify {
            for button in self
                .registered_children()
                .snapshot()
                .iter()
                .filter_map(|c| c.cast::<Button>())
            {
                if let Some(flyout) = button.flyout() {
                    flyout.hide();
                }
            }
        }
        for tool in self
            .registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarToolButton>())
        {
            tool.set_current_value(
                ToggleButton::is_checked_property(),
                Some(active.as_ref() == Some(&tool)),
            );
        }
        if let Some(tool) = &active {
            if tool.tool_kind() != InkToolbarTool::Eraser {
                *self.recent_tool.borrow_mut() = Some(tool.downgrade());
            }
        }
        self.selection_sync.set(false);
        self.update_drawing_attributes();
        // As WinUI (`ApplyToolStateToInkCanvas`), no active tool leaves the presenter's mode
        // as it is: a toolbar without a tool does not switch inking off.
        if let Some((target, active)) = self.effective_ink_presenter().zip(active.as_ref()) {
            let mode = match Some(active.tool_kind()) {
                Some(InkToolbarTool::Eraser) => InkInputProcessingMode::Erasing,
                Some(
                    InkToolbarTool::BallpointPen
                    | InkToolbarTool::Pencil
                    | InkToolbarTool::Highlighter
                    | InkToolbarTool::CustomPen,
                ) => InkInputProcessingMode::Inking,
                _ => InkInputProcessingMode::None,
            };
            target.input_processing_configuration().set_mode(mode);
        }
        if notify {
            for (_, handler) in self.active_tool_changed.snapshot().iter() {
                handler(self);
            }
        }
    }
    fn update_drawing_attributes(&self) {
        let Some(attrs) = self
            .active_tool()
            .and_then(|tool| tool.cast::<InkToolbarPenButton>())
            .and_then(|pen| pen.create_ink_drawing_attributes().ok())
        else {
            return;
        };
        let attrs = Some(attrs);
        let previous = self.ink_drawing_attributes();
        self.set_and_raise_cell(
            Self::ink_drawing_attributes_property(),
            &self.drawing_attributes,
            attrs,
        );
        if let (Some(attrs), Some(target)) = (attrs, self.effective_ink_presenter()) {
            let _ = target.update_default_drawing_attributes(&attrs);
        }
        if previous != attrs {
            for (_, handler) in self.ink_drawing_attributes_changed.snapshot().iter() {
                handler(self);
            }
        }
    }
    fn update_target(&self) {
        let next = self.effective_ink_presenter();
        let previous = self.current_presenter.replace(next.clone());
        if previous.as_ref().map(Rc::as_ptr) != next.as_ref().map(Rc::as_ptr) {
            let subscription = self.presenter_subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            if let Some(old) = previous {
                old.set_ruler_visible(false);
                old.set_protractor_visible(false);
            }
            if let Some(presenter) = next.as_ref() {
                let weak = self.to_ref().downgrade();
                let source = Rc::downgrade(presenter);
                let mode = Cell::new(presenter.high_contrast_adjustment());
                let subscription = presenter.changed(move || {
                    if let (Some(toolbar), Some(presenter)) = (weak.upgrade(), source.upgrade()) {
                        let next = presenter.high_contrast_adjustment();
                        if mode.replace(next) != next {
                            toolbar.refresh_pen_palettes();
                        }
                    }
                });
                *self.presenter_subscription.borrow_mut() = Some(subscription);
            }
        }
        self.refresh_pen_palettes();
        for stencil in self
            .registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarStencilButton>())
        {
            stencil.set_presenter(next.clone());
        }
        self.apply_active_tool(false);
        self.apply_stencil();
    }
    fn refresh_pen_palettes(&self) {
        for pen in self
            .registered_children()
            .snapshot()
            .iter()
            .filter_map(|child| child.cast::<InkToolbarPenButton>())
        {
            pen.refresh_palette();
        }
    }
    fn apply_stencil(&self) {
        let enabled = self.is_stencil_button_checked();
        let ruler = self.is_ruler_button_checked();
        let stencils: Vec<_> = self
            .registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarStencilButton>())
            .collect();
        let selected = stencils
            .first()
            .map(|b| b.selected_stencil())
            .unwrap_or_default();
        let was_sync = self.selection_sync.replace(true);
        for stencil in &stencils {
            stencil.set_current_value(ToggleButton::is_checked_property(), Some(enabled));
        }
        let rulers: Vec<_> = self
            .registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<InkToolbarToggleButton>())
            .filter(|b| b.toggle_kind() == InkToolbarToggle::Ruler)
            .collect();
        for toggle in &rulers {
            toggle.set_current_value(ToggleButton::is_checked_property(), Some(ruler));
        }
        self.selection_sync.set(was_sync);
        // As WinUI, `IsRulerButtonChecked` shows the ruler only when there is a ruler button.
        let ruler = ruler && !rulers.is_empty();
        if let Some(target) = self.effective_ink_presenter() {
            target
                .set_ruler_visible(ruler || (enabled && selected == InkToolbarStencilKind::Ruler));
            target.set_protractor_visible(
                !ruler && enabled && selected == InkToolbarStencilKind::Protractor,
            );
        }
    }
    fn raise_stencil_changed(&self) {
        let Some(stencil_button) = self
            .registered_children()
            .snapshot()
            .iter()
            .find_map(|c| c.cast::<InkToolbarStencilButton>())
        else {
            return;
        };
        let args = InkToolbarIsStencilButtonCheckedChangedEventArgs {
            stencil_kind: stencil_button.selected_stencil(),
            stencil_button,
        };
        for (_, handler) in self.stencil_changed.snapshot().iter() {
            handler(&args);
        }
    }
    fn update_flyout_placement(&self) {
        let placement = match self.button_flyout_placement() {
            InkToolbarButtonFlyoutPlacement::Top => PlacementMode::TopEdgeAlignedLeft,
            InkToolbarButtonFlyoutPlacement::Bottom | InkToolbarButtonFlyoutPlacement::Auto => {
                PlacementMode::BottomEdgeAlignedLeft
            }
            InkToolbarButtonFlyoutPlacement::Left => PlacementMode::Left,
            InkToolbarButtonFlyoutPlacement::Right => PlacementMode::Right,
        };
        for button in self
            .registered_children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<Button>())
        {
            if let Some(flyout) = button.flyout().and_then(|f| f.cast::<PopupFlyoutBase>()) {
                flyout.set_placement(placement);
            }
            for (name, direction) in [
                (":flyout-top", PlacementMode::TopEdgeAlignedLeft),
                (":flyout-bottom", PlacementMode::BottomEdgeAlignedLeft),
                (":flyout-left", PlacementMode::Left),
                (":flyout-right", PlacementMode::Right),
            ] {
                button.pseudo_classes().set(name, placement == direction);
            }
        }
    }
}
/// Use the same device-pixel rounding as Border and Decorator while retaining native panel layout.
pub(crate) fn round_panel_thickness(control: &Layoutable, thickness: Thickness) -> Thickness {
    if control.use_layout_rounding() {
        LayoutHelper::round_layout_thickness(thickness, LayoutHelper::get_layout_scale(control))
    } else {
        thickness
    }
}
fn subscribe(
    list: &ToolbarHandlers,
    handler: impl Fn(&InkToolbar) + 'static,
) -> Rc<dyn IDisposable> {
    let token = list.add(Rc::new(handler));
    let weak = Rc::downgrade(list);
    Disposable::create(move || {
        if let Some(list) = weak.upgrade() {
            list.remove(token);
        }
    })
}

#[cfg(test)]
#[path = "ink_toolbar_tests.rs"]
mod tests;

/// The kind of a built-in toolbar button: the kinds the toolbar has at most one of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonKind {
    Tool(InkToolbarTool),
    Toggle(InkToolbarToggle),
    Menu(InkToolbarMenuKind),
}

fn button_kind(child: &Ref<Control>) -> Option<ButtonKind> {
    if let Some(tool) = child.cast::<InkToolbarToolButton>() {
        let kind = tool.tool_kind();
        return (!matches!(kind, InkToolbarTool::CustomPen | InkToolbarTool::CustomTool))
            .then_some(ButtonKind::Tool(kind));
    }
    if let Some(toggle) = child.cast::<InkToolbarToggleButton>() {
        let kind = toggle.toggle_kind();
        return (kind != InkToolbarToggle::Custom).then_some(ButtonKind::Toggle(kind));
    }
    child
        .cast::<InkToolbarMenuButton>()
        .map(|menu| ButtonKind::Menu(menu.menu_kind()))
}
