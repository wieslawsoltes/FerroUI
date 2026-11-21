//! Native modal overlay for the source RibbonColorDialog, with eager request and standard future completion.
//! Copyright Wiesław Šoltés, MIT. Platform/ContentDialog adaptations are tracked explicitly.
use super::{
    RibbonColorDialogHost,
    ribbon_color_dialog_content::RibbonColorDialogContent,
    ribbon_dialog_session::{RibbonDialogOpenError, RibbonDialogSession},
};
use ferroribbon_core::localization::RibbonStrings;
use ferroui_base::{Ref, data::core::plugins::TaskValue, media::Color, styling::ThemeVariant};
use ferroui_controls::{Control, TopLevel};
use futures_channel::oneshot;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RibbonColorDialogError {
    AlreadyOpen,
    RootNotVisible,
    MissingOverlay,
    CompletionDropped,
    /// Mirrors the source fault from `strings.MoreColors.TrimEnd('.')`.
    NullLocalizedTitle,
    /// The source permits this text, but the selected native text APIs use UTF-8.
    /// Retain the exact value which would be assigned; never substitute text.
    UnsupportedUtf16 {
        property: &'static str,
        units: Vec<u16>,
    },
}
impl std::fmt::Display for RibbonColorDialogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyOpen => f.write_str("Only one color dialog can be open per window."),
            Self::RootNotVisible => f.write_str("The dialog root is not visible."),
            Self::MissingOverlay => f.write_str("The root has no native popup overlay layer."),
            Self::CompletionDropped => {
                f.write_str("The color dialog closed without reporting a result.")
            }
            Self::NullLocalizedTitle => f.write_str("The More Colors dialog title is missing."),
            Self::UnsupportedUtf16 { property, .. } => {
                write!(
                    f,
                    "Unsupported UTF-16 in {property}: the selected FerroUI text API requires UTF-8."
                )
            }
        }
    }
}
impl std::error::Error for RibbonColorDialogError {}
struct DialogLabels {
    title: String,
    primary: Option<String>,
    close: Option<String>,
}
impl DialogLabels {
    fn utf8(units: &[u16], property: &'static str) -> Result<String, RibbonColorDialogError> {
        String::from_utf16(units).map_err(|_| RibbonColorDialogError::UnsupportedUtf16 {
            property,
            units: units.to_vec(),
        })
    }
    fn read(strings: &RibbonStrings) -> Result<Self, RibbonColorDialogError> {
        let title = strings
            .more_colors()
            .ok_or(RibbonColorDialogError::NullLocalizedTitle)?;
        let units = title.as_utf16();
        let end = units
            .iter()
            .rposition(|u| *u != u16::from(b'.'))
            .map_or(0, |i| i + 1);
        // TrimEnd('.') acts on source UTF-16 units before the native boundary.
        let title = Self::utf8(&units[..end], "ContentDialog.Title")?;
        let primary = strings
            .ok()
            .map(|s| Self::utf8(s.as_utf16(), "ContentDialog.PrimaryButtonText"))
            .transpose()?;
        let close = strings
            .cancel()
            .map(|s| Self::utf8(s.as_utf16(), "ContentDialog.CloseButtonText"))
            .transpose()?;
        Ok(Self {
            title,
            primary,
            close,
        })
    }
}
type ResultValue = Result<Option<Color>, RibbonColorDialogError>;
/// Observation of an eagerly opened dialog. Dropping it does not dismiss the
/// visible dialog. Await once, or explicitly use a standard shared future.
pub type RibbonColorDialogFuture = Pin<Box<dyn Future<Output = ResultValue>>>;
ferroui_base::ferro_thread_local! {static ACTIVE:RefCell<Vec<Rc<Session>>>=const{RefCell::new(Vec::new())};}
struct Session {
    shield: Ref<RibbonColorDialogHost>,
    content: Rc<RibbonColorDialogContent>,
    completion: RefCell<Option<oneshot::Sender<ResultValue>>>,
    closing: Cell<bool>,
    scope: RefCell<Option<Rc<RibbonDialogSession>>>,
}
impl Session {
    fn finish(&self, primary: bool) {
        if self.closing.replace(true) {
            return;
        }
        if let Some(scope) = self.scope.take() {
            scope.dismiss();
        }
        ACTIVE.with_borrow_mut(|active| active.retain(|s| !std::ptr::eq(&**s, self)));
        let completion = self.completion.take().expect("one color dialog completion");
        let content = self.content.clone();
        // Original ContentDialog queues its result after Closed. Read the color
        // in that continuation, so Closed handlers may still edit the content.
        self.shield.after_dialog_closed(move || {
            // The observer may have been dropped; closing and cleanup still run.
            let _ = completion.send(Ok(primary.then(|| content.current())));
        });
    }
    #[cfg(test)]
    fn contains_focus(&self) -> bool {
        self.scope
            .borrow()
            .as_ref()
            .is_some_and(|scope| scope.contains_focus())
    }
}
fn open_error(error: RibbonDialogOpenError) -> RibbonColorDialogError {
    match error {
        RibbonDialogOpenError::AlreadyOpen => RibbonColorDialogError::AlreadyOpen,
        RibbonDialogOpenError::RootNotVisible => RibbonColorDialogError::RootNotVisible,
        RibbonDialogOpenError::MissingOverlay => RibbonColorDialogError::MissingOverlay,
    }
}
/// The original public static dialog service. TopLevel maps the source XamlRoot;
/// None theme maps ElementTheme.Default and inherits the owner's actual variant.
pub struct RibbonColorDialog;
impl RibbonColorDialog {
    pub fn show_async(
        root: Option<Ref<TopLevel>>,
        initial: Color,
        theme: Option<ThemeVariant>,
    ) -> RibbonColorDialogFuture {
        let Some(owner) = root else {
            return Box::pin(std::future::ready(Ok(None)));
        };
        let content = RibbonColorDialogContent::new(initial);
        let strings = RibbonStrings::current();
        let labels = match DialogLabels::read(&strings) {
            Ok(labels) => labels,
            Err(error) => {
                return Box::pin(std::future::ready(Err(error)));
            }
        };
        // Source reads labels before ContentDialog.ShowAsync performs open validation.
        let owner = match RibbonDialogSession::preflight(owner) {
            Ok((owner, _)) => owner,
            Err(error) => {
                return Box::pin(std::future::ready(Err(open_error(error))));
            }
        };
        let shield = RibbonColorDialogHost::new();
        shield.set_default_button(crate::compat::ContentDialogButton::Primary);
        shield.set_title(Some(Rc::new(labels.title)));
        shield.set_primary_button_text(labels.primary);
        shield.set_close_button_text(labels.close);
        shield.set_content(Some(Control::boxed(content.root.clone())));
        crate::theming::RibbonTheme::set_theme(Some(&shield), theme).expect("dialog theme");
        let (completion, result) = oneshot::channel();
        let s = Rc::new(Session {
            shield,
            content,
            completion: RefCell::new(Some(completion)),
            closing: Cell::new(false),
            scope: RefCell::default(),
        });
        let weak = Rc::downgrade(&s);
        let scope =
            match RibbonDialogSession::new(owner, s.shield.clone().upcast(), move |primary| {
                if let Some(s) = weak.upgrade() {
                    s.finish(primary == crate::compat::ContentDialogButton::Primary);
                }
            }) {
                Ok(scope) => scope,
                Err(error) => {
                    return Box::pin(std::future::ready(Err(open_error(error))));
                }
            };
        s.scope.replace(Some(scope.clone()));
        ACTIVE.with_borrow_mut(|all| all.push(s));
        scope.show();
        Box::pin(async move {
            result
                .await
                .map_err(|_| RibbonColorDialogError::CompletionDropped)?
        })
    }
}
/// Continue the two production More Colors handlers with their original eager
/// completion, posted start and default-priority native resumption.
pub(crate) fn continue_color_dialog(
    mut future: RibbonColorDialogFuture,
    completed: impl FnOnce(ResultValue) + 'static,
) {
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use std::task::{Context, Poll, Waker};
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => completed(result),
        Poll::Pending => Dispatcher::ui_thread().post_local(
            move || {
                drop(Dispatcher::ui_thread().start_local_with_priority(
                    async move { completed(future.await) },
                    DispatcherPriority::DEFAULT,
                ));
            },
            DispatcherPriority::DEFAULT,
        ),
    }
}
#[cfg(test)]
#[path = "../color_dialog_tests.rs"]
mod tests;

ferroui_base::ferro_static_type!(RibbonColorDialog);
ferroui_base::ferro_markup_type!(static RibbonColorDialog {type_info:RibbonColorDialog,namespace:"FerroRibbon.Controls",methods:[static fn ShowAsync(Option<Ref<TopLevel>>,Color)->TaskValue=>|root,initial|crate::metadata::color_dialog::show_async(root,initial,None),static fn ShowAsync(Option<Ref<TopLevel>>,Color,Option<ThemeVariant>)->TaskValue=>crate::metadata::color_dialog::show_async]});
