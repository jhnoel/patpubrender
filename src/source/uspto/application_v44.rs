use crate::error::{ParseError, SerializeError};
use crate::model::document::PatentDocument;
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;

use super::application_v4x;

pub struct UsptoApplicationV44Adapter;

impl FormatAdapter for UsptoApplicationV44Adapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        application_v4x::parse_document(input, SourceFormat::UsptoApplicationV44)
    }

    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError> {
        application_v4x::write_document(doc)
    }
}
