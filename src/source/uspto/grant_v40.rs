use crate::error::{ParseError, SerializeError};
use crate::model::document::PatentDocument;
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;

use super::grant_v4x;

pub struct UsptoGrantV40Adapter;

impl FormatAdapter for UsptoGrantV40Adapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        grant_v4x::parse_document(input, SourceFormat::UsptoGrantV40)
    }

    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError> {
        grant_v4x::write_document(doc)
    }
}
