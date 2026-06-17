# patpubrender (Python)

Python bindings for [`patpubrender`](https://github.com/jhnoel/patpubrender) —
parse USPTO patent grant/application XML into a structured document and render
compact Markdown. The heavy lifting is done in Rust.

```bash
pip install patpubrender
```

```python
import patpubrender

xml = open("US12345678.xml").read()

# One-shot render
md = patpubrender.parse_to_markdown(xml)
md = patpubrender.parse_to_markdown(xml, template="{{title}}\n\n{{claims}}")

# Structured access
doc = patpubrender.parse(xml)
doc.publication_number      # -> str | None
doc.title                   # -> str | None
doc.inventors               # -> list[str]
doc.claims[0].number        # -> int
doc.claims[0].text          # -> str
doc.abstract_text           # -> str | None
doc.to_markdown()           # -> str
doc.to_markdown(template="{{frontmatter}}\n\n{{abstract}}\n\n{{claims}}")

patpubrender.detect_format(xml)   # -> str, e.g. "UsptoGrantV47"
```

## Templates

`to_markdown` / `parse_to_markdown` accept an optional section-placeholder
template. Placeholders: `{{frontmatter}}`, `{{title}}`, `{{abstract}}`,
`{{description}}`, `{{claims}}`. Each expands to a fully-rendered block
(`{{title}}` already includes its `# ` heading); the template controls section
order and surrounding text.

## License

Apache-2.0.
