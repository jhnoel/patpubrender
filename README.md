# patpubrender

Parse USPTO patent **grant** and **application** XML — plus legacy Green Book
"APS" plain text — into a canonical document model, and render compact,
front-mattered **Markdown**. Optionally pack rendered documents into an
addressable, compressed **shard** archive.

The crate is layered so you only pay for what you use:

| Tier | Cargo feature | Adds | Pulls in |
|------|---------------|------|----------|
| Renderer | *(default)* | parse + model + Markdown render | `xmloxide` |
| Shard codec | `shard` | write/read the `.zst` + `.idx` archive format | `zstd` |
| Bulk ingest | `ingest` | render whole USPTO weekly ZIPs into shards | `zip`, `rayon` (implies `shard`) |

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

### Shard codec (`--features shard`)

A shard is a pair of files: `<stem>.zst` holds one independent zstd frame per
document (frame independence is what enables random access), and `<stem>.idx` is
a TSV of `doc_key⇥offset⇥length` rows. Write and read live together in
`patpubrender::shard` so the on-disk format has a single owner.

```rust
use patpubrender::shard::{ShardWriter, ShardReader, parse_shard_index};
```

### Bulk ingest (`--features ingest`)

`patpubrender::ingest::render_shard_from_zip` renders a USPTO weekly bulk ZIP
into a shard plus a `.biblio.jsonl` sidecar and a `.manifest.json`.

## CLI

```
patpubrender render [INPUT] [--output OUT]
    INPUT: a file, a directory, or - / omitted for stdin
    file / stdin → stdout (or --output FILE)
    directory    → all docs concatenated to stdout, or one .md per file into --output DIR

patpubrender shard write (--zip ZIP | --dir DIR_OF_ZIPS) [--output DIR] [--limit N] [--jobs N]
    (requires --features ingest)

patpubrender shard read --shard FILE.zst (--key KEY | --offset N --length L) [--index FILE.idx] [--output OUT]
    (requires --features shard; --index defaults to <shard-stem>.idx)
```

Install the full CLI with `cargo install patpubrender --features ingest`.

When a directory is rendered to stdout, documents are separated by four newlines
(`\n\n\n\n`) — an unambiguous record boundary, since the renderer never emits
that sequence inside a document.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
