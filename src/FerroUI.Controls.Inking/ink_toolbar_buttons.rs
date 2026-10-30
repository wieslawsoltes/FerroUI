//! Toolbar controls adapted from the WinUI InkToolbar control family.
use super::color_names::{
    color_name, color_resource_key, contrast_ratio, default_palette, high_contrast_colors,
};
#[path = "ink_toolbar_strings.rs"]
pub(crate) mod strings;
use super::{
    InkDrawingAttributes, InkError, InkHighContrastAdjustment, InkToolbar, InkToolbarFlyoutItem,
    InkToolbarFlyoutItemKind, InkToolbarMenuKind, InkToolbarStencilKind, InkToolbarToggle,
    InkToolbarTool, PenTipShape,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use ferroui_controls::{
    Button, ButtonImpl, ButtonImplExt, CheckBox, ContentControlImpl, Control, ControlImpl, Flyout,
    RadioButton, StackPanel,
};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{
    HoldingRoutedEventArgs, HoldingState, InputElement, InputElementImpl, InputElementImplExt,
    TappedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElement, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Palette = FerroList<Rc<dyn IBrush>>;

/// A mutually exclusive toolbar tool; inherits keyboard and selection automation from RadioButton.
#[repr(C)]
pub struct InkToolbarToolButton {
    base: RadioButton,
    tool_kind: Cell<InkToolbarTool>,
    permit_flyout: Cell<bool>,
}
ferro_class!(InkToolbarToolButton: RadioButton);
ferro_class_info!(InkToolbarToolButton { new: InkToolbarToolButton::new, markup: {
    properties: [ToolKind: InkToolbarTool { get: InkToolbarToolButton::tool_kind }],
} });
ferro_impl_classes!(InkToolbarToolButton: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, TemplatedControlImpl, ContentControlImpl, ToggleButtonImpl);
impl InputElementImpl for InkToolbarToolButton {
    fn on_right_tapped(this: &Self, event: &TappedEventArgs) {
        Self::parent_on_right_tapped(this, event);
        if this.is_effectively_enabled() {
            this.activate_secondary();
            event.set_handled(true);
        }
    }
    fn on_holding(this: &Self, event: &HoldingRoutedEventArgs) {
        Self::parent_on_holding(this, event);
        if this.is_effectively_enabled() && event.holding_state() == HoldingState::Started {
            this.activate_secondary();
            event.set_handled(true);
        }
    }
}
impl ButtonImpl for InkToolbarToolButton {
    fn on_click(this: &Self) {
        let was_checked = this.is_checked() == Some(true);
        let previous = this.permit_flyout.replace(was_checked);
        Self::parent_on_click(this);
        this.permit_flyout.set(previous);
    }
    fn open_flyout(this: &Self) {
        if this.permit_flyout.get() && this.can_open_configuration() {
            Self::parent_open_flyout(this);
        }
    }
}
impl ControlImpl for InkToolbarToolButton {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::InkToolbarToolButtonAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarToolButton {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        toolbar_style_key(this, RadioButton::TYPE)
    }
}
ferro_properties! { impl InkToolbarToolButton {
    pub fn is_extension_glyph_shown_property() -> StyledProperty<bool> { FerroProperty::register::<InkToolbarToolButton,_>("IsExtensionGlyphShown", true) }
} }
impl InkToolbarToolButton {
    fn static_constructor() {
        InputElement::is_holding_enabled_property()
            .override_default_value::<InkToolbarToolButton>(true);
    }
    fn activate_secondary(&self) {
        self.set_current_value(ToggleButton::is_checked_property(), Some(true));
        self.open_flyout();
    }
    pub(crate) fn can_open_configuration(&self) -> bool {
        !toolbar_is_inking(self)
            && self
                .to_ref()
                .cast::<InkToolbarCustomPenButton>()
                .is_none_or(|pen| pen.configuration_content().is_some())
            && self
                .to_ref()
                .cast::<InkToolbarCustomToolButton>()
                .is_none_or(|tool| tool.configuration_content().is_some())
            && self
                .to_ref()
                .cast::<InkToolbarEraserButton>()
                .is_none_or(|eraser| eraser.is_clear_all_visible())
    }
    pub fn construct() -> Self {
        Self {
            base: RadioButton::construct(),
            tool_kind: Cell::new(InkToolbarTool::CustomTool),
            permit_flyout: Cell::new(true),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn tool_kind(&self) -> InkToolbarTool {
        self.tool_kind.get()
    }
    pub(crate) fn set_tool_kind(&self, value: InkToolbarTool) {
        self.tool_kind.set(value);
    }
    pub fn is_extension_glyph_shown(&self) -> bool {
        self.get_value(Self::is_extension_glyph_shown_property())
    }
    pub fn set_is_extension_glyph_shown(&self, value: bool) {
        self.set_value(Self::is_extension_glyph_shown_property(), value);
    }
}

/// Base pen tool. Palettes are observable handles owned independently by each pen.
#[repr(C)]
pub struct InkToolbarPenButton {
    base: InkToolbarToolButton,
    palette_subscription: RefCell<Option<(Palette, u64)>>,
    selected_brush: RefCell<Option<Rc<dyn IBrush>>>,
    contrast_palette: RefCell<Option<Vec<Rc<dyn IBrush>>>>,
    contrast_signature: Cell<Option<(InkHighContrastAdjustment, Color, Color)>>,
    normal_index: Cell<i32>,
    contrast_index: Cell<i32>,
}
ferro_class! { InkToolbarPenButton: InkToolbarToolButton, virtuals InkToolbarPenButtonImpl: ToggleButtonImpl {
    fn create_ink_drawing_attributes(this) -> Result<InkDrawingAttributes, InkError>;
} }
ferro_class_info!(InkToolbarPenButton {
    new: InkToolbarPenButton::new
});
ferro_impl_classes!(InkToolbarPenButton: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl, ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarPenButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_palette(default_palette(false));
        let weak = this.to_ref().downgrade();
        this.resources_changed(move |_| {
            if let Some(pen) = weak.upgrade() {
                pen.refresh_palette();
            }
        });
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(pen) = weak.upgrade() {
                pen.refresh_palette();
            }
        });
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::palette_property().as_property() {
            if let Some((old, token)) = this.palette_subscription.borrow_mut().take() {
                old.remove_collection_changed(token);
            }
            let palette = this.palette();
            let weak = this.to_ref().downgrade();
            let token = palette.add_collection_changed(Rc::new(move |_| {
                if let Some(pen) = weak.upgrade() {
                    pen.rebuild_contrast_palette(true);
                }
            }));
            *this.palette_subscription.borrow_mut() = Some((palette, token));
            this.rebuild_contrast_palette(true);
        } else if change.property() == Self::selected_brush_index_property().as_property() {
            this.update_selected_brush();
        } else if change.property() == Self::min_stroke_width_property().as_property()
            || change.property() == Self::max_stroke_width_property().as_property()
        {
            this.coerce_value(Self::selected_stroke_width_property().as_property());
        }
    }
}
impl InkToolbarPenButtonImpl for InkToolbarPenButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        let attrs = InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width, width),
            ..InkDrawingAttributes::default()
        };
        attrs.validate()?;
        Ok(attrs)
    }
}
ferro_properties! { impl InkToolbarPenButton {
    pub fn palette_property() -> StyledProperty<FerroList<Rc<dyn IBrush>>> { FerroProperty::register::<InkToolbarPenButton,_>("Palette", FerroList::new()) }
    pub fn min_stroke_width_property() -> StyledProperty<f64> { FerroProperty::register_with::<InkToolbarPenButton,_>("MinStrokeWidth", StyledPropertyOptions::new(0.5_f64).coerce(|_,v| if v.is_finite() && v > 0.0 {v} else {0.5})) }
    pub fn max_stroke_width_property() -> StyledProperty<f64> { FerroProperty::register_with::<InkToolbarPenButton,_>("MaxStrokeWidth", StyledPropertyOptions::new(32.0_f64).coerce(|_,v| if v.is_finite() && v > 0.0 {v} else {32.0})) }
    pub fn selected_brush_property() -> DirectProperty<InkToolbarPenButton,Option<Rc<dyn IBrush>>> { FerroProperty::register_direct::<InkToolbarPenButton,_>("SelectedBrush",InkToolbarPenButton::selected_brush,None,None) }
    pub fn selected_brush_index_property() -> StyledProperty<i32> { FerroProperty::register::<InkToolbarPenButton,_>("SelectedBrushIndex", 0) }
    pub fn selected_stroke_width_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<InkToolbarPenButton,_>("SelectedStrokeWidth", StyledPropertyOptions::new(2.0_f64).coerce(|sender,value| {
            let min = sender.get_value(Self::min_stroke_width_property());
            let max = sender.get_value(Self::max_stroke_width_property()).max(min);
            if value.is_finite() { value.clamp(min,max) } else { min }
        }))
    }
} }
impl InkToolbarPenButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToolButton::construct(),
            palette_subscription: RefCell::new(None),
            selected_brush: RefCell::new(None),
            contrast_palette: RefCell::new(None),
            contrast_signature: Cell::new(None),
            normal_index: Cell::new(0),
            contrast_index: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn palette(&self) -> Palette {
        self.get_value(Self::palette_property())
    }
    pub fn set_palette(&self, value: Palette) {
        self.set_value(Self::palette_property(), value);
    }
    pub fn min_stroke_width(&self) -> f64 {
        self.get_value(Self::min_stroke_width_property())
    }
    pub fn set_min_stroke_width(&self, value: f64) {
        self.set_value(Self::min_stroke_width_property(), value);
    }
    pub fn max_stroke_width(&self) -> f64 {
        self.get_value(Self::max_stroke_width_property())
    }
    pub fn set_max_stroke_width(&self, value: f64) {
        self.set_value(Self::max_stroke_width_property(), value);
    }
    pub fn selected_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.selected_brush.borrow().clone()
    }
    pub fn selected_brush_index(&self) -> i32 {
        self.get_value(Self::selected_brush_index_property())
    }
    pub fn set_selected_brush_index(&self, value: i32) {
        self.set_value(Self::selected_brush_index_property(), value);
    }
    pub fn selected_stroke_width(&self) -> f64 {
        self.get_value(Self::selected_stroke_width_property())
    }
    pub fn set_selected_stroke_width(&self, value: f64) {
        self.set_value(Self::selected_stroke_width_property(), value);
    }
    pub fn determine_stroke_width(&self) -> f64 {
        self.selected_stroke_width().clamp(
            self.min_stroke_width(),
            self.max_stroke_width().max(self.min_stroke_width()),
        )
    }
    pub fn selected_color(&self) -> Result<Color, InkError> {
        self.selected_brush()
            .and_then(|b| b.as_solid_color_brush().map(|s| s.color()))
            .ok_or(InkError::InvalidInput(
                "Ink pens require a selected solid-color brush",
            ))
    }
    pub(crate) fn effective_palette(&self) -> Vec<Rc<dyn IBrush>> {
        self.contrast_palette
            .borrow()
            .clone()
            .unwrap_or_else(|| self.palette().snapshot().as_ref().clone())
    }
    pub(crate) fn refresh_palette(&self) {
        self.rebuild_contrast_palette(false);
    }
    fn rebuild_contrast_palette(&self, force: bool) {
        let mut parent = self.parent();
        let mut mode = InkHighContrastAdjustment::UseSystemColorsWhenNecessary;
        while let Some(element) = parent {
            if let Some(toolbar) = element.cast::<InkToolbar>() {
                mode = toolbar
                    .effective_ink_presenter()
                    .map(|p| p.high_contrast_adjustment())
                    .unwrap_or(mode);
                break;
            }
            parent = element.parent();
        }
        let signature = high_contrast_colors(self, self.tool_kind() == InkToolbarTool::Highlighter)
            .map(|(background, system)| (mode, background, system));
        let previous = self.contrast_signature.replace(signature);
        if force || previous != signature {
            let palette = signature.map(|(mode, background, system)| {
                let mut result = self
                    .palette()
                    .snapshot()
                    .iter()
                    .filter(|brush| {
                        brush
                            .as_solid_color_brush()
                            .is_some_and(|brush| match mode {
                                InkHighContrastAdjustment::UseOriginalColors => true,
                                InkHighContrastAdjustment::UseSystemColors => false,
                                InkHighContrastAdjustment::UseSystemColorsWhenNecessary => {
                                    contrast_ratio(brush.color(), background) >= 7.0
                                }
                            })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if !result.iter().any(|brush| {
                    brush
                        .as_solid_color_brush()
                        .is_some_and(|brush| brush.color() == system)
                }) {
                    result.push(Rc::new(ImmutableSolidColorBrush::new(system)) as Rc<dyn IBrush>);
                }
                result
            });
            *self.contrast_palette.borrow_mut() = palette;
            if previous.is_some() != signature.is_some() {
                let index = if signature.is_some() {
                    self.contrast_index.get()
                } else {
                    self.normal_index.get()
                };
                self.set_current_value(Self::selected_brush_index_property(), index);
            }
        }
        self.update_selected_brush();
    }
    fn update_selected_brush(&self) {
        let index = self.selected_brush_index();
        let items = self.effective_palette();
        // The source retains the previous brush for an invalid palette index.
        let Some(selected) = (index >= 0)
            .then(|| items.get(index as usize).cloned())
            .flatten()
        else {
            return;
        };
        if self.contrast_signature.get().is_some() {
            self.contrast_index.set(index);
        } else {
            self.normal_index.set(index);
        }
        let selected = Some(selected);
        self.set_and_raise(
            Self::selected_brush_property(),
            &self.selected_brush,
            selected.clone(),
        );
        let help = selected.and_then(|b| {
            b.as_solid_color_brush().and_then(|brush| {
                color_resource_key(brush.color()).map(|_| color_name(self, brush.color()))
            })
        });
        AutomationProperties::set_help_text(self, help.as_deref());
    }
}

#[repr(C)]
pub struct InkToolbarBallpointPenButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarBallpointPenButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarBallpointPenButton {
    new: InkToolbarBallpointPenButton::new
});
ferro_impl_classes!(InkToolbarBallpointPenButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarBallpointPenButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::BallpointPen);
        strings::bind_content(this, "InkToolbarBallpointPenButtonName");
    }
}
impl InkToolbarPenButtonImpl for InkToolbarBallpointPenButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        Ok(InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width, width),
            ..InkDrawingAttributes::default()
        })
    }
}
impl InkToolbarBallpointPenButton {
    fn static_constructor() {
        // The stroke widths of the style of WinUI's `InkToolbarBallpointPenButton` (`InkToolBar.xaml`), as
        // defaults, so the button has them without the WinUI theme too.
        override_stroke_widths::<InkToolbarBallpointPenButton>(1.0, 24.0, 2.0);
    }
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarPencilButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarPencilButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarPencilButton {
    new: InkToolbarPencilButton::new
});
ferro_impl_classes!(InkToolbarPencilButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarPencilButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::Pencil);
        strings::bind_content(this, "InkToolbarPencilButtonName");
    }
}
impl InkToolbarPenButtonImpl for InkToolbarPencilButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        Ok(InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width, width),
            ..InkDrawingAttributes::create_for_pencil()
        })
    }
}
impl InkToolbarPencilButton {
    fn static_constructor() {
        // The stroke widths of the style of WinUI's `InkToolbarPencilButton` (`InkToolBar.xaml`), as
        // defaults, so the button has them without the WinUI theme too.
        override_stroke_widths::<InkToolbarPencilButton>(1.0, 24.0, 4.0);
    }
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarHighlighterButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarHighlighterButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarHighlighterButton {
    new: InkToolbarHighlighterButton::new
});
ferro_impl_classes!(InkToolbarHighlighterButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarHighlighterButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::Highlighter);
        strings::bind_content(this, "InkToolbarHighlighterButtonName");
        this.set_palette(default_palette(true));
        // As WinUI (`InkToolBarHighlighterButton.h`): a local value, above the style's 36.
        this.set_selected_stroke_width(16.0);
    }
}
impl InkToolbarPenButtonImpl for InkToolbarHighlighterButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        let width = this.determine_stroke_width();
        Ok(InkDrawingAttributes {
            color: this.selected_color()?,
            size: Size::new(width / 3.0, width),
            pen_tip: PenTipShape::Rectangle,
            draw_as_highlighter: true,
            ..InkDrawingAttributes::default()
        })
    }
}
impl InkToolbarHighlighterButton {
    fn static_constructor() {
        // The style of WinUI's highlighter sets 12, 64 and 36; its constructor then sets 16
        // as a local value, which wins (see `constructed`).
        override_stroke_widths::<InkToolbarHighlighterButton>(12.0, 64.0, 16.0);
    }
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Custom pen construction uses a real virtual dispatch, so subclasses can define the nib.
#[repr(C)]
pub struct InkToolbarCustomPen {
    base: FerroObject,
}
ferro_class! { InkToolbarCustomPen: FerroObject, virtuals InkToolbarCustomPenImpl: FerroObjectImpl {
    fn create_ink_drawing_attributes_core(this, brush: Option<Rc<dyn IBrush>>, stroke_width: f64) -> Result<InkDrawingAttributes,InkError>;
} }
ferro_class_info!(InkToolbarCustomPen {
    new: InkToolbarCustomPen::new
});
ferro_impl_classes!(InkToolbarCustomPen: FerroObjectImpl);
impl InkToolbarCustomPenImpl for InkToolbarCustomPen {
    fn create_ink_drawing_attributes_core(
        _this: &Self,
        brush: Option<Rc<dyn IBrush>>,
        stroke_width: f64,
    ) -> Result<InkDrawingAttributes, InkError> {
        let color = brush
            .as_ref()
            .and_then(|brush| brush.as_solid_color_brush())
            .map(|brush| brush.color())
            .unwrap_or_else(|| InkDrawingAttributes::default().color);
        let attrs = InkDrawingAttributes {
            color,
            size: Size::new(stroke_width, stroke_width),
            ..InkDrawingAttributes::default()
        };
        attrs.validate()?;
        Ok(attrs)
    }
}
impl InkToolbarCustomPen {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn create_ink_drawing_attributes(
        &self,
        brush: Option<Rc<dyn IBrush>>,
        stroke_width: f64,
    ) -> Result<InkDrawingAttributes, InkError> {
        self.create_ink_drawing_attributes_core(brush, stroke_width)
    }
}
#[repr(C)]
pub struct InkToolbarCustomPenButton {
    base: InkToolbarPenButton,
}
ferro_class!(InkToolbarCustomPenButton:InkToolbarPenButton);
ferro_class_info!(InkToolbarCustomPenButton {
    new: InkToolbarCustomPenButton::new
});
ferro_impl_classes!(InkToolbarCustomPenButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarCustomPenButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::CustomPen);
    }
}
impl InkToolbarPenButtonImpl for InkToolbarCustomPenButton {
    fn create_ink_drawing_attributes(this: &Self) -> Result<InkDrawingAttributes, InkError> {
        match this.custom_pen() {
            Some(pen) => pen.create_ink_drawing_attributes(
                this.selected_brush(),
                this.determine_stroke_width(),
            ),
            None => {
                let width = this.determine_stroke_width();
                Ok(InkDrawingAttributes {
                    color: this.selected_color()?,
                    size: Size::new(width, width),
                    ..InkDrawingAttributes::default()
                })
            }
        }
    }
}
ferro_properties! {impl InkToolbarCustomPenButton{
    pub fn custom_pen_property()->StyledProperty<Option<Ref<InkToolbarCustomPen>>>{FerroProperty::register::<InkToolbarCustomPenButton,_>("CustomPen",None)}
    pub fn configuration_content_property()->StyledProperty<Option<Ref<Control>>>{FerroProperty::register::<InkToolbarCustomPenButton,_>("ConfigurationContent",None)}
}}
impl InkToolbarCustomPenButton {
    fn static_constructor() {
        // The stroke widths of the style of WinUI's `InkToolbarCustomPenButton` (`InkToolBar.xaml`), as
        // defaults, so the button has them without the WinUI theme too.
        override_stroke_widths::<InkToolbarCustomPenButton>(1.0, 10.0, 5.0);
    }
    pub fn construct() -> Self {
        Self {
            base: InkToolbarPenButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn custom_pen(&self) -> Option<Ref<InkToolbarCustomPen>> {
        self.get_value(Self::custom_pen_property())
    }
    pub fn set_custom_pen(&self, value: Option<Ref<InkToolbarCustomPen>>) {
        self.set_value(Self::custom_pen_property(), value);
    }
    pub fn configuration_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::configuration_content_property())
    }
    pub fn set_configuration_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::configuration_content_property(), value);
    }
}

#[repr(C)]
pub struct InkToolbarEraserButton {
    base: InkToolbarToolButton,
}
ferro_class!(InkToolbarEraserButton:InkToolbarToolButton);
ferro_class_info!(InkToolbarEraserButton {
    new: InkToolbarEraserButton::new
});
ferro_impl_classes!(InkToolbarEraserButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarEraserButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::Eraser);
        strings::bind_content(this, "InkToolbarEraserButtonName");
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_clear_all_visible_property().as_property() {
            this.set_is_extension_glyph_shown(this.is_clear_all_visible());
        }
    }
}
impl InkToolbarEraserButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToolButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarCustomToolButton {
    base: InkToolbarToolButton,
}
ferro_class!(InkToolbarCustomToolButton:InkToolbarToolButton);
ferro_class_info!(InkToolbarCustomToolButton {
    new: InkToolbarCustomToolButton::new
});
ferro_impl_classes!(InkToolbarCustomToolButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarCustomToolButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_tool_kind(InkToolbarTool::CustomTool);
    }
}
impl InkToolbarCustomToolButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToolButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
fn override_stroke_widths<T: ferroui_base::ObjectType>(minimum: f64, maximum: f64, selected: f64) {
    InkToolbarPenButton::min_stroke_width_property().override_default_value::<T>(minimum);
    InkToolbarPenButton::max_stroke_width_property().override_default_value::<T>(maximum);
    InkToolbarPenButton::selected_stroke_width_property().override_default_value::<T>(selected);
}

/// Whether a change of `property` of a pen button can change the drawing attributes it
/// creates: WinUI listens to `SelectedBrush` and `SelectedStrokeWidth` alone; the port also
/// names what those two are computed from.
pub(crate) fn affects_pen_attributes(property: &FerroProperty) -> bool {
    property == InkToolbarPenButton::selected_brush_property().as_property()
        || property == InkToolbarPenButton::selected_brush_index_property().as_property()
        || property == InkToolbarPenButton::selected_stroke_width_property().as_property()
        || property == InkToolbarPenButton::min_stroke_width_property().as_property()
        || property == InkToolbarPenButton::max_stroke_width_property().as_property()
        || property == InkToolbarPenButton::palette_property().as_property()
        || property == InkToolbarCustomPenButton::custom_pen_property().as_property()
}

ferro_properties! {impl InkToolbarEraserButton{pub fn is_clear_all_visible_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarEraserButton,_>("IsClearAllVisible",true)}}}
impl InkToolbarEraserButton {
    pub fn is_clear_all_visible(&self) -> bool {
        self.get_value(Self::is_clear_all_visible_property())
    }
    pub fn set_is_clear_all_visible(&self, value: bool) {
        self.set_value(Self::is_clear_all_visible_property(), value);
    }
}
ferro_properties! {impl InkToolbarCustomToolButton{pub fn configuration_content_property()->StyledProperty<Option<Ref<Control>>>{FerroProperty::register::<InkToolbarCustomToolButton,_>("ConfigurationContent",None)}}}
impl InkToolbarCustomToolButton {
    pub fn configuration_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::configuration_content_property())
    }
    pub fn set_configuration_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::configuration_content_property(), value);
    }
}

#[repr(C)]
pub struct InkToolbarToggleButton {
    base: CheckBox,
    toggle_kind: Cell<InkToolbarToggle>,
}
ferro_class!(InkToolbarToggleButton:CheckBox);
ferro_class_info!(InkToolbarToggleButton{new:InkToolbarToggleButton::new,markup:{properties:[ToggleKind:InkToolbarToggle{get:InkToolbarToggleButton::toggle_kind}],}});
ferro_impl_classes!(InkToolbarToggleButton:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl StyledElementImpl for InkToolbarToggleButton {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        toolbar_style_key(this, CheckBox::TYPE)
    }
}
impl InkToolbarToggleButton {
    pub fn construct() -> Self {
        Self {
            base: CheckBox::construct(),
            toggle_kind: Cell::new(InkToolbarToggle::Custom),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn toggle_kind(&self) -> InkToolbarToggle {
        self.toggle_kind.get()
    }
}
#[repr(C)]
pub struct InkToolbarCustomToggleButton {
    base: InkToolbarToggleButton,
}
ferro_class!(InkToolbarCustomToggleButton:InkToolbarToggleButton);
ferro_class_info!(InkToolbarCustomToggleButton {
    new: InkToolbarCustomToggleButton::new
});
ferro_impl_classes!(InkToolbarCustomToggleButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarCustomToggleButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.toggle_kind.set(InkToolbarToggle::Custom);
    }
}
impl InkToolbarCustomToggleButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToggleButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarRulerButton {
    base: InkToolbarToggleButton,
}
ferro_class!(InkToolbarRulerButton:InkToolbarToggleButton);
ferro_class_info!(InkToolbarRulerButton {
    new: InkToolbarRulerButton::new
});
ferro_impl_classes!(InkToolbarRulerButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl FerroObjectImpl for InkToolbarRulerButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.toggle_kind.set(InkToolbarToggle::Ruler);
        strings::bind_content(this, "InkToolbarStencilRulerName");
    }
}
impl InkToolbarRulerButton {
    pub fn construct() -> Self {
        Self {
            base: InkToolbarToggleButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
#[repr(C)]
pub struct InkToolbarMenuButton {
    base: ToggleButton,
}
ferro_class!(InkToolbarMenuButton:ToggleButton);
ferro_class_info!(InkToolbarMenuButton{new:InkToolbarMenuButton::new,markup:{properties:[MenuKind:InkToolbarMenuKind{get:InkToolbarMenuButton::menu_kind}],}});
ferro_impl_classes!(InkToolbarMenuButton: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl, ToggleButtonImpl);
impl ControlImpl for InkToolbarMenuButton {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::InkToolbarMenuButtonAutomationPeer::new(this).upcast()
    }
}

impl StyledElementImpl for InkToolbarMenuButton {
    fn style_key_override(this: &Self) -> &'static TypeInfo {
        toolbar_style_key(this, ToggleButton::TYPE)
    }
}
ferro_properties! {impl InkToolbarMenuButton{pub fn is_extension_glyph_shown_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarMenuButton,_>("IsExtensionGlyphShown",true)}}}
impl InkToolbarMenuButton {
    pub fn construct() -> Self {
        Self {
            base: ToggleButton::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn menu_kind(&self) -> InkToolbarMenuKind {
        InkToolbarMenuKind::Stencil
    }
    pub fn is_extension_glyph_shown(&self) -> bool {
        self.get_value(Self::is_extension_glyph_shown_property())
    }
    pub fn set_is_extension_glyph_shown(&self, value: bool) {
        self.set_value(Self::is_extension_glyph_shown_property(), value);
    }
}

#[repr(C)]
pub struct InkToolbarStencilButton {
    base: InkToolbarMenuButton,
    presenter: RefCell<Option<Rc<super::InkPresenter>>>,
    ruler: RefCell<Option<Rc<super::InkPresenterRuler>>>,
    protractor: RefCell<Option<Rc<super::InkPresenterProtractor>>>,
    menu_items: RefCell<Vec<(InkToolbarStencilKind, Ref<InkToolbarFlyoutItem>)>>,
    syncing_menu: Cell<bool>,
    permit_flyout: Cell<bool>,
}
ferro_class!(InkToolbarStencilButton:InkToolbarMenuButton);
ferro_class_info!(InkToolbarStencilButton {
    new: InkToolbarStencilButton::new
});
ferro_impl_classes!(InkToolbarStencilButton:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl InputElementImpl for InkToolbarStencilButton {
    fn on_right_tapped(this: &Self, event: &TappedEventArgs) {
        Self::parent_on_right_tapped(this, event);
        if this.is_effectively_enabled() {
            this.activate_secondary();
            event.set_handled(true);
        }
    }
    fn on_holding(this: &Self, event: &HoldingRoutedEventArgs) {
        Self::parent_on_holding(this, event);
        if this.is_effectively_enabled() && event.holding_state() == HoldingState::Started {
            this.activate_secondary();
            event.set_handled(true);
        }
    }
}
impl ToggleButtonImpl for InkToolbarStencilButton {
    fn toggle(this: &Self) {
        // Two choices retain the active stencil when opening configuration. A single choice
        // behaves as a normal toggle, just as the source's one-stencil mode.
        let checked = if this.is_ruler_item_visible() && this.is_protractor_item_visible() {
            true
        } else {
            this.is_checked() != Some(true)
        };
        this.set_current_value(ToggleButton::is_checked_property(), Some(checked));
    }
}
impl ButtonImpl for InkToolbarStencilButton {
    fn on_click(this: &Self) {
        let previous = this.permit_flyout.replace(this.is_checked() == Some(true));
        Self::parent_on_click(this);
        this.permit_flyout.set(previous);
    }
    fn open_flyout(this: &Self) {
        if this.is_ruler_item_visible()
            && this.is_protractor_item_visible()
            && this.permit_flyout.get()
            && !toolbar_is_inking(this)
        {
            Self::parent_open_flyout(this);
        }
    }
}
impl FerroObjectImpl for InkToolbarStencilButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        strings::bind_content(this, "InkToolbarStencilButtonName");
        this.create_stencil_flyout();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::selected_stencil_property().as_property()
            && !this.syncing_menu.get()
        {
            // As WinUI (`OnSelectedStencilChanged`): the item of the newly selected stencil is
            // shown, rather than the selection being forced back to the one visible item.
            match this.selected_stencil() {
                InkToolbarStencilKind::Ruler => this.set_is_ruler_item_visible(true),
                InkToolbarStencilKind::Protractor => this.set_is_protractor_item_visible(true),
            }
        }
        if property == Self::selected_stencil_property().as_property()
            || property == ToggleButton::is_checked_property().as_property()
            || property == Self::is_ruler_item_visible_property().as_property()
            || property == Self::is_protractor_item_visible_property().as_property()
        {
            this.synchronize_stencil_menu();
        }
    }
}
ferro_properties! {impl InkToolbarStencilButton{
    pub fn ruler_property()->DirectProperty<InkToolbarStencilButton,Option<Rc<super::InkPresenterRuler>>>{FerroProperty::register_direct::<InkToolbarStencilButton,_>("Ruler",InkToolbarStencilButton::ruler,None,None)}
    pub fn protractor_property()->DirectProperty<InkToolbarStencilButton,Option<Rc<super::InkPresenterProtractor>>>{FerroProperty::register_direct::<InkToolbarStencilButton,_>("Protractor",InkToolbarStencilButton::protractor,None,None)}
    pub fn selected_stencil_property()->StyledProperty<InkToolbarStencilKind>{FerroProperty::register::<InkToolbarStencilButton,_>("SelectedStencil",InkToolbarStencilKind::Ruler)}
    pub fn is_ruler_item_visible_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarStencilButton,_>("IsRulerItemVisible",true)}
    pub fn is_protractor_item_visible_property()->StyledProperty<bool>{FerroProperty::register::<InkToolbarStencilButton,_>("IsProtractorItemVisible",true)}
}}
impl InkToolbarStencilButton {
    fn static_constructor() {
        InputElement::is_holding_enabled_property()
            .override_default_value::<InkToolbarStencilButton>(true);
    }
    fn activate_secondary(&self) {
        if self.is_ruler_item_visible() && self.is_protractor_item_visible() {
            self.set_current_value(ToggleButton::is_checked_property(), Some(true));
            self.open_flyout();
        } else {
            self.on_click();
        }
    }
    pub fn construct() -> Self {
        Self {
            base: InkToolbarMenuButton::construct(),
            presenter: RefCell::new(None),
            ruler: RefCell::new(None),
            protractor: RefCell::new(None),
            menu_items: RefCell::new(Vec::new()),
            syncing_menu: Cell::new(false),
            permit_flyout: Cell::new(true),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn create_stencil_flyout(&self) {
        let panel = StackPanel::new();
        let mut items = Vec::new();
        for (kind, label) in [
            (InkToolbarStencilKind::Ruler, "InkToolbarStencilRulerName"),
            (
                InkToolbarStencilKind::Protractor,
                "InkToolbarStencilProtractorName",
            ),
        ] {
            let item = InkToolbarFlyoutItem::new();
            item.set_kind(InkToolbarFlyoutItemKind::RadioCheck);
            item.set_name(Some(if kind == InkToolbarStencilKind::Ruler {
                "StencilRuler".to_owned()
            } else {
                "StencilProtractor".to_owned()
            }));
            strings::bind_content(&item, label);
            let weak = self.to_ref().downgrade();
            item.checked(move |_| {
                if let Some(button) = weak.upgrade() {
                    if button.syncing_menu.replace(true) {
                        return;
                    }
                    button.set_selected_stencil(kind);
                    button.set_is_checked(Some(true));
                    button.syncing_menu.set(false);
                    button.synchronize_stencil_menu();
                }
            });
            let weak = self.to_ref().downgrade();
            item.unchecked(move |_| {
                if let Some(button) = weak.upgrade() {
                    // An item unchecked because the other item of the radio group was checked:
                    // the checked item selects its stencil (WinUI selects through the click of
                    // an item, after which the group is unchecked).
                    let other_checked = button
                        .menu_items
                        .borrow()
                        .iter()
                        .any(|(other, item)| *other != kind && item.is_checked() == Some(true));
                    if other_checked {
                        return;
                    }
                    if button.syncing_menu.replace(true) {
                        return;
                    }
                    if button.selected_stencil() == kind {
                        button.set_is_checked(Some(false));
                    }
                    button.syncing_menu.set(false);
                    button.synchronize_stencil_menu();
                }
            });
            let weak = self.to_ref().downgrade();
            item.add_handler(Button::click_event(), move |_, _| {
                if let Some(button) = weak.upgrade() {
                    if let Some(flyout) = button.flyout() {
                        flyout.hide();
                    }
                }
            });
            panel.children().add(&item);
            items.push((kind, item));
        }
        *self.menu_items.borrow_mut() = items;
        let flyout = toolbar_flyout(self);
        flyout.set_content(Some(Rc::new(panel.upcast::<Control>())));
        self.set_flyout(flyout);
        self.synchronize_stencil_menu();
    }
    fn synchronize_stencil_menu(&self) {
        if self.syncing_menu.replace(true) {
            return;
        }
        let items = self.menu_items.borrow().clone();
        let ruler = self.is_ruler_item_visible();
        let protractor = self.is_protractor_item_visible();
        if ruler != protractor {
            self.set_current_value(
                Self::selected_stencil_property(),
                if ruler {
                    InkToolbarStencilKind::Ruler
                } else {
                    InkToolbarStencilKind::Protractor
                },
            );
        }
        self.set_current_value(
            InkToolbarMenuButton::is_extension_glyph_shown_property(),
            ruler && protractor,
        );
        let size = i32::from(ruler) + i32::from(protractor);
        let mut position = 0;
        for (kind, item) in items {
            let visible = if kind == InkToolbarStencilKind::Ruler {
                ruler
            } else {
                protractor
            };
            item.set_is_visible(visible);
            item.set_is_checked(Some(
                self.selected_stencil() == kind && self.is_checked() == Some(true),
            ));
            if visible {
                position += 1;
            }
            AutomationProperties::set_position_in_set(&item, if visible { position } else { 0 });
            AutomationProperties::set_size_of_set(&item, size);
        }
        self.syncing_menu.set(false);
    }
    pub(crate) fn set_presenter(&self, value: Option<Rc<super::InkPresenter>>) {
        let ruler = value.as_ref().map(|p| p.ruler());
        let protractor = value.as_ref().map(|p| p.protractor());
        *self.presenter.borrow_mut() = value;
        self.set_and_raise(Self::ruler_property(), &self.ruler, ruler);
        self.set_and_raise(Self::protractor_property(), &self.protractor, protractor);
    }
    pub fn ruler(&self) -> Option<Rc<super::InkPresenterRuler>> {
        self.ruler.borrow().clone()
    }
    pub fn protractor(&self) -> Option<Rc<super::InkPresenterProtractor>> {
        self.protractor.borrow().clone()
    }
    pub fn selected_stencil(&self) -> InkToolbarStencilKind {
        self.get_value(Self::selected_stencil_property())
    }
    pub fn set_selected_stencil(&self, value: InkToolbarStencilKind) {
        self.set_value(Self::selected_stencil_property(), value);
    }
    pub fn is_ruler_item_visible(&self) -> bool {
        self.get_value(Self::is_ruler_item_visible_property())
    }
    pub fn set_is_ruler_item_visible(&self, value: bool) {
        self.set_value(Self::is_ruler_item_visible_property(), value);
    }
    pub fn is_protractor_item_visible(&self) -> bool {
        self.get_value(Self::is_protractor_item_visible_property())
    }
    pub fn set_is_protractor_item_visible(&self, value: bool) {
        self.set_value(Self::is_protractor_item_visible_property(), value);
    }
}

/// A flyout of a toolbar button. As WinUI (`InkToolbarFlyoutStyle`, applied by the tool and menu
/// buttons), its presenter takes the control theme `InkToolbarFlyoutStyle` of the button's
/// resources when it opens, if the styles of the toolbar define one.
pub(crate) fn toolbar_flyout(owner: &Control) -> Ref<Flyout> {
    let flyout = Flyout::new();
    let weak_owner = owner.to_ref().downgrade();
    let weak_flyout = flyout.downgrade();
    let _ = flyout.opening(move |_| {
        let (Some(owner), Some(flyout)) = (weak_owner.upgrade(), weak_flyout.upgrade()) else {
            return;
        };
        let theme = owner
            .try_find_resource(&ResourceKey::from("InkToolbarFlyoutStyle"), None)
            .flatten();
        if let Some(theme) = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(&theme) {
            flyout.set_flyout_presenter_theme(theme);
        }
    });
    flyout
}

pub(crate) fn toolbar_is_inking(control: &Control) -> bool {
    let mut parent = control.parent();
    while let Some(element) = parent {
        if let Some(toolbar) = element.cast::<InkToolbar>() {
            return toolbar
                .effective_ink_presenter()
                .is_some_and(|presenter| presenter.is_inking());
        }
        parent = element.parent();
    }
    false
}

/// Use a dedicated theme when one is installed, retaining the framework's native control
/// theme otherwise. Resource changes invalidate the implicit theme through StyledElement.
pub(crate) fn toolbar_style_key(
    this: &StyledElement,
    fallback: &'static TypeInfo,
) -> &'static TypeInfo {
    let actual = this.get_type();
    if this.theme().is_some() {
        return actual;
    }
    let mut candidate = Some(actual);
    while let Some(kind) = candidate {
        if kind == fallback {
            break;
        }
        if this
            .try_find_resource(&ResourceKey::Type(kind), None)
            .flatten()
            .is_some_and(|resource| resource.downcast_ref::<Ref<ControlTheme>>().is_some())
        {
            return kind;
        }
        candidate = kind.base_type();
    }
    fallback
}
