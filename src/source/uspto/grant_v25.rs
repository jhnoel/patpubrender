use xmloxide::tree::{NodeId, NodeKind};

use crate::error::{ParseError, SerializeError};
use crate::model::bibliographic::{
    ApplicationNumber, BibliographicInformation, BibliographicPart, ClassificationIpc,
    ClassificationUs, ContinuityData, ContinuityPart, DocumentId, DomesticFilingData,
    DomesticFilingPart, NamedParties, NamedParty, RelatedDocument, TechnicalInformation,
    TechnicalInformationPart,
};
use crate::model::claims::{Claim, Claims, ClaimsPart, Heading};
use crate::model::description::{
    AbstractPart, AbstractSection, BriefDescriptionOfDrawings, Description, DescriptionPart,
    DescriptionSectionPart, DetailedDescription, Drawings, DrawingsPart, Figure, FigurePart, Image,
    Paragraph, Section, SectionPart, SummaryOfInvention,
};
use crate::model::document::{DocumentPart, PatentDocument};
use crate::model::opaque::{OpaqueBlock, OpaqueInline};
use crate::model::runs::{CrossReference, DependentClaimReference, InlineContainer, Run};
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;
use crate::source::xml;

pub struct UsptoGrantV25Adapter;

impl FormatAdapter for UsptoGrantV25Adapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        let document = xml::parse_document(input)?;
        let root = xml::root_element(&document)?;

        if document.node_name(root) != Some("PATDOC") {
            return Err(ParseError::UnsupportedStructure(
                "UsptoGrantV25 requires PATDOC root".to_string(),
            ));
        }

        let mut parts = Vec::new();
        for child in xml::child_elements(&document, root) {
            parts.push(parse_top_level_part(&document, child));
        }

        Ok(PatentDocument {
            source_format: SourceFormat::UsptoGrantV25,
            prolog: xml::prolog(input, &document),
            attributes: xml::attributes(&document, root),
            parts,
        })
    }

    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError> {
        let mut output = String::new();
        xml::write_prolog(&mut output, &doc.prolog);
        xml::start_tag(&mut output, "PATDOC", &doc.attributes);
        for part in &doc.parts {
            write_document_part(&mut output, part);
        }
        xml::end_tag(&mut output, "PATDOC");
        Ok(output)
    }
}

fn parse_top_level_part(document: &xmloxide::Document, node: NodeId) -> DocumentPart {
    match document.node_name(node) {
        Some("SDOBI") => {
            DocumentPart::BibliographicInformation(parse_bibliographic_information(document, node))
        }
        Some("SDOAB") => DocumentPart::AbstractSection(parse_abstract(document, node)),
        Some("SDODE") => DocumentPart::Description(parse_description(document, node)),
        Some("SDOCL") => DocumentPart::Claims(parse_claims(document, node)),
        Some("SDODR") => DocumentPart::Drawings(parse_drawings(document, node)),
        _ => DocumentPart::Opaque(opaque_block(document, node)),
    }
}

fn parse_bibliographic_information(
    document: &xmloxide::Document,
    node: NodeId,
) -> BibliographicInformation {
    let mut parts = Vec::new();

    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some("B100") => parts.push(BibliographicPart::DocumentId(parse_b100(document, child))),
            Some("B200") => parts.push(BibliographicPart::DomesticFilingData(parse_b200(
                document, child,
            ))),
            Some("B600") => parts.push(BibliographicPart::ContinuityData(parse_b600(
                document, child,
            ))),
            Some("B500") => parts.extend(parse_b500(document, child)),
            Some("B700") => parts.extend(parse_b700(document, child)),
            _ => parts.push(BibliographicPart::Opaque(opaque_block(document, child))),
        }
    }

    BibliographicInformation {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_b100(document: &xmloxide::Document, node: NodeId) -> DocumentId {
    DocumentId {
        doc_number: descendant_text(document, node, &["B110", "DNUM", "PDAT"]).unwrap_or_default(),
        kind_code: descendant_text(document, node, &["B130", "PDAT"]),
        document_date: descendant_text(document, node, &["B140", "DATE", "PDAT"]),
        country_code: descendant_text(document, node, &["B190", "PDAT"]),
    }
}

fn parse_b200(document: &xmloxide::Document, node: NodeId) -> DomesticFilingData {
    let mut parts = Vec::new();
    if let Some(doc_number) = descendant_text(document, node, &["B210", "DNUM", "PDAT"]) {
        parts.push(DomesticFilingPart::ApplicationNumber(ApplicationNumber {
            attributes: vec![],
            appl_type: None,
            doc_number: Some(normalize_numeric_identifier(&doc_number)),
        }));
    }
    if let Some(date) = descendant_text(document, node, &["B220", "DATE", "PDAT"]) {
        parts.push(DomesticFilingPart::FilingDate(date));
    }

    DomesticFilingData {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn normalize_numeric_identifier(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        trimmed.to_string()
    } else if trimmed.trim_start_matches('0').is_empty() {
        "0".to_string()
    } else {
        trimmed.trim_start_matches('0').to_string()
    }
}

fn parse_b600(document: &xmloxide::Document, node: NodeId) -> ContinuityData {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("B620") => ContinuityPart::RelatedDocument(parse_b620(document, child)),
            Some("B680US") => ContinuityPart::RelatedDocument(parse_b680us(document, child)),
            _ => ContinuityPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    ContinuityData {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_b620(document: &xmloxide::Document, node: NodeId) -> RelatedDocument {
    let parent_us = child_element(document, node, "PARENT-US");
    let child_doc = parent_us.and_then(|value| child_element(document, value, "CDOC"));
    let child_doc_number =
        child_doc.and_then(|value| descendant_text(document, value, &["DOC", "DNUM", "PDAT"]));
    let parent_doc = parent_us.and_then(|value| child_element(document, value, "PDOC"));
    let parent_doc_number =
        parent_doc.and_then(|value| descendant_text(document, value, &["DOC", "DNUM", "PDAT"]));
    let parent_date =
        parent_doc.and_then(|value| descendant_text(document, value, &["DOC", "DATE", "PDAT"]));

    RelatedDocument {
        attributes: xml::attributes(document, node),
        parent_doc_number,
        parent_date,
        child_doc_number,
        relationship: Some("parent-us".to_string()),
    }
}

fn parse_b680us(document: &xmloxide::Document, node: NodeId) -> RelatedDocument {
    RelatedDocument {
        attributes: xml::attributes(document, node),
        parent_doc_number: descendant_text(document, node, &["DOC", "DNUM", "PDAT"]),
        parent_date: descendant_text(document, node, &["DOC", "DATE", "PDAT"]),
        child_doc_number: None,
        relationship: Some("us-provisional-application".to_string()),
    }
}

fn parse_b700(document: &xmloxide::Document, node: NodeId) -> Vec<BibliographicPart> {
    let mut parts = Vec::new();

    if let Some(child) = child_element(document, node, "B720") {
        parts.push(BibliographicPart::Applicants(parse_named_parties_v25(
            document, child, "B721",
        )));
    }
    if let Some(child) = child_element(document, node, "B730") {
        parts.push(BibliographicPart::Assignees(parse_named_parties_v25(
            document, child, "B731",
        )));
    }

    parts
}

fn parse_named_parties_v25(
    document: &xmloxide::Document,
    node: NodeId,
    item_tag: &str,
) -> NamedParties {
    let parties = xml::child_elements(document, node)
        .into_iter()
        .filter(|child| document.node_name(*child) == Some(item_tag))
        .map(|child| NamedParty {
            attributes: xml::attributes(document, child),
            name: parse_party_name_v25(document, child),
        })
        .collect();

    NamedParties {
        attributes: xml::attributes(document, node),
        parties,
    }
}

fn parse_party_name_v25(document: &xmloxide::Document, node: NodeId) -> Option<String> {
    if let Some(name) =
        descendant_text(document, node, &["PARTY-US", "NAM", "ONM", "STEXT", "PDAT"])
    {
        return Some(name);
    }

    let first = descendant_text(document, node, &["PARTY-US", "NAM", "FNM", "PDAT"]);
    let last = descendant_text(document, node, &["PARTY-US", "NAM", "SNM", "STEXT", "PDAT"]);
    match (first, last) {
        (Some(first), Some(last)) => Some(format!("{first} {last}")),
        (Some(first), None) => Some(first),
        (None, Some(last)) => Some(last),
        (None, None) => None,
    }
}

fn parse_b500(document: &xmloxide::Document, node: NodeId) -> Vec<BibliographicPart> {
    let mut technical_parts = Vec::new();
    let mut extra_parts = Vec::new();

    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some("B510") => technical_parts.push(TechnicalInformationPart::ClassificationIpc(
                ClassificationIpc {
                    attributes: xml::attributes(document, child),
                    main_classification: descendant_text(document, child, &["B511", "PDAT"]),
                    further_classification: vec![],
                },
            )),
            Some("B520") => technical_parts.push(TechnicalInformationPart::ClassificationUs(
                ClassificationUs {
                    attributes: xml::attributes(document, child),
                    national_classification: descendant_text(document, child, &["B521", "PDAT"]),
                    further_classification: vec![],
                },
            )),
            Some("B540") => {
                if let Some(title) = descendant_text(document, child, &["STEXT", "PDAT"]) {
                    technical_parts.push(TechnicalInformationPart::TitleOfInvention(title));
                }
            }
            _ => extra_parts.push(BibliographicPart::Opaque(opaque_block(document, child))),
        }
    }

    let mut parts = Vec::new();
    if !technical_parts.is_empty() {
        parts.push(BibliographicPart::TechnicalInformation(
            TechnicalInformation {
                attributes: xml::attributes(document, node),
                parts: technical_parts,
            },
        ));
    }
    parts.extend(extra_parts);
    parts
}

fn parse_abstract(document: &xmloxide::Document, node: NodeId) -> AbstractSection {
    AbstractSection {
        attributes: xml::attributes(document, node),
        parts: btext_children(document, node)
            .into_iter()
            .map(|child| match document.node_name(child) {
                Some("PARA") => AbstractPart::Paragraph(parse_paragraph(document, child)),
                _ => AbstractPart::Opaque(opaque_block(document, child)),
            })
            .collect(),
    }
}

fn parse_description(document: &xmloxide::Document, node: NodeId) -> Description {
    let mut parts = Vec::new();

    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some("BRFSUM") => parts.push(DescriptionPart::SummaryOfInvention(SummaryOfInvention {
                attributes: xml::attributes(document, child),
                parts: parse_section_parts(document, child),
            })),
            Some("DRWDESC") => parts.push(DescriptionPart::BriefDescriptionOfDrawings(
                BriefDescriptionOfDrawings {
                    attributes: xml::attributes(document, child),
                    parts: parse_section_parts(document, child),
                },
            )),
            Some("DETDESC") => {
                parts.push(DescriptionPart::DetailedDescription(DetailedDescription {
                    attributes: xml::attributes(document, child),
                    parts: parse_section_parts(document, child),
                }))
            }
            _ => parts.push(DescriptionPart::Opaque(opaque_block(document, child))),
        }
    }

    Description {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_section_parts(document: &xmloxide::Document, node: NodeId) -> Vec<DescriptionSectionPart> {
    let mut parts = Vec::new();
    let mut current_section: Option<Section> = None;

    for child in btext_children(document, node) {
        match document.node_name(child) {
            Some("H") => {
                flush_section(&mut parts, &mut current_section);
                current_section = Some(Section {
                    attributes: vec![],
                    parts: vec![SectionPart::Heading(parse_heading(document, child))],
                });
            }
            Some("PARA") => {
                let section = current_section.get_or_insert_with(|| Section {
                    attributes: vec![],
                    parts: vec![],
                });
                section
                    .parts
                    .push(SectionPart::Paragraph(parse_paragraph(document, child)));
            }
            _ => {
                if let Some(section) = current_section.as_mut() {
                    section
                        .parts
                        .push(SectionPart::Opaque(opaque_block(document, child)));
                } else {
                    parts.push(DescriptionSectionPart::Opaque(opaque_block(
                        document, child,
                    )));
                }
            }
        }
    }

    flush_section(&mut parts, &mut current_section);
    parts
}

fn flush_section(parts: &mut Vec<DescriptionSectionPart>, current: &mut Option<Section>) {
    if let Some(section) = current.take() {
        parts.push(DescriptionSectionPart::Section(section));
    }
}

fn parse_claims(document: &xmloxide::Document, node: NodeId) -> Claims {
    let mut parts = Vec::new();

    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some("H") => parts.push(ClaimsPart::Heading(parse_heading(document, child))),
            Some("CL") => {
                for claim in xml::child_elements(document, child) {
                    match document.node_name(claim) {
                        Some("CLM") => parts.push(ClaimsPart::Claim(parse_claim(document, claim))),
                        _ => parts.push(ClaimsPart::Opaque(opaque_block(document, claim))),
                    }
                }
            }
            Some("CLM") => parts.push(ClaimsPart::Claim(parse_claim(document, child))),
            _ => parts.push(ClaimsPart::Opaque(opaque_block(document, child))),
        }
    }

    Claims {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_claim(document: &xmloxide::Document, node: NodeId) -> Claim {
    let mut content = Vec::new();

    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some("PARA") | Some("CLMSTEP") => content.push(Run::ClaimText(InlineContainer {
                attributes: xml::attributes(document, child),
                content: parse_runs(document, text_container(document, child).unwrap_or(child)),
            })),
            _ => content.push(Run::Opaque(OpaqueInline {
                xml: xml::xml_fragment(document, child),
            })),
        }
    }

    Claim {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "ID").map(ToOwned::to_owned),
        content,
    }
}

fn parse_drawings(document: &xmloxide::Document, node: NodeId) -> Drawings {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("EMI") => DrawingsPart::Figure(Figure {
                attributes: vec![],
                id: document.attribute(child, "ID").map(ToOwned::to_owned),
                parts: vec![FigurePart::Image(Image {
                    attributes: xml::attributes(document, child),
                    id: document.attribute(child, "ID").map(ToOwned::to_owned),
                    file: document.attribute(child, "FILE").map(ToOwned::to_owned),
                    imf: None,
                    ti: None,
                })],
            }),
            _ => DrawingsPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Drawings {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_heading(document: &xmloxide::Document, node: NodeId) -> Heading {
    Heading {
        attributes: xml::attributes(document, node),
        level: document
            .attribute(node, "LVL")
            .and_then(|value| value.parse().ok()),
        align: None,
        content: parse_runs(document, text_container(document, node).unwrap_or(node)),
    }
}

fn parse_paragraph(document: &xmloxide::Document, node: NodeId) -> Paragraph {
    Paragraph {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "ID").map(ToOwned::to_owned),
        level: document
            .attribute(node, "LVL")
            .and_then(|value| value.parse().ok()),
        content: parse_runs(document, text_container(document, node).unwrap_or(node)),
    }
}

fn parse_runs(document: &xmloxide::Document, node: NodeId) -> Vec<Run> {
    let mut runs = Vec::new();

    for child in document.children(node) {
        match &document.node(child).kind {
            NodeKind::Text { content } | NodeKind::CData { content } => {
                if !content.trim().is_empty() {
                    runs.push(Run::Text(content.clone()));
                }
            }
            NodeKind::EntityRef { value, name } => {
                runs.push(Run::Text(
                    value.clone().unwrap_or_else(|| format!("&{name};")),
                ));
            }
            NodeKind::Element { name, .. } => match name.as_str() {
                "PTEXT" | "STEXT" => runs.extend(parse_runs(document, child)),
                "PDAT" => {
                    let text = xml::text_content(document, child);
                    if !text.trim().is_empty() {
                        runs.push(Run::Text(text));
                    }
                }
                "BOLD" => runs.push(Run::Bold(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "ITALIC" => runs.push(Run::Italic(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "HIL" => runs.push(Run::Highlight(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "FGREF" => runs.push(Run::FigureReference(CrossReference {
                    attributes: xml::attributes(document, child),
                    target: document.attribute(child, "ID").map(ToOwned::to_owned),
                    content: parse_runs(document, child),
                })),
                "CLREF" => runs.push(Run::DependentClaimReference(DependentClaimReference {
                    attributes: xml::attributes(document, child),
                    depends_on: document.attribute(child, "ID").map(ToOwned::to_owned),
                    content: parse_runs(document, child),
                })),
                _ => runs.push(Run::Opaque(OpaqueInline {
                    xml: xml::xml_fragment(document, child),
                })),
            },
            _ => {}
        }
    }

    runs
}

fn btext_children(document: &xmloxide::Document, node: NodeId) -> Vec<NodeId> {
    if let Some(btext) = child_element(document, node, "BTEXT") {
        xml::child_elements(document, btext)
    } else {
        xml::child_elements(document, node)
    }
}

fn text_container(document: &xmloxide::Document, node: NodeId) -> Option<NodeId> {
    child_element(document, node, "PTEXT").or_else(|| child_element(document, node, "STEXT"))
}

fn child_element(document: &xmloxide::Document, node: NodeId, name: &str) -> Option<NodeId> {
    xml::child_elements(document, node)
        .into_iter()
        .find(|child| document.node_name(*child) == Some(name))
}

fn descendant_text(document: &xmloxide::Document, node: NodeId, path: &[&str]) -> Option<String> {
    let mut current = node;
    for name in path {
        current = child_element(document, current, name)?;
    }
    let value = xml::text_content(document, current).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn opaque_block(document: &xmloxide::Document, node: NodeId) -> OpaqueBlock {
    OpaqueBlock {
        xml: xml::xml_fragment(document, node),
    }
}

fn write_document_part(output: &mut String, part: &DocumentPart) {
    match part {
        DocumentPart::BibliographicInformation(value) => {
            write_bibliographic_information(output, value)
        }
        DocumentPart::AbstractSection(value) => write_abstract(output, value),
        DocumentPart::Description(value) => write_description(output, value),
        DocumentPart::Claims(value) => write_claims(output, value),
        DocumentPart::Drawings(value) => write_drawings(output, value),
        DocumentPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
    }
}

fn write_bibliographic_information(output: &mut String, value: &BibliographicInformation) {
    xml::start_tag(output, "SDOBI", &value.attributes);
    for part in &value.parts {
        match part {
            BibliographicPart::DocumentId(value) => {
                xml::start_tag(output, "B100", &[]);
                write_nested_pdat(output, "B110", "DNUM", &value.doc_number);
                if let Some(kind) = &value.kind_code {
                    write_pdat_block(output, "B130", kind);
                }
                if let Some(date) = &value.document_date {
                    xml::start_tag(output, "B140", &[]);
                    write_pdat_block(output, "DATE", date);
                    xml::end_tag(output, "B140");
                }
                if let Some(country) = &value.country_code {
                    write_pdat_block(output, "B190", country);
                }
                xml::end_tag(output, "B100");
            }
            BibliographicPart::DomesticFilingData(value) => {
                xml::start_tag(output, "B200", &value.attributes);
                if let Some(number) = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::ApplicationNumber(number) => number.doc_number.as_deref(),
                    _ => None,
                }) {
                    write_nested_pdat(output, "B210", "DNUM", number);
                }
                if let Some(series) = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::ApplicationNumberSeriesCode(value) => Some(value.as_str()),
                    _ => None,
                }) {
                    write_pdat_block(output, "B211US", series);
                }
                if let Some(date) = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::FilingDate(value) => Some(value.as_str()),
                    _ => None,
                }) {
                    xml::start_tag(output, "B220", &[]);
                    write_pdat_block(output, "DATE", date);
                    xml::end_tag(output, "B220");
                }
                xml::end_tag(output, "B200");
            }
            BibliographicPart::ContinuityData(value) => write_b600(output, value),
            BibliographicPart::TechnicalInformation(value) => {
                xml::start_tag(output, "B500", &value.attributes);
                for part in &value.parts {
                    match part {
                        TechnicalInformationPart::ClassificationIpc(value) => {
                            xml::start_tag(output, "B510", &value.attributes);
                            if let Some(main) = &value.main_classification {
                                write_pdat_block(output, "B511", main);
                            }
                            xml::end_tag(output, "B510");
                        }
                        TechnicalInformationPart::ClassificationUs(value) => {
                            xml::start_tag(output, "B520", &value.attributes);
                            if let Some(main) = &value.national_classification {
                                write_pdat_block(output, "B521", main);
                            }
                            xml::end_tag(output, "B520");
                        }
                        TechnicalInformationPart::TitleOfInvention(value) => {
                            xml::start_tag(output, "B540", &[]);
                            write_text_wrapper(output, "STEXT", value);
                            xml::end_tag(output, "B540");
                        }
                        TechnicalInformationPart::Opaque(value) => {
                            xml::write_xml_fragment(output, &value.xml)
                        }
                    }
                }
                xml::end_tag(output, "B500");
            }
            BibliographicPart::Applicants(value) => {
                write_b700_parties(output, "B720", "B721", value)
            }
            BibliographicPart::Assignees(value) => {
                write_b700_parties(output, "B730", "B731", value)
            }
            BibliographicPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
            _ => {}
        }
    }
    xml::end_tag(output, "SDOBI");
}

fn write_b600(output: &mut String, value: &ContinuityData) {
    xml::start_tag(output, "B600", &value.attributes);
    for part in &value.parts {
        match part {
            ContinuityPart::NonProvisionalOfProvisional(value)
            | ContinuityPart::RelatedDocument(value) => match value.relationship.as_deref() {
                Some("us-provisional-application") => {
                    xml::start_tag(output, "B680US", &value.attributes);
                    xml::start_tag(output, "DOC", &[]);
                    write_nested_pdat_if_some(output, "DNUM", value.parent_doc_number.as_deref());
                    if let Some(date) = &value.parent_date {
                        xml::start_tag(output, "DATE", &[]);
                        write_pdat(output, date);
                        xml::end_tag(output, "DATE");
                    }
                    if value.parent_doc_number.is_some() || value.parent_date.is_some() {
                        write_pdat_block(output, "KIND", "00");
                    }
                    xml::end_tag(output, "DOC");
                    xml::end_tag(output, "B680US");
                }
                _ => {
                    xml::start_tag(output, "B620", &value.attributes);
                    xml::start_tag(output, "PARENT-US", &[]);
                    if let Some(child) = &value.child_doc_number {
                        xml::start_tag(output, "CDOC", &[]);
                        write_nested_pdat(output, "DOC", "DNUM", child);
                        xml::end_tag(output, "CDOC");
                    }
                    if value.parent_doc_number.is_some() || value.parent_date.is_some() {
                        xml::start_tag(output, "PDOC", &[]);
                        xml::start_tag(output, "DOC", &[]);
                        write_nested_pdat_if_some(
                            output,
                            "DNUM",
                            value.parent_doc_number.as_deref(),
                        );
                        if let Some(date) = &value.parent_date {
                            xml::start_tag(output, "DATE", &[]);
                            write_pdat(output, date);
                            xml::end_tag(output, "DATE");
                        }
                        xml::end_tag(output, "DOC");
                        xml::end_tag(output, "PDOC");
                    }
                    xml::end_tag(output, "PARENT-US");
                    xml::end_tag(output, "B620");
                }
            },
            ContinuityPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    xml::end_tag(output, "B600");
}

fn write_b700_parties(
    output: &mut String,
    container_tag: &str,
    item_tag: &str,
    value: &NamedParties,
) {
    xml::start_tag(output, "B700", &[]);
    xml::start_tag(output, container_tag, &value.attributes);
    for party in &value.parties {
        xml::start_tag(output, item_tag, &party.attributes);
        xml::start_tag(output, "PARTY-US", &[]);
        xml::start_tag(output, "NAM", &[]);
        if let Some(name) = &party.name {
            xml::start_tag(output, "ONM", &[]);
            write_text_wrapper(output, "STEXT", name);
            xml::end_tag(output, "ONM");
        }
        xml::end_tag(output, "NAM");
        xml::end_tag(output, "PARTY-US");
        xml::end_tag(output, item_tag);
    }
    xml::end_tag(output, container_tag);
    xml::end_tag(output, "B700");
}

fn write_abstract(output: &mut String, value: &AbstractSection) {
    xml::start_tag(output, "SDOAB", &value.attributes);
    xml::start_tag(output, "BTEXT", &[]);
    for part in &value.parts {
        match part {
            AbstractPart::Paragraph(value) => write_paragraph(output, value),
            AbstractPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    xml::end_tag(output, "BTEXT");
    xml::end_tag(output, "SDOAB");
}

fn write_description(output: &mut String, value: &Description) {
    xml::start_tag(output, "SDODE", &value.attributes);
    for part in &value.parts {
        match part {
            DescriptionPart::SummaryOfInvention(value) => {
                write_description_container(output, "BRFSUM", &value.parts)
            }
            DescriptionPart::BriefDescriptionOfDrawings(value) => {
                write_description_container(output, "DRWDESC", &value.parts)
            }
            DescriptionPart::DetailedDescription(value) => {
                write_description_container(output, "DETDESC", &value.parts)
            }
            DescriptionPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
            DescriptionPart::CrossReferenceToRelatedApplications(_) => {}
        }
    }
    xml::end_tag(output, "SDODE");
}

fn write_description_container(output: &mut String, tag: &str, parts: &[DescriptionSectionPart]) {
    xml::start_tag(output, tag, &[]);
    xml::start_tag(output, "BTEXT", &[]);
    write_section_parts(output, parts);
    xml::end_tag(output, "BTEXT");
    xml::end_tag(output, tag);
}

fn write_section_parts(output: &mut String, parts: &[DescriptionSectionPart]) {
    for part in parts {
        match part {
            DescriptionSectionPart::Section(value) => {
                for part in &value.parts {
                    match part {
                        SectionPart::Heading(value) => write_heading(output, value),
                        SectionPart::Paragraph(value) => write_paragraph(output, value),
                        SectionPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
                    }
                }
            }
            DescriptionSectionPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
}

fn write_claims(output: &mut String, value: &Claims) {
    xml::start_tag(output, "SDOCL", &value.attributes);
    let mut claims = Vec::new();
    for part in &value.parts {
        match part {
            ClaimsPart::Heading(value) => write_heading(output, value),
            ClaimsPart::Claim(value) => claims.push(value),
            ClaimsPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    if !claims.is_empty() {
        xml::start_tag(output, "CL", &[]);
        for claim in claims {
            xml::start_tag(output, "CLM", &claim.attributes);
            for run in &claim.content {
                match run {
                    Run::ClaimText(value) => {
                        let tag = if value
                            .attributes
                            .iter()
                            .any(|attribute| attribute.local_name == "ID")
                        {
                            "PARA"
                        } else {
                            "CLMSTEP"
                        };
                        xml::start_tag(output, tag, &value.attributes);
                        write_text_run_wrapper(output, "PTEXT", &value.content);
                        xml::end_tag(output, tag);
                    }
                    Run::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
                    other => {
                        xml::start_tag(output, "PARA", &[]);
                        write_text_run_wrapper(output, "PTEXT", std::slice::from_ref(other));
                        xml::end_tag(output, "PARA");
                    }
                }
            }
            xml::end_tag(output, "CLM");
        }
        xml::end_tag(output, "CL");
    }
    xml::end_tag(output, "SDOCL");
}

fn write_drawings(output: &mut String, value: &Drawings) {
    xml::start_tag(output, "SDODR", &value.attributes);
    for part in &value.parts {
        match part {
            DrawingsPart::Figure(value) => {
                for part in &value.parts {
                    match part {
                        FigurePart::Image(value) => {
                            xml::start_empty_tag(output, "EMI", &value.attributes)
                        }
                        FigurePart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
                    }
                }
            }
            DrawingsPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
            _ => {}
        }
    }
    xml::end_tag(output, "SDODR");
}

fn write_heading(output: &mut String, value: &Heading) {
    xml::start_tag(output, "H", &value.attributes);
    write_text_run_wrapper(output, "STEXT", &value.content);
    xml::end_tag(output, "H");
}

fn write_paragraph(output: &mut String, value: &Paragraph) {
    xml::start_tag(output, "PARA", &value.attributes);
    write_text_run_wrapper(output, "PTEXT", &value.content);
    xml::end_tag(output, "PARA");
}

fn write_text_wrapper(output: &mut String, wrapper: &str, value: &str) {
    xml::start_tag(output, wrapper, &[]);
    write_pdat(output, value);
    xml::end_tag(output, wrapper);
}

fn write_text_run_wrapper(output: &mut String, wrapper: &str, runs: &[Run]) {
    xml::start_tag(output, wrapper, &[]);
    write_runs(output, runs);
    xml::end_tag(output, wrapper);
}

fn write_runs(output: &mut String, runs: &[Run]) {
    for run in runs {
        match run {
            Run::Text(value) => write_pdat(output, value),
            Run::Bold(value) => {
                xml::start_tag(output, "BOLD", &value.attributes);
                write_runs(output, &value.content);
                xml::end_tag(output, "BOLD");
            }
            Run::Italic(value) => {
                xml::start_tag(output, "ITALIC", &value.attributes);
                write_runs(output, &value.content);
                xml::end_tag(output, "ITALIC");
            }
            Run::Highlight(value) => {
                xml::start_tag(output, "HIL", &value.attributes);
                write_runs(output, &value.content);
                xml::end_tag(output, "HIL");
            }
            Run::ClaimText(value) => write_runs(output, &value.content),
            Run::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
            Run::Number(value) => write_runs(output, &value.content),
            Run::CrossReference(value) => write_runs(output, &value.content),
            Run::FigureReference(value) => {
                xml::start_tag(output, "FGREF", &value.attributes);
                write_runs(output, &value.content);
                xml::end_tag(output, "FGREF");
            }
            Run::DependentClaimReference(value) => {
                xml::start_tag(output, "CLREF", &value.attributes);
                write_runs(output, &value.content);
                xml::end_tag(output, "CLREF");
            }
        }
    }
}

fn write_pdat(output: &mut String, value: &str) {
    xml::start_tag(output, "PDAT", &[]);
    output.push_str(&xml::escape_text(value));
    xml::end_tag(output, "PDAT");
}

fn write_pdat_block(output: &mut String, tag: &str, value: &str) {
    xml::start_tag(output, tag, &[]);
    write_pdat(output, value);
    xml::end_tag(output, tag);
}

fn write_nested_pdat(output: &mut String, outer: &str, inner: &str, value: &str) {
    xml::start_tag(output, outer, &[]);
    write_pdat_block(output, inner, value);
    xml::end_tag(output, outer);
}

fn write_nested_pdat_if_some(output: &mut String, outer: &str, value: Option<&str>) {
    if let Some(value) = value {
        write_nested_pdat(output, outer, "PDAT", value);
    }
}
