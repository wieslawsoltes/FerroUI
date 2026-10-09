use super::{IInlineHost, Inline, InlineCollection, InlineImpl, InlineUIContainer, Run, TextElementImpl, TextElementImplExt};
use crate::Control;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::metadata::IAddChild;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, IntoRef, Ref, Size, StyledElementImpl, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Span element used for grouping other `Inline` elements.
#[repr(C)]
pub struct Span {
    base: Inline,
    inlines_invalidated: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(Span: Inline);
ferroui_base::ferro_class_info!(Span {
    new: Span::new,
    interfaces: [
        Rc<dyn IAddChild<Ref<Inline>>> => Span::as_add_child_of_inline,
        Rc<dyn IAddChild<Ref<Control>>> => Span::as_add_child_of_control,
        Rc<dyn IAddChild<String>> => Span::as_add_child_of_string,
    ],
});

/// Implements the child-adding contracts for the span behind a class
/// handle.
struct SpanAddChild(Ref<Span>);

impl SpanAddChild {
    fn inlines(&self) -> Option<InlineCollection> {
        self.0.get_value(Span::inlines_property())
    }
}

impl IAddChild<Ref<Inline>> for SpanAddChild {
    fn add_child(&self, inline: Ref<Inline>) {
        if let Some(inlines) = self.inlines() {
            inlines.add(inline);
        }
    }

    fn reference_id(&self) -> usize {
        &*self.0 as *const Span as usize
    }
}

impl IAddChild<Ref<Control>> for SpanAddChild {
    fn add_child(&self, child: Ref<Control>) {
        if let Some(inlines) = self.inlines() {
            inlines.add(InlineUIContainer::with_child(child));
        }
    }

    fn reference_id(&self) -> usize {
        &*self.0 as *const Span as usize
    }
}

impl IAddChild<String> for SpanAddChild {
    fn add_child(&self, text: String) {
        if let Some(inlines) = self.inlines() {
            inlines.add(Run::with_text(Some(&text)));
        }
    }

    fn reference_id(&self) -> usize {
        &*self.0 as *const Span as usize
    }
}

ferroui_base::ferro_impl_classes!(Span: StyledElementImpl);

impl FerroObjectImpl for Span {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let inlines = InlineCollection::new();
        inlines.set_logical_children(Some(this));
        this.set_inlines(inlines);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        // The reference compares with the name of the property field
        // ("InlinesProperty"), which no property has: replacing the
        // collection is not observed.
        if change.property().name() == "InlinesProperty" {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<InlineCollection>>();
            this.on_inlines_changed(old_value, new_value);
            if let Some(host) = this.inline_host() {
                host.invalidate();
            }
        }
    }
}

impl TextElementImpl for Span {
    fn on_inline_host_changed(
        this: &Self,
        old_value: Option<&Rc<dyn IInlineHost>>,
        new_value: Option<&Rc<dyn IInlineHost>>,
    ) {
        Self::parent_on_inline_host_changed(this, old_value, new_value);

        this.inlines().set_inline_host(new_value.cloned());
    }
}

impl InlineImpl for Span {
    fn build_text_run(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        for inline in this.inlines().snapshot().iter() {
            inline.build_text_run(text_runs);
        }
    }

    fn measure_embedded_controls(this: &Self, block_size: Size) -> bool {
        let mut resized = false;

        for inline in this.inlines().snapshot().iter() {
            resized |= inline.measure_embedded_controls(block_size);
        }

        resized
    }

    fn append_text(this: &Self, string_builder: &mut String) {
        for inline in this.inlines().snapshot().iter() {
            inline.append_text(string_builder);
        }
    }
}

ferroui_base::ferro_properties! { impl Span {
    ferro_property!(
        /// Defines the `Inlines` property.
        pub fn inlines_property() -> StyledProperty<Option<InlineCollection>> {
            FerroProperty::register::<Span, _>("Inlines", None)
        }
    );
} }

impl Span {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Inline::construct(), inlines_invalidated: RefCell::new(None) }
    }

    /// The span as the contract through which markup adds an inline child.
    pub fn as_add_child_of_inline(this: Ref<Span>) -> Rc<dyn IAddChild<Ref<Inline>>> {
        Rc::new(SpanAddChild(this))
    }

    /// The span as the contract through which markup adds a control child:
    /// the control is wrapped in an inline container.
    pub fn as_add_child_of_control(this: Ref<Span>) -> Rc<dyn IAddChild<Ref<Control>>> {
        Rc::new(SpanAddChild(this))
    }

    /// The span as the contract through which markup adds a text child: the
    /// text becomes a run.
    pub fn as_add_child_of_string(this: Ref<Span>) -> Rc<dyn IAddChild<String>> {
        Rc::new(SpanAddChild(this))
    }

    /// Initializes a new instance of a `Span` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the inlines.
    ///
    /// Panics when the collection was removed by setting the property to
    /// its unset value.
    pub fn inlines(&self) -> InlineCollection {
        self.get_value(Self::inlines_property()).expect("the inlines of a span cannot be null")
    }

    pub fn set_inlines(&self, value: InlineCollection) {
        self.set_value(Self::inlines_property(), Some(value))
    }

    fn on_inlines_changed(&self, old_value: Option<InlineCollection>, new_value: Option<InlineCollection>) {
        if let Some(old_value) = old_value {
            old_value.set_logical_children(None);
            old_value.set_inline_host(None);
            if let Some(subscription) = self.inlines_invalidated.borrow_mut().take() {
                subscription.dispose();
            }
        }

        if let Some(new_value) = new_value {
            new_value.set_logical_children(Some(self));
            new_value.set_inline_host(self.inline_host());
            let weak = self.to_ref().downgrade();
            *self.inlines_invalidated.borrow_mut() = Some(new_value.invalidated(move || {
                if let Some(host) = weak.upgrade().and_then(|this| this.inline_host()) {
                    host.invalidate();
                }
            }));
        }
    }

    /// Adds an inline child (the content model of the element).
    pub fn add_child(&self, inline: impl IntoRef<Inline>) {
        if let Some(inlines) = self.get_value(Self::inlines_property()) {
            inlines.add(inline);
        }
    }

    /// Adds a control child, wrapped in an `InlineUIContainer`.
    pub fn add_child_control(&self, child: impl IntoRef<Control>) {
        if let Some(inlines) = self.get_value(Self::inlines_property()) {
            inlines.add(InlineUIContainer::with_child(child));
        }
    }

    /// Adds a text child, wrapped in a `Run`.
    pub fn add_child_text(&self, text: &str) {
        if let Some(inlines) = self.get_value(Self::inlines_property()) {
            inlines.add(Run::with_text(Some(text)));
        }
    }
}
