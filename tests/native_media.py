"""Only inject into newly created virtual devices grabbed by this test."""
import asyncio,glob,json,os,sys
from pathlib import Path

async def main():
 helper=await asyncio.create_subprocess_exec(str(Path(sys.argv[1]).resolve()),'--input-helper',stdin=-1,stdout=-1,stderr=-1)
 captures=[]
 try:
  ready=(await asyncio.wait_for(helper.stdout.readline(),5)).decode().strip()
  media=(await asyncio.wait_for(helper.stderr.readline(),5)).decode().strip()
  assert ready.startswith('READY ') and media.startswith('MEDIA '),(ready,media)
  for path in [ready.split(' ',1)[1],media.split(' ',1)[1]]:
   for _ in range(50):
    nodes=glob.glob(path+'/event*')
    if nodes and os.path.exists('/dev/input/'+Path(nodes[0]).name):break
    await asyncio.sleep(.1)
   capture=await asyncio.create_subprocess_exec('stdbuf','-oL','libinput','debug-events','--device','/dev/input/'+Path(nodes[0]).name,'--grab','--enable-tap','--set-tap-map=lrm','--show-keycodes',stdout=-1,stderr=-2)
   captures.append(capture)
   first=await asyncio.wait_for(capture.stdout.readline(),5)
   assert b'DEVICE_ADDED' in first,first
  async def frame(data):
   helper.stdin.write((json.dumps(data)+'\n').encode());await helper.stdin.drain()
  for action in ['play_pause','previous','next','volume_up','volume_down','volume_up']:
   await frame({'media':action});await asyncio.sleep(.12)
  # A buffered native tap has its down/up sent back-to-back. It must still
  # produce the correct native button, with no duplicated click.
  for n in [1,2,3]:
   await frame({'points':[{'id':i,'x':.3+i*.1,'y':.4} for i in range(n)]})
   await frame({'points':[]});await asyncio.sleep(.4)
  helper.stdin.close();await helper.wait();await asyncio.sleep(.1)
  output=[]
  for c in captures:
   c.terminate();output.append((await c.communicate())[0].decode())
  touch,keys=output
  for key,count in [('KEY_PLAYPAUSE',1),('KEY_PREVIOUSSONG',1),('KEY_NEXTSONG',1),('KEY_VOLUMEUP',2),('KEY_VOLUMEDOWN',1)]:
   matching=[line for line in keys.splitlines() if key in line]
   assert sum('pressed' in line for line in matching)==count,(key,keys)
   assert sum('released' in line for line in matching)==count,(key,keys)
  for button in ['BTN_LEFT','BTN_RIGHT','BTN_MIDDLE']:
   matching=[line for line in touch.splitlines() if button in line]
   assert sum('pressed' in line for line in matching)==1,(button,touch)
   assert sum('released' in line for line in matching)==1,(button,touch)
  assert 'KEYBOARD_KEY' not in touch and 'POINTER_BUTTON' not in keys
  Path('evidence').mkdir(exist_ok=True)
  Path('evidence/native-media.txt').write_text(touch+'\n'+keys)
  print('PASS: isolated Linux media key presses/releases and native 1/2/3-finger taps')
 finally:
  for c in captures:
   if c.returncode is None:c.terminate();await c.wait()
  if helper.returncode is None:helper.terminate();await helper.wait()
asyncio.run(main())
