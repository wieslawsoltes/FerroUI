//! Pen palette and width editing built from the existing FerroUI controls.
use super::color_names::{bind_color_name, high_contrast_colors};
use super::ink_toolbar::round_panel_thickness;
use super::ink_toolbar_buttons::{strings, toolbar_style_key};
use super::{InkToolbarFlyoutItemKind, InkToolbarPenButton};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{RangeBase, TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use ferroui_controls::shapes::{Ellipse, Path};
use ferroui_controls::utils::BorderRenderHelper;
use ferroui_controls::{
    Border, ButtonImpl, Canvas, ContentControlImpl, Control, ControlImpl, Decorator, PanelImpl,
    Slider, SliderImpl, StackPanel, StackPanelImpl, TextBlock, WrapPanel,
};
use ferroui_base::animation::easings::CubicEaseOut;
use ferroui_base::animation::{Animation, Cue, FillMode, IClock, IGlobalClock, KeyFrame, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyboardNavigation,
    KeyboardNavigationMode, NavigationMethod,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt, Orientation};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    BackgroundSizing, BoxShadows, DrawingContext, FontWeight, IBrush, ITransform, MatrixTransform,
    PenLineCap, PenLineJoin, PolylineGeometry, TransformGroup, TranslateTransform,
};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable, Observable};
use ferroui_base::styling::Setter;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, CornerRadius,
    DirectProperty, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Point, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, Thickness, TypeInfo, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct InkToolbarStrokeWidthSlider {
    base: Slider,
}
ferro_class!(InkToolbarStrokeWidthSlider:Slider);
ferro_class_info!(InkToolbarStrokeWidthSlider {
    new: InkToolbarStrokeWidthSlider::new
});
ferro_impl_classes!(InkToolbarStrokeWidthSlider: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, SliderImpl);
impl ControlImpl for InkToolbarStrokeWidthSlider {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::InkToolbarStrokeWidthSliderAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarStrokeWidthSlider {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        toolbar_style_key(this, Slider::TYPE)
    }
}
impl FerroObjectImpl for InkToolbarStrokeWidthSlider {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_small_change(0.5);
        this.set_large_change(2.0);
        strings::bind_name(this, "InkToolbarPenConfigurationSizeSliderName");
    }
}
impl InkToolbarStrokeWidthSlider {
    pub fn construct() -> Self {
        Self {
            base: Slider::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[repr(C)]
pub struct InkToolbarPenConfigurationControl {
    base: StackPanel,
    // The pen's flyout owns its configuration. Keep the reverse owner link weak.
    pen: RefCell<Option<WeakRef<InkToolbarPenButton>>>,
    pen_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    palette_subscription: RefCell<Option<(FerroList<Rc<dyn IBrush>>, u64)>>,
    palette_panel: RefCell<Option<Ref<WrapPanel>>>,
    slider: RefCell<Option<Ref<InkToolbarStrokeWidthSlider>>>,
    syncing: Cell<bool>,
    displayed_palette: RefCell<Vec<Rc<dyn IBrush>>>,
    preview: RefCell<Option<(Ref<Canvas>, Ref<Path>)>>,
    border_renderer: RefCell<BorderRenderHelper>,
    entrance: RefCell<Option<Rc<CompositeDisposable>>>,
    entrance_epoch: Cell<u64>,
}
ferro_class!(InkToolbarPenConfigurationControl:StackPanel);
ferro_class_info!(InkToolbarPenConfigurationControl {
    new: InkToolbarPenConfigurationControl::new
});
ferro_impl_classes!(InkToolbarPenConfigurationControl:StyledElementImpl,InteractiveImpl,ControlImpl,PanelImpl);
impl LayoutableImpl for InkToolbarPenConfigurationControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let insets = this.layout_insets();
        Self::parent_measure_override(this, available_size.deflate(insets)).inflate(insets)
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        Self::parent_arrange_override(this, final_size.deflate(this.layout_insets()));
        final_size
    }
}
impl StackPanelImpl for InkToolbarPenConfigurationControl {
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
impl VisualImpl for InkToolbarPenConfigurationControl {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        this.start_entrance();
    }
    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        this.stop_entrance();
        Self::parent_on_detached_from_visual_tree(this, event);
    }
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
impl InputElementImpl for InkToolbarPenConfigurationControl {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if event.handled() {
            return;
        }
        let panel = this.palette_panel.borrow().clone();
        let Some(panel) = panel else {
            return;
        };
        let children = panel.children().snapshot();
        let Some(index) = children
            .iter()
            .position(|child| child.is_keyboard_focus_within())
        else {
            return;
        };
        let destination = match event.key {
            Key::Left => index.checked_sub(1),
            Key::Right => index.checked_add(1),
            Key::Up => index.checked_sub(6),
            Key::Down => index.checked_add(6),
            Key::Home => Some(0),
            Key::End => children.len().checked_sub(1),
            _ => return,
        };
        if let Some(destination) = destination.filter(|&index| index < children.len()) {
            let child = &children[destination];
            if child.focus_with(NavigationMethod::Directional, event.key_modifiers) {
                if let Some(pen) = this.pen_button() {
                    pen.set_selected_brush_index(destination as i32);
                }
            }
        }
        event.set_handled(true);
    }
}
impl FerroObjectImpl for InkToolbarPenConfigurationControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_orientation(Orientation::Vertical);
        this.set_spacing(8.0);
        let palette = WrapPanel::new();
        palette.set_width(312.0);
        palette.set_margin(Thickness::new(4.0, 0.0, 4.0, 2.0));
        KeyboardNavigation::set_tab_navigation(&palette, KeyboardNavigationMode::Once);
        strings::bind_name(&palette, "InkToolbarPenConfigurationColorsLabel");
        let colors_title = TextBlock::new();
        colors_title.set_font_weight(FontWeight::SemiBold);
        colors_title.set_margin(Thickness::new(12.0, 0.0, 12.0, 0.0));
        strings::bind_label(&colors_title, "InkToolbarPenConfigurationColorsLabel");
        AutomationProperties::set_heading_level(&colors_title, 2);
        let size_title = TextBlock::new();
        size_title.set_font_weight(FontWeight::SemiBold);
        size_title.set_margin(Thickness::new(12.0, 0.0, 12.0, 0.0));
        strings::bind_label(&size_title, "InkToolbarPenConfigurationSizeLabel");
        AutomationProperties::set_heading_level(&size_title, 2);
        let preview_canvas = Canvas::new();
        preview_canvas.set_width(296.0);
        preview_canvas.set_margin(Thickness::new(12.0, 0.0, 12.0, 4.0));
        preview_canvas.set_is_hit_test_visible(false);
        let preview = Path::new();
        preview.set_data(PolylineGeometry::with_points(PREVIEW_POINTS, false));
        preview.set_stroke_line_cap(PenLineCap::Round);
        preview.set_stroke_join(PenLineJoin::Round);
        preview_canvas.children().add(preview.clone());
        let slider = InkToolbarStrokeWidthSlider::new();
        slider.set_width(296.0);
        slider.set_height(44.0);
        slider.set_margin(Thickness::new(12.0, 0.0, 12.0, 0.0));
        let weak = this.to_ref().downgrade();
        slider.property_changed(move |change| {
            if change.property() != RangeBase::value_property().as_property() {
                return;
            }
            if let Some(config) = weak.upgrade() {
                if !config.syncing.get() {
                    if let Some(pen) = config.pen_button() {
                        pen.set_selected_stroke_width(change.get_new_value::<f64>());
                    }
                }
            }
        });
        this.children().add(colors_title);
        this.children().add(palette.clone());
        this.children().add(size_title);
        this.children().add(preview_canvas.clone());
        this.children().add(slider.clone());
        *this.palette_panel.borrow_mut() = Some(palette);
        *this.slider.borrow_mut() = Some(slider);
        *this.preview.borrow_mut() = Some((preview_canvas, preview));
        let weak = this.to_ref().downgrade();
        this.resources_changed(move |_| {
            if let Some(config) = weak.upgrade() {
                config.rebuild_palette();
            }
        });
        this.bind_pen();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::pen_button_property().as_property() {
            this.bind_pen();
        }
    }
}
ferro_properties! {impl InkToolbarPenConfigurationControl{
    pub fn pen_button_property()->DirectProperty<InkToolbarPenConfigurationControl,Option<Ref<InkToolbarPenButton>>>{FerroProperty::register_direct::<InkToolbarPenConfigurationControl,_>("PenButton",InkToolbarPenConfigurationControl::pen_button,None,None)}
    pub fn border_brush_property()->StyledProperty<Option<Rc<dyn IBrush>>>{Border::border_brush_property().add_owner::<InkToolbarPenConfigurationControl>()}
    pub fn border_thickness_property()->StyledProperty<Thickness>{Border::border_thickness_property().add_owner::<InkToolbarPenConfigurationControl>()}
    pub fn corner_radius_property()->StyledProperty<CornerRadius>{Border::corner_radius_property().add_owner::<InkToolbarPenConfigurationControl>()}
    pub fn background_sizing_property()->StyledProperty<BackgroundSizing>{Border::background_sizing_property().add_owner::<InkToolbarPenConfigurationControl>()}
    pub fn padding_property()->StyledProperty<Thickness>{Decorator::padding_property().add_owner::<InkToolbarPenConfigurationControl>()}
}}
impl InkToolbarPenConfigurationControl {
    fn static_constructor() {
        Visual::affects_render::<InkToolbarPenConfigurationControl>(&[
            Self::border_brush_property().as_property(),
            Self::border_thickness_property().as_property(),
            Self::corner_radius_property().as_property(),
            Self::background_sizing_property().as_property(),
        ]);
        Layoutable::affects_measure::<InkToolbarPenConfigurationControl>(&[
            Self::padding_property().as_property(),
            Self::border_thickness_property().as_property(),
        ]);
    }
    pub fn construct() -> Self {
        Self {
            base: StackPanel::construct(),
            pen: RefCell::new(None),
            pen_subscription: RefCell::new(None),
            palette_subscription: RefCell::new(None),
            palette_panel: RefCell::new(None),
            slider: RefCell::new(None),
            syncing: Cell::new(false),
            displayed_palette: RefCell::new(Vec::new()),
            preview: RefCell::new(None),
            border_renderer: RefCell::new(BorderRenderHelper::new()),
            entrance: RefCell::new(None),
            entrance_epoch: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn for_pen(pen: &InkToolbarPenButton) -> Ref<Self> {
        let result = Self::new();
        *result.pen.borrow_mut() = Some(pen.to_ref().downgrade());
        result.raise_direct_property_changed(
            Self::pen_button_property(),
            &None,
            &Some(pen.to_ref()),
        );
        result
    }
    pub fn pen_button(&self) -> Option<Ref<InkToolbarPenButton>> {
        self.pen.borrow().as_ref().and_then(WeakRef::upgrade)
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
    pub fn stroke_width_slider(&self) -> Option<Ref<InkToolbarStrokeWidthSlider>> {
        self.slider.borrow().clone()
    }
    /// Portable adaptation of InkToolbar.xaml's Grid.ChildrenTransitions / EntranceThemeTransition.
    /// Each visible section enters from 40 DIPs to the right over 200 ms, staggered by 30 ms.
    /// The framework clock, key-frame animation and property priorities own interpolation;
    /// no local opacity or render-transform value is replaced by the transition.
    fn start_entrance(&self) {
        self.stop_entrance();
        let clock = self.clock().or_else(|| {
            FerroLocator::current()
                .get_service::<dyn IGlobalClock>()
                .map(|clock| clock as Rc<dyn IClock>)
        });
        let Some(clock) = clock else {
            // A host without an animation clock still has a fully visible configuration.
            return;
        };
        let children: Vec<_> = self
            .children()
            .snapshot()
            .iter()
            .filter(|child| child.is_visible())
            .cloned()
            .collect();
        if children.is_empty() {
            return;
        }
        let subscriptions = Rc::new(CompositeDisposable::new());
        *self.entrance.borrow_mut() = Some(subscriptions.clone());
        let epoch = self.entrance_epoch.get();
        let remaining = Rc::new(Cell::new(children.len()));
        for (index, child) in children.iter().enumerate() {
            let transform = TransformGroup::new();
            if let Some(original) = child.render_transform() {
                transform
                    .children()
                    .add(MatrixTransform::with_matrix(original.value()).upcast());
            }
            transform.children().add(TranslateTransform::new().upcast());
            let transform: Rc<dyn ITransform> = transform.into();
            if let Some(overlay) = child.set_value_with_priority(
                Visual::render_transform_property(),
                Some(transform),
                BindingPriority::Animation,
            ) {
                subscriptions.add(overlay);
            }
            let animation = Animation::new();
            animation.set_duration(TimeSpan::from_milliseconds(200.0));
            animation.set_delay(TimeSpan::from_milliseconds(index as f64 * 30.0));
            animation.set_fill_mode(FillMode::Backward);
            animation.set_easing(CubicEaseOut::new());
            animation.children().add(KeyFrame::with_cue(
                Cue::new(0.0),
                [
                    Setter::new(Visual::opacity_property(), 0.0) as _,
                    Setter::new(TranslateTransform::x_property(), 40.0) as _,
                ],
            ));
            // A valueless key-frame setter uses the current base value as its endpoint,
            // including a new application opacity written while the entrance is running.
            let final_opacity = Setter::empty();
            final_opacity.set_property(Some(Visual::opacity_property().as_property()));
            animation.children().add(KeyFrame::with_cue(
                Cue::new(1.0),
                [
                    final_opacity as _,
                    Setter::new(TranslateTransform::x_property(), 0.0) as _,
                ],
            ));
            let weak = self.to_ref().downgrade();
            let remaining = remaining.clone();
            let completed = Rc::new(move || {
                remaining.set(remaining.get().saturating_sub(1));
                if remaining.get() == 0 {
                    if let Some(configuration) = weak.upgrade() {
                        if configuration.entrance_epoch.get() == epoch {
                            configuration.stop_entrance();
                        }
                    }
                }
            });
            subscriptions.add(animation.apply(
                child,
                Some(clock.clone()),
                Observable::return_(true),
                Some(completed),
                true,
            ));
        }
    }
    fn stop_entrance(&self) {
        self.entrance_epoch
            .set(self.entrance_epoch.get().wrapping_add(1));
        let subscriptions = self.entrance.borrow_mut().take();
        if let Some(subscriptions) = subscriptions {
            subscriptions.dispose();
        }
    }
    fn bind_pen(&self) {
        if let Some(subscription) = self.pen_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(pen) = self.pen_button() {
            let weak = self.to_ref().downgrade();
            let subscription = pen.property_changed(move |change| {
                if let Some(config) = weak.upgrade() {
                    if !super::ink_toolbar_buttons::affects_pen_attributes(change.property()) {
                        return;
                    }
                    if change.property() == InkToolbarPenButton::palette_property().as_property() {
                        config.bind_palette();
                    }
                    let palette_changed = config.displayed_palette.borrow().as_slice()
                        != pen_palette(&config).as_slice();
                    if palette_changed {
                        config.rebuild_palette();
                    }
                    config.synchronize();
                }
            });
            *self.pen_subscription.borrow_mut() = Some(subscription);
        }
        self.bind_palette();
        self.synchronize();
    }
    fn bind_palette(&self) {
        if let Some((palette, token)) = self.palette_subscription.borrow_mut().take() {
            palette.remove_collection_changed(token);
        }
        if let Some(pen) = self.pen_button() {
            let palette = pen.palette();
            let weak = self.to_ref().downgrade();
            let token = palette.add_collection_changed(Rc::new(move |_| {
                if let Some(config) = weak.upgrade() {
                    config.rebuild_palette();
                }
            }));
            *self.palette_subscription.borrow_mut() = Some((palette, token));
        }
        self.rebuild_palette();
    }
    fn rebuild_palette(&self) {
        let panel = self.palette_panel.borrow().clone();
        let Some(panel) = panel else {
            return;
        };
        panel.children().clear();
        let Some(pen) = self.pen_button() else {
            return;
        };
        let effective_palette = pen.effective_palette();
        *self.displayed_palette.borrow_mut() = effective_palette.clone();
        for (index, brush) in effective_palette.iter().enumerate() {
            let item = InkToolbarFlyoutItem::new();
            item.set_kind(InkToolbarFlyoutItemKind::Radio);
            item.set_width(52.0);
            item.set_height(52.0);
            item.classes().add("ink-palette-item");
            let swatch = Ellipse::new();
            swatch.set_width(32.0);
            swatch.set_height(32.0);
            swatch.set_fill(Some(brush.clone()));
            if let Some((_, system)) = high_contrast_colors(self, false) {
                swatch.set_stroke(Some(
                    Rc::new(ImmutableSolidColorBrush::new(system)) as Rc<dyn IBrush>
                ));
                swatch.set_stroke_thickness(1.0);
            }
            item.set_content(Some(Rc::new(swatch.upcast::<Control>())));
            bind_color_name(
                &item,
                brush.as_solid_color_brush().map(|brush| brush.color()),
            );
            let weak = pen.downgrade();
            item.checked(move |_| {
                if let Some(pen) = weak.upgrade() {
                    pen.set_selected_brush_index(index as i32);
                }
            });
            panel.children().add(item);
        }
        self.synchronize();
    }
    fn synchronize(&self) {
        if self.syncing.replace(true) {
            return;
        }
        let pen = self.pen_button();
        let slider = self.slider.borrow().clone();
        let panel = self.palette_panel.borrow().clone();
        if let (Some(pen), Some(slider)) = (pen.as_ref(), slider) {
            slider.set_minimum(pen.min_stroke_width());
            slider.set_maximum(pen.max_stroke_width().max(pen.min_stroke_width()));
            slider.set_range_value(pen.selected_stroke_width());
        }
        if let (Some(pen), Some(panel)) = (pen, panel) {
            for (index, item) in panel.children().snapshot().iter().enumerate() {
                if let Some(item) = item.cast::<InkToolbarFlyoutItem>() {
                    let selected = index as i32 == pen.selected_brush_index();
                    item.set_is_checked(Some(selected));
                    if selected {
                        KeyboardNavigation::set_tab_once_active_element(
                            &panel,
                            Some(item.upcast()),
                        );
                    }
                }
            }
        }
        self.syncing.set(false);
        self.update_preview();
    }
    /// The sample stroke, rendered with the active pen's drawing attributes.
    pub fn stroke_preview(&self) -> Option<Ref<Path>> {
        self.preview.borrow().as_ref().map(|(_, path)| path.clone())
    }
    fn update_preview(&self) {
        let pen = self.pen_button();
        let preview = self.preview.borrow().clone();
        let (Some(pen), Some((canvas, path))) = (pen, preview) else {
            return;
        };
        let attrs = pen.create_ink_drawing_attributes().ok();
        let brush = attrs
            .map(|attrs| Rc::new(ImmutableSolidColorBrush::new(attrs.color)) as Rc<dyn IBrush>)
            .or_else(|| pen.selected_brush());
        path.set_stroke(brush);
        path.set_stroke_thickness(
            attrs
                .map(|attrs| attrs.size.width)
                .unwrap_or_else(|| pen.selected_stroke_width()),
        );
        path.set_opacity(if attrs.is_some_and(|attrs| attrs.draw_as_highlighter) {
            0.5
        } else {
            1.0
        });
        let min_y = PREVIEW_POINTS
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = PREVIEW_POINTS
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        let height = max_y - min_y + pen.max_stroke_width().max(path.stroke_thickness()) + 10.0;
        canvas.set_height(height);
        Canvas::set_top(&path, height / 2.0 - (min_y + max_y) / 2.0);
    }
}
impl Drop for InkToolbarPenConfigurationControl {
    fn drop(&mut self) {
        self.stop_entrance();
        if let Some(subscription) = self.pen_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some((palette, token)) = self.palette_subscription.get_mut().take() {
            palette.remove_collection_changed(token);
        }
    }
}
fn pen_palette(config: &InkToolbarPenConfigurationControl) -> Vec<Rc<dyn IBrush>> {
    config
        .pen_button()
        .map(|pen| pen.effective_palette())
        .unwrap_or_default()
}

type ItemHandlers = Rc<HandlerList<dyn Fn(&InkToolbarFlyoutItem)>>;
#[repr(C)]
pub struct InkToolbarFlyoutItem {
    base: ToggleButton,
    checked: ItemHandlers,
    unchecked: ItemHandlers,
}
ferro_class!(InkToolbarFlyoutItem:ToggleButton);
ferro_class_info!(InkToolbarFlyoutItem {
    new: InkToolbarFlyoutItem::new
});
ferro_impl_classes!(InkToolbarFlyoutItem: VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl);
impl ControlImpl for InkToolbarFlyoutItem {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::InkToolbarFlyoutItemAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarFlyoutItem {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        toolbar_style_key(this, ToggleButton::TYPE)
    }
}
impl FerroObjectImpl for InkToolbarFlyoutItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == ToggleButton::is_checked_property().as_property() {
            if this.is_checked() == Some(true) {
                this.uncheck_others_in_group();
            }
            let handlers = if this.is_checked() == Some(true) {
                &this.checked
            } else {
                &this.unchecked
            };
            for (_, handler) in handlers.snapshot().iter() {
                handler(this);
            }
        }
    }
}
impl ToggleButtonImpl for InkToolbarFlyoutItem {
    fn toggle(this: &Self) {
        let value = match this.kind() {
            InkToolbarFlyoutItemKind::Simple => return,
            InkToolbarFlyoutItemKind::Radio => true,
            InkToolbarFlyoutItemKind::Check | InkToolbarFlyoutItemKind::RadioCheck => {
                this.is_checked() != Some(true)
            }
        };
        this.set_current_value(ToggleButton::is_checked_property(), Some(value));
    }
}
ferro_properties! {impl InkToolbarFlyoutItem{pub fn kind_property()->StyledProperty<InkToolbarFlyoutItemKind>{FerroProperty::register::<InkToolbarFlyoutItem,_>("Kind",InkToolbarFlyoutItemKind::Simple)}}}
impl InkToolbarFlyoutItem {
    pub fn construct() -> Self {
        Self {
            base: ToggleButton::construct(),
            checked: Rc::new(HandlerList::new()),
            unchecked: Rc::new(HandlerList::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// As WinUI (`ForAllOthersInGroup`): a radio item that becomes checked unchecks the other
    /// items of its kind in the same panel of the flyout.
    fn uncheck_others_in_group(&self) {
        let kind = self.kind();
        if !matches!(
            kind,
            InkToolbarFlyoutItemKind::Radio | InkToolbarFlyoutItemKind::RadioCheck
        ) {
            return;
        }
        let Some(panel) = self.parent().and_then(|parent| parent.cast::<ferroui_controls::Panel>()) else {
            return;
        };
        let this: Ref<Control> = self.to_ref().upcast();
        for item in panel
            .children()
            .snapshot()
            .iter()
            .filter(|child| **child != this)
            .filter_map(|child| child.cast::<InkToolbarFlyoutItem>())
            .filter(|item| item.kind() == kind && item.is_checked() == Some(true))
        {
            item.set_is_checked(Some(false));
        }
    }
    pub fn kind(&self) -> InkToolbarFlyoutItemKind {
        self.get_value(Self::kind_property())
    }
    pub fn set_kind(&self, value: InkToolbarFlyoutItemKind) {
        self.set_value(Self::kind_property(), value);
    }
    pub fn checked(
        &self,
        handler: impl Fn(&InkToolbarFlyoutItem) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.checked, handler)
    }
    pub fn unchecked(
        &self,
        handler: impl Fn(&InkToolbarFlyoutItem) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.unchecked, handler)
    }
}
fn subscribe(
    list: &ItemHandlers,
    handler: impl Fn(&InkToolbarFlyoutItem) + 'static,
) -> Rc<dyn IDisposable> {
    let token = list.add(Rc::new(handler));
    let weak = Rc::downgrade(list);
    Disposable::create(move || {
        if let Some(list) = weak.upgrade() {
            list.remove(token);
        }
    })
}

// The source sample stroke; its fixed bounds avoid preview movement while resizing the nib.
const PREVIEW_POINTS: [Point; 103] = [
    Point::new(24.0, 28.5),
    Point::new(26.175271, 26.452467),
    Point::new(28.468156, 24.57762),
    Point::new(30.860594, 22.862812),
    Point::new(33.253032, 21.148004),
    Point::new(35.745021, 19.593234),
    Point::new(38.3185, 18.185855),
    Point::new(40.891979, 16.778476),
    Point::new(43.546946, 15.518487),
    Point::new(46.26534, 14.39324),
    Point::new(48.983733, 13.267993),
    Point::new(51.765552, 12.277489),
    Point::new(54.592735, 11.409078),
    Point::new(57.419917, 10.540668),
    Point::new(60.292463, 9.794352),
    Point::new(63.192308, 9.1574822),
    Point::new(66.092153, 8.5206123),
    Point::new(69.019299, 7.9931887),
    Point::new(71.955681, 7.5625631),
    Point::new(74.892063, 7.1319376),
    Point::new(77.837682, 6.7981102),
    Point::new(80.774476, 6.5484328),
    Point::new(83.711269, 6.2987555),
    Point::new(86.639236, 6.1332282),
    Point::new(89.540314, 6.0392029),
    Point::new(92.12168, 5.9271775),
    Point::new(94.691318, 5.9313799),
    Point::new(97.248453, 6.040089),
    Point::new(99.805587, 6.1487981),
    Point::new(102.35022, 6.3620141),
    Point::new(104.88157, 6.668016),
    Point::new(107.41293, 6.974018),
    Point::new(109.931, 7.3728059),
    Point::new(112.43502, 7.852659),
    Point::new(114.93904, 8.332512),
    Point::new(117.42901, 8.8934302),
    Point::new(119.90414, 9.5236926),
    Point::new(122.37928, 10.153955),
    Point::new(124.83959, 10.853562),
    Point::new(127.28429, 11.610792),
    Point::new(129.72899, 12.368022),
    Point::new(132.15809, 13.182876),
    Point::new(134.57081, 14.043632),
    Point::new(136.98352, 14.904388),
    Point::new(139.37986, 15.811047),
    Point::new(141.75904, 16.751888),
    Point::new(144.13822, 17.692728),
    Point::new(146.50025, 18.667751),
    Point::new(148.84434, 19.665234),
    Point::new(150.92055, 20.558561),
    Point::new(153.02009, 21.393996),
    Point::new(155.13997, 22.171709),
    Point::new(157.25986, 22.949423),
    Point::new(159.40009, 23.669414),
    Point::new(161.55771, 24.331854),
    Point::new(163.71533, 24.994294),
    Point::new(165.89033, 25.599182),
    Point::new(168.07973, 26.146689),
    Point::new(170.26914, 26.694195),
    Point::new(172.47295, 27.184321),
    Point::new(174.6882, 27.617235),
    Point::new(176.90346, 28.050149),
    Point::new(179.13015, 28.425851),
    Point::new(181.3653, 28.744513),
    Point::new(183.60046, 29.063175),
    Point::new(185.84408, 29.324795),
    Point::new(188.0932, 29.529545),
    Point::new(190.34231, 29.734295),
    Point::new(192.59692, 29.882174),
    Point::new(194.85406, 29.973352),
    Point::new(197.11119, 30.06453),
    Point::new(199.37085, 30.099008),
    Point::new(201.63006, 30.076955),
    Point::new(203.88927, 30.054901),
    Point::new(206.14803, 29.976318),
    Point::new(208.40338, 29.841374),
    Point::new(210.65872, 29.70643),
    Point::new(212.91064, 29.515126),
    Point::new(215.15617, 29.267631),
    Point::new(219.64724, 28.772643),
    Point::new(224.11274, 28.052893),
    Point::new(228.52891, 27.109745),
    Point::new(230.73699, 26.638171),
    Point::new(232.93274, 26.110747),
    Point::new(235.11319, 25.527643),
    Point::new(237.29363, 24.94454),
    Point::new(239.45877, 24.305756),
    Point::new(241.60564, 23.611464),
    Point::new(243.7525, 22.917171),
    Point::new(245.88109, 22.167369),
    Point::new(247.98843, 21.362228),
    Point::new(250.09577, 20.557087),
    Point::new(252.18187, 19.696606),
    Point::new(254.24374, 18.780957),
    Point::new(257.44071, 17.278777),
    Point::new(260.5682, 15.598548),
    Point::new(263.55223, 13.698859),
    Point::new(265.04425, 12.749015),
    Point::new(266.50041, 11.744305),
    Point::new(267.91145, 10.679554),
    Point::new(269.32249, 9.6148034),
    Point::new(270.68842, 8.4900108),
    Point::new(272.0, 7.3),
];

#[cfg(test)]
#[path = "ink_toolbar_configuration_tests.rs"]
mod tests;
