#!/usr/bin/env python3
"""Fail closed on release identity, immutable versions and registry checksums."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import time
import tomllib
from urllib.error import HTTPError
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parent.parent
TAG = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")


def tag_version(tag):
    if not TAG.fullmatch(tag):
        raise ValueError("Release tag must be vX.Y.Z with canonical decimal components")
    return tag[1:]


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def resolve(tag, expected_commit=None):
    version = tag_version(tag)
    git("show-ref", "--verify", f"refs/tags/{tag}")
    commit = git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}")
    if expected_commit is not None and commit != expected_commit:
        raise ValueError("The release tag no longer identifies the checked commit")
    manifest = tomllib.loads(git("show", f"{commit}:Cargo.toml"))
    if manifest["package"]["name"] != "z58":
        raise ValueError("The release package name must be z58")
    if manifest["package"]["version"] != version:
        raise ValueError("Release tag does not match Cargo.toml version")
    return version, commit


def registry_version(version):
    request = Request("https://index.crates.io/3/z/z58", headers={
        "User-Agent": "z58-release (https://github.com/LIghtJUNction/z58)",
        "Cache-Control": "no-cache",
    })
    try:
        with urlopen(request, timeout=20) as response:
            entries = [json.loads(line) for line in response.read().splitlines() if line]
    except HTTPError as error:
        if error.code == 404:
            return None
        raise
    return next((entry for entry in entries if entry["vers"] == version), None)


def archive_identity(version, expected_commit):
    path = ROOT / "target/package" / f"z58-{version}.crate"
    if git("status", "--porcelain"):
        raise ValueError("Release checkout must be clean")
    if git("rev-parse", "HEAD") != expected_commit:
        raise ValueError("Release checkout differs from the validated commit")
    with tarfile.open(path) as archive:
        entry = archive.extractfile(f"z58-{version}/.cargo_vcs_info.json")
        if entry is None:
            raise ValueError("Package has no VCS provenance")
        vcs = json.load(entry)
    if vcs["git"]["sha1"] != expected_commit or vcs["git"].get("dirty", False):
        raise ValueError("Package VCS provenance differs from the checked commit")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def assert_published(version, checksum, wait_seconds):
    deadline = time.monotonic() + wait_seconds
    while True:
        entry = registry_version(version)
        if entry is not None:
            if entry["cksum"] != checksum or entry.get("yanked", False):
                raise ValueError("Registry version is yanked or has a different package checksum")
            print(f"crates.io confirmed z58 {version}, sha256 {checksum}")
            return
        if time.monotonic() >= deadline:
            raise ValueError("Published version did not appear in the crates.io index in time")
        time.sleep(5)


def output(**values):
    for key, value in values.items():
        print(f"{key}={value}")
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as handle:
            for key, value in values.items():
                handle.write(f"{key}={value}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("resolve", "unpublished", "archive", "published"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--commit")
    parser.add_argument("--checksum")
    parser.add_argument("--wait", type=int, default=0)
    args = parser.parse_args()
    version = tag_version(args.tag)
    if args.operation == "resolve":
        version, commit = resolve(args.tag, args.commit)
        output(tag=args.tag, version=version, commit=commit)
    elif args.operation == "unpublished":
        if registry_version(version) is not None:
            raise ValueError(f"z58 {version} is already published; versions are immutable")
        print(f"z58 {version} has not been published")
    elif args.operation == "archive":
        if not args.commit:
            parser.error("archive requires --commit")
        resolve(args.tag, args.commit)
        output(checksum=archive_identity(version, args.commit))
    else:
        if not args.checksum or not re.fullmatch(r"[0-9a-f]{64}", args.checksum):
            parser.error("published requires a SHA-256 --checksum")
        assert_published(version, args.checksum, args.wait)


if __name__ == "__main__":
    main()
