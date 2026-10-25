import subprocess,pathlib,json
paths=set()
for line in subprocess.check_output(['ldd','target/release/glide'],text=True).splitlines():
 if '=> /' in line:
  p=line.split('=> ')[1].split()[0]
  if pathlib.Path(p).name not in {'libc.so.6','libm.so.6','libdl.so.2','libpthread.so.0','librt.so.1','ld-linux-x86-64.so.2','libresolv.so.2'}:paths.add(p)
for name in ['glib-2.0/schemas','icons/Adwaita','icons/hicolor']:
 paths.update(str(p) for p in (pathlib.Path('/usr/share')/name).rglob('*') if p.is_file())
rows=set()
for i in range(0,len(paths),100):
 out=subprocess.run(['rpm','-qf','--qf','%{NAME}\t%{VERSION}-%{RELEASE}\t%{ARCH}\t%{SOURCERPM}\t%{LICENSE}\n',*sorted(paths)[i:i+100]],text=True,capture_output=True).stdout
 rows.update(l for l in out.splitlines() if '\t' in l)
pathlib.Path('dist').mkdir(exist_ok=True)
pathlib.Path('dist/native-packages.tsv').write_text('name\tversion\tarch\tsource_rpm\tlicense\n'+'\n'.join(sorted(rows))+'\n')
print('\n'.join(sorted(rows)))
