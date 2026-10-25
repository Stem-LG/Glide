#!/usr/bin/env python3
"""Run inside the Fedora build container after cargo build --release."""
import hashlib, os, re, shutil, subprocess, tomllib
from pathlib import Path
root=Path(__file__).resolve().parent.parent
os.chdir(root)
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
app=root/'dist/Glide.AppDir'
if app.exists():shutil.rmtree(app)
(app/'usr/bin').mkdir(parents=True)
(app/'usr/lib').mkdir()
(app/'usr/share/applications').mkdir(parents=True)
(app/'usr/share/icons/hicolor/scalable/apps').mkdir(parents=True)
shutil.copy2('target/release/glide',app/'usr/bin/glide')
subprocess.run(['strip',str(app/'usr/bin/glide')],check=True)
for dest in [app/'io.github.glide.Touchpad.desktop',app/'usr/share/applications/io.github.glide.Touchpad.desktop']:
 shutil.copy2('data/io.github.glide.Touchpad.desktop',dest)
for dest in [app/'io.github.glide.Touchpad.svg',app/'usr/share/icons/hicolor/scalable/apps/io.github.glide.Touchpad.svg']:
 shutil.copy2('data/io.github.glide.Touchpad.svg',dest)
(app/'.DirIcon').symlink_to('io.github.glide.Touchpad.svg')
# ldd traverses transitive ELF dependencies. glibc and graphics drivers stay on host.
excluded={'libc.so.6','libm.so.6','libdl.so.2','libpthread.so.0','librt.so.1','ld-linux-x86-64.so.2','libresolv.so.2'}
output=subprocess.check_output(['ldd',str(app/'usr/bin/glide')],text=True)
for src in re.findall(r'=> (/\S+)',output):
 p=Path(src)
 if p.name not in excluded:shutil.copy2(p,app/'usr/lib'/p.name)
for name in ['glib-2.0/schemas','icons/Adwaita','icons/hicolor']:
 src=Path('/usr/share')/name
 if src.exists():shutil.copytree(src,app/'usr/share'/name,dirs_exist_ok=True)
# Include package license notices for bundled runtime dependencies.
shutil.copytree('/usr/share/licenses',app/'usr/share/licenses',dirs_exist_ok=True)
shutil.copy2('README.md',app/'README.md')
shutil.copy2('LICENSE',app/'LICENSE')
shutil.copy2('THIRD_PARTY.md',app/'THIRD_PARTY.md')
shutil.copytree('docs',app/'docs')
shutil.copytree('dist/rust-notices',app/'usr/share/licenses/glide-rust')
for inventory in ['native-packages.tsv','rust-crates.tsv']:
 shutil.copy2(root/'dist'/inventory, app/inventory)
# Rust standard-library notices (the standard library is linked statically).
sysroot=Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())
shutil.copytree(sysroot/'share/doc/rust',app/'usr/share/licenses/rust-stdlib',ignore=shutil.ignore_patterns('html'),dirs_exist_ok=True)
shutil.copytree('data/setup',app/'setup')
(app/'AppRun').write_text('''#!/bin/sh
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$HERE/usr/bin/glide" "$@"
''')
(app/'AppRun').chmod(0o755)
runtime=root/'dist/runtime-x86_64'
assert hashlib.sha256(runtime.read_bytes()).hexdigest()=='617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46','Runtime checksum mismatch'
squash=root/'dist/glide.squashfs'
subprocess.run(['mksquashfs',str(app),str(squash),'-noappend','-comp','zstd','-processors','2','-no-progress'],check=True)
out=root/f'dist/Glide-{version}-x86_64.AppImage'
temporary=out.with_suffix('.AppImage.tmp')
with temporary.open('wb') as target:
 for p in [runtime,squash]:
  with p.open('rb') as source:shutil.copyfileobj(source,target)
temporary.chmod(0o755)
os.replace(temporary,out)
(root/'dist/SHA256SUMS').write_text(hashlib.sha256(out.read_bytes()).hexdigest()+'  '+out.name+'\n')
print(out)
