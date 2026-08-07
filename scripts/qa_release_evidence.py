#!/usr/bin/env python3
"""Bind GPUI Toolkit release QA artifacts to source and toolchain provenance."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import subprocess
import sys
from pathlib import Path
from typing import Iterable


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-toolkit-release-evidence-manifest"

REQUIRED_ARTIFACTS = (
    "qa/perf/baseline.json",
    "qa/visual/baselines/component-lab-metal-pr-v1.tar.zst",
    "target/gpui-conformance/component-lab.json",
    "target/gpui-conformance/component-lab.md",
    "target/gpui-conformance/design-tokens.json",
    "target/gpui-conformance/design-tokens.md",
    "target/qa/accessibility/desktop-evidence.json",
    "target/qa/accessibility/desktop-evidence.md",
    "target/qa/cov/report.md",
    "target/qa/cov/summary.json",
    "target/qa/perf/current.json",
    "target/qa/perf/report.md",
    "target/qa/visual/component-lab-manifest.json",
    "target/qa/visual/component-lab-manifest.md",
    "target/qa/visual/report.md",
    "target/qa/visual/showcase-manifest.json",
    "target/qa/visual/showcase-manifest.md",
)

OPTIONAL_ARTIFACTS = (
    "target/qa/visual/component-lab-capture.json",
    "target/qa/visual/component-lab-capture.md",
    "target/qa/visual/component-lab-diff.json",
    "target/qa/visual/component-lab-diff.md",
)

PLATFORM_EVIDENCE = {
    "android-emulator": "target/qa/platform/android-emulator/evidence.json",
    "ios-simulator": "target/qa/platform/ios-simulator/evidence.json",
    "tvos-simulator": "target/qa/platform/tvos-simulator/evidence.json",
}


class EvidenceError(RuntimeError):
    pass


def command_output(root: Path, *args: str) -> str:
    completed = subprocess.run(
        args,
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_provenance(root: Path) -> dict[str, object]:
    status = command_output(root, "git", "status", "--porcelain=v1", "--untracked-files=all")
    return {
        "revision": command_output(root, "git", "rev-parse", "HEAD"),
        "dirty": bool(status),
        "commit_timestamp": int(command_output(root, "git", "show", "-s", "--format=%ct", "HEAD")),
    }


def toolchain_provenance(root: Path) -> dict[str, str]:
    return {
        "cargo": command_output(root, "cargo", "--version"),
        "just": command_output(root, "just", "--version"),
        "python": platform.python_version(),
        "rustc": command_output(root, "rustc", "--version", "--verbose"),
    }


def embedded_source_binding(path: Path, revision: str) -> dict[str, object] | None:
    if path.suffix != ".json":
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict):
        return None

    embedded_revision = value.get("source_revision")
    embedded_dirty = value.get("source_dirty")
    environment = value.get("metadata", {}).get("environment", {}) if isinstance(value.get("metadata"), dict) else {}
    if embedded_revision is None and isinstance(environment, dict):
        embedded_revision = environment.get("source_revision")
        embedded_dirty = environment.get("source_dirty")
    if embedded_revision is None:
        return None
    return {
        "revision": embedded_revision,
        "dirty": embedded_dirty,
        "matches_manifest_source": embedded_revision == revision and embedded_dirty is False,
    }


def artifact_row(root: Path, relative: str, revision: str) -> dict[str, object]:
    path = root / relative
    row: dict[str, object] = {
        "path": relative,
        "sha256": sha256(path),
        "size_bytes": path.stat().st_size,
    }
    binding = embedded_source_binding(path, revision)
    if binding is not None:
        row["embedded_source"] = binding
    return row


def collect_artifacts(root: Path, revision: str) -> list[dict[str, object]]:
    missing = [relative for relative in REQUIRED_ARTIFACTS if not (root / relative).is_file()]
    if missing:
        raise EvidenceError("missing required QA artifacts: " + ", ".join(missing))

    paths = set(REQUIRED_ARTIFACTS)
    paths.update(relative for relative in OPTIONAL_ARTIFACTS if (root / relative).is_file())
    for relative in PLATFORM_EVIDENCE.values():
        directory = (root / relative).parent
        if directory.is_dir():
            paths.update(
                path.relative_to(root).as_posix()
                for path in directory.iterdir()
                if path.is_file()
            )
    return [artifact_row(root, relative, revision) for relative in sorted(paths)]


def validate_required_platforms(
    root: Path,
    artifacts: Iterable[dict[str, object]],
    required_platforms: Iterable[str],
) -> None:
    by_path = {str(row["path"]): row for row in artifacts}
    errors: list[str] = []
    for platform_id in required_platforms:
        relative = PLATFORM_EVIDENCE[platform_id]
        row = by_path.get(relative)
        if row is None:
            errors.append(f"{platform_id}: missing {relative}")
            continue
        binding = row.get("embedded_source")
        if not isinstance(binding, dict) or binding.get("matches_manifest_source") is not True:
            errors.append(f"{platform_id}: evidence is dirty or belongs to another source revision")
    if errors:
        raise EvidenceError("invalid required platform evidence: " + "; ".join(errors))


def build_manifest(
    root: Path,
    *,
    require_clean: bool = False,
    required_platforms: Iterable[str] = (),
) -> dict[str, object]:
    source = source_provenance(root)
    if require_clean and source["dirty"]:
        raise EvidenceError("release evidence requires a clean worktree")
    revision = str(source["revision"])
    artifacts = collect_artifacts(root, revision)
    validate_required_platforms(root, artifacts, required_platforms)
    return {
        "schema_version": SCHEMA_VERSION,
        "report_type": REPORT_TYPE,
        "source": source,
        "host": {
            "machine": platform.machine(),
            "platform": platform.platform(),
            "system": platform.system(),
        },
        "toolchains": toolchain_provenance(root),
        "artifacts": artifacts,
    }


def render_markdown(manifest: dict[str, object]) -> str:
    source = manifest["source"]
    host = manifest["host"]
    toolchains = manifest["toolchains"]
    artifacts = manifest["artifacts"]
    assert isinstance(source, dict)
    assert isinstance(host, dict)
    assert isinstance(toolchains, dict)
    assert isinstance(artifacts, list)
    lines = [
        "# GPUI Toolkit release evidence manifest",
        "",
        f"- schema_version: {manifest['schema_version']}",
        f"- report_type: `{manifest['report_type']}`",
        f"- source_revision: `{source['revision']}`",
        f"- source_dirty: `{str(source['dirty']).lower()}`",
        f"- source_commit_timestamp: `{source['commit_timestamp']}`",
        f"- host: `{host['system']} {host['machine']}`",
        f"- rustc: `{str(toolchains['rustc']).splitlines()[0]}`",
        f"- cargo: `{toolchains['cargo']}`",
        f"- artifacts: {len(artifacts)}",
        "",
        "| Artifact | Bytes | SHA-256 | Embedded source binding |",
        "| --- | ---: | --- | --- |",
    ]
    for row in artifacts:
        binding = row.get("embedded_source")
        binding_text = "manifest-bound"
        if isinstance(binding, dict):
            binding_text = "matched" if binding.get("matches_manifest_source") else "mismatch"
        lines.append(
            f"| `{row['path']}` | {row['size_bytes']} | `{row['sha256']}` | {binding_text} |"
        )
    return "\n".join(lines) + "\n"


def write_atomic(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(content, encoding="utf-8")
    temporary.replace(path)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-json", type=Path, default=Path("target/qa/release-evidence.json"))
    parser.add_argument("--output-markdown", type=Path, default=Path("target/qa/release-evidence.md"))
    parser.add_argument("--require-clean", action="store_true")
    parser.add_argument(
        "--require-platform",
        action="append",
        choices=sorted(PLATFORM_EVIDENCE),
        default=[],
    )
    args = parser.parse_args(argv)
    root = Path(__file__).resolve().parent.parent
    try:
        manifest = build_manifest(
            root,
            require_clean=args.require_clean,
            required_platforms=args.require_platform,
        )
    except (EvidenceError, OSError, subprocess.CalledProcessError) as error:
        print(f"release evidence failed: {error}", file=sys.stderr)
        return 1
    write_atomic(args.output_json, json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    write_atomic(args.output_markdown, render_markdown(manifest))
    print(f"Release evidence manifest: {args.output_json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
