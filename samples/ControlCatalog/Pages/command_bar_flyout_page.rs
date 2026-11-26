//! Not from upstream: the page of WinUI's CommandBarFlyout (docs/porting/winui-controls.md,
//! section 10.17), the class of `Pages/CommandBarFlyoutPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::{ContextRequestedEventArgs, InputElement};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    Button, CheckBox, CommandBarButton, CommandBarFlyout, Control, ContentPage, FlyoutShowMode, PlacementMode,
    Symbol, SymbolIcon, TextBlock,
};

#[repr(C)]
pub struct CommandBarFlyoutPage {
    base: ContentPage,
}

content_page_class!(CommandBarFlyoutPage);
ferro_class_info!(CommandBarFlyoutPage { new: CommandBarFlyoutPage::new });
xaml_class!(CommandBarFlyoutPage, "/Pages/CommandBarFlyoutPage.xaml");

/// A command of the flyout that writes its label to `output`.
fn command(label: &str, symbol: Option<Symbol>, output: &Ref<TextBlock>) -> Ref<CommandBarButton> {
    let button = CommandBarButton::new();
    button.set_label(Some(label));
    if let Some(symbol) = symbol {
        button.set_icon(Some(Control::boxed(SymbolIcon::with_symbol(symbol)) as BoxedValue));
    }
    let output = output.downgrade();
    let text = format!("You clicked: {label}");
    button.click(move |_, _| {
        if let Some(output) = output.upgrade() {
            output.set_text(Some(&text));
        }
    });
    button
}

impl CommandBarFlyoutPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.wire();
        this
    }

    /// The flyout with Share, Save and Delete in the row and Resize and Move in the menu, opened
    /// transient by the first button and standard by the second (and its context request).
    fn wire(&self) {
        let output = self.get_control::<TextBlock>("Output");
        let flyout = CommandBarFlyout::new();
        flyout.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        for (label, symbol) in [("Share", Symbol::Share), ("Save", Symbol::Save), ("Delete", Symbol::Delete)] {
            flyout.primary_commands().add(command(label, Some(symbol), &output).as_command_bar_element());
        }
        for label in ["Resize", "Move"] {
            flyout.secondary_commands().add(command(label, None, &output).as_command_bar_element());
        }
        let always_expanded = self.get_control::<CheckBox>("AlwaysExpanded").downgrade();
        let show = move |flyout: &Ref<CommandBarFlyout>, target: &Control, mode: FlyoutShowMode| {
            let expanded = always_expanded.upgrade().is_some_and(|check| check.is_checked() == Some(true));
            flyout.set_always_expanded(expanded);
            flyout.set_show_mode(mode);
            flyout.show_at(target);
        };
        let transient = self.get_control::<Button>("Transient");
        let (open, show_transient) = (flyout.clone(), show.clone());
        let weak = transient.downgrade();
        transient.click(move |_, _| {
            if let Some(button) = weak.upgrade() {
                show_transient(&open, &button, FlyoutShowMode::Transient);
            }
        });
        let standard = self.get_control::<Button>("Standard");
        let (open, show_standard) = (flyout.clone(), show.clone());
        let weak = standard.downgrade();
        standard.click(move |_, _| {
            if let Some(button) = weak.upgrade() {
                show_standard(&open, &button, FlyoutShowMode::Standard);
            }
        });
        let weak = standard.downgrade();
        standard.add_handler(InputElement::context_requested_event(), move |_, e: &ContextRequestedEventArgs| {
            if let Some(button) = weak.upgrade() {
                show(&flyout, &button, FlyoutShowMode::Standard);
                e.set_handled(true);
            }
        });
    }
}
