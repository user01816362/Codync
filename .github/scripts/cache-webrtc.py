"""Cache the pinned WebRTC archive through GitHub's release API before Xcode resolves it."""
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import time
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen


def request(url, accept="application/vnd.github+json"):
    req = Request(url, headers={"Accept": accept, "User-Agent": "Codync-CI"})
    token = os.environ.get("GH_TOKEN")
    if token and urlsplit(url).hostname == "api.github.com":
        # urllib does not forward unredirected headers to the release-asset CDN.
        req.add_unredirected_header("Authorization", f"Bearer {token}")
    return urlopen(req, timeout=120)


def metadata(path):
    with request(f"https://api.github.com/repos/stasel/WebRTC/{path}") as response:
        return json.load(response)


def main():
    package = Path("apps/ios/Kit/Package.swift").read_text()
    version = re.search(r'\.package\(url: "https://github.com/stasel/WebRTC.git", exact: "([0-9.]+)"\)', package)
    if not version:
        raise ValueError("Expected an exact WebRTC version in Package.swift")
    version = version[1]
    manifest = base64.b64decode(metadata(f"contents/Package.swift?ref={version}")["content"]).decode()
    artifact = re.search(r'url:\s*"(https://github.com/stasel/WebRTC/releases/download/[^"\s]+)"\s*,\s*checksum:\s*"([a-f0-9]{64})"', manifest)
    if not artifact:
        raise ValueError("Expected one checksummed WebRTC binary target")
    url, checksum = artifact.groups()
    release = metadata(f"releases/tags/{version}")
    asset = next(item for item in release["assets"] if item["browser_download_url"] == url)
    cache = Path(sys.argv[1]) / "artifacts"
    cache.mkdir(parents=True, exist_ok=True)
    # SwiftPM BinaryArtifactsManager uses the URL mangled as a C99 identifier.
    destination = cache / re.sub(r"[^A-Za-z0-9_]", "_", url)
    if destination.exists() and hashlib.sha256(destination.read_bytes()).hexdigest() == checksum:
        print(f"WebRTC {version}: verified cached archive")
        return
    temporary = destination.with_suffix(".download")
    try:
        with request(asset["url"], "application/octet-stream") as response, temporary.open("wb") as output:
            digest = hashlib.sha256()
            for block in iter(lambda: response.read(1024 * 1024), b""):
                output.write(block)
                digest.update(block)
        if digest.hexdigest() != checksum:
            raise ValueError("WebRTC archive does not match the pinned manifest checksum")
        os.replace(temporary, destination)
        print(f"WebRTC {version}: downloaded and verified archive")
    finally:
        temporary.unlink(missing_ok=True)


if __name__ == "__main__":
    for attempt in range(1, 4):
        try:
            main()
            break
        except Exception as error:
            if attempt == 3 or isinstance(error, HTTPError) and error.code in (403, 429):
                raise
            print(f"WebRTC cache attempt {attempt} failed: {error}; retrying", flush=True)
            time.sleep(10 * attempt)
