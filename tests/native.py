"""Test only a freshly created virtual device, exclusively grabbed by libinput."""
import asyncio, glob, json, os, sys
from pathlib import Path

async def main():
 binary=str(Path(sys.argv[1] if len(sys.argv)>1 else 'target/debug/glide').resolve())
 helper=await asyncio.create_subprocess_exec(binary,'--input-helper',stdin=asyncio.subprocess.PIPE,stdout=asyncio.subprocess.PIPE)
 capture=None
 try:
  ready=(await asyncio.wait_for(helper.stdout.readline(),5)).decode().strip()
  assert ready.startswith('READY '),ready
  path=ready.split(' ',1)[1]
  for _ in range(30):
   nodes=glob.glob(path+'/event*')
   if nodes and os.path.exists('/dev/input/'+Path(nodes[0]).name):break
   await asyncio.sleep(.1)
  device='/dev/input/'+Path(nodes[0]).name
  capture=await asyncio.create_subprocess_exec('stdbuf','-oL','libinput','debug-events','--device',device,'--grab',stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.STDOUT)
  first=await asyncio.wait_for(capture.stdout.readline(),5)
  assert b'DEVICE_ADDED' in first,first
  await asyncio.sleep(.1)
  async def frame(points):
   helper.stdin.write((json.dumps({'points':points, 'cursor_speed':2.0 if '--speeds' in sys.argv else 1.0, 'scroll_speed':0.5 if '--speeds' in sys.argv else 1.0})+'\n').encode());await helper.stdin.drain();await asyncio.sleep(.018)
  # Three-finger swipe, reverse, end.
  for j in list(range(26))+list(range(25,-1,-1)):
   await frame([{'id':i,'x':.2+i*.12+j*.004,'y':.4} for i in range(3)])
  await frame([]);await asyncio.sleep(.15)
  # Two-finger scroll.
  for j in range(30):await frame([{'id':i,'x':.35+i*.15,'y':.3+j*.005} for i in range(2)])
  await frame([]);await asyncio.sleep(.15)
  # Pinch outward, then leave contacts down to test helper watchdog.
  for j in range(35):await frame([{'id':0,'x':.4-j*.004,'y':.4},{'id':1,'x':.6+j*.004,'y':.4}])
  await asyncio.sleep(1.8)
  helper.stdin.close();await helper.wait();await asyncio.sleep(.1)
  capture.terminate();out=first+(await capture.communicate())[0]
  text=out.decode();Path('evidence').mkdir(exist_ok=True);Path('evidence/native-speeds.txt' if '--speeds' in sys.argv else 'evidence/native-gestures.txt').write_text(text)
  for expected in ['GESTURE_SWIPE_BEGIN','GESTURE_SWIPE_UPDATE','GESTURE_SWIPE_END','POINTER_SCROLL_FINGER','GESTURE_PINCH_BEGIN','GESTURE_PINCH_END']:
   assert expected in text,expected+' missing:\n'+text
  print('PASS: Rust swipe/reversal, two-finger scroll, pinch, watchdog release; device '+device)
 finally:
  if capture and capture.returncode is None:capture.terminate();await capture.wait()
  if helper.returncode is None:helper.stdin.close();await helper.wait()
asyncio.run(main())
