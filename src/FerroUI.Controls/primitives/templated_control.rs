use super::TemplateAppliedEventArgs;
use crate::documents::TextElement;
use crate::templates::IControlTemplate;
use crate::{Border, Control, ControlImpl, Decorator};
use ferroui_base::controls::ResourcesChangedEventArgs;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::media::{
    BackgroundSizing, FontFamily, FontFeatureCollection, FontStretch, FontStyle, FontWeight, IBrush,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AttachedProperty,
    CornerRadius, FerroObject, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs,
    Nullable, Ref, StyledElement, StyledElementImpl, StyledElementImplExt, StyledProperty, Thickness, Visual,
    VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A lookless control whose visual appearance is defined by its `Template`.
#[repr(C)]
pub struct TemplatedControl {
    base: Control,
    applied_template: RefCell<Option<Rc<dyn IControlTemplate>>>,
}

ferro_class! {
    TemplatedControl: Control, virtuals TemplatedControlImpl: ControlImpl {
        /// Called when the control's template is applied.
        ///
        /// In simple terms, this means the method is called just before the
        /// control is displayed.
        fn on_apply_template(this, e: &TemplateAppliedEventArgs);
        /// Called when the `Template` property changes.
        fn on_template_changed(this, e: &FerroPropertyChangedEventArgs<'_>);
    }
}
ferroui_base::ferro_class_info!(TemplatedControl { new: TemplatedControl::new });

ferro_impl_classes!(TemplatedControl: VisualImpl, InteractiveImpl, InputElementImpl);

impl FerroObjectImpl for TemplatedControl {}

impl StyledElementImpl for TemplatedControl {
    fn notify_child_resources_changed(this: &Self, e: ResourcesChangedEventArgs) {
        if let Some(children) = this.visual_children_snapshot() {
            for child in children.iter() {
                child.notify_resources_changed(e, true);
            }
        }

        Self::parent_notify_child_resources_changed(this, e);
    }

    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(child) = this.visual_children_snapshot().and_then(|children| children.first().cloned()) {
            child.notify_attached_to_logical_tree(e);
        }

        Self::parent_on_attached_to_logical_tree(this, e);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        if let Some(child) = this.visual_children_snapshot().and_then(|children| children.first().cloned()) {
            child.notify_detached_from_logical_tree(e);
        }

        Self::parent_on_detached_from_logical_tree(this, e);
    }

    fn on_control_theme_changed(this: &Self) {
        Self::parent_on_control_theme_changed(this);

        if let Some(children) = this.visual_children_snapshot() {
            let this_ref: Ref<FerroObject> = this.to_ref().upcast();
            for child in children.iter() {
                if child.templated_parent().as_ref() == Some(&this_ref) {
                    child.on_templated_parent_control_theme_changed();
                }
            }
        }
    }
}

impl LayoutableImpl for TemplatedControl {
    fn apply_template(this: &Self) {
        let template = this.template();

        // Apply the template if it is not the same as the template already
        // applied - except for in the case that the template is null and
        // we're not attached to the logical tree. In that case, the template
        // has probably been cleared because the style setting the template
        // has been detached, so we want to wait until it's re-attached to the
        // logical tree as if it's re-attached to the same tree the template
        // will be the same and we don't need to do anything.
        let applied_template = this.applied_template.borrow().clone();
        if applied_template != template && (template.is_some() || this.is_attached_to_logical_tree()) {
            if this.visual_children_count() > 0 {
                for child in this.get_template_descendants() {
                    child.set_templated_parent(None);
                    child.set_parent(None);
                }

                this.visual_children().clear();
            }

            if let Some(template) = &template {
                let this_ref = this.to_ref();
                if let Some(template_result) = template.build(&this_ref) {
                    let (child, name_scope) = template_result.deconstruct();
                    Self::apply_templated_parent(&child, Some(&this_ref.clone().upcast()));
                    child.set_parent(this_ref);
                    this.visual_children().add(child.upcast());

                    let e = TemplateAppliedEventArgs::new(name_scope);
                    this.on_apply_template(&e);
                    this.raise_event(&e);
                }
            }

            *this.applied_template.borrow_mut() = template;
        }
    }
}

impl ControlImpl for TemplatedControl {
    fn get_template_focus_target(this: &Self) -> Option<Ref<Control>> {
        for child in this.get_template_descendants() {
            if let Some(control) = child.cast::<Control>() {
                if Self::get_is_template_focus_target(&control) {
                    return Some(control);
                }
            }
        }

        Some(this.to_ref().upcast())
    }
}

impl TemplatedControlImpl for TemplatedControl {
    fn on_apply_template(_this: &Self, _e: &TemplateAppliedEventArgs) {}

    fn on_template_changed(this: &Self, _e: &FerroPropertyChangedEventArgs<'_>) {
        this.invalidate_measure();
    }
}

ferroui_base::ferro_properties! { impl TemplatedControl {
    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `BackgroundSizing` property.
        pub fn background_sizing_property() -> StyledProperty<BackgroundSizing> {
            Border::background_sizing_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `BorderBrush` property.
        pub fn border_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::border_brush_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `BorderThickness` property.
        pub fn border_thickness_property() -> StyledProperty<Thickness> {
            Border::border_thickness_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `CornerRadius` property.
        pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
            Border::corner_radius_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `FontFamily` property.
        pub fn font_family_property() -> StyledProperty<FontFamily> {
            TextElement::font_family_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `FontFeatures` property.
        pub fn font_features_property() -> StyledProperty<Option<FontFeatureCollection>> {
            TextElement::font_features_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `FontSize` property.
        pub fn font_size_property() -> StyledProperty<f64> {
            TextElement::font_size_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `FontStyle` property.
        pub fn font_style_property() -> StyledProperty<FontStyle> {
            TextElement::font_style_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `FontWeight` property.
        pub fn font_weight_property() -> StyledProperty<FontWeight> {
            TextElement::font_weight_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `FontStretch` property.
        pub fn font_stretch_property() -> StyledProperty<FontStretch> {
            TextElement::font_stretch_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `Foreground` property.
        pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextElement::foreground_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `LetterSpacing` property.
        pub fn letter_spacing_property() -> StyledProperty<f64> {
            TextElement::letter_spacing_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `Padding` property.
        pub fn padding_property() -> StyledProperty<Thickness> {
            Decorator::padding_property().add_owner::<TemplatedControl>()
        }
    );

    ferro_property!(
        /// Defines the `Template` property.
        pub fn template_property() -> StyledProperty<Option<Rc<dyn IControlTemplate>>> {
            FerroProperty::register::<TemplatedControl, _>("Template", None)
        }
    );

    ferro_property!(
        /// Defines the `IsTemplateFocusTarget` attached property.
        pub fn is_template_focus_target_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<TemplatedControl, Control, _>("IsTemplateFocusTarget", false)
        }
    );
} }

impl TemplatedControl {
    ferro_routed_event!(
        /// Defines the `TemplateApplied` routed event.
        pub fn template_applied_event() -> RoutedEvent<TemplateAppliedEventArgs> {
            RoutedEvent::register::<TemplatedControl, _>("TemplateApplied", RoutingStrategies::DIRECT)
        }
    );

    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<TemplatedControl>(true);
        Self::template_property().changed().add_class_handler::<TemplatedControl>(|x, e| x.on_template_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Control::construct(), applied_template: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the control's template is applied.
    pub fn template_applied(
        &self,
        handler: impl Fn(&Interactive, &TemplateAppliedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::template_applied_event(), handler)
    }

    /// The brush used to draw the control's background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// How the control's background is drawn relative to the control's
    /// border.
    pub fn background_sizing(&self) -> BackgroundSizing {
        self.get_value(Self::background_sizing_property())
    }

    pub fn set_background_sizing(&self, value: BackgroundSizing) {
        self.set_value(Self::background_sizing_property(), value)
    }

    /// The brush used to draw the control's border.
    pub fn border_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::border_brush_property())
    }

    pub fn set_border_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::border_brush_property(), value)
    }

    /// The thickness of the control's border.
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

    /// The font family used to draw the control's text.
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }

    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value)
    }

    /// The font features turned on/off.
    pub fn font_features(&self) -> Option<FontFeatureCollection> {
        self.get_value(Self::font_features_property())
    }

    pub fn set_font_features(&self, value: Option<FontFeatureCollection>) {
        self.set_value(Self::font_features_property(), value)
    }

    /// The size of the control's text in points.
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }

    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value)
    }

    /// The font style used to draw the control's text.
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }

    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value)
    }

    /// The font weight used to draw the control's text.
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }

    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value)
    }

    /// The font stretch used to draw the control's text.
    pub fn font_stretch(&self) -> FontStretch {
        self.get_value(Self::font_stretch_property())
    }

    pub fn set_font_stretch(&self, value: FontStretch) {
        self.set_value(Self::font_stretch_property(), value)
    }

    /// The brush used to draw the control's text and other foreground elements.
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }

    /// The letter spacing for the control's text content.
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
    }

    pub fn set_letter_spacing(&self, value: f64) {
        self.set_value(Self::letter_spacing_property(), value)
    }

    /// The padding placed between the border of the control and its content.
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }

    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value)
    }

    /// The template that defines the control's appearance.
    pub fn template(&self) -> Option<Rc<dyn IControlTemplate>> {
        self.get_value(Self::template_property())
    }

    pub fn set_template(&self, value: Option<Rc<dyn IControlTemplate>>) {
        self.set_value(Self::template_property(), value)
    }

    /// Gets the value of the `IsTemplateFocusTarget` attached property on a
    /// control.
    pub fn get_is_template_focus_target(control: &Control) -> bool {
        control.get_value(Self::is_template_focus_target_property())
    }

    /// Sets the value of the `IsTemplateFocusTarget` attached property on a
    /// control.
    ///
    /// When a control is navigated to using the keyboard, a focus adorner is
    /// shown - usually around the control itself. However if the templated
    /// control has a template child with this attached property set to true,
    /// the focus adorner is shown around that control instead.
    pub fn set_is_template_focus_target(control: &Control, value: bool) {
        control.set_value(Self::is_template_focus_target_property(), value)
    }

    /// Sets the templated parent of a control and of its logical descendants
    /// that do not have one yet.
    pub fn apply_templated_parent(control: &StyledElement, templated_parent: Option<&Ref<FerroObject>>) {
        control.set_templated_parent(Nullable(templated_parent.cloned()));

        let children = control.logical_children().snapshot();

        for child in children.iter() {
            if child.templated_parent().is_none() {
                Self::apply_templated_parent(child, templated_parent);
            }
        }
    }
}
