use crate::model::bibliographic::BibliographicInformation;
use crate::model::claims::Claims;
use crate::model::description::{AbstractSection, Description, Drawings};
use crate::model::opaque::{OpaqueBlock, XmlAttribute, XmlProlog};
use crate::source::detect::SourceFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatentDocument {
    pub source_format: SourceFormat,
    pub prolog: XmlProlog,
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DocumentPart>,
}

impl Default for PatentDocument {
    fn default() -> Self {
        Self {
            source_format: SourceFormat::UsptoApplicationV15,
            prolog: XmlProlog::default(),
            attributes: vec![],
            parts: vec![],
        }
    }
}

pub type PatentApplicationPublication = PatentDocument;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentPart {
    BibliographicInformation(BibliographicInformation),
    AbstractSection(AbstractSection),
    Description(Description),
    Claims(Claims),
    Drawings(Drawings),
    Opaque(OpaqueBlock),
}
