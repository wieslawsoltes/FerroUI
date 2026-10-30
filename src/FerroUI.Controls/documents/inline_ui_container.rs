use super::{EmbeddedControlRun, IInlineHost, Inline, InlineImpl, TextElementImpl};
use crate::Control;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, IntoRef, Nullable, Ref, Size, StyledElement, StyledElementImpl,
    StyledProperty, Visual,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `InlineUIContainer` - a wrapper for embedding a control into text flow
/// content.
#[repr(C)]
pub struct InlineUIContainer {
    base: Inline,
    measured_width: Cell<f64>,
    measured_metrics: Cell<Option<(Size, f64)>>,
    hosted_metrics: RefCell<Rc<Cell<Option<(Size, f64)>>>>,
}

ferro_class!(InlineUIContainer: Inline);
ferroui_base::ferro_class_info!(InlineUIContainer {
    new: InlineUIContainer::new
});

ferroui_base::ferro_impl_classes!(InlineUIContainer: StyledElementImpl);

impl FerroObjectImpl for InlineUIContainer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::child_property().as_property() {
            let (old_child, new_child) = change.get_old_and_new_value::<Option<Ref<Control>>>();
            // A property callback can already have replaced the proposed child.
            if this.child() != new_child {
                return;
            }
            this.measured_width.set(f64::NAN);
            this.measured_metrics.set(None);
            this.hosted_metrics.replace(Rc::new(Cell::new(None)));
            let host = this.inline_host();

            if let Some(old_child) = old_child {
                let old_visual_parent = old_child.visual_parent();
                this.logical_children()
                    .remove(&old_child.clone().upcast::<StyledElement>());
                // A linked rich paragraph can currently host this object in
                // an overflow page even though its logical inline host is the
                // document owner. Detach the actual visual owner immediately.
                // Logical removal can transfer it elsewhere or restore it as
                // Child; in either case the callback's ownership wins.
                if this.child().as_ref() != Some(&old_child) {
                    if let Some(parent) = old_visual_parent {
                        if old_child.visual_parent().as_ref() == Some(&parent) {
                            parent
                                .visual_children()
                                .remove(&old_child.clone().upcast::<Visual>());
                        }
                    }
                }
            }

            if this.child() == new_child {
                if let Some(new_child) = new_child {
                    let logical = new_child.clone().upcast::<StyledElement>();
                    if !this.logical_children().contains(&logical) {
                        this.logical_children().add(logical);
                    }
                    if this.child().as_ref() == Some(&new_child) {
                        if let Some(current_host) = this.inline_host() {
                            this.attach_child_to_host(&new_child, &current_host);
                        }
                    }
                }
            }

            if let Some(host) = &host {
                host.invalidate();
            }
        }
    }
}

impl TextElementImpl for InlineUIContainer {
    fn on_inline_host_changed(
        this: &Self,
        old_value: Option<&Rc<dyn IInlineHost>>,
        new_value: Option<&Rc<dyn IInlineHost>>,
    ) {
        let Some(child) = this.child() else { return };
        if !this.has_inline_host(new_value) {
            return;
        }

        if old_value.is_some() {
            if let Some(parent) = child.visual_parent() {
                parent
                    .visual_children()
                    .remove(&child.clone().upcast::<Visual>());
            }
        }

        if this.child().as_ref() == Some(&child) && this.has_inline_host(new_value) {
            if let Some(new_value) = new_value {
                this.attach_child_to_host(&child, new_value);
            }
        }
    }
}

impl InlineImpl for InlineUIContainer {
    fn build_text_run(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        // A container without a child contributes no run.
        let Some(child) = this.child() else { return };

        text_runs.push(Rc::new(
            EmbeddedControlRun::new(child, this.create_text_run_properties())
                .with_hosted_cache(this.hosted_metrics.borrow().clone()),
        ));
    }

    fn measure_embedded_controls(this: &Self, block_size: Size) -> bool {
        let Some(child) = this.child() else {
            return false;
        };

        if this.measured_width.get() != block_size.width || !child.is_measure_valid() {
            child.measure(Size::new(block_size.width, f64::INFINITY));
            this.measured_width.set(block_size.width);
        }

        // A different first-line baseline can leave desired size unchanged.
        // Also detect a child that was independently remeasured before its host.
        // This is layout invalidation only; the document's text is unchanged.
        let metrics = (
            child.desired_size(),
            EmbeddedControlRun::measured_baseline(&child),
        );
        this.measured_metrics.replace(Some(metrics)) != Some(metrics)
    }

    fn append_text(this: &Self, string_builder: &mut String) {
        // The embedded control run occupies one position in the text layout
        // (the default text source length). Append the Unicode Object
        // Replacement Character so that the text of the inlines stays in
        // sync with the character offsets returned by text hit testing.
        if this.child().is_some() {
            string_builder.push('\u{FFFC}');
        }
    }
}

ferroui_base::ferro_properties! { impl InlineUIContainer {
    ferro_property!(
        /// Defines the `Child` property.
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<InlineUIContainer, _>("Child", None)
        }
    );
} }

impl InlineUIContainer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Inline::construct(),
            measured_width: Cell::new(f64::NAN),
            measured_metrics: Cell::new(None),
            hosted_metrics: RefCell::new(Rc::new(Cell::new(None))),
        }
    }

    fn has_inline_host(&self, expected: Option<&Rc<dyn IInlineHost>>) -> bool {
        match (self.inline_host().as_ref(), expected) {
            (Some(current), Some(expected)) => Rc::ptr_eq(current, expected),
            (None, None) => true,
            _ => false,
        }
    }

    fn attach_child_to_host(&self, child: &Ref<Control>, host: &Rc<dyn IInlineHost>) {
        // Both tree notifications and host access are callback boundaries.
        // An inline transition must never reclaim a child transferred there.
        host.with_visual_children(&mut |children| {
            if self.child().as_ref() == Some(child)
                && self.has_inline_host(Some(host))
                && child.visual_parent().is_none()
            {
                children.add(child.clone().upcast::<Visual>());
            }
        });
    }

    /// Initializes a new instance of the `InlineUIContainer` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Initializes a new instance of the `InlineUIContainer` class with the
    /// control set as its child.
    pub fn with_child(child: impl IntoRef<Control>) -> Ref<Self> {
        let container = Self::new();
        container.set_child(child.into_ref());
        container
    }

    /// The control of the container.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.get_value(Self::child_property())
    }

    pub fn set_child(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::child_property(), value.into().0)
    }
}
