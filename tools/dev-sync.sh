#!/bin/bash
# Developer helper: apply this bundle to a copy of the pinned Horizon source and copy only the changed
# files into a persistent build tree, so cargo keeps its fingerprints and rebuilds just what changed.
# Needs cargo on PATH. Set HORIZON_WORK (a scratch folder) and HORIZON_LAUNCHER (the launcher data folder) first.
set -e
REPO="$(cd "$(dirname "$0")/.." && pwd)"
: "${HORIZON_WORK:?set HORIZON_WORK to a scratch folder}"
: "${HORIZON_LAUNCHER:?set HORIZON_LAUNCHER to the horizon-launcher data folder}"
# The Horizon commit the bundle pins (horizon-native.json); HORIZON_REV tries another, and HORIZON_SRC names the folder that holds its source.
PIN=$(python -c 'import json,sys; print(json.load(open(sys.argv[1]))["horizon"])' "$REPO/horizon-native.json")
REV=${HORIZON_REV:-$PIN}
MIXINS=$(python -c 'import json,sys; print(json.load(open(sys.argv[1]))["mixins"][:12])' "$REPO/horizon-native.json")
D="$HORIZON_WORK/mods/baritone/mixins/baritone"
rm -rf "$D"; mkdir -p "$D"
cp "$REPO/horizon-mixin.toml" "$D/"
# Another commit than the one the toml pins: the copy that is applied says so.
sed -i "s/horizon_revision = \"[0-9a-f]\{40\}\"/horizon_revision = \"$REV\"/" "$D/horizon-mixin.toml"
cp -r "$REPO/src" "$REPO/hooks" "$D/"
cd "$HORIZON_WORK"
rm -rf stage-new
"$HORIZON_LAUNCHER/native/tools/$MIXINS/bin/horizon-mixins.exe" apply \
  --workspace "${HORIZON_SRC:-$HORIZON_LAUNCHER/native/source/$REV}" --mods mods/baritone/mixins --out stage-new
python - <<'PY'
import filecmp, os, shutil
src, dst = 'stage-new', 'stage'
changed = 0
for root, _, files in os.walk(src):
    rel = os.path.relpath(root, src)
    os.makedirs(os.path.join(dst, rel), exist_ok=True)
    for f in files:
        a, b = os.path.join(root, f), os.path.join(dst, rel, f)
        if not os.path.exists(b) or not filecmp.cmp(a, b, shallow=False):
            shutil.copyfile(a, b)
            changed += 1
print('changed files:', changed)
PY
