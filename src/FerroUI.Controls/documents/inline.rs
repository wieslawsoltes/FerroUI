use super::{TextElement, TextElementImpl, Typography};
use crate::Control;
use ferroui_base::media::text_formatting::{GenericTextRunProperties, TextRun, TextRunProperties};
use ferroui_base::media::{
    subscribe_invalidated, BaselineAlignment, IBrush, TextDecorationCollection, Typeface,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_property, AttachedProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions,
};
use std::cell::RefCell;
use std::rc::Rc;

/// `Inline` is the base class for inline text elements.
///
/// This class is abstract.
#[repr(C)]
pub struct Inline {
    base: TextElement,
    decoration_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

ferro_class! {
    Inline: TextElement, virtuals InlineImpl: TextElementImpl {
        /// Appends the text runs of the inline to `text_runs`.
        #[doc(hidden)]
        fn build_text_run(this, text_runs: &mut Vec<Rc<dyn TextRun>>);
        /// Measures the controls embedded in the inline for a block of
        /// `block_size`. Returns true when the size of one of them changed.
        #[doc(hidden)]
        fn measure_embedded_controls(this, block_size: Size) -> bool;
        /// Appends the text of the inline to `string_builder`.
        #[doc(hidden)]
        fn append_text(this, string_builder: &mut String);
    }
}

ferroui_base::ferro_impl_classes!(Inline: StyledElementImpl);
ferroui_base::ferro_impl_classes!(Inline: TextElementImpl);

impl FerroObjectImpl for Inline {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.refresh_decoration_subscriptions();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::text_decorations_property().as_property() {
            this.refresh_decoration_subscriptions();
        }

        match change.property().name() {
            "TextDecorations" | "BaselineAlignment" => {
                this.invalidate_format_change(change);
            }
            _ => {}
        }
    }
}

impl Drop for Inline {
    fn drop(&mut self) {
        for subscription in self.decoration_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

impl InlineImpl for Inline {
    fn build_text_run(_this: &Self, _text_runs: &mut Vec<Rc<dyn TextRun>>) {
        panic!("Inline is abstract.")
    }

    fn measure_embedded_controls(_this: &Self, _block_size: Size) -> bool {
        false
    }

    fn append_text(_this: &Self, _string_builder: &mut String) {
        panic!("Inline is abstract.")
    }
}

ferroui_base::ferro_properties! { impl Inline {
    ferro_property!(
        /// `AttachedProperty` for the `TextDecorations` property.
        pub fn text_decorations_property() -> AttachedProperty<Option<TextDecorationCollection>> {
            FerroProperty::register_attached_with::<Inline, Inline, _>(
                "TextDecorations",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    ferro_property!(
        /// `StyledProperty` for the `BaselineAlignment` property.
        pub fn baseline_alignment_property() -> StyledProperty<BaselineAlignment> {
            FerroProperty::register::<Inline, _>("BaselineAlignment", BaselineAlignment::Baseline)
        }
    );
} }

impl Inline {
    fn invalidate_decoration_change(&self) {
        if let Some(host) = self.inline_host() {
            if self
                .get_base_value(Self::text_decorations_property())
                .is_some()
            {
                host.invalidate();
            } else {
                host.invalidate_layout();
            }
        }
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TextElement::construct(),
            decoration_subscriptions: RefCell::new(Vec::new()),
        }
    }

    fn refresh_decoration_subscriptions(&self) {
        let old = std::mem::take(&mut *self.decoration_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
        let Some(decorations) = self.text_decorations() else {
            return;
        };
        let mut subscriptions = Vec::new();
        let weak = self.to_ref().downgrade();
        subscriptions.push(decorations.collection_changed(move |_| {
            if let Some(inline) = weak.upgrade() {
                inline.refresh_decoration_subscriptions();
                inline.invalidate_decoration_change();
            }
        }));
        for decoration in decorations.iter() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(decoration.property_changed(move |_| {
                if let Some(inline) = weak.upgrade() {
                    inline.refresh_decoration_subscriptions();
                    inline.invalidate_decoration_change();
                }
            }));
            if let Some(dashes) = decoration.stroke_dash_array() {
                let weak = self.to_ref().downgrade();
                subscriptions.push(dashes.collection_changed(move |_| {
                    if let Some(inline) = weak.upgrade() {
                        inline.invalidate_decoration_change();
                    }
                }));
            }
            if decoration
                .stroke()
                .as_ref()
                .and_then(|brush| brush.as_object())
                .is_none()
            {
                continue;
            }
            let weak = self.to_ref().downgrade();
            let changed: Rc<dyn Fn()> = Rc::new(move || {
                if let Some(host) = weak.upgrade().and_then(|inline| inline.inline_host()) {
                    host.invalidate_render();
                }
            });
            if let Some(subscription) = subscribe_invalidated(&decoration.stroke(), changed) {
                subscriptions.push(subscription);
            }
        }
        *self.decoration_subscriptions.borrow_mut() = subscriptions;
    }

    /// The `TextDecorations` property specifies decorations that are added
    /// to the text of an element.
    pub fn text_decorations(&self) -> Option<TextDecorationCollection> {
        self.get_value(Self::text_decorations_property())
    }

    pub fn set_text_decorations(&self, value: Option<TextDecorationCollection>) {
        self.set_value(Self::text_decorations_property(), value)
    }

    /// Describes how the baseline for a text-based element is positioned on
    /// the vertical axis, relative to the established baseline for text.
    pub fn baseline_alignment(&self) -> BaselineAlignment {
        self.get_value(Self::baseline_alignment_property())
    }

    pub fn set_baseline_alignment(&self, value: BaselineAlignment) {
        self.set_value(Self::baseline_alignment_property(), value)
    }

    /// Gets the value of the attached `TextDecorations` property on a
    /// control.
    pub fn get_text_decorations(control: &Control) -> Option<TextDecorationCollection> {
        control.get_value(Self::text_decorations_property())
    }

    /// Sets the value of the attached `TextDecorations` property on a
    /// control.
    pub fn set_text_decorations_on(control: &Control, value: Option<TextDecorationCollection>) {
        control.set_value(Self::text_decorations_property(), value)
    }

    /// Creates the run properties of the text of the inline.
    pub fn create_text_run_properties(&self) -> Rc<dyn TextRunProperties> {
        let parent_or_self_background = self.background().or_else(|| self.find_parent_background());

        let typeface = Typeface::with_style(
            self.font_family(),
            self.font_style(),
            self.font_weight(),
            self.font_stretch(),
        );

        Rc::new(GenericTextRunProperties::with_all(
            typeface,
            self.font_size(),
            self.text_decorations(),
            self.foreground(),
            parent_or_self_background,
            self.baseline_alignment(),
            Some(self.resolved_language()),
            Typography::effective_font_features(self),
        ))
    }

    fn find_parent_background(&self) -> Option<Rc<dyn IBrush>> {
        let mut parent = self.parent();

        while let Some(inline) = parent.and_then(|parent| parent.cast::<Inline>()) {
            if let Some(background) = inline.background() {
                return Some(background);
            }

            parent = inline.parent();
        }

        None
    }
}
