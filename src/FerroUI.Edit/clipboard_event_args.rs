use ferroui_base::ferro_routed_event_args;
use ferroui_base::input::{DataFormat, DataTransfer};
use ferroui_base::interactivity::{CancelRoutedEventArgs, RoutedEvent, RoutedEventArgs};
use std::cell::RefCell;
use std::rc::Rc;

/// How plain clipboard text should be inserted. Marker formats carry this
/// intent separately. Whole-line insertion normalizes line endings and tabs
/// according to the destination document and editor options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ClipboardSelectionKind {
    #[default]
    Plain,
    Rectangular,
    WholeLine,
}

ferroui_base::ferro_markup_enum!(ClipboardSelectionKind { Plain, Rectangular, WholeLine }, { namespace: "FerroUI.Edit" });

/// Raised before a prepared copy or cut is sent to the platform clipboard.
/// Handlers can change any format on the data items, replace the complete data
/// transfer, or cancel the operation. Clones share their cancellation and data.
#[derive(Clone)]
pub struct CopyingToClipboardEventArgs {
    base: CancelRoutedEventArgs,
    data: Rc<RefCell<Rc<DataTransfer>>>,
    is_cut: bool,
    kind: ClipboardSelectionKind,
    formatting_error: Option<String>,
}

ferro_routed_event_args!(CopyingToClipboardEventArgs: CancelRoutedEventArgs);

impl CopyingToClipboardEventArgs {
    pub fn new<T: ?Sized>(
        event: &RoutedEvent<T>,
        data: Rc<DataTransfer>,
        is_cut: bool,
        kind: ClipboardSelectionKind,
        formatting_error: Option<String>,
    ) -> Self {
        Self {
            base: CancelRoutedEventArgs::with_event(event),
            data: Rc::new(RefCell::new(data)),
            is_cut,
            kind,
            formatting_error,
        }
    }

    pub fn data(&self) -> Rc<DataTransfer> {
        self.data.borrow().clone()
    }

    pub fn set_data(&self, data: Rc<DataTransfer>) {
        let old = self.data.replace(data);
        drop(old);
    }

    pub fn is_cut(&self) -> bool {
        self.is_cut
    }

    pub fn kind(&self) -> ClipboardSelectionKind {
        self.kind
    }

    /// A rich-format error does not prevent plain text from being copied.
    pub fn formatting_error(&self) -> Option<&str> {
        self.formatting_error.as_deref()
    }

    pub fn text(&self) -> Option<String> {
        self.data()
            .items()
            .iter()
            .find_map(|item| item.try_get_value(&DataFormat::text()))
    }
}

/// Raised only after the platform has accepted a clipboard write.
#[derive(Clone)]
pub struct TextCopiedEventArgs {
    base: RoutedEventArgs,
    text: Option<String>,
    is_cut: bool,
}

ferro_routed_event_args!(TextCopiedEventArgs: RoutedEventArgs);

impl TextCopiedEventArgs {
    pub fn new<T: ?Sized>(event: &RoutedEvent<T>, text: Option<String>, is_cut: bool) -> Self {
        Self {
            base: RoutedEventArgs::with_event(event),
            text,
            is_cut,
        }
    }

    /// The prepared transfer's plain text, if a customization retained it.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// The user's operation. A cut can still decline deletion if the document,
    /// selection or edit policy changes while the clipboard write is pending.
    pub fn is_cut(&self) -> bool {
        self.is_cut
    }
}

/// Raised after clipboard text is read and before any document mutation.
#[derive(Clone)]
pub struct PastingFromClipboardEventArgs {
    base: CancelRoutedEventArgs,
    payload: Rc<RefCell<(String, ClipboardSelectionKind)>>,
}

ferro_routed_event_args!(PastingFromClipboardEventArgs: CancelRoutedEventArgs);

impl PastingFromClipboardEventArgs {
    pub fn new<T: ?Sized>(
        event: &RoutedEvent<T>,
        text: String,
        kind: ClipboardSelectionKind,
    ) -> Self {
        Self {
            base: CancelRoutedEventArgs::with_event(event),
            payload: Rc::new(RefCell::new((text, kind))),
        }
    }

    pub fn text(&self) -> String {
        self.payload.borrow().0.clone()
    }

    pub fn set_text(&self, text: String) {
        let old = std::mem::replace(&mut self.payload.borrow_mut().0, text);
        drop(old);
    }

    pub fn kind(&self) -> ClipboardSelectionKind {
        self.payload.borrow().1
    }

    pub fn set_kind(&self, kind: ClipboardSelectionKind) {
        self.payload.borrow_mut().1 = kind;
    }
}

/// Raised after an accepted paste has completed its single document transaction.
#[derive(Clone)]
pub struct TextPastedEventArgs {
    base: RoutedEventArgs,
    text: String,
    kind: ClipboardSelectionKind,
}

ferro_routed_event_args!(TextPastedEventArgs: RoutedEventArgs);

impl TextPastedEventArgs {
    pub fn new<T: ?Sized>(
        event: &RoutedEvent<T>,
        text: String,
        kind: ClipboardSelectionKind,
    ) -> Self {
        Self {
            base: RoutedEventArgs::with_event(event),
            text,
            kind,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn kind(&self) -> ClipboardSelectionKind {
        self.kind
    }
}
