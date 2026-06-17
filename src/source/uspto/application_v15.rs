use xmloxide::tree::{NodeId, NodeKind};

use crate::error::{ParseError, SerializeError};
use crate::model::bibliographic::{
    Address, ApplicationNumber, BibliographicInformation, BibliographicPart, ClassificationIpc,
    ClassificationUs, ContinuityData, ContinuityPart, CorrespondenceAddress,
    CorrespondenceAddressPart, DocumentId, DomesticFilingData, DomesticFilingPart, Inventor,
    Inventors, InventorsPart, NamedParties, RelatedDocument, TechnicalInformation,
    TechnicalInformationPart,
};
use crate::model::claims::{Claim, ClaimsPart};
use crate::model::claims::{Claims, Heading};
use crate::model::description::{
    AbstractPart, AbstractSection, BriefDescriptionOfDrawings, CrossReferencePart,
    CrossReferenceToRelatedApplications, Description, DescriptionPart, DescriptionSectionPart,
    DetailedDescription, Drawings, DrawingsPart, Figure, FigurePart, Image, Paragraph, Section,
    SectionPart, SummaryOfInvention,
};
use crate::model::document::{DocumentPart, PatentDocument};
use crate::model::opaque::{OpaqueBlock, OpaqueInline};
use crate::model::runs::{CrossReference, DependentClaimReference, InlineContainer, Number, Run};
use crate::source::detect::SourceFormat;
use crate::source::traits::FormatAdapter;
use crate::source::xml;

pub struct UsptoApplicationV15Adapter;

impl FormatAdapter for UsptoApplicationV15Adapter {
    fn parse_document(&self, input: &str) -> Result<PatentDocument, ParseError> {
        parse_document(input, SourceFormat::UsptoApplicationV15)
    }

    fn write_document(&self, doc: &PatentDocument) -> Result<String, SerializeError> {
        Ok(write_document(doc))
    }
}

pub(super) fn parse_document(
    input: &str,
    source_format: SourceFormat,
) -> Result<PatentDocument, ParseError> {
    let document = xml::parse_document(input)?;
    let root = xml::root_element(&document)?;

    if document.node_name(root) != Some("patent-application-publication") {
        return Err(ParseError::UnsupportedStructure(
            "UsptoApplicationV15 requires patent-application-publication root".to_string(),
        ));
    }

    let mut parts = Vec::new();
    for child in xml::child_elements(&document, root) {
        parts.push(parse_top_level_part(&document, child));
    }

    Ok(PatentDocument {
        source_format,
        prolog: xml::prolog(input, &document),
        attributes: xml::attributes(&document, root),
        parts,
    })
}

fn parse_top_level_part(document: &xmloxide::Document, node: NodeId) -> DocumentPart {
    match document.node_name(node) {
        Some("subdoc-bibliographic-information") => {
            DocumentPart::BibliographicInformation(parse_bibliographic_information(document, node))
        }
        Some("subdoc-abstract") => DocumentPart::AbstractSection(AbstractSection {
            attributes: xml::attributes(document, node),
            parts: parse_abstract_section(document, node),
        }),
        Some("subdoc-description") => DocumentPart::Description(parse_description(document, node)),
        Some("subdoc-claims") => DocumentPart::Claims(parse_claims(document, node)),
        Some("subdoc-drawings") => DocumentPart::Drawings(parse_drawings(document, node)),
        _ => DocumentPart::Opaque(OpaqueBlock {
            xml: xml::xml_fragment(document, node),
        }),
    }
}

fn parse_abstract_section(document: &xmloxide::Document, node: NodeId) -> Vec<AbstractPart> {
    xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("paragraph") => AbstractPart::Paragraph(parse_paragraph(document, child)),
            _ => AbstractPart::Opaque(opaque_block(document, child)),
        })
        .collect()
}

fn parse_claims(document: &xmloxide::Document, node: NodeId) -> Claims {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("heading") => ClaimsPart::Heading(parse_heading(document, child)),
            Some("claim") => ClaimsPart::Claim(parse_claim(document, child)),
            _ => ClaimsPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Claims {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_claim(document: &xmloxide::Document, node: NodeId) -> Claim {
    Claim {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "id").map(ToOwned::to_owned),
        content: parse_runs(document, node),
    }
}

fn parse_drawings(document: &xmloxide::Document, node: NodeId) -> Drawings {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("heading") => DrawingsPart::Heading(parse_heading(document, child)),
            Some("representative-figure") => DrawingsPart::RepresentativeFigure(
                xml::text_content(document, child).trim().to_string(),
            ),
            Some("figure") => DrawingsPart::Figure(parse_figure(document, child)),
            _ => DrawingsPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Drawings {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_figure(document: &xmloxide::Document, node: NodeId) -> Figure {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("image") => FigurePart::Image(parse_image(document, child)),
            _ => FigurePart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Figure {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "id").map(ToOwned::to_owned),
        parts,
    }
}

fn parse_image(document: &xmloxide::Document, node: NodeId) -> Image {
    Image {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "id").map(ToOwned::to_owned),
        file: document.attribute(node, "file").map(ToOwned::to_owned),
        imf: document.attribute(node, "imf").map(ToOwned::to_owned),
        ti: document.attribute(node, "ti").map(ToOwned::to_owned),
    }
}

fn parse_description(document: &xmloxide::Document, node: NodeId) -> Description {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("cross-reference-to-related-applications") => {
                DescriptionPart::CrossReferenceToRelatedApplications(parse_cross_reference_section(
                    document, child,
                ))
            }
            Some("summary-of-invention") => {
                DescriptionPart::SummaryOfInvention(parse_summary_of_invention(document, child))
            }
            Some("brief-description-of-drawings") => DescriptionPart::BriefDescriptionOfDrawings(
                parse_brief_description_of_drawings(document, child),
            ),
            Some("detailed-description") => {
                DescriptionPart::DetailedDescription(parse_detailed_description(document, child))
            }
            _ => DescriptionPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Description {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_cross_reference_section(
    document: &xmloxide::Document,
    node: NodeId,
) -> CrossReferenceToRelatedApplications {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("heading") => CrossReferencePart::Heading(parse_heading(document, child)),
            Some("paragraph") => CrossReferencePart::Paragraph(parse_paragraph(document, child)),
            _ => CrossReferencePart::Opaque(opaque_block(document, child)),
        })
        .collect();

    CrossReferenceToRelatedApplications {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_summary_of_invention(document: &xmloxide::Document, node: NodeId) -> SummaryOfInvention {
    SummaryOfInvention {
        attributes: xml::attributes(document, node),
        parts: parse_description_section_parts(document, node),
    }
}

fn parse_brief_description_of_drawings(
    document: &xmloxide::Document,
    node: NodeId,
) -> BriefDescriptionOfDrawings {
    BriefDescriptionOfDrawings {
        attributes: xml::attributes(document, node),
        parts: parse_description_section_parts(document, node),
    }
}

fn parse_detailed_description(document: &xmloxide::Document, node: NodeId) -> DetailedDescription {
    DetailedDescription {
        attributes: xml::attributes(document, node),
        parts: parse_description_section_parts(document, node),
    }
}

fn parse_description_section_parts(
    document: &xmloxide::Document,
    node: NodeId,
) -> Vec<DescriptionSectionPart> {
    xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("section") => DescriptionSectionPart::Section(parse_section(document, child)),
            _ => DescriptionSectionPart::Opaque(opaque_block(document, child)),
        })
        .collect()
}

fn parse_section(document: &xmloxide::Document, node: NodeId) -> Section {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("heading") => SectionPart::Heading(parse_heading(document, child)),
            Some("paragraph") => SectionPart::Paragraph(parse_paragraph(document, child)),
            _ => SectionPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Section {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_heading(document: &xmloxide::Document, node: NodeId) -> Heading {
    Heading {
        attributes: xml::attributes(document, node),
        level: document
            .attribute(node, "lvl")
            .and_then(|value| value.parse().ok()),
        align: document.attribute(node, "align").map(ToOwned::to_owned),
        content: parse_runs(document, node),
    }
}

fn parse_paragraph(document: &xmloxide::Document, node: NodeId) -> Paragraph {
    Paragraph {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "id").map(ToOwned::to_owned),
        level: document
            .attribute(node, "lvl")
            .and_then(|value| value.parse().ok()),
        content: parse_runs(document, node),
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
                "number" => runs.push(Run::Number(Number {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "cross-reference" => runs.push(Run::CrossReference(CrossReference {
                    attributes: xml::attributes(document, child),
                    target: document.attribute(child, "target").map(ToOwned::to_owned),
                    content: parse_runs(document, child),
                })),
                "figref" => runs.push(Run::FigureReference(CrossReference {
                    attributes: xml::attributes(document, child),
                    target: document
                        .attribute(child, "idref")
                        .or_else(|| document.attribute(child, "target"))
                        .map(ToOwned::to_owned),
                    content: parse_runs(document, child),
                })),
                "dependent-claim-reference" => {
                    runs.push(Run::DependentClaimReference(DependentClaimReference {
                        attributes: xml::attributes(document, child),
                        depends_on: document
                            .attribute(child, "depends_on")
                            .map(ToOwned::to_owned),
                        content: parse_runs(document, child),
                    }))
                }
                "bold" => runs.push(Run::Bold(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "italic" => runs.push(Run::Italic(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "highlight" => runs.push(Run::Highlight(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "claim-text" => runs.push(Run::ClaimText(InlineContainer {
                    attributes: xml::attributes(document, child),
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

fn parse_bibliographic_information(
    document: &xmloxide::Document,
    node: NodeId,
) -> BibliographicInformation {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("document-id") => {
                BibliographicPart::DocumentId(parse_document_id(document, child))
            }
            Some("publication-filing-type") => BibliographicPart::PublicationFilingType(
                xml::text_content(document, child).trim().to_string(),
            ),
            Some("domestic-filing-data") => {
                BibliographicPart::DomesticFilingData(parse_domestic_filing_data(document, child))
            }
            Some("technical-information") => BibliographicPart::TechnicalInformation(
                parse_technical_information(document, child),
            ),
            Some("continuity-data") => {
                BibliographicPart::ContinuityData(parse_continuity_data(document, child))
            }
            Some("inventors") => BibliographicPart::Inventors(parse_inventors(document, child)),
            Some("correspondence-address") => BibliographicPart::CorrespondenceAddress(
                parse_correspondence_address(document, child),
            ),
            _ => opaque_block_part(document, child),
        })
        .collect();

    BibliographicInformation {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_document_id(document: &xmloxide::Document, node: NodeId) -> DocumentId {
    DocumentId {
        doc_number: child_text(document, node, "doc-number").unwrap_or_default(),
        kind_code: child_text(document, node, "kind-code"),
        document_date: child_text(document, node, "document-date"),
        country_code: child_text(document, node, "country-code"),
    }
}

fn parse_domestic_filing_data(document: &xmloxide::Document, node: NodeId) -> DomesticFilingData {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("application-number") => {
                DomesticFilingPart::ApplicationNumber(parse_application_number(document, child))
            }
            Some("application-number-series-code") => {
                DomesticFilingPart::ApplicationNumberSeriesCode(
                    xml::text_content(document, child).trim().to_string(),
                )
            }
            Some("filing-date") => DomesticFilingPart::FilingDate(
                xml::text_content(document, child).trim().to_string(),
            ),
            _ => DomesticFilingPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    DomesticFilingData {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_application_number(document: &xmloxide::Document, node: NodeId) -> ApplicationNumber {
    ApplicationNumber {
        attributes: xml::attributes(document, node),
        appl_type: document.attribute(node, "appl-type").map(ToOwned::to_owned),
        doc_number: child_text(document, node, "doc-number"),
    }
}

fn parse_technical_information(
    document: &xmloxide::Document,
    node: NodeId,
) -> TechnicalInformation {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("classification-ipc") => TechnicalInformationPart::ClassificationIpc(
                parse_classification_ipc(document, child),
            ),
            Some("classification-us") => {
                TechnicalInformationPart::ClassificationUs(parse_classification_us(document, child))
            }
            Some("title-of-invention") => TechnicalInformationPart::TitleOfInvention(
                xml::text_content(document, child).trim().to_string(),
            ),
            _ => TechnicalInformationPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    TechnicalInformation {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_classification_ipc(document: &xmloxide::Document, node: NodeId) -> ClassificationIpc {
    ClassificationIpc {
        attributes: xml::attributes(document, node),
        main_classification: first_non_empty_descendant_text(document, node, &["ipc"]),
        further_classification: vec![],
    }
}

fn parse_classification_us(document: &xmloxide::Document, node: NodeId) -> ClassificationUs {
    ClassificationUs {
        attributes: xml::attributes(document, node),
        national_classification: first_non_empty_descendant_text(
            document,
            node,
            &["class", "subclass"],
        )
        .map(|value| value.trim().to_string()),
        further_classification: vec![],
    }
}

fn parse_continuity_data(document: &xmloxide::Document, node: NodeId) -> ContinuityData {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("continuations")
            | Some("a-371-of-international")
            | Some("continuation-of")
            | Some("continuation-in-part-of") => {
                ContinuityPart::NonProvisionalOfProvisional(parse_related_document(document, child))
            }
            _ => ContinuityPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    ContinuityData {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_related_document(document: &xmloxide::Document, node: NodeId) -> RelatedDocument {
    RelatedDocument {
        attributes: xml::attributes(document, node),
        parent_doc_number: first_non_empty_descendant_text(document, node, &["doc-number"]),
        parent_date: first_non_empty_descendant_text(document, node, &["document-date"]),
        child_doc_number: None,
        relationship: document.node_name(node).map(ToOwned::to_owned),
    }
}

fn parse_inventors(document: &xmloxide::Document, node: NodeId) -> Inventors {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("first-named-inventor") => {
                InventorsPart::FirstNamedInventor(parse_inventor(document, child))
            }
            Some("inventor") => InventorsPart::Inventor(parse_inventor(document, child)),
            _ => InventorsPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    Inventors {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_inventor(document: &xmloxide::Document, node: NodeId) -> Inventor {
    Inventor {
        attributes: xml::attributes(document, node),
        first_name: first_non_empty_descendant_text(document, node, &["given-name"]),
        last_name: first_non_empty_descendant_text(document, node, &["family-name"]),
        city: first_non_empty_descendant_text(document, node, &["city"]),
        state: first_non_empty_descendant_text(document, node, &["state"]),
        country: first_non_empty_descendant_text(document, node, &["country-code"]),
    }
}

fn parse_correspondence_address(
    document: &xmloxide::Document,
    node: NodeId,
) -> CorrespondenceAddress {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("name-1") => CorrespondenceAddressPart::Name1(
                xml::text_content(document, child).trim().to_string(),
            ),
            Some("name-2") => CorrespondenceAddressPart::Name2(
                xml::text_content(document, child).trim().to_string(),
            ),
            Some("address") => CorrespondenceAddressPart::Address(parse_address(document, child)),
            _ => CorrespondenceAddressPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    CorrespondenceAddress {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_address(document: &xmloxide::Document, node: NodeId) -> Address {
    let mut lines = Vec::new();
    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some(name) if name.starts_with("address-") => {
                lines.push(xml::text_content(document, child).trim().to_string());
            }
            _ => {}
        }
    }

    Address {
        attributes: xml::attributes(document, node),
        city: child_text(document, node, "city"),
        state: child_text(document, node, "state"),
        country: first_non_empty_descendant_text(document, node, &["country-code"]),
        postal_code: child_text(document, node, "postalcode"),
        lines,
    }
}

fn child_text(document: &xmloxide::Document, node: NodeId, name: &str) -> Option<String> {
    xml::child_elements(document, node)
        .into_iter()
        .find(|child| document.node_name(*child) == Some(name))
        .map(|child| xml::text_content(document, child).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn first_non_empty_descendant_text(
    document: &xmloxide::Document,
    node: NodeId,
    names: &[&str],
) -> Option<String> {
    document
        .descendants(node)
        .find(|child| {
            document.is_element(*child)
                && names
                    .iter()
                    .any(|name| document.node_name(*child) == Some(*name))
        })
        .map(|child| xml::text_content(document, child))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn opaque_block_part(document: &xmloxide::Document, node: NodeId) -> BibliographicPart {
    BibliographicPart::Opaque(opaque_block(document, node))
}

fn opaque_block(document: &xmloxide::Document, node: NodeId) -> OpaqueBlock {
    OpaqueBlock {
        xml: xml::xml_fragment(document, node),
    }
}

pub(super) fn write_document(doc: &PatentDocument) -> String {
    let mut output = String::new();

    if let Some(version) = &doc.prolog.xml_version {
        output.push_str("<?xml");
        output.push_str(&format!(" version=\"{version}\""));
        if let Some(encoding) = &doc.prolog.xml_encoding {
            output.push_str(&format!(" encoding=\"{encoding}\""));
        }
        if let Some(standalone) = &doc.prolog.xml_standalone {
            output.push_str(&format!(" standalone=\"{standalone}\""));
        }
        output.push_str("?>");
    }

    if let Some(doctype_name) = &doc.prolog.doctype_name {
        output.push_str("<!DOCTYPE ");
        output.push_str(doctype_name);
        if let Some(public_id) = &doc.prolog.doctype_public_id {
            output.push_str(&format!(" PUBLIC \"{public_id}\""));
        }
        if let Some(system_id) = &doc.prolog.doctype_system_id {
            if doc.prolog.doctype_public_id.is_none() {
                output.push_str(" SYSTEM");
            }
            output.push_str(&format!(" \"{system_id}\""));
        }
        if let Some(internal_subset) = &doc.prolog.internal_subset {
            output.push_str(" [");
            output.push_str(internal_subset);
            output.push(']');
        }
        output.push('>');
    }

    start_tag(
        &mut output,
        "patent-application-publication",
        &doc.attributes,
    );
    for part in &doc.parts {
        write_document_part(&mut output, part);
    }
    end_tag(&mut output, "patent-application-publication");

    output
}

fn write_document_part(output: &mut String, part: &DocumentPart) {
    match part {
        DocumentPart::BibliographicInformation(value) => {
            write_bibliographic_information(output, value)
        }
        DocumentPart::AbstractSection(value) => {
            start_tag(output, "subdoc-abstract", &value.attributes);
            for part in &value.parts {
                match part {
                    AbstractPart::Paragraph(value) => write_paragraph(output, value),
                    AbstractPart::Opaque(value) => write_xml_fragment(output, &value.xml),
                }
            }
            end_tag(output, "subdoc-abstract");
        }
        DocumentPart::Description(value) => write_description(output, value),
        DocumentPart::Claims(value) => write_claims(output, value),
        DocumentPart::Drawings(value) => write_drawings(output, value),
        DocumentPart::Opaque(value) => write_xml_fragment(output, &value.xml),
    }
}

fn write_bibliographic_information(output: &mut String, value: &BibliographicInformation) {
    start_tag(
        output,
        "subdoc-bibliographic-information",
        &value.attributes,
    );
    for part in &value.parts {
        match part {
            BibliographicPart::DocumentId(value) => write_document_id(output, "document-id", value),
            BibliographicPart::PublicationFilingType(value) => {
                write_text_element(output, "publication-filing-type", value)
            }
            BibliographicPart::DomesticFilingData(value) => {
                write_domestic_filing_data(output, value)
            }
            BibliographicPart::TechnicalInformation(value) => {
                write_technical_information(output, value)
            }
            BibliographicPart::ContinuityData(value) => write_continuity_data(output, value),
            BibliographicPart::Applicants(value) => {
                write_named_parties(output, "applicants", "applicant", value)
            }
            BibliographicPart::Assignees(value) => {
                write_named_parties(output, "assignees", "assignee", value)
            }
            BibliographicPart::Inventors(value) => write_inventors(output, value),
            BibliographicPart::CorrespondenceAddress(value) => {
                write_correspondence_address(output, value)
            }
            BibliographicPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "subdoc-bibliographic-information");
}

fn write_document_id(output: &mut String, tag: &str, value: &DocumentId) {
    start_tag(output, tag, &[]);
    write_text_element(output, "doc-number", &value.doc_number);
    if let Some(value) = &value.kind_code {
        write_text_element(output, "kind-code", value);
    }
    if let Some(value) = &value.document_date {
        write_text_element(output, "document-date", value);
    }
    if let Some(value) = &value.country_code {
        write_text_element(output, "country-code", value);
    }
    end_tag(output, tag);
}

fn write_domestic_filing_data(output: &mut String, value: &DomesticFilingData) {
    start_tag(output, "domestic-filing-data", &value.attributes);
    for part in &value.parts {
        match part {
            DomesticFilingPart::ApplicationNumber(value) => {
                start_tag(output, "application-number", &value.attributes);
                if let Some(value) = &value.doc_number {
                    write_text_element(output, "doc-number", value);
                }
                end_tag(output, "application-number");
            }
            DomesticFilingPart::ApplicationNumberSeriesCode(value) => {
                write_text_element(output, "application-number-series-code", value)
            }
            DomesticFilingPart::FilingDate(value) => {
                write_text_element(output, "filing-date", value)
            }
            DomesticFilingPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "domestic-filing-data");
}

fn write_technical_information(output: &mut String, value: &TechnicalInformation) {
    start_tag(output, "technical-information", &value.attributes);
    for part in &value.parts {
        match part {
            TechnicalInformationPart::ClassificationIpc(value) => {
                start_tag(output, "classification-ipc", &value.attributes);
                if let Some(text) = &value.main_classification {
                    start_tag(output, "classification-ipc-primary", &[]);
                    write_text_element(output, "ipc", text);
                    end_tag(output, "classification-ipc-primary");
                }
                end_tag(output, "classification-ipc");
            }
            TechnicalInformationPart::ClassificationUs(value) => {
                start_tag(output, "classification-us", &value.attributes);
                if let Some(text) = &value.national_classification {
                    start_tag(output, "classification-us-primary", &[]);
                    start_tag(output, "uspc", &[]);
                    write_text_element(output, "class", text);
                    end_tag(output, "uspc");
                    end_tag(output, "classification-us-primary");
                }
                end_tag(output, "classification-us");
            }
            TechnicalInformationPart::TitleOfInvention(value) => {
                write_text_element(output, "title-of-invention", value)
            }
            TechnicalInformationPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "technical-information");
}

fn write_continuity_data(output: &mut String, value: &ContinuityData) {
    start_tag(output, "continuity-data", &value.attributes);
    for part in &value.parts {
        match part {
            ContinuityPart::NonProvisionalOfProvisional(value) => {
                let tag = value.relationship.as_deref().unwrap_or("continuation-of");
                start_tag(output, tag, &value.attributes);
                start_tag(output, "document-id", &[]);
                if let Some(doc_number) = &value.parent_doc_number {
                    write_text_element(output, "doc-number", doc_number);
                }
                if let Some(date) = &value.parent_date {
                    write_text_element(output, "document-date", date);
                }
                end_tag(output, "document-id");
                end_tag(output, tag);
            }
            ContinuityPart::RelatedDocument(value) => {
                let tag = value.relationship.as_deref().unwrap_or("related-document");
                start_tag(output, tag, &value.attributes);
                start_tag(output, "document-id", &[]);
                if let Some(doc_number) = &value.parent_doc_number {
                    write_text_element(output, "doc-number", doc_number);
                }
                if let Some(date) = &value.parent_date {
                    write_text_element(output, "document-date", date);
                }
                end_tag(output, "document-id");
                end_tag(output, tag);
            }
            ContinuityPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "continuity-data");
}

fn write_named_parties(
    output: &mut String,
    container_tag: &str,
    item_tag: &str,
    value: &NamedParties,
) {
    start_tag(output, container_tag, &value.attributes);
    for party in &value.parties {
        start_tag(output, item_tag, &party.attributes);
        if let Some(name) = &party.name {
            start_tag(output, "addressbook", &[]);
            write_text_element(output, "name", name);
            end_tag(output, "addressbook");
        }
        end_tag(output, item_tag);
    }
    end_tag(output, container_tag);
}

fn write_inventors(output: &mut String, value: &Inventors) {
    start_tag(output, "inventors", &value.attributes);
    for part in &value.parts {
        match part {
            InventorsPart::FirstNamedInventor(value) => {
                write_inventor(output, "first-named-inventor", value)
            }
            InventorsPart::Inventor(value) => write_inventor(output, "inventor", value),
            InventorsPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "inventors");
}

fn write_inventor(output: &mut String, tag: &str, value: &Inventor) {
    start_tag(output, tag, &value.attributes);
    start_tag(output, "name", &[]);
    if let Some(first_name) = &value.first_name {
        write_text_element(output, "given-name", first_name);
    }
    if let Some(last_name) = &value.last_name {
        write_text_element(output, "family-name", last_name);
    }
    end_tag(output, "name");
    if value.city.is_some() || value.state.is_some() || value.country.is_some() {
        start_tag(output, "residence", &[]);
        start_tag(output, "residence-us", &[]);
        if let Some(city) = &value.city {
            write_text_element(output, "city", city);
        }
        if let Some(state) = &value.state {
            write_text_element(output, "state", state);
        }
        if let Some(country) = &value.country {
            write_text_element(output, "country-code", country);
        }
        end_tag(output, "residence-us");
        end_tag(output, "residence");
    }
    end_tag(output, tag);
}

fn write_correspondence_address(output: &mut String, value: &CorrespondenceAddress) {
    start_tag(output, "correspondence-address", &value.attributes);
    for part in &value.parts {
        match part {
            CorrespondenceAddressPart::Name1(value) => write_text_element(output, "name-1", value),
            CorrespondenceAddressPart::Name2(value) => write_text_element(output, "name-2", value),
            CorrespondenceAddressPart::Address(value) => write_address(output, value),
            CorrespondenceAddressPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "correspondence-address");
}

fn write_address(output: &mut String, value: &Address) {
    start_tag(output, "address", &value.attributes);
    for (index, line) in value.lines.iter().enumerate() {
        write_text_element(output, &format!("address-{}", index + 1), line);
    }
    if let Some(city) = &value.city {
        write_text_element(output, "city", city);
    }
    if let Some(state) = &value.state {
        write_text_element(output, "state", state);
    }
    if let Some(postal_code) = &value.postal_code {
        write_text_element(output, "postalcode", postal_code);
    }
    if let Some(country) = &value.country {
        start_tag(output, "country", &[]);
        write_text_element(output, "country-code", country);
        end_tag(output, "country");
    }
    end_tag(output, "address");
}

fn write_description(output: &mut String, value: &Description) {
    start_tag(output, "subdoc-description", &value.attributes);
    for part in &value.parts {
        match part {
            DescriptionPart::CrossReferenceToRelatedApplications(value) => {
                start_tag(
                    output,
                    "cross-reference-to-related-applications",
                    &value.attributes,
                );
                for part in &value.parts {
                    match part {
                        CrossReferencePart::Heading(value) => write_heading(output, value),
                        CrossReferencePart::Paragraph(value) => write_paragraph(output, value),
                        CrossReferencePart::Opaque(value) => write_xml_fragment(output, &value.xml),
                    }
                }
                end_tag(output, "cross-reference-to-related-applications");
            }
            DescriptionPart::SummaryOfInvention(value) => write_description_container(
                output,
                "summary-of-invention",
                &value.attributes,
                &value.parts,
            ),
            DescriptionPart::BriefDescriptionOfDrawings(value) => write_description_container(
                output,
                "brief-description-of-drawings",
                &value.attributes,
                &value.parts,
            ),
            DescriptionPart::DetailedDescription(value) => write_description_container(
                output,
                "detailed-description",
                &value.attributes,
                &value.parts,
            ),
            DescriptionPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "subdoc-description");
}

fn write_description_container(
    output: &mut String,
    tag: &str,
    attributes: &[crate::model::opaque::XmlAttribute],
    parts: &[DescriptionSectionPart],
) {
    start_tag(output, tag, attributes);
    for part in parts {
        match part {
            DescriptionSectionPart::Section(value) => {
                start_tag(output, "section", &value.attributes);
                for part in &value.parts {
                    match part {
                        SectionPart::Heading(value) => write_heading(output, value),
                        SectionPart::Paragraph(value) => write_paragraph(output, value),
                        SectionPart::Opaque(value) => write_xml_fragment(output, &value.xml),
                    }
                }
                end_tag(output, "section");
            }
            DescriptionSectionPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, tag);
}

fn write_heading(output: &mut String, value: &Heading) {
    start_tag(output, "heading", &value.attributes);
    write_runs(output, &value.content);
    end_tag(output, "heading");
}

fn write_paragraph(output: &mut String, value: &Paragraph) {
    start_tag(output, "paragraph", &value.attributes);
    write_runs(output, &value.content);
    end_tag(output, "paragraph");
}

fn write_claims(output: &mut String, value: &Claims) {
    start_tag(output, "subdoc-claims", &value.attributes);
    for part in &value.parts {
        match part {
            ClaimsPart::Heading(value) => write_heading(output, value),
            ClaimsPart::Claim(value) => {
                start_tag(output, "claim", &value.attributes);
                write_runs(output, &value.content);
                end_tag(output, "claim");
            }
            ClaimsPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "subdoc-claims");
}

fn write_drawings(output: &mut String, value: &Drawings) {
    start_tag(output, "subdoc-drawings", &value.attributes);
    for part in &value.parts {
        match part {
            DrawingsPart::Heading(value) => write_heading(output, value),
            DrawingsPart::RepresentativeFigure(value) => {
                write_text_element(output, "representative-figure", value)
            }
            DrawingsPart::Figure(value) => {
                start_tag(output, "figure", &value.attributes);
                for part in &value.parts {
                    match part {
                        FigurePart::Image(value) => {
                            start_empty_tag(output, "image", &value.attributes)
                        }
                        FigurePart::Opaque(value) => write_xml_fragment(output, &value.xml),
                    }
                }
                end_tag(output, "figure");
            }
            DrawingsPart::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
    end_tag(output, "subdoc-drawings");
}

fn write_runs(output: &mut String, runs: &[Run]) {
    for run in runs {
        match run {
            Run::Text(value) => output.push_str(&escape_text(value)),
            Run::Number(value) => {
                write_inline_container(output, "number", &value.attributes, &value.content)
            }
            Run::CrossReference(value) => {
                write_inline_container(output, "cross-reference", &value.attributes, &value.content)
            }
            Run::FigureReference(value) => {
                write_inline_container(output, "figref", &value.attributes, &value.content)
            }
            Run::DependentClaimReference(value) => write_inline_container(
                output,
                "dependent-claim-reference",
                &value.attributes,
                &value.content,
            ),
            Run::Bold(value) => {
                write_inline_container(output, "bold", &value.attributes, &value.content)
            }
            Run::Italic(value) => {
                write_inline_container(output, "italic", &value.attributes, &value.content)
            }
            Run::Highlight(value) => {
                write_inline_container(output, "highlight", &value.attributes, &value.content)
            }
            Run::ClaimText(value) => {
                write_inline_container(output, "claim-text", &value.attributes, &value.content)
            }
            Run::Opaque(value) => write_xml_fragment(output, &value.xml),
        }
    }
}

fn write_inline_container(
    output: &mut String,
    tag: &str,
    attributes: &[crate::model::opaque::XmlAttribute],
    content: &[Run],
) {
    start_tag(output, tag, attributes);
    write_runs(output, content);
    end_tag(output, tag);
}

fn write_xml_fragment(output: &mut String, fragment: &crate::model::opaque::XmlFragment) {
    match fragment {
        crate::model::opaque::XmlFragment::Text(value) => {
            output.push_str(&escape_text(value));
        }
        crate::model::opaque::XmlFragment::Element {
            name,
            attributes,
            children,
        } => {
            let tag = qualified_name(name.prefix.as_deref(), &name.local_name);
            start_tag(output, &tag, attributes);
            for child in children {
                write_xml_fragment(output, child);
            }
            end_tag(output, &tag);
        }
    }
}

fn write_text_element(output: &mut String, tag: &str, value: &str) {
    start_tag(output, tag, &[]);
    output.push_str(&escape_text(value));
    end_tag(output, tag);
}

fn start_tag(output: &mut String, tag: &str, attributes: &[crate::model::opaque::XmlAttribute]) {
    output.push('<');
    output.push_str(tag);
    write_attributes(output, attributes);
    output.push('>');
}

fn start_empty_tag(
    output: &mut String,
    tag: &str,
    attributes: &[crate::model::opaque::XmlAttribute],
) {
    output.push('<');
    output.push_str(tag);
    write_attributes(output, attributes);
    output.push_str("/>");
}

fn end_tag(output: &mut String, tag: &str) {
    output.push_str("</");
    output.push_str(tag);
    output.push('>');
}

fn write_attributes(output: &mut String, attributes: &[crate::model::opaque::XmlAttribute]) {
    for attribute in attributes {
        output.push(' ');
        output.push_str(&qualified_name(
            attribute.prefix.as_deref(),
            &attribute.local_name,
        ));
        output.push_str("=\"");
        output.push_str(&escape_attribute(&attribute.value));
        output.push('"');
    }
}

fn qualified_name(prefix: Option<&str>, local_name: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}:{local_name}"),
        None => local_name.to_string(),
    }
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            '&' if looks_like_entity(&chars[index..]) => {
                escaped.push('&');
                index += 1;
            }
            '&' => {
                escaped.push_str("&amp;");
                index += 1;
            }
            '<' => {
                escaped.push_str("&lt;");
                index += 1;
            }
            '>' => {
                escaped.push_str("&gt;");
                index += 1;
            }
            character => {
                escaped.push(character);
                index += 1;
            }
        }
    }

    escaped
}

fn escape_attribute(value: &str) -> String {
    escape_text(value).replace('"', "&quot;")
}

fn looks_like_entity(chars: &[char]) -> bool {
    let Some(position) = chars.iter().position(|character| *character == ';') else {
        return false;
    };

    if position < 2 {
        return false;
    }

    chars[1..position]
        .iter()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '#' | '-' | '_'))
}
