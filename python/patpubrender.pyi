"""Type stubs for patpubrender — USPTO patent XML to Markdown."""

from typing import Optional

__version__: str

def parse(xml: str) -> Document:
    """Parse USPTO patent XML into a structured Document. Raises ValueError on
    unrecognized or malformed input."""

def parse_to_markdown(xml: str, template: Optional[str] = None) -> str:
    """Parse and render Markdown in one call. Raises ValueError on bad input or
    an invalid template."""

def detect_format(xml: str) -> str:
    """Return the detected USPTO source format (e.g. 'UsptoGrantV47')."""

class Claim:
    number: int
    text: str

class Document:
    publication_number: Optional[str]
    patent_number: Optional[str]
    application_number: Optional[str]
    title: Optional[str]
    filing_date: Optional[str]
    publication_date: Optional[str]
    priority_date: Optional[str]
    inventors: list[str]
    applicants: list[str]
    assignees: list[str]
    ipc_classifications: list[str]
    us_classifications: list[str]
    source_format: str
    abstract_text: Optional[str]
    claims: list[Claim]

    def to_markdown(self, template: Optional[str] = None) -> str:
        """Render this document to Markdown, optionally with a section-placeholder
        template ({{frontmatter}}, {{title}}, {{abstract}}, {{description}},
        {{claims}}, {{body}})."""
