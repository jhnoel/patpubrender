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
use crate::model::opaque::{OpaqueBlock, OpaqueInline, XmlAttribute};
use crate::model::runs::{CrossReference, DependentClaimReference, InlineContainer, Number, Run};
use crate::source::detect::SourceFormat;
use crate::source::xml;

pub(crate) fn parse_document(
    input: &str,
    source_format: SourceFormat,
) -> Result<PatentDocument, ParseError> {
    let document = xml::parse_document(input)?;
    let root = xml::root_element(&document)?;

    if document.node_name(root) != Some("us-patent-application") {
        return Err(ParseError::UnsupportedStructure(
            "v4.x applications require us-patent-application root".to_string(),
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

pub(crate) fn write_document(doc: &PatentDocument) -> Result<String, SerializeError> {
    let mut output = String::new();
    xml::write_prolog(&mut output, &doc.prolog);
    xml::start_tag(&mut output, "us-patent-application", &doc.attributes);
    for part in &doc.parts {
        write_document_part(&mut output, part);
    }
    xml::end_tag(&mut output, "us-patent-application");
    Ok(output)
}

fn parse_top_level_part(document: &xmloxide::Document, node: NodeId) -> DocumentPart {
    match document.node_name(node) {
        Some("us-bibliographic-data-application") => {
            DocumentPart::BibliographicInformation(parse_bibliographic_information(document, node))
        }
        Some("abstract") => DocumentPart::AbstractSection(parse_abstract_section(document, node)),
        Some("description") => DocumentPart::Description(parse_description(document, node)),
        Some("claims") => DocumentPart::Claims(parse_claims(document, node)),
        Some("drawings") => DocumentPart::Drawings(parse_drawings(document, node)),
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
            Some("publication-reference") => {
                if let Some(document_id) = child_element(document, child, "document-id") {
                    parts.push(BibliographicPart::DocumentId(parse_document_id_v4(
                        document,
                        document_id,
                    )));
                } else {
                    parts.push(BibliographicPart::Opaque(opaque_block(document, child)));
                }
            }
            Some("application-reference") => parts.push(BibliographicPart::DomesticFilingData(
                parse_application_reference(document, child),
            )),
            Some("us-related-documents") => parts.push(BibliographicPart::ContinuityData(
                parse_continuity_data(document, child),
            )),
            Some("parties") | Some("us-parties") => parts.extend(parse_parties(document, child)),
            Some("assignees") => parts.push(BibliographicPart::Assignees(parse_named_parties(
                document,
                child,
                &["assignee"],
            ))),
            Some("us-application-series-code") => {
                if let Some(BibliographicPart::DomesticFilingData(value)) = parts.last_mut() {
                    value
                        .parts
                        .push(DomesticFilingPart::ApplicationNumberSeriesCode(
                            xml::text_content(document, child).trim().to_string(),
                        ));
                } else {
                    parts.push(BibliographicPart::DomesticFilingData(DomesticFilingData {
                        attributes: vec![],
                        parts: vec![DomesticFilingPart::ApplicationNumberSeriesCode(
                            xml::text_content(document, child).trim().to_string(),
                        )],
                    }));
                }
            }
            Some("classification-ipc") => {
                push_technical_information(
                    &mut parts,
                    TechnicalInformationPart::ClassificationIpc(parse_classification_ipc_legacy(
                        document, child,
                    )),
                );
            }
            Some("classifications-ipcr") => {
                push_technical_information(
                    &mut parts,
                    TechnicalInformationPart::ClassificationIpc(parse_classifications_ipcr(
                        document, child,
                    )),
                );
            }
            Some("classification-national") => {
                push_technical_information(
                    &mut parts,
                    TechnicalInformationPart::ClassificationUs(parse_classification_us(
                        document, child,
                    )),
                );
            }
            Some("invention-title") => {
                push_technical_information(
                    &mut parts,
                    TechnicalInformationPart::TitleOfInvention(
                        xml::text_content(document, child).trim().to_string(),
                    ),
                );
            }
            _ => parts.push(BibliographicPart::Opaque(opaque_block(document, child))),
        }
    }

    BibliographicInformation {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn push_technical_information(parts: &mut Vec<BibliographicPart>, part: TechnicalInformationPart) {
    if let Some(BibliographicPart::TechnicalInformation(value)) = parts.last_mut() {
        value.parts.push(part);
    } else {
        parts.push(BibliographicPart::TechnicalInformation(
            TechnicalInformation {
                attributes: vec![],
                parts: vec![part],
            },
        ));
    }
}

fn parse_document_id_v4(document: &xmloxide::Document, node: NodeId) -> DocumentId {
    DocumentId {
        doc_number: child_text(document, node, &["doc-number"]).unwrap_or_default(),
        kind_code: child_text(document, node, &["kind"]),
        document_date: child_text(document, node, &["date"]),
        country_code: child_text(document, node, &["country"]),
    }
}

fn parse_application_reference(document: &xmloxide::Document, node: NodeId) -> DomesticFilingData {
    let document_id = child_element(document, node, "document-id");
    let application_number = ApplicationNumber {
        attributes: vec![],
        appl_type: document.attribute(node, "appl-type").map(ToOwned::to_owned),
        doc_number: document_id.and_then(|value| child_text(document, value, &["doc-number"])),
    };
    let filing_date = document_id.and_then(|value| child_text(document, value, &["date"]));

    let mut parts = vec![DomesticFilingPart::ApplicationNumber(application_number)];
    if let Some(value) = filing_date {
        parts.push(DomesticFilingPart::FilingDate(value));
    }

    DomesticFilingData {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_continuity_data(document: &xmloxide::Document, node: NodeId) -> ContinuityData {
    let parts = xml::child_elements(document, node)
        .into_iter()
        .map(|child| match document.node_name(child) {
            Some("us-provisional-application") => ContinuityPart::RelatedDocument(
                parse_simple_related_document(document, child, "us-provisional-application"),
            ),
            Some("continuation") | Some("continuation-in-part") | Some("division") => {
                ContinuityPart::RelatedDocument(parse_related_document_relation(
                    document,
                    child,
                    document.node_name(child).unwrap_or("related-document"),
                ))
            }
            _ => ContinuityPart::Opaque(opaque_block(document, child)),
        })
        .collect();

    ContinuityData {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn parse_related_document_relation(
    document: &xmloxide::Document,
    node: NodeId,
    relationship: &str,
) -> RelatedDocument {
    let relation = child_element(document, node, "relation");
    let parent_doc = relation
        .and_then(|value| child_element(document, value, "parent-doc"))
        .and_then(|value| child_element(document, value, "document-id"));
    let child_doc = relation
        .and_then(|value| child_element(document, value, "child-doc"))
        .and_then(|value| child_element(document, value, "document-id"));

    RelatedDocument {
        attributes: xml::attributes(document, node),
        parent_doc_number: parent_doc
            .and_then(|value| child_text(document, value, &["doc-number"])),
        parent_date: parent_doc.and_then(|value| child_text(document, value, &["date"])),
        child_doc_number: child_doc.and_then(|value| child_text(document, value, &["doc-number"])),
        relationship: Some(relationship.to_string()),
    }
}

fn parse_simple_related_document(
    document: &xmloxide::Document,
    node: NodeId,
    relationship: &str,
) -> RelatedDocument {
    let document_id = child_element(document, node, "document-id");
    RelatedDocument {
        attributes: xml::attributes(document, node),
        parent_doc_number: document_id
            .and_then(|value| child_text(document, value, &["doc-number"])),
        parent_date: document_id.and_then(|value| child_text(document, value, &["date"])),
        child_doc_number: None,
        relationship: Some(relationship.to_string()),
    }
}

fn parse_parties(document: &xmloxide::Document, node: NodeId) -> Vec<BibliographicPart> {
    let mut parts = Vec::new();
    for child in xml::child_elements(document, node) {
        match document.node_name(child) {
            Some("applicants") | Some("us-applicants") => {
                parts.push(BibliographicPart::Applicants(parse_named_parties(
                    document,
                    child,
                    &["applicant", "us-applicant"],
                )))
            }
            Some("assignees") => parts.push(BibliographicPart::Assignees(parse_named_parties(
                document,
                child,
                &["assignee"],
            ))),
            _ => parts.push(BibliographicPart::Opaque(opaque_block(document, child))),
        }
    }
    parts
}

fn parse_named_parties(
    document: &xmloxide::Document,
    node: NodeId,
    item_names: &[&str],
) -> NamedParties {
    let parties = xml::child_elements(document, node)
        .into_iter()
        .filter(|child| {
            item_names
                .iter()
                .any(|name| document.node_name(*child) == Some(*name))
        })
        .map(|child| NamedParty {
            attributes: xml::attributes(document, child),
            name: parse_party_name(document, child),
        })
        .collect();

    NamedParties {
        attributes: xml::attributes(document, node),
        parties,
    }
}

fn parse_party_name(document: &xmloxide::Document, node: NodeId) -> Option<String> {
    let addressbook = child_element(document, node, "addressbook")?;
    if let Some(orgname) = child_text(document, addressbook, &["orgname", "name"]) {
        return Some(orgname);
    }

    let parts = [
        child_text(document, addressbook, &["first-name"]),
        child_text(document, addressbook, &["middle-name"]),
        child_text(document, addressbook, &["last-name"]),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    (!parts.is_empty()).then(|| parts.join(" "))
}

fn parse_classification_ipc_legacy(
    document: &xmloxide::Document,
    node: NodeId,
) -> ClassificationIpc {
    let main_classification = child_text(document, node, &["main-classification"]);
    let further_classification = child_elements_named(document, node, "further-classification")
        .into_iter()
        .map(|child| xml::text_content(document, child).trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();

    ClassificationIpc {
        attributes: xml::attributes(document, node),
        main_classification,
        further_classification,
    }
}

fn parse_classifications_ipcr(document: &xmloxide::Document, node: NodeId) -> ClassificationIpc {
    let mut values = child_elements_named(document, node, "classification-ipcr")
        .into_iter()
        .filter_map(|child| format_ipcr_symbol(document, child))
        .collect::<Vec<_>>();

    let main_classification = values.first().cloned();
    let further_classification = if values.is_empty() {
        vec![]
    } else {
        values.drain(1..).collect()
    };

    ClassificationIpc {
        attributes: xml::attributes(document, node),
        main_classification,
        further_classification,
    }
}

fn format_ipcr_symbol(document: &xmloxide::Document, node: NodeId) -> Option<String> {
    let section = child_text(document, node, &["section"])?;
    let class = child_text(document, node, &["class"])?;
    let subclass = child_text(document, node, &["subclass"])?;
    let main_group = child_text(document, node, &["main-group"])?;
    let subgroup = child_text(document, node, &["subgroup"])?;
    Some(format!(
        "{section}{class}{subclass}{}/{}",
        main_group.trim(),
        subgroup.trim(),
    ))
}

fn parse_classification_us(document: &xmloxide::Document, node: NodeId) -> ClassificationUs {
    ClassificationUs {
        attributes: xml::attributes(document, node),
        national_classification: child_text(document, node, &["main-classification"]),
        further_classification: child_elements_named(document, node, "further-classification")
            .into_iter()
            .map(|child| xml::text_content(document, child).trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
    }
}

fn parse_abstract_section(document: &xmloxide::Document, node: NodeId) -> AbstractSection {
    AbstractSection {
        attributes: xml::attributes(document, node),
        parts: xml::child_elements(document, node)
            .into_iter()
            .map(|child| match document.node_name(child) {
                Some("p") | Some("paragraph") => {
                    AbstractPart::Paragraph(parse_paragraph(document, child))
                }
                _ => AbstractPart::Opaque(opaque_block(document, child)),
            })
            .collect(),
    }
}

fn parse_description(document: &xmloxide::Document, node: NodeId) -> Description {
    let mut parts = Vec::new();
    let mut active = None;

    for child in xml::child_content_nodes(document, node) {
        match &document.node(child).kind {
            NodeKind::ProcessingInstruction { target, data } => {
                let opaque = DescriptionPart::Opaque(opaque_block(document, child));
                if is_tail_marker(data.as_deref()) {
                    flush_description_bucket(&mut parts, &mut active);
                    parts.push(opaque);
                    continue;
                }

                if let Some(kind) = marker_kind(target) {
                    flush_description_bucket(&mut parts, &mut active);
                    parts.push(opaque);
                    active = Some(DescriptionBucket::new(kind));
                } else if let Some(bucket) = active.as_mut() {
                    bucket.push_opaque(opaque_block(document, child));
                } else {
                    parts.push(opaque);
                }
            }
            NodeKind::Element { name, .. } => match name.as_str() {
                "heading" => {
                    let bucket = active.get_or_insert_with(|| {
                        DescriptionBucket::new(DescriptionBucketKind::Detailed)
                    });
                    bucket.push_heading(parse_heading(document, child));
                }
                "p" | "paragraph" => {
                    let bucket = active.get_or_insert_with(|| {
                        DescriptionBucket::new(DescriptionBucketKind::Detailed)
                    });
                    bucket.push_paragraph(parse_paragraph(document, child));
                }
                "description-of-drawings" => {
                    let bucket = active.get_or_insert_with(|| {
                        DescriptionBucket::new(DescriptionBucketKind::Brief)
                    });
                    push_description_children(document, child, bucket);
                }
                _ => {
                    if let Some(bucket) = active.as_mut() {
                        bucket.push_opaque(opaque_block(document, child));
                    } else {
                        parts.push(DescriptionPart::Opaque(opaque_block(document, child)));
                    }
                }
            },
            NodeKind::Text { content } | NodeKind::CData { content } => {
                if !content.trim().is_empty() {
                    if let Some(bucket) = active.as_mut() {
                        bucket.push_opaque(OpaqueBlock {
                            xml: xml::xml_fragment(document, child),
                        });
                    } else {
                        parts.push(DescriptionPart::Opaque(OpaqueBlock {
                            xml: xml::xml_fragment(document, child),
                        }));
                    }
                }
            }
            _ => {}
        }
    }

    flush_description_bucket(&mut parts, &mut active);

    Description {
        attributes: xml::attributes(document, node),
        parts,
    }
}

fn push_description_children(
    document: &xmloxide::Document,
    node: NodeId,
    bucket: &mut DescriptionBucket,
) {
    for child in xml::child_content_nodes(document, node) {
        match document.node_name(child) {
            Some("heading") => bucket.push_heading(parse_heading(document, child)),
            Some("p") | Some("paragraph") => {
                bucket.push_paragraph(parse_paragraph(document, child))
            }
            _ => bucket.push_opaque(opaque_block(document, child)),
        }
    }
}

fn marker_kind(target: &str) -> Option<DescriptionBucketKind> {
    let normalized = target.to_ascii_lowercase();
    match normalized.as_str() {
        "summary-of-invention" | "brfsum" => Some(DescriptionBucketKind::Summary),
        "brief-description-of-drawings" => Some(DescriptionBucketKind::Brief),
        "detailed-description" | "detdesc" => Some(DescriptionBucketKind::Detailed),
        _ => None,
    }
}

fn is_tail_marker(data: Option<&str>) -> bool {
    data.is_some_and(|value| value.contains("end=\"tail\""))
}

fn flush_description_bucket(
    parts: &mut Vec<DescriptionPart>,
    bucket: &mut Option<DescriptionBucket>,
) {
    if let Some(bucket) = bucket.take() {
        parts.push(bucket.finish());
    }
}

fn parse_claims(document: &xmloxide::Document, node: NodeId) -> Claims {
    Claims {
        attributes: xml::attributes(document, node),
        parts: xml::child_elements(document, node)
            .into_iter()
            .map(|child| match document.node_name(child) {
                Some("heading") => ClaimsPart::Heading(parse_heading(document, child)),
                Some("claim") => ClaimsPart::Claim(parse_claim(document, child)),
                _ => ClaimsPart::Opaque(opaque_block(document, child)),
            })
            .collect(),
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
    Drawings {
        attributes: xml::attributes(document, node),
        parts: xml::child_elements(document, node)
            .into_iter()
            .map(|child| match document.node_name(child) {
                Some("heading") => DrawingsPart::Heading(parse_heading(document, child)),
                Some("representative-figure") => DrawingsPart::RepresentativeFigure(
                    xml::text_content(document, child).trim().to_string(),
                ),
                Some("figure") => DrawingsPart::Figure(parse_figure(document, child)),
                _ => DrawingsPart::Opaque(opaque_block(document, child)),
            })
            .collect(),
    }
}

fn parse_figure(document: &xmloxide::Document, node: NodeId) -> Figure {
    Figure {
        attributes: xml::attributes(document, node),
        id: document.attribute(node, "id").map(ToOwned::to_owned),
        parts: xml::child_elements(document, node)
            .into_iter()
            .map(|child| match document.node_name(child) {
                Some("img") | Some("image") => FigurePart::Image(parse_image(document, child)),
                _ => FigurePart::Opaque(opaque_block(document, child)),
            })
            .collect(),
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

fn parse_heading(document: &xmloxide::Document, node: NodeId) -> Heading {
    Heading {
        attributes: xml::attributes(document, node),
        level: document
            .attribute(node, "level")
            .or_else(|| document.attribute(node, "lvl"))
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
            .attribute(node, "level")
            .or_else(|| document.attribute(node, "lvl"))
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
                "claim-ref" | "dependent-claim-reference" => {
                    runs.push(Run::DependentClaimReference(DependentClaimReference {
                        attributes: xml::attributes(document, child),
                        depends_on: document
                            .attribute(child, "depends_on")
                            .or_else(|| document.attribute(child, "idref"))
                            .map(ToOwned::to_owned),
                        content: parse_runs(document, child),
                    }))
                }
                "b" | "bold" => runs.push(Run::Bold(InlineContainer {
                    attributes: xml::attributes(document, child),
                    content: parse_runs(document, child),
                })),
                "i" | "italic" => runs.push(Run::Italic(InlineContainer {
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
            NodeKind::ProcessingInstruction { .. } => runs.push(Run::Opaque(OpaqueInline {
                xml: xml::xml_fragment(document, child),
            })),
            _ => {}
        }
    }

    runs
}

fn child_element(document: &xmloxide::Document, node: NodeId, name: &str) -> Option<NodeId> {
    xml::child_elements(document, node)
        .into_iter()
        .find(|child| document.node_name(*child) == Some(name))
}

fn child_elements_named(document: &xmloxide::Document, node: NodeId, name: &str) -> Vec<NodeId> {
    xml::child_elements(document, node)
        .into_iter()
        .filter(|child| document.node_name(*child) == Some(name))
        .collect()
}

fn child_text(document: &xmloxide::Document, node: NodeId, names: &[&str]) -> Option<String> {
    xml::child_elements(document, node)
        .into_iter()
        .find(|child| {
            names
                .iter()
                .any(|name| document.node_name(*child) == Some(*name))
        })
        .map(|child| xml::text_content(document, child).trim().to_string())
        .filter(|value| !value.is_empty())
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
        DocumentPart::AbstractSection(value) => {
            xml::start_tag(output, "abstract", &value.attributes);
            for part in &value.parts {
                match part {
                    AbstractPart::Paragraph(value) => write_paragraph(output, value, "p"),
                    AbstractPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
                }
            }
            xml::end_tag(output, "abstract");
        }
        DocumentPart::Description(value) => write_description(output, value),
        DocumentPart::Claims(value) => write_claims(output, value),
        DocumentPart::Drawings(value) => write_drawings(output, value),
        DocumentPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
    }
}

fn write_bibliographic_information(output: &mut String, value: &BibliographicInformation) {
    xml::start_tag(
        output,
        "us-bibliographic-data-application",
        &value.attributes,
    );
    for part in &value.parts {
        match part {
            BibliographicPart::DocumentId(value) => {
                xml::start_tag(output, "publication-reference", &[]);
                write_document_id(output, value);
                xml::end_tag(output, "publication-reference");
            }
            BibliographicPart::DomesticFilingData(value) => {
                write_domestic_filing_data(output, value)
            }
            BibliographicPart::ContinuityData(value) => write_continuity_data(output, value),
            BibliographicPart::Applicants(value) => {
                write_named_parties(output, "parties", "applicants", "applicant", value)
            }
            BibliographicPart::Assignees(value) => {
                write_named_parties(output, "", "assignees", "assignee", value)
            }
            BibliographicPart::TechnicalInformation(value) => {
                write_technical_information(output, value)
            }
            BibliographicPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
            _ => {}
        }
    }
    xml::end_tag(output, "us-bibliographic-data-application");
}

fn write_document_id(output: &mut String, value: &DocumentId) {
    xml::start_tag(output, "document-id", &[]);
    if let Some(value) = &value.country_code {
        xml::write_text_element(output, "country", value);
    }
    xml::write_text_element(output, "doc-number", &value.doc_number);
    if let Some(value) = &value.kind_code {
        xml::write_text_element(output, "kind", value);
    }
    if let Some(value) = &value.document_date {
        xml::write_text_element(output, "date", value);
    }
    xml::end_tag(output, "document-id");
}

fn write_domestic_filing_data(output: &mut String, value: &DomesticFilingData) {
    xml::start_tag(output, "application-reference", &value.attributes);
    xml::start_tag(output, "document-id", &[]);
    if let Some(number) = value.parts.iter().find_map(|part| match part {
        DomesticFilingPart::ApplicationNumber(number) => number.doc_number.as_deref(),
        _ => None,
    }) {
        xml::write_text_element(output, "doc-number", number);
    }
    if let Some(date) = value.parts.iter().find_map(|part| match part {
        DomesticFilingPart::FilingDate(value) => Some(value.as_str()),
        _ => None,
    }) {
        xml::write_text_element(output, "date", date);
    }
    xml::end_tag(output, "document-id");
    xml::end_tag(output, "application-reference");

    if let Some(series) = value.parts.iter().find_map(|part| match part {
        DomesticFilingPart::ApplicationNumberSeriesCode(value) => Some(value.as_str()),
        _ => None,
    }) {
        xml::write_text_element(output, "us-application-series-code", series);
    }
}

fn write_technical_information(output: &mut String, value: &TechnicalInformation) {
    for part in &value.parts {
        match part {
            TechnicalInformationPart::ClassificationIpc(value) => {
                xml::start_tag(output, "classification-ipc", &value.attributes);
                if let Some(main) = &value.main_classification {
                    xml::write_text_element(output, "main-classification", main);
                }
                for further in &value.further_classification {
                    xml::write_text_element(output, "further-classification", further);
                }
                xml::end_tag(output, "classification-ipc");
            }
            TechnicalInformationPart::ClassificationUs(value) => {
                xml::start_tag(output, "classification-national", &value.attributes);
                if let Some(main) = &value.national_classification {
                    xml::write_text_element(output, "main-classification", main);
                }
                for further in &value.further_classification {
                    xml::write_text_element(output, "further-classification", further);
                }
                xml::end_tag(output, "classification-national");
            }
            TechnicalInformationPart::TitleOfInvention(value) => {
                xml::write_text_element(output, "invention-title", value)
            }
            TechnicalInformationPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
}

fn write_continuity_data(output: &mut String, value: &ContinuityData) {
    xml::start_tag(output, "us-related-documents", &value.attributes);
    for part in &value.parts {
        match part {
            ContinuityPart::NonProvisionalOfProvisional(value)
            | ContinuityPart::RelatedDocument(value) => {
                let tag = value.relationship.as_deref().unwrap_or("related-document");
                xml::start_tag(output, tag, &value.attributes);
                if value.child_doc_number.is_some() {
                    xml::start_tag(output, "relation", &[]);
                    xml::start_tag(output, "parent-doc", &[]);
                    write_related_document_id(
                        output,
                        value.parent_doc_number.as_deref(),
                        value.parent_date.as_deref(),
                    );
                    xml::end_tag(output, "parent-doc");
                    xml::start_tag(output, "child-doc", &[]);
                    write_related_document_id(output, value.child_doc_number.as_deref(), None);
                    xml::end_tag(output, "child-doc");
                    xml::end_tag(output, "relation");
                } else {
                    write_related_document_id(
                        output,
                        value.parent_doc_number.as_deref(),
                        value.parent_date.as_deref(),
                    );
                }
                xml::end_tag(output, tag);
            }
            ContinuityPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    xml::end_tag(output, "us-related-documents");
}

fn write_related_document_id(output: &mut String, doc_number: Option<&str>, date: Option<&str>) {
    xml::start_tag(output, "document-id", &[]);
    if let Some(number) = doc_number {
        xml::write_text_element(output, "doc-number", number);
    }
    if let Some(date) = date {
        xml::write_text_element(output, "date", date);
    }
    xml::end_tag(output, "document-id");
}

fn write_named_parties(
    output: &mut String,
    outer_tag: &str,
    container_tag: &str,
    item_tag: &str,
    value: &NamedParties,
) {
    if !outer_tag.is_empty() {
        xml::start_tag(output, outer_tag, &[]);
    }
    xml::start_tag(output, container_tag, &value.attributes);
    for party in &value.parties {
        xml::start_tag(output, item_tag, &party.attributes);
        if let Some(name) = &party.name {
            xml::start_tag(output, "addressbook", &[]);
            if name.contains(' ') {
                xml::write_text_element(output, "name", name);
            } else {
                xml::write_text_element(output, "orgname", name);
            }
            xml::end_tag(output, "addressbook");
        }
        xml::end_tag(output, item_tag);
    }
    xml::end_tag(output, container_tag);
    if !outer_tag.is_empty() {
        xml::end_tag(output, outer_tag);
    }
}

fn write_description(output: &mut String, value: &Description) {
    xml::start_tag(output, "description", &value.attributes);
    for part in &value.parts {
        match part {
            DescriptionPart::SummaryOfInvention(value) => {
                write_description_section_parts(output, &value.parts)
            }
            DescriptionPart::BriefDescriptionOfDrawings(value) => {
                write_description_section_parts(output, &value.parts)
            }
            DescriptionPart::DetailedDescription(value) => {
                write_description_section_parts(output, &value.parts)
            }
            DescriptionPart::CrossReferenceToRelatedApplications(value) => {
                for part in &value.parts {
                    match part {
                        crate::model::description::CrossReferencePart::Heading(value) => {
                            write_heading(output, value)
                        }
                        crate::model::description::CrossReferencePart::Paragraph(value) => {
                            write_paragraph(output, value, "p")
                        }
                        crate::model::description::CrossReferencePart::Opaque(value) => {
                            xml::write_xml_fragment(output, &value.xml)
                        }
                    }
                }
            }
            DescriptionPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    xml::end_tag(output, "description");
}

fn write_description_section_parts(output: &mut String, parts: &[DescriptionSectionPart]) {
    for part in parts {
        match part {
            DescriptionSectionPart::Section(value) => {
                for part in &value.parts {
                    match part {
                        SectionPart::Heading(value) => write_heading(output, value),
                        SectionPart::Paragraph(value) => write_paragraph(output, value, "p"),
                        SectionPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
                    }
                }
            }
            DescriptionSectionPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
}

fn write_heading(output: &mut String, value: &Heading) {
    xml::start_tag(output, "heading", &value.attributes);
    write_runs(output, &value.content);
    xml::end_tag(output, "heading");
}

fn write_paragraph(output: &mut String, value: &Paragraph, tag: &str) {
    xml::start_tag(output, tag, &value.attributes);
    write_runs(output, &value.content);
    xml::end_tag(output, tag);
}

fn write_claims(output: &mut String, value: &Claims) {
    xml::start_tag(output, "claims", &value.attributes);
    for part in &value.parts {
        match part {
            ClaimsPart::Heading(value) => write_heading(output, value),
            ClaimsPart::Claim(value) => {
                xml::start_tag(output, "claim", &value.attributes);
                write_runs(output, &value.content);
                xml::end_tag(output, "claim");
            }
            ClaimsPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    xml::end_tag(output, "claims");
}

fn write_drawings(output: &mut String, value: &Drawings) {
    xml::start_tag(output, "drawings", &value.attributes);
    for part in &value.parts {
        match part {
            DrawingsPart::Heading(value) => write_heading(output, value),
            DrawingsPart::RepresentativeFigure(value) => {
                xml::write_text_element(output, "representative-figure", value)
            }
            DrawingsPart::Figure(value) => {
                xml::start_tag(output, "figure", &value.attributes);
                for part in &value.parts {
                    match part {
                        FigurePart::Image(value) => {
                            xml::start_empty_tag(output, "img", &value.attributes)
                        }
                        FigurePart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
                    }
                }
                xml::end_tag(output, "figure");
            }
            DrawingsPart::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
    xml::end_tag(output, "drawings");
}

fn write_runs(output: &mut String, runs: &[Run]) {
    for run in runs {
        match run {
            Run::Text(value) => output.push_str(&xml::escape_text(value)),
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
                write_inline_container(output, "b", &value.attributes, &value.content)
            }
            Run::Italic(value) => {
                write_inline_container(output, "i", &value.attributes, &value.content)
            }
            Run::Highlight(value) => {
                write_inline_container(output, "highlight", &value.attributes, &value.content)
            }
            Run::ClaimText(value) => {
                write_inline_container(output, "claim-text", &value.attributes, &value.content)
            }
            Run::Opaque(value) => xml::write_xml_fragment(output, &value.xml),
        }
    }
}

fn write_inline_container(
    output: &mut String,
    tag: &str,
    attributes: &[XmlAttribute],
    content: &[Run],
) {
    xml::start_tag(output, tag, attributes);
    write_runs(output, content);
    xml::end_tag(output, tag);
}

struct DescriptionBucket {
    kind: DescriptionBucketKind,
    parts: Vec<DescriptionSectionPart>,
    current_section: Option<Section>,
}

impl DescriptionBucket {
    fn new(kind: DescriptionBucketKind) -> Self {
        Self {
            kind,
            parts: vec![],
            current_section: None,
        }
    }

    fn push_heading(&mut self, heading: Heading) {
        self.flush_section();
        self.current_section = Some(Section {
            attributes: vec![],
            parts: vec![SectionPart::Heading(heading)],
        });
    }

    fn push_paragraph(&mut self, paragraph: Paragraph) {
        let section = self.current_section.get_or_insert_with(|| Section {
            attributes: vec![],
            parts: vec![],
        });
        section.parts.push(SectionPart::Paragraph(paragraph));
    }

    fn push_opaque(&mut self, value: OpaqueBlock) {
        if let Some(section) = self.current_section.as_mut() {
            section.parts.push(SectionPart::Opaque(value));
        } else {
            self.parts.push(DescriptionSectionPart::Opaque(value));
        }
    }

    fn finish(mut self) -> DescriptionPart {
        self.flush_section();
        match self.kind {
            DescriptionBucketKind::Summary => {
                DescriptionPart::SummaryOfInvention(SummaryOfInvention {
                    attributes: vec![],
                    parts: self.parts,
                })
            }
            DescriptionBucketKind::Brief => {
                DescriptionPart::BriefDescriptionOfDrawings(BriefDescriptionOfDrawings {
                    attributes: vec![],
                    parts: self.parts,
                })
            }
            DescriptionBucketKind::Detailed => {
                DescriptionPart::DetailedDescription(DetailedDescription {
                    attributes: vec![],
                    parts: self.parts,
                })
            }
        }
    }

    fn flush_section(&mut self) {
        if let Some(section) = self.current_section.take() {
            self.parts.push(DescriptionSectionPart::Section(section));
        }
    }
}

#[derive(Clone, Copy)]
enum DescriptionBucketKind {
    Summary,
    Brief,
    Detailed,
}
