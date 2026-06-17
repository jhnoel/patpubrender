use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectError {
    MalformedXml,
    UnsupportedRoot(String),
    UnknownFormat,
    ConflictingVersionMarkers,
}

impl Display for DetectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedXml => f.write_str("malformed XML"),
            Self::UnsupportedRoot(root) => write!(f, "unsupported XML root '{root}'"),
            Self::UnknownFormat => f.write_str("unknown XML format"),
            Self::ConflictingVersionMarkers => f.write_str("conflicting XML version markers"),
        }
    }
}

impl std::error::Error for DetectError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Detect(DetectError),
    UnsupportedStructure(String),
    MalformedXml(String),
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
