#!/usr/bin/env python3
"""Fail if the cargo graph carries more than one pollster version.

The toolkit standardised on the pollster 1.x line; the old optional `rfd`
edge used to drag in a separate pollster 0.4 copy (issue #87). Duplicates
here mean some dependency reintroduced the second line.
"""
import json
import subprocess
import sys


def pollster_versions(metadata: dict) -> list[str]:
    return sorted(
        {
            pkg["version"]
            for pkg in metadata.get("packages", [])
            if pkg.get("name") == "pollster"
        }
    )


def main() -> int:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--all-features"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    versions = pollster_versions(json.loads(out))
    if len(versions) > 1:
        print(
            "error: multiple pollster versions in graph: " + ", ".join(versions)
        )
        return 1
    print(f"ok: pollster versions in graph: {versions or 'none'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
