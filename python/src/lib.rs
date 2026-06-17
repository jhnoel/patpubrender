//! Python bindings for `patpubrender`.
//!
//! Exposes parsing (`parse`, `parse_to_markdown`, `detect_format`) and a
//! structured `Document` with metadata getters, `claims`, `abstract_text`, and
//! `to_markdown(template=None)`.

use ppr::extract;
use ppr::model::document::PatentDocument;
use ppr::render::biblio::{BiblioRecord, extract_biblio};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn py_err<E: std::fmt::Display>(err: E) -> PyErr {
    PyValueError::new_err(err.to_string())
}

fn render(doc: &PatentDocument, template: Option<&str>) -> PyResult<String> {
    match template {
        Some(template) => ppr::render_markdown_with_template(doc, template).map_err(py_err),
        None => Ok(ppr::render_markdown(doc)),
    }
}

/// Parse USPTO patent XML and render Markdown in one call.
#[pyfunction]
#[pyo3(signature = (xml, template=None))]
fn parse_to_markdown(xml: &str, template: Option<&str>) -> PyResult<String> {
    let doc = ppr::parse_patent_xml(xml).map_err(py_err)?;
    render(&doc, template)
}

/// Detect the USPTO source format of `xml` without a full parse.
#[pyfunction]
fn detect_format(xml: &str) -> PyResult<String> {
    ppr::detect_source_format(xml)
        .map(|format| format!("{format:?}"))
        .map_err(py_err)
}

/// Parse USPTO patent XML into a structured [`Document`].
#[pyfunction]
fn parse(xml: &str) -> PyResult<Document> {
    let doc = ppr::parse_patent_xml(xml).map_err(py_err)?;
    Ok(Document::new(doc))
}

/// A single claim, numbered by its 1-based position.
#[pyclass(frozen)]
struct Claim {
    #[pyo3(get)]
    number: usize,
    #[pyo3(get)]
    text: String,
}

#[pymethods]
impl Claim {
    fn __repr__(&self) -> String {
        let preview: String = self.text.chars().take(60).collect();
        let ellipsis = if self.text.chars().count() > 60 {
            "…"
        } else {
            ""
        };
        format!("Claim(number={}, text={preview:?}{ellipsis})", self.number)
    }
}

/// A parsed patent document: metadata getters plus `claims`, `abstract_text`,
/// and `to_markdown(template=None)`.
#[pyclass(frozen)]
struct Document {
    doc: PatentDocument,
    biblio: BiblioRecord,
}

impl Document {
    fn new(doc: PatentDocument) -> Self {
        let biblio = extract_biblio(&doc);
        Self { doc, biblio }
    }
}

#[pymethods]
impl Document {
    #[getter]
    fn publication_number(&self) -> Option<String> {
        self.biblio.publication_number.clone()
    }
    #[getter]
    fn patent_number(&self) -> Option<String> {
        self.biblio.patent_number.clone()
    }
    #[getter]
    fn application_number(&self) -> Option<String> {
        self.biblio.application_number.clone()
    }
    #[getter]
    fn title(&self) -> Option<String> {
        self.biblio.invention_title.clone()
    }
    #[getter]
    fn filing_date(&self) -> Option<String> {
        self.biblio.filing_date.clone()
    }
    #[getter]
    fn publication_date(&self) -> Option<String> {
        self.biblio.publication_date.clone()
    }
    #[getter]
    fn priority_date(&self) -> Option<String> {
        self.biblio.priority_date.clone()
    }
    #[getter]
    fn inventors(&self) -> Vec<String> {
        self.biblio.inventor_names.clone()
    }
    #[getter]
    fn applicants(&self) -> Vec<String> {
        self.biblio.applicant_names.clone()
    }
    #[getter]
    fn assignees(&self) -> Vec<String> {
        self.biblio.assignee_names.clone()
    }
    #[getter]
    fn ipc_classifications(&self) -> Vec<String> {
        self.biblio.ipc_classifications.clone()
    }
    #[getter]
    fn us_classifications(&self) -> Vec<String> {
        self.biblio.us_classifications.clone()
    }
    #[getter]
    fn source_format(&self) -> String {
        format!("{:?}", self.doc.source_format)
    }
    #[getter]
    fn abstract_text(&self) -> Option<String> {
        extract::abstract_text(&self.doc)
    }
    #[getter]
    fn claims(&self) -> Vec<Claim> {
        extract::claims(&self.doc)
            .into_iter()
            .map(|c| Claim {
                number: c.number,
                text: c.text,
            })
            .collect()
    }

    /// Render this document to Markdown, optionally with a section-placeholder
    /// template (`{{frontmatter}}`, `{{title}}`, `{{abstract}}`,
    /// `{{description}}`, `{{claims}}`, `{{body}}`).
    #[pyo3(signature = (template=None))]
    fn to_markdown(&self, template: Option<&str>) -> PyResult<String> {
        render(&self.doc, template)
    }

    fn __repr__(&self) -> String {
        format!(
            "Document(publication_number={}, title={})",
            opt_repr(&self.biblio.publication_number),
            opt_repr(&self.biblio.invention_title),
        )
    }
}

/// Format an `Option<String>` as Python would show it: `None` or `'value'`.
fn opt_repr(value: &Option<String>) -> String {
    match value {
        Some(value) => format!("{value:?}"),
        None => "None".to_string(),
    }
}

#[pymodule]
fn patpubrender(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_to_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(detect_format, m)?)?;
    m.add_class::<Document>()?;
    m.add_class::<Claim>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
