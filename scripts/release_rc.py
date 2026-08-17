#!/usr/bin/env python3
"""Build an offline, reproducible GPUI Toolkit release-candidate bundle."""

from __future__ import annotations

import argparse
import datetime as dt
import gzip
import hashlib
import json
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-toolkit-release-candidate"
WAVE_ONE_PACKAGES = ("gpui-design", "gpui-profiler", "gpui-ui-kit-macros")
SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")


class ReleaseError(RuntimeError):
    pass


def resolve_command(args: tuple[str, ...]) -> list[str]:
    """Resolve Rust tools when a restricted shell omits Cargo's bin directory."""

    if not args:
        return []
    executable = args[0]
    if os.path.isabs(executable) or shutil.which(executable):
        return list(args)
    if executable in {"cargo", "rustc", "rustup", "just"}:
        fallback = Path.home() / ".cargo" / "bin" / executable
        if fallback.is_file() or fallback.is_symlink():
            return [str(fallback), *args[1:]]
    return list(args)


def run(root: Path, *args: str, text: bool = True) -> str | bytes:
    completed = subprocess.run(
        resolve_command(args),
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=text,
    )
    return completed.stdout.strip() if text else completed.stdout


def workspace_version(root: Path) -> str:
    cargo_toml = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r"(?ms)^\[workspace\.package\].*?^version\s*=\s*\"([^\"]+)\"", cargo_toml)
    if not match:
        raise ReleaseError("Cargo.toml has no [workspace.package] version")
    return match.group(1)


def validate_version(root: Path, version: str) -> None:
    if not SEMVER.fullmatch(version):
        raise ReleaseError(f"invalid semantic version: {version!r}")
    declared = workspace_version(root)
    if version != declared:
        raise ReleaseError(f"requested version {version} does not match workspace version {declared}")


def ensure_clean(root: Path) -> None:
    status = run(root, "git", "status", "--porcelain=v1", "--untracked-files=all")
    if status:
        raise ReleaseError("release candidates require a clean worktree")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def git_archive(root: Path, destination: Path, prefix: str, paths: tuple[str, ...] = ()) -> None:
    command = ["git", "archive", "--format=tar", f"--prefix={prefix}", "HEAD", *paths]
    tar_bytes = run(root, *command, text=False)
    with destination.open("wb") as output:
        with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=0) as compressed:
            compressed.write(tar_bytes)


def cargo_metadata(root: Path) -> dict[str, Any]:
    raw = run(root, "cargo", "metadata", "--locked", "--offline", "--format-version", "1")
    return json.loads(raw)


def package_source(package: dict[str, Any]) -> str:
    source = package.get("source")
    if source:
        return source
    return "workspace"


def download_location(source: str) -> str:
    if source == "workspace":
        return "NOASSERTION"
    for prefix in ("registry+", "git+"):
        if source.startswith(prefix):
            return source.removeprefix(prefix)
    return source


def license_inventory(metadata: dict[str, Any]) -> list[dict[str, str]]:
    rows = []
    for package in metadata["packages"]:
        rows.append(
            {
                "license": package.get("license") or "NOASSERTION",
                "name": package["name"],
                "source": package_source(package),
                "version": package["version"],
            }
        )
    return sorted(rows, key=lambda row: (row["name"], row["version"], row["source"]))


def spdx_document(metadata: dict[str, Any], version: str, commit: str, created: str) -> dict[str, Any]:
    packages = []
    relationships = []
    for index, package in enumerate(license_inventory(metadata), start=1):
        spdx_id = f"SPDXRef-Package-{index}"
        packages.append(
            {
                "SPDXID": spdx_id,
                "downloadLocation": download_location(package["source"]),
                "filesAnalyzed": False,
                "licenseConcluded": "NOASSERTION",
                "licenseDeclared": package["license"],
                "name": package["name"],
                "versionInfo": package["version"],
            }
        )
        relationships.append(
            {
                "spdxElementId": "SPDXRef-DOCUMENT",
                "relationshipType": "DESCRIBES",
                "relatedSpdxElement": spdx_id,
            }
        )
    return {
        "SPDXID": "SPDXRef-DOCUMENT",
        "creationInfo": {"created": created, "creators": ["Tool: gpui-toolkit-release-rc/1"]},
        "dataLicense": "CC0-1.0",
        "documentNamespace": f"https://github.com/pierreaubert/gpui-toolkit/releases/{version}/spdx/{commit}",
        "name": f"gpui-toolkit-{version}",
        "packages": packages,
        "relationships": relationships,
        "spdxVersion": "SPDX-2.3",
    }


def render_licenses(rows: list[dict[str, str]]) -> str:
    lines = [
        "# GPUI Toolkit license inventory",
        "",
        "| Package | Version | License | Source |",
        "| --- | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(f"| {row['name']} | {row['version']} | {row['license']} | {row['source']} |")
    return "\n".join(lines) + "\n"


def package_wave_one(root: Path, staging: Path, metadata: dict[str, Any]) -> list[str]:
    versions = {
        package["name"]: package["version"]
        for package in metadata["packages"]
        if package["name"] in WAVE_ONE_PACKAGES and package_source(package) == "workspace"
    }
    names = []
    for package in WAVE_ONE_PACKAGES:
        if package not in versions:
            raise ReleaseError(f"cargo metadata does not contain workspace package {package}")
        subprocess.run(
            resolve_command(("cargo", "package", "--locked", "--offline", "-p", package)),
            cwd=root,
            check=True,
        )
        source = root / "target" / "package" / f"{package}-{versions[package]}.crate"
        if not source.is_file():
            raise ReleaseError(f"cargo package did not produce {source.name}")
        destination = staging / source.name
        shutil.copyfile(source, destination)
        names.append(destination.name)
    return names


def build(root: Path, version: str, output: Path) -> Path:
    validate_version(root, version)
    ensure_clean(root)
    if output.exists():
        raise ReleaseError(f"output already exists: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)

    commit = str(run(root, "git", "rev-parse", "HEAD"))
    epoch = int(str(run(root, "git", "show", "-s", "--format=%ct", "HEAD")))
    created = dt.datetime.fromtimestamp(epoch, tz=dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    metadata = cargo_metadata(root)

    with tempfile.TemporaryDirectory(prefix="rc-staging-", dir=output.parent) as temp:
        staging = Path(temp) / output.name
        staging.mkdir()
        source_name = f"gpui-toolkit-{version}-source.tar.gz"
        gallery_name = f"gpui-toolkit-{version}-visual-gallery.tar.gz"
        git_archive(root, staging / source_name, f"gpui-toolkit-{version}/")
        git_archive(
            root,
            staging / gallery_name,
            f"gpui-toolkit-{version}/",
            ("assets/component-lab-gallery",),
        )
        package_names = package_wave_one(root, staging, metadata)

        licenses = license_inventory(metadata)
        write_json(staging / "licenses.json", {"licenses": licenses, "schema_version": 1})
        (staging / "licenses.md").write_text(render_licenses(licenses), encoding="utf-8")
        write_json(staging / "sbom.spdx.json", spdx_document(metadata, version, commit, created))

        cargo_lock_sha = sha256(root / "Cargo.lock")
        provenance = {
            "schema_version": SCHEMA_VERSION,
            "report_type": REPORT_TYPE,
            "version": version,
            "git_commit": commit,
            "source_date_epoch": epoch,
            "created": created,
            "dirty": False,
            "network_used": False,
            "published": False,
            "wave_one_packages": list(WAVE_ONE_PACKAGES),
            "artifacts": [source_name, gallery_name, *package_names, "licenses.json", "licenses.md", "sbom.spdx.json"],
            "materials": {"Cargo.lock.sha256": cargo_lock_sha},
            "tools": {
                "cargo": str(run(root, "cargo", "--version")),
                "rustc": str(run(root, "rustc", "--version")),
            },
        }
        write_json(staging / "provenance.json", provenance)

        artifacts = sorted(path for path in staging.iterdir() if path.name != "SHA256SUMS")
        checksums = "".join(f"{sha256(path)}  {path.name}\n" for path in artifacts)
        (staging / "SHA256SUMS").write_text(checksums, encoding="utf-8")
        staging.replace(output)
    return output


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    parser.add_argument("--output-dir", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    output = args.output_dir or root / "target" / "release" / f"gpui-toolkit-{args.version}-rc"
    try:
        result = build(root, args.version, output)
    except (ReleaseError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"release RC failed: {error}") from error
    print(f"Release candidate artifacts: {result}")


if __name__ == "__main__":
    main()
