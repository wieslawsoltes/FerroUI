use super::{RtfReader, RtfWriter};
use crate::documents::DocumentSnapshot;
use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferItem,
    IAsyncDataTransfer,
};
use std::rc::Rc;

/// The formats offered when rich text is copied to a system clipboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RichEditClipboardFormat {
    #[default]
    AllFormats = 0,
    PlainText = 1,
}
ferroui_base::ferro_markup_enum!(RichEditClipboardFormat { AllFormats, PlainText }, { namespace: "FerroUI.Controls" });

/// Chooses the clipboard representation used by a TOM paste operation.
///
/// Native TOM accepts zero for the best format or a platform clipboard format
/// identifier. Registered RTF identifiers vary by platform and session; this
/// typed policy uses FerroUI's named data formats instead of inventing a number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RichTextPasteFormat {
    /// Exact in-process document content, then RTF, then Unicode plain text.
    #[default]
    BestAvailable,
    /// Unicode plain text only, even when a richer representation is present.
    PlainText,
    /// RTF only. Missing or malformed RTF never falls back to plain text.
    Rtf,
}

pub(super) enum RichTextTransferContent {
    PlainText(String),
    RichText(DocumentSnapshot),
}

impl RichTextTransferContent {
    fn into_snapshot(self) -> DocumentSnapshot {
        match self {
            Self::PlainText(text) => DocumentSnapshot::from_text(&text),
            Self::RichText(snapshot) => snapshot,
        }
    }
}

/// Framework-native clipboard exchange. In-process transfers retain the exact immutable
/// document fragment, including inline controls; external applications receive text and RTF.
pub struct RichTextTransfer;

impl RichTextTransfer {
    pub fn document_format() -> DataFormatOf<DocumentSnapshot> {
        DataFormat::create_in_process_format("FerroUI.RichText.DocumentSnapshot")
    }

    pub fn rtf_formats() -> Vec<DataFormatOf<Rc<[u8]>>> {
        [
            "Rich Text Format",
            "text/rtf",
            "application/rtf",
            "public.rtf",
        ]
        .into_iter()
        .map(DataFormat::create_bytes_platform_format)
        .collect()
    }

    /// Builds one clipboard item containing all applicable formats. If a rich feature has
    /// no RTF representation, the exact in-process fragment and plain text are still offered.
    pub fn create(
        fragment: &DocumentSnapshot,
        format: RichEditClipboardFormat,
    ) -> Rc<dyn IAsyncDataTransfer> {
        Self::create_with_tab_context(fragment, format, None)
    }

    /// Includes RTF using the owning editor's default interval while retaining
    /// the exact original snapshot in the in-process clipboard format.
    pub fn create_with_default_tab_stop(
        fragment: &DocumentSnapshot,
        format: RichEditClipboardFormat,
        default_tab_stop: f64,
    ) -> Rc<dyn IAsyncDataTransfer> {
        Self::create_with_tab_context(fragment, format, Some(default_tab_stop))
    }

    fn create_with_tab_context(
        fragment: &DocumentSnapshot,
        format: RichEditClipboardFormat,
        default_tab_stop: Option<f64>,
    ) -> Rc<dyn IAsyncDataTransfer> {
        let item = DataTransferItem::new();
        // The model's soft line break (U+000B, decision D10) becomes the platform's
        // line terminator in plain text.
        let line_terminator = if cfg!(windows) { "\r\n" } else { "\n" };
        item.set(&DataFormat::text(), Some(fragment.text().replace('\u{000B}', line_terminator)));
        if format == RichEditClipboardFormat::AllFormats {
            item.set(&Self::document_format(), Some(fragment.clone()));
            let rtf = default_tab_stop.map_or_else(
                || RtfWriter::write(fragment),
                |interval| RtfWriter::write_with_default_tab_stop(fragment, interval),
            );
            if let Ok(rtf) = rtf {
                let rtf: Rc<[u8]> = Rc::from(rtf.into_bytes());
                for format in Self::rtf_formats() {
                    item.set(&format, Some(rtf.clone()));
                }
            }
        }
        let transfer = DataTransfer::new();
        transfer.add(item);
        transfer
    }

    /// Reads the highest-fidelity offered format. A malformed advertised RTF payload is
    /// reported to the caller instead of silently replacing formatted content with text.
    pub async fn read(
        data: &dyn IAsyncDataTransfer,
        plain_text_only: bool,
    ) -> Result<Option<DocumentSnapshot>, ClipboardError> {
        Self::read_with_format(
            data,
            if plain_text_only {
                RichTextPasteFormat::PlainText
            } else {
                RichTextPasteFormat::BestAvailable
            },
        )
        .await
    }

    /// Imports external RTF for insertion into an existing editor. Its document
    /// default is replaced by the destination's interval; explicit stops remain.
    /// Exact in-process fragments retain their paragraph formatting unchanged.
    pub async fn read_for_insertion(
        data: &dyn IAsyncDataTransfer,
        plain_text_only: bool,
        default_tab_stop: f64,
    ) -> Result<Option<DocumentSnapshot>, ClipboardError> {
        Self::read_with_format_for_insertion(
            data,
            if plain_text_only {
                RichTextPasteFormat::PlainText
            } else {
                RichTextPasteFormat::BestAvailable
            },
            default_tab_stop,
        )
        .await
    }

    /// Checks advertised formats without requesting or decoding their payloads.
    /// A true result does not promise that a later platform read will succeed or
    /// that an advertised RTF payload is well formed.
    pub fn can_read(
        data: &dyn IAsyncDataTransfer,
        format: RichTextPasteFormat,
    ) -> Result<bool, ClipboardError> {
        let offered = data.try_formats()?;
        let has = |format: &DataFormat| offered.iter().any(|candidate| candidate == format);
        Ok(match format {
            RichTextPasteFormat::BestAvailable => {
                has(&Self::document_format())
                    || Self::rtf_formats().iter().any(|format| has(format))
                    || has(&DataFormat::text())
            }
            RichTextPasteFormat::PlainText => has(&DataFormat::text()),
            RichTextPasteFormat::Rtf => Self::rtf_formats().iter().any(|format| has(format)),
        })
    }

    /// Reads exactly the requested representation policy. The caller retains
    /// ownership of `data` and is responsible for disposing a retrieved transfer.
    pub async fn read_with_format(
        data: &dyn IAsyncDataTransfer,
        format: RichTextPasteFormat,
    ) -> Result<Option<DocumentSnapshot>, ClipboardError> {
        Ok(Self::read_content(data, format, None)
            .await?
            .map(RichTextTransferContent::into_snapshot))
    }

    /// Reads a chosen format for an existing editor, applying its default tab
    /// interval to external RTF while retaining explicit paragraph tab stops.
    pub async fn read_with_format_for_insertion(
        data: &dyn IAsyncDataTransfer,
        format: RichTextPasteFormat,
        default_tab_stop: f64,
    ) -> Result<Option<DocumentSnapshot>, ClipboardError> {
        Ok(Self::read_content(data, format, Some(default_tab_stop))
            .await?
            .map(RichTextTransferContent::into_snapshot))
    }

    pub(super) async fn read_content(
        data: &dyn IAsyncDataTransfer,
        format: RichTextPasteFormat,
        default_tab_stop: Option<f64>,
    ) -> Result<Option<RichTextTransferContent>, ClipboardError> {
        if default_tab_stop.is_some_and(|interval| !interval.is_finite() || interval <= 0.0) {
            return Err(ClipboardError::other(
                "Invalid destination default tab spacing",
            ));
        }
        if format == RichTextPasteFormat::BestAvailable {
            if let Some(snapshot) = data.try_get_value_async(&Self::document_format()).await? {
                snapshot.validate().map_err(|error| ClipboardError::other(error.to_string()))?;
                return Ok(Some(RichTextTransferContent::RichText(snapshot)));
            }
        }
        if format != RichTextPasteFormat::PlainText {
            for format in Self::rtf_formats() {
                if let Some(bytes) = data.try_get_value_async(&format).await? {
                    let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
                    let mut snapshot = RtfReader::read(bytes)
                        .map_err(|error| ClipboardError::other(error.to_string()))?;
                    if let Some(interval) = default_tab_stop {
                        for paragraph in &mut snapshot.paragraphs {
                            paragraph.format.default_tab_stop = Some(interval);
                        }
                    }
                    return Ok(Some(RichTextTransferContent::RichText(snapshot)));
                }
            }
        }
        if format == RichTextPasteFormat::Rtf {
            return Ok(None);
        }
        Ok(data
            .try_get_text_async()
            .await?
            .map(RichTextTransferContent::PlainText))
    }
}


#[cfg(test)]
#[path = "rich_text_link_transfer_tests.rs"]
mod link_field_tests;
