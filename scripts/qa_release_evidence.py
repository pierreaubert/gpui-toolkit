#!/usr/bin/env python3
"""Bind GPUI Toolkit release QA artifacts to source and toolchain provenance."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import platform
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import Iterable


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-toolkit-release-evidence-manifest"

MESH_PLOT_LOCAL_CAPTURE_COUNT = 99
MESH_PLOT_VERSIONED_BASELINE_COUNT = 9
MESH_PLOT_VISUAL_CAPTURE_ARTIFACT = "target/qa/visual/component-lab-capture.json"
MESH_PLOT_VISUAL_DIFF_ARTIFACT = "target/qa/visual/component-lab-diff.json"
MESH_PLOT_SCREEN_READER_RUNBOOK = "qa/accessibility/mesh-plot-screen-reader-qa.md"

REQUIRED_ARTIFACTS = (
    "qa/perf/baseline.json",
    MESH_PLOT_SCREEN_READER_RUNBOOK,
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
    MESH_PLOT_VISUAL_CAPTURE_ARTIFACT,
    MESH_PLOT_VISUAL_DIFF_ARTIFACT,
    "target/qa/visual/component-lab-manifest.json",
    "target/qa/visual/component-lab-manifest.md",
    "target/qa/visual/report.md",
    "target/qa/visual/showcase-manifest.json",
    "target/qa/visual/showcase-manifest.md",
)

MESH_PLOT_BENCHMARKS = {
    ("gpui-d3rs", "mesh_prep"),
    ("gpui-px", "mesh_plot_frames"),
}
MESH_PLOT_BASELINE_MARKER = "px-mesh-plot__"

OPTIONAL_ARTIFACTS = (
    "target/qa/visual/component-lab-capture.md",
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


def read_json_object(path: Path, description: str) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise EvidenceError(f"invalid {description} {path.name}: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError(f"invalid {description} {path.name}: expected a JSON object")
    return value


def validate_mesh_plot_visual_capture(root: Path) -> None:
    """Require all 99 local MeshPlot actual captures to be present."""
    capture = read_json_object(
        root / MESH_PLOT_VISUAL_CAPTURE_ARTIFACT,
        "MeshPlot local visual capture",
    )
    cases = capture.get("cases")
    if (
        capture.get("report_type") != "gpui-component-lab-render-capture"
        or capture.get("passed") is not True
        or capture.get("requested_count") != MESH_PLOT_LOCAL_CAPTURE_COUNT
        or capture.get("captured_count") != MESH_PLOT_LOCAL_CAPTURE_COUNT
        or capture.get("failed_count") != 0
        or not isinstance(cases, list)
        or len(cases) != MESH_PLOT_LOCAL_CAPTURE_COUNT
    ):
        raise EvidenceError(
            "MeshPlot local visual capture must report 99 requested/captured "
            "cases, zero failures, and a passing capture run"
        )

    root_resolved = root.resolve()
    capture_ids: set[str] = set()
    actual_paths: set[Path] = set()
    for case in cases:
        if not isinstance(case, dict):
            raise EvidenceError("MeshPlot local visual capture contains a malformed case")
        capture_id = case.get("capture_id")
        actual_path_text = case.get("actual_path")
        if (
            not isinstance(capture_id, str)
            or not capture_id
            or capture_id in capture_ids
            or case.get("status") != "Captured"
            or not isinstance(actual_path_text, str)
            or not actual_path_text
        ):
            raise EvidenceError(
                "MeshPlot local visual capture must contain 99 unique Captured cases "
                "with actual_path entries"
            )
        candidate = (root / actual_path_text).resolve()
        try:
            candidate.relative_to(root_resolved)
        except ValueError as error:
            raise EvidenceError(
                f"MeshPlot actual capture path escapes the repository: {actual_path_text}"
            ) from error
        if "actual" not in candidate.parts or not candidate.is_file():
            raise EvidenceError(f"missing MeshPlot local actual capture: {actual_path_text}")
        capture_ids.add(capture_id)
        actual_paths.add(candidate)

    if len(capture_ids) != MESH_PLOT_LOCAL_CAPTURE_COUNT or len(actual_paths) != MESH_PLOT_LOCAL_CAPTURE_COUNT:
        raise EvidenceError("MeshPlot local visual capture does not contain 99 unique actual images")


def validate_mesh_plot_benchmarks(root: Path) -> None:
    """Require MeshPlot benchmark records in both release perf artifacts."""
    missing: list[str] = []
    for relative in ("qa/perf/baseline.json", "target/qa/perf/current.json"):
        path = root / relative
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
            raise EvidenceError(f"invalid performance artifact {relative}: {error}") from error
        records = data.get("records") if isinstance(data, dict) else None
        keys = {
            (record.get("crate"), record.get("bench"))
            for record in records
            if isinstance(record, dict)
        } if isinstance(records, list) else set()
        for crate, bench in sorted(MESH_PLOT_BENCHMARKS - keys):
            missing.append(f"{relative}:{crate}:{bench}")
    if missing:
        raise EvidenceError(
            "MeshPlot benchmark evidence is missing; run the registered MeshPlot "
            "benchmarks on the reference host: " + ", ".join(missing)
        )


def visual_baseline_members(path: Path) -> list[str]:
    """List members of the checked-in zstd-compressed visual archive."""
    try:
        completed = subprocess.run(
            ["zstd", "-d", "-c", str(path)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        with tarfile.open(fileobj=io.BytesIO(completed.stdout), mode="r:") as archive:
            return [member.name for member in archive.getmembers()]
    except (OSError, subprocess.CalledProcessError, tarfile.TarError) as error:
        raise EvidenceError(f"invalid visual baseline archive {path.name}: {error}") from error


def validate_mesh_plot_visual_baseline(root: Path) -> set[str]:
    """Require exactly nine versioned MeshPlot entries in the visual archive."""
    archive = root / "qa/visual/baselines/component-lab-metal-pr-v1.tar.zst"
    members = visual_baseline_members(archive)
    ids = [
        Path(member).stem
        for member in members
        if (
            Path(member).parent == Path("metal/baseline")
            and Path(member).name.startswith(MESH_PLOT_BASELINE_MARKER)
            and not Path(member).name.startswith("._")
            and Path(member).suffix.lower() == ".png"
        )
    ]
    if len(ids) != MESH_PLOT_VERSIONED_BASELINE_COUNT or len(set(ids)) != len(ids):
        raise EvidenceError(
            "MeshPlot visual baseline evidence must contain exactly 9 unique "
            "versioned PNG captures"
        )
    return set(ids)


def validate_mesh_plot_visual_diff(root: Path, baseline_ids: set[str]) -> None:
    """Require a passing zero-diff report for the nine versioned captures."""
    diff = read_json_object(
        root / MESH_PLOT_VISUAL_DIFF_ARTIFACT,
        "MeshPlot visual diff",
    )
    cases = diff.get("cases")
    if (
        diff.get("report_type") != "gpui-component-lab-visual-diff"
        or diff.get("passed") is not True
        or diff.get("compared_count") != MESH_PLOT_VERSIONED_BASELINE_COUNT
        or diff.get("failed_count") != 0
        or diff.get("max_changed_pixels") != 0
        or not isinstance(cases, list)
        or len(cases) != MESH_PLOT_VERSIONED_BASELINE_COUNT
    ):
        raise EvidenceError(
            "MeshPlot visual diff must report 9 compared cases, zero failures, "
            "zero changed pixels, and a passing diff run"
        )

    diff_ids: list[str] = []
    for case in cases:
        if not isinstance(case, dict):
            raise EvidenceError("MeshPlot visual diff contains a malformed case")
        capture_id = case.get("capture_id")
        if (
            not isinstance(capture_id, str)
            or capture_id in diff_ids
            or case.get("status") != "Passed"
            or case.get("changed_pixels") != 0
            or case.get("max_channel_delta") != 0
        ):
            raise EvidenceError("MeshPlot visual diff contains a failed or duplicate case")
        diff_ids.append(capture_id)

    if set(diff_ids) != baseline_ids:
        raise EvidenceError(
            "MeshPlot visual diff cases must match the 9 versioned baseline captures"
        )


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
    validate_mesh_plot_benchmarks(root)
    validate_mesh_plot_visual_capture(root)
    baseline_ids = validate_mesh_plot_visual_baseline(root)
    validate_mesh_plot_visual_diff(root, baseline_ids)
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
