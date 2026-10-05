use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, DirectProperty,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::ControlImpl;
use std::cell::RefCell;
use std::rc::Rc;

/// Asks whether to replace an existing file: the content of the flyout the
/// managed storage provider shows when the user saves to a file that
/// exists.
#[repr(C)]
pub struct ManagedFileChooserOverwritePrompt {
    base: TemplatedControl,
    result: Rc<HandlerList<dyn Fn(bool)>>,
    file_name: RefCell<String>,
}

ferro_class!(ManagedFileChooserOverwritePrompt: TemplatedControl);
ferro_impl_classes!(
    ManagedFileChooserOverwritePrompt: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);
ferro_class_info!(ManagedFileChooserOverwritePrompt {
    new: ManagedFileChooserOverwritePrompt::new,
    markup: {
        methods: [
            fn Confirm() => ManagedFileChooserOverwritePrompt::confirm,
            fn Cancel() => ManagedFileChooserOverwritePrompt::cancel,
        ],
    },
});

ferro_properties! { impl ManagedFileChooserOverwritePrompt {
    ferro_property!(
        /// Defines the `FileName` property.
        pub fn file_name_property() -> DirectProperty<ManagedFileChooserOverwritePrompt, String> {
            FerroProperty::register_direct::<ManagedFileChooserOverwritePrompt, _>(
                "FileName",
                |o| o.file_name(),
                Some(|o, v| o.set_file_name(v)),
                String::new(),
            )
        }
    );
} }

impl ManagedFileChooserOverwritePrompt {
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), result: Rc::new(HandlerList::new()), file_name: RefCell::new(String::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised with the answer of the user: `true` to replace the file.
    pub fn result(&self, handler: impl Fn(bool) + 'static) -> Rc<dyn IDisposable> {
        let token = self.result.add(Rc::new(handler));
        let handlers = self.result.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn raise_result(&self, value: bool) {
        for (_, handler) in self.result.snapshot().iter() {
            handler(value);
        }
    }

    /// The name of the file that exists.
    pub fn file_name(&self) -> String {
        self.file_name.borrow().clone()
    }

    pub fn set_file_name(&self, value: String) {
        self.set_and_raise(Self::file_name_property(), &self.file_name, value);
    }

    /// Answers that the file is to be replaced.
    pub fn confirm(&self) {
        self.raise_result(true);
    }

    /// Answers that the file is to be kept.
    pub fn cancel(&self) {
        self.raise_result(false);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn confirm_and_cancel_raise_the_result() {
        let prompt = ManagedFileChooserOverwritePrompt::new();
        let results = Rc::new(RefCell::new(Vec::new()));
        let sink = results.clone();
        let subscription = prompt.result(move |result| sink.borrow_mut().push(result));

        prompt.confirm();
        prompt.cancel();
        subscription.dispose();
        prompt.confirm();

        assert_eq!(vec![true, false], *results.borrow());
    }

    #[test]
    fn file_name_is_a_direct_property() {
        let prompt = ManagedFileChooserOverwritePrompt::new();
        assert_eq!("", prompt.file_name());

        prompt.set_file_name("a.txt".to_string());

        assert_eq!("a.txt", prompt.get_direct_value(ManagedFileChooserOverwritePrompt::file_name_property()));
    }
}
