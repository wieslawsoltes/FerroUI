//! Standard shared observation used only by source-corpus tests that sample
//! completion repeatedly from lifecycle callbacks. No task implementation.
use crate::controls::{RibbonColorDialog, RibbonColorDialogFuture};
use ferroui_base::{Ref, media::Color, styling::ThemeVariant};
use ferroui_controls::TopLevel;
use futures_util::{FutureExt, future::Shared};

pub(crate) fn show_color(
    root: Option<Ref<TopLevel>>,
    initial: Color,
    theme: Option<ThemeVariant>,
) -> Shared<RibbonColorDialogFuture> {
    RibbonColorDialog::show_async(root, initial, theme).shared()
}
