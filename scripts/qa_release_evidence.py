#!/usr/bin/env python3
"""Bind GPUI Toolkit release QA artifacts to source and toolchain provenance."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import io
import json
import platform
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import Iterable

from mesh_wgpu_manifest import (
    WgpuManifestError,
    compare_manifests,
    validate_manifest,
)


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-toolkit-release-evidence-manifest"

MESH_PLOT_LOCAL_CAPTURE_COUNT = 99
MESH_PLOT_VERSIONED_BASELINE_COUNT = 99
MESH_PLOT_VISUAL_CAPTURE_ARTIFACT = "target/qa/visual/component-lab-capture.json"
MESH_PLOT_VISUAL_DIFF_ARTIFACT = "target/qa/visual/component-lab-diff.json"
MESH_PLOT_SCREEN_READER_RUNBOOK = "qa/accessibility/mesh-plot-screen-reader-qa.md"
MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT = (
    "target/qa/visual/mesh-plot-wgpu/actual/manifest.json"
)
MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT = (
    "qa/visual/baselines/mesh-plot-wgpu-v1/manifest.json"
)

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
    MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT,
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
# Presence of a benchmark binary alone is not enough to establish release
# performance coverage. Keep this list in terms of Criterion group/function
# names so the validator proves the workload matrix without depending on
# machine-specific timings or record ordering.
MESH_PLOT_BENCHMARK_WORKLOADS = {
    ("gpui-d3rs", "mesh_prep"): (
        ("mesh_prep", "prepare_100000_triangles"),
        ("mesh_prep", "prepare_200000_triangles"),
        ("mesh_prep", "marching_isolines_100000_triangles"),
        ("mesh_prep", "marching_bands_100000_triangles"),
        ("mesh_prep", "bvh_200000_triangles"),
        ("mesh_prep", "revolve_full_64_segments"),
        ("mesh_prep", "revolve_full_64_segments_with_vertex_field"),
        ("mesh_prep", "revolve_partial_capped_64_segments"),
        ("mesh_prep", "revolve_partial_capped_64_segments_with_vertex_field"),
        ("mesh_prep", "lod_proxy_upload_100000_triangles"),
        ("mesh_prep", "lod_full_restore_upload_100000_triangles"),
        ("mesh_prep", "lod_proxy_field_mapping_100000_triangles"),
        ("mesh_prep", "lod_drag_transition_100000_triangles"),
    ),
    ("gpui-px", "mesh_plot_frames"): (
        ("mesh_plot_build_200000_triangles", "mesh_plot_build_200000_triangles"),
        ("mesh_plot_fit_200000_triangles", "mesh_plot_fit_200000_triangles"),
        ("mesh_plot_retained_frames", "field_replace_100000_values"),
        ("mesh_plot_retained_frames", "camera_100000_values"),
        ("mesh_plot_retained_picking", "surface_bvh_pick"),
        ("mesh_plot_retained_picking", "revolved_bvh_pick"),
    ),
}
MESH_PLOT_BASELINE_MARKER = "px-mesh-plot"

OPTIONAL_ARTIFACTS = (
    "target/qa/visual/component-lab-capture.md",
    "target/qa/visual/component-lab-diff.md",
    MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT,
)

PLATFORM_EVIDENCE = {
    "android-emulator": "target/qa/platform/android-emulator/evidence.json",
    "ios-simulator": "target/qa/platform/ios-simulator/evidence.json",
    "tvos-simulator": "target/qa/platform/tvos-simulator/evidence.json",
}


class EvidenceError(RuntimeError):
    pass


def resolve_command(args: tuple[str, ...]) -> list[str]:
    """Resolve toolchain and archive commands in restricted shells."""

    if not args:
        return []
    executable = args[0]
    if Path(executable).is_absolute():
        return list(args)
    found = shutil.which(executable)
    if found:
        return [found, *args[1:]]
    fallback_dirs: tuple[Path, ...]
    if executable in {"cargo", "rustc", "rustup", "just"}:
        fallback_dirs = (Path.home() / ".cargo" / "bin",)
    elif executable in {"zstd", "tar"}:
        fallback_dirs = (
            Path("/opt/homebrew/bin"),
            Path("/usr/local/bin"),
            Path("/usr/bin"),
        )
    else:
        fallback_dirs = ()
    for directory in fallback_dirs:
        fallback = directory / executable
        if fallback.is_file() or fallback.is_symlink():
            return [str(fallback), *args[1:]]
    return list(args)


def command_output(root: Path, *args: str) -> str:
    completed = subprocess.run(
        resolve_command(args),
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


def validate_mesh_plot_benchmarks(
    root: Path,
    *,
    strict: bool = False,
    source_revision: str | None = None,
) -> None:
    """Require MeshPlot benchmark records in both release perf artifacts.

    Developer evidence only needs to identify the registered binaries because
    it may intentionally use a partial local run. A clean release manifest
    must additionally contain every named workload in
    ``MESH_PLOT_BENCHMARK_WORKLOADS``.
    """
    missing: list[str] = []
    for relative in ("qa/perf/baseline.json", "target/qa/perf/current.json"):
        path = root / relative
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
            raise EvidenceError(f"invalid performance artifact {relative}: {error}") from error
        records = data.get("records") if isinstance(data, dict) else None
        if strict and isinstance(data, dict):
            metadata = data.get("metadata")
            environment = metadata.get("environment") if isinstance(metadata, dict) else None
            if not isinstance(environment, dict) or environment.get("source_dirty") is not False:
                raise EvidenceError(
                    f"{relative}: strict release evidence must declare source_dirty: false"
                )
            if (
                relative == "target/qa/perf/current.json"
                and source_revision is not None
                and environment.get("source_revision") is not None
                and environment.get("source_revision") != source_revision
            ):
                raise EvidenceError(
                    f"{relative}: current performance evidence belongs to another source revision"
                )
        keys = {
            (record.get("crate"), record.get("bench"))
            for record in records
            if isinstance(record, dict)
        } if isinstance(records, list) else set()
        missing_binaries = sorted(MESH_PLOT_BENCHMARKS - keys)
        if missing_binaries:
            missing.extend(f"{relative}:{crate}:{bench}" for crate, bench in missing_binaries)
            raise EvidenceError(
                "MeshPlot benchmark evidence is missing; run the registered MeshPlot "
                "benchmarks on the reference host: " + ", ".join(missing)
            )
        if not strict:
            continue
        for crate, bench in sorted(MESH_PLOT_BENCHMARKS):
            record_keys = {
                (
                    str(record.get("group", "")),
                    str(record.get("function", "")),
                )
                for record in records
                if isinstance(record, dict)
                and record.get("crate") == crate
                and record.get("bench") == bench
            }
            missing_workloads = [
                f"{crate}:{bench}:{group}/{function}"
                for group, function in MESH_PLOT_BENCHMARK_WORKLOADS[(crate, bench)]
                if not any(
                    fnmatch.fnmatchcase(record_group, group)
                    and fnmatch.fnmatchcase(record_function, function)
                    for record_group, record_function in record_keys
                )
            ]
            if missing_workloads:
                raise EvidenceError(
                    "MeshPlot benchmark evidence is missing required workloads: "
                    + ", ".join(missing_workloads)
                )


def visual_baseline_members(path: Path) -> list[str]:
    """List members of the checked-in zstd-compressed visual archive."""
    try:
        completed = subprocess.run(
            resolve_command(("zstd", "-d", "-c", str(path))),
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        with tarfile.open(fileobj=io.BytesIO(completed.stdout), mode="r:") as archive:
            return [member.name for member in archive.getmembers()]
    except (OSError, subprocess.CalledProcessError, tarfile.TarError) as error:
        raise EvidenceError(f"invalid visual baseline archive {path.name}: {error}") from error


def validate_mesh_plot_visual_baseline(root: Path) -> set[str]:
    """Require all 99 versioned MeshPlot entries in the visual archive."""
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
            "MeshPlot visual baseline evidence must contain exactly 99 unique "
            "versioned PNG captures"
        )
    return set(ids)


def validate_mesh_plot_visual_diff(root: Path, baseline_ids: set[str]) -> None:
    """Require a passing zero-diff report for all 99 versioned captures."""
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
            "MeshPlot visual diff must report 99 compared cases, zero failures, "
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
            "MeshPlot visual diff cases must match the 99 versioned baseline captures"
        )


def validate_mesh_plot_wgpu_visual(root: Path, *, require_baseline: bool) -> None:
    """Validate the adapter-backed WGPU capture and optional release baseline."""

    actual_path = root / MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT
    try:
        actual = validate_manifest(
            actual_path,
            repo_root=root,
            require_images=True,
            allow_skipped=True,
        )
    except WgpuManifestError as error:
        raise EvidenceError(str(error)) from error

    if actual.get("status", "captured") == "skipped":
        if require_baseline:
            raise EvidenceError("WGPU visual evidence is skipped but is required for release")
        return

    baseline_path = root / MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT
    if not baseline_path.is_file():
        if require_baseline:
            raise EvidenceError(
                "missing WGPU visual baseline; promote "
                f"{MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT} from a clean release run"
            )
        return
    try:
        baseline = validate_manifest(
            baseline_path,
            repo_root=root,
            require_images=False,
            allow_skipped=False,
        )
        compare_manifests(actual, baseline)
    except WgpuManifestError as error:
        raise EvidenceError(str(error)) from error


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
    require_wgpu_visual: bool | None = None,
    required_platforms: Iterable[str] = (),
) -> dict[str, object]:
    if require_wgpu_visual is None:
        require_wgpu_visual = require_clean
    source = source_provenance(root)
    if require_clean and source["dirty"]:
        raise EvidenceError("release evidence requires a clean worktree")
    revision = str(source["revision"])
    artifacts = collect_artifacts(root, revision)
    validate_mesh_plot_benchmarks(
        root,
        strict=require_clean,
        source_revision=revision,
    )
    validate_mesh_plot_visual_capture(root)
    baseline_ids = validate_mesh_plot_visual_baseline(root)
    validate_mesh_plot_visual_diff(root, baseline_ids)
    validate_mesh_plot_wgpu_visual(root, require_baseline=require_wgpu_visual)
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
