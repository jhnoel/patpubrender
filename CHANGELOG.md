# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

Initial release.

### Added
- Parse USPTO patent grant/application XML and Green Book "APS" plain text into
  a canonical document model (`parse_patent_xml`, `parse_patent_aps`).
- Render compact, front-mattered Markdown (`render_markdown`).
- User-overridable output via section-placeholder templates
  (`render_markdown_with_template`; `{{frontmatter}}`, `{{title}}`,
  `{{abstract}}`, `{{description}}`, `{{claims}}`, `{{body}}`).
- Structured field extraction (`extract::claims`, `extract::abstract_text`,
  `render::biblio::extract_biblio`).
- Optional `shard` feature: the `.zst` + `.idx` codec (write and read).
- Optional `ingest` feature: bulk USPTO weekly-ZIP rendering into shards.
- `patpubrender` CLI: `render`, `shard write`, `shard read`.
- Python SDK (PyPI) with structured `Document` access and `to_markdown`.
