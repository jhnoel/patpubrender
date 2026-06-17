//! Structured field extraction from a parsed [`PatentDocument`].
//!
//! Metadata (numbers, dates, parties, classifications) is available via
//! [`crate::render::biblio::extract_biblio`]. This module adds the document-body
//! extractors — claims and abstract as plain text — that the biblio record does
//! not carry. Together they back the structured access surface (e.g. the Python
//! SDK) without exposing the raw parse tree.

use crate::model::claims::ClaimsPart;
use crate::model::description::AbstractPart;
use crate::model::document::{DocumentPart, PatentDocument};
use crate::render::markdown::flatten_runs_plain;

/// A single claim as plain text, numbered by its 1-based position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimText {
    pub number: usize,
    pub text: String,
}

/// Every claim in the document, in source order, flattened to plain text.
pub fn claims(document: &PatentDocument) -> Vec<ClaimText> {
    let mut out = Vec::new();
    for part in &document.parts {
        if let DocumentPart::Claims(claims) = part {
            for claim_part in &claims.parts {
                if let ClaimsPart::Claim(claim) = claim_part {
                    let text = flatten_runs_plain(&claim.content).trim().to_string();
                    if !text.is_empty() {
                        out.push(ClaimText {
                            number: out.len() + 1,
                            text,
                        });
                    }
                }
            }
        }
    }
    out
}

/// The abstract as plain text, paragraphs joined by a blank line.
pub fn abstract_text(document: &PatentDocument) -> Option<String> {
    for part in &document.parts {
        if let DocumentPart::AbstractSection(section) = part {
            let text = section
                .parts
                .iter()
                .filter_map(|p| match p {
                    AbstractPart::Paragraph(paragraph) => {
                        Some(flatten_runs_plain(&paragraph.content))
                    }
                    AbstractPart::Opaque(_) => None,
                })
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }
    None
}
