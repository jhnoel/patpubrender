use crate::error::{ParseError, SerializeError};
use crate::model::document::PatentDocument;

pub trait FormatAdapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError>;
    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError>;
}
