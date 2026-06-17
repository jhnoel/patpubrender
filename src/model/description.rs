use crate::model::claims::Heading;
use crate::model::opaque::{OpaqueBlock, XmlAttribute};
use crate::model::runs::Run;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AbstractSection {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<AbstractPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbstractPart {
    Paragraph(Paragraph),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Description {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DescriptionPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptionPart {
    CrossReferenceToRelatedApplications(CrossReferenceToRelatedApplications),
    SummaryOfInvention(SummaryOfInvention),
    BriefDescriptionOfDrawings(BriefDescriptionOfDrawings),
    DetailedDescription(DetailedDescription),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrossReferenceToRelatedApplications {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<CrossReferencePart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossReferencePart {
    Heading(Heading),
    Paragraph(Paragraph),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SummaryOfInvention {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DescriptionSectionPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BriefDescriptionOfDrawings {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DescriptionSectionPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetailedDescription {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DescriptionSectionPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptionSectionPart {
    Section(Section),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Section {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<SectionPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionPart {
    Heading(Heading),
    Paragraph(Paragraph),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub attributes: Vec<XmlAttribute>,
    pub id: Option<String>,
    pub level: Option<u32>,
    pub content: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Drawings {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<DrawingsPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawingsPart {
    Heading(Heading),
    RepresentativeFigure(String),
    Figure(Figure),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Figure {
    pub attributes: Vec<XmlAttribute>,
    pub id: Option<String>,
    pub parts: Vec<FigurePart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigurePart {
    Image(Image),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Image {
    pub attributes: Vec<XmlAttribute>,
    pub id: Option<String>,
    pub file: Option<String>,
    pub imf: Option<String>,
    pub ti: Option<String>,
}
