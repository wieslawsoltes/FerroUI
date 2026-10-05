use super::{EmbeddedControlRun, IInlineHost, Inline, InlineImpl, TextElementImpl};
use crate::Control;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, IntoRef, Nullable, Ref, Size, StyledElement, StyledElementImpl, StyledProperty,
    Visual,
};
use std::cell::Cell;
use std::rc::Rc;

/// `InlineUIContainer` - a wrapper for embedding a control into text flow
/// content.
#[repr(C)]
pub struct InlineUIContainer {
    base: Inline,
    measured_width: Cell<f64>,
}

ferro_class!(InlineUIContainer: Inline);
ferroui_base::ferro_class_info!(InlineUIContainer { new: InlineUIContainer::new });

impl StyledElementImpl for InlineUIContainer {}

impl FerroObjectImpl for InlineUIContainer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::child_property().as_property() {
            let (old_child, new_child) = change.get_old_and_new_value::<Option<Ref<Control>>>();
            let host = this.inline_host();

            if let Some(old_child) = old_child {
                this.logical_children().remove(&old_child.clone().upcast::<StyledElement>());
                if let Some(host) = &host {
                    host.with_visual_children(&mut |children| {
                        children.remove(&old_child.clone().upcast::<Visual>());
                    });
                }
            }

            if let Some(new_child) = new_child {
                this.logical_children().add(new_child.clone().upcast::<StyledElement>());
                if let Some(host) = &host {
                    host.with_visual_children(&mut |children| children.add(new_child.clone().upcast::<Visual>()));
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
        let child: Ref<Visual> = child.upcast();

        if let Some(old_value) = old_value {
            old_value.with_visual_children(&mut |children| {
                children.remove(&child);
            });
        }

        if let Some(new_value) = new_value {
            new_value.with_visual_children(&mut |children| children.add(child.clone()));
        }
    }
}

impl InlineImpl for InlineUIContainer {
    fn build_text_run(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        // A container without a child contributes no run.
        let Some(child) = this.child() else { return };

        text_runs.push(Rc::new(EmbeddedControlRun::new(child, this.create_text_run_properties())));
    }

    fn measure_embedded_controls(this: &Self, block_size: Size) -> bool {
        let Some(child) = this.child() else { return false };

        if this.measured_width.get() == block_size.width && child.is_measure_valid() {
            return false;
        }

        let previous_size = child.desired_size();

        child.measure(Size::new(block_size.width, f64::INFINITY));
        this.measured_width.set(block_size.width);

        child.desired_size() != previous_size
    }

    fn append_text(_this: &Self, string_builder: &mut String) {
        // The embedded control run occupies one position in the text layout
        // (the default text source length). Append the Unicode Object
        // Replacement Character so that the text of the inlines stays in
        // sync with the character offsets returned by text hit testing.
        string_builder.push('\u{FFFC}');
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
        Self { base: Inline::construct(), measured_width: Cell::new(f64::NAN) }
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
