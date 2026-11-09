#!/usr/bin/env python3
"""Check every Cargo dependency kind and the actual publish-file boundary."""

import json
from pathlib import Path
import subprocess
import tomllib


def check_manifest(manifest, root):
    for key in ("dependencies", "build-dependencies", "dev-dependencies"):
        if manifest.get(key):
            raise ValueError(f"Published crate must have no {key}")
        for target, tables in manifest.get("target", {}).items():
            if tables.get(key):
                raise ValueError(f"Published crate has {target} {key}")
    if manifest["package"].get("build") not in (None, False):
        raise ValueError("Custom build scripts are not allowed")
    if (root / "build.rs").exists():
        raise ValueError("build.rs is not allowed")
    if manifest["package"].get("rust-version") != "1.70":
        raise ValueError("Update the MSRV matrix together with rust-version")
    if "comparison" not in manifest.get("workspace", {}).get("exclude", []):
        raise ValueError("comparison must be excluded from the published workspace")
    if "#![no_std]" not in (root / "src/lib.rs").read_text():
        raise ValueError("The library must unconditionally declare #![no_std]")


def check_archive_names(names):
    forbidden = ("comparison/", ".github/", "scripts/", "target/")
    required = {"Cargo.toml", "Cargo.lock", "src/lib.rs", "README.md"}
    if not required.issubset(names):
        raise ValueError(f"Package is missing {sorted(required - set(names))}")
    for name in names:
        if name.startswith(forbidden) or name == "build.rs":
            raise ValueError(f"Unexpected published file: {name}")


def main():
    root = Path(__file__).resolve().parent.parent
    check_manifest(tomllib.loads((root / "Cargo.toml").read_text()), root)
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline"],
        cwd=root, text=True,
    ))
    if len(metadata["packages"]) != 1 or len(metadata["workspace_members"]) != 1:
        raise ValueError("The release workspace must contain exactly one package")
    package = metadata["packages"][0]
    if package["dependencies"]:
        raise ValueError("Cargo reports dependencies in the release crate")
    if any("custom-build" in target["kind"] for target in package["targets"]):
        raise ValueError("Cargo reports a build script")
    names = subprocess.check_output(
        ["cargo", "package", "--list", "--locked"], cwd=root, text=True,
    ).splitlines()
    check_archive_names(names)
    print("Zero dependencies; core-only manifest; isolated comparison; package files:")
    print("\n".join(names))


if __name__ == "__main__":
    main()
