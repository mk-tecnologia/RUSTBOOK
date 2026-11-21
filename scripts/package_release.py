"""Build the public manifest for the native Docker release archives."""

import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tomllib


def main():
    directory = Path(sys.argv[1])
    commit = os.environ["GITHUB_SHA"]
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("Invalid build commit")
    with (Path(__file__).resolve().parents[1] / "Cargo.toml").open("rb") as source:
        version = tomllib.load(source)["package"]["version"]
    images = {}
    for arch in ("amd64", "arm64"):
        archive = directory / f"rustbook-linux-{arch}.tar.gz"
        with archive.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        images[arch] = {"asset": archive.name, "sha256": digest,
                        "image": f"rustbook:{commit}"}
    (directory / "manifest.json").write_text(json.dumps({
        "commit": commit, "version": version, "images": images,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
