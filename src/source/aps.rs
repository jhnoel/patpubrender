//! USPTO Green Book "APS" plain-text patent grants (product PTGRAPS, 1976-2001).
//!
//! These weekly files predate the XML grant formats. Each file holds many
//! concatenated patent records; a record begins with a line that is exactly
//! `PATN`. Within a record, every data line carries a left-justified tag in
//! columns 1-4 and a value beginning at column 6. Continuation lines have a
//! blank tag (leading spaces) and continue the previous field's value, joined
//! with a single space. Section tags (e.g. `INVT`, `CLAS`, `CLMS`) introduce
//! groups, and their sub-tags belong to the most recently opened section.
//!
//! This adapter parses one record into the shared [`PatentDocument`] model so
//! the existing Markdown + biblio render pipeline can consume it unchanged.

use crate::error::{ParseError, SerializeError};
use crate::model::bibliographic::{
    ApplicationNumber, BibliographicInformation, BibliographicPart, ClassificationIpc,
    ClassificationUs, DocumentId, DomesticFilingData, DomesticFilingPart, Inventor, Inventors,
    InventorsPart, NamedParties, NamedParty, TechnicalInformation, TechnicalInformationPart,
};
use crate::model::claims::{Claim, Claims, ClaimsPart};
use crate::model::description::{
    AbstractPart, AbstractSection, BriefDescriptionOfDrawings, Description, DescriptionPart,
    DescriptionSectionPart, DetailedDescription, Paragraph, Section, SectionPart,
    SummaryOfInvention,
};
use crate::model::document::{DocumentPart, PatentDocument};
use crate::model::runs::{InlineContainer, Run};
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;

pub struct UsptoGrantApsGreenBookAdapter;

impl FormatAdapter for UsptoGrantApsGreenBookAdapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        parse_aps_document(input)
    }

    fn write_document(&self, _doc: &PatentDocument) -> Result<String, SerializeError> {
        // The Green Book is a read-only legacy ingest path; pdp renders it but
        // never re-serializes it. No round-trip is defined for this format.
        Err(SerializeError::UnsupportedFormat(
            "UsptoGrantApsGreenBook does not support serialization".to_string(),
        ))
    }
}

/// A single tagged field plus the section it belongs to.
#[derive(Debug, Clone)]
struct Field {
    /// The most recent section tag, e.g. "INVT", "CLAS", "CLMS". Empty for the
    /// `PATN` section's own scalar fields.
    section: String,
    tag: String,
    value: String,
}

/// Split a record's lines into section-scoped fields, unwrapping continuation
/// lines (blank tag, indented text) into the preceding field's value.
fn collect_fields(record: &str) -> Vec<Field> {
    let section_tags = [
        "PATN", "INVT", "ASSG", "PRIR", "REIS", "RLAP", "PARN", "CLAS", "UREF", "FREF", "OREF",
        "LREP", "PCTA", "ABST", "GOVT", "BSUM", "DRWD", "DETD", "CLMS", "DCLM",
    ];

    let mut fields: Vec<Field> = Vec::new();
    // The record's leading `PATN` marker is stripped by `strip_record_header`,
    // so the top-level scalar fields (WKU/ISD/TTL/...) appear before any other
    // section tag; treat them as belonging to the `PATN` section.
    let mut current_section = String::from("PATN");

    for line in record.lines() {
        if line.trim().is_empty() {
            // Blank lines inside preformatted blocks (TBL/EQU) are significant;
            // append them to the active field's value verbatim.
            if let Some(last) = fields.last_mut()
                && is_verbatim_tag(&last.tag)
            {
                last.value.push('\n');
            }
            continue;
        }

        let tag = line_tag(line);
        match tag {
            // Continuation line: blank tag, value continues the previous field.
            None => {
                if let Some(last) = fields.last_mut() {
                    if is_verbatim_tag(&last.tag) {
                        // Preserve the table/equation layout verbatim (keep the
                        // original column padding, drop only trailing blanks).
                        last.value.push('\n');
                        last.value.push_str(line.get(5..).unwrap_or("").trim_end());
                    } else {
                        // Prose continuation: the text is indented past column 6;
                        // trim that indent and join with a single space.
                        let cont = line_value(line).trim_start();
                        if last.value.is_empty() {
                            last.value.push_str(cont);
                        } else {
                            last.value.push(' ');
                            last.value.push_str(cont);
                        }
                    }
                }
            }
            Some(tag) => {
                if section_tags.contains(&tag) {
                    current_section = tag.to_string();
                    // `PATN`/section markers carry no value of their own; scalar
                    // fields follow on subsequent lines.
                    if tag == "PATN" {
                        // A new record should never appear mid-record, but be safe.
                        continue;
                    }
                    // Some section markers (INVT/ASSG/UREF...) introduce a fresh
                    // repeatable group; we still push a marker-less boundary by
                    // recording the section change implicitly via current_section.
                    continue;
                }

                fields.push(Field {
                    section: current_section.clone(),
                    tag: tag.to_string(),
                    value: line_value(line).to_string(),
                });
            }
        }
    }

    fields
}

/// Tags whose content is preformatted (ASCII tables, equations) and must be
/// kept verbatim with original line breaks and spacing.
fn is_verbatim_tag(tag: &str) -> bool {
    matches!(tag, "TBL" | "EQU" | "TBL3")
}

/// The 1-4 column tag if the line carries one, else `None` (continuation line).
fn line_tag(line: &str) -> Option<&str> {
    if line.starts_with(' ') {
        return None;
    }
    let tag_field = line.get(0..4).unwrap_or(line);
    let tag = tag_field.trim_end();
    if tag.is_empty() { None } else { Some(tag) }
}

/// Field value: everything from column 6 onward, trimmed of trailing spaces.
fn line_value(line: &str) -> &str {
    line.get(5..).unwrap_or("").trim_end()
}

fn parse_aps_document(input: &str) -> Result<PatentDocument, ParseError> {
    // Tolerate a leading file header (HHHHHT ... APS1 ... ISSUE) and a leading
    // `PATN` marker before the first field.
    let record = strip_record_header(input);
    let fields = collect_fields(record);

    if fields.is_empty() {
        return Err(ParseError::UnsupportedStructure(
            "APS record has no recognizable tagged fields".to_string(),
        ));
    }

    // A valid grant record must at least carry a patent number (WKU).
    let wku = scalar(&fields, "PATN", "WKU").or_else(|| scalar(&fields, "", "WKU"));
    let Some(wku) = wku else {
        return Err(ParseError::UnsupportedStructure(
            "APS record missing WKU (patent number)".to_string(),
        ));
    };

    let parts = vec![
        DocumentPart::BibliographicInformation(build_bibliographic(&fields, &wku)),
        // Abstract, description, and claims are appended conditionally below.
    ];
    let mut parts = parts;

    if let Some(abstract_section) = build_abstract(&fields) {
        parts.push(DocumentPart::AbstractSection(abstract_section));
    }
    if let Some(description) = build_description(&fields) {
        parts.push(DocumentPart::Description(description));
    }
    if let Some(claims) = build_claims(&fields) {
        parts.push(DocumentPart::Claims(claims));
    }

    Ok(PatentDocument {
        source_format: SourceFormat::UsptoGrantApsGreenBook,
        prolog: Default::default(),
        attributes: vec![],
        parts,
    })
}

/// Drop the optional weekly file header and the leading `PATN` marker so the
/// remainder begins at the first tagged field.
fn strip_record_header(input: &str) -> &str {
    let mut start = 0usize;
    for line in input.lines() {
        let trimmed = line.trim();
        let is_header = trimmed.starts_with("HHHHH") || trimmed == "PATN" || trimmed.is_empty();
        if is_header {
            start += line.len();
            // account for the line terminator that `lines()` stripped
            start += newline_len(&input[start..]);
        } else {
            break;
        }
    }
    &input[start..]
}

fn newline_len(rest: &str) -> usize {
    if rest.starts_with("\r\n") {
        2
    } else if rest.starts_with('\n') || rest.starts_with('\r') {
        1
    } else {
        0
    }
}

// --- Bibliographic --------------------------------------------------------

fn build_bibliographic(fields: &[Field], wku: &str) -> BibliographicInformation {
    let mut parts = Vec::new();

    let number = WkuNumber::parse(wku);
    parts.push(BibliographicPart::DocumentId(DocumentId {
        doc_number: number.doc_number.clone(),
        kind_code: number.kind_code.clone(),
        document_date: scalar(fields, "PATN", "ISD"),
        country_code: Some("US".to_string()),
    }));

    // Domestic filing: application number (APN) + filing date (APD).
    let mut filing_parts = Vec::new();
    if let Some(apn) = scalar(fields, "PATN", "APN") {
        filing_parts.push(DomesticFilingPart::ApplicationNumber(ApplicationNumber {
            attributes: vec![],
            appl_type: scalar(fields, "PATN", "APT"),
            doc_number: Some(normalize_apn(&apn)),
        }));
    }
    // NOTE: The Green Book APN carries a trailing check character (`500649&`,
    // and digit check chars on numeric serials). `normalize_apn` strips a
    // trailing symbol check char but deliberately does NOT strip a trailing
    // digit, because (a) these pre-2001 serials do not join to any downstream
    // record dataset and (b) it is not certain every week appends a digit check
    // char — stripping one risks corrupting a real serial. See `normalize_apn`.
    if let Some(apd) = scalar(fields, "PATN", "APD") {
        filing_parts.push(DomesticFilingPart::FilingDate(apd));
    }
    if !filing_parts.is_empty() {
        parts.push(BibliographicPart::DomesticFilingData(DomesticFilingData {
            attributes: vec![],
            parts: filing_parts,
        }));
    }

    // Technical information: title + classifications.
    let mut tech_parts = Vec::new();
    if let Some(title) = scalar(fields, "PATN", "TTL") {
        tech_parts.push(TechnicalInformationPart::TitleOfInvention(title));
    }
    let ipc = section_values(fields, "CLAS", "ICL");
    if !ipc.is_empty() {
        let (main, rest) = ipc.split_first().unwrap();
        tech_parts.push(TechnicalInformationPart::ClassificationIpc(
            ClassificationIpc {
                attributes: vec![],
                main_classification: Some(normalize_ipc(main)),
                further_classification: rest.iter().map(|value| normalize_ipc(value)).collect(),
            },
        ));
    }
    let us: Vec<String> = section_values(fields, "CLAS", "OCL")
        .into_iter()
        .chain(section_values(fields, "CLAS", "XCL"))
        .map(|value| normalize_us_class(&value))
        .collect();
    if !us.is_empty() {
        let (main, rest) = us.split_first().unwrap();
        tech_parts.push(TechnicalInformationPart::ClassificationUs(
            ClassificationUs {
                attributes: vec![],
                national_classification: Some(main.clone()),
                further_classification: rest.to_vec(),
            },
        ));
    }
    if !tech_parts.is_empty() {
        parts.push(BibliographicPart::TechnicalInformation(
            TechnicalInformation {
                attributes: vec![],
                parts: tech_parts,
            },
        ));
    }

    // Inventors (INVT, repeatable). Each `NAM` opens a new inventor.
    let inventors = build_inventors(fields);
    if !inventors.parts.is_empty() {
        parts.push(BibliographicPart::Inventors(inventors));
    }

    // Assignees (ASSG, repeatable).
    let assignees = build_named_parties(fields, "ASSG");
    if !assignees.parties.is_empty() {
        parts.push(BibliographicPart::Assignees(assignees));
    }

    BibliographicInformation {
        attributes: vec![],
        parts,
    }
}

fn build_inventors(fields: &[Field]) -> Inventors {
    let mut parts = Vec::new();
    let mut current: Option<Inventor> = None;
    let mut first = true;

    for field in fields.iter().filter(|f| f.section == "INVT") {
        match field.tag.as_str() {
            "NAM" => {
                flush_inventor(&mut parts, &mut current, &mut first);
                let (last_name, first_name) = split_aps_name(&field.value);
                current = Some(Inventor {
                    attributes: vec![],
                    first_name,
                    last_name,
                    city: None,
                    state: None,
                    country: None,
                });
            }
            "CTY" => set_inventor_field(&mut current, |inv| inv.city = Some(field.value.clone())),
            "STA" => set_inventor_field(&mut current, |inv| inv.state = Some(field.value.clone())),
            "CNT" => {
                set_inventor_field(&mut current, |inv| inv.country = Some(field.value.clone()))
            }
            _ => {}
        }
    }
    flush_inventor(&mut parts, &mut current, &mut first);

    Inventors {
        attributes: vec![],
        parts,
    }
}

fn set_inventor_field(current: &mut Option<Inventor>, f: impl FnOnce(&mut Inventor)) {
    if let Some(inv) = current.as_mut() {
        f(inv);
    }
}

fn flush_inventor(
    parts: &mut Vec<InventorsPart>,
    current: &mut Option<Inventor>,
    first: &mut bool,
) {
    if let Some(inventor) = current.take() {
        if *first {
            parts.push(InventorsPart::FirstNamedInventor(inventor));
            *first = false;
        } else {
            parts.push(InventorsPart::Inventor(inventor));
        }
    }
}

fn build_named_parties(fields: &[Field], section: &str) -> NamedParties {
    let parties = fields
        .iter()
        .filter(|f| f.section == section && f.tag == "NAM")
        .map(|f| NamedParty {
            attributes: vec![],
            name: Some(f.value.clone()),
        })
        .collect();
    NamedParties {
        attributes: vec![],
        parties,
    }
}

// --- Abstract / description / claims --------------------------------------

fn build_abstract(fields: &[Field]) -> Option<AbstractSection> {
    let parts: Vec<AbstractPart> = fields
        .iter()
        .filter(|f| f.section == "ABST")
        .filter_map(|f| body_paragraph(f).map(AbstractPart::Paragraph))
        .collect();
    (!parts.is_empty()).then_some(AbstractSection {
        attributes: vec![],
        parts,
    })
}

fn build_description(fields: &[Field]) -> Option<Description> {
    let mut parts = Vec::new();

    if let Some(section_parts) = body_sections(fields, "BSUM") {
        parts.push(DescriptionPart::SummaryOfInvention(SummaryOfInvention {
            attributes: vec![],
            parts: section_parts,
        }));
    }
    if let Some(section_parts) = body_sections(fields, "DRWD") {
        parts.push(DescriptionPart::BriefDescriptionOfDrawings(
            BriefDescriptionOfDrawings {
                attributes: vec![],
                parts: section_parts,
            },
        ));
    }
    if let Some(section_parts) = body_sections(fields, "DETD") {
        parts.push(DescriptionPart::DetailedDescription(DetailedDescription {
            attributes: vec![],
            parts: section_parts,
        }));
    }

    (!parts.is_empty()).then_some(Description {
        attributes: vec![],
        parts,
    })
}

/// Build the section/heading structure for a body region. `PAC` lines start a
/// new heading-led section; paragraphs (`PAR`/`PAL`/`PA0`..`PA3`) and verbatim
/// blocks (`TBL`/`EQU`) attach to the current section.
fn body_sections(fields: &[Field], section: &str) -> Option<Vec<DescriptionSectionPart>> {
    let region: Vec<&Field> = fields.iter().filter(|f| f.section == section).collect();
    if region.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut current: Option<Section> = None;

    for field in region {
        match field.tag.as_str() {
            "PAC" => {
                flush_section(&mut parts, &mut current);
                current = Some(Section {
                    attributes: vec![],
                    parts: vec![SectionPart::Heading(crate::model::claims::Heading {
                        attributes: vec![],
                        level: None,
                        align: None,
                        content: vec![Run::Text(field.value.clone())],
                    })],
                });
            }
            _ => {
                if let Some(paragraph) = body_paragraph(field) {
                    let sec = current.get_or_insert_with(|| Section {
                        attributes: vec![],
                        parts: vec![],
                    });
                    sec.parts.push(SectionPart::Paragraph(paragraph));
                }
            }
        }
    }
    flush_section(&mut parts, &mut current);
    Some(parts)
}

fn flush_section(parts: &mut Vec<DescriptionSectionPart>, current: &mut Option<Section>) {
    if let Some(section) = current.take() {
        parts.push(DescriptionSectionPart::Section(section));
    }
}

/// A body text field (`PAR`, `PAL`, `PA0`..`PA3`, `TBL`, `EQU`) as a paragraph.
fn body_paragraph(field: &Field) -> Option<Paragraph> {
    if field.value.is_empty() {
        return None;
    }
    let content = if is_verbatim_tag(&field.tag) {
        // Wrap preformatted tables/equations in a fenced code block so the
        // renderer keeps the layout. The renderer flattens runs to text, so the
        // fences survive as plain text lines.
        vec![Run::Text(format!("```\n{}\n```", field.value))]
    } else {
        vec![Run::Text(field.value.clone())]
    };
    Some(Paragraph {
        attributes: vec![],
        id: None,
        level: paragraph_level(&field.tag),
        content,
    })
}

fn paragraph_level(tag: &str) -> Option<u32> {
    match tag {
        "PA0" => Some(0),
        "PA1" => Some(1),
        "PA2" => Some(2),
        "PA3" => Some(3),
        _ => None,
    }
}

fn build_claims(fields: &[Field]) -> Option<Claims> {
    // Utility/reissue claims live in CLMS (numbered PAR lines); design patents
    // carry a single DCLM claim.
    let mut parts = Vec::new();

    let clms: Vec<&Field> = fields.iter().filter(|f| f.section == "CLMS").collect();
    if !clms.is_empty() {
        // Each `NUM` opens a claim; `PAR`/`PAL`/`PAC` lines are its text. `STM`
        // is the lead-in ("I claim:") and is dropped (it is not a claim).
        let mut current: Option<Vec<Run>> = None;
        for field in clms {
            match field.tag.as_str() {
                "STM" => {}
                "NUM" => {
                    flush_claim(&mut parts, &mut current);
                    current = Some(Vec::new());
                }
                _ => {
                    if !field.value.is_empty() {
                        let runs = current.get_or_insert_with(Vec::new);
                        runs.push(Run::ClaimText(InlineContainer {
                            attributes: vec![],
                            content: vec![Run::Text(field.value.clone())],
                        }));
                    }
                }
            }
        }
        flush_claim(&mut parts, &mut current);
    }

    // Design claim: a single DCLM/PAR.
    let dclm: Vec<Run> = fields
        .iter()
        .filter(|f| f.section == "DCLM" && !f.value.is_empty())
        .map(|f| {
            Run::ClaimText(InlineContainer {
                attributes: vec![],
                content: vec![Run::Text(f.value.clone())],
            })
        })
        .collect();
    if !dclm.is_empty() {
        parts.push(ClaimsPart::Claim(Claim {
            attributes: vec![],
            id: None,
            content: dclm,
        }));
    }

    (!parts.is_empty()).then_some(Claims {
        attributes: vec![],
        parts,
    })
}

fn flush_claim(parts: &mut Vec<ClaimsPart>, current: &mut Option<Vec<Run>>) {
    if let Some(content) = current.take()
        && !content.is_empty()
    {
        parts.push(ClaimsPart::Claim(Claim {
            attributes: vec![],
            id: None,
            content,
        }));
    }
}

// --- Field accessors ------------------------------------------------------

/// First value for a scalar tag within a section.
fn scalar(fields: &[Field], section: &str, tag: &str) -> Option<String> {
    fields
        .iter()
        .find(|f| f.section == section && f.tag == tag)
        .map(|f| f.value.clone())
        .filter(|value| !value.is_empty())
}

/// All values for a repeatable tag within a section.
fn section_values(fields: &[Field], section: &str, tag: &str) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.section == section && f.tag == tag && !f.value.is_empty())
        .map(|f| f.value.clone())
        .collect()
}

// --- Normalization --------------------------------------------------------

/// A parsed WKU patent number: a clean digit string plus a synthesized kind
/// code so the shared renderer classifies the record as a grant.
struct WkuNumber {
    doc_number: String,
    kind_code: Option<String>,
}

impl WkuNumber {
    fn parse(wku: &str) -> Self {
        let raw = wku.trim();
        // Split a leading alphabetic prefix (D, RE, PP, etc.) from the number.
        let prefix: String = raw
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let mut body: Vec<char> = raw[prefix.len()..].chars().collect();
        // The Green Book WKU is `[prefix] + zero-padded number + one trailing
        // check character`. That check character is the LAST char of the field
        // and may be a digit OR a symbol (e.g. `&`), so it must be dropped
        // BEFORE filtering to digits — otherwise a symbol check char would be
        // removed by the filter and a real digit popped in its place. Examples:
        //   `043757022` -> `4375702`, `06332220&` -> `6332220`,
        //   `D02681420` -> `268142`, `RE0311677` -> `31167`.
        if body.len() > 1 {
            body.pop();
        }
        let digits: String = body.into_iter().filter(|c| c.is_ascii_digit()).collect();
        let stripped = digits.trim_start_matches('0');
        let number = if stripped.is_empty() { "0" } else { stripped };

        // The full grant publication number keeps the kind prefix on display
        // (e.g. D268142, RE31167, PP4988); utility grants are bare digits. The
        // kind code is synthesized so `is_patent_grant` (B/E/S) is satisfied:
        //   - design  (D)  -> "S"
        //   - reissue (RE) -> "E"
        //   - utility      -> "B1" (pre-2001 grants had no pre-grant publication)
        //   - plant   (PP) -> kept as prefix; no grant kind exists in B/E/S, so
        //     patent_number stays null for the (rare) plant case.
        let (doc_number, kind_code) = match prefix.as_str() {
            "D" => (format!("D{number}"), Some("S".to_string())),
            "RE" => (format!("RE{number}"), Some("E".to_string())),
            "PP" | "P" => (format!("{prefix}{number}"), None),
            "" => (number.to_string(), Some("B1".to_string())),
            other => (format!("{other}{number}"), Some("B1".to_string())),
        };

        WkuNumber {
            doc_number,
            kind_code,
        }
    }
}

/// The clean, display patent number for a WKU (kind prefix retained for
/// design/reissue/plant; bare digits for utility), e.g. `043757022` ->
/// `4375702`, `D02681420` -> `D268142`. Used as the shard `doc_key`.
#[cfg_attr(not(feature = "ingest"), allow(dead_code))]
pub fn aps_doc_key_from_wku(wku: &str) -> String {
    WkuNumber::parse(wku).doc_number
}

/// Normalize a Green Book application number by dropping only a trailing
/// *symbol* check character (e.g. `500649&` -> `500649`). A trailing *digit*
/// is intentionally preserved (see the note at the call site): it is ambiguous
/// whether it is a check digit or part of the serial, and these legacy serials
/// do not join to any downstream dataset. Foreign / PCT serials with letters or
/// dashes (`25-04-80-8`, `MI00O0404`) are left untouched.
fn normalize_apn(value: &str) -> String {
    let trimmed = value.trim();
    match trimmed.chars().last() {
        Some(last) if !last.is_ascii_alphanumeric() => {
            // All preceding chars must be digits for this to be a numeric serial
            // plus a symbol check char; otherwise leave the field as-is.
            let head = &trimmed[..trimmed.len() - last.len_utf8()];
            if !head.is_empty() && head.bytes().all(|b| b.is_ascii_digit()) {
                head.to_string()
            } else {
                trimmed.to_string()
            }
        }
        _ => trimmed.to_string(),
    }
}

/// Green Book names are `Last; First` (or just `Last`). Return `(last, first)`.
fn split_aps_name(value: &str) -> (Option<String>, Option<String>) {
    match value.split_once(';') {
        Some((last, first)) => (non_empty(last), non_empty(first)),
        None => (non_empty(value), None),
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// IPC class in the Green Book is space-padded fixed columns (e.g.
/// `A42B  300`). Collapse internal runs of spaces to a single space.
fn normalize_ipc(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// US class likewise carries column padding (e.g. `  2423`, `D 2 27`). Collapse
/// runs of spaces.
fn normalize_us_class(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

// --- Record splitting -----------------------------------------------------

/// A line is a record boundary iff it is a bare `PATN` tag carrying no value.
/// Returns true if `buffer[pos..]` begins such a line (and `pos` is at column
/// 0). Some weekly files are fixed-width and pad every line with trailing spaces
/// to ~column 80, so `PATN` may be followed by run of spaces (or tabs) before
/// the line terminator; that padding is still a bare marker, not a value.
fn is_patn_line_at(buffer: &[u8], pos: usize) -> bool {
    const PATN: &[u8] = b"PATN";
    if !buffer[pos..].starts_with(PATN) {
        return false;
    }
    let mut j = pos + PATN.len();
    while let Some(&b) = buffer.get(j) {
        match b {
            b' ' | b'\t' => j += 1,
            b'\n' | b'\r' => return true,
            _ => return false,
        }
    }
    // EOF after `PATN` (and any padding) — a trailing empty record marker.
    true
}

/// Byte offsets of every record boundary (`PATN` at the start of a line) in the
/// buffer. The weekly file header precedes the first boundary. Used by the
/// shard streamer to split the concatenated APS stream.
pub fn aps_record_starts(buffer: &[u8]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut at_line_start = true;
    let mut i = 0usize;
    while i < buffer.len() {
        if at_line_start && is_patn_line_at(buffer, i) {
            starts.push(i);
        }
        at_line_start = matches!(buffer[i], b'\n' | b'\r');
        i += 1;
    }
    starts
}

/// Find the next `PATN` record boundary in `buffer` at or after `from`,
/// requiring it to begin a line. Returns the byte offset, or `None`.
pub fn next_aps_record_start(buffer: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i < buffer.len() {
        let at_line_start = i == 0 || matches!(buffer[i - 1], b'\n' | b'\r');
        if at_line_start && is_patn_line_at(buffer, i) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::biblio::extract_biblio;
    use crate::render_markdown;

    const UTILITY: &str = "\
PATN
WKU  043757022
SRC  6
APN  2745224
APT  1
ART  353
APD  19810617
TTL  Device for mounting an earmuff on a helmet
ISD  19830308
NCL  9
INVT
NAM  Lundin; Tord R.
CTY  Billesholm
CNT  SEX
ASSG
NAM  Gullfiber AB
CTY  Billesholm
CNT  SEX
COD  03
CLAS
OCL    2423
XCL    2209
ICL  A42B  300
ICL  A42B  124
ABST
PAL  An attachment device in the form of a bearing housing (10) and an arm (38)
      projecting out through an opening (44) in the housing.
BSUM
PAR  The present invention is concerned with an attachment device including a
      bearing housing designed for mounting on a helmet.
DRWD
PAR  FIG. 1 shows the new attachment device in perspective.
DETD
PAR  The attachment device of FIG. 1 includes a bearing housing 10 consisting of
      a base 12.
CLMS
STM  I claim:
NUM  1.
PAR  1. An attachment device including a bearing housing (10) designed for
      mounting on a helmet.
NUM  2.
PAR  2. An attachment device as claimed in claim 1, characterized in that the
      spring element (44) is annular.
";

    const DESIGN: &str = "\
PATN
WKU  D02681420
SRC  6
APN  1989073
APT  4
APD  19801021
TTL  Protective garment for an ice hockey player
ISD  19830308
NCL  1
INVT
NAM  Livernois; John
CTY  St. Jean
CNT  CAX
CLAS
OCL  D 2 27
ICL  D0202
DRWD
PAL  FIG. 1 is a front and left side perspective view.
DCLM
PAR  The ornamental design for a protective garment for an ice hockey player, as
      shown.
";

    #[test]
    fn splits_records_on_bare_patn_line() {
        let stream = format!("HHHHHT        APS1        ISSUE - 830308\n{UTILITY}{DESIGN}");
        // The first byte offset of every `PATN` record boundary in the stream.
        let starts = aps_record_starts(stream.as_bytes());
        // Two records: the header precedes the first boundary.
        assert_eq!(starts.len(), 2);
        let mut bounds = starts.clone();
        bounds.push(stream.len());
        let r0 = &stream[bounds[0]..bounds[1]];
        let r1 = &stream[bounds[1]..bounds[2]];
        assert!(r0.contains("Device for mounting an earmuff"));
        assert!(r1.contains("Protective garment for an ice hockey"));
    }

    #[test]
    fn splits_records_in_fixed_width_padded_files() {
        // Some weekly files pad every line with trailing spaces to ~column 80,
        // so the `PATN` marker is followed by spaces before the line break. Each
        // record must still be detected (regression for the 1997-98 0-patent
        // weeks, where the unpadded-only detector found zero boundaries).
        let pad = |line: &str| format!("{line:<80}\r\n");
        let mut stream = String::new();
        stream.push_str(&pad("HHHHHT        APS1        ISSUE - 981006"));
        for body in [UTILITY, DESIGN] {
            for line in body.lines() {
                stream.push_str(&pad(line));
            }
        }
        let starts = aps_record_starts(stream.as_bytes());
        assert_eq!(starts.len(), 2, "both padded records detected");
        assert!(is_patn_line_at(stream.as_bytes(), starts[0]));
    }

    #[test]
    fn parses_utility_patent() {
        let doc = parse_aps_document(UTILITY).expect("utility parses");
        let biblio = extract_biblio(&doc);
        assert_eq!(biblio.patent_number.as_deref(), Some("US4375702B1"));
        assert_eq!(biblio.publication_number, None);
        assert_eq!(biblio.publication_date.as_deref(), Some("19830308"));
        assert_eq!(biblio.application_number.as_deref(), Some("2745224"));
        assert_eq!(biblio.filing_date.as_deref(), Some("19810617"));
        assert_eq!(
            biblio.invention_title.as_deref(),
            Some("Device for mounting an earmuff on a helmet")
        );
        assert_eq!(biblio.inventor_names, vec!["Tord R. Lundin".to_string()]);
        assert_eq!(biblio.assignee_names, vec!["Gullfiber AB".to_string()]);
        assert_eq!(biblio.ipc_classifications, vec!["A42B 300", "A42B 124"]);
        assert!(biblio.us_classifications.contains(&"2423".to_string()));

        let md = render_markdown(&doc);
        assert!(md.contains("# Device for mounting an earmuff on a helmet"));
        assert!(md.contains("## Abstract"));
        assert!(md.contains("## Summary Of The Invention"));
        assert!(md.contains("## Description Of The Invention"));
        assert!(md.contains("## Claims"));
        assert!(md.contains("An attachment device including a bearing housing"));
    }

    #[test]
    fn parses_design_patent() {
        let doc = parse_aps_document(DESIGN).expect("design parses");
        let biblio = extract_biblio(&doc);
        // Design grants get kind "S", which `is_patent_grant` accepts.
        assert_eq!(biblio.patent_number.as_deref(), Some("USD268142S"));
        let md = render_markdown(&doc);
        assert!(md.contains("# Protective garment for an ice hockey player"));
        assert!(md.contains("## Claims"));
        assert!(md.contains("The ornamental design for a protective garment"));
    }

    #[test]
    fn unwraps_continuation_lines() {
        let fields = collect_fields(UTILITY);
        let title = scalar(&fields, "PATN", "TTL").unwrap();
        assert_eq!(title, "Device for mounting an earmuff on a helmet");
        // The abstract PAL spans two lines and must join with a single space.
        let abst = scalar(&fields, "ABST", "PAL").unwrap();
        assert_eq!(
            abst,
            "An attachment device in the form of a bearing housing (10) and an arm (38) projecting out through an opening (44) in the housing."
        );
    }

    #[test]
    fn normalizes_wku_numbers() {
        assert_eq!(WkuNumber::parse("043757022").doc_number, "4375702");
        assert_eq!(
            WkuNumber::parse("043757022").kind_code.as_deref(),
            Some("B1")
        );
        let design = WkuNumber::parse("D02681420");
        assert_eq!(design.doc_number, "D268142");
        assert_eq!(design.kind_code.as_deref(), Some("S"));
        let reissue = WkuNumber::parse("RE0311677");
        assert_eq!(reissue.doc_number, "RE31167");
        assert_eq!(reissue.kind_code.as_deref(), Some("E"));
        let plant = WkuNumber::parse("PP0049883");
        assert_eq!(plant.doc_number, "PP4988");
        assert_eq!(plant.kind_code, None);
        // A non-digit (`&`) trailing check character must be dropped before the
        // digit filter, not after — otherwise a real digit is lost.
        assert_eq!(WkuNumber::parse("06332220&").doc_number, "6332220");
        let design_amp = WkuNumber::parse("D0452361&");
        assert_eq!(design_amp.doc_number, "D452361");
    }

    #[test]
    fn normalizes_application_numbers() {
        // Symbol check char dropped; trailing digit preserved.
        assert_eq!(normalize_apn("500649&"), "500649");
        assert_eq!(normalize_apn("2745224"), "2745224");
        // Foreign / PCT serials with letters or dashes left untouched.
        assert_eq!(normalize_apn("25-04-80-8"), "25-04-80-8");
        assert_eq!(normalize_apn("MI00O0404"), "MI00O0404");
    }

    #[test]
    fn rejects_record_without_wku() {
        let junk = "PATN\nSRC  6\nART  353\n";
        assert!(parse_aps_document(junk).is_err());
    }
}
