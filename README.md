# patpubrender

Parse USPTO patent **grant** and **application** XML — plus legacy Green Book
"APS" plain text — into a canonical document model, and render compact,
front-mattered **Markdown**. Optionally pack rendered documents into an
addressable, compressed **shard** archive.

The crate is layered so you only pay for what you use:

| Cargo feature | Adds | Pulls in |
|---------------|------|----------|
| *(default)* | parse + model + Markdown render | `xmloxide` |
| `shard` | the `.zst`/`.idx` archive — write/read codec **and** bulk USPTO weekly-ZIP ingest | `zstd`, `zip`, `rayon` |

## Library

```toml
[dependencies]
patpubrender = "0.1"
```

```rust
let xml = std::fs::read_to_string("US12345678.xml")?;
let doc = patpubrender::parse_patent_xml(&xml)?;     // auto-detects schema version
let markdown = patpubrender::render_markdown(&doc);
```

Each rendered document begins with a YAML frontmatter block (publication/patent
numbers, dates, application number, classifications, priority chain) followed by
the title, abstract, claims, and description.

### Custom templates

Override the default layout with a section-placeholder template — plain text
with `{{placeholder}}` tokens. The placeholders are `frontmatter`, `title`,
`abstract`, `description`, and `claims`. Each placeholder expands to a
**fully-rendered block** — e.g. `{{title}}` already includes the `# ` heading and
`{{abstract}}` its `## Abstract` heading — so the template only controls order and
surrounding text. No expression language, no dependency.

```rust
let tmpl = "{{title}}\n\n> Source: USPTO\n\n{{claims}}\n\n{{frontmatter}}";
let md = patpubrender::render_markdown_with_template(&doc, tmpl)?;
```

From the CLI: `patpubrender render US123.xml --template my.md`.

### Shards (`--features shard`)

A shard is a pair of files: `<stem>.zst` holds one independent zstd frame per
document (frame independence is what enables random access), and `<stem>.idx` is
a TSV of `doc_key⇥offset⇥length` rows. Everything shard-related lives in one
module, `patpubrender::shard` — the write/read codec and bulk ingest:

```rust
use patpubrender::shard::{ShardWriter, ShardReader, parse_shard_index};

// Render a USPTO weekly bulk ZIP into a shard + .biblio.jsonl + .manifest.json:
patpubrender::shard::render_shard_from_zip("ipg260101.zip", "out/", None)?;
```

## CLI

```
patpubrender render [INPUT] [--output OUT] [--template FILE]
    INPUT: a file, a directory, or - / omitted for stdin
    file / stdin → stdout (or --output FILE)
    directory    → all docs concatenated to stdout, or one .md per file into --output DIR
    --template   → a .md template with {{frontmatter}}/{{title}}/{{abstract}}/
                   {{description}}/{{claims}} placeholders

patpubrender shard write (--zip ZIP | --dir DIR_OF_ZIPS) [--output DIR] [--limit N] [--jobs N]
patpubrender shard read --shard FILE.zst (--key KEY | --offset N --length L) [--index FILE.idx] [--output OUT]
    (both require --features shard; read's --index defaults to <shard-stem>.idx)
```

Install the full CLI with `cargo install patpubrender --features shard`.

When a directory is rendered to stdout, documents are separated by four newlines
(`\n\n\n\n`) — an unambiguous record boundary, since the renderer never emits
that sequence inside a document.

## Python

A Python SDK (structured `Document` access + `to_markdown`) is published to
PyPI from [`python/`](python/):

```bash
pip install patpubrender
```

```python
import patpubrender
doc = patpubrender.parse(open("US12345678.xml").read())
print(doc.title, doc.claims[0].text)
md = doc.to_markdown(template="{{title}}\n\n{{claims}}")
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE). See [NOTICE](NOTICE).

This project parses USPTO patent data formats but is not affiliated with or
endorsed by the USPTO.
