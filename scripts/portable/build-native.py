#!/usr/bin/env python3
"""Build the pinned UI stack against Ubuntu 22.04, never the host's libc."""
import hashlib, json, os, pathlib, subprocess, tarfile, urllib.request, shutil
root = pathlib.Path(__file__).resolve().parents[2]
work = root / 'build/portable'
sources = work / 'sources'
sources.mkdir(parents=True, exist_ok=True)
options = {
 'glib': ['-Dtests=false','-Dinstalled_tests=false','-Dintrospection=disabled','-Ddocumentation=false','-Dman-pages=disabled','-Dselinux=disabled','-Dlibmount=disabled'],
 'harfbuzz': ['-Dtests=disabled','-Ddocs=disabled','-Dintrospection=disabled','-Dutilities=disabled','-Dicu=disabled','-Dcairo=disabled'],
 'cairo': ['-Dtests=disabled','-Dgtk_doc=false'],
 'fontconfig': ['-Ddoc=disabled','-Dtests=disabled','-Dtests-external-fonts=disabled','-Dtools=disabled','-Dcache-build=disabled','-Dxml-backend=libxml2','-Dbaseconfig-dir=/etc/fonts','-Dconfig-dir=/etc/fonts/conf.d'],
 'pango': ['-Dintrospection=disabled','-Ddocumentation=false','-Dbuild-testsuite=false','-Dbuild-examples=false'],
 'wayland': ['-Ddocumentation=false','-Dtests=false'],
 'wayland-protocols': ['-Dtests=false'],
 'gtk': ['-Dintrospection=disabled','-Ddocumentation=false','-Dbuild-demos=false','-Dbuild-testsuite=false','-Dbuild-tests=false','-Dbuild-examples=false','-Dvulkan=disabled','-Dmedia-gstreamer=disabled','-Dprint-cups=disabled','-Dcloudproviders=disabled','-Dtracker=disabled','-Df16c=disabled'],
 'libadwaita': ['-Dintrospection=disabled','-Dvapi=false','-Ddocumentation=false','-Dtests=false','-Dexamples=false'],
}
for record in json.loads((root/'scripts/portable/sources.json').read_text()):
 name, version = record['name'], record['version']
 archive = sources / pathlib.Path(record['url']).name
 if not archive.exists():
  urllib.request.urlretrieve(record['url'], archive)
 assert hashlib.sha256(archive.read_bytes()).hexdigest() == record['sha256'], archive
 src = work / f'{name}-{version}'
 if not (src/'.glide-extracted').exists():
  if src.exists():shutil.rmtree(src)
  with tarfile.open(archive) as tar:
   # Materialize archive file links. Older Ubuntu Python's data filter treats
   # symlink targets as archive-root-relative instead of parent-relative.
   members = tar.getmembers()
   tar.extractall(work, members=[m for m in members if not (m.issym() or m.islnk())], filter='data')
   for member in members:
    if member.issym() or member.islnk():
     destination = (work/member.name).resolve()
     destination.relative_to(src.resolve())
     destination.parent.mkdir(parents=True, exist_ok=True)
     with tar.extractfile(member) as stream:
      destination.write_bytes(stream.read())
  (src/'.glide-extracted').write_text(record['sha256'])
 build = work / f'{name}-build'
 stamp = pathlib.Path('/opt/glide/share/glide-build') / name
 stamp.parent.mkdir(parents=True, exist_ok=True)
 fingerprint = hashlib.sha256(json.dumps([record, options[name]]).encode()).hexdigest()
 if stamp.exists() and stamp.read_text() == fingerprint and pathlib.Path('/opt/glide').exists():
  print(f'Already built {name} {version}', flush=True)
  continue
 command = ['meson','setup',str(build),str(src),'--prefix=/opt/glide','--libdir=lib','--buildtype=release','--wrap-mode=nodownload',*options[name]]
 if (build/'build.ninja').exists():command.append('--reconfigure')
 subprocess.run(command, check=True)
 subprocess.run(['meson','compile','-C',str(build),'-j',os.environ.get('GLIDE_BUILD_JOBS','4')], check=True)
 subprocess.run(['meson','install','-C',str(build)], check=True)
 stamp.write_text(fingerprint)
print('Native stack ready', flush=True)
