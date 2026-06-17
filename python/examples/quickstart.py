# /// script
# requires-python = ">=3.9"
# dependencies = ["patpubrender"]
# ///
"""Render a USPTO patent XML file to Markdown via the patpubrender PyPI package.

Run it with uv — no manual install, no virtualenv. uv reads the inline
dependency metadata above, fetches patpubrender from PyPI into an ephemeral
environment, and runs the script:

    uv run python/examples/quickstart.py path/to/patent.xml
"""

import sys

import patpubrender


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: uv run quickstart.py <patent.xml>")

    xml = open(sys.argv[1], encoding="utf-8").read()

    doc = patpubrender.parse(xml)
    print(f"format : {doc.source_format}")
    print(f"title  : {doc.title}")
    print(f"claims : {len(doc.claims)}")
    print("-" * 40)
    print(doc.to_markdown())


if __name__ == "__main__":
    main()
