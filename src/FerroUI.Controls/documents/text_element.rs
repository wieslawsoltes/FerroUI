use super::IInlineHost;
use crate::{Border, Control};
use crate::primitives::TemplatedControl;
use ferroui_base::controls::NameScope;
use ferroui_base::media::{
    subscribe_invalidated, Brushes, FontFamily, FontFeatureCollection, FontStretch, FontStyle,
    FontWeight, IBrush,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_property, AttachedProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, Ref,
    FerroProperty, FerroPropertyChangedEventArgs, StyledElement, StyledElementImpl, StyledProperty,
    StyledPropertyOptions,
};
use std::cell::RefCell;
use std::rc::Rc;


#[cfg(test)]
#[path = "text_element_namescope_tests.rs"]
mod text_element_namescope_tests;

/// `TextElement` is the base class for content in text based controls.
/// It also defines the inherited font and foreground attached properties
/// used by every control that displays text.
///
/// This class is abstract.
#[repr(C)]
pub struct TextElement {
    base: StyledElement,
    inline_host: RefCell<Option<Rc<dyn IInlineHost>>>,
    media_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

ferro_class! {
    TextElement: StyledElement, virtuals TextElementImpl: StyledElementImpl {
        /// Called when the host of the element changes.
        #[doc(hidden)]
        fn on_inline_host_changed(this, old_value: Option<&Rc<dyn IInlineHost>>, new_value: Option<&Rc<dyn IInlineHost>>);
    }
}

ferroui_base::ferro_impl_classes!(TextElement: StyledElementImpl);

impl FerroObjectImpl for TextElement {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.refresh_media_subscriptions();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if matches!(
            change.property().name(),
            "FontFeatures" | "Foreground" | "Background"
        ) {
            this.refresh_media_subscriptions();
        }

        if super::Typography::is_property(change.property()) {
            this.invalidate_format_change(change);
        }

        match change.property().name() {
            "Background" | "FontFamily" | "FontSize" | "FontStyle" | "FontWeight"
            | "FontStretch" | "Foreground" | "FontFeatures" | "LetterSpacing" | "Language" => {
                this.invalidate_format_change(change);
            }
            _ => {}
        }
    }
}

impl Drop for TextElement {
    fn drop(&mut self) {
        for subscription in self.media_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

impl TextElementImpl for TextElement {
    fn on_inline_host_changed(
        _this: &Self,
        _old_value: Option<&Rc<dyn IInlineHost>>,
        _new_value: Option<&Rc<dyn IInlineHost>>,
    ) {
    }
}

ferroui_base::ferro_properties! { impl TextElement {
    ferro_property!(
        /// The inherited text language, represented by FerroUI's culture value.
        /// Markup accepts a language tag, and an empty tag resolves to the
        /// preferred application language when text is shaped.
        pub fn language_property() -> AttachedProperty<CultureInfo> {
            FerroProperty::register_attached_with::<TextElement, Control, _>(
                "Language",
                StyledPropertyOptions::new(super::text_language::default_language())
                    .inherits(true)
                    .validate(|language: &CultureInfo| super::text_language::is_valid_language_tag(language.name())),
            )
        }
    );

    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<TextElement>()
        }
    );

    ferro_property!(
        /// Defines the `FontFamily` property.
        pub fn font_family_property() -> AttachedProperty<FontFamily> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "FontFamily",
                StyledPropertyOptions::new(FontFamily::default_family()).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `FontFeatures` property.
        pub fn font_features_property() -> AttachedProperty<Option<FontFeatureCollection>> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "FontFeatures",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `FontSize` property.
        pub fn font_size_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "FontSize",
                StyledPropertyOptions::new(12.0)
                    .inherits(true)
                    .validate(|font_size: &f64| *font_size > 0.0 && !font_size.is_nan() && !font_size.is_infinite()),
            )
        }
    );

    ferro_property!(
        /// Defines the `FontStyle` property.
        pub fn font_style_property() -> AttachedProperty<FontStyle> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "FontStyle",
                StyledPropertyOptions::new(FontStyle::default()).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `FontWeight` property.
        pub fn font_weight_property() -> AttachedProperty<FontWeight> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "FontWeight",
                StyledPropertyOptions::new(FontWeight::Normal).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `FontStretch` property.
        pub fn font_stretch_property() -> AttachedProperty<FontStretch> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "FontStretch",
                StyledPropertyOptions::new(FontStretch::Normal).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `Foreground` property.
        pub fn foreground_property() -> AttachedProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register_attached_with::<TextElement, TextElement, _>(
                "Foreground",
                StyledPropertyOptions::new(Some(Brushes::black() as Rc<dyn IBrush>)).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `LetterSpacing` property.
        pub fn letter_spacing_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<TextElement, Control, _>(
                "LetterSpacing",
                StyledPropertyOptions::new(0.0).inherits(true),
            )
        }
    );
} }

impl TextElement {
    /// Finds a registered object in this element's nearest logical name scope.
    ///
    /// The result may be a control or another text element. A missing name or
    /// name scope returns `None`. The nearest scope controls lookup, including
    /// `ChildNameScope`'s parent fallback after completion; no descendant tree
    /// search or additional ancestor-scope search is performed here. XAML
    /// registers names when it loads, while code-created content uses the
    /// existing `NameScope` API. Each call follows the current parent chain.
    /// Finds a named object in the nearest name scope of the element: a scope set on
    /// the element or a logical ancestor, or, for an element in a control template,
    /// the name scope of that template (the one `TemplateApplied` carries), as
    /// WinUI's `TextElement.FindName` finds template names.
    pub fn find_name(&self, name: &str) -> Option<Ref<FerroObject>> {
        let mut current: Option<Ref<StyledElement>> = Some(self.to_ref().upcast());
        while let Some(element) = current {
            if let Some(scope) = NameScope::get_name_scope(&element) {
                return scope.find(name);
            }
            let parent = element.parent();
            if let Some(parent) = &parent {
                // The root of a template has the templated control as its logical
                // parent and as its templated parent.
                let is_template_root =
                    element.templated_parent().is_some_and(|templated| templated == parent.clone().upcast::<FerroObject>());
                if is_template_root {
                    if let Some(scope) = parent.cast::<TemplatedControl>().and_then(|control| control.template_name_scope()) {
                        return scope.find(name);
                    }
                }
            }
            current = parent;
        }
        None
    }

    pub(crate) fn invalidate_format_change(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(host) = self.inline_host() {
            if change.is_inherited_value_change() {
                host.invalidate_layout();
            } else {
                host.invalidate();
            }
        }
    }

    /// Gets the assigned or inherited language before empty-tag resolution.
    pub fn language(&self) -> CultureInfo {
        self.get_value(Self::language_property())
    }

    pub fn set_language(&self, value: CultureInfo) {
        self.set_value(Self::language_property(), value);
    }

    /// The language passed to font fallback and shaping for this element.
    pub fn resolved_language(&self) -> CultureInfo {
        super::text_language::resolved_language(&self.language())
    }

    /// Whether `Language` is set on this element or on an element it inherits from.
    pub fn has_language(&self) -> bool {
        let property = Self::language_property().as_property();
        let mut current: Option<Ref<FerroObject>> = Some(self.to_ref().upcast());
        while let Some(object) = current {
            if object.is_set(property) {
                return true;
            }
            current = object.inheritance_parent();
        }
        false
    }

    /// The culture of the text runs of this element: the resolved language when
    /// `Language` is set on it or inherited from an element that sets it, else none.
    ///
    /// Deviation (DEVIATIONS.md, Text block): `Inline.CreateTextRunProperties` passes no
    /// culture; a run with a language gets it (WinUI's `TextElement.Language`), and one
    /// without keeps upstream's `None`, so text with Inlines shapes as plain text.
    pub fn run_culture(&self) -> Option<CultureInfo> {
        self.has_language().then(|| self.resolved_language())
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            inline_host: RefCell::new(None),
            media_subscriptions: RefCell::new(Vec::new()),
        }
    }

    fn refresh_media_subscriptions(&self) {
        let old = std::mem::take(&mut *self.media_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let mut subscriptions = Vec::new();
        if let Some(features) = self.font_features() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(features.collection_changed(move |_| {
                if let Some(element) = weak.upgrade() {
                    if let Some(host) = element.inline_host() {
                        if element
                            .get_base_value(Self::font_features_property())
                            .is_some()
                        {
                            host.invalidate();
                        } else {
                            host.invalidate_layout();
                        }
                    }
                }
            }));
        }
        for brush in [self.foreground(), self.background()] {
            if brush.as_ref().and_then(|brush| brush.as_object()).is_none() {
                continue;
            }
            let weak = self.to_ref().downgrade();
            let changed: Rc<dyn Fn()> = Rc::new(move || {
                if let Some(host) = weak.upgrade().and_then(|element| element.inline_host()) {
                    host.invalidate_render();
                }
            });
            if let Some(subscription) = subscribe_invalidated(&brush, changed) {
                subscriptions.push(subscription);
            }
        }
        *self.media_subscriptions.borrow_mut() = subscriptions;
    }

    /// A brush used to paint the control's background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// The font family.
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }

    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value)
    }

    /// The font features.
    pub fn font_features(&self) -> Option<FontFeatureCollection> {
        self.get_value(Self::font_features_property())
    }

    pub fn set_font_features(&self, value: Option<FontFeatureCollection>) {
        self.set_value(Self::font_features_property(), value)
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

    /// A brush used to paint the text.
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }

    /// The letter spacing.
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
    }

    pub fn set_letter_spacing(&self, value: f64) {
        self.set_value(Self::letter_spacing_property(), value)
    }

    /// Gets the value of the attached `FontFamily` property on a control.
    pub fn get_font_family(control: &Control) -> FontFamily {
        control.get_value(Self::font_family_property())
    }

    /// Sets the value of the attached `FontFamily` property on a control.
    pub fn set_font_family_on(control: &Control, value: FontFamily) {
        control.set_value(Self::font_family_property(), value)
    }

    /// Gets the value of the attached `FontFeatures` property on a control.
    pub fn get_font_features(control: &Control) -> Option<FontFeatureCollection> {
        control.get_value(Self::font_features_property())
    }

    /// Sets the value of the attached `FontFeatures` property on a control.
    pub fn set_font_features_on(control: &Control, value: Option<FontFeatureCollection>) {
        control.set_value(Self::font_features_property(), value)
    }

    /// Gets the value of the attached `LetterSpacing` property on a control.
    pub fn get_letter_spacing(control: &Control) -> f64 {
        control.get_value(Self::letter_spacing_property())
    }

    /// Sets the value of the attached `LetterSpacing` property on a control.
    pub fn set_letter_spacing_on(control: &Control, value: f64) {
        control.set_value(Self::letter_spacing_property(), value)
    }

    /// Gets the value of the attached `FontSize` property on a control.
    pub fn get_font_size(control: &Control) -> f64 {
        control.get_value(Self::font_size_property())
    }

    /// Sets the value of the attached `FontSize` property on a control.
    pub fn set_font_size_on(control: &Control, value: f64) {
        control.set_value(Self::font_size_property(), value)
    }

    /// Gets the value of the attached `FontStyle` property on a control.
    pub fn get_font_style(control: &Control) -> FontStyle {
        control.get_value(Self::font_style_property())
    }

    /// Sets the value of the attached `FontStyle` property on a control.
    pub fn set_font_style_on(control: &Control, value: FontStyle) {
        control.set_value(Self::font_style_property(), value)
    }

    /// Gets the value of the attached `FontWeight` property on a control.
    pub fn get_font_weight(control: &Control) -> FontWeight {
        control.get_value(Self::font_weight_property())
    }

    /// Sets the value of the attached `FontWeight` property on a control.
    pub fn set_font_weight_on(control: &Control, value: FontWeight) {
        control.set_value(Self::font_weight_property(), value)
    }

    /// Gets the value of the attached `FontStretch` property on a control.
    pub fn get_font_stretch(control: &Control) -> FontStretch {
        control.get_value(Self::font_stretch_property())
    }

    /// Sets the value of the attached `FontStretch` property on a control.
    pub fn set_font_stretch_on(control: &Control, value: FontStretch) {
        control.set_value(Self::font_stretch_property(), value)
    }

    /// Gets the value of the attached `Foreground` property on a control.
    pub fn get_foreground(control: &Control) -> Option<Rc<dyn IBrush>> {
        control.get_value(Self::foreground_property())
    }

    /// Sets the value of the attached `Foreground` property on a control.
    pub fn set_foreground_on(control: &Control, value: Option<Rc<dyn IBrush>>) {
        control.set_value(Self::foreground_property(), value)
    }

    /// The host of the element.
    pub(crate) fn inline_host(&self) -> Option<Rc<dyn IInlineHost>> {
        self.inline_host.borrow().clone()
    }

    pub(crate) fn set_inline_host(&self, value: Option<Rc<dyn IInlineHost>>) {
        let old_value = self.inline_host.replace(value.clone());
        self.on_inline_host_changed(old_value.as_ref(), value.as_ref());
    }
}
