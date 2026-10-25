#!/usr/bin/env python3
"""Assemble the portable AppImage inside the Ubuntu 22.04 build container."""
import csv, hashlib, json, os, pathlib, re, shutil, subprocess
try:
 import tomllib
except ImportError:
 import tomli as tomllib
root = pathlib.Path(__file__).resolve().parents[2]
os.chdir(root)
version = tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
outdir = root/'dist/portable'
outdir.mkdir(exist_ok=True)
app = outdir/'Glide.AppDir'
if app.exists():shutil.rmtree(app)
for name in ['usr/bin','usr/lib','usr/lib/gio/modules','usr/share/applications','usr/share/icons/hicolor/scalable/apps','usr/share/licenses']:
 (app/name).mkdir(parents=True,exist_ok=True)
# Host libc, loader and graphics drivers/dispatch stay together. Libraries built
# here target glibc <= 2.35; never copy a newer host's libc or graphics drivers.
# Keep the host C++ runtime too: Mesa/LLVM drivers may need newer GLIBCXX symbols.
excluded = {'libc.so.6','libm.so.6','libdl.so.2','libpthread.so.0','librt.so.1','libresolv.so.2',
 'ld-linux-x86-64.so.2','libEGL.so.1','libGL.so.1','libGLX.so.0','libGLdispatch.so.0',
 'libOpenGL.so.0','libGLESv2.so.2','libdrm.so.2','libgbm.so.1',
 'libstdc++.so.6','libgcc_s.so.1'}
inputs = set()
def copy(source,dest):
 source = pathlib.Path(source)
 dest.parent.mkdir(parents=True,exist_ok=True)
 shutil.copy2(source,dest)
 inputs.add(source)
def dependencies(binary):
 output = subprocess.check_output(['ldd',str(binary)],text=True)
 if 'not found' in output:raise RuntimeError(output)
 for name in re.findall(r'=> (/\S+)',output):
  p=pathlib.Path(name)
  if p.name not in excluded:copy(p,app/'usr/lib'/p.name)
copy(root/'target/portable/release/glide',app/'usr/bin/glide')
dependencies(app/'usr/bin/glide')
# GdkPixbuf dynamically loads image decoders; include these and their dependencies.
loaderdir = pathlib.Path('/usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders')
for source in loaderdir.glob('*.so'):
 copy(source,app/'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders'/source.name)
 dependencies(source)
query = pathlib.Path('/usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders')
copy(query,app/'usr/bin/gdk-pixbuf-query-loaders')
dependencies(query)
for source in [pathlib.Path('/opt/glide/share/glib-2.0/schemas'),pathlib.Path('/usr/share/icons/Adwaita'),pathlib.Path('/usr/share/icons/hicolor')]:
 if source.exists():
  dest=app/'usr/share'/source.relative_to(source.parents[1])
  # /opt/glide/share/glib-2.0/schemas and /usr/share/icons/* each keep two levels.
  shutil.copytree(source,dest,dirs_exist_ok=True)
  inputs.update(p for p in source.rglob('*') if p.is_file())
shutil.copy2('data/io.github.glide.Touchpad.desktop',app/'io.github.glide.Touchpad.desktop')
shutil.copy2('data/io.github.glide.Touchpad.desktop',app/'usr/share/applications/io.github.glide.Touchpad.desktop')
shutil.copy2('data/io.github.glide.Touchpad.svg',app/'io.github.glide.Touchpad.svg')
shutil.copy2('data/io.github.glide.Touchpad.svg',app/'usr/share/icons/hicolor/scalable/apps/io.github.glide.Touchpad.svg')
(app/'.DirIcon').symlink_to('io.github.glide.Touchpad.svg')
# Inventory and license notices for every Ubuntu-provided bundled file.
packages = set()
# Theme caches are generated from the adjacent, inventoried icon files.
query_paths=[str(p) for p in sorted(inputs) if p.name != 'icon-theme.cache' and not str(p).startswith(('/opt/glide',str(root)))]
owned=set()
for offset in range(0,len(query_paths),100):
 result=subprocess.run(['dpkg-query','-S',*query_paths[offset:offset+100]],capture_output=True,text=True)
 for line in result.stdout.splitlines():
  owner,path=line.split(': ',1);packages.add(owner);owned.add(path)
for source in set(query_paths)-owned:
 # Jammy's merged-/usr layout can give ldd a different path from dpkg.
 alternate=source.replace('/lib/','/usr/lib/',1) if source.startswith('/lib/') else source.replace('/usr/lib/','/lib/',1)
 result=subprocess.run(['dpkg-query','-S',alternate],capture_output=True,text=True)
 if result.returncode:raise RuntimeError(f'No source-package owner for {source}')
 packages.update(line.split(': ')[0] for line in result.stdout.splitlines())
rows=[]
for package in sorted(packages):
 record=subprocess.check_output(['dpkg-query','-W','-f=${Package}\t${Version}\t${Architecture}\t${source:Package}\t${source:Version}',package],text=True).split('\t')
 rows.append(record)
 docs=pathlib.Path('/usr/share/doc')/record[0]
 notice=docs/'copyright'
 if not notice.exists():raise FileNotFoundError(notice)
 copy(notice,app/'usr/share/licenses/ubuntu'/record[0]/'copyright')
with (outdir/'native-packages.tsv').open('w') as stream:
 writer=csv.writer(stream,delimiter='\t',lineterminator='\n')
 writer.writerow(['name','version','arch','source_package','source_version']);writer.writerows(rows)
shutil.copytree('/usr/share/common-licenses',app/'usr/share/licenses/common-licenses')
records=json.loads((root/'scripts/portable/sources.json').read_text())
for record in records:
 source=root/'build/portable'/f"{record['name']}-{record['version']}"
 for file in source.rglob('*'):
  if file.is_file() and (file.name.upper().startswith(('COPYING','LICENSE','NOTICE','COPYRIGHT')) or 'LICENSES' in file.parts):
   dest=app/'usr/share/licenses/upstream'/source.name/file.relative_to(source)
   dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(file,dest)
shutil.copytree(root/'dist/rust-notices',app/'usr/share/licenses/rust-crates')
sysroot=pathlib.Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())
shutil.copytree(sysroot/'share/doc/rust',app/'usr/share/licenses/rust-stdlib',ignore=shutil.ignore_patterns('html'))
for name in ['LICENSE','README.md','THIRD_PARTY.md']:
 shutil.copy2(root/name,app/name)
shutil.copytree(root/'docs',app/'docs')
shutil.copytree(root/'data/setup',app/'setup')
shutil.copy2(root/'dist/rust-crates.tsv',outdir/'rust-crates.tsv')
for name in ['native-packages.tsv','rust-crates.tsv']:
 shutil.copy2(outdir/name,app/name)
(app/'usr/share/glide').mkdir(parents=True,exist_ok=True)
(app/'usr/share/glide/fonts.conf').write_text('''<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <dir>/usr/share/fonts</dir><dir>/usr/local/share/fonts</dir>
  <dir prefix="xdg">fonts</dir><dir>~/.fonts</dir>
  <include ignore_missing="yes">/etc/fonts/conf.d</include>
  <cachedir prefix="xdg">fontconfig</cachedir>
</fontconfig>
''')
(app/'AppRun').write_text('''#!/bin/sh
set -eu
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
GLIDE_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export GIO_MODULE_DIR="$HERE/usr/lib/gio/modules"
export GDK_PIXBUF_MODULEDIR="$HERE/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/glide"
mkdir -p "$CACHE"
export GDK_PIXBUF_MODULE_FILE="$CACHE/pixbuf-loaders.cache"
LD_LIBRARY_PATH="$GLIDE_LIBRARY_PATH" "$HERE/usr/bin/gdk-pixbuf-query-loaders" > "$GDK_PIXBUF_MODULE_FILE.tmp.$$"
mv "$GDK_PIXBUF_MODULE_FILE.tmp.$$" "$GDK_PIXBUF_MODULE_FILE"
if [ -z "${FONTCONFIG_FILE:-}" ]; then
  if [ -f /etc/fonts/fonts.conf ]; then
    export FONTCONFIG_FILE=/etc/fonts/fonts.conf
  else
    export FONTCONFIG_FILE="$HERE/usr/share/glide/fonts.conf"
  fi
fi
export LD_LIBRARY_PATH="$GLIDE_LIBRARY_PATH"
exec "$HERE/usr/bin/glide" "$@"
''')
(app/'AppRun').chmod(0o755)
# Fail packaging if any bundled ELF requires a libc newer than the baseline.
required=set()
for file in app.rglob('*'):
 if not file.is_file() or file.is_symlink():continue
 with file.open('rb') as stream:magic=stream.read(4)
 if magic != b'\x7fELF':continue
 output=subprocess.check_output(['readelf','--version-info',str(file)],text=True)
 names=set(re.findall(r'Name: (GLIBC_\S+)',output))
 special={name for name in names if not re.fullmatch(r'GLIBC_[0-9.]+',name)}
 if special:raise RuntimeError(f'{file}: unsupported private/new glibc ABI requirements {special}')
 versions={tuple(map(int,v.split('.'))) for v in re.findall(r'GLIBC_([0-9.]+)',output)}
 if versions and max(versions) > (2,35):raise RuntimeError(f'{file}: GLIBC_{max(versions)} exceeds 2.35')
 required.update(versions)
 subprocess.run(['strip','--strip-unneeded',str(file)],check=True)
info={'version':version,'architecture':'x86_64','build_base':'Ubuntu 22.04','glibc_max_required':'.'.join(map(str,max(required))),'upstream':records}
(outdir/'build-info.json').write_text(json.dumps(info,indent=2)+'\n')
shutil.copy2(outdir/'build-info.json',app/'build-info.json')
runtime=root/'dist/runtime-x86_64'
assert hashlib.sha256(runtime.read_bytes()).hexdigest()=='617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46'
squash=outdir/'glide.squashfs'
subprocess.run(['mksquashfs',str(app),str(squash),'-noappend','-comp','zstd','-processors','2','-no-progress'],check=True)
output=outdir/f'Glide-{version}-x86_64.AppImage';temporary=output.with_suffix('.AppImage.tmp')
with temporary.open('wb') as stream:
 for file in [runtime,squash]:
  with file.open('rb') as source:shutil.copyfileobj(source,stream)
temporary.chmod(0o755);os.replace(temporary,output)
print(f'Packaged {output}; maximum GLIBC requirement {info["glibc_max_required"]}',flush=True)
