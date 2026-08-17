#!/usr/bin/env python3
"""Bind GPUI Toolkit release QA artifacts to source and toolchain provenance."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import io
import json
import math
import platform
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import Iterable

from mesh_wgpu_manifest import (
    COMPARISON_IDS,
    WgpuManifestError,
    compare_manifests,
    validate_manifest,
)
from mesh_plot_expanded_visual import ExpandedVisualError, validate_expanded_report
from mesh_plot_visual_compare import VisualCompareError, _decode_png


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-toolkit-release-evidence-manifest"

MESH_PLOT_LOCAL_CAPTURE_COUNT = 99
MESH_PLOT_VERSIONED_BASELINE_COUNT = 99
MESH_PLOT_VISUAL_CAPTURE_ARTIFACT = "target/qa/visual/component-lab-capture.json"
MESH_PLOT_VISUAL_DIFF_ARTIFACT = "target/qa/visual/component-lab-diff.json"
MESH_PLOT_SCREEN_READER_RUNBOOK = "qa/accessibility/mesh-plot-screen-reader-qa.md"
MESH_PLOT_CVD_RUNBOOK = "qa/visual/mesh-plot-cvd-qa.md"
MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT = (
    "target/qa/visual/mesh-plot-wgpu/actual/manifest.json"
)
MESH_PLOT_METAL_VISUAL_CAPTURE_ARTIFACT = (
    "target/qa/visual/mesh-plot-metal/actual/manifest.json"
)
MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT = (
    "qa/visual/baselines/mesh-plot-wgpu-v1/manifest.json"
)
MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT = (
    "target/qa/visual/mesh-plot-cross-adapter.json"
)
MESH_PLOT_EXPANDED_CROSS_ADAPTER_VISUAL_ARTIFACT = (
    "target/qa/visual/mesh-plot-cross-adapter-expanded.json"
)
MESH_PLOT_CVD_ARTIFACT = "target/qa/visual/mesh-plot-cvd.json"
MESH_PLOT_PRODUCT_VISUAL_DIR = "target/qa/visual/mesh-plot-product"
MESH_PLOT_PRODUCT_VISUAL_ARTIFACT = f"{MESH_PLOT_PRODUCT_VISUAL_DIR}/manifest.json"
MESH_PLOT_PRODUCT_CASE_IDS = (
    "metal-plain",
    "metal-selected",
    "wgpu-plain",
    "wgpu-selected",
)
MESH_COMPUTE_EVIDENCE_ARTIFACT = "target/qa/perf/mesh-compute-gpu.json"
MESH_LOD_EVIDENCE_ARTIFACT = "target/qa/perf/mesh-lod/mesh-lod-evidence.json"
MESH_METAL_MEMORY_EVIDENCE_ARTIFACT = (
    "target/qa/perf/mesh-metal-memory/mesh-metal-memory-evidence.json"
)

REQUIRED_ARTIFACTS = (
    "qa/perf/baseline.json",
    MESH_PLOT_SCREEN_READER_RUNBOOK,
    MESH_PLOT_CVD_RUNBOOK,
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
    MESH_PLOT_PRODUCT_VISUAL_ARTIFACT,
    MESH_COMPUTE_EVIDENCE_ARTIFACT,
    MESH_LOD_EVIDENCE_ARTIFACT,
    MESH_METAL_MEMORY_EVIDENCE_ARTIFACT,
    MESH_PLOT_CVD_ARTIFACT,
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
        ("mesh_plot_png_frame", "mesh_plot_png_frame"),
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
    MESH_PLOT_METAL_VISUAL_CAPTURE_ARTIFACT,
    MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT,
    MESH_PLOT_EXPANDED_CROSS_ADAPTER_VISUAL_ARTIFACT,
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
    product_dir = root / MESH_PLOT_PRODUCT_VISUAL_DIR
    if product_dir.is_dir():
        paths.update(
            path.relative_to(root).as_posix()
            for path in product_dir.rglob("*")
            if path.is_file()
        )
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


def validate_visual_source_binding(
    report: dict[str, object],
    description: str,
    *,
    require_clean: bool,
    source_revision: str | None,
) -> None:
    """Validate optional developer provenance and strict source binding."""

    embedded_revision = report.get("source_revision")
    embedded_dirty = report.get("source_dirty")
    if embedded_revision is not None and not isinstance(embedded_revision, str):
        raise EvidenceError(f"{description} source_revision must be a string")
    if embedded_dirty is not None and not isinstance(embedded_dirty, bool):
        raise EvidenceError(f"{description} source_dirty must be boolean")
    if require_clean and (
        not isinstance(source_revision, str)
        or embedded_revision != source_revision
        or embedded_dirty is not False
    ):
        raise EvidenceError(
            f"{description} is missing clean source provenance or belongs to another source revision"
        )


def validate_mesh_plot_visual_capture(
    root: Path,
    *,
    require_clean: bool = False,
    source_revision: str | None = None,
) -> None:
    """Require all 99 local MeshPlot actual captures to be present."""
    capture = read_json_object(
        root / MESH_PLOT_VISUAL_CAPTURE_ARTIFACT,
        "MeshPlot local visual capture",
    )
    validate_visual_source_binding(
        capture,
        "MeshPlot local visual capture",
        require_clean=require_clean,
        source_revision=source_revision,
    )
    if capture.get("status") == "skipped":
        reason = capture.get("reason")
        if not isinstance(reason, str) or not reason.strip():
            raise EvidenceError(
                "skipped MeshPlot local visual capture must include a reason"
            )
        if require_clean:
            raise EvidenceError(
                "MeshPlot local visual capture is skipped but is required for release"
            )
        return
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


def validate_mesh_plot_visual_diff(
    root: Path,
    baseline_ids: set[str],
    *,
    strict: bool = False,
    source_revision: str | None = None,
) -> None:
    """Validate all 99 visual cases, requiring zero diff only in strict mode.

    The developer lane must be able to describe a complete capture whose
    checked-in baselines are known to be stale. Release mode still requires a
    passing zero-diff report, so stale visuals cannot be promoted silently.
    """
    diff = read_json_object(
        root / MESH_PLOT_VISUAL_DIFF_ARTIFACT,
        "MeshPlot visual diff",
    )
    validate_visual_source_binding(
        diff,
        "MeshPlot visual diff",
        require_clean=strict,
        source_revision=source_revision,
    )
    if diff.get("status") == "skipped":
        reason = diff.get("reason")
        if not isinstance(reason, str) or not reason.strip():
            raise EvidenceError("skipped MeshPlot visual diff must include a reason")
        if strict:
            raise EvidenceError(
                "MeshPlot visual diff is skipped but is required for release"
            )
        return
    cases = diff.get("cases")
    if (
        diff.get("report_type") != "gpui-component-lab-visual-diff"
        or not isinstance(diff.get("passed"), bool)
        or diff.get("compared_count") != MESH_PLOT_VERSIONED_BASELINE_COUNT
        or not isinstance(diff.get("failed_count"), int)
        or isinstance(diff.get("failed_count"), bool)
        or diff.get("failed_count") < 0
        or not isinstance(diff.get("max_changed_pixels"), int)
        or isinstance(diff.get("max_changed_pixels"), bool)
        or diff.get("max_changed_pixels") < 0
        or not isinstance(cases, list)
        or len(cases) != MESH_PLOT_VERSIONED_BASELINE_COUNT
    ):
        raise EvidenceError("MeshPlot visual diff must report all 99 compared cases")

    diff_ids: list[str] = []
    for case in cases:
        if not isinstance(case, dict):
            raise EvidenceError("MeshPlot visual diff contains a malformed case")
        capture_id = case.get("capture_id")
        status = case.get("status")
        changed_pixels = case.get("changed_pixels")
        max_channel_delta = case.get("max_channel_delta")
        artifact_paths = {
            key: case.get(key)
            for key in ("baseline_path", "actual_path", "diff_path")
        }
        if (
            not isinstance(capture_id, str)
            or capture_id in diff_ids
            or status not in {"Passed", "Different"}
            or not isinstance(changed_pixels, int)
            or isinstance(changed_pixels, bool)
            or changed_pixels < 0
            or not isinstance(max_channel_delta, int)
            or isinstance(max_channel_delta, bool)
            or not 0 <= max_channel_delta <= 255
        ):
            raise EvidenceError("MeshPlot visual diff contains an invalid or duplicate case")
        if status == "Passed" and (changed_pixels != 0 or max_channel_delta != 0):
            raise EvidenceError("passed MeshPlot visual diff cases must have zero changes")
        if status == "Different" and changed_pixels == 0 and max_channel_delta == 0:
            raise EvidenceError("different MeshPlot visual diff cases must report a change")
        if strict and any(not isinstance(value, str) or not value for value in artifact_paths.values()):
            raise EvidenceError(
                f"strict MeshPlot visual diff case {capture_id} must include artifact paths"
            )
        for key, path_text in artifact_paths.items():
            if path_text is None:
                continue
            if not isinstance(path_text, str) or Path(path_text).suffix.lower() != ".png":
                raise EvidenceError(
                    f"MeshPlot visual diff case {capture_id} has an invalid {key}"
                )
            artifact = _safe_repo_artifact_path(
                root,
                path_text,
                f"MeshPlot visual diff case {capture_id} {key}",
            )
            if not artifact.is_file() or artifact.stat().st_size == 0:
                raise EvidenceError(
                    f"missing MeshPlot visual diff artifact {key}: {path_text}"
                )
        diff_ids.append(capture_id)

    if set(diff_ids) != baseline_ids:
        raise EvidenceError(
            "MeshPlot visual diff cases must match the 99 versioned baseline captures"
        )
    observed_failed = sum(case.get("status") == "Different" for case in cases)
    if diff.get("failed_count") != observed_failed or diff.get("passed") != (observed_failed == 0):
        raise EvidenceError("MeshPlot visual diff summary does not match its cases")
    if strict and (
        observed_failed != 0
        or diff.get("max_changed_pixels") != 0
        or diff.get("passed") is not True
    ):
        raise EvidenceError(
            "MeshPlot visual diff must report 99 compared cases, zero failures, "
            "zero changed pixels, and a passing diff run"
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


def validate_mesh_plot_metal_visual(root: Path, *, require_capture: bool) -> None:
    """Validate the native Metal capture and its adapter availability contract."""

    actual_path = root / MESH_PLOT_METAL_VISUAL_CAPTURE_ARTIFACT
    try:
        actual = validate_manifest(
            actual_path,
            repo_root=root,
            require_images=True,
            allow_skipped=not require_capture,
            expected_renderer="metal-headless",
        )
    except WgpuManifestError as error:
        raise EvidenceError(str(error)) from error

    if actual.get("status", "captured") == "skipped" and require_capture:
        raise EvidenceError("Metal visual evidence is skipped but is required for release")


def validate_mesh_plot_product_visual(
    root: Path,
    *,
    require_capture: bool,
    source_revision: str,
) -> None:
    """Validate the high-level GPUI MeshPlot product capture contract."""

    path = root / MESH_PLOT_PRODUCT_VISUAL_ARTIFACT
    if not path.is_file():
        if require_capture:
            raise EvidenceError(
                "missing high-level MeshPlot product capture manifest; run "
                "scripts/qa_mesh_product_visual.sh on the reference host"
            )
        return

    manifest = read_json_object(path, "MeshPlot product visual capture")
    status = manifest.get("status", "captured")
    if status == "skipped":
        if require_capture:
            raise EvidenceError(
                "high-level MeshPlot product visual evidence is skipped but is required"
            )
        if not isinstance(manifest.get("reason"), str) or not manifest["reason"].strip():
            raise EvidenceError("skipped MeshPlot product evidence must include a reason")
        return
    if status != "captured":
        raise EvidenceError(f"unknown MeshPlot product visual status: {status!r}")

    if (
        manifest.get("schema_version") != 1
        or manifest.get("report_type") != "gpui-mesh-plot-product-capture"
        or manifest.get("logical_width") != 600
        or manifest.get("logical_height") != 400
        or manifest.get("width") != 1200
        or manifest.get("height") != 800
    ):
        raise EvidenceError("MeshPlot product visual manifest has an invalid schema or size")

    composition = manifest.get("composition")
    if (
        not isinstance(composition, dict)
        or composition.get("axes_present") is not True
        or composition.get("axis_titles") != ["x", "y"]
        or composition.get("colorbar_present") is not True
        or composition.get("selection_annotation_contract") is not True
    ):
        raise EvidenceError(
            "MeshPlot product visual manifest must declare axes, axis titles, "
            "colorbar, and selection annotation composition"
        )

    embedded_revision = manifest.get("source_revision")
    embedded_dirty = manifest.get("source_dirty")
    if embedded_revision is not None and not isinstance(embedded_revision, str):
        raise EvidenceError("MeshPlot product source_revision must be a string")
    if embedded_dirty is not None and not isinstance(embedded_dirty, bool):
        raise EvidenceError("MeshPlot product source_dirty must be boolean")
    if require_capture and (
        embedded_revision != source_revision or embedded_dirty is not False
    ):
        raise EvidenceError(
            "MeshPlot product capture is dirty or belongs to another source revision"
        )

    cases = manifest.get("cases")
    if not isinstance(cases, list) or len(cases) != len(MESH_PLOT_PRODUCT_CASE_IDS):
        raise EvidenceError("MeshPlot product visual manifest must contain four cases")

    root_resolved = root.resolve()
    seen_ids: set[str] = set()
    seen_paths: set[Path] = set()
    decoded: dict[str, tuple[int, int, bytes]] = {}
    for case in cases:
        if not isinstance(case, dict):
            raise EvidenceError("MeshPlot product visual manifest has a malformed case")
        case_id = case.get("id")
        if (
            not isinstance(case_id, str)
            or case_id not in MESH_PLOT_PRODUCT_CASE_IDS
            or case_id in seen_ids
        ):
            raise EvidenceError("MeshPlot product visual manifest has invalid case IDs")
        renderer = "metal-headless" if case_id.startswith("metal-") else "wgpu-headless"
        state = "selected" if case_id.endswith("selected") else "plain"
        comparison_id = (
            "px.mesh_plot.product.selection"
            if state == "selected"
            else "px.mesh_plot.product.axes"
        )
        if (
            case.get("renderer") != renderer
            or case.get("state") != state
            or case.get("comparison_id") != comparison_id
            or case.get("artifact_kind") != "png"
            or case.get("axes_present") is not True
            or case.get("selection_annotation") is not (state == "selected")
        ):
            raise EvidenceError(f"MeshPlot product case {case_id} has invalid metadata")
        path_text = case.get("path")
        if not isinstance(path_text, str) or Path(path_text).suffix.lower() != ".png":
            raise EvidenceError(f"MeshPlot product case {case_id} must reference a PNG")
        image = _safe_repo_artifact_path(
            root,
            path_text,
            f"MeshPlot product case {case_id}",
            base=path.parent,
        )
        if not image.is_file() or image.stat().st_size == 0:
            raise EvidenceError(f"missing MeshPlot product image: {path_text}")
        try:
            image.resolve().relative_to(root_resolved)
            decoded[case_id] = _decode_png(image)
        except (ValueError, VisualCompareError) as error:
            raise EvidenceError(f"invalid MeshPlot product image {path_text}: {error}") from error
        if decoded[case_id][0:2] != (1200, 800):
            raise EvidenceError(f"MeshPlot product case {case_id} has unexpected dimensions")
        seen_ids.add(case_id)
        seen_paths.add(image.resolve())

    if set(seen_ids) != set(MESH_PLOT_PRODUCT_CASE_IDS) or len(seen_paths) != 4:
        raise EvidenceError("MeshPlot product visual cases are incomplete or duplicated")

    paired = manifest.get("paired_comparison")
    if not isinstance(paired, dict):
        raise EvidenceError("MeshPlot product capture has no paired axes/selection comparison")
    expected_pairs = {
        "axes": (
            "px.mesh_plot.product.axes",
            "metal-plain",
            "wgpu-plain",
        ),
        "selection": (
            "px.mesh_plot.product.selection",
            "metal-selected",
            "wgpu-selected",
        ),
    }
    for pair_name, (comparison_id, metal_case, wgpu_case) in expected_pairs.items():
        pair = paired.get(pair_name)
        if not isinstance(pair, dict) or (
            pair.get("comparison_id") != comparison_id
            or pair.get("metal_case") != metal_case
            or pair.get("wgpu_case") != wgpu_case
        ):
            raise EvidenceError(f"MeshPlot product paired {pair_name} comparison is malformed")
        changed_pixels = pair.get("changed_pixels")
        changed_fraction = pair.get("changed_fraction")
        if (
            not isinstance(changed_pixels, int)
            or isinstance(changed_pixels, bool)
            or changed_pixels < 0
            or not isinstance(changed_fraction, (int, float))
            or isinstance(changed_fraction, bool)
            or not math.isfinite(float(changed_fraction))
            or not 0.0 <= float(changed_fraction) <= 1.0
        ):
            raise EvidenceError(f"MeshPlot product paired {pair_name} metrics are invalid")

    for renderer in ("metal", "wgpu"):
        plain = decoded[f"{renderer}-plain"]
        selected = decoded[f"{renderer}-selected"]
        if plain[0:2] != selected[0:2]:
            raise EvidenceError(f"MeshPlot {renderer} product captures have different sizes")
        changed = sum(
            1
            for offset in range(0, len(plain[2]), 4)
            if plain[2][offset : offset + 4] != selected[2][offset : offset + 4]
        )
        selected_case = next(case for case in cases if case["id"] == f"{renderer}-selected")
        if changed <= 0 or selected_case.get("changed_pixels_from_plain") != changed:
            raise EvidenceError(
                f"MeshPlot {renderer} selected product capture must differ from plain output"
            )


def validate_mesh_plot_cvd(
    root: Path,
    *,
    require_capture: bool,
    source_revision: str,
) -> None:
    """Validate the automated CVD screen over rendered product captures."""

    path = root / MESH_PLOT_CVD_ARTIFACT
    if not path.is_file():
        if require_capture:
            raise EvidenceError(
                "missing MeshPlot CVD screen; run scripts/qa_mesh_cvd.sh "
                "on the product capture host"
            )
        return
    report = read_json_object(path, "MeshPlot CVD screen")
    status = report.get("status", "captured")
    if status == "skipped":
        if require_capture:
            raise EvidenceError("MeshPlot CVD screen is skipped but is required")
        if not isinstance(report.get("reason"), str) or not report["reason"].strip():
            raise EvidenceError("skipped MeshPlot CVD screen must include a reason")
        return
    if status != "captured":
        raise EvidenceError(f"unknown MeshPlot CVD screen status: {status!r}")
    if (
        report.get("schema_version") != 1
        or report.get("report_type") != "gpui-mesh-plot-cvd-screen"
        or report.get("manual_review_required") is not True
    ):
        raise EvidenceError("MeshPlot CVD screen has an invalid schema")
    embedded_revision = report.get("source_revision")
    embedded_dirty = report.get("source_dirty")
    if embedded_revision is not None and not isinstance(embedded_revision, str):
        raise EvidenceError("MeshPlot CVD source_revision must be a string")
    if embedded_dirty is not None and not isinstance(embedded_dirty, bool):
        raise EvidenceError("MeshPlot CVD source_dirty must be boolean")
    if require_capture and (
        embedded_revision != source_revision or embedded_dirty is not False
    ):
        raise EvidenceError("MeshPlot CVD screen is dirty or belongs to another source revision")

    cases = report.get("cases")
    if (
        not isinstance(cases, list)
        or {case.get("id") for case in cases if isinstance(case, dict)}
        != set(MESH_PLOT_PRODUCT_CASE_IDS)
    ):
        raise EvidenceError("MeshPlot CVD screen must describe all four product cases")
    for case in cases:
        if not isinstance(case, dict):
            raise EvidenceError("MeshPlot CVD screen contains a malformed case")
        if case.get("width") != 1200 or case.get("height") != 800:
            raise EvidenceError("MeshPlot CVD case has unexpected dimensions")

    deficiencies = report.get("deficiencies")
    if not isinstance(deficiencies, dict) or set(deficiencies) != {"protan", "deutan", "tritan"}:
        raise EvidenceError("MeshPlot CVD screen must cover protan, deutan, and tritan")
    for name, metrics in deficiencies.items():
        if not isinstance(metrics, dict):
            raise EvidenceError(f"MeshPlot CVD {name} metrics are malformed")
        metric_cases = metrics.get("cases")
        deltas = metrics.get("selection_changed_pixels")
        if (
            not isinstance(metric_cases, dict)
            or set(metric_cases) != set(MESH_PLOT_PRODUCT_CASE_IDS)
            or not isinstance(deltas, dict)
        ):
            raise EvidenceError(f"MeshPlot CVD {name} metrics are incomplete")
        for case_id, case_metrics in metric_cases.items():
            if (
                not isinstance(case_metrics, dict)
                or not isinstance(case_metrics.get("unique_rgb_colors"), int)
                or isinstance(case_metrics.get("unique_rgb_colors"), bool)
                or case_metrics["unique_rgb_colors"] <= 0
                or case_metrics.get("finite") is not True
            ):
                raise EvidenceError(f"MeshPlot CVD {name} case {case_id} metrics are invalid")
        for renderer in ("metal", "wgpu"):
            changed = deltas.get(renderer)
            if not isinstance(changed, int) or isinstance(changed, bool) or changed <= 0:
                raise EvidenceError(
                    f"MeshPlot CVD {name} {renderer} selection is not distinguishable"
                )


def validate_mesh_compute_evidence(
    root: Path,
    *,
    require_capture: bool,
    source_revision: str,
) -> None:
    """Validate adapter-backed Metal compute parity and timing evidence."""

    path = root / MESH_COMPUTE_EVIDENCE_ARTIFACT
    if not path.is_file():
        if require_capture:
            raise EvidenceError(
                "missing MeshPlot compute evidence; run scripts/qa_mesh_compute.sh "
                "on the reference host"
            )
        return
    manifest = read_json_object(path, "MeshPlot compute evidence")
    status = manifest.get("status", "captured")
    if status == "skipped":
        if require_capture:
            raise EvidenceError("MeshPlot compute evidence is skipped but is required")
        if not isinstance(manifest.get("reason"), str) or not manifest["reason"].strip():
            raise EvidenceError("skipped MeshPlot compute evidence must include a reason")
        return
    if status != "captured":
        raise EvidenceError(f"unknown MeshPlot compute evidence status: {status!r}")
    if (
        manifest.get("schema_version") != 1
        or manifest.get("report_type") != "gpui-mesh-compute-gpu-evidence"
        or manifest.get("adapter_backed") is not True
    ):
        raise EvidenceError("MeshPlot compute evidence has an invalid schema")
    backend = manifest.get("backend")
    if not isinstance(backend, str) or not backend.strip():
        raise EvidenceError("MeshPlot compute evidence has no adapter backend")
    if require_capture and backend != "metal":
        raise EvidenceError(f"MeshPlot compute release evidence must use Metal, got {backend!r}")

    embedded_revision = manifest.get("source_revision")
    embedded_dirty = manifest.get("source_dirty")
    if embedded_revision is not None and not isinstance(embedded_revision, str):
        raise EvidenceError("MeshPlot compute source_revision must be a string")
    if embedded_dirty is not None and not isinstance(embedded_dirty, bool):
        raise EvidenceError("MeshPlot compute source_dirty must be boolean")
    if require_capture and (
        embedded_revision != source_revision or embedded_dirty is not False
    ):
        raise EvidenceError(
            "MeshPlot compute evidence is dirty or belongs to another source revision"
        )

    parity = manifest.get("parity")
    if (
        not isinstance(parity, dict)
        or parity.get("field_min_max") is not True
        or parity.get("isolines") is not True
        or parity.get("filled_bands") is not True
    ):
        raise EvidenceError("MeshPlot compute evidence must pass all CPU parity checks")
    timing = manifest.get("timing")
    if not isinstance(timing, dict):
        raise EvidenceError("MeshPlot compute evidence has no timing record")
    sample_count = timing.get("sample_count")
    gpu_time_ns = timing.get("last_gpu_time_ns")
    for name, value in (("sample_count", sample_count), ("last_gpu_time_ns", gpu_time_ns)):
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise EvidenceError(f"MeshPlot compute timing {name} is invalid")
    if require_capture and (
        timing.get("requested") is not True
        or timing.get("enabled") is not True
        or sample_count <= 0
        or gpu_time_ns <= 0
    ):
        raise EvidenceError(
            "Metal compute release evidence requires enabled non-zero GPU timestamps"
        )


def validate_mesh_lod_evidence(
    root: Path,
    *,
    require_capture: bool,
    source_revision: str,
) -> None:
    """Validate adapter-backed drag-time LOD quality and frame-budget evidence."""

    path = root / MESH_LOD_EVIDENCE_ARTIFACT
    if not path.is_file():
        if require_capture:
            raise EvidenceError(
                "missing MeshPlot LOD evidence; run scripts/qa_mesh_lod.sh "
                "on the reference host"
            )
        return
    manifest = read_json_object(path, "MeshPlot LOD evidence")
    status = manifest.get("status", "captured")
    if status == "skipped":
        if require_capture:
            raise EvidenceError("MeshPlot LOD evidence is skipped but is required")
        if not isinstance(manifest.get("reason"), str) or not manifest["reason"].strip():
            raise EvidenceError("skipped MeshPlot LOD evidence must include a reason")
        return
    if status != "captured":
        raise EvidenceError(f"unknown MeshPlot LOD evidence status: {status!r}")
    if (
        manifest.get("schema_version") != 1
        or manifest.get("report_type") != "gpui-mesh-lod-evidence"
        or manifest.get("adapter_backed") is not True
    ):
        raise EvidenceError("MeshPlot LOD evidence has an invalid schema")
    backend = manifest.get("backend")
    if not isinstance(backend, str) or not backend.strip():
        raise EvidenceError("MeshPlot LOD evidence has no adapter backend")
    if require_capture and backend != "metal":
        raise EvidenceError(f"MeshPlot LOD release evidence must use Metal, got {backend!r}")

    embedded_revision = manifest.get("source_revision")
    embedded_dirty = manifest.get("source_dirty")
    if embedded_revision is not None and not isinstance(embedded_revision, str):
        raise EvidenceError("MeshPlot LOD source_revision must be a string")
    if embedded_dirty is not None and not isinstance(embedded_dirty, bool):
        raise EvidenceError("MeshPlot LOD source_dirty must be boolean")
    if require_capture and (
        embedded_revision != source_revision or embedded_dirty is not False
    ):
        raise EvidenceError("MeshPlot LOD evidence is dirty or belongs to another source revision")

    workload = manifest.get("workload")
    if (
        not isinstance(workload, dict)
        or not isinstance(workload.get("full_triangle_count"), int)
        or not isinstance(workload.get("proxy_triangle_count"), int)
        or workload.get("full_triangle_count") <= 0
        or workload.get("proxy_triangle_count") <= 0
        or workload.get("proxy_triangle_count") >= workload.get("full_triangle_count")
        or workload.get("proxy_reduces_triangles") is not True
    ):
        raise EvidenceError("MeshPlot LOD evidence must prove a smaller drag proxy")

    quality = manifest.get("visual_quality")
    if not isinstance(quality, dict) or quality.get("passed") is not True:
        raise EvidenceError("MeshPlot LOD visual-quality evidence did not pass")
    width = quality.get("width")
    height = quality.get("height")
    for name, value in (("width", width), ("height", height)):
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise EvidenceError(f"MeshPlot LOD visual quality {name} is invalid")
    for name in ("full_non_black_pixels", "proxy_non_black_pixels"):
        value = quality.get(name)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise EvidenceError(f"MeshPlot LOD visual quality {name} is invalid")
    changed_fraction = quality.get("proxy_full_changed_fraction")
    if (
        not isinstance(changed_fraction, (int, float))
        or isinstance(changed_fraction, bool)
        or not math.isfinite(float(changed_fraction))
        or not 0.0 <= float(changed_fraction) <= 1.0
    ):
        raise EvidenceError("MeshPlot LOD visual quality changed fraction is invalid")
    for name in ("proxy_path", "full_path"):
        image_text = quality.get(name)
        if not isinstance(image_text, str) or Path(image_text).suffix.lower() != ".png":
            raise EvidenceError(f"MeshPlot LOD visual quality {name} must reference a PNG")
        image = _safe_repo_artifact_path(root, image_text, f"MeshPlot LOD {name}", base=path.parent)
        if not image.is_file() or image.stat().st_size == 0:
            raise EvidenceError(f"missing MeshPlot LOD image: {image_text}")
        try:
            actual_width, actual_height, _ = _decode_png(image)
        except VisualCompareError as error:
            raise EvidenceError(f"invalid MeshPlot LOD image {image_text}: {error}") from error
        if (actual_width, actual_height) != (width, height):
            raise EvidenceError(f"MeshPlot LOD image {image_text} has unexpected dimensions")

    budget = manifest.get("frame_budget")
    if not isinstance(budget, dict):
        raise EvidenceError("MeshPlot LOD evidence has no frame-budget record")
    sample_count = budget.get("sample_count")
    target_ns = budget.get("target_average_ns")
    average_ns = budget.get("average_ns")
    max_ns = budget.get("max_ns")
    for name, value in (
        ("sample_count", sample_count),
        ("target_average_ns", target_ns),
        ("average_ns", average_ns),
        ("max_ns", max_ns),
    ):
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise EvidenceError(f"MeshPlot LOD frame budget {name} is invalid")
    if sample_count < 60 or average_ns > target_ns:
        raise EvidenceError("MeshPlot LOD frame budget exceeded the recorded target")
    if require_capture and budget.get("passed") is not True:
        raise EvidenceError("MeshPlot LOD release evidence must pass the frame budget")

    telemetry = manifest.get("telemetry")
    if not isinstance(telemetry, dict):
        raise EvidenceError("MeshPlot LOD evidence has no retained telemetry")
    restored_uploads = telemetry.get("restored_geometry_upload_count")
    restored_frames = telemetry.get("restored_gpu_frame_count")
    if (
        not isinstance(restored_uploads, int)
        or isinstance(restored_uploads, bool)
        or restored_uploads < 3
        or not isinstance(restored_frames, int)
        or isinstance(restored_frames, bool)
        or restored_frames < sample_count + 2
    ):
        raise EvidenceError("MeshPlot LOD telemetry does not prove proxy/full restoration")


def validate_mesh_metal_memory_evidence(
    root: Path,
    *,
    require_capture: bool,
    source_revision: str,
) -> None:
    """Validate long-run Metal driver-allocation and teardown evidence."""

    path = root / MESH_METAL_MEMORY_EVIDENCE_ARTIFACT
    if not path.is_file():
        if require_capture:
            raise EvidenceError(
                "missing MeshPlot Metal memory evidence; run "
                "scripts/qa_mesh_metal_memory.sh on the reference host"
            )
        return
    manifest = read_json_object(path, "MeshPlot Metal memory evidence")
    status = manifest.get("status", "captured")
    if status == "skipped":
        if require_capture:
            raise EvidenceError("MeshPlot Metal memory evidence is skipped but is required")
        if not isinstance(manifest.get("reason"), str) or not manifest["reason"].strip():
            raise EvidenceError("skipped MeshPlot Metal memory evidence must include a reason")
        return
    if status != "captured":
        raise EvidenceError(f"unknown MeshPlot Metal memory status: {status!r}")
    if (
        manifest.get("schema_version") != 1
        or manifest.get("report_type") != "gpui-mesh-metal-memory-evidence"
        or manifest.get("backend") != "metal"
        or manifest.get("adapter_backed") is not True
    ):
        raise EvidenceError("MeshPlot Metal memory evidence has an invalid schema")

    embedded_revision = manifest.get("source_revision")
    embedded_dirty = manifest.get("source_dirty")
    if embedded_revision is not None and not isinstance(embedded_revision, str):
        raise EvidenceError("MeshPlot Metal memory source_revision must be a string")
    if embedded_dirty is not None and not isinstance(embedded_dirty, bool):
        raise EvidenceError("MeshPlot Metal memory source_dirty must be boolean")
    if require_capture and (
        embedded_revision != source_revision or embedded_dirty is not False
    ):
        raise EvidenceError(
            "MeshPlot Metal memory evidence is dirty or belongs to another source revision"
        )

    sample_count = manifest.get("sample_count")
    samples = manifest.get("samples")
    if (
        not isinstance(sample_count, int)
        or isinstance(sample_count, bool)
        or sample_count < 20
        or not isinstance(samples, list)
        or len(samples) != sample_count
    ):
        raise EvidenceError("MeshPlot Metal memory evidence must contain 20 samples")
    previous_peak = 0
    for sample in samples:
        if not isinstance(sample, dict):
            raise EvidenceError("MeshPlot Metal memory evidence has a malformed sample")
        current = sample.get("driver_allocated_bytes")
        peak = sample.get("peak_driver_allocated_bytes")
        if (
            not isinstance(current, int)
            or isinstance(current, bool)
            or current <= 0
            or not isinstance(peak, int)
            or isinstance(peak, bool)
            or peak < current
            or peak < previous_peak
        ):
            raise EvidenceError("Metal driver allocation samples must be positive and monotonic")
        previous_peak = peak

    before = manifest.get("before_drop")
    after = manifest.get("after_drop")
    contracts = manifest.get("contracts")
    if not isinstance(before, dict) or not isinstance(after, dict) or not isinstance(contracts, dict):
        raise EvidenceError("MeshPlot Metal memory evidence is missing lifecycle records")
    before_current = before.get("driver_allocated_bytes")
    before_peak = before.get("peak_driver_allocated_bytes")
    if (
        not isinstance(before_current, int)
        or isinstance(before_current, bool)
        or before_current <= 0
        or not isinstance(before_peak, int)
        or isinstance(before_peak, bool)
        or before_peak < before_current
    ):
        raise EvidenceError("Metal memory evidence has invalid pre-teardown allocation")
    if (
        after.get("driver_allocated_bytes") is not None
        or after.get("resident_bytes") != 0
        or not isinstance(after.get("memory_release_count"), int)
        or isinstance(after.get("memory_release_count"), bool)
        or after.get("memory_release_count") <= 0
        or after.get("peak_driver_allocated_bytes") != before_peak
    ):
        raise EvidenceError("Metal memory evidence must prove teardown release and peak retention")
    if require_capture and not all(
        contracts.get(name) is True
        for name in (
            "alternating_field_and_geometry_churn",
            "driver_peak_is_monotonic",
            "teardown_clears_current_memory",
            "teardown_preserves_peak",
        )
    ):
        raise EvidenceError("Metal memory release evidence did not pass all lifecycle contracts")


def _safe_repo_artifact_path(
    root: Path,
    path_text: str,
    description: str,
    *,
    base: Path | None = None,
) -> Path:
    candidate = Path(path_text)
    if (
        candidate.is_absolute()
        or not path_text
        or not candidate.parts
        or "\\" in path_text
        or ".." in candidate.parts
    ):
        raise EvidenceError(f"{description} contains an unsafe path: {path_text!r}")
    resolved = ((base or root) / candidate).resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError as error:
        raise EvidenceError(f"{description} escapes the repository: {path_text!r}") from error
    return resolved


def validate_mesh_plot_cross_adapter_visual(
    root: Path,
    *,
    require_report: bool,
    report_path: Path | None = None,
) -> None:
    """Validate the paired Metal/WGPU visual report when release evidence requires it."""

    path = report_path or root / MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT
    if report_path is not None:
        try:
            path.resolve().relative_to(root.resolve())
        except ValueError as error:
            raise EvidenceError(
                f"cross-adapter visual report escapes the repository: {path}"
            ) from error
    if not path.is_file():
        if require_report:
            raise EvidenceError(
                "missing cross-adapter MeshPlot visual report; run the paired "
                "Metal/WGPU capture lane on the reference host"
            )
        return

    report = read_json_object(path, "MeshPlot cross-adapter visual report")
    expected_ids = set(COMPARISON_IDS.values())
    cases = report.get("cases")
    left_renderer = report.get("left_renderer")
    right_renderer = report.get("right_renderer")
    if (
        report.get("schema_version") != 1
        or report.get("report_type") != "gpui-mesh-plot-cross-adapter-visual-diff"
        or report.get("artifact_kind") != "png"
        or report.get("passed") is not True
        or not isinstance(left_renderer, str)
        or not left_renderer.strip()
        or not isinstance(right_renderer, str)
        or not right_renderer.strip()
        or left_renderer == right_renderer
        or report.get("compared_count") != len(expected_ids)
        or report.get("failed_count") != 0
        or not isinstance(cases, list)
        or len(cases) != len(expected_ids)
    ):
        raise EvidenceError(
            "MeshPlot cross-adapter visual report must contain six passed cases "
            "from distinct renderer adapters"
        )

    max_channel_delta = report.get("max_channel_delta")
    max_changed_fraction = report.get("max_changed_fraction")
    if (
        not isinstance(max_channel_delta, int)
        or isinstance(max_channel_delta, bool)
        or max_channel_delta < 0
        or not isinstance(max_changed_fraction, (int, float))
        or isinstance(max_changed_fraction, bool)
        or not math.isfinite(float(max_changed_fraction))
        or not 0.0 <= float(max_changed_fraction) <= 1.0
    ):
        raise EvidenceError("MeshPlot cross-adapter visual report has invalid thresholds")

    seen: set[str] = set()
    for case in cases:
        if not isinstance(case, dict):
            raise EvidenceError("MeshPlot cross-adapter visual report has a malformed case")
        case_id = case.get("id")
        if not isinstance(case_id, str) or case_id not in expected_ids or case_id in seen:
            raise EvidenceError("MeshPlot cross-adapter visual report has invalid case IDs")
        seen.add(case_id)
        if case.get("status") != "Passed":
            raise EvidenceError(f"cross-adapter visual case {case_id} did not pass")
        if case.get("artifact_kind") != "png":
            raise EvidenceError(
                f"cross-adapter visual case {case_id} must declare a PNG artifact"
            )
        changed_pixels = case.get("changed_pixels")
        changed_fraction = case.get("changed_fraction")
        if (
            not isinstance(changed_pixels, int)
            or isinstance(changed_pixels, bool)
            or changed_pixels < 0
            or not isinstance(changed_fraction, (int, float))
            or isinstance(changed_fraction, bool)
            or not math.isfinite(float(changed_fraction))
            or not 0.0 <= float(changed_fraction) <= 1.0
        ):
            raise EvidenceError(f"cross-adapter visual case {case_id} has invalid metrics")
        for key in ("left_path", "right_path"):
            path_text = case.get(key)
            if not isinstance(path_text, str):
                raise EvidenceError(f"cross-adapter visual case {case_id} has no {key}")
            if Path(path_text).suffix.lower() != ".png":
                raise EvidenceError(
                    f"cross-adapter visual case {case_id} {key} must reference a PNG"
                )
            image = _safe_repo_artifact_path(root, path_text, f"cross-adapter case {case_id}")
            if not image.is_file() or image.stat().st_size == 0:
                raise EvidenceError(f"missing cross-adapter visual image: {image}")
    if seen != expected_ids:
        raise EvidenceError("cross-adapter visual report case IDs are incomplete")


def validate_mesh_plot_expanded_visual(root: Path, *, require_report: bool) -> None:
    """Validate the expanded camera/range/mask cross-adapter report."""

    path = root / MESH_PLOT_EXPANDED_CROSS_ADAPTER_VISUAL_ARTIFACT
    if not path.is_file():
        if require_report:
            raise EvidenceError(
                "missing expanded MeshPlot cross-adapter visual report; run the "
                "camera/range/mask paired capture lane on the reference host"
            )
        return

    try:
        report = validate_expanded_report(path)
    except ExpandedVisualError as error:
        raise EvidenceError(str(error)) from error

    for case in report["cases"]:
        assert isinstance(case, dict)
        case_id = str(case["id"])
        for key in ("left_path", "right_path"):
            image = _safe_repo_artifact_path(
                root,
                str(case[key]),
                f"expanded cross-adapter case {case_id}",
            )
            if not image.is_file() or image.stat().st_size == 0:
                raise EvidenceError(f"missing expanded cross-adapter visual image: {image}")


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
    validate_mesh_plot_visual_capture(
        root,
        require_clean=require_clean,
        source_revision=revision,
    )
    baseline_ids = validate_mesh_plot_visual_baseline(root)
    validate_mesh_plot_visual_diff(
        root,
        baseline_ids,
        strict=require_clean,
        source_revision=revision,
    )
    validate_mesh_compute_evidence(
        root,
        require_capture=require_clean,
        source_revision=revision,
    )
    validate_mesh_lod_evidence(
        root,
        require_capture=require_clean,
        source_revision=revision,
    )
    validate_mesh_metal_memory_evidence(
        root,
        require_capture=require_clean,
        source_revision=revision,
    )
    validate_mesh_plot_wgpu_visual(root, require_baseline=require_wgpu_visual)
    validate_mesh_plot_metal_visual(root, require_capture=require_clean)
    validate_mesh_plot_product_visual(
        root,
        require_capture=require_clean,
        source_revision=revision,
    )
    validate_mesh_plot_cvd(
        root,
        require_capture=require_clean,
        source_revision=revision,
    )
    validate_mesh_plot_cross_adapter_visual(root, require_report=require_clean)
    validate_mesh_plot_expanded_visual(root, require_report=require_clean)
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
