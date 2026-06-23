# Changelog

This project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-06-23

- Add release coverage for treating non-patent supplemental XML roots such as
  `sequence-cwu` as supplemental skips during shard ingest.
- Ship the post-0.1.0 documentation and end-to-end examples.

## [0.1.0] - 2026-06-17

First release.

patpubrender parses USPTO patent grant and application XML, and the legacy Green
Book "APS" text format, into one document model, then renders Markdown under a
YAML frontmatter header. Schema version is detected from the source.

- Parse: `parse_patent_xml`, `parse_patent_aps`, `detect_source_format`.
- Render: `render_markdown`, and `render_markdown_with_template` over a
  section-placeholder template (`frontmatter`, `title`, `abstract`,
  `description`, `claims`).
- Extract: bibliographic fields, claims, abstract.
- Shards (`shard` feature): the `.zst`/`.idx` archive — write/read codec and
  bulk weekly-ZIP ingest, under `patpubrender::shard`.
- Command line: `render`, `shard write`, `shard read`.
- Python extension (PyPI): parse to a `Document`, render Markdown.
