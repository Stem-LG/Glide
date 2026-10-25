"""Measure actual libinput output across the last quarter of the phone at 2x.
Only our newly created virtual device is opened and exclusively grabbed.
"""
import asyncio,glob,json,os,re,sys
from pathlib import Path
async def main():
 binary=str(Path(sys.argv[1]).resolve())
 helper=await asyncio.create_subprocess_exec(binary,'--input-helper',stdin=asyncio.subprocess.PIPE,stdout=asyncio.subprocess.PIPE)
 capture=None;reader_task=None;phase='setup';lines=[]
 try:
  ready=(await asyncio.wait_for(helper.stdout.readline(),5)).decode().strip();assert ready.startswith('READY ')
  for _ in range(50):
   nodes=glob.glob(ready.split(' ',1)[1]+'/event*')
   if nodes and os.path.exists('/dev/input/'+Path(nodes[0]).name):break
   await asyncio.sleep(.02)
  device='/dev/input/'+Path(nodes[0]).name
  capture=await asyncio.create_subprocess_exec('stdbuf','-oL','libinput','debug-events','--device',device,'--grab',stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.STDOUT)
  added=await asyncio.wait_for(capture.stdout.readline(),5);assert b'DEVICE_ADDED' in added
  async def read():
   while line:=await capture.stdout.readline():lines.append((phase,line.decode()))
  reader_task=asyncio.create_task(read());await asyncio.sleep(.1)
  async def frame(points,gain=2):
   helper.stdin.write((json.dumps({'points':points,'cursor_speed':gain,'scroll_speed':gain})+'\n').encode());await helper.stdin.drain();await asyncio.sleep(.02)
  for axis in ['x','y']:
   for fingers in [1,2]:
    for gain in [1,2]:
     for reverse in [False,True]:
      label=f'{axis}-{fingers}-{gain}-{reverse}'
      for step in range(101):
       phase=label+('-tail' if step>=80 else '-start')
       position=1-step/100 if reverse else step/100
       points=[{'id':i,'x':.4+i*.12,'y':.4+i*.12} for i in range(fingers)]
       for p in points:p[axis]=position
       await frame(points,gain)
      phase='release';await frame([]);await asyncio.sleep(.1)
      kind='POINTER_MOTION' if fingers==1 else 'POINTER_SCROLL_FINGER'
      events=[line for tag,line in lines if tag==label+'-tail' and kind in line]
      assert len(events)>=15,(label,len(events),'last 20% stopped moving')
  text=''.join(f'{tag} {line}' for tag,line in lines)
  assert 'outside expected range' not in text,text
  Path('evidence').mkdir(exist_ok=True)
  Path('evidence/full-sweep.txt').write_text(added.decode()+text)
  print('PASS: continuous last-quarter libinput motion/scroll in both axes/directions at 1x and 2x')
 finally:
  if helper.returncode is None:helper.stdin.close();await helper.wait()
  if capture and capture.returncode is None:capture.terminate();await capture.wait()
  if reader_task:await reader_task
asyncio.run(main())
