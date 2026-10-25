#!/usr/bin/env python3
"""Prepare locked Rust notices and checksum-pinned AppImage runtime inputs."""
import hashlib,json,pathlib,subprocess,urllib.request
root=pathlib.Path(__file__).resolve().parents[2]
dist=root/'dist';dist.mkdir(exist_ok=True)
for record in json.loads((root/'scripts/portable/runtime-sources.json').read_text()):
 dest=dist/record['path'];dest.parent.mkdir(parents=True,exist_ok=True)
 if not dest.exists():urllib.request.urlretrieve(record['url'],dest)
 assert hashlib.sha256(dest.read_bytes()).hexdigest()==record['sha256'],f'Checksum mismatch: {dest}'
subprocess.run(['cargo','vendor','--locked','--versioned-dirs',str(dist/'source-bundle/vendor')],cwd=root,check=True,stdout=subprocess.DEVNULL)
subprocess.run(['python3','scripts/rust-notices.py'],cwd=root,check=True)
