//! As-published bibliographic sidecar records.
//!
//! `render-shard` writes one JSON line per rendered patent into
//! `<stem>.biblio.jsonl`, alongside the `.zst`/`.idx` pair. The record carries
//! the same as-published bibliographic fields the Markdown frontmatter does, in
//! a machine-loadable form, so a downstream store (the DuckDB catalog) can
//! ingest the snapshot without re-parsing the source XML.

use crate::model::document::PatentDocument;
use crate::render::markdown::{
    applicant_names, assignee_names, bare_application_number, document_id, earliest_priority_date,
    filing_date, inventor_names, ipc_classifications, patent_number, publication_date,
    publication_number, title, us_classifications,
};
use crate::json::json_string;

#[derive(Debug, Clone, Default)]
pub struct BiblioRecord {
    pub country_code: Option<String>,
    pub doc_number: Option<String>,
    pub kind_code: Option<String>,
    pub publication_number: Option<String>,
    pub patent_number: Option<String>,
    pub publication_date: Option<String>,
    pub application_number: Option<String>,
    pub filing_date: Option<String>,
    pub invention_title: Option<String>,
    pub applicant_names: Vec<String>,
    pub assignee_names: Vec<String>,
    pub inventor_names: Vec<String>,
    pub ipc_classifications: Vec<String>,
    pub us_classifications: Vec<String>,
    pub priority_date: Option<String>,
}

pub fn extract_biblio(document: &PatentDocument) -> BiblioRecord {
    let id = document_id(document);
    BiblioRecord {
        country_code: id.and_then(|value| value.country_code.clone()),
        doc_number: id.map(|value| value.doc_number.clone()),
        kind_code: id.and_then(|value| value.kind_code.clone()),
        publication_number: publication_number(document),
        patent_number: patent_number(document),
        publication_date: publication_date(document),
        application_number: bare_application_number(document),
        filing_date: filing_date(document),
        invention_title: title(document),
        applicant_names: applicant_names(document),
        assignee_names: assignee_names(document),
        inventor_names: inventor_names(document),
        ipc_classifications: ipc_classifications(document),
        us_classifications: us_classifications(document),
        priority_date: earliest_priority_date(document),
    }
}

impl BiblioRecord {
    /// One newline-free JSON object, keyed so the sidecar joins back to the
    /// `.idx` rows on (`stem`-derived shard, `doc_key`).
    pub fn to_json_line(&self, doc_key: &str, doc_kind: &str, source_file: &str) -> String {
        let fields = [
            ("doc_key", json_string(doc_key)),
            ("doc_kind", json_string(doc_kind)),
            ("source_file", json_string(source_file)),
            ("country_code", json_opt(&self.country_code)),
            ("doc_number", json_opt(&self.doc_number)),
            ("kind_code", json_opt(&self.kind_code)),
            ("publication_number", json_opt(&self.publication_number)),
            ("patent_number", json_opt(&self.patent_number)),
            ("publication_date", json_opt(&self.publication_date)),
            ("application_number", json_opt(&self.application_number)),
            ("filing_date", json_opt(&self.filing_date)),
            ("invention_title", json_opt(&self.invention_title)),
            ("applicant_names", json_array(&self.applicant_names)),
            ("assignee_names", json_array(&self.assignee_names)),
            ("inventor_names", json_array(&self.inventor_names)),
            ("ipc_classifications", json_array(&self.ipc_classifications)),
            ("us_classifications", json_array(&self.us_classifications)),
            ("priority_date", json_opt(&self.priority_date)),
        ];

        let body = fields
            .iter()
            .map(|(key, value)| format!("{}:{value}", json_string(key)))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{{body}}}")
    }
}

fn json_opt(value: &Option<String>) -> String {
    match value {
        Some(value) => json_string(value),
        None => "null".to_string(),
    }
}

fn json_array(values: &[String]) -> String {
    let body = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_line_escapes_and_orders_fields() {
        let record = BiblioRecord {
            invention_title: Some("Widget \"X\"\nFramework".to_string()),
            inventor_names: vec!["Jane Doe".to_string(), "Li Wei".to_string()],
            ..Default::default()
        };
        let line = record.to_json_line("US20260150770A1", "application", "ipa260514.zip");

        assert!(line.starts_with("{\"doc_key\":\"US20260150770A1\""));
        assert!(line.contains("\"doc_kind\":\"application\""));
        assert!(line.contains("\"invention_title\":\"Widget \\\"X\\\"\\nFramework\""));
        assert!(line.contains("\"inventor_names\":[\"Jane Doe\",\"Li Wei\"]"));
        assert!(line.contains("\"patent_number\":null"));
        assert!(!line.contains('\n'));
    }
}
