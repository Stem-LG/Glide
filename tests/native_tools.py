"""Keypad actions must not lift a touch contact. Grab both test devices first."""
import asyncio,array,fcntl,glob,json,os,sys
from pathlib import Path
async def main():
 h=await asyncio.create_subprocess_exec(str(Path(sys.argv[1]).resolve()),'--input-helper',stdin=-1,stdout=-1,stderr=-1)
 captures=[]
 try:
  ready=(await asyncio.wait_for(h.stdout.readline(),5)).decode().strip()
  media=(await asyncio.wait_for(h.stderr.readline(),5)).decode().strip()
  paths=[]
  for record in [ready,media]:
   for _ in range(50):
    nodes=glob.glob(record.split(' ',1)[1]+'/event*')
    if nodes and os.access('/dev/input/'+Path(nodes[0]).name,os.R_OK):break
    await asyncio.sleep(.1)
   path='/dev/input/'+Path(nodes[0]).name;paths.append(path)
   c=await asyncio.create_subprocess_exec('stdbuf','-oL','libinput','debug-events','--device',path,'--grab',stdout=-1,stderr=-2)
   captures.append(c);assert b'DEVICE_ADDED' in await asyncio.wait_for(c.stdout.readline(),5)
  async def frame(data):
   h.stdin.write((json.dumps(data)+'\n').encode());await h.stdin.drain();await asyncio.sleep(.06)
  def tracking():
   slots=array.array('i',[0x39]+[-1]*5) # ABS_MT_TRACKING_ID followed by five slots
   with open(paths[0],'rb',buffering=0) as f:fcntl.ioctl(f.fileno(),0x80000000|(len(slots)*4<<16)|(ord('E')<<8)|0x0a,slots,True)
   return list(slots)[1:]
  await frame({'points':[{'id':20,'x':.5,'y':.5}]});before=tracking();assert any(v>=0 for v in before)
  await frame({'keypad':'7'});assert tracking()==before,'Keypad interrupted the touch contact'
  await frame({'points':[]});assert tracking()==[-1]*5
  print('PASS: typing on number pad preserves live touchpad contact until explicit release')
 finally:
  for c in captures:
   if c.returncode is None:c.terminate();await c.wait()
  if h.returncode is None:h.stdin.close();await h.wait()
asyncio.run(main())
