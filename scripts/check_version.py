"""Check the shared product version and optional GitHub release tag (Python 3.11+)."""

import json
import os
from pathlib import Path
import re
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]


def main():
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        package = tomllib.load(manifest)["package"]
    version = package["version"]
    # Product policy: x.y.z, optionally followed by SemVer prerelease identifiers.
    number = r"(?:0|[1-9][0-9]*)"
    identifier = rf"(?:{number}|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    pattern = rf"{number}\.{number}\.{number}(?:-{identifier}(?:\.{identifier})*)?"
    if not re.fullmatch(pattern, version, flags=re.ASCII):
        raise ValueError(f"Unsupported product version: {version}")

    with (ROOT / "Cargo.lock").open("rb") as lockfile:
        cargo_lock = tomllib.load(lockfile)
    own_packages = [p for p in cargo_lock["package"] if p["name"] == package["name"]]
    if len(own_packages) != 1:
        raise ValueError("Expected exactly one product entry in Cargo.lock")

    web = json.loads((ROOT / "web/package.json").read_text(encoding="utf-8"))
    web_lock = json.loads((ROOT / "web/package-lock.json").read_text(encoding="utf-8"))
    versions = {
        "Cargo.lock": own_packages[0]["version"],
        "web/package.json": web["version"],
        "web/package-lock.json": web_lock["version"],
        "web/package-lock.json packages[root]": web_lock["packages"][""]["version"],
    }
    ref = os.environ.get("GITHUB_REF", "")
    if ref.startswith("refs/tags/"):
        tag = ref.removeprefix("refs/tags/")
        if tag != f"v{version}":
            raise ValueError(f"Release tag {tag!r} must be v{version}")
    mismatches = [f"{path}: {actual}" for path, actual in versions.items() if actual != version]
    if mismatches:
        raise ValueError(f"Expected {version}; mismatches: " + "; ".join(mismatches))
    print(f"Product version OK: {version}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(f"Version check failed: {error}", file=sys.stderr)
        sys.exit(1)
