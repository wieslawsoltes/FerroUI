use crate::utils::debug_display::{append_optional_boxed_value, build_base_debug_display};
use super::as_content_presenter_host;
use crate::documents::TextElement;
use crate::metadata::PseudoClassesAttribute;
use crate::templates::{FuncDataTemplate, IDataTemplate};
use crate::{Border, ContentControl, Control, ControlImpl, Decorator, TextBlock};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutHelper, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use crate::utils::BorderRenderHelper;
use ferroui_base::media::{
    BackgroundSizing, BoxShadows, DrawingContext, FontFamily, FontStretch, FontStyle, FontWeight, IBrush,
    TextAlignment, TextTrimming, TextWrapping,
};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, CornerRadius, DirectProperty,
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref,
    Size, StyledElement, StyledElementImpl, StyledElementImplExt, StyledProperty, Thickness, Vector, Visual,
    VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Presents a single item of data inside a templated control.
#[repr(C)]
pub struct ContentPresenter {
    base: Control,
    child: RefCell<Option<Ref<Control>>>,
    created_child: Cell<bool>,
    recycling_data_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    /// The data context to use for the next child update instead of the one
    /// derived from the content, when set.
    override_data_context: RefCell<Option<Option<BoxedValue>>>,
    layout_thickness: Cell<Option<Thickness>>,
    scale: Cell<f64>,
    host: RefCell<Option<WeakRef<FerroObject>>>,
    border_renderer: RefCell<BorderRenderHelper>,
}

ferro_class!(ContentPresenter: Control);
ferroui_base::ferro_class_info!(ContentPresenter { new: ContentPresenter::new });
ferro_impl_classes!(ContentPresenter: InteractiveImpl, InputElementImpl, ControlImpl);

impl VisualImpl for ContentPresenter {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.border_renderer.borrow_mut().render(
            context,
            this.bounds().size(),
            this.layout_thickness(),
            this.corner_radius(),
            this.background_sizing(),
            this.background().as_ref(),
            this.border_brush().as_ref(),
            &this.box_shadow(),
        );
    }
}

impl FerroObjectImpl for ContentPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "Content" | "ContentTemplate" => this.content_changed(change),
            "TemplatedParent" => this.templated_parent_changed(change),
            "UseLayoutRounding" | "BorderThickness" => this.layout_thickness.set(None),
            _ => {}
        }
    }
}

impl StyledElementImpl for ContentPresenter {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);
        *this.recycling_data_template.borrow_mut() = None;
        this.created_child.set(false);
        this.invalidate_measure();
    }
}

impl LayoutableImpl for ContentPresenter {
    fn apply_template(this: &Self) {
        if !this.created_child.get() && this.is_attached_to_logical_tree() {
            this.update_child();
        }
    }

    fn measure_override(this: &Self, available_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::measure_child_with_border(
            child.as_deref().map(|c| -> &Layoutable { c }),
            available_size,
            this.padding(),
            this.border_thickness(),
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arrange_override_impl(final_size, Vector::default())
    }
}

impl ContentPresenter {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":empty"]);
}

ferroui_base::ferro_properties! { impl ContentPresenter {
    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `BackgroundSizing` property.
        pub fn background_sizing_property() -> StyledProperty<BackgroundSizing> {
            Border::background_sizing_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `BorderBrush` property.
        pub fn border_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::border_brush_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `BorderThickness` property.
        pub fn border_thickness_property() -> StyledProperty<Thickness> {
            Border::border_thickness_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `CornerRadius` property.
        pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
            Border::corner_radius_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `BoxShadow` property.
        pub fn box_shadow_property() -> StyledProperty<BoxShadows> {
            Border::box_shadow_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `Foreground` property.
        pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextElement::foreground_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `FontFamily` property.
        pub fn font_family_property() -> StyledProperty<FontFamily> {
            TextElement::font_family_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `FontSize` property.
        pub fn font_size_property() -> StyledProperty<f64> {
            TextElement::font_size_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `FontStyle` property.
        pub fn font_style_property() -> StyledProperty<FontStyle> {
            TextElement::font_style_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `FontWeight` property.
        pub fn font_weight_property() -> StyledProperty<FontWeight> {
            TextElement::font_weight_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `FontStretch` property.
        pub fn font_stretch_property() -> StyledProperty<FontStretch> {
            TextElement::font_stretch_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `TextAlignment` property.
        pub fn text_alignment_property() -> StyledProperty<TextAlignment> {
            TextBlock::text_alignment_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `TextWrapping` property.
        pub fn text_wrapping_property() -> StyledProperty<TextWrapping> {
            TextBlock::text_wrapping_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `TextTrimming` property.
        pub fn text_trimming_property() -> StyledProperty<Rc<dyn TextTrimming>> {
            TextBlock::text_trimming_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `LineHeight` property.
        pub fn line_height_property() -> StyledProperty<f64> {
            TextBlock::line_height_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `LetterSpacing` property.
        pub fn letter_spacing_property() -> StyledProperty<f64> {
            TextElement::letter_spacing_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `MaxLines` property.
        pub fn max_lines_property() -> StyledProperty<i32> {
            TextBlock::max_lines_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `Child` property.
        pub fn child_property() -> DirectProperty<ContentPresenter, Option<Ref<Control>>> {
            FerroProperty::register_direct::<ContentPresenter, _>("Child", |o| o.child(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            ContentControl::content_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            ContentControl::content_template_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `VerticalContentAlignment` property.
        pub fn vertical_content_alignment_property() -> StyledProperty<VerticalAlignment> {
            ContentControl::vertical_content_alignment_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `Padding` property.
        pub fn padding_property() -> StyledProperty<Thickness> {
            Decorator::padding_property().add_owner::<ContentPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `RecognizesAccessKey` property.
        pub fn recognizes_access_key_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContentPresenter, _>("RecognizesAccessKey", false)
        }
    );
} }

impl ContentPresenter {
    fn static_constructor() {
        Visual::affects_render::<ContentPresenter>(&[
            Self::background_property().as_property(),
            Self::background_sizing_property().as_property(),
            Self::border_brush_property().as_property(),
            Self::border_thickness_property().as_property(),
            Self::box_shadow_property().as_property(),
            Self::corner_radius_property().as_property(),
        ]);
        Layoutable::affects_arrange::<ContentPresenter>(&[
            Self::horizontal_content_alignment_property().as_property(),
            Self::vertical_content_alignment_property().as_property(),
        ]);
        Layoutable::affects_measure::<ContentPresenter>(&[
            Self::border_thickness_property().as_property(),
            Self::padding_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            child: RefCell::new(None),
            created_child: Cell::new(false),
            recycling_data_template: RefCell::new(None),
            override_data_context: RefCell::new(None),
            layout_thickness: Cell::new(None),
            scale: Cell::new(0.0),
            host: RefCell::new(None),
            border_renderer: RefCell::new(BorderRenderHelper::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A brush with which to paint the background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// How the background is drawn relative to the border.
    pub fn background_sizing(&self) -> BackgroundSizing {
        self.get_value(Self::background_sizing_property())
    }

    pub fn set_background_sizing(&self, value: BackgroundSizing) {
        self.set_value(Self::background_sizing_property(), value)
    }

    /// A brush with which to paint the border.
    pub fn border_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::border_brush_property())
    }

    pub fn set_border_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::border_brush_property(), value)
    }

    /// The thickness of the border.
    pub fn border_thickness(&self) -> Thickness {
        self.get_value(Self::border_thickness_property())
    }

    pub fn set_border_thickness(&self, value: Thickness) {
        self.set_value(Self::border_thickness_property(), value)
    }

    /// The radius of the border rounded corners.
    pub fn corner_radius(&self) -> CornerRadius {
        self.get_value(Self::corner_radius_property())
    }

    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::corner_radius_property(), value)
    }

    /// The box shadow effect parameters.
    pub fn box_shadow(&self) -> BoxShadows {
        self.get_value(Self::box_shadow_property())
    }

    pub fn set_box_shadow(&self, value: BoxShadows) {
        self.set_value(Self::box_shadow_property(), value)
    }

    /// A brush used to paint the text.
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }

    /// The font family.
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }

    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value)
    }

    /// The font size.
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }

    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value)
    }

    /// The font style.
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }

    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value)
    }

    /// The font weight.
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }

    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value)
    }

    /// The font stretch.
    pub fn font_stretch(&self) -> FontStretch {
        self.get_value(Self::font_stretch_property())
    }

    pub fn set_font_stretch(&self, value: FontStretch) {
        self.set_value(Self::font_stretch_property(), value)
    }

    /// The text alignment.
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value)
    }

    /// The text wrapping.
    pub fn text_wrapping(&self) -> TextWrapping {
        self.get_value(Self::text_wrapping_property())
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_value(Self::text_wrapping_property(), value)
    }

    /// The text trimming.
    pub fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        self.get_value(Self::text_trimming_property())
    }

    pub fn set_text_trimming(&self, value: Rc<dyn TextTrimming>) {
        self.set_value(Self::text_trimming_property(), value)
    }

    /// The line height.
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }

    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value)
    }

    /// The letter spacing.
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
    }

    pub fn set_letter_spacing(&self, value: f64) {
        self.set_value(Self::letter_spacing_property(), value)
    }

    /// The max lines.
    pub fn max_lines(&self) -> i32 {
        self.get_value(Self::max_lines_property())
    }

    pub fn set_max_lines(&self, value: i32) {
        self.set_value(Self::max_lines_property(), value)
    }

    /// The control displayed by the presenter.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.child.borrow().clone()
    }

    fn set_child(&self, value: Option<Ref<Control>>) {
        self.set_and_raise(Self::child_property(), &self.child, value);
    }

    /// The content to be displayed by the presenter.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// The data template used to display the content of the control.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value)
    }

    /// The horizontal alignment of the content within the border the
    /// control.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// The vertical alignment of the content within the border of the
    /// control.
    pub fn vertical_content_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_content_alignment_property())
    }

    pub fn set_vertical_content_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_content_alignment_property(), value)
    }

    /// The space between the border and the child control.
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }

    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value)
    }

    /// Determine if the presenter should use access text in its style.
    pub fn recognizes_access_key(&self) -> bool {
        self.get_value(Self::recognizes_access_key_property())
    }

    pub fn set_recognizes_access_key(&self, value: bool) {
        self.set_value(Self::recognizes_access_key_property(), value)
    }

    /// The host content control.
    pub fn host(&self) -> Option<Ref<FerroObject>> {
        self.host.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the content and the data context of the presenter at once: the
    /// child is created with `data_context` instead of the data context
    /// derived from the content.
    pub fn set_content_with_data_context(&self, content: Option<BoxedValue>, data_context: Option<BoxedValue>) {
        *self.override_data_context.borrow_mut() = Some(data_context.clone());

        // If Content didn't change, `update_child` wasn't called and the
        // override wasn't consumed. Apply the data context directly. The
        // guard also runs when setting the content unwinds.
        struct Finally<'a>(&'a ContentPresenter, Option<BoxedValue>);

        impl Drop for Finally<'_> {
            fn drop(&mut self) {
                let is_set = self.0.override_data_context.borrow().is_some();
                if is_set {
                    *self.0.override_data_context.borrow_mut() = None;
                    self.0.set_data_context(self.1.take());
                }
            }
        }

        let _finally = Finally(self, data_context);
        self.set_current_value(Self::content_property(), content);
    }

    /// Updates the `child` control based on the `content`.
    ///
    /// Usually the child control is created automatically when
    /// `apply_template` is called; however for this to happen, the control
    /// needs to be attached to a logical tree (if the control is not attached
    /// to the logical tree, it is reasonable to expect that the data
    /// templates needed for the child are not yet available). This method
    /// forces the child to be updated regardless.
    pub fn update_child(&self) {
        let content = self.content();
        self.update_child_with(content);
    }

    fn update_child_with(&self, content: Option<BoxedValue>) {
        let content_template = self.content_template();
        let old_child = self.child();
        let new_child = self.create_child(content.as_ref(), old_child.as_ref(), content_template.as_ref());

        // Remove the old child if we're not recycling it.
        if new_child != old_child {
            if let Some(old_child) = &old_child {
                self.visual_children().remove(&old_child.clone().upcast());
                self.with_effective_logical_children(|logical_children| {
                    logical_children.remove(&old_child.clone().upcast());
                });
                old_child.set_inheritance_parent(old_child.parent().map(Ref::upcast::<FerroObject>));
            }
        }

        // Consume the override immediately so any reentrant/cascading calls
        // to `update_child` don't incorrectly apply the stale override.
        let override_data_context = self.override_data_context.borrow_mut().take();

        // Set the data context: use the caller-provided override if set,
        // otherwise set to content when a template is present or content
        // isn't a control, or clear for template-less control content.
        let content_is_control = content.as_ref().is_some_and(|content| Control::from_boxed(content).is_some());
        if let Some(data_context) = override_data_context {
            self.set_data_context(data_context);
        } else if content_template.is_some() || !content_is_control {
            self.set_data_context(content);
        } else {
            self.clear_value(StyledElement::data_context_property());
        }

        // Update the child.
        match new_child {
            None => self.set_child(None),
            Some(new_child) => {
                if Some(&new_child) != old_child.as_ref() {
                    new_child.set_inheritance_parent(self.to_ref().upcast::<FerroObject>());
                    self.set_child(Some(new_child.clone()));

                    self.with_effective_logical_children(|logical_children| {
                        let logical: Ref<StyledElement> = new_child.clone().upcast();
                        if !logical_children.contains(&logical) {
                            logical_children.add(logical);
                        }
                    });

                    self.visual_children().add(new_child.upcast());
                }
            }
        }

        self.created_child.set(true);
    }

    /// Runs `f` with the logical children of the host, or with the
    /// presenter's own when it has no host.
    fn with_effective_logical_children(&self, f: impl FnOnce(&FerroList<Ref<StyledElement>>)) {
        let host = self.host();
        match host.as_ref().and_then(|host| as_content_presenter_host(host)) {
            Some(host) => f(host.logical_children()),
            None => f(self.logical_children()),
        }
    }

    /// The border thickness used for rendering: rounded to device pixels
    /// when layout rounding is in use.
    pub fn layout_thickness(&self) -> Thickness {
        self.verify_scale();

        match self.layout_thickness.get() {
            Some(thickness) => thickness,
            None => {
                let mut border_thickness = self.border_thickness();

                if self.use_layout_rounding() {
                    border_thickness = LayoutHelper::round_layout_thickness(border_thickness, self.scale.get());
                }

                self.layout_thickness.set(Some(border_thickness));
                border_thickness
            }
        }
    }

    fn verify_scale(&self) {
        let current_scale = LayoutHelper::get_layout_scale(self);
        if MathUtilities::are_close(current_scale, self.scale.get()) {
            return;
        }

        self.scale.set(current_scale);
        self.layout_thickness.set(None);
    }

    fn create_child(
        &self,
        content: Option<&BoxedValue>,
        old_child: Option<&Ref<Control>>,
        template: Option<&Rc<dyn IDataTemplate>>,
    ) -> Option<Ref<Control>> {
        let mut new_child = content.and_then(Control::from_boxed);

        // We want to allow creating the child from the template, if the
        // content is null. But it's important to not use data templates,
        // otherwise every content presenter without content set would get a
        // child.
        if (new_child.is_none() && (content.is_some() || template.is_some()))
            || (new_child.is_some() && template.is_some())
        {
            let data_template: Rc<dyn IDataTemplate> = match self.find_data_template(content, template) {
                Some(data_template) => data_template,
                None if self.recognizes_access_key() => FuncDataTemplate::access(),
                None => FuncDataTemplate::default_template(),
            };

            if let Some(rdt) = data_template.as_recycling_data_template() {
                let is_same = self.recycling_data_template.borrow().as_ref().is_some_and(|current| {
                    std::ptr::addr_eq(Rc::as_ptr(current), Rc::as_ptr(&data_template))
                });
                let to_recycle = if is_same { old_child.cloned() } else { None };
                new_child = rdt.build_with_existing(content, to_recycle);
                *self.recycling_data_template.borrow_mut() = Some(data_template.clone());
            } else {
                new_child = data_template.build(&content.cloned());
                *self.recycling_data_template.borrow_mut() = None;
            }
        } else {
            *self.recycling_data_template.borrow_mut() = None;
        }

        new_child
    }

    /// Arranges the child within `final_size`, with the origin of the
    /// content area displaced by `offset`.
    pub fn arrange_override_impl(&self, final_size: Size, offset: Vector) -> Size {
        let Some(child) = self.child() else { return final_size };

        let use_layout_rounding = self.use_layout_rounding();
        let scale = LayoutHelper::get_layout_scale(self);
        let mut padding = self.padding();
        let mut border_thickness = self.border_thickness();

        if use_layout_rounding {
            padding = LayoutHelper::round_layout_thickness(padding, scale);
            border_thickness = LayoutHelper::round_layout_thickness(border_thickness, scale);
        }

        padding = padding + border_thickness;
        let horizontal_content_alignment = self.horizontal_content_alignment();
        let vertical_content_alignment = self.vertical_content_alignment();
        let mut available_size = final_size;
        let mut size_for_child = available_size;
        let mut origin_x = offset.x;
        let mut origin_y = offset.y;
        let desired_size = self.desired_size();

        if horizontal_content_alignment != HorizontalAlignment::Stretch {
            size_for_child = size_for_child.with_width(size_for_child.width.min(desired_size.width));
        }

        if vertical_content_alignment != VerticalAlignment::Stretch {
            size_for_child = size_for_child.with_height(size_for_child.height.min(desired_size.height));
        }

        if use_layout_rounding {
            size_for_child = LayoutHelper::round_layout_size_up(size_for_child, scale);
            available_size = LayoutHelper::round_layout_size_up(available_size, scale);
        }

        match horizontal_content_alignment {
            HorizontalAlignment::Center => origin_x += (available_size.width - size_for_child.width) / 2.0,
            HorizontalAlignment::Right => origin_x += available_size.width - size_for_child.width,
            _ => {}
        }

        match vertical_content_alignment {
            VerticalAlignment::Center => origin_y += (available_size.height - size_for_child.height) / 2.0,
            VerticalAlignment::Bottom => origin_y += available_size.height - size_for_child.height,
            _ => {}
        }

        let mut origin = Point::new(origin_x, origin_y);

        if use_layout_rounding {
            origin = LayoutHelper::round_layout_point(origin, scale);
        }

        let bounds_for_child = Rect::from_position_size(origin, size_for_child).deflate_thickness(padding);

        child.arrange(bounds_for_child);

        final_size
    }

    /// Called when the `Content` or `ContentTemplate` property changes.
    fn content_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.created_child.set(false);

        if self.is_attached_to_logical_tree() {
            if e.property().name() == "Content" {
                self.update_child_with(e.get_new_value::<Option<BoxedValue>>());
            } else {
                self.update_child();
            }
        } else if let Some(child) = self.child() {
            self.visual_children().remove(&child.clone().upcast());
            self.with_effective_logical_children(|logical_children| {
                logical_children.remove(&child.clone().upcast());
            });
            child.set_inheritance_parent(child.parent().map(Ref::upcast::<FerroObject>));
            self.set_child(None);
            *self.recycling_data_template.borrow_mut() = None;
        }

        self.update_pseudo_classes();
        self.invalidate_measure();
    }

    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(":empty", self.content().is_none());
    }

    fn templated_parent_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<Option<Ref<FerroObject>>>();
        let registered = new_value.as_ref().is_some_and(|parent| {
            as_content_presenter_host(parent).is_some_and(|host| host.register_content_presenter(self))
        });
        *self.host.borrow_mut() = if registered { new_value.as_ref().map(Ref::downgrade) } else { None };
    }
}

impl ContentPresenter {
    /// Appends the text that describes the presenter in diagnostics: the base description,
    /// its host and, with the content, its content.
    pub(crate) fn build_debug_display(&self, builder: &mut String, include_content: bool) {
        build_base_debug_display(self, builder);

        let host: Option<BoxedValue> = self.host().map(|host| Rc::new(host) as BoxedValue);
        append_optional_boxed_value(builder, "Host", host.as_ref(), false);

        if include_content {
            append_optional_boxed_value(builder, "Content", self.content().as_ref(), true);
        }
    }
}
