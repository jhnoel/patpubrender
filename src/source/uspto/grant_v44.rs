use crate::error::{ParseError, SerializeError};
use crate::model::document::PatentDocument;
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;

use super::grant_v4x;

pub struct UsptoGrantV44Adapter;

impl FormatAdapter for UsptoGrantV44Adapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        grant_v4x::parse_document(input, SourceFormat::UsptoGrantV44)
    }

    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError> {
        grant_v4x::write_document(doc)
    }
}
