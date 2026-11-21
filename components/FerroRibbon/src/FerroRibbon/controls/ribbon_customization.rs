//! Ribbon.Menus.cs customization entry and cancellation through Rust callbacks.
//! Copyright Wiesław Šoltés, MIT. The dialog and its native modal scope own UI lifetime.
use super::*;
use crate::controls::{RibbonCustomizeDialog, RibbonCustomizeDialogError, RibbonCustomizePage};
use ferroui_base::{
    reactive::{Disposable, IDisposable},
    utilities::HandlerList,
};
use ferroui_controls::TopLevel;
/// Setting handled suppresses the built-in customization UI. All handlers run in
/// registration order, even after an earlier handler sets this flag.
pub struct RibbonCustomizeRequestedEventArgs {
    pub page: RibbonCustomizePage,
    pub handled: Cell<bool>,
}
impl PartialEq for RibbonCustomizeRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonCustomizeRequestedEventArgs {namespace:"FerroRibbon.Controls",this:Rc<RibbonCustomizeRequestedEventArgs>,handles:[Rc<RibbonCustomizeRequestedEventArgs>],properties:[Page:RibbonCustomizePage{get:|args:&Rc<RibbonCustomizeRequestedEventArgs>|args.page},Handled:bool{get:|args:&Rc<RibbonCustomizeRequestedEventArgs>|args.handled.get(),set:|args:&Rc<RibbonCustomizeRequestedEventArgs>,value|args.handled.set(value)}]});
type CustomizeHandler = dyn Fn(&Rc<RibbonCustomizeRequestedEventArgs>);
pub(super) struct CustomizationState {
    requested: HandlerList<CustomizeHandler>,
    last_error: RefCell<Option<RibbonCustomizeDialogError>>,
    markup_requested: RefCell<Vec<(MarkupDelegate, u64)>>,
}
impl Default for CustomizationState {
    fn default() -> Self {
        Self {
            requested: HandlerList::new(),
            last_error: RefCell::new(None),
            markup_requested: RefCell::new(Vec::new()),
        }
    }
}
impl Ribbon {
    pub fn add_customize_requested(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let weak = self.to_ref().downgrade();
        let token = self.customization_ui.requested.add(Rc::new(move |args| {
            if let Some(root) = weak.upgrade() {
                callback
                    .try_invoke(&[Some(Control::boxed(root)), Some(Rc::new(args.clone()))])
                    .expect("CustomizeRequested markup handler");
            }
        }));
        self.customization_ui
            .markup_requested
            .borrow_mut()
            .push((handler, token));
    }
    pub fn remove_customize_requested(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut entries = self.customization_ui.markup_requested.borrow_mut();
            entries
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|index| entries.remove(index))
        };
        if let Some((_, token)) = removed {
            self.customization_ui.requested.remove(token);
        }
    }

    pub fn customize_requested(
        &self,
        handler: impl Fn(&RibbonCustomizeRequestedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self
            .customization_ui
            .requested
            .add(Rc::new(move |args| handler(args)));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(ribbon) = weak.upgrade() {
                ribbon.customization_ui.requested.remove(token);
            }
        })
    }
    /// Raises CustomizeRequested before checking attachment. None means the
    /// request was handled or there is no window. Native open failures are typed.
    pub fn show_customize_dialog(
        &self,
        page: RibbonCustomizePage,
    ) -> Result<Option<Ref<RibbonCustomizeDialog>>, RibbonCustomizeDialogError> {
        let args = Rc::new(RibbonCustomizeRequestedEventArgs {
            page,
            handled: Cell::new(false),
        });
        for (_, handler) in self.customization_ui.requested.snapshot().iter() {
            handler(&args);
        }
        if args.handled.get() {
            return Ok(None);
        }
        let Some(root) = TopLevel::get_top_level(Some(self)) else {
            return Ok(None);
        };
        let dialog = RibbonCustomizeDialog::new(self.to_ref(), page)?;
        RibbonTheme::set_theme(Some(&dialog), self.actual_theme_variant())
            .expect("customization theme");
        dialog.show(root)?;
        Ok(Some(dialog))
    }
    /// Diagnostic for source-tolerated busy/closing UI requests; programmatic
    /// callers receive the same error directly from show_customize_dialog.
    pub fn last_customize_dialog_error(&self) -> Option<RibbonCustomizeDialogError> {
        self.customization_ui.last_error.borrow().clone()
    }
    pub(super) fn request_customize_dialog(&self, page: RibbonCustomizePage) {
        self.customization_ui.last_error.replace(None);
        match self.show_customize_dialog(page) {
            Ok(_) => {}
            Err(
                error @ (RibbonCustomizeDialogError::AlreadyOpen
                | RibbonCustomizeDialogError::RootNotVisible),
            ) => {
                self.customization_ui.last_error.replace(Some(error));
            }
            Err(error) => panic!("Ribbon customization request failed: {error}"),
        }
    }
}
