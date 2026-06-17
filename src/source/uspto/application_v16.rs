use crate::error::{ParseError, SerializeError};
use crate::model::document::PatentDocument;
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;

use super::application_v15;

pub struct UsptoApplicationV16Adapter;

impl FormatAdapter for UsptoApplicationV16Adapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        application_v15::parse_document(input, SourceFormat::UsptoApplicationV16)
    }

    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError> {
        Ok(application_v15::write_document(doc))
    }
}
