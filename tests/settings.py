"""Desktop-owned preferences survive server restart, origin changes and re-pairing."""
import asyncio, json, os, socket, sys, tempfile
from pathlib import Path
import aiohttp

async def main():
 binary=str(Path(sys.argv[1]).resolve())
 with tempfile.TemporaryDirectory() as directory:
  async def start():
   with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
   proc=await asyncio.create_subprocess_exec(binary,'--headless','--preview','--port',str(port),env={**os.environ,'GLIDE_CONFIG_DIR':directory},stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.PIPE)
   await asyncio.wait_for(proc.stdout.readline(),5)
   code=(await asyncio.wait_for(proc.stdout.readline(),5)).decode().strip().split()[-1]
   for _ in range(100):
    try:
     reader,writer=await asyncio.open_connection('127.0.0.1',port);writer.close();await writer.wait_closed();break
    except OSError:await asyncio.sleep(.02)
   else:raise AssertionError('Preview did not start')
   return proc,f'http://127.0.0.1:{port}',code
  async def receive(ws,key):
   while True:
    data=await asyncio.wait_for(ws.receive_json(),2)
    if key in data:return data
  async def connect(client,url,token):
   ws=await client.ws_connect(url+'/ws',headers={'Origin':url});await ws.send_json({'token':token});return ws
  proc=None
  try:
   async with aiohttp.ClientSession() as client:
    proc,url,code=await start()
    async def pair(url,code,**extra):
     async with client.post(url+'/api/pair',headers={'Origin':url},json={'code':code,'name':'Test phone',**extra}) as r:
      return r.status,await r.json() if r.status==200 else await r.text()
    status,paired=await pair(url,code);assert status==200 and 'token' in paired
    ws=await connect(client,url,paired['token']);ready=await receive(ws,'ready');assert ready['settings']['cursor_speed']==1
    assert ready['settings']['media_controls'] is False
    await ws.send_json({'media':'play_pause','seq':101})
    assert (await receive(ws,'ack'))['media'] is None
    settings={'cursor_speed':1.7,'scroll_speed':.6,'show_points':True,'haptics':False,'media_controls':True,'haptic_duration_ms':42,'theme':'dark'}
    await ws.send_json({'settings':settings});saved=await receive(ws,'settings_saved');assert saved['settings']==settings
    persisted=json.loads(Path(directory,'config.json').read_text());assert persisted['devices'][0]['settings']==settings
    await ws.send_json({'media':'play_pause','seq':102})
    assert (await receive(ws,'ack'))['media']=='play_pause'
    await ws.send_json({'media':'play_pause','seq':103})
    assert (await receive(ws,'ack'))['media'] is None
    await ws.send_json({'settings':{**settings,'scroll_speed':9}});await receive(ws,'settings_error');assert json.loads(Path(directory,'config.json').read_text())['devices'][0]['settings']==settings
    for invalid in [0,4,101,-1,25.5,'25']:
     await ws.send_json({'settings':{**settings,'haptic_duration_ms':invalid}});await receive(ws,'settings_error')
     assert json.loads(Path(directory,'config.json').read_text())['devices'][0]['settings']==settings
    await ws.close();proc.terminate();await proc.wait()
    old_url=url;proc,url,code=await start();assert url!=old_url
    # Existing credential still maps to the stable profile after restart.
    ws=await connect(client,url,paired['token']);ready=await receive(ws,'ready');assert ready['settings']==settings and ready['device_id']==paired['id']
    await ws.close();await asyncio.sleep(.1)
    # No unauthenticated profile listing. Correct PIN exposes only id/name.
    status,data=await pair(url,'wrong');assert status==401
    status,data=await pair(url,code);assert status==200 and data['profiles']==[{'id':paired['id'],'name':'Test phone'}]
    status,restored=await pair(url,code,profile_id=paired['id']);assert status==200 and restored['id']==paired['id'] and restored['token']!=paired['token']
    ws=await connect(client,url,paired['token']);assert 'error' in await receive(ws,'error');await ws.close()
    ws=await connect(client,url,restored['token']);assert (await receive(ws,'ready'))['settings']==settings
    await ws.close()
    assert len(json.loads(Path(directory,'config.json').read_text())['devices'])==1
    print('PASS: desktop persistence, invalid-setting rejection, restart/new port, authenticated profile restore, stable ID, credential rotation')
  finally:
   if proc and proc.returncode is None:proc.terminate();await proc.wait()
asyncio.run(main())
