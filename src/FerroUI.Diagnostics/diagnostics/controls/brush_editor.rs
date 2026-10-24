//! Port of `Diagnostics/Controls/BrushEditor.xaml.cs` (the class; its template is the style
//! of `BrushEditor.xaml`).

use ferroui_base::{FerroObjectImplExt, VisualImplExt};
use ferroui_base::input::InputElementImplExt;
use ferroui_controls::primitives::TemplatedControlImplExt;
use ferroui_base::input::{InputElementImpl, PointerPressedEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{Brushes, Color, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Typeface};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::metadata::MarkupType;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, StyledElementImpl, TypeInfo,
    VisualImpl,
};
use ferroui_controls::primitives::{FlyoutBase, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl};
use ferroui_controls::{Button, Control, ControlImpl, Flyout, ToolTip};
use ferroui_controls_color_picker::{AlphaComponentPosition, ColorChangedEventArgs, ColorView};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// An editor of a brush: shows the brush with the text of its color, opens a color view in a
/// flyout when pressed, and clears the brush with the clear button of its template
/// (`PART_ClearButton`).
#[repr(C)]
pub struct BrushEditor {
    base: TemplatedControl,
    clear_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    color_view: Ref<ColorView>,
    brush: RefCell<Option<Rc<dyn IBrush>>>,
}

ferro_class!(BrushEditor: TemplatedControl);
ferro_impl_classes!(BrushEditor: LayoutableImpl, InteractiveImpl, ControlImpl);
ferro_class_info!(BrushEditor {
    new: BrushEditor::new,
    markup: {
        attributes: [TemplatePart("PART_ClearButton", type(Ref<Button>))],
    },
});

ferro_properties! {
    impl BrushEditor {
        /// Defines the `Brush` property.
        pub fn brush_property() -> DirectProperty<BrushEditor, Option<Rc<dyn IBrush>>> {
            FerroProperty::register_direct::<BrushEditor, _>(
                "Brush",
                |o| o.brush(),
                Some(|o, v| o.set_brush(v)),
                None,
            )
        }
    }
}

impl FerroObjectImpl for BrushEditor {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(this.color_view.clone())));
        FlyoutBase::set_attached_flyout(this, flyout);

        let weak = this.to_ref().downgrade();
        // The handler lives as long as the color view (the subscription is never disposed, as in
        // the managed original).
        let _ = this.color_view.color_changed(move |e: &ColorChangedEventArgs| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            // Avoid unnecessary value setters by checking if color was actually changed.
            let brush = this.brush();
            let unchanged = brush
                .as_ref()
                .and_then(|brush| brush.as_solid_color_brush().map(|old_solid_brush| old_solid_brush.color()))
                .is_some_and(|old_color| old_color == e.new_color());
            if !unchanged {
                this.set_brush(Some(Rc::new(ImmutableSolidColorBrush::new(e.new_color())) as Rc<dyn IBrush>));
            }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        // The outline and the text of the swatch are colours of the theme variant.
        if change.property().name() == "ActualThemeVariant" {
            this.invalidate_visual();
        }

        if change.property() == Self::brush_property().as_property() {
            let brush = this.brush();
            if let Some(scb) = brush.as_ref().and_then(|brush| brush.as_solid_color_brush()) {
                this.color_view.set_color(scb.color());
            }
            let tip = brush.as_deref().map(brush_type_name).unwrap_or_else(|| "(null)".to_string());
            ToolTip::set_tip(this, Some(Rc::new(tip) as BoxedValue));
            this.invalidate_visual();
        }
    }
}

impl StyledElementImpl for BrushEditor {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        BrushEditor::TYPE
    }
}

impl TemplatedControlImpl for BrushEditor {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if let Some((button, token)) = this.clear_button.borrow_mut().take() {
            button.remove_handler(Button::click_event(), token);
        }

        if let Some(button) = e.name_scope().find_as::<Button>("PART_ClearButton") {
            let weak = this.to_ref().downgrade();
            // `clearHandler`: `(s, e) => Brush = default`.
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.set_brush(None);
                }
            });
            *this.clear_button.borrow_mut() = Some((button, token));
        }
    }
}

impl InputElementImpl for BrushEditor {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        FlyoutBase::show_attached_flyout(this);
    }
}

impl VisualImpl for BrushEditor {
    fn render(this: &Self, context: &mut DrawingContext) {
        Self::parent_render(this, context);

        // Deviation (DEVIATIONS.md, Developer tools): the swatch in the design of the tools: in
        // the coordinates of the editor (upstream fills `Bounds`, its place in the parent, which
        // moves the swatch when the editor has a margin), with rounded corners and the outline of
        // a control of the theme; a null brush is an empty swatch with "(null)" in the secondary
        // colour (upstream: black), and a brush that is not a solid colour names its type.
        let size = this.bounds().size();
        let rect = Rect::new(0.5, 0.5, (size.width - 1.0).max(0.0), (size.height - 1.0).max(0.0));
        // The resources of the variant of the editor (a lookup without one gets the default
        // dictionary, the dark one).
        let theme = this.actual_theme_variant();
        let resource = |key: &str| {
            this.try_find_resource(&ResourceKey::from(key), theme.as_ref())
                .and_then(|value| from_markup_value::<Rc<dyn IBrush>>(&value))
        };

        let brush = this.brush();
        if let Some(brush) = &brush {
            context.fill_rectangle(brush, rect, 4.0);
        }
        let stroke = resource("ControlStrongStrokeColorDefaultBrush")
            .and_then(|stroke| stroke.as_solid_color_brush().map(|solid| solid.color()))
            .unwrap_or(Color::from_argb(0x72, 0, 0, 0));
        let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(Some(Rc::new(ImmutableSolidColorBrush::new(stroke))), 1.0));
        context.draw_rectangle_outline(&pen, rect, 4.0);

        let (text, text_brush) = match &brush {
            Some(brush) => (
                brush.as_solid_color_brush().map(|solid| solid.color().to_string()).unwrap_or_else(|| brush_type_name(&**brush)),
                Self::get_text_brush(brush).unwrap_or_else(|| {
                    resource("TextFillColorPrimaryBrush").unwrap_or_else(|| Brushes::black() as Rc<dyn IBrush>)
                }),
            ),
            None => ("(null)".to_string(), resource("TextFillColorSecondaryBrush").unwrap_or_else(|| Brushes::gray())),
        };

        let ft = FormattedText::new(
            &text,
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            Typeface::default(),
            11.0,
            Some(text_brush),
        );

        context.draw_text(&ft, Point::new(size.width / 2.0 - ft.width() / 2.0, size.height / 2.0 - ft.height() / 2.0));
    }
}

impl BrushEditor {
    pub fn construct() -> Self {
        let color_view = ColorView::new();
        // Always match XAML.
        color_view.set_hex_input_alpha_position(AlphaComponentPosition::Leading);

        Self {
            base: TemplatedControl::construct(),
            clear_button: RefCell::new(None),
            color_view,
            brush: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.borrow().clone()
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_and_raise(Self::brush_property(), &self.brush, value);
    }

    /// Get Contrasted Text Color.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): `None` for a colour that is mostly transparent,
    /// whose text is drawn in the text colour of the theme: the swatch shows the background
    /// through it, and the white or black of its luminance did not show on it (the translucent
    /// brushes of the WinUI 3 theme are most of the brushes of the tools).
    fn get_text_brush(brush: &Rc<dyn IBrush>) -> Option<Rc<dyn IBrush>> {
        if let Some(solid) = brush.as_solid_color_brush() {
            let color = solid.color();
            if (color.a as f64) * brush.opacity() < 128.0 {
                return None;
            }
            let l = get_relative_luminance(color);

            return Some(if l < 0.5 { Brushes::white() } else { Brushes::black() });
        }
        Some(Brushes::white())
    }
}

/// `ColorHelper.GetRelativeLuminance` of the color picker: the relative (perceptual) luminance
/// of a color, 1 closer to white and 0 closer to black.
///
/// Deviation (DEVIATIONS.md, Developer tools): the color picker crate keeps `ColorHelper` internal
/// (`mod helpers` is private); this is its formula.
pub(crate) fn get_relative_luminance(color: Color) -> f64 {
    fn component(value: u8) -> f64 {
        if value <= 10 {
            value as f64 / 3294.0
        } else {
            (value as f64 / 269.0 + 0.0513).powf(2.4)
        }
    }

    0.2126 * component(color.r) + 0.7152 * component(color.g) + 0.0722 * component(color.b)
}

/// The name of the class of a brush (`Brush.GetType().Name`): the class of an object, else the
/// markup name of its type.
///
/// Deviation (DEVIATIONS.md, Developer tools): a brush that is neither an object nor a type with
/// markup metadata has no name to show in the port (no run-time type names); its contract name is
/// shown.
fn brush_type_name(brush: &dyn IBrush) -> String {
    if let Some(object) = brush.as_object() {
        return object.get_type().name().to_string();
    }
    let any: &dyn Any = brush.as_any();
    match MarkupType::find_by_handle(any.type_id()) {
        Some(markup) => markup.name.to_string(),
        None => "IBrush".to_string(),
    }
}
