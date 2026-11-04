"""Builds dist/baritone.hnmod from this folder, and a copy named after the Nightly it is pinned to.

The .hnmod is a zip of horizon-native.json (the registry entry), horizon-mixin.toml and every file the
toml names as a `source` or a `helper`, and the LICENSE, under mixins/baritone/. Run it from the repository root:

    python tools/pack.py                    # pinned to the commit in horizon-native.json
    python tools/pack.py 6475f62f3c83...    # also one file for each other Horizon commit given (40 letters)

A mod is built into the Horizon commit it is pinned to, so each Nightly a player can be on needs a file of its own, and the file says
which: dist/baritone-<version>-nightly-<date>-<7 letters of the commit>.hnmod, the way the launcher's Versions tab shows it.
Only pin a commit the sources were checked against (the mixin tool applies them and the game compiles).
"""
import json
import re
import shutil
import sys
import zipfile

NIGHTLIES = {
    '142388e1c136e622652ec7505af15d8ad195e041': '2026-10-28',
    'ba6344a59cb9e755b730dcf02285fc0d33b97d3c': '2026-10-30',
    '3fb4c3138e0c5d8fdd2fdb555cf31368e8897497': '2026-11-04',
    '6475f62f3c834b7489e1816bf47f47ea5eb2b879': '2026-11-04',
}

toml = open('horizon-mixin.toml', encoding='utf-8').read()
version = re.search(r'^version = "([^"]+)"', toml, re.M).group(1)
sources = re.findall(r'^(?:source|helper) = "([^"]+)"', toml, re.M)
files = ['horizon-mixin.toml'] + sorted(set(sources))
meta = json.load(open('horizon-native.json', encoding='utf-8'))
meta['version'] = version
stamp = (2026, 1, 1, 0, 0, 0)  # a fixed time, so the same sources always give the same file


def pack(path, pin):
    """The .hnmod at `path`, pinned to the Horizon commit `pin`."""
    entry = dict(meta, horizon=pin)
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr(zipfile.ZipInfo('horizon-native.json', stamp), json.dumps(entry, indent=2, sort_keys=True) + '\n')
        for f in files + ['LICENSE']:
            data = open(f, 'rb').read()
            if f == 'horizon-mixin.toml':
                data = re.sub(rb'horizon_revision = "[0-9a-f]{40}"', b'horizon_revision = "' + pin.encode() + b'"', data)
            z.writestr(zipfile.ZipInfo('mixins/baritone/' + f, stamp), data)


def named(pin):
    when = NIGHTLIES.get(pin)
    return f"dist/baritone-{version}-nightly-{when}-{pin[:7]}.hnmod" if when else f"dist/baritone-{version}-horizon-{pin[:7]}.hnmod"


pack('dist/baritone.hnmod', meta['horizon'])
print(version, len(files) + 1, 'files ->', 'dist/baritone.hnmod')
for pin in [meta['horizon']] + sys.argv[1:]:
    pack(named(pin), pin)
    print('and', named(pin))
