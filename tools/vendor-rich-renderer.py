#!/usr/bin/env python3
"""Refresh pinned offline renderer assets. Downloads source bundles; installs no packages."""
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = {
    'mermaid': ('12.3.0', 'https://registry.npmjs.org/mermaid/-/mermaid-12.3.0.tgz', 'package/dist/mermaid.min.js'),
    'katex': ('0.19.1', 'https://registry.npmjs.org/katex/-/katex-0.19.1.tgz', 'package/dist/katex.min.js'),
    'highlight': ('11.12.0', 'https://registry.npmjs.org/@highlightjs/cdn-assets/-/cdn-assets-11.12.0.tgz', 'package/highlight.min.js'),
}
EXPECTED = {
    'mermaid': '336df803d87a38b70a01e736e666c8f7601d2ce42f5e47c8fafe5d721074b9a3',
    'katex': 'fa140498b8cfc8302cbe5e7cb1bdbc1f952bc38b3fa258ede09e73edbf9a74bc',
    'highlight': '8ab71eb09c51f501e5e25157d9cff100e46cc29bcbfc744d0b746d451fca7f53',
}
assets, notices = {}, ['# Offline rich renderer dependencies\n\nRefresh with `python3 tools/vendor-rich-renderer.py`. These are prebuilt source bundles, not locally installed packages.\n']
for name, (version, url, member) in PACKAGES.items():
    with urllib.request.urlopen(url, timeout=60) as response:
        archive_bytes = response.read()
    archive = tarfile.open(fileobj=io.BytesIO(archive_bytes), mode='r:gz')
    source = archive.extractfile(member).read()
    if hashlib.sha256(source).hexdigest() != EXPECTED[name]:
        raise ValueError(f'{name} source integrity check failed')
    assets[name] = base64.b64encode(gzip.compress(source, mtime=0)).decode('ascii')
    license_text = archive.extractfile('package/LICENSE').read().decode('utf-8')
    notices.append(f'\n## {name} {version}\n\nSource: {url}\n\nBundle SHA-256: `{hashlib.sha256(source).hexdigest()}`\n\n```text\n{license_text}\n```\n')
template = (ROOT / 'tools/rich-renderer.html').read_text()
html = template.replace('__CODYNC_ASSETS__', json.dumps(assets, separators=(',', ':')))
for target in [
    'apps/desktop/src/renderer/views/thread/rich-renderer.html',
    'apps/ios/Kit/Sources/CodyncUI/Resources/rich-renderer.html',
    'apps/android/app/src/main/assets/rich-renderer.html',
]:
    (ROOT / target).write_text(html)
(ROOT / 'docs/features/rich-renderer-notices.md').write_text(''.join(notices))
