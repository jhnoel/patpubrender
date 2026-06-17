use crate::model::opaque::{OpaqueBlock, XmlAttribute};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BibliographicInformation {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<BibliographicPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BibliographicPart {
    DocumentId(DocumentId),
    PublicationFilingType(String),
    DomesticFilingData(DomesticFilingData),
    TechnicalInformation(TechnicalInformation),
    ContinuityData(ContinuityData),
    Applicants(NamedParties),
    Assignees(NamedParties),
    Inventors(Inventors),
    CorrespondenceAddress(CorrespondenceAddress),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentId {
    pub doc_number: String,
    pub kind_code: Option<String>,
    pub document_date: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DomesticFilingData {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DomesticFilingPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomesticFilingPart {
    ApplicationNumber(ApplicationNumber),
    ApplicationNumberSeriesCode(String),
    FilingDate(String),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TechnicalInformation {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<TechnicalInformationPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TechnicalInformationPart {
    ClassificationIpc(ClassificationIpc),
    ClassificationUs(ClassificationUs),
    TitleOfInvention(String),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContinuityData {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<ContinuityPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuityPart {
    NonProvisionalOfProvisional(RelatedDocument),
    RelatedDocument(RelatedDocument),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NamedParties {
    pub attributes: Vec<XmlAttribute>,
    pub parties: Vec<NamedParty>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NamedParty {
    pub attributes: Vec<XmlAttribute>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventors {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<InventorsPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventorsPart {
    FirstNamedInventor(Inventor),
    Inventor(Inventor),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CorrespondenceAddress {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<CorrespondenceAddressPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrespondenceAddressPart {
    Name1(String),
    Name2(String),
    Address(Address),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplicationNumber {
    pub attributes: Vec<XmlAttribute>,
    pub appl_type: Option<String>,
    pub doc_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassificationIpc {
    pub attributes: Vec<XmlAttribute>,
    pub main_classification: Option<String>,
    pub further_classification: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassificationUs {
    pub attributes: Vec<XmlAttribute>,
    pub national_classification: Option<String>,
    pub further_classification: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventor {
    pub attributes: Vec<XmlAttribute>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelatedDocument {
    pub attributes: Vec<XmlAttribute>,
    pub parent_doc_number: Option<String>,
    pub parent_date: Option<String>,
    pub child_doc_number: Option<String>,
    pub relationship: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Address {
    pub attributes: Vec<XmlAttribute>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub lines: Vec<String>,
}
