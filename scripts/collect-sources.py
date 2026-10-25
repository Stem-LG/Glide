#!/usr/bin/env python3
"""Collect exact Fedora SRPMs and locked Rust crate sources inside the build container."""
import csv,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parent.parent
os.chdir(root)
subprocess.run(['python3','scripts/native-inventory.py'],check=True,stdout=subprocess.DEVNULL)
rows=list(csv.DictReader(Path('dist/native-packages.tsv').open(),delimiter='\t'))
source=root/'dist/source-bundle'
(source/'native').mkdir(parents=True,exist_ok=True)
# One installed binary package per SRPM; dnf resolves the exact NVR to its source.
packages={r['source_rpm']:f"{r['name']}-{r['version']}.{r['arch']}" for r in rows}
subprocess.run(['dnf','download','--skip-unavailable','--source','--destdir='+str(source/'native'),*packages.values()],check=True)
missing=[name for name in packages if not (source/'native'/name).exists()]
for filename in missing:
 import urllib.request
 name,version,release=filename.removesuffix('.src.rpm').rsplit('-',2)
 url=f'https://kojipkgs.fedoraproject.org/packages/{name}/{version}/{release}/src/{filename}'
 print('Fetching archived source:',filename,flush=True)
 urllib.request.urlretrieve(url,source/'native'/filename)
for filename in packages:
 subprocess.run(['rpm','--checksig','--nosignature',str(source/'native'/filename)],check=True,stdout=subprocess.DEVNULL)
with (source/'vendor-config.toml').open('w') as config:
 subprocess.run(['cargo','vendor','--locked','--versioned-dirs',str(source/'vendor')],stdout=config,check=True)
# Make the vendor config portable after extracting the source archive.
p=source/'vendor-config.toml';p.write_text(p.read_text().replace(str(source/'vendor'),'vendor'))
print(f'Collected {len(packages)} exact native source packages and locked Rust sources')
