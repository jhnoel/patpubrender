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

## Library

```toml
[dependencies]
patpubrender = "0.1"
```

```rust
let xml = std::fs::read_to_string("US12345678.xml")?;
let doc = patpubrender::parse_patent_xml(&xml)?;   // schema version detected
let md  = patpubrender::render_markdown(&doc);
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
let tmpl = "{{title}}\n\n{{claims}}\n\n{{frontmatter}}";
let md = patpubrender::render_markdown_with_template(&doc, tmpl)?;
```

## Shards

A shard is two files. `<stem>.zst` holds one independent zstd frame per
document; independence is what permits random access. `<stem>.idx` is a table
of `doc_key`, `offset`, `length`, tab-separated, one row per frame. The
`patpubrender::shard` module owns both the codec and bulk ingest.

```rust
use patpubrender::shard::{ShardWriter, ShardReader, parse_shard_index};

// weekly bulk ZIP -> shard, .biblio.jsonl sidecar, .manifest.json
patpubrender::shard::render_shard_from_zip("ipg260101.zip", "out/", None)?;
```

Requires `--features shard`.

## Command line

```
patpubrender render [INPUT] [--output OUT] [--template FILE]
patpubrender shard write (--zip ZIP | --dir DIR) [--output DIR] [--limit N] [--jobs N]
patpubrender shard read  --shard FILE.zst (--key KEY | --offset N --length L) [--index FILE]
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
import patpubrender
doc = patpubrender.parse(open("US12345678.xml").read())
doc.title, doc.claims[0].text
doc.to_markdown(template="{{title}}\n\n{{claims}}")
```

## License

Apache 2.0; see [LICENSE](LICENSE) and [NOTICE](NOTICE). Not affiliated with
or endorsed by the United States Patent and Trademark Office.
