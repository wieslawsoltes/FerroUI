//! Native one-dialog future. No general Task/delegate/runtime implementation.
use super::{
    RibbonDialogHost, RibbonDialogOutcome,
    ribbon_dialog_session::{RibbonDialogOpenError, RibbonDialogSession},
};
use ferroui_base::Ref;
use ferroui_controls::TopLevel;
use futures_channel::oneshot;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
/// One observation of a native dialog outcome. An abandoned producer is an
/// explicit channel error; dropping this receiver does not dismiss the dialog.
pub type RibbonDialogFuture = oneshot::Receiver<RibbonDialogOutcome>;
impl RibbonDialogHost {
    /// Shows this exact native control in the owner's modal overlay. Layout,
    /// focus scope, input routing and cleanup use the library's shared session.
    pub fn show_dialog(
        &self,
        root: Ref<TopLevel>,
    ) -> Result<RibbonDialogFuture, RibbonDialogOpenError> {
        let (completion, result) = oneshot::channel();
        let completion = RefCell::new(Some(completion));
        let slot: Rc<RefCell<Option<Weak<RibbonDialogSession>>>> = Rc::new(RefCell::new(None));
        let callback = slot.clone();
        let host = self.to_ref().downgrade();
        let session = RibbonDialogSession::new(root, self.to_ref(), move |button| {
            let session = callback.borrow().as_ref().and_then(Weak::upgrade);
            if let Some(session) = session
                && session.dismiss()
                && let Some(host) = host.upgrade()
            {
                let completion = completion.take().expect("one native dialog completion");
                host.after_dialog_closed(move || {
                    // Cleanup and Closed callbacks run even without an observer.
                    let _ = completion.send(RibbonDialogOutcome::from_button(button));
                });
            }
        })?;
        slot.replace(Some(Rc::downgrade(&session)));
        session.show();
        Ok(result)
    }
}
impl std::fmt::Display for RibbonDialogOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AlreadyOpen => "A native ribbon dialog is already open for this window.",
            Self::RootNotVisible => "The dialog owner is not visible.",
            Self::MissingOverlay => "The dialog owner has no native overlay layer.",
        })
    }
}
impl std::error::Error for RibbonDialogOpenError {}
