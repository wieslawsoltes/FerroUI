//! Not from upstream: the page of WinUI's XamlUICommand (docs/porting/winui-controls.md, section
//! 10.16), the class of `Pages/XamlUICommandPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{
    Button, CheckBox, CommandBarButton, ContentPage, KeyboardAccelerator, MenuItem, Symbol, SymbolIconSource,
    TextBlock, XamlUICommand,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct XamlUICommandPage {
    base: ContentPage,
}

content_page_class!(XamlUICommandPage);
ferro_class_info!(XamlUICommandPage { new: XamlUICommandPage::new });
xaml_class!(XamlUICommandPage, "/Pages/XamlUICommandPage.xaml");

impl XamlUICommandPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.wire();
        this
    }

    /// The command with its label, icon, Ctrl+D and description; running it counts the runs, the
    /// check box answers CanExecuteRequested and the rename button changes the label every source
    /// shows.
    fn wire(&self) {
        let command = XamlUICommand::with_label("Favorite");
        command.set_description("Marks the page as a favorite");
        let icon = SymbolIconSource::new();
        icon.set_symbol(Symbol::Favorite);
        command.set_icon_source(Some(icon.upcast()));
        command.keyboard_accelerators().add(KeyboardAccelerator::with_key(Key::D, KeyModifiers::CONTROL));

        let runs = Rc::new(Cell::new(0));
        let output = self.get_control::<TextBlock>("Output").downgrade();
        command
            .execute_requested(move |command, _| {
                runs.set(runs.get() + 1);
                if let Some(output) = output.upgrade() {
                    output.set_text(Some(&format!("{} ran {} times", command.label(), runs.get())));
                }
            })
            .detach();
        let can_execute = self.get_control::<CheckBox>("CanExecute");
        let check = can_execute.downgrade();
        command
            .can_execute_requested(move |_, args| {
                args.set_can_execute(check.upgrade().is_some_and(|check| check.is_checked() == Some(true)));
            })
            .detach();
        let notified = command.clone();
        can_execute.click(move |_, _| notified.notify_can_execute_changed());

        self.get_control::<Button>("CommandButton").set_command(Some(command.as_command()));
        self.get_control::<CommandBarButton>("CommandBarCommandButton").set_command(Some(command.as_command()));
        self.get_control::<MenuItem>("CommandMenuItem").set_command(Some(command.as_command()));
        self.get_control::<Button>("Rename").click(move |_, _| {
            let label = if command.label() == "Favorite" { "Starred" } else { "Favorite" };
            command.set_label(label);
        });
    }
}
