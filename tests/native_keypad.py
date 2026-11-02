"""Only type into a new virtual keyboard grabbed exclusively by libinput."""
import asyncio,glob,json,os,sys
from pathlib import Path
async def main():
 h=await asyncio.create_subprocess_exec(str(Path(sys.argv[1]).resolve()),'--input-helper',stdin=-1,stdout=-1,stderr=-1)
 c=None
 try:
  ready=(await asyncio.wait_for(h.stdout.readline(),5)).decode().strip();assert ready.startswith('READY ')
  media=(await asyncio.wait_for(h.stderr.readline(),5)).decode().strip();assert media.startswith('MEDIA ')
  for _ in range(50):
   nodes=glob.glob(media.split(' ',1)[1]+'/event*')
   if nodes and os.access('/dev/input/'+Path(nodes[0]).name,os.R_OK):break
   await asyncio.sleep(.1)
  c=await asyncio.create_subprocess_exec('stdbuf','-oL','libinput','debug-events','--device','/dev/input/'+Path(nodes[0]).name,'--grab','--show-keycodes',stdout=-1,stderr=-2)
  first=await asyncio.wait_for(c.stdout.readline(),5);assert b'DEVICE_ADDED' in first,first
  keys={str(i):'KEY_KP'+str(i) for i in range(10)}
  keys.update(decimal='KEY_KPDOT',add='KEY_KPPLUS',subtract='KEY_KPMINUS',multiply='KEY_KPASTERISK',divide='KEY_KPSLASH',enter='KEY_KPENTER',backspace='KEY_BACKSPACE',delete='KEY_DELETE',tab='KEY_TAB',num_lock='KEY_NUMLOCK')
  states=[p.read_text().strip() for p in Path('/sys/class/leds').glob('*::numlock/brightness')]
  expected_numlock=1 if states and all(v=='0' for v in states) else 0
  for name in keys:
   h.stdin.write((json.dumps({'keypad':name})+'\n').encode());await h.stdin.drain();await asyncio.sleep(.04)
  report=json.loads((await asyncio.wait_for(h.stdout.readline(),3)).decode())
  assert report['num_lock_state'] in ['on','off','unknown'],report
  if states and all(v=='1' for v in states):assert report['num_lock_state']=='on',report
  # Repeating enable must never turn an already-enabled Num Lock off.
  h.stdin.write((json.dumps({'keypad':'num_lock','seq':99})+'\n').encode());await h.stdin.drain()
  repeated=json.loads((await asyncio.wait_for(h.stdout.readline(),3)).decode())
  if states and all(v=='1' for v in states):assert repeated['num_lock_state']=='on',repeated
  h.stdin.close();await h.wait();await asyncio.sleep(.1)
  c.terminate();text=(first+(await c.communicate())[0]).decode()
  for name,code in keys.items():
   lines=[line for line in text.splitlines() if code+' ' in line]
   expected=expected_numlock*2 if name=='num_lock' else 1
   assert sum('pressed' in line for line in lines)==expected,(name,text)
   assert sum('released' in line for line in lines)==expected,(name,text)
  Path('evidence').mkdir(exist_ok=True);Path('evidence/native-keypad.txt').write_text(text)
  print('PASS: native keypad presses/releases and conditional Num Lock enable; no desktop keystrokes injected')
 finally:
  if c and c.returncode is None:c.terminate();await c.wait()
  if h.returncode is None:h.terminate();await h.wait()
asyncio.run(main())
