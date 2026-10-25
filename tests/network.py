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
                ws=await client.ws_connect(url+'/ws',headers=headers);await ws.send_json({'token':token});assert (await ws.receive_json())['ready'];await ws.close()
                for _ in range(3):
                    async with client.post(url+'/api/pair',headers=headers,json={'code':'bad','name':'x'}) as r: await r.read()
                async with client.post(url+'/api/pair',headers=headers,json={'code':'bad','name':'x'}) as r: assert r.status==429
            print('PASS: origin, pairing, hashed persistence, single owner, invalid input, timeout, reconnect, rate limit')
        finally:
            proc.terminate();await proc.wait()
            errors=(await proc.stderr.read()).decode()
            if errors:print(errors)
asyncio.run(main())
