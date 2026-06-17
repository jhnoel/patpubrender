# patpubrender

A Python extension for [patpubrender](https://github.com/jhnoel/patpubrender):
parse USPTO patent grant and application XML into a document object, and render
Markdown. The implementation is Rust, compiled to a native extension.

```
pip install patpubrender
```

## Usage

```python
import patpubrender

xml = open("US12345678.xml").read()

# render
md = patpubrender.parse_to_markdown(xml)
md = patpubrender.parse_to_markdown(xml, template="{{title}}\n\n{{claims}}")

# document object
doc = patpubrender.parse(xml)
doc.publication_number     # str | None
doc.title                  # str | None
doc.inventors              # list[str]
doc.claims[0].number       # int
doc.claims[0].text         # str
doc.abstract_text          # str | None
doc.to_markdown()          # str

patpubrender.detect_format(xml)   # "UsptoGrantV47"
```

Unrecognized or malformed input raises `ValueError`.

## Templates

`to_markdown` and `parse_to_markdown` take an optional template: text with
`{{frontmatter}}`, `{{title}}`, `{{abstract}}`, `{{description}}`, and
`{{claims}}` placeholders. Each expands to a complete block, heading included.
The template fixes order and surrounding text.

## License

Apache 2.0.
