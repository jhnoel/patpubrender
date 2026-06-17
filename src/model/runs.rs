use crate::model::opaque::{OpaqueInline, XmlAttribute};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InlineContainer {
    pub attributes: Vec<XmlAttribute>,
    pub content: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Number {
    pub attributes: Vec<XmlAttribute>,
    pub content: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrossReference {
    pub attributes: Vec<XmlAttribute>,
    pub target: Option<String>,
    pub content: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DependentClaimReference {
    pub attributes: Vec<XmlAttribute>,
    pub depends_on: Option<String>,
    pub content: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    Text(String),
    Number(Number),
    CrossReference(CrossReference),
    FigureReference(CrossReference),
    DependentClaimReference(DependentClaimReference),
    Bold(InlineContainer),
    Italic(InlineContainer),
    Highlight(InlineContainer),
    ClaimText(InlineContainer),
    Opaque(OpaqueInline),
}
