# End-to-end tests

These scripts exercise the shipped CLI and Python extension against a real USPTO utility-patent grant fixture from the PTGRXML bulk data product (`ipg250128.xml`, record `US12207575-20250128.XML`, kind `B2`).

## Rust CLI

```sh
./e2e/rust/run.sh
```

The Rust e2e renders XML to Markdown, creates a temporary USPTO-style weekly ZIP, writes a shard with `--features shard`, and reads the document back by key.

## Python package

```sh
cd python && python -m pip install .
cd ..
./e2e/python/run.sh
```

The Python e2e imports `patpubrender`, parses the same fixture, renders Markdown, and checks structured fields.
