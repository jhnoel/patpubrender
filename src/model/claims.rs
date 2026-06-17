use crate::model::opaque::{OpaqueBlock, XmlAttribute};
use crate::model::runs::Run;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Claims {
    pub attributes: Vec<XmlAttribute>,
    pub parts: Vec<ClaimsPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimsPart {
    Heading(Heading),
    Claim(Claim),
    Opaque(OpaqueBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Claim {
    pub attributes: Vec<XmlAttribute>,
    pub id: Option<String>,
    pub content: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Heading {
    pub attributes: Vec<XmlAttribute>,
    pub level: Option<u32>,
    pub align: Option<String>,
    pub content: Vec<Run>,
}
