//! Regression coverage for the application-number doubling bug.
//!
//! A US application number is `series + serial`. The USPTO source XML's
//! `<doc-number>` for an application ALREADY embeds the series prefix
//! (e.g. doc-number `14633232` with a separate series code `14`). The renderer
//! must therefore NOT concatenate series with the full doc-number, or it would
//! emit a doubled value (`14/14633232` / key `1414633232`). These tests assert
//! patpubrender emits the canonical display label `14/633232` and never the doubled
//! form.

use patpubrender::SourceFormat;
use patpubrender::model::bibliographic::{
    ApplicationNumber, BibliographicInformation, BibliographicPart, DomesticFilingData,
    DomesticFilingPart, TechnicalInformation, TechnicalInformationPart,
};
use patpubrender::model::document::{DocumentPart, PatentDocument};
use patpubrender::render_markdown;

fn application_document(doc_number: &str, series: &str) -> PatentDocument {
    PatentDocument {
        source_format: SourceFormat::UsptoApplicationV40,
        parts: vec![DocumentPart::BibliographicInformation(
            BibliographicInformation {
                parts: vec![
                    BibliographicPart::TechnicalInformation(TechnicalInformation {
                        parts: vec![TechnicalInformationPart::TitleOfInvention(
                            "Application label test".to_string(),
                        )],
                        ..Default::default()
                    }),
                    BibliographicPart::DomesticFilingData(DomesticFilingData {
                        parts: vec![
                            DomesticFilingPart::ApplicationNumber(ApplicationNumber {
                                doc_number: Some(doc_number.to_string()),
                                ..Default::default()
                            }),
                            DomesticFilingPart::ApplicationNumberSeriesCode(series.to_string()),
                        ],
                        ..Default::default()
                    }),
                ],
                ..Default::default()
            },
        )],
        ..Default::default()
    }
}

#[test]
fn modern_doc_number_already_carries_series_is_not_doubled() {
    // doc-number "14633232" already includes the series "14".
    let rendered = render_markdown(&application_document("14633232", "14"));
    assert!(
        rendered.contains("application_number: 14/633232"),
        "expected canonical 14/633232, got:\n{rendered}"
    );
    // The doubled form must never appear, in any shape.
    assert!(
        !rendered.contains("14/14633232"),
        "doubled slash form leaked:\n{rendered}"
    );
    assert!(
        !rendered.contains("1414633232"),
        "doubled bare form leaked:\n{rendered}"
    );
}

#[test]
fn legacy_serial_only_doc_number_joins_series() {
    // Legacy doc-number "633232" is serial-only (< 8 digits); the series must
    // be prepended to reach the canonical display.
    let rendered = render_markdown(&application_document("633232", "14"));
    assert!(
        rendered.contains("application_number: 14/633232"),
        "expected 14/633232 for legacy serial-only, got:\n{rendered}"
    );
}
