#!/usr/bin/env python3
"""Writes the release notes for one version.

The notes on a release page are:

    a screenshot of that version
    a blurb about what the application is and does
    the notes for that version

The blurb is at the top of CHANGELOG.md, and the screenshot is the first
line of each version's part of it. This puts them in the order above.

    python3 dev/release-notes.py v0.2.2 > notes.md
"""
import re
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: release-notes.py <tag>", file=sys.stderr)
        return 2

    tag = sys.argv[1]
    version = tag[1:] if tag.startswith("v") else tag

    with open("CHANGELOG.md", encoding="utf-8") as handle:
        text = handle.read()

    # The part before the first version is the title and the blurb.
    parts = re.split(r"^## ", text, flags=re.MULTILINE)
    header = parts[0].splitlines()

    sections = {}
    for part in parts[1:]:
        lines = part.splitlines()
        # The first line is the version heading, which the release already
        # has as its title.
        sections[lines[0].split()[0]] = lines[1:]

    if version not in sections:
        print(f"no notes for {version} in the changelog", file=sys.stderr)
        return 1

    lines = sections[version]
    screenshot = next((line for line in lines if line.startswith("![")), "")
    body = [line for line in lines if not line.startswith("![")]
    # The blank lines at the ends go; the ones in the middle are the
    # paragraphs, and they stay.
    while body and not body[0].strip():
        body.pop(0)
    while body and not body[-1].strip():
        body.pop()
    # The title is the first line of the header; the rest is the blurb.
    blurb = "\n".join(header[1:]).strip()

    print(screenshot)
    print()
    print(blurb)
    print()
    print("\n".join(body))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
