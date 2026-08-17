from __future__ import annotations

import json
import struct
import subprocess
import tempfile
import unittest
import zlib
from contextlib import nullcontext
from pathlib import Path
from unittest import mock

from qa_release_evidence import (
    EvidenceError,
    MESH_PLOT_BENCHMARK_WORKLOADS,
    MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT,
    MESH_PLOT_EXPANDED_CROSS_ADAPTER_VISUAL_ARTIFACT,
    MESH_PLOT_METAL_VISUAL_CAPTURE_ARTIFACT,
    MESH_PLOT_LOCAL_CAPTURE_COUNT,
    MESH_PLOT_PRODUCT_VISUAL_ARTIFACT,
    MESH_PLOT_CVD_ARTIFACT,
    MESH_COMPUTE_EVIDENCE_ARTIFACT,
    MESH_LOD_EVIDENCE_ARTIFACT,
    MESH_METAL_MEMORY_EVIDENCE_ARTIFACT,
    MESH_PLOT_VERSIONED_BASELINE_COUNT,
    MESH_PLOT_VISUAL_CAPTURE_ARTIFACT,
    MESH_PLOT_VISUAL_DIFF_ARTIFACT,
    MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT,
    MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT,
    PLATFORM_EVIDENCE,
    REQUIRED_ARTIFACTS,
    build_manifest,
    resolve_command,
    render_markdown,
    validate_mesh_plot_benchmarks,
    validate_mesh_plot_cross_adapter_visual,
    validate_mesh_plot_expanded_visual,
    validate_mesh_plot_visual_capture,
    validate_mesh_plot_visual_diff,
)


PERF_ENVIRONMENT = {
    "system": "TestOS",
    "machine": "arm64",
    "cpu_model": "fixture",
    "source_dirty": False,
    "rustc": {"release": "1.90.0", "host": "arm64-test"},
}


def perf_fixture() -> dict[str, object]:
    records: list[dict[str, object]] = [
        {"crate": crate, "bench": bench}
        for crate, bench in (
            ("gpui-d3rs", "mesh_prep"),
            ("gpui-px", "mesh_plot_frames"),
        )
    ]
    for (crate, bench), workloads in MESH_PLOT_BENCHMARK_WORKLOADS.items():
        records.extend(
            {
                "crate": crate,
                "bench": bench,
                "group": group,
                "function": function,
                "median_ns": 1.0,
                "mean_ns": 1.0,
                "unit": "ns",
            }
            for group, function in workloads
        )
    return {
        "version": 2,
        "metadata": {"environment": PERF_ENVIRONMENT},
        "records": records,
    }


MESH_PLOT_BASELINE_IDS = tuple(
    f"px-mesh-plot-fixture-{index:02d}"
    for index in range(MESH_PLOT_VERSIONED_BASELINE_COUNT)
)
WGPU_CASE_IDS = ("mesh", "smooth", "cell", "wireframe", "isoline", "revolve")
WGPU_COMPARISON_IDS = {
    "mesh": "px.mesh_plot.mesh_only",
    "smooth": "px.mesh_plot.smooth_fill",
    "cell": "px.mesh_plot.flat_fill",
    "wireframe": "px.mesh_plot.wireframe",
    "isoline": "px.mesh_plot.isolines",
    "revolve": "px.mesh_plot.revolve",
}


def mesh_plot_capture_fixture(root: Path) -> dict[str, object]:
    cases = []
    for index in range(MESH_PLOT_LOCAL_CAPTURE_COUNT):
        capture_id = f"px-mesh-plot__local-{index:03d}"
        actual_path = Path(
            "target/qa/visual/component-lab/metal/actual"
        ) / f"{capture_id}.png"
        path = root / actual_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"fixture image")
        cases.append(
            {
                "capture_id": capture_id,
                "story_id": "px.mesh_plot.fixture",
                "renderer_id": "metal",
                "actual_path": actual_path.as_posix(),
                "status": "Captured",
            }
        )
    return {
        "schema_version": 1,
        "report_type": "gpui-component-lab-render-capture",
        "renderer_id": "metal",
        "passed": True,
        "source_revision": "f" * 40,
        "source_dirty": False,
        "requested_count": MESH_PLOT_LOCAL_CAPTURE_COUNT,
        "captured_count": MESH_PLOT_LOCAL_CAPTURE_COUNT,
        "failed_count": 0,
        "cases": cases,
    }


def mesh_plot_diff_fixture(root: Path) -> dict[str, object]:
    cases = []
    for capture_id in MESH_PLOT_BASELINE_IDS:
        paths = {
            "baseline_path": Path(
                "target/qa/visual/component-lab/metal/baseline"
            ) / f"{capture_id}.png",
            "actual_path": Path(
                "target/qa/visual/component-lab/metal/actual"
            ) / f"{capture_id}.png",
            "diff_path": Path("target/qa/visual/component-lab/metal/diff")
            / f"{capture_id}.png",
        }
        for relative in paths.values():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"fixture diff image")
        cases.append(
            {
                "capture_id": capture_id,
                "status": "Passed",
                "changed_pixels": 0,
                "max_channel_delta": 0,
                **{key: value.as_posix() for key, value in paths.items()},
            }
        )
    return {
        "schema_version": 1,
        "report_type": "gpui-component-lab-visual-diff",
        "passed": True,
        "source_revision": "f" * 40,
        "source_dirty": False,
        "compared_count": MESH_PLOT_VERSIONED_BASELINE_COUNT,
        "failed_count": 0,
        "max_changed_pixels": 0,
        "cases": cases,
    }


def mesh_plot_wgpu_fixture(root: Path) -> None:
    actual_dir = root / "target/qa/visual/mesh-plot-wgpu/actual"
    actual_dir.mkdir(parents=True, exist_ok=True)
    cases = []
    for index, case_id in enumerate(WGPU_CASE_IDS):
        (actual_dir / f"{case_id}.png").write_bytes(f"fixture-{case_id}".encode())
        cases.append(
            {
                "id": case_id,
                "comparison_id": WGPU_COMPARISON_IDS[case_id],
                "description": f"fixture {case_id}",
                "path": f"{case_id}.png",
                "opaque_pixels": index + 1,
                "rgba_checksum": f"fnv1a64:{index + 1:016x}",
            }
        )
    manifest = {
        "schema_version": 1,
        "renderer": "wgpu-headless",
        "status": "captured",
        "width": 256,
        "height": 192,
        "cases": cases,
    }
    actual_path = root / MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT
    actual_path.write_text(json.dumps(manifest), encoding="utf-8")
    baseline_path = root / MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT
    baseline_path.parent.mkdir(parents=True, exist_ok=True)
    baseline_path.write_text(json.dumps(manifest), encoding="utf-8")

    metal_manifest = {**manifest, "renderer": "metal-headless"}
    metal_path = root / MESH_PLOT_METAL_VISUAL_CAPTURE_ARTIFACT
    metal_path.parent.mkdir(parents=True, exist_ok=True)
    for case_id in WGPU_CASE_IDS:
        (metal_path.parent / f"{case_id}.png").write_bytes(f"metal-{case_id}".encode())
    metal_path.write_text(json.dumps(metal_manifest), encoding="utf-8")


def mesh_plot_cross_adapter_fixture(root: Path) -> None:
    image_dir = root / "target/qa/visual/mesh-plot-cross-adapter"
    image_dir.mkdir(parents=True, exist_ok=True)
    cases = []
    for comparison_id in sorted(WGPU_COMPARISON_IDS.values()):
        left_path = image_dir / f"{comparison_id.replace('.', '-')}-metal.png"
        right_path = image_dir / f"{comparison_id.replace('.', '-')}-wgpu.png"
        left_path.write_bytes(b"metal fixture image")
        right_path.write_bytes(b"wgpu fixture image")
        cases.append(
            {
                "id": comparison_id,
                "artifact_kind": "png",
                "left_path": left_path.relative_to(root).as_posix(),
                "right_path": right_path.relative_to(root).as_posix(),
                "status": "Passed",
                "width": 256,
                "height": 192,
                "changed_pixels": 0,
                "changed_fraction": 0.0,
                "max_channel_delta": 0,
                "mean_channel_delta": 0.0,
            }
        )
    report = {
        "schema_version": 1,
        "report_type": "gpui-mesh-plot-cross-adapter-visual-diff",
        "artifact_kind": "png",
        "passed": True,
        "left_renderer": "metal-reference",
        "right_renderer": "wgpu-headless",
        "max_channel_delta": 0,
        "max_changed_fraction": 0.0,
        "compared_count": len(cases),
        "failed_count": 0,
        "cases": cases,
    }
    report_path = root / MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report), encoding="utf-8")


def mesh_plot_expanded_fixture(root: Path) -> None:
    image_dir = root / "target/qa/visual/mesh-plot-expanded"
    image_dir.mkdir(parents=True, exist_ok=True)
    cases = []
    case_ids = (
        "px.mesh_plot.state.camera",
        "px.mesh_plot.state.range",
        "px.mesh_plot.state.masked",
    )
    for index, case_id in enumerate(case_ids):
        left_path = image_dir / f"{index}-metal.png"
        right_path = image_dir / f"{index}-wgpu.png"
        left_path.write_bytes(b"metal expanded fixture image")
        right_path.write_bytes(b"wgpu expanded fixture image")
        cases.append(
            {
                "id": case_id,
                "left_path": left_path.relative_to(root).as_posix(),
                "right_path": right_path.relative_to(root).as_posix(),
                "status": "Passed",
                "changed_pixels": 0,
                "changed_fraction": 0.0,
                "max_channel_delta": 0,
                "mean_channel_delta": 0.0,
            }
        )
    report = {
        "schema_version": 1,
        "report_type": "gpui-mesh-plot-cross-adapter-visual-diff",
        "passed": True,
        "left_renderer": "metal-headless",
        "right_renderer": "wgpu-headless",
        "max_channel_delta": 0,
        "max_changed_fraction": 0.0,
        "compared_count": 3,
        "failed_count": 0,
        "cases": cases,
    }
    report_path = root / MESH_PLOT_EXPANDED_CROSS_ADAPTER_VISUAL_ARTIFACT
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report), encoding="utf-8")


def product_png(width: int, height: int, pixel: bytes) -> bytes:
    rows = b"".join(b"\x00" + pixel * width for _ in range(height))

    def chunk(kind: bytes, data: bytes) -> bytes:
        payload = kind + data
        return (
            struct.pack(">I", len(data))
            + payload
            + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(rows))
        + chunk(b"IEND", b"")
    )


def mesh_plot_product_fixture(root: Path) -> None:
    output_dir = root / "target/qa/visual/mesh-plot-product"
    (output_dir / "metal").mkdir(parents=True, exist_ok=True)
    (output_dir / "wgpu").mkdir(parents=True, exist_ok=True)
    plain = product_png(1200, 800, bytes((10, 20, 30, 255)))
    selected = product_png(1200, 800, bytes((20, 30, 40, 255)))
    for renderer in ("metal", "wgpu"):
        (output_dir / renderer / "plain.png").write_bytes(plain)
        (output_dir / renderer / "selected.png").write_bytes(selected)
    cases = []
    for renderer in ("metal", "wgpu"):
        cases.extend(
            [
                {
                    "id": f"{renderer}-plain",
                    "renderer": f"{renderer}-headless",
                    "state": "plain",
                    "comparison_id": "px.mesh_plot.product.axes",
                    "artifact_kind": "png",
                    "path": f"{renderer}/plain.png",
                    "axes_present": True,
                    "selection_annotation": False,
                },
                {
                    "id": f"{renderer}-selected",
                    "renderer": f"{renderer}-headless",
                    "state": "selected",
                    "comparison_id": "px.mesh_plot.product.selection",
                    "artifact_kind": "png",
                    "path": f"{renderer}/selected.png",
                    "axes_present": True,
                    "selection_annotation": True,
                    "changed_pixels_from_plain": 1200 * 800,
                },
            ]
        )
    manifest = {
        "schema_version": 1,
        "report_type": "gpui-mesh-plot-product-capture",
        "status": "captured",
        "logical_width": 600,
        "logical_height": 400,
        "width": 1200,
        "height": 800,
        "composition": {
            "axes_present": True,
            "axis_titles": ["x", "y"],
            "colorbar_present": True,
            "selection_annotation_contract": True,
        },
        "paired_comparison": {
            "axes": {
                "comparison_id": "px.mesh_plot.product.axes",
                "metal_case": "metal-plain",
                "wgpu_case": "wgpu-plain",
                "changed_pixels": 0,
                "changed_fraction": 0.0,
            },
            "selection": {
                "comparison_id": "px.mesh_plot.product.selection",
                "metal_case": "metal-selected",
                "wgpu_case": "wgpu-selected",
                "changed_pixels": 0,
                "changed_fraction": 0.0,
            },
        },
        "source_revision": "f" * 40,
        "source_dirty": False,
        "cases": cases,
    }
    (output_dir / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")


def mesh_plot_cvd_fixture(root: Path) -> None:
    cases = [
        {
            "id": f"{renderer}-{state}",
            "renderer": f"{renderer}-headless",
            "state": state,
            "path": f"{renderer}/{state}.png",
            "width": 1200,
            "height": 800,
        }
        for renderer in ("metal", "wgpu")
        for state in ("plain", "selected")
    ]
    metrics = {
        "cases": {
            case["id"]: {"unique_rgb_colors": 2, "finite": True} for case in cases
        },
        "selection_changed_pixels": {"metal": 1, "wgpu": 1},
    }
    (root / MESH_PLOT_CVD_ARTIFACT).parent.mkdir(parents=True, exist_ok=True)
    (root / MESH_PLOT_CVD_ARTIFACT).write_text(
        json.dumps(
            {
                "schema_version": 1,
                "report_type": "gpui-mesh-plot-cvd-screen",
                "status": "captured",
                "source_revision": "f" * 40,
                "source_dirty": False,
                "manual_review_required": True,
                "cases": cases,
                "deficiencies": {name: metrics for name in ("protan", "deutan", "tritan")},
            }
        ),
        encoding="utf-8",
    )


def mesh_compute_fixture(root: Path) -> None:
    path = root / MESH_COMPUTE_EVIDENCE_ARTIFACT
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "report_type": "gpui-mesh-compute-gpu-evidence",
                "status": "captured",
                "backend": "metal",
                "adapter_backed": True,
                "parity": {
                    "field_min_max": True,
                    "isolines": True,
                    "filled_bands": True,
                },
                "timing": {
                    "requested": True,
                    "enabled": True,
                    "sample_count": 3,
                    "last_gpu_time_ns": 120,
                },
                "source_revision": "f" * 40,
                "source_dirty": False,
            }
        ),
        encoding="utf-8",
    )


def mesh_lod_fixture(root: Path) -> None:
    output_dir = root / "target/qa/perf/mesh-lod"
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "proxy.png").write_bytes(product_png(160, 120, bytes((10, 20, 30, 255))))
    (output_dir / "full.png").write_bytes(product_png(160, 120, bytes((20, 30, 40, 255))))
    (root / MESH_LOD_EVIDENCE_ARTIFACT).write_text(
        json.dumps(
            {
                "schema_version": 1,
                "report_type": "gpui-mesh-lod-evidence",
                "status": "captured",
                "backend": "metal",
                "adapter_backed": True,
                "workload": {
                    "full_triangle_count": 2048,
                    "proxy_triangle_count": 128,
                    "proxy_reduces_triangles": True,
                },
                "visual_quality": {
                    "width": 160,
                    "height": 120,
                    "full_non_black_pixels": 19200,
                    "proxy_non_black_pixels": 19200,
                    "proxy_full_changed_fraction": 1.0,
                    "passed": True,
                    "proxy_path": "proxy.png",
                    "full_path": "full.png",
                },
                "frame_budget": {
                    "sample_count": 60,
                    "target_average_ns": 20_000_000,
                    "total_ns": 600_000_000,
                    "average_ns": 10_000_000,
                    "max_ns": 15_000_000,
                    "passed": True,
                },
                "telemetry": {
                    "proxy_gpu_frame_count": 61,
                    "restored_gpu_frame_count": 62,
                    "restored_geometry_upload_count": 3,
                    "restored_gpu_frame_time_ns": 1000,
                },
                "source_revision": "f" * 40,
                "source_dirty": False,
            }
        ),
        encoding="utf-8",
    )


def mesh_metal_memory_fixture(root: Path) -> None:
    path = root / MESH_METAL_MEMORY_EVIDENCE_ARTIFACT
    path.parent.mkdir(parents=True, exist_ok=True)
    samples = [
        {
            "revision": index + 1,
            "operation": "field" if index % 2 == 0 else "geometry",
            "driver_allocated_bytes": 10_000 + index,
            "peak_driver_allocated_bytes": 10_000 + index,
            "resident_bytes": 5_000,
            "peak_resident_bytes": 5_000,
            "geometry_upload_count": index // 2 + 1,
            "memory_release_count": 0,
        }
        for index in range(20)
    ]
    path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "report_type": "gpui-mesh-metal-memory-evidence",
                "status": "captured",
                "backend": "metal",
                "adapter_backed": True,
                "sample_count": len(samples),
                "samples": samples,
                "before_drop": {
                    "driver_allocated_bytes": 10_020,
                    "peak_driver_allocated_bytes": 10_020,
                    "resident_bytes": 5_000,
                    "peak_resident_bytes": 5_000,
                    "geometry_upload_count": 11,
                },
                "after_drop": {
                    "driver_allocated_bytes": None,
                    "resident_bytes": 0,
                    "memory_release_count": 1,
                    "peak_driver_allocated_bytes": 10_020,
                },
                "contracts": {
                    "alternating_field_and_geometry_churn": True,
                    "driver_peak_is_monotonic": True,
                    "teardown_clears_current_memory": True,
                    "teardown_preserves_peak": True,
                },
                "source_revision": "f" * 40,
                "source_dirty": False,
            }
        ),
        encoding="utf-8",
    )


class ReleaseEvidenceTests(unittest.TestCase):
    def make_repo(self) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for relative in REQUIRED_ARTIFACTS:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            if relative in {"qa/perf/baseline.json", "target/qa/perf/current.json"}:
                path.write_text(json.dumps(perf_fixture()), encoding="utf-8")
            elif relative == MESH_PLOT_VISUAL_CAPTURE_ARTIFACT:
                path.write_text(json.dumps(mesh_plot_capture_fixture(root)), encoding="utf-8")
            elif relative == MESH_PLOT_VISUAL_DIFF_ARTIFACT:
                path.write_text(json.dumps(mesh_plot_diff_fixture(root)), encoding="utf-8")
            elif relative == MESH_PLOT_CVD_ARTIFACT:
                path.write_text(json.dumps({"status": "skipped", "reason": "fixture"}), encoding="utf-8")
            else:
                path.write_text(f"fixture for {relative}\n", encoding="utf-8")
        mesh_plot_wgpu_fixture(root)
        mesh_plot_cross_adapter_fixture(root)
        mesh_plot_expanded_fixture(root)
        mesh_plot_product_fixture(root)
        mesh_plot_cvd_fixture(root)
        mesh_compute_fixture(root)
        mesh_lod_fixture(root)
        mesh_metal_memory_fixture(root)
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=Evidence Test",
                "-c",
                "user.email=evidence@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
            cwd=root,
            check=True,
            env={
                **__import__("os").environ,
                "GIT_AUTHOR_DATE": "2024-01-01T00:00:00Z",
                "GIT_COMMITTER_DATE": "2024-01-01T00:00:00Z",
            },
        )
        return root

    def build(self, root: Path, visual_members: list[str] | None = None, **kwargs):
        source_override = kwargs.pop("source_override", None)
        source_context = (
            mock.patch("qa_release_evidence.source_provenance", return_value=source_override)
            if source_override is not None
            else nullcontext()
        )
        with (
            source_context,
            mock.patch(
                "qa_release_evidence.toolchain_provenance",
                return_value={"cargo": "cargo test", "just": "just test", "python": "3.test", "rustc": "rustc test"},
            ),
            mock.patch(
                "qa_release_evidence.visual_baseline_members",
                return_value=visual_members
                if visual_members is not None
                else [f"metal/baseline/{capture_id}.png" for capture_id in MESH_PLOT_BASELINE_IDS],
            ),
        ):
            return build_manifest(root, **kwargs)

    def test_manifest_binds_required_artifacts_without_private_paths(self):
        root = self.make_repo()
        manifest = self.build(
            root,
            require_clean=True,
            source_override={
                "revision": "f" * 40,
                "dirty": False,
                "commit_timestamp": 0,
            },
        )

        self.assertFalse(manifest["source"]["dirty"])
        self.assertEqual(len(manifest["source"]["revision"]), 40)
        self.assertEqual(len(manifest["artifacts"]), len(REQUIRED_ARTIFACTS) + 8)
        encoded = json.dumps(manifest, sort_keys=True)
        self.assertNotIn(str(root), encoded)
        for row in manifest["artifacts"]:
            self.assertEqual(len(row["sha256"]), 64)
            self.assertGreater(row["size_bytes"], 0)

        markdown = render_markdown(manifest)
        self.assertIn("source_dirty: `false`", markdown)
        self.assertIn("manifest-bound", markdown)

    def test_resolve_command_finds_homebrew_archive_tools(self):
        with mock.patch("qa_release_evidence.shutil.which", return_value=None):
            with mock.patch("qa_release_evidence.Path.is_file", return_value=True):
                self.assertEqual(
                    resolve_command(("zstd", "-d")),
                    ["/opt/homebrew/bin/zstd", "-d"],
                )

    def test_missing_required_artifact_fails(self):
        root = self.make_repo()
        missing = root / REQUIRED_ARTIFACTS[0]
        missing.unlink()
        with self.assertRaisesRegex(EvidenceError, "missing required QA artifacts"):
            self.build(root)

    def test_missing_mesh_plot_benchmark_evidence_fails(self):
        root = self.make_repo()
        baseline = root / "qa/perf/baseline.json"
        data = json.loads(baseline.read_text(encoding="utf-8"))
        data["records"] = []
        baseline.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "MeshPlot benchmark evidence is missing"):
            self.build(root)

    def test_clean_manifest_requires_named_mesh_plot_workloads(self):
        root = self.make_repo()
        baseline = root / "qa/perf/baseline.json"
        data = json.loads(baseline.read_text(encoding="utf-8"))
        data["records"] = [
            record
            for record in data["records"]
            if record.get("function") != "mesh_plot_fit_200000_triangles"
        ]
        baseline.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "required workloads"):
            validate_mesh_plot_benchmarks(root, strict=True)

    def test_strict_manifest_rejects_dirty_performance_evidence(self):
        root = self.make_repo()
        baseline = root / "qa/perf/baseline.json"
        data = json.loads(baseline.read_text(encoding="utf-8"))
        data["metadata"]["environment"]["source_dirty"] = True
        baseline.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "source_dirty: false"):
            validate_mesh_plot_benchmarks(root, strict=True)

    def test_missing_mesh_plot_visual_baseline_fails(self):
        root = self.make_repo()
        with self.assertRaisesRegex(EvidenceError, "MeshPlot visual baseline evidence"):
            self.build(root, visual_members=["metal/baseline/px-line__fixture.png"])

    def test_macos_archive_sidecars_do_not_count_as_baselines(self):
        root = self.make_repo()
        members = [
            f"metal/baseline/._{capture_id}.png"
            for capture_id in MESH_PLOT_BASELINE_IDS
        ] + [
            f"metal/baseline/{capture_id}.png"
            for capture_id in MESH_PLOT_BASELINE_IDS
        ]
        manifest = self.build(root, visual_members=members)
        self.assertEqual(len(manifest["artifacts"]), len(REQUIRED_ARTIFACTS) + 8)

    def test_mesh_plot_local_capture_requires_99_actual_cases(self):
        root = self.make_repo()
        capture = root / MESH_PLOT_VISUAL_CAPTURE_ARTIFACT
        data = json.loads(capture.read_text(encoding="utf-8"))
        data["captured_count"] = MESH_PLOT_LOCAL_CAPTURE_COUNT - 1
        capture.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "99 requested/captured"):
            self.build(root)

    def test_developer_visual_capture_accepts_explicit_adapter_skip(self):
        root = self.make_repo()
        capture = root / MESH_PLOT_VISUAL_CAPTURE_ARTIFACT
        data = json.loads(capture.read_text(encoding="utf-8"))
        data.update(
            {
                "status": "skipped",
                "reason": "no usable Metal adapter",
                "passed": False,
                "requested_count": MESH_PLOT_LOCAL_CAPTURE_COUNT,
                "captured_count": 0,
                "failed_count": 0,
                "cases": [],
            }
        )
        capture.write_text(json.dumps(data), encoding="utf-8")
        validate_mesh_plot_visual_capture(root)
        with self.assertRaisesRegex(EvidenceError, "skipped but is required"):
            validate_mesh_plot_visual_capture(
                root,
                require_clean=True,
                source_revision="f" * 40,
            )

    def test_developer_visual_diff_accepts_explicit_adapter_skip(self):
        root = self.make_repo()
        diff = root / MESH_PLOT_VISUAL_DIFF_ARTIFACT
        data = json.loads(diff.read_text(encoding="utf-8"))
        data.update(
            {
                "status": "skipped",
                "reason": "no usable Metal adapter",
                "passed": False,
                "compared_count": 0,
                "failed_count": 0,
                "max_changed_pixels": 0,
                "cases": [],
            }
        )
        diff.write_text(json.dumps(data), encoding="utf-8")
        validate_mesh_plot_visual_diff(root, set(MESH_PLOT_BASELINE_IDS))
        with self.assertRaisesRegex(EvidenceError, "skipped but is required"):
            validate_mesh_plot_visual_diff(
                root,
                set(MESH_PLOT_BASELINE_IDS),
                strict=True,
                source_revision="f" * 40,
            )

    def test_mesh_plot_diff_must_match_99_baselines(self):
        root = self.make_repo()
        diff = root / MESH_PLOT_VISUAL_DIFF_ARTIFACT
        data = json.loads(diff.read_text(encoding="utf-8"))
        data["compared_count"] = MESH_PLOT_VERSIONED_BASELINE_COUNT - 1
        diff.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "99 compared cases"):
            self.build(root)

    def test_developer_visual_diff_accepts_complete_stale_baselines(self):
        root = self.make_repo()
        diff = root / MESH_PLOT_VISUAL_DIFF_ARTIFACT
        data = json.loads(diff.read_text(encoding="utf-8"))
        data["passed"] = False
        data["failed_count"] = 1
        data["cases"][0].update(
            {
                "status": "Different",
                "changed_pixels": 1,
                "max_channel_delta": 1,
            }
        )
        diff.write_text(json.dumps(data), encoding="utf-8")

        validate_mesh_plot_visual_diff(
            root,
            set(MESH_PLOT_BASELINE_IDS),
        )
        with self.assertRaisesRegex(EvidenceError, "zero failures"):
            validate_mesh_plot_visual_diff(
                root,
                set(MESH_PLOT_BASELINE_IDS),
                strict=True,
                source_revision="f" * 40,
            )

    def test_strict_visual_diff_requires_persisted_artifact_paths(self):
        root = self.make_repo()
        diff = root / MESH_PLOT_VISUAL_DIFF_ARTIFACT
        data = json.loads(diff.read_text(encoding="utf-8"))
        data["cases"][0].pop("diff_path")
        diff.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "must include artifact paths"):
            validate_mesh_plot_visual_diff(
                root,
                set(MESH_PLOT_BASELINE_IDS),
                strict=True,
                source_revision="f" * 40,
            )

    def test_strict_visual_reports_require_matching_source_revision(self):
        root = self.make_repo()
        diff = root / MESH_PLOT_VISUAL_DIFF_ARTIFACT
        data = json.loads(diff.read_text(encoding="utf-8"))
        data["source_revision"] = "0" * 40
        diff.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "another source revision"):
            validate_mesh_plot_visual_diff(
                root,
                set(MESH_PLOT_BASELINE_IDS),
                strict=True,
                source_revision="f" * 40,
            )

        capture = root / MESH_PLOT_VISUAL_CAPTURE_ARTIFACT
        data = json.loads(capture.read_text(encoding="utf-8"))
        data["source_dirty"] = True
        capture.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "clean source provenance"):
            validate_mesh_plot_visual_capture(
                root,
                require_clean=True,
                source_revision="f" * 40,
            )

    def test_wgpu_manifest_requires_the_canonical_six_cases(self):
        root = self.make_repo()
        actual = root / MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT
        data = json.loads(actual.read_text(encoding="utf-8"))
        data["cases"].pop()
        actual.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "exactly 6 cases"):
            self.build(root)

    def test_wgpu_baseline_checksum_must_match_actual(self):
        root = self.make_repo()
        baseline = root / MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT
        data = json.loads(baseline.read_text(encoding="utf-8"))
        data["cases"][0]["rgba_checksum"] = "fnv1a64:ffffffffffffffff"
        baseline.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "WGPU visual mismatch"):
            self.build(root)

    def test_clean_release_requires_wgpu_baseline(self):
        root = self.make_repo()
        (root / MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT).unlink()
        with self.assertRaisesRegex(EvidenceError, "missing WGPU visual baseline"):
            self.build(root, require_wgpu_visual=True)

    def test_clean_release_requires_cross_adapter_report(self):
        root = self.make_repo()
        (root / MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT).unlink()
        with self.assertRaisesRegex(EvidenceError, "missing cross-adapter MeshPlot visual report"):
            validate_mesh_plot_cross_adapter_visual(root, require_report=True)

    def test_cross_adapter_report_requires_distinct_renderers(self):
        root = self.make_repo()
        report_path = root / MESH_PLOT_CROSS_ADAPTER_VISUAL_ARTIFACT
        report = json.loads(report_path.read_text(encoding="utf-8"))
        report["right_renderer"] = report["left_renderer"]
        report_path.write_text(json.dumps(report), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "distinct renderer"):
            validate_mesh_plot_cross_adapter_visual(root, require_report=True)

    def test_cross_adapter_report_path_must_stay_inside_repository(self):
        root = self.make_repo()
        outside = root.parent / "mesh-plot-cross-adapter.json"
        with self.assertRaisesRegex(EvidenceError, "escapes the repository"):
            validate_mesh_plot_cross_adapter_visual(
                root,
                require_report=True,
                report_path=outside,
            )

    def test_clean_release_requires_expanded_cross_adapter_report(self):
        root = self.make_repo()
        (root / MESH_PLOT_EXPANDED_CROSS_ADAPTER_VISUAL_ARTIFACT).unlink()
        with self.assertRaisesRegex(EvidenceError, "missing expanded MeshPlot cross-adapter"):
            validate_mesh_plot_expanded_visual(root, require_report=True)

    def test_product_capture_requires_four_valid_cases(self):
        root = self.make_repo()
        product = root / MESH_PLOT_PRODUCT_VISUAL_ARTIFACT
        data = json.loads(product.read_text(encoding="utf-8"))
        data["cases"].pop()
        product.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "must contain four cases"):
            self.build(root)

    def test_product_capture_requires_paired_axes_and_selection_contract(self):
        root = self.make_repo()
        product = root / MESH_PLOT_PRODUCT_VISUAL_ARTIFACT
        data = json.loads(product.read_text(encoding="utf-8"))
        del data["paired_comparison"]["selection"]
        product.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "paired selection"):
            self.build(root)

    def test_clean_release_rejects_product_source_mismatch(self):
        root = self.make_repo()
        product = root / MESH_PLOT_PRODUCT_VISUAL_ARTIFACT
        data = json.loads(product.read_text(encoding="utf-8"))
        data["source_revision"] = "a" * 40
        product.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "another source revision"):
            self.build(
                root,
                require_clean=True,
                source_override={
                    "revision": "f" * 40,
                    "dirty": False,
                    "commit_timestamp": 0,
                },
            )

    def test_clean_release_requires_compute_gpu_timing(self):
        root = self.make_repo()
        compute = root / MESH_COMPUTE_EVIDENCE_ARTIFACT
        data = json.loads(compute.read_text(encoding="utf-8"))
        data["timing"]["enabled"] = False
        compute.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "requires enabled non-zero GPU timestamps"):
            self.build(
                root,
                require_clean=True,
                source_override={
                    "revision": "f" * 40,
                    "dirty": False,
                    "commit_timestamp": 0,
                },
            )

    def test_clean_release_requires_lod_frame_budget(self):
        root = self.make_repo()
        lod = root / MESH_LOD_EVIDENCE_ARTIFACT
        data = json.loads(lod.read_text(encoding="utf-8"))
        data["frame_budget"]["passed"] = False
        lod.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "frame budget"):
            self.build(
                root,
                require_clean=True,
                source_override={
                    "revision": "f" * 40,
                    "dirty": False,
                    "commit_timestamp": 0,
                },
            )

    def test_clean_release_requires_metal_memory_teardown_contract(self):
        root = self.make_repo()
        memory = root / MESH_METAL_MEMORY_EVIDENCE_ARTIFACT
        data = json.loads(memory.read_text(encoding="utf-8"))
        data["after_drop"]["driver_allocated_bytes"] = 1
        memory.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "teardown release"):
            self.build(
                root,
                require_clean=True,
                source_override={
                    "revision": "f" * 40,
                    "dirty": False,
                    "commit_timestamp": 0,
                },
            )

    def test_developer_skip_is_accepted_but_release_skip_is_rejected(self):
        root = self.make_repo()
        actual = root / MESH_PLOT_WGPU_VISUAL_CAPTURE_ARTIFACT
        actual.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "renderer": "wgpu-headless",
                    "status": "skipped",
                    "reason": "no adapter in test fixture",
                    "cases": [],
                }
            ),
            encoding="utf-8",
        )
        (root / MESH_PLOT_WGPU_VISUAL_BASELINE_ARTIFACT).unlink()
        self.build(root)
        with self.assertRaisesRegex(EvidenceError, "skipped but is required"):
            self.build(root, require_wgpu_visual=True)

    def test_require_clean_rejects_dirty_worktree(self):
        root = self.make_repo()
        (root / "untracked.txt").write_text("dirty\n", encoding="utf-8")
        with self.assertRaisesRegex(EvidenceError, "clean worktree"):
            self.build(root, require_clean=True)

    def test_required_platform_must_match_clean_manifest_source(self):
        root = self.make_repo()
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True
        ).stdout.strip()
        evidence = root / PLATFORM_EVIDENCE["android-emulator"]
        evidence.parent.mkdir(parents=True)
        evidence.write_text(
            json.dumps({"source_revision": revision, "source_dirty": False}), encoding="utf-8"
        )
        manifest = self.build(root, required_platforms=["android-emulator"])
        platform_row = next(
            row for row in manifest["artifacts"] if row["path"] == PLATFORM_EVIDENCE["android-emulator"]
        )
        self.assertTrue(platform_row["embedded_source"]["matches_manifest_source"])

        evidence.write_text(
            json.dumps({"source_revision": "0" * 40, "source_dirty": False}), encoding="utf-8"
        )
        with self.assertRaisesRegex(EvidenceError, "another source revision"):
            self.build(root, required_platforms=["android-emulator"])


if __name__ == "__main__":
    unittest.main()
