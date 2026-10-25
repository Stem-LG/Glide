#!/usr/bin/env python3
"""Archive tracked application files with the collected dependency sources."""
import hashlib,json,pathlib,shutil,subprocess,tarfile
try:
 import tomllib
except ImportError:
 import tomli as tomllib
root=pathlib.Path(__file__).resolve().parents[2]
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
out=root/'dist/portable';bundle=out/'source-bundle';app=bundle/'glide'
if app.exists():shutil.rmtree(app)
for name in subprocess.check_output(['git','ls-files','-z'],cwd=root).decode().split('\0'):
 if not name:continue
 p=root/name
 if not p.is_file():continue
 dest=app/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest)
(bundle/'README.txt').write_text(f'Glide {version} corresponding sources\n\nSee glide/docs/BUILDING.md for build instructions.\nNative Ubuntu sources: native/\nPinned upstream library sources: upstream/\nRust dependencies: vendor/\nAppImage runtime sources: appimage-runtime/\n')
archive=out/f'Glide-{version}-sources.tar.gz'
with tarfile.open(archive,'w:gz',compresslevel=1) as tar:
 tar.add(bundle,arcname=f'Glide-{version}-sources')
files=[f'Glide-{version}-x86_64.AppImage',archive.name,'native-packages.tsv','rust-crates.tsv','build-info.json']
lines=[]
for name in files:
 with (out/name).open('rb') as stream:digest=hashlib.sha256(stream.read()).hexdigest()
 lines.append(f'{digest}  {name}')
(out/'SHA256SUMS').write_text('\n'.join(lines)+'\n')
print('\n'.join(lines))
