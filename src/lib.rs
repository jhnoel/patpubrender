//! USPTO patent document parsing and compact Markdown rendering.
//!
//! The default build is the renderer: parse USPTO grant/application XML (and
//! Green Book "APS" plain text) into the canonical [`PatentDocument`] model, and
//! render it to Markdown. It depends only on `xmloxide`.
//!
//! Two optional tiers extend it:
//! - feature `shard` — [`shard`], the addressable zstd archive codec (write
//!   frames + index, read a frame back by pointer). Pulls in `zstd`.
//! - feature `ingest` — [`ingest`], bulk rendering of USPTO weekly ZIPs into
//!   shards. Implies `shard`; pulls in `zip` + `rayon`.

pub mod error;
pub mod extract;
mod json;
pub mod model;
pub mod render;
mod source;

#[cfg(feature = "shard")]
pub mod shard;

#[cfg(feature = "ingest")]
pub mod ingest;

use error::{ParseError, SerializeError};
use model::document::PatentDocument;
pub use render::markdown::{render_markdown, render_markdown_with_template};
pub use render::template::{DEFAULT_TEMPLATE, Template, TemplateError};
use source::aps::UsptoGrantApsGreenBookAdapter;
pub use source::detect::{SourceFormat, detect_source_format};
use source::traits::FormatAdapter;
use source::uspto::application_v15::UsptoApplicationV15Adapter;
use source::uspto::application_v16::UsptoApplicationV16Adapter;
use source::uspto::application_v40::UsptoApplicationV40Adapter;
use source::uspto::application_v41::UsptoApplicationV41Adapter;
use source::uspto::application_v42::UsptoApplicationV42Adapter;
use source::uspto::application_v43::UsptoApplicationV43Adapter;
use source::uspto::application_v44::UsptoApplicationV44Adapter;
use source::uspto::application_v45::UsptoApplicationV45Adapter;
use source::uspto::application_v46::UsptoApplicationV46Adapter;
use source::uspto::grant_v25::UsptoGrantV25Adapter;
use source::uspto::grant_v40::UsptoGrantV40Adapter;
use source::uspto::grant_v41::UsptoGrantV41Adapter;
use source::uspto::grant_v42::UsptoGrantV42Adapter;
use source::uspto::grant_v43::UsptoGrantV43Adapter;
use source::uspto::grant_v44::UsptoGrantV44Adapter;
use source::uspto::grant_v45::UsptoGrantV45Adapter;
use source::uspto::grant_v46::UsptoGrantV46Adapter;
use source::uspto::grant_v47::UsptoGrantV47Adapter;

fn parse_adapter(format: SourceFormat) -> &'static dyn FormatAdapter {
    match format {
        SourceFormat::UsptoApplicationV15 => &UsptoApplicationV15Adapter,
        SourceFormat::UsptoApplicationV16 => &UsptoApplicationV16Adapter,
        SourceFormat::UsptoApplicationV40 => &UsptoApplicationV40Adapter,
        SourceFormat::UsptoApplicationV41 => &UsptoApplicationV41Adapter,
        SourceFormat::UsptoApplicationV42 => &UsptoApplicationV42Adapter,
        SourceFormat::UsptoApplicationV43 => &UsptoApplicationV43Adapter,
        SourceFormat::UsptoApplicationV44 => &UsptoApplicationV44Adapter,
        SourceFormat::UsptoApplicationV45 => &UsptoApplicationV45Adapter,
        SourceFormat::UsptoApplicationV46 => &UsptoApplicationV46Adapter,
        SourceFormat::UsptoGrantV25 => &UsptoGrantV25Adapter,
        SourceFormat::UsptoGrantV40 => &UsptoGrantV40Adapter,
        SourceFormat::UsptoGrantV41 => &UsptoGrantV41Adapter,
        SourceFormat::UsptoGrantV42 => &UsptoGrantV42Adapter,
        SourceFormat::UsptoGrantV43 => &UsptoGrantV43Adapter,
        SourceFormat::UsptoGrantV44 => &UsptoGrantV44Adapter,
        SourceFormat::UsptoGrantV45 => &UsptoGrantV45Adapter,
        SourceFormat::UsptoGrantV46 => &UsptoGrantV46Adapter,
        SourceFormat::UsptoGrantV47 => &UsptoGrantV47Adapter,
        SourceFormat::UsptoGrantApsGreenBook => &UsptoGrantApsGreenBookAdapter,
    }
}

fn write_adapter(format: SourceFormat) -> &'static dyn FormatAdapter {
    match format {
        SourceFormat::UsptoApplicationV15 => &UsptoApplicationV15Adapter,
        SourceFormat::UsptoApplicationV16 => &UsptoApplicationV16Adapter,
        SourceFormat::UsptoApplicationV40 => &UsptoApplicationV40Adapter,
        SourceFormat::UsptoApplicationV41 => &UsptoApplicationV41Adapter,
        SourceFormat::UsptoApplicationV42 => &UsptoApplicationV42Adapter,
        SourceFormat::UsptoApplicationV43 => &UsptoApplicationV43Adapter,
        SourceFormat::UsptoApplicationV44 => &UsptoApplicationV44Adapter,
        SourceFormat::UsptoApplicationV45 => &UsptoApplicationV45Adapter,
        SourceFormat::UsptoApplicationV46 => &UsptoApplicationV46Adapter,
        SourceFormat::UsptoGrantV25 => &UsptoGrantV25Adapter,
        SourceFormat::UsptoGrantV40 => &UsptoGrantV40Adapter,
        SourceFormat::UsptoGrantV41 => &UsptoGrantV41Adapter,
        SourceFormat::UsptoGrantV42 => &UsptoGrantV42Adapter,
        SourceFormat::UsptoGrantV43 => &UsptoGrantV43Adapter,
        SourceFormat::UsptoGrantV44 => &UsptoGrantV44Adapter,
        SourceFormat::UsptoGrantV45 => &UsptoGrantV45Adapter,
        SourceFormat::UsptoGrantV46 => &UsptoGrantV46Adapter,
        SourceFormat::UsptoGrantV47 => &UsptoGrantV47Adapter,
        SourceFormat::UsptoGrantApsGreenBook => &UsptoGrantApsGreenBookAdapter,
    }
}

/// Parse USPTO patent XML, auto-detecting the schema version.
pub fn parse_patent_xml(input: &str) -> Result<PatentDocument, ParseError> {
    let format = detect_source_format(input)?;
    parse_patent_xml_with_format(input, format)
}

/// Parse a single USPTO Green Book "APS" plain-text patent grant record into
/// the shared [`PatentDocument`] model. This is the non-XML sibling of
/// [`parse_patent_xml`]; everything downstream (Markdown render, biblio sidecar)
/// is identical.
pub fn parse_patent_aps(input: &str) -> Result<PatentDocument, ParseError> {
    UsptoGrantApsGreenBookAdapter.parse_document(input)
}

/// Re-export of the APS record-boundary scanner for the shard streamer.
pub use source::aps::{aps_record_starts, next_aps_record_start};

/// Parse USPTO patent XML with an explicit [`SourceFormat`].
pub fn parse_patent_xml_with_format(
    input: &str,
    format: SourceFormat,
) -> Result<PatentDocument, ParseError> {
    parse_adapter(format).parse_document(input)
}

/// Round-trip serialize a [`PatentDocument`] back to its source XML.
pub fn write_patent_xml(doc: &PatentDocument) -> Result<String, SerializeError> {
    write_adapter(doc.source_format).write_document(doc)
}
