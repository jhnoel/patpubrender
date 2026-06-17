use crate::model::document::PatentDocument;
use crate::model::{
    bibliographic::{
        BibliographicInformation, BibliographicPart, ClassificationIpc, ClassificationUs,
        ContinuityPart, DomesticFilingPart, Inventor, InventorsPart, NamedParties, RelatedDocument,
        TechnicalInformationPart,
    },
    claims::{Claim, ClaimsPart},
    description::{
        AbstractPart, CrossReferencePart, DescriptionPart, DescriptionSectionPart, Paragraph,
        SectionPart,
    },
    document::DocumentPart,
    opaque::{XmlAttribute, XmlFragment, XmlName},
    runs::Run,
};
use crate::render::template::{Sections, Template, TemplateError};

/// Render `value` to Markdown using the default layout.
pub fn render_markdown(value: &PatentDocument) -> String {
    Template::default().render(&build_sections(value))
}

/// Render `value` with a caller-supplied section-placeholder template.
///
/// The template is plain text with `{{placeholder}}` tokens — `frontmatter`,
/// `title`, `abstract`, `description`, or `claims`. See
/// [`crate::render::template`].
pub fn render_markdown_with_template(
    value: &PatentDocument,
    template: &str,
) -> Result<String, TemplateError> {
    Ok(Template::parse(template)?.render(&build_sections(value)))
}

fn build_sections(value: &PatentDocument) -> Sections {
    Sections {
        frontmatter: render_frontmatter(value),
        title: render_title(value),
        r#abstract: render_parts(value, |part| {
            matches!(part, DocumentPart::AbstractSection(_))
        }),
        description: render_parts(value, |part| matches!(part, DocumentPart::Description(_))),
        claims: render_parts(value, |part| matches!(part, DocumentPart::Claims(_))),
    }
}

fn render_frontmatter(value: &PatentDocument) -> String {
    let mut lines = Vec::new();
    push_frontmatter(&mut lines, value);
    trim_join(lines)
}

fn render_title(value: &PatentDocument) -> String {
    match title(value) {
        Some(title) => format!("# {title}"),
        None => String::new(),
    }
}

/// Render the document parts matching `keep`, in source order, each as its own
/// block, joined by a single blank line.
fn render_parts(value: &PatentDocument, keep: impl Fn(&DocumentPart) -> bool) -> String {
    value
        .parts
        .iter()
        .filter(|part| keep(part))
        .map(render_part_block)
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_part_block(part: &DocumentPart) -> String {
    let mut lines = Vec::new();
    match part {
        DocumentPart::AbstractSection(section) => {
            lines.push("## Abstract".to_string());
            for part in &section.parts {
                match part {
                    AbstractPart::Paragraph(paragraph) => {
                        push_paragraph(&mut lines, render_paragraph_text(paragraph));
                    }
                    AbstractPart::Opaque(value) => push_xml_fragment(&mut lines, &value.xml),
                }
            }
        }
        DocumentPart::Description(description) => {
            for part in &description.parts {
                render_description_part(&mut lines, part);
            }
        }
        DocumentPart::Claims(claims) => {
            lines.push("## Claims".to_string());
            for part in &claims.parts {
                match part {
                    ClaimsPart::Heading(heading) => {
                        push_non_empty_line(&mut lines, flatten_runs(&heading.content));
                    }
                    ClaimsPart::Claim(claim) => render_claim(&mut lines, claim),
                    ClaimsPart::Opaque(value) => push_xml_fragment(&mut lines, &value.xml),
                }
            }
        }
        DocumentPart::Opaque(value) => push_xml_fragment(&mut lines, &value.xml),
        _ => {}
    }
    trim_join(lines)
}

/// Drop leading and trailing blank lines, then join with newlines.
fn trim_join(mut lines: Vec<String>) -> String {
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn push_frontmatter(lines: &mut Vec<String>, document: &PatentDocument) {
    let publication_number = publication_number(document);
    let patent_number = patent_number(document);
    let priority_chain = priority_chain(document);
    let applicants = applicant_names(document);
    let assignees = assignee_names(document);

    lines.push("---".to_string());
    lines.push(format_yaml_scalar("filing_date", filing_date(document)));
    lines.push(format_yaml_scalar(
        "application_number",
        application_label(document),
    ));
    lines.push(format_yaml_scalar("publication_number", publication_number));
    lines.push(format_yaml_scalar(
        "publication_date",
        publication_date(document),
    ));
    lines.push(format_yaml_scalar("patent_number", patent_number));
    lines.push(format_yaml_scalar(
        "priority_date",
        priority_date(&priority_chain),
    ));
    push_yaml_string_list(lines, "ipc_classifications", &ipc_classifications(document));
    push_yaml_string_list(lines, "us_classifications", &us_classifications(document));
    if priority_chain.is_empty() {
        lines.push("priority_chain: []".to_string());
    } else {
        lines.push("priority_chain:".to_string());
        for entry in priority_chain {
            lines.push(format!(
                "  - relationship: {}",
                yaml_scalar(Some(entry.relationship))
            ));
            lines.push(format!(
                "    parent_doc_number: {}",
                yaml_scalar(compact_application_doc_number(entry.parent_doc_number))
            ));
            lines.push(format!(
                "    parent_date: {}",
                yaml_scalar(entry.parent_date)
            ));
            lines.push(format!(
                "    child_doc_number: {}",
                yaml_scalar(compact_application_doc_number(entry.child_doc_number))
            ));
        }
    }
    push_named_party_list(lines, "applicants", &applicants);
    push_named_party_list(lines, "assignees", &assignees);
    lines.push("---".to_string());
    lines.push(String::new());
}

fn render_description_part(lines: &mut Vec<String>, part: &DescriptionPart) {
    match part {
        DescriptionPart::CrossReferenceToRelatedApplications(section) => {
            lines.push("## Cross-Reference To Related Applications".to_string());
            for part in &section.parts {
                match part {
                    CrossReferencePart::Heading(_) => {}
                    CrossReferencePart::Paragraph(paragraph) => {
                        push_paragraph(lines, render_paragraph_text(paragraph));
                    }
                    CrossReferencePart::Opaque(value) => push_xml_fragment(lines, &value.xml),
                }
            }
            lines.push(String::new());
        }
        DescriptionPart::SummaryOfInvention(value) => {
            render_description_container(lines, "## Summary Of The Invention", &value.parts)
        }
        DescriptionPart::BriefDescriptionOfDrawings(value) => render_description_container(
            lines,
            "## Brief Description Of The Drawings",
            &value.parts,
        ),
        DescriptionPart::DetailedDescription(value) => {
            render_description_container(lines, "## Description Of The Invention", &value.parts)
        }
        DescriptionPart::Opaque(value) => {
            push_xml_fragment(lines, &value.xml);
            lines.push(String::new());
        }
    }
}

fn render_description_container(
    lines: &mut Vec<String>,
    heading: &str,
    parts: &[DescriptionSectionPart],
) {
    lines.push(heading.to_string());
    for part in parts {
        if let DescriptionSectionPart::Section(section) = part {
            for part in &section.parts {
                match part {
                    SectionPart::Heading(heading) => {
                        push_non_empty_line(
                            lines,
                            format!("### {}", flatten_runs(&heading.content).trim()),
                        );
                    }
                    SectionPart::Paragraph(paragraph) => {
                        push_paragraph(lines, render_paragraph_text(paragraph));
                    }
                    SectionPart::Opaque(value) => push_xml_fragment(lines, &value.xml),
                }
            }
        } else if let DescriptionSectionPart::Opaque(value) = part {
            push_xml_fragment(lines, &value.xml);
        }
    }
    lines.push(String::new());
}

fn render_claim(lines: &mut Vec<String>, claim: &Claim) {
    lines.push("```claim".to_string());
    lines.push(normalize_claim_text(&render_claim_runs(
        &claim.content,
        0,
        false,
    )));
    lines.push("```".to_string());
}

fn render_claim_runs(runs: &[Run], indent: usize, nested_claim_text: bool) -> String {
    let mut output = String::new();
    for run in runs {
        match run {
            Run::ClaimText(container) => {
                let next_indent = if nested_claim_text {
                    indent + 2
                } else {
                    indent
                };
                if nested_claim_text && !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&render_claim_runs(&container.content, next_indent, true));
            }
            Run::Text(value) => output.push_str(&apply_indent(&decode_entities(value), indent)),
            Run::DependentClaimReference(reference) => {
                output.push_str(&render_claim_runs(&reference.content, indent, true))
            }
            Run::Number(value) => output.push_str(&render_claim_runs(&value.content, indent, true)),
            Run::CrossReference(value) => {
                output.push_str(&render_claim_runs(&value.content, indent, true))
            }
            Run::FigureReference(value) => {
                output.push_str(&render_claim_runs(&value.content, indent, true))
            }
            Run::Bold(value) | Run::Italic(value) | Run::Highlight(value) => {
                output.push_str(&render_claim_runs(&value.content, indent, true))
            }
            Run::Opaque(value) => {
                output.push_str(&apply_indent(&render_xml_fragment(&value.xml), indent))
            }
        }
    }
    output
}

fn apply_indent(value: &str, indent: usize) -> String {
    if indent == 0 {
        return value.to_string();
    }

    let prefix = " ".repeat(indent);
    let mut result = String::new();
    for (index, part) in value.split('\n').enumerate() {
        if index > 0 {
            result.push('\n');
        }
        if !part.is_empty() {
            result.push_str(&prefix);
        }
        result.push_str(part);
    }
    result
}

fn flatten_runs(runs: &[Run]) -> String {
    let mut output = String::new();
    for run in runs {
        match run {
            Run::Text(value) => output.push_str(&decode_entities(value)),
            Run::Number(value) => output.push_str(&flatten_runs(&value.content)),
            Run::CrossReference(value) => output.push_str(&flatten_runs(&value.content)),
            Run::FigureReference(value) => output.push_str(&flatten_runs_plain(&value.content)),
            Run::DependentClaimReference(value) => output.push_str(&flatten_runs(&value.content)),
            Run::Bold(value) => {
                output.push_str("**");
                output.push_str(&flatten_runs(&value.content));
                output.push_str("**");
            }
            Run::Italic(value) => {
                output.push('*');
                output.push_str(&flatten_runs(&value.content));
                output.push('*');
            }
            Run::Highlight(value) | Run::ClaimText(value) => {
                output.push_str(&flatten_runs(&value.content))
            }
            Run::Opaque(value) => output.push_str(&render_xml_fragment(&value.xml)),
        }
    }
    output
}

pub(crate) fn flatten_runs_plain(runs: &[Run]) -> String {
    let mut output = String::new();
    for run in runs {
        match run {
            Run::Text(value) => output.push_str(&decode_entities(value)),
            Run::Number(value) => output.push_str(&flatten_runs_plain(&value.content)),
            Run::CrossReference(value) | Run::FigureReference(value) => {
                output.push_str(&flatten_runs_plain(&value.content))
            }
            Run::DependentClaimReference(value) => {
                output.push_str(&flatten_runs_plain(&value.content))
            }
            Run::Bold(value)
            | Run::Italic(value)
            | Run::Highlight(value)
            | Run::ClaimText(value) => output.push_str(&flatten_runs_plain(&value.content)),
            Run::Opaque(value) => output.push_str(&render_xml_fragment(&value.xml)),
        }
    }
    output
}

fn render_paragraph_text(paragraph: &Paragraph) -> String {
    let text = flatten_runs(&paragraph.content);
    if let Some(number) = paragraph_number(paragraph) {
        let trimmed = text.trim_start();
        let prefix = format!("[{number}]");
        if trimmed.starts_with(&prefix) {
            text
        } else {
            format!("{prefix} {trimmed}")
        }
    } else {
        text
    }
}

fn paragraph_number(paragraph: &Paragraph) -> Option<&str> {
    paragraph
        .attributes
        .iter()
        .find(|attribute| attribute.prefix.is_none() && attribute.local_name == "num")
        .map(|attribute| attribute.value.as_str())
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
}

fn push_non_empty_line(lines: &mut Vec<String>, value: String) {
    let trimmed = value.trim().to_string();
    if !trimmed.is_empty() {
        lines.push(trimmed);
    }
}

fn push_paragraph(lines: &mut Vec<String>, value: String) {
    let trimmed = value.trim().to_string();
    if !trimmed.is_empty() {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(trimmed);
    }
}

fn push_xml_fragment(lines: &mut Vec<String>, fragment: &XmlFragment) {
    push_non_empty_line(lines, render_xml_fragment(fragment));
}

fn render_xml_fragment(fragment: &XmlFragment) -> String {
    match fragment {
        XmlFragment::Text(value) => decode_entities(value),
        XmlFragment::Element {
            name,
            attributes,
            children,
        } => {
            let tag = render_xml_name(name);
            let mut output = String::new();
            output.push('<');
            output.push_str(&tag);
            for attribute in attributes {
                output.push(' ');
                output.push_str(&render_xml_attribute(attribute));
            }
            output.push('>');
            for child in children {
                output.push_str(&render_xml_fragment(child));
            }
            output.push_str("</");
            output.push_str(&tag);
            output.push('>');
            output
        }
    }
}

fn render_xml_name(name: &XmlName) -> String {
    match &name.prefix {
        Some(prefix) => format!("{prefix}:{}", name.local_name),
        None => name.local_name.clone(),
    }
}

fn render_xml_attribute(attribute: &XmlAttribute) -> String {
    let name = match &attribute.prefix {
        Some(prefix) => format!("{prefix}:{}", attribute.local_name),
        None => attribute.local_name.clone(),
    };
    format!("{name}=\"{}\"", attribute.value)
}

fn decode_entities(value: &str) -> String {
    let replacements = [
        ("&lsqb;", "["),
        ("&rsqb;", "]"),
        ("&ldquo;", "\""),
        ("&rdquo;", "\""),
        ("&lsquo;", "'"),
        ("&rsquo;", "'"),
        ("&mdash;", "-"),
        ("&ndash;", "-"),
        ("&apos;", "'"),
        ("&quot;", "\""),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&Prime;", "\""),
        ("&prime;", "'"),
        ("&equals;", "="),
        ("&deg;", " deg"),
    ];

    replacements
        .iter()
        .fold(value.to_string(), |current, (entity, decoded)| {
            current.replace(entity, decoded)
        })
}

fn normalize_claim_text(value: &str) -> String {
    value
        .lines()
        .map(normalize_claim_line)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn normalize_claim_line(line: &str) -> String {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{}{}", " ".repeat(indent), normalized)
}

pub(crate) fn title(document: &PatentDocument) -> Option<String> {
    bibliographic(document).and_then(|info| {
        info.parts.iter().find_map(|part| match part {
            BibliographicPart::TechnicalInformation(value) => {
                value.parts.iter().find_map(|part| match part {
                    TechnicalInformationPart::TitleOfInvention(value) => Some(value.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
    })
}

pub(crate) fn publication_date(document: &PatentDocument) -> Option<String> {
    document_id(document).and_then(|value| value.document_date.clone())
}

fn application_label(document: &PatentDocument) -> Option<String> {
    bibliographic(document).and_then(|info| {
        info.parts.iter().find_map(|part| match part {
            BibliographicPart::DomesticFilingData(value) => {
                let series = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::ApplicationNumberSeriesCode(value) => Some(value.clone()),
                    _ => None,
                });
                let number = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::ApplicationNumber(value) => value.doc_number.clone(),
                    _ => None,
                })?;
                let number = number.trim();
                let series = series
                    .map(|series| series.trim().to_string())
                    .filter(|series| !series.is_empty());
                Some(match series {
                    Some(series) => {
                        let serial = if number.len() >= 8 {
                            number.strip_prefix(&series).unwrap_or(number)
                        } else {
                            number
                        };
                        format!("{series}/{serial}")
                    }
                    None => number.to_string(),
                })
            }
            _ => None,
        })
    })
}

pub(crate) fn filing_date(document: &PatentDocument) -> Option<String> {
    bibliographic(document).and_then(|info| {
        info.parts.iter().find_map(|part| match part {
            BibliographicPart::DomesticFilingData(value) => {
                value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::FilingDate(value) => Some(value.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
    })
}

pub(crate) fn ipc_classifications(document: &PatentDocument) -> Vec<String> {
    bibliographic(document)
        .into_iter()
        .flat_map(|info| info.parts.iter())
        .filter_map(|part| match part {
            BibliographicPart::TechnicalInformation(value) => Some(value),
            _ => None,
        })
        .flat_map(|value| value.parts.iter())
        .filter_map(|part| match part {
            TechnicalInformationPart::ClassificationIpc(value) => Some(value),
            _ => None,
        })
        .flat_map(classification_ipc_values)
        .fold(Vec::new(), dedupe_push)
}

pub(crate) fn us_classifications(document: &PatentDocument) -> Vec<String> {
    bibliographic(document)
        .into_iter()
        .flat_map(|info| info.parts.iter())
        .filter_map(|part| match part {
            BibliographicPart::TechnicalInformation(value) => Some(value),
            _ => None,
        })
        .flat_map(|value| value.parts.iter())
        .filter_map(|part| match part {
            TechnicalInformationPart::ClassificationUs(value) => Some(value),
            _ => None,
        })
        .flat_map(classification_us_values)
        .fold(Vec::new(), dedupe_push)
}

pub(crate) fn publication_number(document: &PatentDocument) -> Option<String> {
    let document_id = document_id(document)?;
    (!is_patent_grant(document_id)).then(|| compact_document_number(document, document_id))
}

pub(crate) fn patent_number(document: &PatentDocument) -> Option<String> {
    let document_id = document_id(document)?;
    is_patent_grant(document_id).then(|| compact_document_number(document, document_id))
}

fn priority_chain(document: &PatentDocument) -> Vec<PriorityEntry> {
    let Some(info) = bibliographic(document) else {
        return vec![];
    };

    info.parts
        .iter()
        .flat_map(|part| match part {
            BibliographicPart::ContinuityData(value) => value
                .parts
                .iter()
                .filter_map(|part| match part {
                    ContinuityPart::NonProvisionalOfProvisional(value) => {
                        Some(PriorityEntry::from_related_document(value))
                    }
                    ContinuityPart::RelatedDocument(value) => {
                        Some(PriorityEntry::from_related_document(value))
                    }
                    ContinuityPart::Opaque(_) => None,
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect()
}

fn priority_date(entries: &[PriorityEntry]) -> Option<String> {
    entries
        .iter()
        .filter_map(|entry| entry.parent_date.clone())
        .min()
}

pub(crate) fn applicant_names(document: &PatentDocument) -> Vec<String> {
    let mut names = typed_named_parties(document, PartyKind::Applicant);
    if names.is_empty() {
        names = bibliographic(document)
            .map(opaque_bibliographic_fragments)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|fragment| collect_party_names(&fragment, PartyKind::Applicant))
            .fold(Vec::new(), dedupe_push);
    }
    names
}

pub(crate) fn assignee_names(document: &PatentDocument) -> Vec<String> {
    let mut names = typed_named_parties(document, PartyKind::Assignee);
    if names.is_empty() {
        names = bibliographic(document)
            .map(opaque_bibliographic_fragments)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|fragment| collect_party_names(&fragment, PartyKind::Assignee))
            .fold(Vec::new(), dedupe_push);
    }
    names
}

pub(crate) fn inventor_names(document: &PatentDocument) -> Vec<String> {
    let mut names = bibliographic(document)
        .into_iter()
        .flat_map(|info| info.parts.iter())
        .filter_map(|part| match part {
            BibliographicPart::Inventors(value) => Some(value),
            _ => None,
        })
        .flat_map(|value| value.parts.iter())
        .filter_map(|part| match part {
            InventorsPart::FirstNamedInventor(inventor) | InventorsPart::Inventor(inventor) => {
                inventor_full_name(inventor)
            }
            InventorsPart::Opaque(_) => None,
        })
        .fold(Vec::new(), dedupe_push);
    if names.is_empty() {
        names = bibliographic(document)
            .map(opaque_bibliographic_fragments)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|fragment| collect_party_names(&fragment, PartyKind::Inventor))
            .fold(Vec::new(), dedupe_push);
    }
    names
}

fn inventor_full_name(inventor: &Inventor) -> Option<String> {
    let joined = [inventor.first_name.clone(), inventor.last_name.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = joined.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Application number in the bare numeric form the record-dataset tables key
/// on, e.g. `16789012`. Modern (v4.x) doc-numbers already carry the full
/// 8-digit number including the series code; only shorter legacy serials need
/// the separate series code prepended.
pub(crate) fn bare_application_number(document: &PatentDocument) -> Option<String> {
    let (series, number) = bibliographic(document).and_then(|info| {
        info.parts.iter().find_map(|part| match part {
            BibliographicPart::DomesticFilingData(value) => {
                let series = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::ApplicationNumberSeriesCode(value) => Some(value.clone()),
                    _ => None,
                });
                let number = value.parts.iter().find_map(|part| match part {
                    DomesticFilingPart::ApplicationNumber(value) => value.doc_number.clone(),
                    _ => None,
                })?;
                Some((series, number))
            }
            _ => None,
        })
    })?;

    // Keep digits raw (no zero-stripping): the record dataset and the
    // Go-ingested publication_documents rows both store zero-padded 8-digit
    // numbers (e.g. 09725796), and the current-biblio join matches on them.
    let number = number.trim();
    if !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(number.to_string());
    }
    match series {
        Some(series) if number.len() < 8 && series.bytes().all(|byte| byte.is_ascii_digit()) => {
            Some(format!("{series}{number}"))
        }
        _ => Some(number.to_string()),
    }
}

pub(crate) fn earliest_priority_date(document: &PatentDocument) -> Option<String> {
    priority_date(&priority_chain(document))
}

fn typed_named_parties(document: &PatentDocument, kind: PartyKind) -> Vec<String> {
    bibliographic(document)
        .into_iter()
        .flat_map(|info| info.parts.iter())
        .filter_map(|part| match (kind, part) {
            (PartyKind::Applicant, BibliographicPart::Applicants(value)) => Some(value),
            (PartyKind::Assignee, BibliographicPart::Assignees(value)) => Some(value),
            _ => None,
        })
        .flat_map(named_party_values)
        .fold(Vec::new(), dedupe_push)
}

fn named_party_values(value: &NamedParties) -> Vec<String> {
    value
        .parties
        .iter()
        .filter_map(|party| party.name.clone())
        .collect()
}

pub(crate) fn document_id(
    document: &PatentDocument,
) -> Option<&crate::model::bibliographic::DocumentId> {
    bibliographic(document).and_then(|info| {
        info.parts.iter().find_map(|part| match part {
            BibliographicPart::DocumentId(value) => Some(value),
            _ => None,
        })
    })
}

fn bibliographic(document: &PatentDocument) -> Option<&BibliographicInformation> {
    document.parts.iter().find_map(|part| match part {
        DocumentPart::BibliographicInformation(value) => Some(value),
        _ => None,
    })
}

fn opaque_bibliographic_fragments(info: &BibliographicInformation) -> Vec<XmlFragment> {
    info.parts
        .iter()
        .filter_map(|part| match part {
            BibliographicPart::Opaque(value) => Some(value.xml.clone()),
            _ => None,
        })
        .collect()
}

fn compact_document_number(
    document: &PatentDocument,
    value: &crate::model::bibliographic::DocumentId,
) -> String {
    format!(
        "{}{}{}",
        value
            .country_code
            .clone()
            .unwrap_or_else(|| "US".to_string()),
        rendered_document_number(document, value),
        value.kind_code.clone().unwrap_or_default()
    )
}

fn rendered_document_number(
    _document: &PatentDocument,
    value: &crate::model::bibliographic::DocumentId,
) -> String {
    if is_patent_grant(value) && value.doc_number.bytes().all(|byte| byte.is_ascii_digit()) {
        normalize_numeric_identifier(&value.doc_number)
    } else {
        value.doc_number.clone()
    }
}

fn normalize_numeric_identifier(value: &str) -> String {
    let trimmed = value.trim();
    let normalized = trimmed.trim_start_matches('0');
    if normalized.is_empty() {
        "0".to_string()
    } else {
        normalized.to_string()
    }
}

fn compact_application_doc_number(value: Option<String>) -> Option<String> {
    value.map(|value| compact_application_doc_number_str(&value))
}

fn compact_application_doc_number_str(value: &str) -> String {
    let trimmed = value.trim();
    match trimmed.split_once('/') {
        Some((series, number))
            if !series.is_empty()
                && !number.is_empty()
                && series.bytes().all(|byte| byte.is_ascii_digit())
                && number.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            normalize_numeric_identifier(&format!("{series}{number}"))
        }
        _ => trimmed.to_string(),
    }
}

fn is_patent_grant(value: &crate::model::bibliographic::DocumentId) -> bool {
    value
        .kind_code
        .as_deref()
        .is_some_and(|kind| kind.starts_with('B') || kind.starts_with('E') || kind == "S")
}

fn format_yaml_scalar(key: &str, value: Option<String>) -> String {
    format!("{key}: {}", yaml_scalar(value))
}

fn yaml_scalar(value: Option<String>) -> String {
    match value {
        Some(value) => value,
        None => "null".to_string(),
    }
}

fn push_named_party_list(lines: &mut Vec<String>, key: &str, values: &[String]) {
    if values.is_empty() {
        lines.push(format!("{key}: []"));
        return;
    }

    lines.push(format!("{key}:"));
    for value in values {
        lines.push(format!("  - {}", yaml_scalar(Some(value.clone()))));
    }
}

fn push_yaml_string_list(lines: &mut Vec<String>, key: &str, values: &[String]) {
    if values.is_empty() {
        lines.push(format!("{key}: []"));
        return;
    }

    lines.push(format!("{key}:"));
    for value in values {
        lines.push(format!("  - {}", yaml_scalar(Some(value.clone()))));
    }
}

fn classification_ipc_values(value: &ClassificationIpc) -> Vec<String> {
    [value.main_classification.clone()]
        .into_iter()
        .flatten()
        .chain(value.further_classification.iter().cloned())
        .collect()
}

fn classification_us_values(value: &ClassificationUs) -> Vec<String> {
    [value.national_classification.clone()]
        .into_iter()
        .flatten()
        .chain(value.further_classification.iter().cloned())
        .collect()
}

fn collect_party_names(fragment: &XmlFragment, party_kind: PartyKind) -> Vec<String> {
    match fragment {
        XmlFragment::Text(_) => vec![],
        XmlFragment::Element { name, children, .. } => {
            if party_kind.matches_party_element(&name.local_name) {
                extract_party_name(fragment).into_iter().collect()
            } else {
                children
                    .iter()
                    .flat_map(|child| collect_party_names(child, party_kind))
                    .collect()
            }
        }
    }
}

fn extract_party_name(fragment: &XmlFragment) -> Option<String> {
    first_descendant_text(fragment, &["addressbook", "orgname"])
        .or_else(|| first_descendant_text(fragment, &["organization-name"]))
        .or_else(|| first_descendant_text(fragment, &["orgname"]))
        .or_else(|| combined_descendant_name(fragment, "addressbook"))
        .or_else(|| combined_descendant_name(fragment, "name"))
        .or_else(|| combined_st32_name(fragment))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// ST.32-era (grant v2.5) party names: `<NAM><FNM>first</FNM><SNM>last</SNM></NAM>`,
/// with `<ONM>` carrying organization names.
fn combined_st32_name(fragment: &XmlFragment) -> Option<String> {
    let container = find_descendant_element(fragment, "NAM")?;
    if let Some(org) = first_descendant_text(container, &["ONM"]) {
        return Some(org);
    }
    let first = first_descendant_text(container, &["FNM"]);
    let last = first_descendant_text(container, &["SNM"]);
    let joined = [first, last]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    (!joined.trim().is_empty()).then_some(joined)
}

fn combined_descendant_name(fragment: &XmlFragment, container_name: &str) -> Option<String> {
    let container = find_descendant_element(fragment, container_name)?;
    let first = first_descendant_text(container, &["first-name"])
        .or_else(|| first_descendant_text(container, &["given-name"]));
    let last = first_descendant_text(container, &["last-name"])
        .or_else(|| first_descendant_text(container, &["family-name"]));

    let joined = [first, last]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

    (!joined.trim().is_empty()).then_some(joined)
}

fn first_descendant_text(fragment: &XmlFragment, path: &[&str]) -> Option<String> {
    let target = find_path(fragment, path)?;
    flattened_text(target)
}

fn find_path<'a>(fragment: &'a XmlFragment, path: &[&str]) -> Option<&'a XmlFragment> {
    if path.is_empty() {
        return Some(fragment);
    }

    match fragment {
        XmlFragment::Text(_) => None,
        XmlFragment::Element { children, .. } => children.iter().find_map(|child| match child {
            XmlFragment::Element { name, .. } if name.local_name == path[0] => {
                find_path(child, &path[1..])
            }
            _ => None,
        }),
    }
}

fn find_descendant_element<'a>(fragment: &'a XmlFragment, target: &str) -> Option<&'a XmlFragment> {
    match fragment {
        XmlFragment::Text(_) => None,
        XmlFragment::Element { name, children, .. } => {
            if name.local_name == target {
                Some(fragment)
            } else {
                children
                    .iter()
                    .find_map(|child| find_descendant_element(child, target))
            }
        }
    }
}

fn flattened_text(fragment: &XmlFragment) -> Option<String> {
    let mut output = String::new();
    collect_text(fragment, &mut output);
    let trimmed = output.trim();
    (!trimmed.is_empty()).then(|| decode_entities(trimmed))
}

fn collect_text(fragment: &XmlFragment, output: &mut String) {
    match fragment {
        XmlFragment::Text(value) => output.push_str(value),
        XmlFragment::Element { children, .. } => {
            for child in children {
                collect_text(child, output);
            }
        }
    }
}

fn dedupe_push(mut values: Vec<String>, value: String) -> Vec<String> {
    if !values.contains(&value) {
        values.push(value);
    }
    values
}

#[derive(Debug, Clone)]
struct PriorityEntry {
    relationship: String,
    parent_doc_number: Option<String>,
    parent_date: Option<String>,
    child_doc_number: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum PartyKind {
    Applicant,
    Assignee,
    Inventor,
}

impl PartyKind {
    fn matches_party_element(self, local_name: &str) -> bool {
        match self {
            Self::Applicant => matches!(local_name, "applicant" | "us-applicant"),
            Self::Assignee => local_name == "assignee",
            Self::Inventor => {
                matches!(
                    local_name,
                    "inventor" | "us-inventor" | "first-named-inventor" | "B721"
                )
            }
        }
    }
}

impl PriorityEntry {
    fn from_related_document(value: &RelatedDocument) -> Self {
        Self {
            relationship: value
                .relationship
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            parent_doc_number: value.parent_doc_number.clone(),
            parent_date: value.parent_date.clone(),
            child_doc_number: value.child_doc_number.clone(),
        }
    }
}

#[cfg(test)]
mod application_number_tests {
    use super::bare_application_number;
    use crate::model::bibliographic::{
        ApplicationNumber, BibliographicInformation, BibliographicPart, DomesticFilingData,
        DomesticFilingPart,
    };
    use crate::model::document::{DocumentPart, PatentDocument};

    fn doc(doc_number: &str, series: Option<&str>) -> PatentDocument {
        let mut parts = vec![DomesticFilingPart::ApplicationNumber(ApplicationNumber {
            doc_number: Some(doc_number.to_string()),
            ..Default::default()
        })];
        if let Some(series) = series {
            parts.push(DomesticFilingPart::ApplicationNumberSeriesCode(
                series.to_string(),
            ));
        }
        PatentDocument {
            parts: vec![DocumentPart::BibliographicInformation(
                BibliographicInformation {
                    parts: vec![BibliographicPart::DomesticFilingData(DomesticFilingData {
                        parts,
                        ..Default::default()
                    })],
                    ..Default::default()
                },
            )],
            ..Default::default()
        }
    }

    #[test]
    fn bare_key_for_modern_8digit_doc_number_is_not_doubled() {
        // doc-number already includes the series -> the canonical key is the
        // doc-number verbatim, never series prepended again.
        assert_eq!(
            bare_application_number(&doc("14633232", Some("14"))).as_deref(),
            Some("14633232")
        );
    }

    #[test]
    fn bare_key_for_legacy_serial_only_joins_series() {
        assert_eq!(
            bare_application_number(&doc("633232", Some("14"))).as_deref(),
            Some("14633232")
        );
    }

    #[test]
    fn bare_key_without_series_is_doc_number() {
        assert_eq!(
            bare_application_number(&doc("14633232", None)).as_deref(),
            Some("14633232")
        );
    }
}
