const {test}=require('node:test');
const assert=require('node:assert/strict');
const {FourFingerMedia}=require('../web/media.js');
const surface={width:400,height:800};
const pts=(n,dx=0,dy=0)=>Array.from({length:n},(_,id)=>({id,x:.2+id*.12+dx/400,y:.4+dy/800}));
function setup(enabled=true){
    const native=[],actions=[];let timer;
    const gate=new FourFingerMedia(p=>native.push(p.map(v=>({...v}))),a=>actions.push(a),fn=>{timer=fn;return 1},()=>{timer=null});
    gate.enabled=enabled;
    return {gate,native,actions,update:(p,t=0,c=false)=>gate.update(p,surface,t,c),expire:()=>timer?.()};
}
function land(s){for(let n=1;n<=4;n++)s.update(pts(n),n*10)}
function lift(s,t=150){for(let n=3;n>=0;n--)s.update(pts(n),t+(3-n)*10)}
test('four-finger tap sends play/pause once, with no desktop contacts during staggered landing/lift',()=>{
    const s=setup();land(s);lift(s);assert.deepEqual(s.actions,['play_pause']);assert.deepEqual(s.native,[]);
});
test('media disabled passes every contact set through immediately',()=>{
    const s=setup(false);land(s);lift(s);assert.deepEqual(s.actions,[]);
    assert.deepEqual(s.native.map(p=>p.length),[1,2,3,4,3,2,1,0]);
});
test('one/two/three-finger taps pass to native input',()=>{
    for(let n=1;n<=3;n++){
        const s=setup();for(let i=1;i<=n;i++)s.update(pts(i),i*10);
        for(let i=n-1;i>=0;i--)s.update(pts(i),80+(n-i)*10);
        assert.equal(s.native[0].length,n);assert.equal(s.native.at(-1).length,0);
        assert.deepEqual(s.actions,[]);
    }
});
test('native movement commits before media and is never stolen by a late fourth finger',()=>{
    const s=setup();s.update(pts(3),0);s.update(pts(3,10),20);s.update(pts(4,50),30);s.update([],60);
    assert.deepEqual(s.actions,[]);assert.equal(s.native[0][0].x,.2);
    const t=setup();t.update(pts(1));t.expire();t.update(pts(4),110);t.update([],160);
    assert.deepEqual(t.actions,[]);assert.equal(t.native[0].length,1);
});
test('track swipe is direction locked and fires only once per gesture',()=>{
    for(const [dx,action] of [[-60,'previous'],[60,'next']]){
        const s=setup();land(s);s.update(pts(4,dx),100);s.update(pts(4,dx*2),150);s.update(pts(4,-dx),200);lift(s,250);
        assert.deepEqual(s.actions,[action]);assert.deepEqual(s.native,[]);
    }
});
test('volume repeats by distance and time, supports reversal, and never becomes track switching',()=>{
    const s=setup();land(s);s.update(pts(4,0,-30),100);s.update(pts(4,0,-60),120);
    s.update(pts(4,0,-60),200);s.update(pts(4,0,-30),300);s.update(pts(4,100,-30),400);lift(s,450);
    assert.deepEqual(s.actions,['volume_up','volume_up','volume_down']);assert.deepEqual(s.native,[]);
});
test('hold, jitter, pinch, diagonal movement, fifth finger, cancellation and rejoining do not play/pause',()=>{
    for(const scenario of ['hold','move','pinch','diagonal','fifth','cancel','rejoin']){
        const s=setup();land(s);
        if(scenario==='hold')lift(s,400);
        else {
            if(scenario==='move'){s.update(pts(4,12),80);s.update(pts(4),90);}
            if(scenario==='pinch'){const p=pts(4);p[0].x-=.1;p[3].x+=.1;s.update(p,80);}
            if(scenario==='diagonal')s.update(pts(4,35,35),80);
            if(scenario==='fifth')s.update(pts(5),80);
            if(scenario==='cancel')s.update(pts(3),80,true);
            if(scenario==='rejoin'){s.update(pts(3),80);s.update(pts(4),90);}
            lift(s);
        }
        assert.deepEqual(s.actions,[],scenario);assert.ok(s.native.every(p=>!p.length),scenario);
    }
});
test('cancel clears pending timers and next gesture starts fresh',()=>{
    const s=setup();s.update(pts(2));s.gate.cancel();s.expire();assert.ok(s.native.every(p=>!p.length));
    land(s);lift(s);assert.deepEqual(s.actions,['play_pause']);
});
test('changing settings mid-contact cannot steal native input or leak a reserved media gesture',()=>{
    const s=setup(false);s.update(pts(1));s.gate.enabled=true;land(s);lift(s);
    assert.deepEqual(s.actions,[]);assert.equal(s.native.at(-1).length,0);
    const t=setup();land(t);t.gate.enabled=false;lift(t);
    assert.deepEqual(t.native,[]); // server rejects media when desktop setting is off
    t.update(pts(1),400);assert.equal(t.native.at(-1).length,1);
});
test('default timers are called as browser globals, not as gesture methods',()=>{
    const vm=require('node:vm'),fs=require('node:fs');
    let scheduled=0,cancelled=0;
    const checkReceiver=receiver=>{
        if(receiver && typeof receiver.native==='function')throw TypeError('Illegal invocation');
    };
    vm.runInNewContext(fs.readFileSync(require.resolve('../web/media.js'),'utf8')+`
        const gesture=new FourFingerMedia(()=>{},()=>{});
        gesture.enabled=true;
        gesture.update([{id:1,x:.5,y:.5}],{width:400,height:800},0);
        gesture.cancel();
    `,{
        setTimeout:function(){checkReceiver(this);scheduled++;return 1;},
        clearTimeout:function(){checkReceiver(this);cancelled++;}
    });
    assert.equal(scheduled,1);assert.ok(cancelled>=2);
});
