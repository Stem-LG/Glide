#!/usr/bin/env python3
"""Collect exact corresponding sources for the portable AppImage."""
import csv,json,pathlib,shutil,subprocess
root=pathlib.Path(__file__).resolve().parents[2]
out=root/'dist/portable';bundle=out/'source-bundle'
(bundle/'native').mkdir(parents=True,exist_ok=True)
rows=list(csv.DictReader((out/'native-packages.tsv').open(),delimiter='\t'))
packages=sorted({(r['source_package'],r['source_version']) for r in rows})
subprocess.run(['apt-get','update'],check=True)
for name,version in packages:
 subprocess.run(['apt-get','source','--download-only',f'{name}={version}'],cwd=bundle/'native',check=True)
upstream=bundle/'upstream';upstream.mkdir(exist_ok=True)
for r in json.loads((root/'scripts/portable/sources.json').read_text()):
 name=pathlib.Path(r['url']).name;shutil.copy2(root/'build/portable/sources'/name,upstream/name)
# Use Cargo's locked sources, independently of any previous release artifacts.
with (bundle/'vendor-config.toml').open('w') as stream:
 subprocess.run(['cargo','vendor','--locked','--versioned-dirs',str(bundle/'vendor')],cwd=root,stdout=stream,check=True)
p=bundle/'vendor-config.toml';p.write_text(p.read_text().replace(str(bundle/'vendor'),'vendor'))
runtime=root/'dist/source-bundle/appimage-runtime'
if not runtime.exists():raise FileNotFoundError('Place the pinned runtime source archives in dist/source-bundle/appimage-runtime (see docs/BUILDING.md)')
shutil.copytree(runtime,bundle/'appimage-runtime',dirs_exist_ok=True)
for name in ['native-packages.tsv','rust-crates.tsv','build-info.json']:
 shutil.copy2(out/name,bundle/name)
shutil.copy2(root/'dist/runtime-x86_64',bundle/'runtime-x86_64')
print(f'Collected {len(packages)} exact Ubuntu source packages plus pinned upstream and Rust sources')
