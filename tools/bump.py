"""Raises the version everywhere it is written: horizon-mixin.toml (the one pack.py reads), horizon-native.json,
Cargo.toml and src/baritone.rs. Run it before every build that is going to be tried:

    python tools/bump.py            # 1.1.4 -> 1.1.5
    python tools/bump.py 1.2.0      # set it
"""
import json
import re
import sys

toml = open('horizon-mixin.toml', encoding='utf-8', newline='').read()
current = re.search(r'^version = "([^"]+)"', toml, re.M).group(1)
if len(sys.argv) > 1:
    new = sys.argv[1]
else:
    major, minor, patch = (int(p) for p in current.split('.'))
    new = f'{major}.{minor}.{patch + 1}'


def rewrite(path, pattern, repl, flags=0):
    text = open(path, encoding='utf-8', newline='').read()
    text, n = re.subn(pattern, repl, text, count=1, flags=flags)
    assert n == 1, path
    open(path, 'w', encoding='utf-8', newline='').write(text)


rewrite('horizon-mixin.toml', r'^version = "[^"]+"', f'version = "{new}"', re.M)
rewrite('Cargo.toml', r'^version = "[^"]+"', f'version = "{new}"', re.M)
rewrite('src/baritone.rs', r'pub const VERSION: &str = "[^"]+";', f'pub const VERSION: &str = "{new}";')
meta = json.load(open('horizon-native.json', encoding='utf-8'))
meta['version'] = new
open('horizon-native.json', 'w', encoding='utf-8', newline='\n').write(json.dumps(meta, indent=2, sort_keys=True) + '\n')
print(current, '->', new)
