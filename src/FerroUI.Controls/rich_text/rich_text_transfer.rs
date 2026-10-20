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
        item.set(&DataFormat::text(), Some(fragment.text()));
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
        Self::read_with_tab_context(data, plain_text_only, None).await
    }

    /// Imports external RTF for insertion into an existing editor. Its document
    /// default is replaced by the destination's interval; explicit stops remain.
    /// Exact in-process fragments retain their paragraph formatting unchanged.
    pub async fn read_for_insertion(
        data: &dyn IAsyncDataTransfer,
        plain_text_only: bool,
        default_tab_stop: f64,
    ) -> Result<Option<DocumentSnapshot>, ClipboardError> {
        if !default_tab_stop.is_finite() || default_tab_stop <= 0.0 {
            return Err(ClipboardError::other(
                "Invalid destination default tab spacing",
            ));
        }
        Self::read_with_tab_context(data, plain_text_only, Some(default_tab_stop)).await
    }

    async fn read_with_tab_context(
        data: &dyn IAsyncDataTransfer,
        plain_text_only: bool,
        default_tab_stop: Option<f64>,
    ) -> Result<Option<DocumentSnapshot>, ClipboardError> {
        if !plain_text_only {
            if let Some(snapshot) = data.try_get_value_async(&Self::document_format()).await? {
                return Ok(Some(snapshot));
            }
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
                    return Ok(Some(snapshot));
                }
            }
        }
        Ok(data
            .try_get_text_async()
            .await?
            .map(|text| DocumentSnapshot::from_text(&text)))
    }
}
