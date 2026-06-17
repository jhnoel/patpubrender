//! Public-API coverage for the section-placeholder template override.

use patpubrender::model::bibliographic::{
    BibliographicInformation, BibliographicPart, TechnicalInformation, TechnicalInformationPart,
};
use patpubrender::model::claims::{Claim, Claims, ClaimsPart};
use patpubrender::model::description::{AbstractPart, AbstractSection, Paragraph};
use patpubrender::model::document::{DocumentPart, PatentDocument};
use patpubrender::model::runs::Run;
use patpubrender::{SourceFormat, render_markdown, render_markdown_with_template};

fn sample() -> PatentDocument {
    PatentDocument {
        source_format: SourceFormat::UsptoGrantV47,
        parts: vec![
            DocumentPart::BibliographicInformation(BibliographicInformation {
                parts: vec![BibliographicPart::TechnicalInformation(
                    TechnicalInformation {
                        parts: vec![TechnicalInformationPart::TitleOfInvention(
                            "Widget".to_string(),
                        )],
                        ..Default::default()
                    },
                )],
                ..Default::default()
            }),
            DocumentPart::AbstractSection(AbstractSection {
                parts: vec![AbstractPart::Paragraph(Paragraph {
                    content: vec![Run::Text("An abstract.".to_string())],
                    ..Default::default()
                })],
                ..Default::default()
            }),
            DocumentPart::Claims(Claims {
                parts: vec![ClaimsPart::Claim(Claim {
                    content: vec![Run::Text("A claim.".to_string())],
                    ..Default::default()
                })],
                ..Default::default()
            }),
        ],
        ..Default::default()
    }
}

#[test]
fn default_and_explicit_default_template_agree() {
    let doc = sample();
    let default = render_markdown(&doc);
    let templated = render_markdown_with_template(&doc, patpubrender::DEFAULT_TEMPLATE).unwrap();
    assert_eq!(default, templated);
}

#[test]
fn custom_template_reorders_and_drops_sections() {
    let doc = sample();
    // Claims first, no frontmatter, with a custom wrapper line.
    let out = render_markdown_with_template(&doc, "{{claims}}\n\n---\n\n# {{title}}").unwrap();
    let claims_at = out.find("## Claims").expect("claims present");
    let title_at = out.find("Widget").expect("title present");
    assert!(claims_at < title_at, "claims should precede title:\n{out}");
    assert!(
        !out.contains("publication_number"),
        "frontmatter omitted:\n{out}"
    );
}

#[test]
fn invalid_template_is_an_error() {
    let doc = sample();
    assert!(render_markdown_with_template(&doc, "{{nope}}").is_err());
}
