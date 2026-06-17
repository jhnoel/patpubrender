# patpubrender

patpubrender — render USPTO patent documents as Markdown.

It parses patent grant and application XML, and the legacy Green Book "APS"
text format, into one document model, then renders Markdown under a YAML
frontmatter header. Rendered documents may be packed into a compressed,
randomly addressable archive.

The library requires only an XML reader. Archive support is a feature.

```
    feature     adds                                          dependencies
    (default)   parse, model, Markdown render                 xmloxide
    shard       .zst/.idx archive: codec and bulk ZIP ingest  zstd, zip, rayon
```

## Coverage

patpubrender detects supported source formats from the root element, DOCTYPE,
and/or `dtd-version` marker.

| USPTO source family | Product/profile | Covered versions | Parser coverage |
| --- | --- | --- | --- |
| Red Book XML grants | `us-patent-grant` / ST.32 `PATDOC` | v2.5, v4.0, v4.1, v4.2, v4.3, v4.4, v4.5, v4.6, v4.7 | Bibliographic data, title, abstract, claims, description, parties, classifications, continuity/priority fields, plus opaque preservation for unmodeled XML. |
| Red Book XML applications | `patent-application-publication` / `us-patent-application` | v1.5, v1.6, v4.0, v4.1, v4.2, v4.3, v4.4, v4.5, v4.6 | Same canonical document model as grants, including Markdown rendering and structured Python access. |
| Green Book grants | APS plain-text profile (`PTGRAPS`, `pftaps*.txt`) | APS1 weekly grant records, 1976–2001 era | Patent/application numbers, dates, title, abstract, claims, description, parties, and legal/continuity metadata where present in APS tags. |

Unsupported or ambiguous source files are skipped by bulk ingest with manifest
counters instead of stopping the whole weekly batch.

## Library

```toml
[dependencies]
patpubrender = "0.1"
```

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = std::fs::read_to_string("patent.xml")?;
    let doc = patpubrender::parse_patent_xml(&xml)?; // schema version detected
    let md = patpubrender::render_markdown(&doc);
    std::fs::write("patent.md", md)?;
    Ok(())
}
```

A rendered document opens with a YAML frontmatter block — publication and
patent numbers, dates, application number, classifications, priority chain —
then the title, abstract, claims, and description.

## Templates

The layout is a template: text with `{{...}}` placeholders, namely
`frontmatter`, `title`, `abstract`, `description`, and `claims`. Each expands
to a complete block, heading included: `{{title}}` carries its own `#`,
`{{abstract}}` its `## Abstract`. A template fixes order and surrounding text.
It has no expressions and adds no dependency.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = std::fs::read_to_string("patent.xml")?;
    let doc = patpubrender::parse_patent_xml(&xml)?;
    let tmpl = "{{title}}\n\n{{claims}}\n\n{{frontmatter}}";
    let md = patpubrender::render_markdown_with_template(&doc, tmpl)?;
    println!("{md}");
    Ok(())
}
```

## Shards

A shard is two files. `<stem>.zst` holds one independent zstd frame per
document; independence is what permits random access. `<stem>.idx` is a table
of `doc_key`, `offset`, `length`, tab-separated, one row per frame. The
`patpubrender::shard` module owns both the codec and bulk ingest.

Enable the `shard` feature when using the shard API:

```toml
[dependencies]
patpubrender = { version = "0.1", features = ["shard"] }
```

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // weekly bulk ZIP -> shard, .biblio.jsonl sidecar, .manifest.json
    let stats = patpubrender::shard::render_shard_from_zip("ipg250128.zip", "shards/", None)?;
    println!("wrote {} documents to {}", stats.docs_written, stats.zst_path);
    Ok(())
}
```

### Bulk data product examples

Install the CLI with shard support, then run `shard write` directly on USPTO
weekly bulk ZIPs. The output directory gets `<stem>.zst`, `<stem>.idx`,
`<stem>.biblio.jsonl`, and `<stem>.manifest.json`.

```sh
cargo install patpubrender --features shard
mkdir -p shards

# Red Book patent grant XML weekly ZIPs: ipg*.zip or pg*.zip
patpubrender shard write --zip ipg250128.zip --output shards/ --jobs 8

# Red Book published application XML weekly ZIPs: ipa*.zip or pa*.zip
patpubrender shard write --zip ipa260101.zip --output shards/ --jobs 8

# Green Book APS grant ZIPs containing pftaps*.txt
patpubrender shard write --zip pftaps19830308_wk10.zip --output shards/
```

Read one rendered document back by key, or by an index offset/length pair:

```sh
# XML keys are usually the root file="..." attribute without .XML.
patpubrender shard read --shard shards/ipg250128.zst --key US12207575-20250128
patpubrender shard read --shard shards/ipa260101.zst --key US20260123456-20260101

# APS keys are derived from the WKU patent number.
patpubrender shard read --shard shards/pftaps19830308_wk10.zst --key US4375702B1
```

To process many weekly ZIPs, pass a directory containing ZIP files:

```sh
patpubrender shard write --dir ./bulk/PTGRXML --output shards/ --limit 1000 --jobs 8
```

## Command line

```
patpubrender render [INPUT] [--output OUT] [--template FILE]
patpubrender shard write (--zip ZIP | --dir DIR) [--output DIR] [--limit N] [--jobs N]
patpubrender shard read  --shard FILE.zst (--key KEY | --offset N --length L) [--index FILE]
```

Runnable from this checkout:

```sh
cargo run -- render e2e/fixtures/US12207575-20250128.XML --output /tmp/US12207575.md
```

`render` reads INPUT — a file, a directory, or `-` for standard input, which
is the default. A file or standard input renders to standard output, or to
`--output`. A directory renders every `*.xml` file: concatenated to standard
output, or one `.md` per file under `--output`. Concatenated documents are
separated by four newlines; the renderer never emits that sequence within a
document, so it is an unambiguous boundary.

The `shard` subcommands require `--features shard`. `read` derives `--index`
from the shard name when it is omitted.

```
cargo install patpubrender --features shard
```

## Python

A Python extension is published to PyPI from [`python/`](python/).

```python
from pathlib import Path
import patpubrender

xml = Path("e2e/fixtures/US12207575-20250128.XML").read_text(encoding="utf-8")
doc = patpubrender.parse(xml)
print(doc.title)
print(doc.claims[0].text)
print(doc.to_markdown(template="{{title}}\n\n{{claims}}"))
```

## End-to-end tests

The [`e2e/`](e2e/) folder contains runnable script-based checks:

```sh
./e2e/rust/run.sh      # CLI render plus shard write/read against a ZIP fixture
./e2e/python/run.sh    # Python parse/render smoke test; install python/ first
```

## License

Apache 2.0; see [LICENSE](LICENSE) and [NOTICE](NOTICE). Not affiliated with
or endorsed by the United States Patent and Trademark Office.
