use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectError {
    MalformedXml,
    UnsupportedRoot(String),
    UnknownFormat,
    ConflictingVersionMarkers,
}

impl DetectError {
    pub fn is_supplemental_root(&self) -> bool {
        matches!(self, Self::UnsupportedRoot(root) if is_supplemental_xml_root(root))
    }
}

impl Display for DetectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedXml => f.write_str("malformed XML"),
            Self::UnsupportedRoot(root) if is_supplemental_xml_root(root) => {
                write!(f, "supplemental XML root '{root}'")
            }
            Self::UnsupportedRoot(root) => write!(f, "unsupported XML root '{root}'"),
            Self::UnknownFormat => f.write_str("unknown XML format"),
            Self::ConflictingVersionMarkers => f.write_str("conflicting XML version markers"),
        }
    }
}

impl std::error::Error for DetectError {}

fn is_supplemental_xml_root(root: &str) -> bool {
    matches!(root, "sequence-cwu" | "table")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Detect(DetectError),
    UnsupportedStructure(String),
    MalformedXml(String),
}

impl ParseError {
    pub fn is_supplemental_root(&self) -> bool {
        matches!(self, Self::Detect(error) if error.is_supplemental_root())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Detect(error) => write!(f, "source detection failed: {error}"),
            Self::UnsupportedStructure(message) => f.write_str(message),
            Self::MalformedXml(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<DetectError> for ParseError {
    fn from(value: DetectError) -> Self {
        Self::Detect(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializeError {
    UnsupportedFormat(String),
}

impl Display for SerializeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFormat(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SerializeError {}
