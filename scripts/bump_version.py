#!/usr/bin/env python3
"""Safely select a first release or bump the dependency-free crate version.

Only Cargo.toml's package version and Cargo.lock's matching root entry change.
This is a development/release tool and is excluded from the published crate.
"""

import argparse
from pathlib import Path
import re
import tomllib


VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")
ROOT = Path(__file__).resolve().parent.parent


def rewrite(manifest_text, lock_text, mode):
    if mode not in ("current", "patch", "minor", "major"):
        raise ValueError(f"Unknown release mode: {mode}")
    manifest = tomllib.loads(manifest_text)
    lock = tomllib.loads(lock_text)
    package = manifest["package"]
    if package.get("name") != "z58":
        raise ValueError("Expected the z58 package")
    version = package["version"]
    match = VERSION.fullmatch(version)
    if match is None:
        raise ValueError("Current version must be canonical X.Y.Z")
    roots = [item for item in lock["package"] if item["name"] == "z58"]
    if len(roots) != 1 or roots[0]["version"] != version:
        raise ValueError("Cargo.lock does not match the z58 package version")

    major, minor, patch = map(int, match.groups())
    if mode == "patch":
        patch += 1
    elif mode == "minor":
        minor += 1
        patch = 0
    elif mode == "major":
        major += 1
        minor = patch = 0
    chosen = f"{major}.{minor}.{patch}"
    if chosen == version:
        return chosen, manifest_text, lock_text

    # Restrict replacements to the verified package's existing version entry.
    before = f'version = "{version}"'
    after = f'version = "{chosen}"'
    if manifest_text.count(before) != 1 or lock_text.count(before) != 1:
        raise ValueError("Expected exactly one package-version entry in each file")
    new_manifest = manifest_text.replace(before, after)
    new_lock = lock_text.replace(before, after)
    if tomllib.loads(new_manifest)["package"]["version"] != chosen:
        raise ValueError("Cargo.toml was not updated as expected")
    if [p for p in tomllib.loads(new_lock)["package"] if p["name"] == "z58"][0]["version"] != chosen:
        raise ValueError("Cargo.lock was not updated as expected")
    return chosen, new_manifest, new_lock


def rewrite_comparison_lock(lock_text, chosen):
    lock = tomllib.loads(lock_text)
    roots = [item for item in lock["package"] if item["name"] == "z58"]
    if len(roots) != 1 or "source" in roots[0] or not VERSION.fullmatch(roots[0]["version"]):
        raise ValueError("comparison/Cargo.lock must contain exactly one valid local z58 package")
    current = roots[0]["version"]
    before = f'name = "z58"\nversion = "{current}"'
    after = f'name = "z58"\nversion = "{chosen}"'
    if lock_text.count(before) != 1:
        raise ValueError("Expected exactly one z58 path-package entry in comparison/Cargo.lock")
    updated = lock_text.replace(before, after)
    if [p for p in tomllib.loads(updated)["package"] if p["name"] == "z58"][0]["version"] != chosen:
        raise ValueError("comparison/Cargo.lock was not updated as expected")
    return updated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("current", "patch", "minor", "major"), required=True)
    args = parser.parse_args()
    manifest_path = ROOT / "Cargo.toml"
    lock_path = ROOT / "Cargo.lock"
    old_version = tomllib.loads(manifest_path.read_text())["package"]["version"]
    version, manifest, lock = rewrite(manifest_path.read_text(), lock_path.read_text(), args.mode)
    if args.mode != "current":
        comparison_path = ROOT / "comparison" / "Cargo.lock"
        comparison = rewrite_comparison_lock(comparison_path.read_text(), version) if comparison_path.exists() else None
        manifest_path.write_text(manifest)
        lock_path.write_text(lock)
        if comparison is not None:
            comparison_path.write_text(comparison)
    print(f"v{version}")


if __name__ == "__main__":
    main()
