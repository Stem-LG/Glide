"""Equal CSS-pixel strokes must produce equal libinput motion on both axes.
Only a newly created, exclusively grabbed virtual device is exercised.
"""
import asyncio,glob,json,os,re,statistics,sys
from pathlib import Path
async def main():
 binary=str(Path(sys.argv[1]).resolve());phase='setup';lines=[];capture=None;reader=None
 helper=await asyncio.create_subprocess_exec(binary,'--input-helper',stdin=asyncio.subprocess.PIPE,stdout=asyncio.subprocess.PIPE)
 try:
  ready=(await asyncio.wait_for(helper.stdout.readline(),5)).decode().strip();assert ready.startswith('READY ')
  for _ in range(50):
   nodes=glob.glob(ready.split(' ',1)[1]+'/event*')
   if nodes and os.path.exists('/dev/input/'+Path(nodes[0]).name):break
   await asyncio.sleep(.02)
  capture=await asyncio.create_subprocess_exec('stdbuf','-oL','libinput','debug-events','--device','/dev/input/'+Path(nodes[0]).name,'--grab',stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.STDOUT)
  added=await asyncio.wait_for(capture.stdout.readline(),5);assert b'DEVICE_ADDED' in added
  async def read():
   while line:=await capture.stdout.readline():lines.append((phase,line.decode()))
  reader=asyncio.create_task(read());await asyncio.sleep(.1)
  async def frame(points,width,height,gain):
   helper.stdin.write((json.dumps({'points':points,'surface':{'width':width,'height':height},'cursor_speed':gain,'scroll_speed':gain})+'\n').encode());await helper.stdin.drain();await asyncio.sleep(.02)
  results=[]
  for width,height in [(390,844),(844,390)]:
   for gain in [1,2]:
    for fingers in [1,2]:
     axis_values=[]
     for axis in ['x','y']:
      phase=f'{width}x{height}/{gain}/{fingers}/{axis}'
      for step in range(41):
       points=[{'id':i,'x':.3+i*.1,'y':.3+i*.1} for i in range(fingers)]
       for p in points:p[axis]+=step*2/(width if axis=='x' else height)
       await frame(points,width,height,gain)
      await asyncio.sleep(.05)
      data=[line for tag,line in lines if tag==phase];values=[]
      for line in data:
       if fingers==1 and 'POINTER_MOTION' in line:
        m=re.search(r'\(\s*([+-]?[\d.]+)/\s*([+-]?[\d.]+)\)',line)
        if m:values.append(abs(float(m.group(1 if axis=='x' else 2))))
       if fingers==2 and 'POINTER_SCROLL_FINGER' in line:
        m=re.search((r'horiz' if axis=='x' else r'vert')+r' ([+-]?[\d.]+)/',line)
        if m:values.append(abs(float(m.group(1))))
      assert len(values)>20,(phase,len(values),data[-3:]);axis_values.append(statistics.median(values[10:]))
      phase='release';await frame([],width,height,gain);await asyncio.sleep(.12)
     x,y=axis_values;assert x>0 and abs(x-y)/max(x,y)<.03,(width,height,gain,fingers,x,y)
     results.append(f'{width}x{height} {gain}x {fingers} finger(s): horizontal={x:.2f}, vertical={y:.2f}')
  Path('evidence').mkdir(exist_ok=True)
  Path('evidence/axis-balance.txt').write_text('\n'.join(results)+'\n\n'+''.join(f'{tag} {line}' for tag,line in lines))
  print('PASS: equal horizontal/vertical libinput motion and scroll in portrait/landscape at 1x/2x\n'+'\n'.join(results))
 finally:
  if helper.returncode is None:helper.stdin.close();await helper.wait()
  if capture and capture.returncode is None:capture.terminate();await capture.wait()
  if reader:await reader
asyncio.run(main())
