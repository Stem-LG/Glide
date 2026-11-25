const {test}=require('node:test'),assert=require('node:assert/strict');
const {PcmPlayer}=require('../web/audio.js');
function context(){
 const ctx={state:'running',currentTime:10,destination:{},started:[],outputLatency:.03,baseLatency:.01,
  createBuffer(channels,frames,rate){const data=Array.from({length:channels},()=>new Float32Array(frames));return {channels,frames,rate,getChannelData:c=>data[c]}},
  createBufferSource(){const node={playbackRate:{value:1},connect(){},start(at){ctx.started.push({at,buffer:node.buffer,speed:node.playbackRate.value})}};return node}};
 return ctx;
}
function frame(samples){const b=new ArrayBuffer(samples.length*2),v=new DataView(b);samples.forEach((s,i)=>v.setInt16(i*2,s,true));return b}
const near=(a,b)=>assert.ok(Math.abs(a-b)<1e-6,`${a} vs ${b}`);
test('frames are de-interleaved and scheduled back to back after a short lead',()=>{
 const ctx=context(),p=new PcmPlayer(ctx,48000,2);
 assert.equal(p.push(frame([16384,-16384,32767,-32768])),'queued');assert.equal(p.push(frame([0,0,0,0])),'queued');
 near(ctx.started[0].at,10.05);near(ctx.started[1].at-ctx.started[0].at,2/48000);
 assert.deepEqual([...ctx.started[0].buffer.getChannelData(0)],[.5,32767/32768]);
 assert.deepEqual([...ctx.started[0].buffer.getChannelData(1)],[-.5,-1]);
 near(p.delay,p.next-10+.04);
});
test('a backlog is played fast, then dropped, so delay stays bounded',()=>{
 const ctx=context(),p=new PcmPlayer(ctx,48000,2),chunk=frame(new Array(960).fill(1));
 const results=[];for(let i=0;i<60;i++)results.push(p.push(chunk));
 assert.ok(results.includes('catching-up'));assert.ok(results.includes('dropped'));assert.ok(p.dropped>0);
 assert.ok(ctx.started.some(s=>s.speed>1));assert.ok(p.next-ctx.currentTime<=p.target+.25+.011);
});
test('an underrun rebuffers with a larger target that relaxes during clean playback',()=>{
 const ctx=context(),p=new PcmPlayer(ctx,48000,2),chunk=frame(new Array(960).fill(1));
 p.push(chunk);assert.equal(p.underruns,0);const first=p.target;
 ctx.currentTime=20;assert.equal(p.push(chunk),'queued');assert.equal(p.underruns,1);assert.ok(p.target>first);
 near(ctx.started.at(-1).at,20+first+.02);
 const raised=p.target;for(let i=0;i<3000;i++){ctx.currentTime+=.01;p.push(chunk)}
 assert.ok(p.target<raised);assert.ok(p.target>=p.floor);assert.equal(p.underruns,1);
});
test('nothing is scheduled while the context is suspended or the frame is empty',()=>{
 const ctx=context(),p=new PcmPlayer(ctx);ctx.state='suspended';
 assert.equal(p.push(frame([1,1])),'idle');ctx.state='running';assert.equal(p.push(new ArrayBuffer(0)),'idle');assert.equal(ctx.started.length,0);
});
