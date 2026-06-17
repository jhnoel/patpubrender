//! Build a small patent document, serialize it to USPTO XML, and print it.
//!
//! Demonstrates the model → XML direction and doubles as a way to produce a
//! parseable fixture: `cargo run --example roundtrip_xml > sample.xml`.

use patpubrender::model::bibliographic::{
    BibliographicInformation, BibliographicPart, TechnicalInformation, TechnicalInformationPart,
};
use patpubrender::model::claims::{Claim, Claims, ClaimsPart};
use patpubrender::model::description::{AbstractPart, AbstractSection, Paragraph};
use patpubrender::model::document::{DocumentPart, PatentDocument};
use patpubrender::model::opaque::XmlAttribute;
use patpubrender::model::runs::Run;
use patpubrender::{SourceFormat, write_patent_xml};

fn main() {
    let doc = PatentDocument {
        source_format: SourceFormat::UsptoGrantV47,
        // The root `dtd-version` is the marker the format detector keys on.
        attributes: vec![XmlAttribute {
            prefix: None,
            local_name: "dtd-version".to_string(),
            value: "v4.7 2022-02-17".to_string(),
        }],
        parts: vec![
            DocumentPart::BibliographicInformation(BibliographicInformation {
                parts: vec![BibliographicPart::TechnicalInformation(TechnicalInformation {
                    parts: vec![TechnicalInformationPart::TitleOfInvention(
                        "Self-Sealing Widget".to_string(),
                    )],
                    ..Default::default()
                })],
                ..Default::default()
            }),
            DocumentPart::AbstractSection(AbstractSection {
                parts: vec![AbstractPart::Paragraph(Paragraph {
                    content: vec![Run::Text(
                        "A widget that seals itself under pressure.".to_string(),
                    )],
                    ..Default::default()
                })],
                ..Default::default()
            }),
            DocumentPart::Claims(Claims {
                parts: vec![ClaimsPart::Claim(Claim {
                    content: vec![Run::Text("A self-sealing widget comprising a seal.".to_string())],
                    ..Default::default()
                })],
                ..Default::default()
            }),
        ],
        ..Default::default()
    };

    match write_patent_xml(&doc) {
        Ok(xml) => print!("{xml}"),
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
