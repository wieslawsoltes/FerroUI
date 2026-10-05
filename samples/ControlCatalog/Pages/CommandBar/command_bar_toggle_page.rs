//! Port of `Pages/CommandBar/CommandBarTogglePage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarTogglePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CheckBox, CommandBarToggleButton, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CommandBarTogglePage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
}

user_control_class!(CommandBarTogglePage);
ferro_class_info!(CommandBarTogglePage {
    new: CommandBarTogglePage::new,
    markup: {
        methods: [
            fn OnFormatChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarTogglePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_format_changed(&sender, e.as_routed_event_args())
                },
            fn OnForceBoldChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarTogglePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_bold_changed(&sender, e.as_routed_event_args())
                },
            fn OnForceItalicChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarTogglePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_italic_changed(&sender, e.as_routed_event_args())
                },
            fn OnForceUnderlineChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarTogglePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_underline_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarTogglePage, "/Pages/CommandBar/CommandBarTogglePage.xaml");

impl CommandBarTogglePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);
        this
    }

    fn force_bold_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("ForceBoldCheck") } else { None }
    }

    fn force_italic_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("ForceItalicCheck") } else { None }
    }

    fn force_underline_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("ForceUnderlineCheck") } else { None }
    }

    fn bold_toggle(&self) -> Option<Ref<CommandBarToggleButton>> {
        if self.initialized.get() { self.find_control::<CommandBarToggleButton>("BoldToggle") } else { None }
    }

    fn italic_toggle(&self) -> Option<Ref<CommandBarToggleButton>> {
        if self.initialized.get() { self.find_control::<CommandBarToggleButton>("ItalicToggle") } else { None }
    }

    fn underline_toggle(&self) -> Option<Ref<CommandBarToggleButton>> {
        if self.initialized.get() { self.find_control::<CommandBarToggleButton>("UnderlineToggle") } else { None }
    }

    fn format_status(&self) -> Option<Ref<TextBlock>> {
        if self.initialized.get() { self.find_control::<TextBlock>("FormatStatus") } else { None }
    }

    fn on_format_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(format_status) = self.format_status() else {
            return;
        };
        // The fields of the named elements are all set once one of them is.
        let (Some(bold_toggle), Some(italic_toggle), Some(underline_toggle)) =
            (self.bold_toggle(), self.italic_toggle(), self.underline_toggle())
        else {
            return;
        };
        let (Some(force_bold_check), Some(force_italic_check), Some(force_underline_check)) =
            (self.force_bold_check(), self.force_italic_check(), self.force_underline_check())
        else {
            return;
        };

        let mut active = Vec::new();
        if bold_toggle.is_checked() == Some(true) {
            active.push("Bold");
        }
        if italic_toggle.is_checked() == Some(true) {
            active.push("Italic");
        }
        if underline_toggle.is_checked() == Some(true) {
            active.push("Underline");
        }

        format_status.set_text(Some(&if active.is_empty() {
            String::from("Active: (none)")
        } else {
            format!("Active: {}", active.join(", "))
        }));

        force_bold_check.set_is_checked(bold_toggle.is_checked());
        force_italic_check.set_is_checked(italic_toggle.is_checked());
        force_underline_check.set_is_checked(underline_toggle.is_checked());
    }

    fn on_force_bold_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(bold_toggle), Some(force_bold_check)) = (self.bold_toggle(), self.force_bold_check()) else {
            return;
        };
        bold_toggle.set_is_checked(force_bold_check.is_checked());
    }

    fn on_force_italic_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(italic_toggle), Some(force_italic_check)) = (self.italic_toggle(), self.force_italic_check()) else {
            return;
        };
        italic_toggle.set_is_checked(force_italic_check.is_checked());
    }

    fn on_force_underline_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(underline_toggle), Some(force_underline_check)) =
            (self.underline_toggle(), self.force_underline_check())
        else {
            return;
        };
        underline_toggle.set_is_checked(force_underline_check.is_checked());
    }
}
