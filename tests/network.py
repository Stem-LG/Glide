"""Exercise real HTTP/WebSocket routes without touching desktop input."""
import asyncio, json, os, re, socket, sys, tempfile
from pathlib import Path
from aiohttp import ClientSession, WSServerHandshakeError

async def main():
    binary=str(Path(sys.argv[1] if len(sys.argv)>1 else 'target/debug/glide').resolve())
    with tempfile.TemporaryDirectory(prefix='glide-test-') as directory:
        with socket.socket() as sock:
            sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
        env={**os.environ,'GLIDE_CONFIG_DIR':directory}
        proc=await asyncio.create_subprocess_exec(binary,'--headless','--preview','--port',str(port),env=env,stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.PIPE)
        try:
            await proc.stdout.readline()
            code=(await proc.stdout.readline()).decode().strip().split(': ')[1]
            url=f'http://127.0.0.1:{port}'
            async with ClientSession() as client:
                for _ in range(60):
                    try:
                        async with client.get(url+'/health') as r:
                            assert (await r.json())['app']=='Glide'; break
                    except OSError: await asyncio.sleep(.1)
                else: raise RuntimeError('Server failed to start')
                # Verify actual asset bodies, not just the route status/MIME.
                for path, content in [('/style.css', ':root'), ('/tools.js', 'GlideTools'), ('/tools-ui.js', 'numpad'), ('/audio.js', 'PcmPlayer'), ('/audio-ui.js', 'refreshAudio'), ('/mic-ui.js', 'refreshMic'), ('/mic-worklet.js', 'registerProcessor')]:
                    async with client.get(url+path) as r:
                        body=await r.text();assert r.status==200 and content in body and '<!doctype' not in body,path
                headers={'Origin':url}
                async with client.post(url+'/api/pair',json={'code':code,'name':'Integration device'}) as r: assert r.status==403
                async with client.post(url+'/api/pair',headers=headers,json={'code':'bad','name':'x'}) as r: assert r.status==401
                async with client.post(url+'/api/pair',headers=headers,json={'code':code,'name':'Integration device'}) as r:
                    assert r.status==200; token=(await r.json())['token']
                config=json.loads(Path(directory,'config.json').read_text())
                assert token not in json.dumps(config) and len(config['devices'])==1
                assert Path(directory,'config.json').stat().st_mode&0o777==0o600
                async with client.post(url+'/api/pair',headers=headers,json={'code':code,'name':'Second'}) as r: assert r.status==401
                try: await client.ws_connect(url+'/ws',headers={'Origin':'http://wrong.invalid'})
                except WSServerHandshakeError as e: assert e.status==403
                else: raise AssertionError('Origin check failed')
                ws=await client.ws_connect(url+'/ws',headers=headers);await ws.send_json({'token':token});assert (await ws.receive_json())['ready']
                other=await client.ws_connect(url+'/ws',headers=headers);await other.send_json({'token':token});assert 'error' in await other.receive_json();await other.close()
                await ws.send_json({'points':[{'id':3,'x':.3,'y':.5}],'seq':1});assert (await ws.receive_json())['contacts']==1
                await ws.send_json({'points':[{'id':3,'x':-1,'y':.5}]});await ws.receive();await ws.close()
                await asyncio.sleep(.1)
                ws=await client.ws_connect(url+'/ws',headers=headers);await ws.send_json({'token':token});assert (await ws.receive_json())['ready']
                await asyncio.wait_for(ws.receive(),3);await ws.close()
                await asyncio.sleep(.1)
                ws=await client.ws_connect(url+'/ws',headers=headers);await ws.send_json({'token':token});ready=await ws.receive_json();assert ready['ready']
                # Desktop audio: same-origin, opt-in per device, one listener, tied to the input owner.
                try: await client.ws_connect(url+'/audio',headers={'Origin':'http://wrong.invalid'})
                except WSServerHandshakeError as e: assert e.status==403
                else: raise AssertionError('Audio origin check failed')
                async def listen(credential=token):
                    sock=await client.ws_connect(url+'/audio',headers=headers);await sock.send_json({'token':credential});return sock
                audio=await listen();assert 'error' in await audio.receive_json();await audio.close()
                await ws.send_json({'settings':{**ready['settings'],'audio':True}})
                while 'settings_saved' not in await ws.receive_json(): pass
                audio=await listen('wrong');assert 'error' in await audio.receive_json();await audio.close()
                audio=await listen();assert (await audio.receive_json())['audio']=={'rate':48000,'channels':2}
                frame=await audio.receive_bytes();assert len(frame)==1920 and any(frame)
                second=await listen();assert 'error' in await second.receive_json();await second.close()
                await ws.send_json({'settings':{**ready['settings'],'audio':False}})
                async def ended():
                    while (await audio.receive()).type.name not in ('CLOSE','CLOSING','CLOSED'): pass
                await asyncio.wait_for(ended(),3);await audio.close()
                await ws.send_json({'settings':{**ready['settings'],'audio':True}})
                while 'settings_saved' not in await ws.receive_json(): pass
                audio=await listen();assert 'audio' in await audio.receive_json()
                # Phone microphone: opt-in per device, sane rate, one sender, tied to the input owner.
                async def speak(rate=48000):
                    sock=await client.ws_connect(url+'/mic',headers=headers);await sock.send_json({'token':token,'rate':rate});return sock
                mic=await speak();assert 'error' in await mic.receive_json();await mic.close()
                await ws.send_json({'settings':{**ready['settings'],'audio':True,'microphone':True}})
                while 'settings_saved' not in await ws.receive_json(): pass
                mic=await speak(1000);assert 'error' in await mic.receive_json();await mic.close()
                mic=await speak(44100);assert (await mic.receive_json())['mic']=={'rate':44100}
                second=await speak();assert 'error' in await second.receive_json();await second.close()
                for _ in range(5): await mic.send_bytes(bytes(1920))
                await ws.close()
                await asyncio.wait_for(ended(),3);await audio.close()
                assert (await asyncio.wait_for(mic.receive(),3)).type.name in ('CLOSE','CLOSING','CLOSED');await mic.close()
                for _ in range(3):
                    async with client.post(url+'/api/pair',headers=headers,json={'code':'bad','name':'x'}) as r: await r.read()
                async with client.post(url+'/api/pair',headers=headers,json={'code':'bad','name':'x'}) as r: assert r.status==429
            # The same port serves HTTPS once the secure connection is on; plain HTTP redirects.
            proc.terminate();await proc.wait()
            proc=await asyncio.create_subprocess_exec(binary,'--headless','--preview','--https','--port',str(port),env=env,stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.PIPE)
            assert (await proc.stdout.readline()).decode().strip().endswith(f':{port}') ;code=(await proc.stdout.readline()).decode().strip().split(': ')[1]
            secure=f'https://127.0.0.1:{port}';headers={'Origin':secure}
            async with ClientSession() as client:
                for _ in range(60):
                    try:
                        async with client.get(secure+'/health',ssl=False) as r:
                            assert (await r.json())['app']=='Glide'; break
                    except OSError: await asyncio.sleep(.1)
                else: raise RuntimeError('Secure server failed to start')
                async with client.get(url+'/?x=1',allow_redirects=False) as r:
                    assert r.status==307 and r.headers['Location']==secure+'/?x=1'
                async with client.post(secure+'/api/pair',headers=headers,json={'code':code,'name':'Integration device','profile_id':config['devices'][0]['id']},ssl=False) as r:
                    assert r.status==200; token=(await r.json())['token']
                ws=await client.ws_connect(secure+'/ws',headers=headers,ssl=False);await ws.send_json({'token':token});assert (await ws.receive_json())['ready'];await ws.close()
            assert Path(directory,'tls-key.pem').stat().st_mode&0o777==0o600
            assert 'https' not in Path(directory,'config.json').read_text() or json.loads(Path(directory,'config.json').read_text())['https'] is False
            print('PASS: origin, pairing, hashed persistence, single owner, invalid input, timeout, reconnect, desktop audio, phone microphone, rate limit, HTTPS on the same port')
        finally:
            proc.terminate();await proc.wait()
            errors=(await proc.stderr.read()).decode()
            if errors:print(errors)
asyncio.run(main())
