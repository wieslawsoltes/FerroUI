//! Port of `Diagnostics/Controls/CommitTextBox.cs`.
//!
//! The original notes that `UpdateSourceTrigger` and the validation rules of a binding could
//! remove the need for this control.

use ferroui_base::FerroObjectImplExt;
use ferroui_base::input::InputElementImplExt;
use ferroui_base::input::{FocusChangedEventArgs, InputElementImpl, Key, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, DataValidationErrors, TextBox, TextBoxImpl};
use std::cell::RefCell;

/// A text box whose text is committed to [`committed_text`](Self::committed_text) when Enter is
/// pressed or the focus leaves it, and reverted when Escape is pressed: the text of an edit that
/// does not convert is never committed.
#[repr(C)]
pub struct CommitTextBox {
    base: TextBox,
    committed_text: RefCell<Option<String>>,
}

ferro_class!(CommitTextBox: TextBox);
ferro_impl_classes!(
    CommitTextBox: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    TextBoxImpl
);
ferro_class_info!(CommitTextBox { new: CommitTextBox::new });

ferro_properties! {
    impl CommitTextBox {
        /// Defines the `CommittedText` property.
        pub fn committed_text_property() -> DirectProperty<CommitTextBox, Option<String>> {
            FerroProperty::register_direct::<CommitTextBox, _>(
                "CommittedText",
                |o| o.committed_text(),
                Some(|o, v| o.set_committed_text(v)),
                None,
            )
        }
    }
}

impl StyledElementImpl for CommitTextBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        TextBox::TYPE
    }
}

impl FerroObjectImpl for CommitTextBox {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::committed_text_property().as_property() {
            this.set_text(this.committed_text().as_deref());
        }
    }
}

impl InputElementImpl for CommitTextBox {
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);

        match e.key {
            Key::Enter => {
                this.try_commit();

                e.set_handled(true);
            }

            Key::Escape => {
                this.cancel();

                e.set_handled(true);
            }

            _ => {}
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        this.try_commit();
    }
}

impl CommitTextBox {
    pub fn construct() -> Self {
        Self { base: TextBox::construct(), committed_text: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn committed_text(&self) -> Option<String> {
        self.committed_text.borrow().clone()
    }

    pub fn set_committed_text(&self, value: Option<String>) {
        self.set_and_raise(Self::committed_text_property(), &self.committed_text, value);
    }

    fn cancel(&self) {
        self.set_text(self.committed_text().as_deref());
        DataValidationErrors::clear_errors(self);
    }

    fn try_commit(&self) {
        if !DataValidationErrors::get_has_errors(self) {
            self.set_committed_text(self.text());
        } else {
            self.set_text(self.committed_text().as_deref());
            DataValidationErrors::clear_errors(self);
        }
    }
}
