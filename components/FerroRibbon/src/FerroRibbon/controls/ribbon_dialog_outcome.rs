//! Concrete ribbon modal outcome. WinUI ContentDialogResult is mapped at this
//! boundary; no managed dialog/task/runtime service is recreated.
use crate::compat::ContentDialogButton;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RibbonDialogOutcome {
    #[default]
    Dismissed,
    Primary,
    Secondary,
}
ferroui_base::ferro_markup_enum!(RibbonDialogOutcome {Dismissed, Primary, Secondary},{namespace:"FerroRibbon.Controls"});
impl RibbonDialogOutcome {
    pub(crate) fn from_button(button: ContentDialogButton) -> Self {
        match button {
            ContentDialogButton::Primary => Self::Primary,
            ContentDialogButton::Secondary => Self::Secondary,
            _ => Self::Dismissed,
        }
    }
}
