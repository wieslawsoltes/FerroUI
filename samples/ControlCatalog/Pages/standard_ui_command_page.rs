//! Not from upstream: the page of WinUI's StandardUICommand (docs/porting/winui-controls.md,
//! section 10.16), the class of `Pages/StandardUICommandPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{
    CommandBar, CommandBarButton, ContentPage, StandardUICommand, StandardUICommandKind, TextBlock,
};

/// The kinds of the primary commands of the bar; the others are its secondary commands.
const PRIMARY: &[StandardUICommandKind] = &[
    StandardUICommandKind::Cut,
    StandardUICommandKind::Copy,
    StandardUICommandKind::Paste,
    StandardUICommandKind::Delete,
    StandardUICommandKind::Save,
];

const SECONDARY: &[StandardUICommandKind] = &[
    StandardUICommandKind::Undo,
    StandardUICommandKind::Redo,
    StandardUICommandKind::SelectAll,
    StandardUICommandKind::Share,
    StandardUICommandKind::Open,
    StandardUICommandKind::Close,
];

#[repr(C)]
pub struct StandardUICommandPage {
    base: ContentPage,
}

content_page_class!(StandardUICommandPage);
ferro_class_info!(StandardUICommandPage { new: StandardUICommandPage::new });
xaml_class!(StandardUICommandPage, "/Pages/StandardUICommandPage.xaml");

impl StandardUICommandPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.wire();
        this
    }

    /// A command bar button of each kind, which takes its label, icon and hot key from the
    /// command; running one writes its label and description.
    fn wire(&self) {
        let bar = self.get_control::<CommandBar>("Commands");
        let output = self.get_control::<TextBlock>("Output");
        for (kinds, commands) in [(PRIMARY, bar.primary_commands()), (SECONDARY, bar.secondary_commands())] {
            for &kind in kinds {
                let command = StandardUICommand::with_kind(kind);
                let output = output.downgrade();
                command
                    .execute_requested(move |command, _| {
                        if let Some(output) = output.upgrade() {
                            output.set_text(Some(&format!("{}: {}", command.label(), command.description())));
                        }
                    })
                    .detach();
                let button = CommandBarButton::new();
                button.set_command(Some(command.as_command()));
                commands.add(button.as_command_bar_element());
            }
        }
    }
}
