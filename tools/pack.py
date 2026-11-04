"""Builds dist/baritone.hnmod from this folder.

The .hnmod is a zip of horizon-native.json (the registry entry), horizon-mixin.toml and every file the
toml names as a `source` or a `helper`, and the LICENSE, under mixins/baritone/. Run it from the repository root:

    python tools/pack.py
"""
import json
import re
import zipfile

toml = open('horizon-mixin.toml', encoding='utf-8').read()
version = re.search(r'^version = "([^"]+)"', toml, re.M).group(1)
sources = re.findall(r'^(?:source|helper) = "([^"]+)"', toml, re.M)
files = ['horizon-mixin.toml'] + sorted(set(sources))
meta = json.load(open('horizon-native.json', encoding='utf-8'))
meta['version'] = version
stamp = (2026, 1, 1, 0, 0, 0)  # a fixed time, so the same sources always give the same file
with zipfile.ZipFile('dist/baritone.hnmod', 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr(zipfile.ZipInfo('horizon-native.json', stamp), json.dumps(meta, indent=2, sort_keys=True) + '\n')
    for f in files + ['LICENSE']:
        z.writestr(zipfile.ZipInfo('mixins/baritone/' + f, stamp), open(f, 'rb').read())
print(version, len(files) + 1, 'files ->', 'dist/baritone.hnmod')
