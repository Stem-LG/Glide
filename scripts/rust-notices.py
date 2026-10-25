#!/usr/bin/env python3
"""Collect Cargo metadata and original license notices from vendored sources."""
import pathlib, shutil
try:
    import tomllib
except ImportError:
    import tomli as tomllib
root = pathlib.Path(__file__).resolve().parent.parent
vendor = root / 'dist/source-bundle/vendor'
out = root / 'dist/rust-notices'
out.mkdir(exist_ok=True)
rows = ['name\tversion\tlicense']
for crate in sorted(vendor.iterdir()):
    pkg = tomllib.loads((crate / 'Cargo.toml').read_text())['package']
    rows.append('\t'.join([pkg['name'], pkg['version'], pkg.get('license', pkg.get('license-file', 'See source'))]))
    for src in crate.rglob('*'):
        if src.is_file() and any(src.name.upper().startswith(x) for x in ('LICENSE', 'COPYING', 'NOTICE', 'COPYRIGHT')):
            dest = out / crate.name / src.relative_to(crate)
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dest)
(root / 'dist/rust-crates.tsv').write_text('\n'.join(rows) + '\n')
print(f'Collected notices for {len(rows)-1} crates; full originals remain in the source bundle.')
